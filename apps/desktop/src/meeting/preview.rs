//! The live preview of a meeting (ADR-0019): while it records, the dictation's model transcribes
//! the stretches of speech the VAD closes in each channel, and the page shows them as they come.
//!
//! The frames come from the recorder's [`FrameTap`], the same ones that go to the WAV: the mic is
//! already zeroed while a dictation runs (ADR-0015). Dictation has priority: while one is in
//! flight the preview leaves the model alone and its stretches wait in the queue. The preview
//! lives only in memory; the transcript of record replaces it ([`discard`]). Nothing here opens a
//! connection.
//!
//! The thread starts with the recorder but loads nothing until the first frames arrive, so a
//! capture that does not open costs nothing and the start of a meeting does not wait for it.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use fala_audio::{FrameTap, LiveSegmenter, LiveUtterance, SileroVad, VoiceDetector};
use fala_meeting::SessionId;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::managers::audio::AudioRecordingManager;
use crate::managers::transcription::{ModelBusy, TranscriptionManager};
use crate::transcription_coordinator::TranscriptionCoordinator;

/// How often a waiting stretch is tried again while no frames come (paused, or after the stop).
const RETRY: Duration = Duration::from_millis(250);
/// Audio waiting for the model beyond this drops its oldest stretches (a model that never loads).
const MAX_QUEUED_MS: u64 = 120_000;
/// After the stop, how long the queue may wait for the model before it is dropped.
const DRAIN_GRACE: Duration = Duration::from_secs(60);
/// How often an unloaded model (idle timeout, "unload immediately" after a dictation) is loaded
/// again for the preview.
const RELOAD_EVERY: Duration = Duration::from_secs(10);
/// Fala's start sound plays through the speakers and comes back in the system channel: the
/// first 1.5 s of each take's system channel are not "Them" speaking.
const START_SOUND_FRAMES: usize = 72_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum PreviewChannel {
    /// "You"; in person, the room.
    Mic,
    /// "Them".
    System,
}

/// A stretch transcribed locally, in ms of the session's timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct PreviewLine {
    pub channel: PreviewChannel,
    pub t0_ms: u64,
    pub t1_ms: u64,
    pub text: String,
}

/// The preview of a session: emitted with the lines just added, and returned whole by
/// `meeting_preview` with every line so far.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, Type, Event)]
pub struct MeetingPreview {
    pub id: String,
    pub lines: Vec<PreviewLine>,
    /// Speech heard in the channel that has no text yet ("listening…").
    pub mic_listening: bool,
    pub system_listening: bool,
    /// The transcript of record replaced the preview, or the session went: the page drops it.
    pub replaced: bool,
}

/// What the app keeps of a session's preview.
struct Kept {
    preview: MeetingPreview,
    /// The VAD closed at least one stretch of speech, transcribed or not yet.
    heard: bool,
}

/// The preview of each session not yet replaced by its transcript of record.
static PREVIEWS: LazyLock<Mutex<HashMap<SessionId, Kept>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn previews() -> std::sync::MutexGuard<'static, HashMap<SessionId, Kept>> {
    PREVIEWS.lock().unwrap_or_else(|e| e.into_inner())
}

/// Every line of the session's preview so far (empty without one).
pub(crate) fn snapshot(id: SessionId) -> MeetingPreview {
    previews()
        .get(&id)
        .map(|kept| kept.preview.clone())
        .unwrap_or_else(|| MeetingPreview {
            id: id.to_string(),
            ..MeetingPreview::default()
        })
}

/// The preview heard speech in the session: what makes a stop no mistake.
pub(crate) fn heard(id: SessionId) -> bool {
    previews()
        .get(&id)
        .is_some_and(|kept| kept.heard || !kept.preview.lines.is_empty())
}

