//! The meeting capture thread: the microphone dictation uses and the chosen system output pushed
//! into `fala_audio::MeetingRecorder`, the same loop `fala-cli meeting` runs, plus pause and the
//! switch of either device while recording. A "system only" session opens no mic: the recorder
//! fills that channel with zeros.
//!
//! The mic is a tap of `MIC_HUB`, the stream dictation reads too (ADR-0015): while a dictation
//! runs, the recorder zeroes the mic channel from the dictation's pre-buffer to its end.
//!
//! The streams are opened on the thread that owns them (a cpal stream is not `Send` on every
//! host). Paused time is neither written nor counted: the recorder's clock is an [`ActiveClock`]
//! that only advances while recording. The same [`Gate`] keeps the PC awake while it records and
//! lets it sleep otherwise; a sleep that happens anyway is a pause the OS takes, after which the
//! streams are opened again into the same WAV.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use fala_audio::{
    FrameTap, MeetingRecorder, MeetingWav, MicTap, OmittedSpan, OutputMeter, OutputPeak,
    SystemAudio, MEETING_RATE,
};

use crate::dictation_capture::MIC_HUB;
use crate::power::{self, PowerApi, PowerGuard};

const TICK: Duration = Duration::from_millis(100);
/// How far back a dictation's interval starts: its 300 ms pre-buffer plus the time the start takes
/// to reach this thread.
pub(crate) const OMIT_LOOKBACK: Duration = Duration::from_millis(400);
/// How long the interval goes on after the dictation's last sample (a second device's latency).
pub(crate) const OMIT_MARGIN: Duration = Duration::from_millis(50);
/// After a sleep, how long the devices have to open again before the recording ends.
pub(crate) const WAKE_GRACE: Duration = Duration::from_secs(5);
const WAKE_RETRY: Duration = Duration::from_millis(500);
/// How often levels and the recorded duration go back to the manager (UI at 4 Hz).
pub(crate) const REPORT_EVERY: Duration = Duration::from_millis(250);

pub(crate) enum Control {
    Pause,
    Resume,
    /// Record the system channel from this output from now on; the WAV keeps zeros for the
    /// moment without a source.
    SwitchSystem(String),
    /// The microphone setting changed: record the mic channel from the new choice from now on.
    SwitchMic,
    /// A dictation started at this instant: zero the mic channel from its pre-buffer on.
    BeginOmit(Instant),
    /// The dictation ended (released, cancelled or discarded).
    EndOmit,
    /// The OS is about to sleep: stop writing, put the WAV header on disk and answer.
    Suspend(Sender<()>),
    /// The OS woke: open the devices again and go on writing, unless paused by the person.
    Wake,
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
        /// A dictation zeroed the mic for part of the period: its level counts for nothing.
        mic_omitted: bool,
    },
    /// Spans of the mic zeroed by a dictation, in ms of the recording (`[t0, t1)`): each one
    /// becomes a "dictation left out" line of the transcript (ADR-0015).
    Omitted(Vec<(u64, u64)>),
    /// The system channel now records `name`, or still records the previous output (`error`).
    SystemSwitched { name: String, error: Option<String> },
    /// The mic channel now records this input.
    MicSwitched(String),
    /// The devices are open again after a sleep.
    Woke,
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

    /// How long the current pause has lasted; zero while recording.
    pub(crate) fn paused_for(&self, now: Instant) -> Duration {
        self.paused_since
            .map_or(Duration::ZERO, |since| now.saturating_duration_since(since))
    }

    pub(crate) fn elapsed(&self, now: Instant) -> Duration {
        let until = self.paused_since.unwrap_or(now);
        until
            .saturating_duration_since(self.started)
            .saturating_sub(self.paused_total)
    }
}

/// When the capture writes: neither paused by the person nor asleep with the OS. The clock runs
/// and, with `keep_awake`, the PC is kept awake exactly then; the guard lets go when the gate is
/// dropped (stop, error or panic of the recorder thread).
pub(crate) struct Gate<A: PowerApi> {
    clock: ActiveClock,
    paused: bool,
    asleep: bool,
    keep_awake: bool,
    awake: PowerGuard<A>,
}

impl<A: PowerApi> Gate<A> {
    pub(crate) fn new(now: Instant, keep_awake: bool, api: A) -> Self {
        let mut gate = Self {
            clock: ActiveClock::new(now),
            paused: false,
            asleep: false,
            keep_awake,
            awake: PowerGuard::new(api),
        };
        gate.update(now);
        gate
    }

