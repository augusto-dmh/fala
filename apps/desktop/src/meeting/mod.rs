//! Meeting recording in the desktop shell (phase 2, F2 `meeting-panel`).
//!
//! `MeetingManager` is a facade: the session rules live in `fala-meeting` (start only on an
//! explicit `UserAction`, indicator, cap, silence), the capture in `fala-audio`, the retained
//! audio in `fala-retention` and the rows in `fala-storage`. This module only runs the effects the
//! session returns and reports them to the UI (ADR-0002).
//!
//! ADR-0005: `UserAction::StartRecording` and `ResumeRecording` are built only in
//! [`MeetingManager::start`] and [`MeetingManager::resume`], which only the meeting commands and
//! the tray items call. ADR-0015: dictation and the recorder read one stream per device
//! (`MIC_HUB`), and a dictation during the recording zeroes the meeting's mic channel for its
//! interval ([`MeetingManager::dictation_changed`]). In-person mode still refuses dictation.

pub(crate) mod clip;
pub(crate) mod context;
mod disk;
pub(crate) mod erase;
pub(crate) mod granola;
pub(crate) mod import;
pub(crate) mod lifecycle;
pub(crate) mod output;
pub(crate) mod people;
pub(crate) mod pipeline;
pub(crate) mod recorder;
pub(crate) mod recovery;
pub(crate) mod search;
pub(crate) mod series;
pub(crate) mod sleeps;
pub(crate) mod sound_check;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use chrono::{DateTime, FixedOffset, Local, SecondsFormat};
use fala_meeting::{
    Effect, GapKind, IndicatorKind, Input, Levels, MeetingSession, MuteWatch, RecordingCap,
    SessionConfig, SessionError, SessionId, SessionMode, SessionState, StopReason, UnixMillis,
    UserAction,
};
use fala_storage::{MeetingTake, NewMeeting, StorageError, Store};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Emitter, Manager};
use tauri_specta::Event;

use crate::audio_feedback::{self, SoundType};
use crate::managers::audio::AudioRecordingManager;
use crate::media_pause::{self, MediaAction, Owner};
use crate::power::SleepEvent;
use recorder::{Control, Report};

/// The working WAV inside `audio/<id>/`; `fala-retention` turns it into `mic.opus` and
/// `sys.opus` and deletes it once both validate (ADR-0014).
pub(crate) const WORK_WAV: &str = "recording.wav";

/// Event with the id of a session deleted or moved to the trash (a card's [Discard] included).
pub(crate) const DELETED_EVENT: &str = "meeting-deleted";

/// Event the tray emits when "Record meeting" is clicked before the third-party notice was
/// accepted: the window opens on the Meetings page with the notice instead of recording.
pub(crate) const CONSENT_REQUIRED_EVENT: &str = "meeting-consent-required";

/// The capture modes the panel offers (part 1 of F2, and "system only" of F7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum MeetingMode {
    /// A call with headphones: the mic is "me", the system is "them".
    Meeting,
    /// Everyone in the room: the mic is diarized too.
    InPerson,
    /// What the computer plays (a class, a video): no mic, diarized system channel.
    SystemOnly,
}

impl MeetingMode {
    fn session_mode(self) -> SessionMode {
        match self {
            MeetingMode::Meeting => SessionMode::Meeting,
            MeetingMode::InPerson => SessionMode::InPerson,
            MeetingMode::SystemOnly => SessionMode::SystemOnly,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum MeetingPhase {
    #[default]
    Idle,
    Recording,
    Paused,
    Stopping,
    /// The last session is being turned into retained audio (and, later, a transcript).
    Processing,
}

/// What the UI shows about the current recording; emitted on every change and at 4 Hz while
/// recording.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, Type, Event)]
pub struct MeetingStatus {
    pub state: MeetingPhase,
    pub id: Option<String>,
    pub mode: Option<MeetingMode>,
    /// Recorded time without pauses, from the frames written to the WAV.
    pub recorded_ms: u64,
    pub mic_level: f32,
    pub system_level: f32,
    pub muted_mic: bool,
    pub muted_system: bool,
    /// Set once the session warns that the cap is near; "one more hour" clears it.
    pub cap_remaining_ms: Option<u64>,
    /// Free bytes when recording started with less than 2 GiB free.
    pub low_disk_bytes: Option<u64>,
    /// The output whose sound is the system channel.
    pub system_output: Option<String>,
    /// Another output that plays while the system channel records zeros: the call is there.
    pub call_output: Option<String>,
    /// A channel in digital zero for ~3 s (the pill's dot turns amber).
    pub zero_mic: bool,
    pub zero_system: bool,
    /// A dictation is zeroing the meeting's mic channel now (ADR-0015).
    pub dictating: bool,
}

/// Errors of the meeting commands, serialized as `{"kind": "...", "detail": ...}` so the UI
/// can tell them apart without matching text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum MeetingError {
    ConsentRequired,
    AlreadyActive,
    AlreadyStarted,
    DictationActive,
    InsufficientDisk,
    AudioDevice(String),
    NotActive,
    NotFound,
    InvalidTransition,
    CapAtMaximum,
    Storage(String),
    /// The session is still recording; stop it first.
    StillRecording,
    /// No retained audio (and no working WAV) for the session.
    NoAudio,
    /// The provider key is not in the keyring.
    MissingKey,
    InvalidKey,
    Keyring,
    Transcription(String),
    Cancelled,
    /// A transcription or a notes generation is already running.
    Busy,
    UnknownTemplate,
    Notes(String),
    /// The notes were edited by hand: regenerating needs the user's confirmation.
    NotesEdited,
    /// `ffmpeg`, `ffprobe` or `yt-dlp` is not on the PATH.
    ToolMissing(String),
    /// Only `http://` and `https://` URLs are imported.
    InvalidUrl,
    /// `yt-dlp` did not download the URL; the page suggests downloading and importing the file.
    Download(String),
    /// The file could not be turned into audio; the detail is the tool's last line or the path.
    Import(String),
    /// The imported file has no audio track.
    ImportNoAudio,
    /// The imported source is longer than `fala_media::MAX_DURATION`.
    ImportTooLong,
    /// More than `erase::MAX_LINES` lines in one stretch to erase.
    EraseTooLong,
}

impl std::fmt::Display for MeetingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeetingError::AudioDevice(detail)
            | MeetingError::Storage(detail)
            | MeetingError::Transcription(detail)
            | MeetingError::Notes(detail)
            | MeetingError::ToolMissing(detail)
            | MeetingError::Download(detail)
            | MeetingError::Import(detail) => {
                write!(f, "{self:?}: {detail}")
            }
            other => write!(f, "{other:?}"),
        }
    }
}

impl From<StorageError> for MeetingError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::NotFound(_) => MeetingError::NotFound,
            StorageError::MeetingAlreadyStarted(_) => MeetingError::AlreadyStarted,
            StorageError::NotResumable(_) => MeetingError::InvalidTransition,
            other => MeetingError::Storage(other.to_string()),
        }
    }
}

impl From<SessionError> for MeetingError {
    fn from(error: SessionError) -> Self {
        match error {
            SessionError::InsufficientDisk { .. } => MeetingError::InsufficientDisk,
            SessionError::CapAtMaximum => MeetingError::CapAtMaximum,
            _ => MeetingError::InvalidTransition,
        }
    }
}

