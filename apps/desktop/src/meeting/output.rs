//! Which output the meeting records ("Meeting sound" in the panel) and the warnings about it:
//! the call playing on another output, and a channel in digital zero for ~3 s.
//!
//! The rules live in `fala-meeting` (`OutputWatch`, `ZeroWatch`); the list and the peaks in
//! `fala-audio`. This module picks the output at start and keeps the status fields up to date.

use std::time::Duration;

use fala_audio::{OutputPeak, SystemAudio};
use fala_meeting::{OutputWatch, SessionMode, ZeroWatch};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::{MeetingError, MeetingStatus};

/// An output the "Meeting sound" selector offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct MeetingOutput {
    pub name: String,
    pub is_default: bool,
}

/// The outputs and the remembered choice (`None`: follow the default output).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct MeetingOutputs {
    pub devices: Vec<MeetingOutput>,
    pub selected: Option<String>,
}

pub(crate) fn outputs(selected: Option<String>) -> Result<MeetingOutputs, MeetingError> {
    let devices = SystemAudio::outputs()
        .map_err(|e| MeetingError::AudioDevice(e.to_string()))?
        .into_iter()
        .map(|d| MeetingOutput {
            name: d.name,
            is_default: d.is_default,
        })
        .collect();
    Ok(MeetingOutputs { devices, selected })
}

/// The output to record: the remembered one while it exists, otherwise the default. A headset
/// left at home must not block the recording; the panel shows what is recorded.
pub(crate) fn pick(
    saved: Option<&str>,
    available: &[String],
    default: Option<String>,
) -> Option<String> {
    match saved {
        Some(name) if available.iter().any(|a| a == name) => Some(name.to_string()),
        Some(name) => {
            log::warn!("meeting output {name} not found; recording the default output");
            default
        }
        None => default,
    }
}

pub(crate) fn resolve(saved: Option<&str>) -> Result<String, MeetingError> {
    let available: Vec<String> = match saved {
        Some(_) => SystemAudio::outputs()
            .map(|list| list.into_iter().map(|d| d.name).collect())
            .unwrap_or_default(),
        None => Vec::new(),
    };
    let default = SystemAudio::default_name().ok();
    pick(saved, &available, default).ok_or_else(|| {
        MeetingError::AudioDevice("there is no output to record the meeting sound from".into())
    })
}

/// The two warnings of a running session.
pub(crate) struct Route {
    zero: ZeroWatch,
    watch: OutputWatch,
}

impl Route {
    pub(crate) fn new(mode: SessionMode, system: &str) -> Self {
        Self {
            zero: ZeroWatch::new(mode),
            watch: OutputWatch::new(system),
        }
    }

    pub(crate) fn recording(&self) -> &str {
        self.watch.recording()
    }

    pub(crate) fn switched(&mut self, name: &str) {
        self.watch.set_recording(name);
    }

    /// Reads one report; the result goes to the status with [`RouteStatus::apply`].
    pub(crate) fn observe(
        &mut self,
        recorded: Duration,
        mic_peak: f32,
        system_peak: f32,
        outputs: &[OutputPeak],
    ) -> RouteStatus {
        let zeroed = self.zero.observe(recorded, mic_peak, system_peak);
        let call_output = self
            .watch
            .observe(
                recorded,
                system_peak,
                outputs.iter().map(|p| (p.name.as_str(), p.peak)),
            )
            .map(str::to_string);
        RouteStatus {
            zero_mic: zeroed.mic,
            zero_system: zeroed.system,
            system_output: self.watch.recording().to_string(),
            call_output,
        }
    }
}

/// The status fields one report sets.
pub(crate) struct RouteStatus {
    zero_mic: bool,
    zero_system: bool,
    system_output: String,
    call_output: Option<String>,
}

impl RouteStatus {
    /// `true` when the "plays elsewhere" card appears or goes away.
    pub(crate) fn apply(self, status: &mut MeetingStatus) -> bool {
        let changed = self.call_output.is_some() != status.call_output.is_some();
        status.zero_mic = self.zero_mic;
        status.zero_system = self.zero_system;
        status.system_output = Some(self.system_output);
        status.call_output = self.call_output;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn saved_output_is_used_while_it_exists() {
        let available = names(&["Alto-falantes", "Fone USB"]);
        let default = Some("Alto-falantes".to_string());
        assert_eq!(
            pick(Some("Fone USB"), &available, default.clone()).as_deref(),
            Some("Fone USB")
        );
        // Unplugged: the default, never an error.
        assert_eq!(
            pick(Some("Fone Bluetooth"), &available, default.clone()).as_deref(),
            Some("Alto-falantes")
        );
        assert_eq!(
            pick(None, &available, default).as_deref(),
            Some("Alto-falantes")
        );
        assert_eq!(pick(None, &available, None), None);
    }

    #[test]
    fn status_follows_the_route() {
        let mut route = Route::new(SessionMode::Meeting, "Alto-falantes");
        let mut status = MeetingStatus::default();
        let peaks = vec![OutputPeak {
            name: "Fone USB".into(),
            peak: 0.3,
        }];
        let step = Duration::from_millis(250);
        let mut changes = 0;
        for i in 1..=40 {
            if route
                .observe(step * i, 0.01, 0.0, &peaks)
                .apply(&mut status)
            {
                changes += 1;
            }
        }
        assert_eq!(status.call_output.as_deref(), Some("Fone USB"));
        assert_eq!(status.system_output.as_deref(), Some("Alto-falantes"));
        assert!(status.zero_system && !status.zero_mic);
        assert_eq!(changes, 1);

        route.switched("Fone USB");
        assert!(route
            .observe(step * 41, 0.01, 0.3, &peaks)
            .apply(&mut status));
        assert_eq!(status.call_output, None);
        assert_eq!(status.system_output.as_deref(), Some("Fone USB"));
        assert!(!status.zero_system);
    }
}