    pub(crate) fn pause(&mut self, now: Instant) {
        self.paused = true;
        self.update(now);
    }

    pub(crate) fn resume(&mut self, now: Instant) {
        self.paused = false;
        self.update(now);
    }

    pub(crate) fn sleep(&mut self, now: Instant) {
        self.asleep = true;
        self.update(now);
    }

    pub(crate) fn wake(&mut self, now: Instant) {
        self.asleep = false;
        self.update(now);
    }

    pub(crate) fn writing(&self) -> bool {
        !self.paused && !self.asleep
    }

    pub(crate) fn elapsed(&self, now: Instant) -> Duration {
        self.clock.elapsed(now)
    }

    /// How long the current pause (or sleep) has lasted; zero while writing.
    pub(crate) fn paused_for(&self, now: Instant) -> Duration {
        self.clock.paused_for(now)
    }

    fn update(&mut self, now: Instant) {
        if self.writing() {
            self.clock.resume(now);
        } else {
            self.clock.pause(now);
        }
        self.awake.set(self.keep_awake && self.writing());
    }
}

/// The spans in ms of the recording.
fn omitted_ms(spans: Vec<OmittedSpan>) -> Vec<(u64, u64)> {
    spans
        .into_iter()
        .map(|s| (recorded_ms(s.start), recorded_ms(s.end)))
        .collect()
}

/// Recorded milliseconds for a number of 48 kHz frames written to the WAV.
pub(crate) fn recorded_ms(written_frames: u64) -> u64 {
    written_frames * 1_000 / u64::from(MEETING_RATE)
}

/// The input the mic channel records and the channel of it, asked at every open: the microphone
/// dictation resolves from the settings, `None` for the system default.
pub(crate) type MicChoice = Box<dyn Fn() -> (Option<cpal::Device>, Option<usize>) + Send>;

/// Opens the system output and, with a `mic` choice, that input on a new thread and starts
/// writing `wav`; with `keep_awake` the PC does not sleep while it records. `tap` gets what the
/// WAV gets (the live preview). Returns once the streams are open, with the name of the mic (or
/// with the reason they did not open).
pub(crate) fn spawn(
    system: String,
    mic: Option<MicChoice>,
    wav: PathBuf,
    keep_awake: bool,
    tap: Option<FrameTap>,
    on_report: impl Fn(Report) + Send + 'static,
) -> Result<(Sender<Control>, Option<String>), String> {
    let (control, commands) = mpsc::channel();
    let (ready_tx, ready_rx) = mpsc::channel();
    thread::Builder::new()
        .name("meeting-recorder".to_string())
        .spawn(move || match open(&system, mic, &wav, tap) {
            Ok(capture) => {
                let mic_name = capture.mic.as_ref().map(|mic| mic.name().to_string());
                let _ = ready_tx.send(Ok(mic_name));
                run(capture, commands, keep_awake, on_report);
            }
            Err(e) => {
                let _ = ready_tx.send(Err(e));
            }
        })
        .map_err(|e| format!("could not start the recorder thread: {e}"))?;
    let mic_name = ready_rx
        .recv()
        .map_err(|_| "the recorder thread ended before opening".to_string())??;
    Ok((control, mic_name))
}

struct Capture {
    /// `None` in a "system only" session.
    mic: Option<MicTap>,
    /// Which input `mic` opens, again after a failure, a sleep or a change of the setting.
    choice: Option<MicChoice>,
    /// The channel of the input `mic` reads (`None`: all of them, mixed).
    mic_channel: Option<usize>,
    system: SystemAudio,
    /// The output `system` records, to open it again after a sleep.
    system_name: String,
    recorder: MeetingRecorder,
    /// Peaks of every output, to tell "nothing plays" from "it plays elsewhere"; `None` where
    /// the OS has no meter (Linux until phase 3).
    meter: Option<OutputMeter>,
}

fn open(
    system_name: &str,
    choice: Option<MicChoice>,
    wav: &Path,
    tap: Option<FrameTap>,
) -> Result<Capture, String> {
    if let Some(dir) = wav.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let (system, mic) = streams(system_name, choice.as_ref(), false)?;
    let (mic, mic_channel) = mic.map_or((None, None), |(tap, channel)| (Some(tap), channel));
    let mic_rate = mic.as_ref().map_or(MEETING_RATE, MicTap::sample_rate);
    let file = MeetingWav::create(wav).map_err(|e| e.to_string())?;
    let mut recorder =
        MeetingRecorder::new(mic_rate, system.sample_rate(), file).map_err(|e| e.to_string())?;
    if let Some(tap) = tap {
        recorder.set_tap(tap);
    }
    log::info!(
        "meeting capture open: mic {}, system {system_name}",
        mic.as_ref().map_or("none", MicTap::name)
    );
    let meter = OutputMeter::start()
        .map_err(|e| log::info!("output peaks unavailable: {e}"))
        .ok();
    Ok(Capture {
        mic,
        choice,
        mic_channel,
        system,
        system_name: system_name.to_string(),
        recorder,
        meter,
    })
}