/// For this long after each start, the page and the pill remind that the others must agree to be
/// recorded (Granola shows the same for 5 s on every recording); the notice of ADR-0005 stays.
pub(crate) const CONSENT_CHIP_MS: u64 = 5_000;

/// The pill shows a card over it: the call plays elsewhere, or the consent chip is on.
pub(crate) fn meeting_card(status: &MeetingStatus) -> bool {
    status.call_output.is_some() || status.recorded_ms < CONSENT_CHIP_MS
}

/// Who may start recording, in the order the reasons are reported.
pub(crate) fn check_start(
    consent: Option<&str>,
    dictation_recording: bool,
    active: bool,
) -> Result<(), MeetingError> {
    if consent.is_none() {
        return Err(MeetingError::ConsentRequired);
    }
    if active {
        return Err(MeetingError::AlreadyActive);
    }
    if dictation_recording {
        return Err(MeetingError::DictationActive);
    }
    Ok(())
}

/// The consent date as stored in settings: RFC 3339 with the local offset, in seconds.
pub(crate) fn consent_stamp(at: DateTime<FixedOffset>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, false)
}

/// The sound a session effect plays when feedback sounds are on. It reinforces the indicator and
/// never replaces it (ADR-0005).
pub(crate) fn meeting_sound(effect: &Effect) -> Option<SoundType> {
    match effect {
        Effect::StartCapture => Some(SoundType::MeetingStart),
        Effect::PauseCapture => Some(SoundType::MeetingPause),
        Effect::ResumeCapture => Some(SoundType::MeetingResume),
        _ => None,
    }
}

/// What a session effect does to the media playing on the PC: paused exactly while the capture
/// records, so music never leaks into the system channel.
pub(crate) fn media_action(effect: &Effect) -> Option<MediaAction> {
    match effect {
        Effect::StartCapture | Effect::ResumeCapture => Some(MediaAction::Pause),
        Effect::PauseCapture | Effect::FinalizeCapture | Effect::HideIndicator => {
            Some(MediaAction::Resume)
        }
        _ => None,
    }
}

/// The `recording-error` type that refuses a dictation during a meeting, if any. Only in-person
/// mode refuses: there the mic is the meeting, and zeroing it loses what the room says.
/// ADR-0015 allows it with the same zeroing; this stays blocked until that is confirmed.
pub(crate) fn dictation_refusal(
    indicator: MeetingIndicator,
    in_person: bool,
) -> Option<&'static str> {
    match indicator {
        MeetingIndicator::Recording | MeetingIndicator::Paused if in_person => {
            Some("meeting_in_person")
        }
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrayStart {
    /// Show the window with the third-party notice; nothing records.
    AskConsent,
    Start,
}

pub(crate) fn tray_start(consent: Option<&str>) -> TrayStart {
    if consent.is_some() {
        TrayStart::Start
    } else {
        TrayStart::AskConsent
    }
}

/// How long the suspend handler waits for the WAV header to reach the disk; Windows gives it
/// about two seconds before the PC sleeps.
const SUSPEND_GRACE: Duration = Duration::from_millis(1_500);

/// How long "Stop and quit" waits for the capture to close before quitting anyway.
const QUIT_GRACE: Duration = Duration::from_secs(10);

/// Quitting with a meeting recording or paused asks first: closing Fala cuts the recording.
pub(crate) fn quit_needs_question(indicator: MeetingIndicator) -> bool {
    indicator != MeetingIndicator::None
}

/// The tray's "Quit": asks with the card instead of quitting while a meeting records. Returns
/// whether it asked.
pub(crate) fn ask_before_quit(app: &AppHandle) -> bool {
    if !quit_needs_question(indicator()) {
        return false;
    }
    crate::call_prompt::show_quit(app);
    true
}

/// Which session a start records into: the draft the page created (title and agenda typed
/// before recording), or a new id. `true` when the session row already exists.
pub(crate) fn start_target(
    store: &Store,
    draft: Option<SessionId>,
    now: UnixMillis,
) -> Result<(SessionId, bool), MeetingError> {
    match draft {
        Some(id) => {
            if store.meeting(id)?.started_at.is_some() {
                return Err(MeetingError::AlreadyStarted);
            }
            Ok((id, true))
        }
        None => SessionId::generate(now.0)
            .map(|id| (id, false))
            .map_err(|e| MeetingError::Storage(e.to_string())),
    }
}

/// What "Retomar" records into: the session's capture mode and the next take number. Only a
/// finished, recorded session that is not an import nor in the trash, with no take left open.
pub(crate) fn resume_target(
    store: &Store,
    id: SessionId,
) -> Result<(MeetingMode, u32), MeetingError> {
    let record = store.meeting(id)?;
    let takes = store.meeting_takes(id)?;
    let mode = match record.mode {
        SessionMode::Meeting => MeetingMode::Meeting,
        SessionMode::InPerson => MeetingMode::InPerson,
        SessionMode::SystemOnly => MeetingMode::SystemOnly,
        SessionMode::Import => return Err(MeetingError::InvalidTransition),
    };
    if record.ended_at.is_none()
        || store.meeting_trashed_at(id)?.is_some()
        || takes.iter().any(|t| t.ended_at.is_none())
    {
        return Err(MeetingError::InvalidTransition);
    }
    Ok((mode, takes.last().map_or(2, |t| t.take + 1)))
}

/// Why a finished session stopped.
pub(crate) fn stop_reason(state: SessionState) -> StopReason {
    match state {
        SessionState::Stopping { reason } | SessionState::Stopped { reason } => reason,
        _ => StopReason::User,
    }
}

/// What the pill and the tray show for the meeting. Read lock-free by the tray and the overlay,
/// which must never wait on the manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MeetingIndicator {
    None = 0,
    Recording = 1,
    Paused = 2,
}

static INDICATOR: AtomicU8 = AtomicU8::new(0);
/// The live session records in person; read lock-free by the dictation shortcut.
static IN_PERSON: AtomicBool = AtomicBool::new(false);

pub(crate) fn in_person() -> bool {
    IN_PERSON.load(Ordering::SeqCst)
}

pub(crate) fn indicator() -> MeetingIndicator {
    match INDICATOR.load(Ordering::SeqCst) {
        1 => MeetingIndicator::Recording,
        2 => MeetingIndicator::Paused,
        _ => MeetingIndicator::None,
    }
}

fn show_indicator(app: &AppHandle, kind: IndicatorKind) {
    let value = match kind {
        IndicatorKind::Recording => MeetingIndicator::Recording,
        // A suspended session waits like a paused one.
        IndicatorKind::Paused | IndicatorKind::Suspended => MeetingIndicator::Paused,
    };
    INDICATOR.store(value as u8, Ordering::SeqCst);
    crate::overlay::show_meeting_overlay(app);
    crate::tray::refresh_tray_icon(app);
}

fn hide_indicator(app: &AppHandle) {
    INDICATOR.store(MeetingIndicator::None as u8, Ordering::SeqCst);
    IN_PERSON.store(false, Ordering::SeqCst);
    crate::overlay::hide_meeting_overlay(app);
    crate::tray::refresh_tray_icon(app);
}

