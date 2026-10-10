//! Meeting recording in the desktop shell (phase 2, F2 `meeting-panel`).
//!
//! `MeetingManager` is a facade: the session rules live in `fala-meeting` (start only on an
//! explicit `UserAction`, indicator, cap, silence), the capture in `fala-audio`, the retained
//! audio in `fala-retention` and the rows in `fala-storage`. This module only runs the effects the
//! session returns and reports them to the UI (ADR-0002).
//!
//! ADR-0005: `UserAction::StartRecording` and `ResumeRecording` are built only in
//! [`MeetingManager::start`] and [`MeetingManager::resume`], which only the meeting commands and
//! the tray items call. ADR-0015: without the microphone fan-out, dictation is refused while a
//! meeting records, and the dictation stream is closed so the mic has a single stream.

mod disk;
pub(crate) mod recorder;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use chrono::{DateTime, FixedOffset, Local, SecondsFormat};
use fala_audio::SystemAudio;
use fala_meeting::{
    Effect, IndicatorKind, Input, Levels, MeetingSession, MuteWatch, RecordingCap, SessionConfig,
    SessionError, SessionId, SessionMode, SessionState, StopReason, UnixMillis, UserAction,
};
use fala_storage::{NewMeeting, StorageError, Store};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Emitter, Manager};
use tauri_specta::Event;

use crate::managers::audio::AudioRecordingManager;
use recorder::{Control, Report};

/// The working WAV inside `audio/<id>/`; `fala-retention` turns it into `mic.opus` and
/// `sys.opus` and deletes it once both validate (ADR-0014).
pub(crate) const WORK_WAV: &str = "recording.wav";

/// Event the tray emits when "Record meeting" is clicked before the third-party notice was
/// accepted: the window opens on the Meetings page with the notice instead of recording.
pub(crate) const CONSENT_REQUIRED_EVENT: &str = "meeting-consent-required";

/// The two capture modes the panel offers (part 1 of F2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum MeetingMode {
    /// A call with headphones: the mic is "me", the system is "them".
    Meeting,
    /// Everyone in the room: the mic is diarized too.
    InPerson,
}

