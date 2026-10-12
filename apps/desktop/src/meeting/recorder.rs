//! The meeting capture thread: the default mic and the chosen system output pushed into
//! `fala_audio::MeetingRecorder`, the same loop `fala-cli meeting` runs, plus pause and the
//! switch of the system output while recording. A "system only" session opens no mic: the
//! recorder fills that channel with zeros.
//!
//! The streams are opened on the thread that owns them (a cpal stream is not `Send` on every
//! host). Paused time is neither written nor counted: the recorder's clock is an [`ActiveClock`]
//! that only advances while recording.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use fala_audio::{
    MeetingRecorder, MeetingWav, Mic, OutputMeter, OutputPeak, SystemAudio, MEETING_RATE,
};

const TICK: Duration = Duration::from_millis(100);
/// How often levels and the recorded duration go back to the manager (UI at 4 Hz).
pub(crate) const REPORT_EVERY: Duration = Duration::from_millis(250);

pub(crate) enum Control {
    Pause,
    Resume,
    /// Record the system channel from this output from now on; the WAV keeps zeros for the
    /// moment without a source.
    SwitchSystem(String),
    /// Close the WAV (header and file) and end the thread.
    Finalize,
}

pub(crate) enum Report {
    /// Recorded duration so far, the RMS and the absolute peak of each channel and the peak of
    /// every output since the last report (empty without an output meter).
    Levels {
        recorded: Duration,
        mic: f32,
        system: f32,
        mic_peak: f32,
        system_peak: f32,
        outputs: Vec<OutputPeak>,
    },
    /// The system channel now records `name`, or still records the previous output (`error`).
    SystemSwitched { name: String, error: Option<String> },
    /// The WAV is closed (or the capture failed); the thread is gone.
    Finalized {
        recorded: Duration,
        error: Option<String>,
    },
}

/// Wall time that only advances while not paused.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ActiveClock {
    started: Instant,
    paused_total: Duration,
    paused_since: Option<Instant>,
}

impl ActiveClock {
    pub(crate) fn new(now: Instant) -> Self {
        Self {
            started: now,
            paused_total: Duration::ZERO,
            paused_since: None,
        }
    }

    pub(crate) fn pause(&mut self, now: Instant) {
        if self.paused_since.is_none() {
            self.paused_since = Some(now);
        }
    }

    pub(crate) fn resume(&mut self, now: Instant) {
        if let Some(since) = self.paused_since.take() {
            self.paused_total += now.saturating_duration_since(since);
        }
    }

    pub(crate) fn is_paused(&self) -> bool {
        self.paused_since.is_some()
    }

    pub(crate) fn elapsed(&self, now: Instant) -> Duration {
        let until = self.paused_since.unwrap_or(now);
        until
            .saturating_duration_since(self.started)
            .saturating_sub(self.paused_total)
    }
}

/// Recorded milliseconds for a number of 48 kHz frames written to the WAV.
pub(crate) fn recorded_ms(written_frames: u64) -> u64 {
    written_frames * 1_000 / u64::from(MEETING_RATE)
}

/// Opens the system output and, `with_mic`, the default mic on a new thread and starts writing
/// `wav`. Returns once the streams are open (or with the reason they did not open).
pub(crate) fn spawn(
    system: String,
    with_mic: bool,
    wav: PathBuf,
    on_report: impl Fn(Report) + Send + 'static,
) -> Result<Sender<Control>, String> {
    let (control, commands) = mpsc::channel();
    let (ready_tx, ready_rx) = mpsc::channel();
    thread::Builder::new()
        .name("meeting-recorder".to_string())
        .spawn(move || match open(&system, with_mic, &wav) {
            Ok(capture) => {
                let _ = ready_tx.send(Ok(()));
                run(capture, commands, on_report);
            }
            Err(e) => {
                let _ = ready_tx.send(Err(e));
            }
        })
        .map_err(|e| format!("could not start the recorder thread: {e}"))?;
    ready_rx
        .recv()
        .map_err(|_| "the recorder thread ended before opening".to_string())??;
    Ok(control)
}

struct Capture {
    /// `None` in a "system only" session.
    mic: Option<Mic>,
    system: SystemAudio,
    recorder: MeetingRecorder,
    /// Peaks of every output, to tell "nothing plays" from "it plays elsewhere"; `None` where
    /// the OS has no meter (Linux until phase 3).
    meter: Option<OutputMeter>,
}

fn open(system_name: &str, with_mic: bool, wav: &Path) -> Result<Capture, String> {
    if let Some(dir) = wav.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let system = SystemAudio::open(system_name).map_err(|e| e.to_string())?;
    let mic = with_mic
        .then(|| Mic::open(None))
        .transpose()
        .map_err(|e| e.to_string())?;
    let mic_rate = mic.as_ref().map_or(MEETING_RATE, Mic::sample_rate);
    let file = MeetingWav::create(wav).map_err(|e| e.to_string())?;
    let recorder =
        MeetingRecorder::new(mic_rate, system.sample_rate(), file).map_err(|e| e.to_string())?;
    log::info!(
        "meeting capture open: mic {}, system {system_name}",
        mic.as_ref().map_or("none", Mic::name)
    );
    let meter = OutputMeter::start()
        .map_err(|e| log::info!("output peaks unavailable: {e}"))
        .ok();
    Ok(Capture {
        mic,
        system,
        recorder,
        meter,
    })
}

