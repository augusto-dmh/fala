//! "Test the meeting sound" in the first run: the level of the output a meeting records, read
//! while the person tests and for at most [`LIMIT`]. Nothing is written anywhere and nothing
//! starts without the click (ADR-0005); a recording is a meeting, this is a meter.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use fala_audio::SystemAudio;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;
use tauri_specta::Event;

use super::MeetingError;

/// A forgotten test stops by itself.
const LIMIT: Duration = Duration::from_secs(60);
const TICK: Duration = Duration::from_millis(50);
const REPORT_EVERY: Duration = Duration::from_millis(250);

/// The RMS of the meeting sound since the last event, 0 to 1. `ended`: the test stopped by
/// itself after [`LIMIT`]; a stop the UI asked for sends nothing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type, Event)]
pub struct MeetingSoundLevel {
    pub level: f32,
    pub ended: bool,
}

/// RMS of the samples; no samples is silence.
pub(crate) fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|s| f64::from(*s) * f64::from(*s)).sum();
    (sum / samples.len() as f64).sqrt() as f32
}

/// The test running now, if any: one at a time.
#[derive(Default)]
pub struct SoundCheck {
    running: Mutex<Option<Arc<AtomicBool>>>,
}

impl SoundCheck {
    /// Opens `output` and reports its level until [`SoundCheck::stop`] or [`LIMIT`]. Blocks
    /// until the output is open; a test already running stops first.
    pub fn start(&self, app: AppHandle, output: String) -> Result<(), MeetingError> {
        let stop = self.register();
        let (opened_tx, opened_rx) = sync_channel(1);
        let flag = Arc::clone(&stop);
        // The stream is opened and dropped on its own thread: it is not `Send` on every host.
        let spawned = std::thread::Builder::new()
            .name("meeting-sound-check".into())
            .spawn(move || {
                let mut system = match SystemAudio::open(&output) {
                    Ok(system) => {
                        let _ = opened_tx.send(Ok(()));
                        system
                    }
                    Err(e) => {
                        let _ = opened_tx.send(Err(e.to_string()));
                        return;
                    }
                };
                let started = Instant::now();
                let mut last = Instant::now();
                let mut buffer = Vec::new();
                while !flag.load(Ordering::Relaxed) {
                    if started.elapsed() >= LIMIT {
                        let ended = MeetingSoundLevel {
                            level: 0.0,
                            ended: true,
                        };
                        let _ = ended.emit(&app);
                        return;
                    }
                    std::thread::sleep(TICK);
                    system.drain_into(&mut buffer);
                    if last.elapsed() >= REPORT_EVERY {
                        last = Instant::now();
                        let level = rms(&buffer);
                        buffer.clear();
                        let _ = MeetingSoundLevel {
                            level,
                            ended: false,
                        }
                        .emit(&app);
                    }
                }
            });
        let opened = match spawned {
            Ok(_) => opened_rx.recv().unwrap_or_else(|_| {
                Err("the sound check stopped before opening the output".into())
            }),
            Err(e) => Err(e.to_string()),
        };
        opened.map_err(|detail| {
            self.forget(&stop);
            MeetingError::AudioDevice(detail)
        })
    }

    pub fn stop(&self) {
        if let Some(stop) = self.slot().take() {
            stop.store(true, Ordering::Relaxed);
        }
    }

    /// The flag of a new test, stored before its output opens: a stop sent meanwhile reaches
    /// it. The test it replaces stops.
    fn register(&self) -> Arc<AtomicBool> {
        let stop = Arc::new(AtomicBool::new(false));
        if let Some(previous) = self.slot().replace(Arc::clone(&stop)) {
            previous.store(true, Ordering::Relaxed);
        }
        stop
    }

    /// A test that failed to open leaves the slot, unless another one took it meanwhile.
    fn forget(&self, stop: &Arc<AtomicBool>) {
        let mut slot = self.slot();
        if slot
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, stop))
        {
            *slot = None;
        }
    }

    fn slot(&self) -> MutexGuard<'_, Option<Arc<AtomicBool>>> {
        self.running.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_of_silence_and_of_a_square_wave() {
        assert_eq!(rms(&[]), 0.0);
        assert_eq!(rms(&[0.0; 480]), 0.0);
        let square: Vec<f32> = (0..480)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();
        assert!((rms(&square) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_stop_reaches_a_test_still_opening_and_a_new_test_replaces_the_old() {
        let check = SoundCheck::default();
        check.stop();
        let first = check.register();
        let second = check.register();
        assert!(first.load(Ordering::Relaxed), "the replaced test stops");
        assert!(!second.load(Ordering::Relaxed));
        // The first one failing to open does not drop the second.
        check.forget(&first);
        assert!(check.slot().is_some());
        check.stop();
        assert!(second.load(Ordering::Relaxed));
        assert!(check.slot().is_none());
        check.forget(&second);
        assert!(check.slot().is_none());
    }
}