/// The transcript of record of the session was saved, or the session was deleted: its preview
/// goes, and the page is told.
pub(crate) fn discard(app: &AppHandle, id: SessionId) {
    if previews().remove(&id).is_none() {
        return;
    }
    let event = MeetingPreview {
        id: id.to_string(),
        replaced: true,
        ..MeetingPreview::default()
    };
    if let Err(e) = event.emit(app) {
        log::debug!("meeting preview not emitted: {e}");
    }
}

/// A stretch closed by the VAD, waiting for the model.
struct Job {
    channel: PreviewChannel,
    utterance: LiveUtterance,
}

/// Segmentation and queue of one session, apart from the thread, the model and the app.
pub(crate) struct Preview {
    mic: Option<LiveSegmenter>,
    system: LiveSegmenter,
    queue: VecDeque<Job>,
    /// Where the take starts on the session's timeline.
    offset_ms: u64,
    /// System frames fed so far, up to [`START_SOUND_FRAMES`].
    system_seen: usize,
    /// [`MAX_QUEUED_MS`]; lower in tests.
    max_queued_ms: u64,
}

impl Preview {
    /// `mic` is `None` in a "system only" session.
    pub(crate) fn new(
        mic: Option<Box<dyn VoiceDetector>>,
        system: Box<dyn VoiceDetector>,
        offset_ms: u64,
    ) -> Result<Self, fala_audio::AudioError> {
        Ok(Self {
            mic: mic.map(LiveSegmenter::new).transpose()?,
            system: LiveSegmenter::new(system)?,
            queue: VecDeque::new(),
            offset_ms,
            system_seen: 0,
            max_queued_ms: MAX_QUEUED_MS,
        })
    }

    /// The next frames the WAV got, both channels.
    pub(crate) fn feed(&mut self, mic: &[i16], system: &[i16]) {
        if let Some(segmenter) = self.mic.as_mut() {
            let done = segmenter.feed(mic);
            self.enqueue(PreviewChannel::Mic, done);
        }
        let skip = START_SOUND_FRAMES
            .saturating_sub(self.system_seen)
            .min(system.len());
        self.system_seen += skip;
        let done = if skip == 0 {
            self.system.feed(system)
        } else {
            let mut quiet = system.to_vec();
            quiet[..skip].fill(0);
            self.system.feed(&quiet)
        };
        self.enqueue(PreviewChannel::System, done);
    }

    /// The recording ended: the stretches still open are closed.
    pub(crate) fn finish(&mut self) {
        if let Some(segmenter) = self.mic.as_mut() {
            let done = segmenter.finish();
            self.enqueue(PreviewChannel::Mic, done);
        }
        let done = self.system.finish();
        self.enqueue(PreviewChannel::System, done);
    }

    fn enqueue(
        &mut self,
        channel: PreviewChannel,
        done: Result<Vec<LiveUtterance>, fala_audio::AudioError>,
    ) {
        match done {
            Ok(done) => self
                .queue
                .extend(done.into_iter().map(|utterance| Job { channel, utterance })),
            Err(e) => log::warn!("meeting preview: {channel:?} not segmented: {e}"),
        }
        let mut dropped = 0;
        while self.queued_ms() > self.max_queued_ms {
            self.queue.pop_front();
            dropped += 1;
        }
        if dropped > 0 {
            log::warn!("meeting preview: the model is not keeping up; {dropped} stretches dropped");
        }
    }