/// Opens `name` and, only once it is open, drops the old system stream: a failed switch keeps
/// recording what it recorded. What the old stream still held is written unless paused.
fn switch_system(
    capture: &mut Capture,
    name: &str,
    paused: bool,
    buffer: &mut Vec<f32>,
) -> Result<(), String> {
    let next = SystemAudio::open(name).map_err(|e| e.to_string())?;
    buffer.clear();
    capture.system.drain_into(buffer);
    if !paused {
        capture
            .recorder
            .push_system(buffer)
            .map_err(|e| e.to_string())?;
    }
    capture
        .recorder
        .set_system_rate(next.sample_rate())
        .map_err(|e| e.to_string())?;
    capture.system = next;
    log::info!("meeting system output switched to {name}");
    Ok(())
}

/// RMS and absolute peak accumulated between two reports.
#[derive(Default)]
struct Level {
    sum: f64,
    count: u64,
    peak: f32,
}

impl Level {
    fn add(&mut self, samples: &[f32]) {
        self.sum += samples
            .iter()
            .map(|s| f64::from(*s) * f64::from(*s))
            .sum::<f64>();
        self.count += samples.len() as u64;
        self.peak = samples.iter().fold(self.peak, |m, s| m.max(s.abs()));
    }

    /// (RMS, peak); no samples at all is digital zero.
    fn take(&mut self) -> (f32, f32) {
        let rms = if self.count == 0 {
            0.0
        } else {
            (self.sum / self.count as f64).sqrt() as f32
        };
        let peak = self.peak;
        *self = Level::default();
        (rms, peak)
    }
}

fn run(mut capture: Capture, commands: Receiver<Control>, on_report: impl Fn(Report)) {
    // `meeting-power-guard` keeps the PC awake for exactly the life of this loop.
    let mut clock = ActiveClock::new(Instant::now());
    let mut buffer = Vec::new();
    let (mut mic_level, mut system_level) = (Level::default(), Level::default());
    let mut last_report = Instant::now();
    let mut error = None;
    loop {
        let finalize = match commands.recv_timeout(TICK) {
            Ok(Control::Pause) => {
                clock.pause(Instant::now());
                false
            }
            Ok(Control::Resume) => {
                clock.resume(Instant::now());
                false
            }
            Ok(Control::SwitchSystem(name)) => {
                let paused = clock.is_paused();
                let error = switch_system(&mut capture, &name, paused, &mut buffer).err();
                if let Some(e) = &error {
                    log::warn!("meeting system output not switched to {name}: {e}");
                }
                on_report(Report::SystemSwitched { name, error });
                false
            }
            Ok(Control::Finalize) | Err(RecvTimeoutError::Disconnected) => true,
            Err(RecvTimeoutError::Timeout) => false,
        };
        let now = Instant::now();
        let paused = clock.is_paused();

        // Drained even while paused, so the paused interval is dropped instead of written later.
        buffer.clear();
        if let Some(mic) = capture.mic.as_mut() {
            mic.drain_into(&mut buffer);
        }
        if !paused {
            mic_level.add(&buffer);
            if let Err(e) = capture.recorder.push_mic(&buffer) {
                error = Some(e.to_string());
            }
        }
        buffer.clear();
        capture.system.drain_into(&mut buffer);
        if !paused {
            system_level.add(&buffer);
            if let Err(e) = capture.recorder.push_system(&buffer) {
                error = Some(e.to_string());
            }
        }

        if finalize || error.is_some() {
            break;
        }
        if !paused {
            if let Err(e) = capture.recorder.tick(clock.elapsed(now)) {
                error = Some(e.to_string());
                break;
            }
        }
        if now.duration_since(last_report) >= REPORT_EVERY {
            last_report = now;
            let (mic, mic_peak) = mic_level.take();
            let (system, system_peak) = system_level.take();
            on_report(Report::Levels {
                recorded: Duration::from_millis(recorded_ms(capture.recorder.written())),
                mic,
                system,
                mic_peak,
                system_peak,
                outputs: capture.meter.as_ref().map(|m| m.take()).unwrap_or_default(),
            });
        }
    }

    let recorded = clock.elapsed(Instant::now());
    if let Err(e) = capture.recorder.finish(recorded) {
        error.get_or_insert_with(|| e.to_string());
    }
    if let Some(e) = &error {
        log::error!("meeting capture ended with an error: {e}");
    }
    on_report(Report::Finalized { recorded, error });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paused_time_is_not_counted() {
        let t0 = Instant::now();
        let at = |s: u64| t0 + Duration::from_secs(s);
        let mut clock = ActiveClock::new(t0);
        assert_eq!(clock.elapsed(at(10)), Duration::from_secs(10));
        clock.pause(at(10));
        assert_eq!(clock.elapsed(at(12)), Duration::from_secs(10));
        // A second pause does not move the start of the pause.
        clock.pause(at(13));
        clock.resume(at(15));
        assert_eq!(clock.elapsed(at(15)), Duration::from_secs(10));
        assert_eq!(clock.elapsed(at(18)), Duration::from_secs(13));
        // Resuming twice changes nothing.
        clock.resume(at(18));
        assert_eq!(clock.elapsed(at(18)), Duration::from_secs(13));
    }

    #[test]
    fn recorded_ms_follows_frames() {
        assert_eq!(recorded_ms(48_000), 1_000);
        assert_eq!(recorded_ms(0), 0);
        assert_eq!(recorded_ms(48_000 * 3_600), 3_600_000);
        assert_eq!(recorded_ms(47), 0);
    }
}
