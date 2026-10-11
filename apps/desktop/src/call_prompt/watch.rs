//! Reads who captures the mic once a second and feeds `fala_meeting::CallDetector`.
//!
//! Read only: `fala_audio::platform_activity` lists capture sessions and opens no stream. With
//! "Avisar quando uma chamada começar" off the OS is not read at all.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fala_audio::AudioError;
use fala_meeting::{identify, CallApp, CallDetector, DetectContext, UnixMillis};
use tauri::{AppHandle, Manager};

use crate::meeting::{MeetingManager, MeetingPhase, MeetingStatus};
use crate::settings;

const POLL: Duration = Duration::from_secs(1);

/// The recorded time while a meeting records or is paused.
pub(crate) fn recording(status: &MeetingStatus) -> Option<Duration> {
    match status.state {
        MeetingPhase::Recording | MeetingPhase::Paused => {
            Some(Duration::from_millis(status.recorded_ms))
        }
        MeetingPhase::Idle | MeetingPhase::Stopping | MeetingPhase::Processing => None,
    }
}

/// This process's exe, lowercase, so Fala never takes itself for a call.
fn own_exe() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.file_name().map(|n| n.to_string_lossy().to_lowercase()))
        .unwrap_or_default()
}

fn now() -> UnixMillis {
    UnixMillis(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    )
}

pub fn spawn(app: &AppHandle) {
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("call-watch".into())
        .spawn(move || run(app));
    if let Err(e) = spawned {
        log::error!("call watch thread not started: {e}");
    }
}

fn run(app: AppHandle) {
    let mut activity = fala_audio::platform_activity();
    let own = own_exe();
    let mut detector = CallDetector::new();
    let mut failing = false;
    let mut last: Vec<&'static str> = Vec::new();
    loop {
        std::thread::sleep(POLL);
        let settings = settings::get_settings(&app);
        if !settings.call_prompt_enabled {
            detector = CallDetector::new();
            continue;
        }
        let holders = match activity.mic_holders() {
            Ok(holders) => {
                failing = false;
                holders
            }
            Err(AudioError::Unsupported(what)) => {
                log::info!("call detection off on this system: {what}");
                return;
            }
            Err(e) => {
                if !failing {
                    log::warn!("call detection: {e}");
                    failing = true;
                }
                continue;
            }
        };
        let mut apps: Vec<&'static CallApp> = Vec::new();
        for holder in &holders {
            if let Some(app) = identify(holder.chain(), &own) {
                if !apps.contains(&app) {
                    apps.push(app);
                }
            }
        }
        let ids: Vec<&'static str> = apps.iter().map(|a| a.id).collect();
        if ids != last {
            log::debug!("call apps holding the mic: {ids:?}");
            last = ids;
        }

        let recording = app
            .try_state::<Arc<MeetingManager>>()
            .and_then(|meetings| recording(&meetings.status()));
        if recording.is_some() {
            super::hide_offer(&app);
        }
        let ctx = DetectContext {
            enabled: true,
            muted: &settings.call_prompt_muted_apps,
            recording,
        };
        if let Some(prompt) = detector.observe(now(), &apps, ctx) {
            log::info!("call prompt: {prompt:?}");
            super::show(&app, prompt);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_counts_paused_but_not_processing() {
        let status = |state| MeetingStatus {
            state,
            recorded_ms: 200_000,
            ..MeetingStatus::default()
        };
        assert_eq!(
            recording(&status(MeetingPhase::Recording)),
            Some(Duration::from_secs(200))
        );
        assert_eq!(
            recording(&status(MeetingPhase::Paused)),
            Some(Duration::from_secs(200))
        );
        for state in [
            MeetingPhase::Idle,
            MeetingPhase::Stopping,
            MeetingPhase::Processing,
        ] {
            assert_eq!(recording(&status(state)), None);
        }
    }
}