    fn queued_ms(&self) -> u64 {
        self.queue
            .iter()
            .map(|job| job.utterance.t1_ms - job.utterance.t0_ms)
            .sum()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Speech heard in each channel without text yet: an open stretch or one in the queue.
    pub(crate) fn listening(&self) -> (bool, bool) {
        let queued = |channel| self.queue.iter().any(|job| job.channel == channel);
        (
            self.mic.as_ref().is_some_and(LiveSegmenter::speaking) || queued(PreviewChannel::Mic),
            self.system.speaking() || queued(PreviewChannel::System),
        )
    }

    /// Transcribes the oldest stretch unless `dictating`. `Ok(None)` when the model said nothing
    /// worth a line; `Err(ModelBusy)` leaves the stretch in the queue for later.
    pub(crate) fn step(
        &mut self,
        dictating: bool,
        transcribe: &mut dyn FnMut(&[f32]) -> anyhow::Result<String>,
    ) -> Result<Option<PreviewLine>, ModelBusy> {
        if dictating {
            return Err(ModelBusy);
        }
        let Some(job) = self.queue.front() else {
            return Ok(None);
        };
        let text = match transcribe(&job.utterance.samples) {
            Err(e) if e.is::<ModelBusy>() => return Err(ModelBusy),
            Err(e) => {
                log::warn!("meeting preview: a stretch was not transcribed: {e}");
                String::new()
            }
            Ok(text) => text,
        };
        let job = self.queue.pop_front().ok_or(ModelBusy)?;
        let text = text.trim();
        Ok((!text.is_empty()).then(|| PreviewLine {
            channel: job.channel,
            t0_ms: self.offset_ms + job.utterance.t0_ms,
            t1_ms: self.offset_ms + job.utterance.t1_ms,
            text: text.to_string(),
        }))
    }
}

/// What the recorder's tap sends: the next frames of each channel.
type Frames = (Vec<i16>, Vec<i16>);

/// Starts the preview of `id` on its own thread and returns the tap for the recorder. The thread
/// loads the VAD once the first frames arrive and ends once the tap is dropped (the recording
/// ended) and the queue is empty; without a VAD the meeting records without a preview.
pub(crate) fn start(
    app: &AppHandle,
    id: SessionId,
    with_mic: bool,
    offset_ms: u64,
) -> Option<FrameTap> {
    let (tx, rx) = mpsc::channel::<Frames>();
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("meeting-preview".to_string())
        .spawn(move || {
            // Nothing recorded (the capture did not open): nothing to load.
            let Ok(first) = rx.recv() else {
                return;
            };
            match open(&app, with_mic, offset_ms) {
                Ok(preview) => run(&app, id, preview, first, &rx),
                Err(e) => log::warn!("meeting {id}: no live preview: {e}"),
            }
        });
    if let Err(e) = spawned {
        log::warn!("meeting {id}: preview thread did not start: {e}");
        return None;
    }
    Some(Box::new(move |_first, mic: &[i16], system: &[i16]| {
        let _ = tx.send((mic.to_vec(), system.to_vec()));
    }))
}

/// One Silero per channel; the dictation model starts loading if it is not loaded.
fn open(app: &AppHandle, with_mic: bool, offset_ms: u64) -> anyhow::Result<Preview> {
    let vad = || -> anyhow::Result<Box<dyn VoiceDetector>> {
        let path: PathBuf = app.path().resolve(
            "resources/models/silero_vad_v4.onnx",
            tauri::path::BaseDirectory::Resource,
        )?;
        Ok(Box::new(SileroVad::load(&path)?))
    };
    let mic = with_mic.then(vad).transpose()?;
    let preview = Preview::new(mic, vad()?, offset_ms)?;
    if let Some(tm) = app.try_state::<Arc<TranscriptionManager>>() {
        tm.initiate_model_load();
    }
    Ok(preview)
}

/// A dictation is in flight, from the key to the end of its transcription.
fn dictating(app: &AppHandle) -> bool {
    app.try_state::<TranscriptionCoordinator>()
        .is_some_and(|c| c.is_busy())
        || app
            .try_state::<Arc<AudioRecordingManager>>()
            .is_some_and(|rm| rm.is_recording())
}

fn run(
    app: &AppHandle,
    id: SessionId,
    mut preview: Preview,
    (mic, system): Frames,
    frames: &Receiver<Frames>,
) {
    // A resumed session keeps the lines of its earlier takes until its transcript arrives.
    let earlier = snapshot(id);
    previews().entry(id).or_insert(Kept {
        preview: earlier,
        heard: false,
    });
    let tm = app
        .try_state::<Arc<TranscriptionManager>>()
        .map(|tm| Arc::clone(&tm));
    let mut transcribe = |audio: &[f32]| match &tm {
        Some(tm) => tm.transcribe_preview(audio),
        None => Err(ModelBusy.into()),
    };
    preview.feed(&mic, &system);
    let mut listening = (false, false);
    let mut heard = false;
    let mut ended: Option<Instant> = None;
    let mut last_load = Instant::now();
    loop {
        if ended.is_none() {
            match frames.recv_timeout(RETRY) {
                Ok((mic, system)) => preview.feed(&mic, &system),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    preview.finish();
                    ended = Some(Instant::now());
                }
            }
        }
        if !heard && !preview.is_empty() {
            heard = true;
            if let Some(kept) = previews().get_mut(&id) {
                kept.heard = true;
            }
        }
        let dictation = dictating(app);
        let line = match preview.step(dictation, &mut transcribe) {
            Ok(line) => line,
            Err(ModelBusy) => {
                if ended.is_some_and(|at| at.elapsed() >= DRAIN_GRACE) {
                    log::warn!("meeting {id}: preview ended with stretches the model never took");
                    break;
                }
                // Unloaded since (idle, or "unload immediately" after a dictation): load again.
                if let Some(tm) = tm.as_ref().filter(|tm| !dictation && !tm.is_model_loaded()) {
                    if last_load.elapsed() >= RELOAD_EVERY {
                        last_load = Instant::now();
                        tm.initiate_model_load();
                    }
                }
                if ended.is_some() {
                    std::thread::sleep(RETRY);
                }
                None
            }
        };
        let now = preview.listening();
        if line.is_some() || now != listening {
            listening = now;
            publish(app, id, line, now);
        }
        if ended.is_some() && preview.is_empty() {
            break;
        }
    }
    publish(app, id, None, (false, false));
    if let Some(tm) = &tm {
        tm.maybe_unload_immediately("meeting preview");
    }
    log::info!("meeting {id}: live preview ended");
}