fn now_local() -> DateTime<FixedOffset> {
    Local::now().fixed_offset()
}

fn millis(at: DateTime<FixedOffset>) -> UnixMillis {
    UnixMillis(u64::try_from(at.timestamp_millis()).unwrap_or(0))
}

fn parse_id(id: &str) -> Result<SessionId, MeetingError> {
    id.parse().map_err(|_| MeetingError::NotFound)
}

struct Active {
    session: MeetingSession,
    mode: MeetingMode,
    control: Sender<Control>,
    mute: MuteWatch,
    route: output::Route,
    /// Recorded time at the last `Tick` given to the session (one per second).
    last_tick: Duration,
    /// Loudest level of each channel since the last `Tick`.
    tick_levels: Levels,
    /// Recorded time with speech (`lifecycle::SPEECH_RMS`), for "Recorded by mistake?".
    voiced: Duration,
    /// The capture ended on a device or disk error, not on a stop.
    capture_failed: bool,
    /// A dictation is zeroing the mic channel (`BeginOmit` sent, `EndOmit` not yet).
    dictating: bool,
    /// The PC woke and the devices are being opened again.
    waking: bool,
    /// The devices did not come back after a sleep: nobody stopped the session.
    interrupted: bool,
    /// A take of a resumed session: its spans and sleeps move by its offset, and its stop
    /// closes the take.
    take: Option<MeetingTake>,
}

#[derive(Default)]
struct Inner {
    active: Option<Active>,
    /// Stopped sessions still being finished (end saved, audio retained).
    processing: Vec<SessionId>,
    status: MeetingStatus,
    /// Answered once the recorder put the WAV header on disk before a sleep.
    suspend_ack: Option<Receiver<()>>,
}

pub struct MeetingManager {
    app: AppHandle,
    audio_root: PathBuf,
    store: Mutex<Option<Store>>,
    inner: Mutex<Inner>,
    /// The OS keyring (ADR-0008).
    keys: Arc<dyn fala_secrets::SecretStore>,
    /// The transcription or notes generation in flight, one at a time.
    work: Mutex<pipeline::Work>,
    /// Held for the whole WAV → Opus conversion, and while audio is deleted.
    retaining: Mutex<()>,
    /// The import in flight (a file, a URL or a Granola export), one at a time.
    importing: Mutex<Option<fala_media::CancelToken>>,
    /// Recordings the last run left without an end, found at startup (`recovery`).
    orphans: Mutex<Vec<SessionId>>,
    /// "Stop and quit": the app exits once the stopped session's end is saved.
    quit_after_stop: AtomicBool,
}