/// An open mic and the channel of it that it reads.
type OpenMic = (MicTap, Option<usize>);

/// The output and, with a `choice`, the mic and its channel. `renew` asks the hub for a new mic
/// stream instead of the one already open (after a sleep the old one rarely delivers again).
fn streams(
    system_name: &str,
    choice: Option<&MicChoice>,
    renew: bool,
) -> Result<(SystemAudio, Option<OpenMic>), String> {
    let system = SystemAudio::open(system_name).map_err(|e| e.to_string())?;
    let mic = choice.map(|choice| open_mic(choice, renew)).transpose()?;
    Ok((system, mic))
}

fn default_input() -> Result<cpal::Device, String> {
    use cpal::traits::HostTrait;
    crate::audio_toolkit::get_cpal_host()
        .default_input_device()
        .ok_or_else(|| "no input device".to_string())
}

/// `device` through the hub: when dictation uses the same device, both read one stream. With
/// `renew` the hub drops the stream it had for it, so dictation moves too.
fn hub_open(device: cpal::Device, channel: Option<usize>, renew: bool) -> Result<MicTap, String> {
    let tap = if renew {
        MIC_HUB.renew(device, channel)
    } else {
        MIC_HUB.open(device, channel)
    };
    tap.map_err(|e| e.to_string())
}

/// The chosen input and its channel. One that does not open (held by another app, still pairing
/// after a sleep) gives way to the default input, as before the meeting followed the choice: a
/// mic channel from another input is better than no recording.
fn open_mic(choice: &MicChoice, renew: bool) -> Result<OpenMic, String> {
    let (device, channel) = choice();
    let tap = match device {
        Some(device) => hub_open(device, channel, renew).or_else(|e| {
            log::warn!("meeting mic did not open ({e}); recording the default input");
            hub_open(default_input()?, channel, renew)
        }),
        None => hub_open(default_input()?, channel, renew),
    }?;
    Ok((tap, channel))
}

/// How often a failed mic stream is reopened while recording.
const REOPEN_EVERY: Duration = Duration::from_secs(2);

/// The mic's stream failed (device reset or removed): the hub hands out a new stream of the
/// chosen input, the one dictation reopens too, so both stay on a single stream. Returns the
/// name of the new one; an error is the recorder's, and ends the capture.
fn reopen_mic(
    capture: &mut Capture,
    write: bool,
    buffer: &mut Vec<f32>,
) -> Result<Option<String>, String> {
    let Some(choice) = capture.choice.as_ref() else {
        return Ok(None);
    };
    let (tap, channel) = match open_mic(choice, false) {
        Ok(opened) => opened,
        Err(e) => {
            log::warn!("meeting mic stream failed and did not reopen: {e}");
            return Ok(None);
        }
    };
    let name = replace_mic(capture, tap, channel, write, buffer)?;
    log::warn!("meeting mic stream failed; reopened {name}");
    Ok(Some(name))
}

/// Records the mic channel from `tap` from now on. What the old stream still held is written
/// unless paused, and a tap at another rate goes through `set_mic_rate`, which keeps the clock
/// (the same path as after a sleep). Returns the input's name.
fn replace_mic(
    capture: &mut Capture,
    tap: MicTap,
    channel: Option<usize>,
    write: bool,
    buffer: &mut Vec<f32>,
) -> Result<String, String> {
    buffer.clear();
    let old_rate = capture.mic.as_mut().map(|old| {
        old.drain_into(buffer);
        old.sample_rate()
    });
    if write {
        capture
            .recorder
            .push_mic(buffer)
            .map_err(|e| e.to_string())?;
    }
    if old_rate != Some(tap.sample_rate()) {
        capture
            .recorder
            .set_mic_rate(tap.sample_rate())
            .map_err(|e| e.to_string())?;
    }
    let name = tap.name().to_string();
    capture.mic = Some(tap);
    capture.mic_channel = channel;
    Ok(name)
}