/// Adds `line` to the session's preview and tells the page.
fn publish(app: &AppHandle, id: SessionId, line: Option<PreviewLine>, (mic, system): (bool, bool)) {
    {
        let mut all = previews();
        // Gone means the transcript of record already replaced it.
        let Some(kept) = all.get_mut(&id) else {
            return;
        };
        kept.preview.lines.extend(line.clone());
        kept.preview.mic_listening = mic;
        kept.preview.system_listening = system;
    }
    let event = MeetingPreview {
        id: id.to_string(),
        lines: line.into_iter().collect(),
        mic_listening: mic,
        system_listening: system,
        replaced: false,
    };
    if let Err(e) = event.emit(app) {
        log::debug!("meeting preview not emitted: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fala_audio::AudioError;

    /// Voice when the frame has energy.
    struct LoudVad;

    impl VoiceDetector for LoudVad {
        fn is_voice(&mut self, frame: &[f32]) -> Result<bool, AudioError> {
            Ok(frame.iter().map(|s| s.abs()).sum::<f32>() / frame.len() as f32 > 0.1)
        }
        fn reset(&mut self) {}
    }

    fn preview(with_mic: bool, offset_ms: u64) -> Preview {
        let mic = with_mic.then(|| Box::new(LoudVad) as Box<dyn VoiceDetector>);
        Preview::new(mic, Box::new(LoudVad), offset_ms).unwrap()
    }

    /// `ms` of 48 kHz: a loud tone or digital zero.
    fn audio(ms: usize, voice: bool) -> Vec<i16> {
        (0..ms * 48)
            .map(|i| match (voice, (i / 24) % 2) {
                (false, _) => 0,
                (true, 0) => 16_000,
                (true, _) => -16_000,
            })
            .collect()
    }

    /// Feeds both channels in 100 ms batches, as the recorder does.
    fn feed(p: &mut Preview, mic: &[i16], system: &[i16]) {
        for (m, s) in mic.chunks(4_800).zip(system.chunks(4_800)) {
            p.feed(m, s);
        }
    }

    fn phrase(before_ms: usize) -> Vec<i16> {
        let mut out = audio(before_ms, false);
        out.extend(audio(1_000, true));
        out.extend(audio(1_000, false));
        out
    }

    #[test]
    fn each_channel_becomes_its_lines_on_the_session_timeline() {
        let mut p = preview(true, 60_000);
        // The start sound in the system channel, then digital silence (nothing plays, or the
        // mic zeroed by a dictation): nothing.
        let zeros = vec![0; 48 * 3_000];
        let mut start = audio(1_000, true);
        start.resize(zeros.len(), 0);
        feed(&mut p, &zeros, &start);
        assert!(!p.system.speaking(), "the start sound is not speech");
        assert!(p.is_empty());
        // Them at ~0.5 s, you at ~2 s.
        let mic = phrase(2_000);
        let mut system = phrase(500);
        system.resize(mic.len(), 0);
        feed(&mut p, &mic, &system);
        p.finish();
        let mut said = |samples: &[f32]| Ok(format!(" {} samples ", samples.len()));
        let mut lines = Vec::new();
        while !p.is_empty() {
            lines.extend(p.step(false, &mut said).unwrap());
        }
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert_eq!(lines[0].channel, PreviewChannel::System);
        assert_eq!(lines[1].channel, PreviewChannel::Mic);
        assert!((63_000..=63_500).contains(&lines[0].t0_ms));
        assert!((64_400..=65_000).contains(&lines[1].t0_ms));
        assert!(!lines[0].text.starts_with(' '), "trimmed");
    }

    #[test]
    fn a_dictation_holds_the_queue_and_it_goes_on_in_order_after() {
        let mut p = preview(false, 0);
        let mut system = phrase(1_500);
        system.extend(phrase(0));
        feed(&mut p, &vec![0; system.len()], &system);
        let mut calls = 0;
        let mut said = |_: &[f32]| {
            calls += 1;
            Ok(format!("line {calls}"))
        };
        assert!(
            p.step(true, &mut said).is_err(),
            "dictating: the model is left alone"
        );
        assert!(p.step(true, &mut said).is_err());
        assert_eq!(
            p.listening(),
            (false, true),
            "waiting stretches still show as heard"
        );
        let first = p.step(false, &mut said).unwrap().unwrap();
        let second = p.step(false, &mut said).unwrap().unwrap();
        assert_eq!(
            (first.text.as_str(), second.text.as_str()),
            ("line 1", "line 2")
        );
        assert!(first.t0_ms < second.t0_ms);
        assert!(p.is_empty());
        assert_eq!(p.listening(), (false, false));
    }

    #[test]
    fn a_busy_model_keeps_the_stretch_and_a_failure_drops_it() {
        let mut p = preview(false, 0);
        let system = phrase(1_500);
        feed(&mut p, &vec![0; system.len()], &system);
        let mut busy = |_: &[f32]| Err(ModelBusy.into());
        assert!(p.step(false, &mut busy).is_err());
        assert!(!p.is_empty(), "kept for later");
        let mut broken = |_: &[f32]| Err(anyhow::anyhow!("engine error"));
        assert_eq!(p.step(false, &mut broken).unwrap(), None);
        assert!(p.is_empty(), "a failed stretch is not tried forever");
    }

    #[test]
    fn a_model_that_never_comes_drops_the_oldest_audio() {
        let mut p = preview(false, 0);
        p.max_queued_ms = 20_000;
        // 40 s of speech: cut into 8 s stretches, more than the queue keeps.
        let system = audio(40_000, true);
        feed(&mut p, &vec![0; system.len()], &system);
        assert!(p.queued_ms() <= 20_000);
        assert!(
            p.queue.front().unwrap().utterance.t0_ms > 0,
            "the oldest went first"
        );
    }
}