impl MeetingManager {
    pub fn new(app: &AppHandle) -> Result<Self, anyhow::Error> {
        let data = crate::portable::app_data_dir(app)?;
        let db = data.join("fala.sqlite");
        let store = match Store::open(&db, &data.join("notas")) {
            Ok(store) => Some(store),
            Err(e) => {
                log::error!(
                    "fala.sqlite unavailable at {}: {e}; meetings cannot be saved",
                    db.display()
                );
                None
            }
        };
        // Before anything can record: afterwards an unended session may be the live one.
        let audio_root = data.join("audio");
        let orphans = match &store {
            Some(store) => store
                .meetings()
                .map(|records| recovery::find_orphans(&records, &audio_root))
                .unwrap_or_else(|e| {
                    log::error!("meetings not read for orphans: {e}");
                    Vec::new()
                }),
            None => Vec::new(),
        };
        if !orphans.is_empty() {
            log::info!("{} meeting recording(s) left without an end", orphans.len());
        }
        Ok(Self {
            app: app.clone(),
            audio_root,
            store: Mutex::new(store),
            inner: Mutex::new(Inner::default()),
            keys: Arc::new(fala_secrets::KeyringStore),
            work: Mutex::new(pipeline::Work::default()),
            retaining: Mutex::new(()),
            importing: Mutex::new(None),
            orphans: Mutex::new(orphans),
            quit_after_stop: AtomicBool::new(false),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Runs `f` with the store. Never called while holding the store and then taking `inner`.
    fn with_store<T>(
        &self,
        f: impl FnOnce(&mut Store) -> Result<T, MeetingError>,
    ) -> Result<T, MeetingError> {
        let mut guard = self.store.lock().unwrap_or_else(|e| e.into_inner());
        let store = guard
            .as_mut()
            .ok_or_else(|| MeetingError::Storage("fala.sqlite is unavailable".to_string()))?;
        f(store)
    }

    fn session_dir(&self, id: SessionId) -> PathBuf {
        fala_retention::session_audio_dir(&self.audio_root, id)
    }

    pub fn status(&self) -> MeetingStatus {
        self.lock().status.clone()
    }

    /// The session records, or stopped and is still being finished.
    pub(crate) fn is_live(&self, id: SessionId) -> bool {
        let inner = self.lock();
        inner.active.as_ref().is_some_and(|a| a.session.id() == id)
            || inner.processing.contains(&id)
    }

    /// Records that the third-party notice was accepted, once; returns the stored date.
    pub fn accept_consent(&self) -> String {
        let mut settings = crate::settings::get_settings(&self.app);
        if let Some(at) = settings.meeting_consent_accepted_at.clone() {
            return at;
        }
        let at = consent_stamp(now_local());
        settings.meeting_consent_accepted_at = Some(at.clone());
        crate::settings::write_settings(&self.app, settings);
        let _ = self.app.emit(
            "settings-changed",
            serde_json::json!({ "setting": "meeting_consent_accepted_at", "value": at }),
        );
        log::info!("meeting third-party notice accepted at {at}");
        at
    }

    /// A session that does not record yet: title and agenda typed before the call.
    pub fn create_draft(&self, title: &str) -> Result<String, MeetingError> {
        let now = now_local();
        let id =
            SessionId::generate(millis(now).0).map_err(|e| MeetingError::Storage(e.to_string()))?;
        self.with_store(|store| {
            store.create_meeting(&NewMeeting {
                id,
                title: title.to_string(),
                mode: SessionMode::Meeting,
                local_only: false,
                created_at: now,
            })?;
            Ok(())
        })?;
        Ok(id.to_string())
    }

    pub fn set_title(&self, id: &str, title: &str) -> Result<(), MeetingError> {
        let id = parse_id(id)?;
        self.with_store(|store| Ok(store.set_meeting_title(id, title)?))
    }

    /// Replaces the notes typed for the session (before or during the recording).
    pub fn save_annotations(&self, id: &str, text: &str) -> Result<(), MeetingError> {
        let id = parse_id(id)?;
        self.with_store(|store| Ok(store.save_meeting_annotations(id, text)?))
    }

    /// "Record meeting": the only place a capture starts (ADR-0005).
    pub fn start(
        &self,
        mode: MeetingMode,
        title: &str,
        draft: Option<&str>,
    ) -> Result<MeetingStatus, MeetingError> {
        let draft = draft.map(parse_id).transpose()?;
        self.begin(mode, title, draft, None)
    }

    /// "Retomar" on a finished session: the next take, by the same path as a start (ADR-0005:
    /// only a click, with the third-party notice and the indicator). The take records in
    /// `audio/<id>/t<N>/` and opens in the store once the capture is open.
    pub fn resume_session(&self, id: &str) -> Result<MeetingStatus, MeetingError> {
        let id = parse_id(id)?;
        // The last stop of this session is still being retained or transcribed: its transcript
        // would be lost to the new take.
        if self
            .work
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .involves(id)
        {
            return Err(MeetingError::Busy);
        }
        let (mode, take) = self.with_store(|store| resume_target(store, id))?;
        self.begin(mode, "", None, Some((id, take)))
    }

    /// A start: into a draft or a new session, or (`resume`: the session and the take number)
    /// a take of a finished one.
    fn begin(
        &self,
        mode: MeetingMode,
        title: &str,
        draft: Option<SessionId>,
        resume: Option<(SessionId, u32)>,
    ) -> Result<MeetingStatus, MeetingError> {
        let settings = crate::settings::get_settings(&self.app);
        let consent = settings.meeting_consent_accepted_at;
        let dictation = self
            .app
            .try_state::<Arc<AudioRecordingManager>>()
            .is_some_and(|rm| rm.is_recording());
        let mut inner = self.lock();
        check_start(consent.as_deref(), dictation, inner.active.is_some())?;
        if resume.is_some_and(|(id, _)| inner.processing.contains(&id)) {
            return Err(MeetingError::Busy);
        }

        let now = now_local();
        let (id, existing) = match resume {
            Some((id, _)) => (id, true),
            None => self.with_store(|store| start_target(store, draft, millis(now)))?,
        };
        let dir = match resume {
            Some((_, take)) => self.session_dir(id).join(format!("t{take}")),
            None => self.session_dir(id),
        };
        std::fs::create_dir_all(&dir)
            .map_err(|e| MeetingError::Storage(format!("{}: {e}", dir.display())))?;
        let free = disk::free_bytes(&dir).unwrap_or_else(|| {
            log::warn!("free disk space unknown at {}", dir.display());
            u64::MAX
        });

        let mut session = MeetingSession::new(SessionConfig {
            id,
            mode: mode.session_mode(),
            title: title.to_string(),
            local_only: false,
            cap: RecordingCap::from_hours(settings.meeting_cap_hours).unwrap_or_default(),
        });
        let effects = session.apply(
            millis(now),
            Input::User(UserAction::StartRecording {
                free_disk_bytes: free,
            }),
        )?;

        let system = output::resolve(settings.meeting_output_device.as_deref())?;
        IN_PERSON.store(mode == MeetingMode::InPerson, Ordering::SeqCst);
        let mut control = None;
        let mut low_disk = None;
        for effect in effects {
            match effect {
                Effect::WarnLowDisk { free_bytes } => low_disk = Some(free_bytes),
                Effect::ShowIndicator(kind) => show_indicator(&self.app, kind),
                Effect::StartCapture => {
                    let app = self.app.clone();
                    let sink = move |report| {
                        if let Some(manager) = app.try_state::<Arc<MeetingManager>>() {
                            manager.on_report(report);
                        }
                    };
                    let with_mic = mode.session_mode().has_mic();
                    let wav = dir.join(WORK_WAV);
                    let awake = settings.meeting_keep_awake;
                    match recorder::spawn(system.clone(), with_mic, wav, awake, sink) {
                        Ok(tx) => {
                            control = Some(tx);
                            self.feedback(&Effect::StartCapture);
                        }
                        Err(e) => {
                            hide_indicator(&self.app);
                            // Nothing was recorded: leave no empty `audio/<id>/` behind (only
                            // removes an empty directory).
                            let _ = std::fs::remove_dir(&dir);
                            log::error!("meeting capture did not open: {e}");
                            return Err(MeetingError::AudioDevice(e));
                        }
                    }
                }
                other => log::debug!("meeting start: unexpected effect {other:?}"),
            }
        }
        let control = control.ok_or(MeetingError::InvalidTransition)?;

        // The row is written only once the capture is open, so a device that fails leaves no
        // session behind. The audio matters more than the row: a failed write is logged and the
        // recording goes on.
        let mut take = None;
        if let Some((_, number)) = resume {
            // Without its row the take would be stopped as the session's first recording (and a
            // short one even discarded with the session): no row, no take.
            match self.with_store(|store| Ok(store.start_meeting_take(id, now)?)) {
                Ok(opened) if opened.take == number => take = Some(opened),
                opened => {
                    let _ = control.send(Control::Finalize);
                    hide_indicator(&self.app);
                    if opened.is_ok() {
                        let closed = self.with_store(|store| {
                            Ok(store.finish_meeting_take(id, now, 0, Some(StopReason::User))?)
                        });
                        log::error!(
                            "meeting {id}: take {number} opened under another number: {closed:?}"
                        );
                    }
                    return Err(opened.err().unwrap_or(MeetingError::InvalidTransition));
                }
            }
        }
        let saved = self.with_store(|store| {
            if resume.is_some() {
                return Ok(());
            }
            if !existing {
                store.create_meeting(&NewMeeting {
                    id,
                    title: title.to_string(),
                    mode: mode.session_mode(),
                    local_only: false,
                    created_at: now,
                })?;
            } else if !title.trim().is_empty() {
                store.set_meeting_title(id, title)?;
            }
            Ok(store.start_meeting_recording(id, mode.session_mode(), now)?)
        });
        if let Err(e) = saved {
            log::error!("meeting {id}: start not saved: {e}");
        }
        log::info!("meeting {id} recording ({mode:?})");
        inner.active = Some(Active {
            session,
            mode,
            control,
            mute: MuteWatch::new(mode.session_mode()),
            route: output::Route::new(mode.session_mode(), &system),
            last_tick: Duration::ZERO,
            tick_levels: Levels::default(),
            voiced: Duration::ZERO,
            capture_failed: false,
            dictating: false,
            waking: false,
            interrupted: false,
            take,
        });
        inner.status = MeetingStatus {
            low_disk_bytes: low_disk,
            system_output: Some(system),
            ..MeetingStatus::default()
        };
        Ok(self.publish(&mut inner))
    }

    pub fn pause(&self) -> Result<MeetingStatus, MeetingError> {
        let status = self.act(UserAction::PauseRecording)?;
        crate::call_prompt::hide_still_recording(&self.app);
        Ok(status)
    }

    /// "Resume": the only place a paused capture records again (ADR-0005).
    pub fn resume(&self) -> Result<MeetingStatus, MeetingError> {
        self.act(UserAction::ResumeRecording)
    }

    pub fn stop(&self) -> Result<MeetingStatus, MeetingError> {
        self.act(UserAction::StopRecording)
    }

    /// "Keep recording" on "Still recording?": the silence counts from zero again.
    pub fn keep_recording(&self) -> Result<MeetingStatus, MeetingError> {
        self.act(UserAction::KeepRecording)
    }

    /// The "Still recording?" card is on screen: only now may silence stop the recording.
    pub(crate) fn silence_warning_shown(&self) {
        let mut inner = self.lock();
        if let Some(active) = inner.active.as_mut() {
            let _ = active
                .session
                .apply(millis(now_local()), Input::SilenceWarningShown);
        }
    }

    /// The PC sleeps or wakes (`power::watch_sleep`): a recording goes on in the same session.
    pub(crate) fn on_sleep(&self, event: SleepEvent) {
        let input = match event {
            SleepEvent::Suspending => Input::OsSuspended,
            SleepEvent::Resumed => Input::OsResumed,
        };
        let ack = {
            let mut inner = self.lock();
            let Some(active) = inner.active.as_mut() else {
                return;
            };
            if event == SleepEvent::Suspending {
                // The silence count restarts; nobody can answer the card while asleep.
                crate::call_prompt::hide_still_recording(&self.app);
            }
            match active.session.apply(millis(now_local()), input) {
                Ok(effects) => self.run_effects(&mut inner, effects),
                Err(e) => log::warn!("meeting: {event:?} refused: {e}"),
            }
            self.publish(&mut inner);
            inner.suspend_ack.take()
        };
        // Outside the lock: the recorder may be reporting meanwhile.
        if let Some(ack) = ack {
            if ack.recv_timeout(SUSPEND_GRACE).is_err() {
                log::warn!("meeting WAV header not flushed before sleep; it is at most 1 s behind");
            }
        }
    }

    /// "One more hour" on the cap warning.
    pub fn extend_cap(&self) -> Result<MeetingStatus, MeetingError> {
        self.act(UserAction::ExtendCap)?;
        let mut inner = self.lock();
        inner.status.cap_remaining_ms = None;
        Ok(self.publish(&mut inner))
    }

    fn act(&self, action: UserAction) -> Result<MeetingStatus, MeetingError> {
        let mut inner = self.lock();
        let active = inner.active.as_mut().ok_or(MeetingError::NotActive)?;
        let effects = active
            .session
            .apply(millis(now_local()), Input::User(action))?;
        self.run_effects(&mut inner, effects);
        Ok(self.publish(&mut inner))
    }

    /// "Record from this device": the system channel switches to `name` while recording. Only
    /// this session changes; the remembered choice stays as the panel set it.
    pub fn switch_output(&self, name: &str) -> Result<MeetingStatus, MeetingError> {
        let mut inner = self.lock();
        let active = inner.active.as_ref().ok_or(MeetingError::NotActive)?;
        if active.route.recording() != name {
            self.send(&inner, Control::SwitchSystem(name.to_string()));
        }
        Ok(self.publish(&mut inner))
    }

    /// "Stop and quit" on the quit question: the capture closes, the end is saved and the app
    /// exits (`finish`). Should the capture not close, the app exits after a few seconds anyway:
    /// the WAV header is at most a second behind, and the next start offers to recover it.
    pub fn stop_and_quit(&self) {
        self.quit_after_stop.store(true, Ordering::SeqCst);
        if let Err(e) = self.stop() {
            log::info!("quit: nothing left to stop ({e})");
            let (capturing, finishing) = {
                let inner = self.lock();
                (inner.active.is_some(), !inner.processing.is_empty())
            };
            if !capturing {
                if finishing {
                    // `finish` exits at its next step; a transcription in flight ends now.
                    self.cancel_transcription();
                } else {
                    self.app.exit(0);
                }
                return;
            }
        }
        // Still capturing (or `Stopping`): `finish` exits once the WAV is closed.
        let app = self.app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(QUIT_GRACE);
            log::warn!("quit: the meeting capture did not close in time");
            app.exit(0);
        });
    }

    /// The tray's "Record meeting": without the notice accepted, opens the window on it.
    pub fn start_from_tray(&self) {
        let consent = crate::settings::get_settings(&self.app).meeting_consent_accepted_at;
        match tray_start(consent.as_deref()) {
            TrayStart::AskConsent => {
                crate::show_main_window(&self.app);
                let _ = self.app.emit(CONSENT_REQUIRED_EVENT, ());
            }
            TrayStart::Start => {
                if let Err(e) = self.start(MeetingMode::Meeting, "", None) {
                    log::warn!("meeting not started from the tray: {e}");
                }
            }
        }
    }

    fn on_report(&self, report: Report) {
        let mut inner = self.lock();
        let Some(active) = inner.active.as_mut() else {
            return;
        };
        match report {
            Report::Levels {
                recorded,
                mic,
                system,
                mic_peak,
                system_peak,
                outputs,
                mic_omitted,
            } => {
                let levels = Levels {
                    mic,
                    system,
                    mic_omitted,
                };
                let muted = active.mute.observe(recorded, levels);
                let route = active
                    .route
                    .observe(recorded, mic_peak, system_peak, &outputs);
                active.tick_levels.mic = active.tick_levels.mic.max(mic);
                active.tick_levels.system = active.tick_levels.system.max(system);
                active.tick_levels.mic_omitted |= mic_omitted;
                let mut effects = Vec::new();
                if recorded >= active.last_tick + Duration::from_secs(1) {
                    let elapsed = recorded - active.last_tick;
                    active.last_tick = recorded;
                    let levels = std::mem::take(&mut active.tick_levels);
                    if lifecycle::heard_speech(levels) {
                        active.voiced += elapsed;
                    }
                    match active
                        .session
                        .apply(millis(now_local()), Input::Tick { recorded, levels })
                    {
                        Ok(more) => effects = more,
                        Err(e) => log::warn!("meeting tick refused: {e}"),
                    }
                }
                inner.status.recorded_ms = u64::try_from(recorded.as_millis()).unwrap_or(u64::MAX);
                inner.status.mic_level = mic;
                inner.status.system_level = system;
                inner.status.muted_mic = muted.mic;
                inner.status.muted_system = muted.system;
                route.apply(&mut inner.status);
                // Only a change resizes the pill's window.
                crate::overlay::set_meeting_card(&self.app, meeting_card(&inner.status));
                self.run_effects(&mut inner, effects);
            }
            Report::Omitted(spans) => {
                let id = active.session.id();
                // A take's spans are in its own time; the table is on the session's timeline.
                let offset = active.take.as_ref().map_or(0, |take| take.offset_ms);
                // Saved now, so a crash keeps the marker of every dictation already closed.
                let saved = self.with_store(|store| {
                    for (t0, t1) in &spans {
                        store.add_meeting_omission(id, t0 + offset, t1 + offset)?;
                    }
                    Ok(())
                });
                if let Err(e) = saved {
                    log::error!("meeting {id}: dictation span not saved: {e}");
                }
            }
            Report::SystemSwitched { name, error } => {
                if error.is_none() {
                    active.route.switched(&name);
                    inner.status.system_output = Some(name);
                    inner.status.call_output = None;
                    crate::overlay::set_meeting_card(&self.app, meeting_card(&inner.status));
                }
            }
            Report::Woke => {
                active.waking = false;
                let id = active.session.id();
                let sleep = active
                    .session
                    .gaps()
                    .iter()
                    .rev()
                    .find(|gap| gap.kind == GapKind::Suspend)
                    .and_then(sleeps::from_gap);
                if let Some(mut sleep) = sleep {
                    sleep.at_ms += active.take.as_ref().map_or(0, |take| take.offset_ms);
                    if let Err(e) = sleeps::append(&self.session_dir(id), sleep) {
                        log::warn!("meeting {id}: sleep marker not saved: {e}");
                    }
                    if matches!(active.session.state(), SessionState::Recording { .. }) {
                        let slept = Duration::from_millis(sleep.slept_ms);
                        crate::call_prompt::show_woke(&self.app, slept);
                    }
                }
            }
            Report::Finalized { recorded, error } => {
                if let Some(e) = error {
                    log::error!("meeting capture failed: {e}");
                    active.capture_failed = true;
                    // A stop asked meanwhile (the person, the cap) stays that stop.
                    active.interrupted = active.waking
                        && !matches!(active.session.state(), SessionState::Stopping { .. });
                }
                let now = millis(now_local());
                if matches!(
                    active.session.state(),
                    SessionState::Recording { .. }
                        | SessionState::Paused { .. }
                        | SessionState::Suspended { .. }
                ) {
                    // The capture ended on its own (device or disk error): stop as the user
                    // would; its FinalizeCapture is already done.
                    let _ = active
                        .session
                        .apply(now, Input::User(UserAction::StopRecording));
                }
                let finalized = active.session.apply(now, Input::CaptureFinalized);
                inner.status.recorded_ms = u64::try_from(recorded.as_millis()).unwrap_or(u64::MAX);
                match finalized {
                    Ok(effects) => self.run_effects(&mut inner, effects),
                    Err(e) => log::error!("meeting finalize refused: {e}"),
                }
            }
        }
        self.publish(&mut inner);
    }

    /// The sound of a session effect and what it does to the media playing on the PC.
    fn feedback(&self, effect: &Effect) {
        if let Some(sound) = meeting_sound(effect) {
            audio_feedback::play_feedback_sound(&self.app, sound);
        }
        match media_action(effect) {
            Some(MediaAction::Pause) => {
                if crate::settings::get_settings(&self.app).pause_media_on_meeting {
                    media_pause::pause(Owner::Meeting);
                }
            }
            // Always: a pause asked before the setting went off is still resumed.
            Some(MediaAction::Resume) => media_pause::resume(Owner::Meeting),
            None => {}
        }
    }

    fn run_effects(&self, inner: &mut Inner, effects: Vec<Effect>) {
        for effect in effects {
            self.feedback(&effect);
            match effect {
                Effect::WarnLowDisk { free_bytes } => {
                    inner.status.low_disk_bytes = Some(free_bytes)
                }
                Effect::ShowIndicator(kind) => show_indicator(&self.app, kind),
                Effect::HideIndicator => hide_indicator(&self.app),
                Effect::PauseCapture => self.send(inner, Control::Pause),
                Effect::ResumeCapture => self.send(inner, Control::Resume),
                Effect::FinalizeCapture => self.send(inner, Control::Finalize),
                Effect::SuspendCapture => {
                    let (done, ack) = mpsc::channel();
                    self.send(inner, Control::Suspend(done));
                    inner.suspend_ack = Some(ack);
                }
                Effect::WakeCapture => {
                    if let Some(active) = inner.active.as_mut() {
                        active.waking = true;
                    }
                    self.send(inner, Control::Wake);
                }
                Effect::CapWarning { remaining } => {
                    inner.status.cap_remaining_ms =
                        Some(u64::try_from(remaining.as_millis()).unwrap_or(u64::MAX));
                }
                Effect::SilenceWarning { remaining } => {
                    crate::call_prompt::show_still_recording(&self.app, remaining)
                }
                Effect::Process => self.process(inner),
                // Starting is done by `start`.
                Effect::StartCapture => {
                    log::debug!("meeting: effect {effect:?} not handled here");
                }
            }
        }
    }

    fn send(&self, inner: &Inner, control: Control) {
        if let Some(active) = &inner.active {
            if active.control.send(control).is_err() {
                log::warn!("meeting recorder thread is gone");
            }
        }
    }

    /// Hands the finished session to a worker: end saved, audio retained, mic given back.
    fn process(&self, inner: &mut Inner) {
        let Some(active) = inner.active.take() else {
            return;
        };
        // A dictation still running goes on without a meeting: its card goes.
        if active.dictating {
            inner.status.dictating = false;
            crate::overlay::set_meeting_dictation(&self.app, false);
        }
        let id = active.session.id();
        // `None`: the capture did not come back after a sleep, recorded as interrupted (#128).
        let reason = (!active.interrupted).then(|| stop_reason(active.session.state()));
        let recorded_ms = inner.status.recorded_ms;
        let voiced_ms = u64::try_from(active.voiced.as_millis()).unwrap_or(u64::MAX);
        // Only a stop the person clicked, or silence, may look like a mistake; never the cap or a
        // device that failed.
        let may_be_mistake = !active.capture_failed
            && matches!(reason, Some(StopReason::User | StopReason::Silence));
        inner.processing.push(id);
        crate::call_prompt::hide_still_recording(&self.app);
        if reason == Some(StopReason::Silence) && voiced_ms >= lifecycle::LITTLE_SPEECH_MS {
            crate::call_prompt::show_silence_stopped(&self.app);
        }
        let app = self.app.clone();
        let take = active.take.clone();
        let dir = match &take {
            Some(take) => self.session_dir(id).join(format!("t{}", take.take)),
            None => self.session_dir(id),
        };
        log::info!("meeting {id} stopped ({reason:?}, {recorded_ms} ms, {voiced_ms} ms of speech)");
        let spawned = std::thread::Builder::new()
            .name("meeting-process".to_string())
            .spawn(move || {
                if let Some(manager) = app.try_state::<Arc<MeetingManager>>() {
                    // A take is never discarded as a mistake: the session already has a recording.
                    let voiced_ms = (may_be_mistake && take.is_none()).then_some(voiced_ms);
                    manager.finish(id, reason, recorded_ms, voiced_ms, &dir, take.as_ref());
                }
            });
        if let Err(e) = spawned {
            log::error!("meeting {id}: processing thread did not start: {e}");
            inner.processing.retain(|p| *p != id);
        }
    }

    fn finish(
        &self,
        id: SessionId,
        reason: Option<StopReason>,
        recorded_ms: u64,
        // `None` when the stop cannot be a mistake (cap, failed capture, a take).
        voiced_ms: Option<u64>,
        dir: &Path,
        take: Option<&MeetingTake>,
    ) {
        let saved = self.with_store(|store| match (take, reason) {
            (Some(_), reason) => {
                store.finish_meeting_take(id, now_local(), recorded_ms, reason)?;
                Ok(())
            }
            (None, Some(reason)) => {
                Ok(store.finish_meeting(id, now_local(), recorded_ms, reason)?)
            }
            (None, None) => Ok(store.recover_meeting(id, now_local(), recorded_ms)?),
        });
        if let Err(e) = saved {
            log::error!("meeting {id}: end not saved: {e}");
        }
        // The WAV is closed and the end saved: the next start retains and transcribes it.
        if self.quit_if_asked(id) {
            return;
        }
        self.retain(id, dir);
        if self.quit_if_asked(id) {
            return;
        }
        {
            let mut inner = self.lock();
            inner.processing.retain(|p| *p != id);
            self.publish(&mut inner);
        }
        if let Some(voiced_ms) = voiced_ms {
            if self.handle_mistake(id, recorded_ms, voiced_ms) {
                return;
            }
        }
        let lines = match take {
            Some(take) => self.transcribe_take_if_keyed(id, take),
            None => self.transcribe_if_keyed(id),
        };
        if self.quit_if_asked(id) {
            return;
        }
        // After a stop, the notes follow the transcript (`auto_notes`); after a take, notes that
        // exist stay as they are until the person writes them again.
        if let Some(lines) = lines {
            self.auto_notes(id, &lines);
        }
        self.quit_if_asked(id);
    }

    /// "Stop and quit" was clicked: each step of `finish` that completes is a safe place to exit.
    fn quit_if_asked(&self, id: SessionId) -> bool {
        let quit = self.quit_after_stop.load(Ordering::SeqCst);
        if quit {
            log::info!("meeting {id}: stopped to quit");
            self.app.exit(0);
        }
        quit
    }

    /// With the key in the keyring, the transcript follows a stop, an import, a recovery or a
    /// "Keep" without another click.
    pub(crate) fn transcribe_if_keyed(&self, id: SessionId) -> Option<Vec<pipeline::MeetingLine>> {
        if !pipeline::has_key(self.keys.as_ref(), pipeline::SCRIBE_KEY) {
            return None;
        }
        match self.transcribe(&id.to_string()) {
            Ok(lines) => Some(lines),
            Err(e) => {
                log::warn!("meeting {id}: automatic transcription failed: {e}");
                None
            }
        }
    }

    /// After a take's stop, with the key in the keyring: only the take is sent, and its
    /// transcript goes after the session's.
    pub(crate) fn transcribe_take_if_keyed(
        &self,
        id: SessionId,
        take: &MeetingTake,
    ) -> Option<Vec<pipeline::MeetingLine>> {
        if !pipeline::has_key(self.keys.as_ref(), pipeline::SCRIBE_KEY) {
            return None;
        }
        self.transcribe_with(id, Some(take))
            .map_err(|e| log::warn!("meeting {id}: take not transcribed: {e}"))
            .ok()
    }

    /// `recording.wav` becomes `mic.opus` and `sys.opus`; on any error the WAV stays.
    fn retain(&self, id: SessionId, dir: &Path) {
        // One conversion at a time: the stop's worker and a "transcribe" click must never write
        // the same `.part` files, nor one delete the WAV the other still reads (ADR-0014). The
        // second caller waits and then finds the WAV gone.
        let _retaining = self.retaining.lock().unwrap_or_else(|e| e.into_inner());
        let wav = dir.join(WORK_WAV);
        if !wav.exists() {
            return;
        }
        self.progress(id, pipeline::ProgressStage::Retaining);
        match fala_retention::retain_wav(&wav, dir) {
            Ok(audio) => {
                log::info!("meeting {id}: audio retained ({} samples)", audio.samples);
                if let Err(e) = self.with_store(|store| Ok(store.set_meeting_audio_retained(id)?)) {
                    log::error!("meeting {id}: retained audio not saved: {e}");
                }
            }
            Err(e) => log::error!("meeting {id}: audio not retained, WAV kept: {e}"),
        }
    }

    /// A dictation started (`true`) or ended: while it lasts, the recording's mic channel is
    /// zeroed from the dictation's pre-buffer on (ADR-0015). Nothing to do without a mic.
    pub(crate) fn dictation_changed(&self, dictating: bool, at: std::time::Instant) {
        let mut inner = self.lock();
        let Some(active) = inner.active.as_mut() else {
            return;
        };
        if !active.mode.session_mode().has_mic() || active.dictating == dictating {
            return;
        }
        active.dictating = dictating;
        let control = if dictating {
            Control::BeginOmit(at)
        } else {
            Control::EndOmit
        };
        self.send(&inner, control);
        inner.status.dictating = dictating;
        crate::overlay::set_meeting_dictation(&self.app, dictating);
        self.publish(&mut inner);
    }

    /// Refreshes the phase and id from the session, emits the status and returns it.
    fn publish(&self, inner: &mut Inner) -> MeetingStatus {
        let (state, id, mode) = match &inner.active {
            Some(active) => (
                match active.session.state() {
                    SessionState::Recording { .. } => MeetingPhase::Recording,
                    SessionState::Paused { .. } | SessionState::Suspended { .. } => {
                        MeetingPhase::Paused
                    }
                    SessionState::Stopping { .. } => MeetingPhase::Stopping,
                    SessionState::Idle | SessionState::Stopped { .. } => MeetingPhase::Processing,
                },
                Some(active.session.id().to_string()),
                Some(active.mode),
            ),
            None if !inner.processing.is_empty() => (MeetingPhase::Processing, None, None),
            None => (MeetingPhase::Idle, None, None),
        };
        if state == MeetingPhase::Idle || (state == MeetingPhase::Processing && id.is_none()) {
            inner.status = MeetingStatus::default();
        }
        inner.status.state = state;
        inner.status.id = id;
        inner.status.mode = mode;
        if let Err(e) = inner.status.clone().emit(&self.app) {
            log::debug!("meeting status not emitted: {e}");
        }
        inner.status.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn store(name: &str) -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join(format!("{name}.sqlite")), dir.path()).unwrap();
        (dir, store)
    }