/// The microphone setting changed: the new choice is opened and, only once it is open, recorded
/// from; one that does not open keeps the old one. `None` when the choice is the input and
/// channel already recorded. An error is the recorder's, and ends the capture.
fn switch_mic(
    capture: &mut Capture,
    write: bool,
    buffer: &mut Vec<f32>,
) -> Result<Option<String>, String> {
    use cpal::traits::DeviceTrait;
    let Some(choice) = capture.choice.as_ref() else {
        return Ok(None);
    };
    let (device, channel) = choice();
    let opened = device.map_or_else(default_input, Ok).and_then(|device| {
        let name = device.name().map_err(|e| e.to_string())?;
        let current = capture.mic.as_ref().map(MicTap::name);
        if current == Some(name.as_str()) && capture.mic_channel == channel {
            return Ok(None);
        }
        hub_open(device, channel, false).map(Some)
    });
    let tap = match opened {
        Ok(Some(tap)) => tap,
        Ok(None) => return Ok(None),
        Err(e) => {
            log::warn!("meeting mic not switched: {e}");
            return Ok(None);
        }
    };
    let name = replace_mic(capture, tap, channel, write, buffer)?;
    log::info!("meeting mic switched to {name}");
    Ok(Some(name))
}

/// How far back from the newest sample the mic is zeroed for a dictation that started `late`
/// ago: the pre-buffer and the delay, minus the current pause, which recorded nothing. A pause
/// long enough leaves nothing recorded to zero.
pub(crate) fn omit_lookback(late: Duration, paused_for: Duration) -> Duration {
    (OMIT_LOOKBACK + late).saturating_sub(paused_for)
}