impl MeetingMode {
    fn session_mode(self) -> SessionMode {
        match self {
            MeetingMode::Meeting => SessionMode::Meeting,
            MeetingMode::InPerson => SessionMode::InPerson,
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
}

impl std::fmt::Display for MeetingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeetingError::AudioDevice(detail) | MeetingError::Storage(detail) => {
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

/// The `recording-error` type that refuses a dictation while a meeting records, if any.
pub(crate) fn dictation_refusal(indicator: MeetingIndicator) -> Option<&'static str> {
    match indicator {
        MeetingIndicator::Recording | MeetingIndicator::Paused => Some("meeting_active"),
        MeetingIndicator::None => None,
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
    /// Recorded time at the last `Tick` given to the session (one per second).
    last_tick: Duration,
    /// Loudest level of each channel since the last `Tick`.
    tick_levels: Levels,
}

#[derive(Default)]
struct Inner {
    active: Option<Active>,
    /// Sessions whose audio is being retained.
    processing: usize,
    status: MeetingStatus,
}

pub struct MeetingManager {
    app: AppHandle,
    audio_root: PathBuf,
    store: Mutex<Option<Store>>,
    inner: Mutex<Inner>,
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
        Ok(Self {
            app: app.clone(),
            audio_root: data.join("audio"),
            store: Mutex::new(store),
            inner: Mutex::new(Inner::default()),
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
        let consent = crate::settings::get_settings(&self.app).meeting_consent_accepted_at;
        let dictation = self
            .app
            .try_state::<Arc<AudioRecordingManager>>()
            .is_some_and(|rm| rm.is_recording());
        let mut inner = self.lock();
        check_start(consent.as_deref(), dictation, inner.active.is_some())?;
        let draft = draft.map(parse_id).transpose()?;

        let now = now_local();
        let (id, existing) = self.with_store(|store| start_target(store, draft, millis(now)))?;
        let dir = self.session_dir(id);
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
            cap: RecordingCap::default(),
        });
        let effects = session.apply(
            millis(now),
            Input::User(UserAction::StartRecording {
                free_disk_bytes: free,
            }),
        )?;

        let system =
            SystemAudio::default_name().map_err(|e| MeetingError::AudioDevice(e.to_string()))?;
        self.release_dictation_mic();
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
                    match recorder::spawn(system.clone(), dir.join(WORK_WAV), sink) {
                        Ok(tx) => control = Some(tx),
                        Err(e) => {
                            hide_indicator(&self.app);
                            self.restore_dictation_mic();
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
        let saved = self.with_store(|store| {
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
            last_tick: Duration::ZERO,
            tick_levels: Levels::default(),
        });
        inner.status = MeetingStatus {
            low_disk_bytes: low_disk,
            ..MeetingStatus::default()
        };
        Ok(self.publish(&mut inner))
    }

    pub fn pause(&self) -> Result<MeetingStatus, MeetingError> {
        self.act(UserAction::PauseRecording)
    }

    /// "Resume": the only place a paused capture records again (ADR-0005).
    pub fn resume(&self) -> Result<MeetingStatus, MeetingError> {
        self.act(UserAction::ResumeRecording)
    }

    pub fn stop(&self) -> Result<MeetingStatus, MeetingError> {
        self.act(UserAction::StopRecording)
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
            } => {
                let levels = Levels { mic, system };
                let muted = active.mute.observe(recorded, levels);
                active.tick_levels.mic = active.tick_levels.mic.max(mic);
                active.tick_levels.system = active.tick_levels.system.max(system);
                let mut effects = Vec::new();
                if recorded >= active.last_tick + Duration::from_secs(1) {
                    active.last_tick = recorded;
                    let levels = std::mem::take(&mut active.tick_levels);
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
                self.run_effects(&mut inner, effects);
            }
            Report::Finalized { recorded, error } => {
                if let Some(e) = error {
                    log::error!("meeting capture failed: {e}");
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

    fn run_effects(&self, inner: &mut Inner, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::WarnLowDisk { free_bytes } => {
                    inner.status.low_disk_bytes = Some(free_bytes)
                }
                Effect::ShowIndicator(kind) => show_indicator(&self.app, kind),
                Effect::HideIndicator => hide_indicator(&self.app),
                Effect::PauseCapture => self.send(inner, Control::Pause),
                Effect::ResumeCapture => self.send(inner, Control::Resume),
                Effect::FinalizeCapture => self.send(inner, Control::Finalize),
                Effect::CapWarning { remaining } => {
                    inner.status.cap_remaining_ms =
                        Some(u64::try_from(remaining.as_millis()).unwrap_or(u64::MAX));
                }
                Effect::Process => self.process(inner),
                // Starting is done by `start`; suspension is not detected yet (TODO(windows)).
                Effect::StartCapture | Effect::AskContinueOrStop => {
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
        let id = active.session.id();
        let reason = stop_reason(active.session.state());
        let recorded_ms = inner.status.recorded_ms;
        inner.processing += 1;
        let app = self.app.clone();
        let dir = self.session_dir(id);
        log::info!("meeting {id} stopped ({reason:?}, {recorded_ms} ms)");
        let spawned = std::thread::Builder::new()
            .name("meeting-process".to_string())
            .spawn(move || {
                if let Some(manager) = app.try_state::<Arc<MeetingManager>>() {
                    manager.finish(id, reason, recorded_ms, &dir);
                }
            });
        if let Err(e) = spawned {
            log::error!("meeting {id}: processing thread did not start: {e}");
            inner.processing -= 1;
        }
    }

    fn finish(&self, id: SessionId, reason: StopReason, recorded_ms: u64, dir: &Path) {
        if let Err(e) = self
            .with_store(|store| Ok(store.finish_meeting(id, now_local(), recorded_ms, reason)?))
        {
            log::error!("meeting {id}: end not saved: {e}");
        }
        self.restore_dictation_mic();
        self.retain(id, dir);
        let mut inner = self.lock();
        inner.processing = inner.processing.saturating_sub(1);
        self.publish(&mut inner);
    }

    /// `recording.wav` becomes `mic.opus` and `sys.opus`; on any error the WAV stays.
    fn retain(&self, id: SessionId, dir: &Path) {
        let wav = dir.join(WORK_WAV);
        if !wav.exists() {
            return;
        }
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

    fn release_dictation_mic(&self) {
        if let Some(rm) = self.app.try_state::<Arc<AudioRecordingManager>>() {
            rm.release_microphone_for_meeting();
        }
    }

    fn restore_dictation_mic(&self) {
        if let Some(rm) = self.app.try_state::<Arc<AudioRecordingManager>>() {
            if let Err(e) = rm.restore_microphone_after_meeting() {
                log::warn!("dictation microphone not reopened after the meeting: {e}");
            }
        }
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
            None if inner.processing > 0 => (MeetingPhase::Processing, None, None),
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
    fn tray_start_without_consent_asks() {
        assert_eq!(tray_start(None), TrayStart::AskConsent);
        assert_eq!(
            tray_start(Some("2026-10-09T22:14:03-03:00")),
            TrayStart::Start
        );
    }

    #[test]
    fn dictation_blocked_while_recording() {
        assert_eq!(
            dictation_refusal(MeetingIndicator::Recording),
            Some("meeting_active")
        );
        assert_eq!(
            dictation_refusal(MeetingIndicator::Paused),
            Some("meeting_active")
        );
        assert_eq!(dictation_refusal(MeetingIndicator::None), None);
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
}