    #[test]
    fn pill_card_for_the_call_or_the_first_seconds() {
        let mut status = MeetingStatus::default();
        assert!(meeting_card(&status));
        status.recorded_ms = CONSENT_CHIP_MS - 1;
        assert!(meeting_card(&status));
        status.recorded_ms = CONSENT_CHIP_MS;
        assert!(!meeting_card(&status));
        status.call_output = Some("Fone".into());
        assert!(meeting_card(&status));
    }

    #[test]
    fn start_guard_order() {
        assert_eq!(
            check_start(None, false, false),
            Err(MeetingError::ConsentRequired)
        );
        assert_eq!(
            check_start(None, true, true),
            Err(MeetingError::ConsentRequired)
        );
        let consent = Some("2026-10-09T22:14:03-03:00");
        assert_eq!(
            check_start(consent, true, false),
            Err(MeetingError::DictationActive)
        );
        assert_eq!(
            check_start(consent, false, true),
            Err(MeetingError::AlreadyActive)
        );
        assert_eq!(check_start(consent, false, false), Ok(()));
    }

    #[test]
    fn consent_stamp_and_error_shape() {
        let at = FixedOffset::west_opt(3 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, 10, 9, 22, 14, 3)
            .unwrap();
        assert_eq!(consent_stamp(at), "2026-10-09T22:14:03-03:00");
        assert_eq!(
            serde_json::to_value(MeetingError::ConsentRequired).unwrap(),
            serde_json::json!({ "kind": "consent_required" })
        );
        assert_eq!(
            serde_json::to_value(MeetingError::AudioDevice("sem mic".into())).unwrap(),
            serde_json::json!({ "kind": "audio_device", "detail": "sem mic" })
        );
    }

