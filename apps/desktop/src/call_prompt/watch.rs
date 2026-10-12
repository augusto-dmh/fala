//! Reads who captures the mic once a second and feeds `fala_meeting::CallDetector`.
//!
//! Read only: `fala_audio::platform_activity` lists capture sessions and opens no stream. With
//! "Avisar quando uma chamada começar" off the OS is not read at all.
//!
//! A browser with the mic only counts as a call when one of its windows shows a meeting
//! (`fala_meeting::web_call`). Window titles are local context (ADR-0012): read here, matched
//! and dropped, never stored, sent or logged.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fala_audio::AudioError;
use fala_meeting::{
    identify, web_call, CallDetector, CallHolder, CallPrompt, DetectContext, UnixMillis,
};
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
    let mut last: Vec<(&'static str, Option<&'static str>)> = Vec::new();
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
        let mut apps: Vec<CallHolder> = Vec::new();
        for holder in &holders {
            let Some(app) = identify(holder.chain(), &own) else {
                continue;
            };
            if apps.iter().any(|seen| seen.app == app) {
                continue;
            }
            // The detector keeps the meeting for the whole hold: read the titles until it has one.
            let known = detector
                .holding()
                .find(|held| held.app == app)
                .and_then(|held| held.web);
            let web = match known {
                Some(web) => Some(web),
                None if app.is_browser() => {
                    web_call(window_titles(app.exes()).iter().map(String::as_str))
                }
                None => None,
            };
            apps.push(CallHolder { app, web });
        }
        let ids: Vec<(&'static str, Option<&'static str>)> = apps
            .iter()
            .map(|a| (a.app.id, a.web.map(|web| web.name)))
            .collect();
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
            let (kind, holder) = match prompt {
                CallPrompt::OfferRecording { app, web } => ("offer", CallHolder { app, web }),
                CallPrompt::AskStop { app, web } => ("ask stop", CallHolder { app, web }),
            };
            log::info!(
                "call prompt: {kind} for {} {:?}",
                holder.app.id,
                holder.web.map(|web| web.name)
            );
            super::show(&app, prompt);
        }
    }
}

/// The titles of the visible top-level windows whose process runs one of `exes` (lowercase).
/// Only for `web_call`: the caller drops them right after the match.
#[cfg(windows)]
fn window_titles(exes: &[&str]) -> Vec<String> {
    use std::collections::HashMap;

    use windows::core::{BOOL, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        IsWindowVisible,
    };

    unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` is the `Vec` below, alive for the whole `EnumWindows` call.
        let windows = unsafe { &mut *(lparam.0 as *mut Vec<(u32, HWND)>) };
        if unsafe { IsWindowVisible(hwnd) }.as_bool() {
            let mut pid = 0u32;
            unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
            windows.push((pid, hwnd));
        }
        BOOL(1)
    }

    /// Lowercase exe name of `pid`, without the path.
    fn exe_name(pid: u32) -> Option<String> {
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let queried = unsafe {
            QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
        };
        let _ = unsafe { CloseHandle(process) };
        queried.ok()?;
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        let name = path.rsplit(['\\', '/']).next()?;
        Some(name.to_lowercase())
    }

    let mut windows: Vec<(u32, HWND)> = Vec::new();
    // SAFETY: `collect` only writes to `windows`, which outlives the call.
    if let Err(e) = unsafe { EnumWindows(Some(collect), LPARAM(&mut windows as *mut _ as isize)) } {
        log::debug!("call detection: EnumWindows failed: {e}");
        return Vec::new();
    }
    let mut ours: HashMap<u32, bool> = HashMap::new();
    let mut titles = Vec::new();
    for (pid, hwnd) in windows {
        let is_ours = *ours
            .entry(pid)
            .or_insert_with(|| exe_name(pid).is_some_and(|exe| exes.contains(&exe.as_str())));
        if !is_ours {
            continue;
        }
        let len = unsafe { GetWindowTextLengthW(hwnd) };
        if len <= 0 {
            continue;
        }
        let mut buf = vec![0u16; len as usize + 1];
        let copied = unsafe { GetWindowTextW(hwnd, &mut buf) };
        if copied > 0 {
            titles.push(String::from_utf16_lossy(&buf[..copied as usize]));
        }
    }
    titles
}

/// Off Windows there is no reader yet: a browser never counts as a call.
#[cfg(not(windows))]
fn window_titles(_exes: &[&str]) -> Vec<String> {
    Vec::new()
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