/// After a sleep the old streams rarely deliver again: both are opened anew (the output by
/// name, the mic as chosen now), retried for [`WAKE_GRACE`]. From half of it on, an
/// output that did not come back (a Bluetooth headset still pairing) gives way to the default
/// one; the new name is returned then. What the old streams held is dropped: it was captured
/// around the sleep.
fn reopen(capture: &mut Capture) -> Result<Option<String>, String> {
    let started = Instant::now();
    let deadline = started + WAKE_GRACE;
    let choice = capture.choice.as_ref();
    let (name, (system, mic)) = loop {
        let mut opened = streams(&capture.system_name, choice, true)
            .map(|streams| (capture.system_name.clone(), streams));
        if opened.is_err() && started.elapsed() >= WAKE_GRACE / 2 {
            if let Ok(default) = SystemAudio::default_name() {
                if default != capture.system_name {
                    opened = streams(&default, choice, true).map(|streams| (default, streams));
                }
            }
        }
        match opened {
            Ok(opened) => break opened,
            Err(e) if Instant::now() + WAKE_RETRY < deadline => {
                log::info!("meeting capture not back after sleep yet: {e}");
                thread::sleep(WAKE_RETRY);
            }
            Err(e) => return Err(format!("the capture did not come back after sleep: {e}")),
        }
    };
    capture
        .recorder
        .set_system_rate(system.sample_rate())
        .map_err(|e| e.to_string())?;
    if let Some((mic, _)) = &mic {
        capture
            .recorder
            .set_mic_rate(mic.sample_rate())
            .map_err(|e| e.to_string())?;
    }
    capture.system = system;
    if let Some((mic, channel)) = mic {
        capture.mic = Some(mic);
        capture.mic_channel = channel;
    }
    log::info!("meeting capture open again after sleep: {name}");
    let switched = (name != capture.system_name).then(|| name.clone());
    capture.system_name = name;
    Ok(switched)
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
    capture.system_name = name.to_string();
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

/// Moves what each stream captured into the recorder. Drained even while not writing, so a paused
/// interval is dropped instead of written later.
fn pump(
    capture: &mut Capture,
    buffer: &mut Vec<f32>,
    levels: (&mut Level, &mut Level),
    write: bool,
) -> Result<(), String> {
    buffer.clear();
    if let Some(mic) = capture.mic.as_mut() {
        mic.drain_into(buffer);
    }
    if write {
        levels.0.add(buffer);
        capture
            .recorder
            .push_mic(buffer)
            .map_err(|e| e.to_string())?;
    }
    buffer.clear();
    capture.system.drain_into(buffer);
    if write {
        levels.1.add(buffer);
        capture
            .recorder
            .push_system(buffer)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// The OS is about to sleep: what was captured up to now is written and on disk, header included.
fn suspend<A: PowerApi>(
    capture: &mut Capture,
    gate: &mut Gate<A>,
    buffer: &mut Vec<f32>,
    levels: (&mut Level, &mut Level),
) -> Result<(), String> {
    let now = Instant::now();
    let pumped = pump(capture, buffer, levels, gate.writing());
    let written = if gate.writing() {
        capture
            .recorder
            .tick(gate.elapsed(now))
            .map_err(|e| e.to_string())
    } else {
        Ok(())
    };
    gate.sleep(now);
    let flushed = capture.recorder.flush().map_err(|e| e.to_string());
    pumped.and(written).and(flushed)
}

fn run(
    mut capture: Capture,
    commands: Receiver<Control>,
    keep_awake: bool,
    on_report: impl Fn(Report),
) {
    let mut gate = Gate::new(Instant::now(), keep_awake, power::Os);
    let mut buffer = Vec::new();
    let (mut mic_level, mut system_level) = (Level::default(), Level::default());
    let mut last_report = Instant::now();
    let mut error = None;
    // A dictation is zeroing the mic now, and did at some point since the last report.
    let (mut omitting, mut omitted) = (false, false);
    let mut last_reopen: Option<Instant> = None;
    loop {
        let mut omit: Option<Option<Instant>> = None;
        let finalize = match commands.recv_timeout(TICK) {
            Ok(Control::Pause) => {
                gate.pause(Instant::now());
                false
            }
            Ok(Control::Resume) => {
                gate.resume(Instant::now());
                false
            }
            Ok(Control::SwitchSystem(name)) => {
                let paused = !gate.writing();
                let error = switch_system(&mut capture, &name, paused, &mut buffer).err();
                if let Some(e) = &error {
                    log::warn!("meeting system output not switched to {name}: {e}");
                }
                on_report(Report::SystemSwitched { name, error });
                false
            }
            Ok(Control::SwitchMic) => {
                match switch_mic(&mut capture, gate.writing(), &mut buffer) {
                    Ok(Some(name)) => on_report(Report::MicSwitched(name)),
                    Ok(None) => {}
                    Err(e) => error = Some(e),
                }
                false
            }
            Ok(Control::BeginOmit(at)) => {
                omit = Some(Some(at));
                false
            }
            Ok(Control::EndOmit) => {
                omit = Some(None);
                false
            }
            Ok(Control::Suspend(done)) => {
                let levels = (&mut mic_level, &mut system_level);
                error = suspend(&mut capture, &mut gate, &mut buffer, levels).err();
                let _ = done.send(());
                false
            }
            Ok(Control::Wake) => {
                let mic_before = capture.mic.as_ref().map(|mic| mic.name().to_string());
                match reopen(&mut capture) {
                    Ok(switched) => {
                        if let Some(name) = switched {
                            on_report(Report::SystemSwitched { name, error: None });
                        }
                        let mic_after = capture.mic.as_ref().map(|mic| mic.name().to_string());
                        if let Some(name) =
                            mic_after.filter(|after| Some(after) != mic_before.as_ref())
                        {
                            on_report(Report::MicSwitched(name));
                        }
                        gate.wake(Instant::now());
                        on_report(Report::Woke);
                    }
                    Err(e) => error = Some(e),
                }
                false
            }
            Ok(Control::Finalize) | Err(RecvTimeoutError::Disconnected) => true,
            Err(RecvTimeoutError::Timeout) => false,
        };
        let now = Instant::now();
        let writing = gate.writing();

        if error.is_none() {
            let levels = (&mut mic_level, &mut system_level);
            error = pump(&mut capture, &mut buffer, levels, writing).err();
        }
        let failed = capture.mic.as_ref().is_some_and(MicTap::failed);
        if failed && last_reopen.is_none_or(|at| now.duration_since(at) >= REOPEN_EVERY) {
            last_reopen = Some(now);
            match reopen_mic(&mut capture, writing, &mut buffer) {
                Ok(Some(name)) => on_report(Report::MicSwitched(name)),
                Ok(None) => {}
                Err(e) => error = Some(e),
            }
        }
        // After the push, so the interval is measured from the newest sample.
        match omit {
            Some(Some(at)) => {
                let late = now.saturating_duration_since(at);
                capture
                    .recorder
                    .begin_omit(omit_lookback(late, gate.paused_for(now)));
                (omitting, omitted) = (true, true);
            }
            Some(None) => {
                // Not writing, nothing more is recorded: the interval ends where the recording did.
                let margin = if writing { OMIT_MARGIN } else { Duration::ZERO };
                capture.recorder.end_omit(margin);
                omitting = false;
            }
            None => {}
        }
        let spans = capture.recorder.take_omitted();
        if !spans.is_empty() {
            on_report(Report::Omitted(omitted_ms(spans)));
        }
        if finalize || error.is_some() {
            break;
        }
        if writing {
            if let Err(e) = capture.recorder.tick(gate.elapsed(now)) {
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
                mic_omitted: std::mem::replace(&mut omitted, omitting),
            });
        }
    }

    let recorded = gate.elapsed(Instant::now());
    // The PC may sleep again from here on: the WAV is closed below either way.
    drop(gate);
    match capture.recorder.finish_omitted(recorded) {
        // A dictation still running ends with the recording.
        Ok((_, _, spans)) if !spans.is_empty() => on_report(Report::Omitted(omitted_ms(spans))),
        Ok(_) => {}
        Err(e) => {
            error.get_or_insert_with(|| e.to_string());
        }
    }
    if let Some(e) = &error {
        log::error!("meeting capture ended with an error: {e}");
    }
    on_report(Report::Finalized { recorded, error });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::power::tests::FakePower;

    #[test]
    fn the_pc_stays_awake_exactly_while_the_capture_writes() {
        let fake = FakePower::default();
        let t0 = Instant::now();
        let at = |s: u64| t0 + Duration::from_secs(s);
        {
            let mut gate = Gate::new(t0, true, fake.clone());
            assert_eq!(fake.calls(), vec![true], "start");
            gate.pause(at(10));
            gate.resume(at(20));
            gate.sleep(at(30));
            gate.wake(at(90));
            assert!(gate.writing());
            // Neither the pause nor the sleep is counted.
            assert_eq!(gate.elapsed(at(100)), Duration::from_secs(30));
        }
        // Pause, resume, sleep, wake; then the stop, or an error, drops the gate.
        assert_eq!(fake.calls(), vec![true, false, true, false, true, false]);
    }

    #[test]
    fn waking_keeps_the_pause_of_the_person() {
        let fake = FakePower::default();
        let t0 = Instant::now();
        let mut gate = Gate::new(t0, true, fake.clone());
        gate.pause(t0);
        gate.sleep(t0);
        gate.wake(t0 + Duration::from_secs(60));
        assert!(!gate.writing());
        assert_eq!(fake.calls(), vec![true, false]);
        gate.resume(t0 + Duration::from_secs(70));
        assert_eq!(fake.calls(), vec![true, false, true]);
    }

    #[test]
    fn with_the_setting_off_nothing_is_asked() {
        let fake = FakePower::default();
        let t0 = Instant::now();
        {
            let mut gate = Gate::new(t0, false, fake.clone());
            gate.pause(t0);
            gate.resume(t0);
            gate.sleep(t0);
            gate.wake(t0);
        }
        assert!(fake.calls().is_empty());
    }

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
    fn a_dictation_in_a_pause_zeroes_only_what_was_recorded() {
        let ms = Duration::from_millis;
        assert_eq!(omit_lookback(ms(0), ms(0)), OMIT_LOOKBACK);
        assert_eq!(
            omit_lookback(ms(30), ms(0)),
            ms(430),
            "a late start still covers"
        );
        assert_eq!(
            omit_lookback(ms(0), ms(150)),
            ms(250),
            "the pre-buffer reaches back past the pause"
        );
        assert_eq!(omit_lookback(ms(0), Duration::from_secs(5)), Duration::ZERO);
        let t0 = Instant::now();
        let mut clock = ActiveClock::new(t0);
        assert_eq!(
            clock.paused_for(t0 + Duration::from_secs(3)),
            Duration::ZERO
        );
        clock.pause(t0 + Duration::from_secs(3));
        assert_eq!(
            clock.paused_for(t0 + Duration::from_secs(5)),
            Duration::from_secs(2)
        );
    }

    #[test]
    fn omitted_spans_are_reported_in_ms() {
        let spans = vec![OmittedSpan {
            start: 48_000,
            end: 72_000,
        }];
        assert_eq!(omitted_ms(spans), [(1_000, 1_500)]);
    }

    #[test]
    fn recorded_ms_follows_frames() {
        assert_eq!(recorded_ms(48_000), 1_000);
        assert_eq!(recorded_ms(0), 0);
        assert_eq!(recorded_ms(48_000 * 3_600), 3_600_000);
        assert_eq!(recorded_ms(47), 0);
    }
}