    #[test]
    fn system_only_opens_no_mic() {
        // `start` opens the mic only when the session mode has one.
        let mode = MeetingMode::SystemOnly.session_mode();
        assert_eq!(mode, SessionMode::SystemOnly);
        assert!(!mode.has_mic());
        assert!(MeetingMode::Meeting.session_mode().has_mic());
        assert!(MeetingMode::InPerson.session_mode().has_mic());
        assert_eq!(
            serde_json::to_value(MeetingMode::SystemOnly).unwrap(),
            serde_json::json!("system_only")
        );
    }

    #[test]
    fn tray_start_without_consent_asks() {
        assert_eq!(tray_start(None), TrayStart::AskConsent);
        assert_eq!(
            tray_start(Some("2026-10-09T22:14:03-03:00")),
            TrayStart::Start
        );
    }

    #[test]
    fn each_session_step_plays_its_sound_and_moves_the_media() {
        let mut session = MeetingSession::new(SessionConfig {
            id: SessionId::generate(1_000).unwrap(),
            mode: SessionMode::Meeting,
            title: String::new(),
            local_only: false,
            cap: RecordingCap::default(),
        });
        let mut step = |at: u64, input: Input| {
            let effects = session.apply(UnixMillis(at), input).unwrap();
            let sounds: Vec<_> = effects.iter().filter_map(meeting_sound).collect();
            let media: Vec<_> = effects.iter().filter_map(media_action).collect();
            (sounds, media)
        };
        let start = Input::User(UserAction::StartRecording {
            free_disk_bytes: u64::MAX,
        });
        assert_eq!(
            step(1_000, start),
            (vec![SoundType::MeetingStart], vec![MediaAction::Pause])
        );
        assert_eq!(
            step(2_000, Input::User(UserAction::PauseRecording)),
            (vec![SoundType::MeetingPause], vec![MediaAction::Resume])
        );
        assert_eq!(
            step(3_000, Input::User(UserAction::ResumeRecording)),
            (vec![SoundType::MeetingResume], vec![MediaAction::Pause])
        );
        // Stopping has no sound of its own; the media plays again once the capture closes.
        assert_eq!(
            step(4_000, Input::User(UserAction::StopRecording)),
            (vec![], vec![MediaAction::Resume])
        );
        assert_eq!(
            step(5_000, Input::CaptureFinalized),
            (vec![], vec![MediaAction::Resume])
        );
    }

    #[test]
    fn quitting_while_recording_asks() {
        assert!(quit_needs_question(MeetingIndicator::Recording));
        assert!(quit_needs_question(MeetingIndicator::Paused));
        assert!(!quit_needs_question(MeetingIndicator::None));
    }

    #[test]
    fn dictation_is_refused_only_in_person() {
        for indicator in [MeetingIndicator::Recording, MeetingIndicator::Paused] {
            assert_eq!(dictation_refusal(indicator, false), None, "{indicator:?}");
            assert_eq!(
                dictation_refusal(indicator, true),
                Some("meeting_in_person"),
                "{indicator:?}"
            );
        }
        assert_eq!(dictation_refusal(MeetingIndicator::None, false), None);
        assert_eq!(dictation_refusal(MeetingIndicator::None, true), None);
    }

    #[test]
    fn stop_reasons_reach_storage() {
        for reason in [
            StopReason::User,
            StopReason::Silence,
            StopReason::CapReached,
        ] {
            assert_eq!(stop_reason(SessionState::Stopped { reason }), reason);
            assert_eq!(stop_reason(SessionState::Stopping { reason }), reason);
        }
        let (_dir, store) = store("reasons");
        let at = now_local();
        for (n, reason) in [StopReason::Silence, StopReason::CapReached]
            .into_iter()
            .enumerate()
        {
            let id = SessionId::from_parts(1_800_000_000_000 + n as u64, 7);
            store
                .create_meeting(&NewMeeting {
                    id,
                    title: String::new(),
                    mode: SessionMode::Meeting,
                    local_only: false,
                    created_at: at,
                })
                .unwrap();
            store
                .start_meeting_recording(id, SessionMode::Meeting, at)
                .unwrap();
            store
                .finish_meeting(id, at, 1_000, stop_reason(SessionState::Stopped { reason }))
                .unwrap();
            assert_eq!(store.meeting(id).unwrap().stop_reason, Some(reason));
        }
    }

    #[test]
    fn start_uses_the_draft() {
        let (_dir, store) = store("draft");
        let now = now_local();
        let draft = SessionId::from_parts(1_800_000_000_000, 1);
        store
            .create_meeting(&NewMeeting {
                id: draft,
                title: "1:1".into(),
                mode: SessionMode::Meeting,
                local_only: false,
                created_at: now,
            })
            .unwrap();
        assert_eq!(
            start_target(&store, Some(draft), millis(now)),
            Ok((draft, true))
        );
        store
            .start_meeting_recording(draft, SessionMode::Meeting, now)
            .unwrap();
        assert_eq!(
            start_target(&store, Some(draft), millis(now)),
            Err(MeetingError::AlreadyStarted)
        );
        let (fresh, existing) = start_target(&store, None, millis(now)).unwrap();
        assert!(!existing);
        assert_ne!(fresh, draft);
        assert_eq!(store.meetings().unwrap().len(), 1);
        let missing = SessionId::from_parts(1_900_000_000_000, 2);
        assert_eq!(
            start_target(&store, Some(missing), millis(now)),
            Err(MeetingError::NotFound)
        );
    }

    #[test]
    fn only_a_finished_recorded_session_resumes_and_takes_count_up() {
        let (_dir, mut store) = store("resume");
        let now = now_local();
        let id = SessionId::from_parts(1_800_000_000_000, 1);
        let meeting = |id, mode| NewMeeting {
            id,
            title: String::new(),
            mode,
            local_only: false,
            created_at: now,
        };
        store
            .create_meeting(&meeting(id, SessionMode::InPerson))
            .unwrap();
        // A draft and a recording do not resume.
        assert_eq!(
            resume_target(&store, id),
            Err(MeetingError::InvalidTransition)
        );
        store
            .start_meeting_recording(id, SessionMode::InPerson, now)
            .unwrap();
        assert_eq!(
            resume_target(&store, id),
            Err(MeetingError::InvalidTransition)
        );
        store
            .finish_meeting(id, now, 60_000, StopReason::User)
            .unwrap();
        // Finished: take 2, in the session's own mode.
        assert_eq!(resume_target(&store, id), Ok((MeetingMode::InPerson, 2)));
        store.start_meeting_take(id, now).unwrap();
        // A take still open: no other.
        assert_eq!(
            resume_target(&store, id),
            Err(MeetingError::InvalidTransition)
        );
        store
            .finish_meeting_take(id, now, 1_000, Some(StopReason::User))
            .unwrap();
        assert_eq!(resume_target(&store, id), Ok((MeetingMode::InPerson, 3)));
        // An import never resumes, nor a session in the trash.
        let import = SessionId::from_parts(1_800_000_000_001, 2);
        store
            .create_meeting(&meeting(import, SessionMode::Import))
            .unwrap();
        store
            .start_meeting_recording(import, SessionMode::Import, now)
            .unwrap();
        store
            .finish_meeting(import, now, 0, StopReason::User)
            .unwrap();
        assert_eq!(
            resume_target(&store, import),
            Err(MeetingError::InvalidTransition)
        );
        store.trash_meeting(id, now).unwrap();
        assert_eq!(
            resume_target(&store, id),
            Err(MeetingError::InvalidTransition)
        );
    }
}
