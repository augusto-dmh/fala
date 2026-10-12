//! What happens to a stopped meeting: the transcript from ElevenLabs Scribe (ADR-0005), the
//! notes from the notes LLM (ADR-0016), and the views the Meetings page reads.
//!
//! Keys come only from the OS keyring (ADR-0008): `elevenlabs` for the transcript and the
//! `anthropic` key the post-processing settings already store, for the notes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, FixedOffset};
use fala_asr::meeting::{
    CancelToken, Channel, ElevenLabsScribe, MeetingRecording, MeetingTranscriber, Progress,
    ScribeOptions, Segment, Speaker, Stage, MIC_FILE, SYSTEM_FILE,
};
use fala_core::{Dictionary, Language};
use fala_meeting::SessionId;
use fala_notes::{
    builtin_templates, generate_notes, load_user_templates, Claude, NotesError, NotesInput,
    Template,
};
use fala_secrets::{ApiKey, SecretStore};
use fala_storage::{
    MeetingRecord, MeetingSegment, NameSuggestion, SegmentChannel, SegmentSpeaker, Store,
};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use super::context::{notes_settings, NotesSettings};
use super::people::speaker_names;
use super::{parse_id, MeetingError, MeetingManager, WORK_WAV};

/// The notes follow a stop only from this much transcript text and recorded time (Granola waits
/// for about the same before it writes notes on its own); below it, "Generate notes" stays a
/// click.
pub(crate) const AUTO_NOTES_MIN_CHARS: usize = 2_500;
pub(crate) const AUTO_NOTES_MIN_MS: u64 = 3 * 60 * 1000;

/// Keyring account of the ElevenLabs key (`fala-cli key set elevenlabs` stores the same one).
pub(crate) const SCRIBE_KEY: &str = "elevenlabs";

/// A session in the list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct MeetingSummary {
    pub id: String,
    pub title: String,
    /// `meeting`, `in_person`, `system_only` or `import`.
    pub mode: String,
    /// RFC 3339 with the local offset at the time.
    pub created_at: String,
    /// `None` while the session is a draft that never recorded.
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub recorded_ms: u64,
    pub audio_retained: bool,
    pub transcribed: bool,
    pub has_notes: bool,
}

/// One transcript line: `person` is `None` for "me" (the mic outside in-person mode).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct MeetingLine {
    pub seq: u32,
    /// `mic` or `system`.
    pub channel: String,
    pub person: Option<u32>,
    /// The name applied to "Person N" from a suggestion; absent when there is none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub name: Option<String>,
    pub t0_ms: u64,
    pub t1_ms: u64,
    pub text: String,
}

/// An opened session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct MeetingDetail {
    pub summary: MeetingSummary,
    /// Notes typed before (agenda) and during the call.
    pub annotations: String,
    pub lines: Vec<MeetingLine>,
    pub notes_md: Option<String>,
    pub notes_template: Option<String>,
    /// The notes were edited by hand since they were generated.
    pub notes_edited: bool,
    /// Its notes are being written now (a click, or the stop's chain): a page that opens later
    /// still shows it.
    pub notes_running: bool,
    /// Both retained Opus files are on disk, so a transcript line can be played.
    pub audio_available: bool,
    /// Where the PC slept during the recording, for the transcript's markers.
    #[serde(default)]
    #[specta(optional)]
    pub sleeps: Vec<super::sleeps::MeetingSleep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct MeetingTemplate {
    pub id: String,
    pub name: String,
    /// What the notes are for; the pickers search it with the name.
    pub purpose: String,
    pub source: TemplateSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum TemplateSource {
    Builtin,
    /// A `*.json` of `<data>/templates`, edited outside the app.
    User,
}

/// A file of the templates folder that is not a template, shown as "file.json: error".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct TemplateFileError {
    pub file: String,
    pub error: String,
}

/// The built-in templates, then the person's own, and the files that did not load.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct MeetingTemplates {
    pub templates: Vec<MeetingTemplate>,
    pub errors: Vec<TemplateFileError>,
}

/// Which keys exist in the keyring; never the keys themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct MeetingKeys {
    pub elevenlabs: bool,
    pub anthropic: bool,
}

/// Where the processing of a session is, at least once a second while the transcript runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type, Event)]
pub struct MeetingProgress {
    pub id: String,
    pub stage: ProgressStage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub enum ProgressStage {
    /// An import: `phase` is `download` (bytes `done` of `total`, 0 when unknown) or `convert`
    /// (milliseconds of audio).
    Importing {
        phase: String,
        done: u64,
        total: u64,
    },
    /// The working WAV becomes the two Opus files.
    Retaining,
    /// `phase` is `uploading` (bytes `done` of `total`) or `waiting` for the provider.
    Transcribing {
        channel: String,
        phase: String,
        done: u64,
        total: u64,
    },
    Notes,
    Done,
    Failed {
        error: MeetingError,
    },
}

fn rfc3339(at: &DateTime<FixedOffset>) -> String {
    at.to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
}

fn mode_str(record: &MeetingRecord) -> String {
    serde_json::to_value(record.mode)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

pub(crate) fn summary(record: &MeetingRecord) -> MeetingSummary {
    MeetingSummary {
        id: record.id.to_string(),
        title: record.title.clone(),
        mode: mode_str(record),
        created_at: rfc3339(&record.created_at),
        started_at: record.started_at.as_ref().map(rfc3339),
        ended_at: record.ended_at.as_ref().map(rfc3339),
        recorded_ms: record.recorded_ms,
        audio_retained: record.audio_retained,
        transcribed: record.transcribed_at.is_some(),
        has_notes: record.notes_md.is_some(),
    }
}

pub(crate) fn line(segment: &MeetingSegment) -> MeetingLine {
    MeetingLine {
        seq: segment.seq,
        channel: match segment.channel {
            SegmentChannel::Mic => "mic",
            SegmentChannel::System => "system",
        }
        .to_string(),
        person: match segment.speaker {
            SegmentSpeaker::Me => None,
            SegmentSpeaker::Person(n) => Some(n),
        },
        name: None,
        t0_ms: segment.t0_ms,
        t1_ms: segment.t1_ms,
        text: segment.text.clone(),
    }
}

/// The ASR segments in the stored shape; `fala-storage` numbers them by `t0_ms` when saving.
pub(crate) fn segments_for_store(segments: Vec<Segment>) -> Vec<MeetingSegment> {
    segments
        .into_iter()
        .map(|segment| MeetingSegment {
            seq: 0,
            channel: match segment.channel {
                Channel::Mic => SegmentChannel::Mic,
                Channel::System => SegmentChannel::System,
            },
            speaker: match segment.speaker {
                Speaker::Me => SegmentSpeaker::Me,
                Speaker::Person(n) => SegmentSpeaker::Person(n.get()),
            },
            t0_ms: segment.t0_ms,
            t1_ms: segment.t1_ms,
            text: segment.text,
        })
        .collect()
}

/// The stored segments as the notes crate reads them: `seq` is the id the `^sN` pointers cite,
/// and a "Person N" with an applied name goes by that name.
pub(crate) fn notes_segments(
    segments: &[MeetingSegment],
    names: &HashMap<u32, String>,
) -> Vec<fala_notes::Segment> {
    segments
        .iter()
        .map(|segment| fala_notes::Segment {
            id: segment.seq,
            channel: match segment.channel {
                SegmentChannel::Mic => fala_notes::Channel::Mic,
                SegmentChannel::System => fala_notes::Channel::System,
            },
            speaker: match segment.speaker {
                SegmentSpeaker::Me => fala_notes::Speaker::Me,
                SegmentSpeaker::Person(n) => match names.get(&n) {
                    Some(name) => fala_notes::Speaker::Named(name.clone()),
                    None => fala_notes::Speaker::Person(n),
                },
            },
            t0_ms: segment.t0_ms,
            t1_ms: segment.t1_ms,
            text: segment.text.clone(),
        })
        .collect()
}

/// The applied names that may go to the notes LLM: a name with `@` may be an e-mail, which never
/// goes out (ADR-0016), so that person stays "Person N" there. The transcript and the local
/// Markdown keep the name as typed.
pub(crate) fn outgoing_names(names: HashMap<u32, String>) -> HashMap<u32, String> {
    names
        .into_iter()
        .filter(|(_, name)| !name.contains('@'))
        .collect()
}

/// The ElevenLabs key, from the keyring only.
pub(crate) fn scribe_key(store: &dyn SecretStore) -> Result<ApiKey, MeetingError> {
    store
        .get(SCRIBE_KEY)
        .map_err(|_| MeetingError::Keyring)?
        .ok_or(MeetingError::MissingKey)
}

/// The keyring seen by the notes client; read only inside the request (ADR-0016).
pub(crate) struct KeyringKeys(pub(crate) Arc<dyn SecretStore>);

impl fala_notes::KeySource for KeyringKeys {
    fn api_key(&self, provider: &str) -> Result<Option<fala_notes::ApiKey>, NotesError> {
        match self.0.get(provider) {
            Ok(Some(key)) => fala_notes::ApiKey::new(key.expose()).map(Some),
            Ok(None) => Ok(None),
            Err(_) => Err(NotesError::KeyStore),
        }
    }
}

pub(crate) fn has_key(store: &dyn SecretStore, provider: &str) -> bool {
    matches!(store.get(provider), Ok(Some(_)))
}

/// The session still has its working WAV and not both Opus files: retain before transcribing
/// (a crash, or a retention that failed before).
pub(crate) fn needs_retain(dir: &Path) -> bool {
    dir.join(WORK_WAV).is_file()
        && !(dir.join(MIC_FILE).is_file() && dir.join(SYSTEM_FILE).is_file())
}

/// `<data>/templates`: the person's own templates, read again on every use (no watcher).
pub(crate) fn templates_dir(app: &tauri::AppHandle) -> Result<PathBuf, MeetingError> {
    crate::portable::app_data_dir(app)
        .map(|data| data.join("templates"))
        .map_err(|e| MeetingError::Storage(e.to_string()))
}

/// A built-in template, or one of `dir`; a user file never shadows a built-in id.
pub(crate) fn find_template(dir: &Path, id: &str) -> Result<Template, MeetingError> {
    let builtin = builtin_templates().map_err(|e| MeetingError::Notes(e.to_string()))?;
    builtin
        .into_iter()
        .chain(load_user_templates(dir).0)
        .find(|t| t.id == id)
        .ok_or(MeetingError::UnknownTemplate)
}

pub(crate) fn templates(dir: &Path) -> MeetingTemplates {
    let (user, errors) = load_user_templates(dir);
    let entry = |source: TemplateSource| {
        move |t: Template| MeetingTemplate {
            id: t.id,
            name: t.name,
            purpose: t.purpose,
            source,
        }
    };
    MeetingTemplates {
        templates: builtin_templates()
            .unwrap_or_default()
            .into_iter()
            .map(entry(TemplateSource::Builtin))
            .chain(user.into_iter().map(entry(TemplateSource::User)))
            .collect(),
        errors: errors
            .into_iter()
            .map(|e| TemplateFileError {
                file: e.file,
                error: e.error,
            })
            .collect(),
    }
}

/// What the notes LLM receives, from the stored session (ADR-0016 list: title, local date and
/// start time, notes language, profile, dictionary, participants, template, the typed notes, the
/// transcript with the applied names).
pub(crate) fn notes_input(
    record: &MeetingRecord,
    segments: &[MeetingSegment],
    settings: &NotesSettings,
    participants: Vec<String>,
    names: &HashMap<u32, String>,
    template: Template,
) -> NotesInput {
    let start = record.started_at.unwrap_or(record.created_at);
    NotesInput {
        title: record.title.clone(),
        date: start.format("%Y-%m-%d").to_string(),
        start_time: start.format("%H:%M").to_string(),
        language: settings.language,
        profile: settings.profile.clone(),
        dictionary: settings.dictionary.clone(),
        participants,
        template,
        annotations: record.annotations.clone(),
        transcript: notes_segments(segments, names),
        local_only: record.local_only,
    }
}

/// Whether the notes follow the transcript of a stop without a click: the setting is on, the
/// session may reach the LLM (never "local only"), the key is there and the call is long enough.
pub(crate) fn auto_notes_due(
    enabled: bool,
    record: &MeetingRecord,
    has_key: bool,
    lines: &[MeetingLine],
) -> bool {
    let chars: usize = lines.iter().map(|line| line.text.chars().count()).sum();
    enabled
        && has_key
        && !record.local_only
        && record.notes_md.is_none()
        && record.recorded_ms >= AUTO_NOTES_MIN_MS
        && chars >= AUTO_NOTES_MIN_CHARS
}

/// The notes were edited after `started`, the moment a generation began: its result must not
/// replace an edit nobody confirmed. Stored times have whole seconds, so the start is cut to the
/// second (an edit in that same second counts as during).
pub(crate) fn edited_since(
    edited_at: Option<DateTime<FixedOffset>>,
    started: DateTime<FixedOffset>,
) -> bool {
    let started = started.timestamp();
    edited_at.is_some_and(|at| at.timestamp() >= started)
}

/// Regenerating with the same template discards the edits: once edited by hand, only after the
/// confirmation. Another template is a new version the person asked for, as in Granola.
pub(crate) fn guard_edited_notes(
    store: &Store,
    id: SessionId,
    template_id: &str,
    replace_edited: bool,
) -> Result<(), MeetingError> {
    if replace_edited {
        return Ok(());
    }
    let same_template = store.meeting(id)?.notes_template.as_deref() == Some(template_id);
    if same_template && store.meeting_notes_edited(id)? {
        return Err(MeetingError::NotesEdited);
    }
    Ok(())
}

impl From<NotesError> for MeetingError {
    fn from(error: NotesError) -> Self {
        match error {
            NotesError::MissingKey => MeetingError::MissingKey,
            NotesError::KeyStore => MeetingError::Keyring,
            other => MeetingError::Notes(other.to_string()),
        }
    }
}

impl From<fala_asr::AsrError> for MeetingError {
    fn from(error: fala_asr::AsrError) -> Self {
        match error {
            fala_asr::AsrError::Cancelled => MeetingError::Cancelled,
            fala_asr::AsrError::MissingChannel(_) => MeetingError::NoAudio,
            other => MeetingError::Transcription(other.to_string()),
        }
    }
}

/// The long job in flight: a transcription (cancellable) or a notes generation.
#[derive(Default)]
pub(crate) struct Work {
    transcribing: Option<(SessionId, CancelToken)>,
    notes: Option<SessionId>,
}

impl Work {
    /// The session being transcribed, whose audio is being read.
    pub(crate) fn transcribing(&self) -> Option<SessionId> {
        self.transcribing.as_ref().map(|(id, _)| *id)
    }

    /// A transcription or a notes generation is working on `id`.
    pub(crate) fn involves(&self, id: SessionId) -> bool {
        self.transcribing() == Some(id) || self.notes == Some(id)
    }
}

/// Clears the job slot when the job ends, however it ends.
struct WorkGuard<'a> {
    work: &'a std::sync::Mutex<Work>,
    notes: bool,
}

impl Drop for WorkGuard<'_> {
    fn drop(&mut self) {
        let mut work = self.work.lock().unwrap_or_else(|e| e.into_inner());
        if self.notes {
            work.notes = None;
        } else {
            work.transcribing = None;
        }
    }
}

/// What the Scribe request takes from the settings: the dictation language, and the dictionary
/// as `keyterms` only while that setting is on (ADR-0005).
pub(crate) fn scribe_options(settings: &crate::settings::AppSettings) -> ScribeOptions {
    ScribeOptions {
        language: Some(crate::managers::history_dictations::language_from_setting(
            &settings.selected_language,
        )),
        keyterms: settings.meeting_keyterms,
        dictionary: Dictionary::new(settings.custom_words.clone()),
    }
}

fn progress_stage(progress: Progress) -> ProgressStage {
    let channel = match progress.channel {
        Channel::Mic => "mic",
        Channel::System => "system",
    }
    .to_string();
    let (phase, done, total) = match progress.stage {
        Stage::Uploading { sent, total } => ("uploading", sent, total),
        Stage::Waiting => ("waiting", 0, 0),
        Stage::Transcribing { done_ms, total_ms } => ("local", done_ms, total_ms),
    };
    ProgressStage::Transcribing {
        channel,
        phase: phase.to_string(),
        done,
        total,
    }
}

impl MeetingManager {
    pub(crate) fn progress(&self, id: SessionId, stage: ProgressStage) {
        let event = MeetingProgress {
            id: id.to_string(),
            stage,
        };
        if let Err(e) = event.emit(&self.app) {
            log::debug!("meeting progress not emitted: {e}");
        }
    }

    pub fn list(&self) -> Result<Vec<MeetingSummary>, MeetingError> {
        self.with_store(|store| Ok(store.meetings()?.iter().map(summary).collect()))
    }

    pub fn detail(&self, id: &str) -> Result<MeetingDetail, MeetingError> {
        let id = parse_id(id)?;
        let audio_available = super::clip::audio_available(&self.session_dir(id));
        let sleeps = super::sleeps::read(&self.session_dir(id));
        let notes_running = self.work.lock().unwrap_or_else(|e| e.into_inner()).notes == Some(id);
        self.with_store(|store| {
            let record = store.meeting(id)?;
            let names = speaker_names(store, id)?;
            let lines = store
                .meeting_segments(id)?
                .iter()
                .map(|segment| MeetingLine {
                    name: match segment.speaker {
                        SegmentSpeaker::Person(n) => names.get(&n).cloned(),
                        SegmentSpeaker::Me => None,
                    },
                    ..line(segment)
                })
                .collect();
            Ok(MeetingDetail {
                summary: summary(&record),
                annotations: record.annotations,
                lines,
                notes_md: record.notes_md,
                notes_template: record.notes_template,
                notes_edited: store.meeting_notes_edited(id)?,
                notes_running,
                audio_available,
                sleeps,
            })
        })
    }

    /// The WAV the ▶ of a transcript line plays: `[t0, t1]` with margins (`clip::window`).
    pub fn clip(&self, id: &str, t0_ms: u64, t1_ms: u64) -> Result<Vec<u8>, MeetingError> {
        let id = parse_id(id)?;
        super::clip::clip_wav(&self.session_dir(id), t0_ms, t1_ms)
    }

    /// "Copy Markdown": the note document with its transcript anchors.
    pub fn markdown(&self, id: &str) -> Result<String, MeetingError> {
        let id = parse_id(id)?;
        let settings = notes_settings(&crate::settings::get_settings(&self.app));
        self.with_store(|store| {
            let record = store.meeting(id)?;
            let segments = store.meeting_segments(id)?;
            let template = self
                .template(record.notes_template.as_deref().unwrap_or("geral"))
                .or_else(|_| self.template("geral"))?;
            let input = notes_input(
                &record,
                &segments,
                &settings,
                store.meeting_participants(id)?,
                &speaker_names(store, id)?,
                template,
            );
            Ok(fala_notes::note_document(
                &input,
                record.notes_md.as_deref(),
            ))
        })
    }

    pub fn keys(&self) -> MeetingKeys {
        MeetingKeys {
            elevenlabs: has_key(self.keys.as_ref(), SCRIBE_KEY),
            anthropic: has_key(self.keys.as_ref(), fala_notes::KEY_PROVIDER),
        }
    }

    /// Stores the ElevenLabs key in the keyring; it is never logged nor sent back.
    pub fn set_transcription_key(&self, key: &str) -> Result<MeetingKeys, MeetingError> {
        let key = ApiKey::new(key.trim()).map_err(|_| MeetingError::InvalidKey)?;
        self.keys
            .set(SCRIBE_KEY, &key)
            .map_err(|_| MeetingError::Keyring)?;
        log::info!("ElevenLabs key stored in the keyring");
        Ok(self.keys())
    }

    /// Transcribes a stopped session with Scribe v2 and stores the segments (ADR-0005).
    pub fn transcribe(&self, id: &str) -> Result<Vec<MeetingLine>, MeetingError> {
        let id = parse_id(id)?;
        if self.status().id.as_deref() == Some(id.to_string().as_str()) {
            return Err(MeetingError::StillRecording);
        }
        let record = self.with_store(|store| Ok(store.meeting(id)?))?;
        if record.started_at.is_none() || record.ended_at.is_none() {
            return Err(MeetingError::StillRecording);
        }
        let cancel = CancelToken::new();
        {
            let mut work = self.work.lock().unwrap_or_else(|e| e.into_inner());
            if work.transcribing.is_some() {
                return Err(MeetingError::Busy);
            }
            work.transcribing = Some((id, cancel.clone()));
        }
        let _guard = WorkGuard {
            work: &self.work,
            notes: false,
        };
        let result = self.run_transcription(id, &record, &cancel);
        match &result {
            Ok(lines) => {
                log::info!("meeting {id}: {} transcript segments", lines.len());
                self.progress(id, ProgressStage::Done);
            }
            Err(error) => self.progress(
                id,
                ProgressStage::Failed {
                    error: error.clone(),
                },
            ),
        }
        result
    }

    fn run_transcription(
        &self,
        id: SessionId,
        record: &MeetingRecord,
        cancel: &CancelToken,
    ) -> Result<Vec<MeetingLine>, MeetingError> {
        let dir = self.session_dir(id);
        if needs_retain(&dir) {
            self.retain(id, &dir);
        }
        let recording = MeetingRecording::from_session_dir(&dir, record.mode)?;
        let key = scribe_key(self.keys.as_ref())?;
        let options = scribe_options(&crate::settings::get_settings(&self.app));
        let mut scribe = ElevenLabsScribe::new(key, options);
        let segments = scribe.transcribe_session(&recording, cancel, &mut |progress| {
            self.progress(id, progress_stage(progress));
        })?;
        let now = chrono::Local::now().fixed_offset();
        self.with_store(|store| {
            store.replace_meeting_segments(id, segments_for_store(segments), now)?;
            Ok(store.meeting_segments(id)?.iter().map(line).collect())
        })
    }

    /// The notes as edited in the page; regenerating them now asks first.
    pub fn save_edited_notes(&self, id: &str, markdown: &str) -> Result<(), MeetingError> {
        let id = parse_id(id)?;
        self.with_store(|store| {
            Ok(store.save_edited_meeting_notes(
                id,
                markdown,
                chrono::Local::now().fixed_offset(),
            )?)
        })
    }

    /// The template for the next notes, picked in the draft or while recording; ignored once the
    /// session has notes.
    pub fn set_template(&self, id: &str, template_id: &str) -> Result<(), MeetingError> {
        let id = parse_id(id)?;
        self.template(template_id)?;
        self.with_store(|store| {
            store.set_meeting_template(id, template_id)?;
            Ok(())
        })
    }

    /// After the transcript of a stop: the notes, with the session's template, when
    /// [`auto_notes_due`] says so. Progress and failure reach the page as `MeetingProgress`.
    pub(crate) fn auto_notes(&self, id: SessionId, lines: &[MeetingLine]) {
        let enabled = crate::settings::get_settings(&self.app).meeting_auto_notes;
        let record = match self.with_store(|store| Ok(store.meeting(id)?)) {
            Ok(record) => record,
            Err(e) => return log::warn!("meeting {id}: automatic notes skipped: {e}"),
        };
        let has_key = has_key(self.keys.as_ref(), fala_notes::KEY_PROVIDER);
        if !auto_notes_due(enabled, &record, has_key, lines) {
            log::debug!("meeting {id}: notes left to a click");
            return;
        }
        let template = record
            .notes_template
            .filter(|t| match self.template(t) {
                Ok(_) => true,
                Err(e) => {
                    log::warn!("meeting {id}: template {t} unavailable ({e}); notes use geral");
                    false
                }
            })
            .unwrap_or_else(|| "geral".to_string());
        if let Err(e) = self.generate_notes(&id.to_string(), &template, false) {
            log::warn!("meeting {id}: automatic notes failed: {e}");
        }
    }

    /// Cancels the transcription in flight, if any; it returns `cancelled` within a second.
    pub fn cancel_transcription(&self) {
        let work = self.work.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((id, cancel)) = &work.transcribing {
            log::info!("meeting {id}: transcription cancelled");
            cancel.cancel();
        }
    }

    /// A built-in template or one of the person's own.
    fn template(&self, id: &str) -> Result<Template, MeetingError> {
        find_template(&templates_dir(&self.app)?, id)
    }

    /// Generates the notes with a built-in or user template and stores the Markdown (ADR-0016). Notes
    /// edited by hand are only replaced by the same template with `replace_edited`, the click on
    /// the confirmation.
    pub fn generate_notes(
        &self,
        id: &str,
        template_id: &str,
        replace_edited: bool,
    ) -> Result<String, MeetingError> {
        let id = parse_id(id)?;
        let template = self.template(template_id)?;
        self.with_store(|store| guard_edited_notes(store, id, template_id, replace_edited))?;
        {
            let mut work = self.work.lock().unwrap_or_else(|e| e.into_inner());
            if work.notes.is_some() {
                return Err(MeetingError::Busy);
            }
            work.notes = Some(id);
        }
        let _guard = WorkGuard {
            work: &self.work,
            notes: true,
        };
        let settings = notes_settings(&crate::settings::get_settings(&self.app));
        let (input, transcribed_at) = self.with_store(|store| {
            let record = store.meeting(id)?;
            let input = notes_input(
                &record,
                &store.meeting_segments(id)?,
                &settings,
                store.meeting_participants(id)?,
                &outgoing_names(speaker_names(store, id)?),
                template,
            );
            Ok((input, record.transcribed_at))
        })?;
        self.progress(id, ProgressStage::Notes);
        let started = chrono::Local::now().fixed_offset();
        let claude = Claude::new(Arc::new(KeyringKeys(Arc::clone(&self.keys))));
        let notes = match generate_notes(&input, &claude) {
            Ok(notes) => notes,
            Err(e) => {
                let error = MeetingError::from(e);
                self.progress(
                    id,
                    ProgressStage::Failed {
                        error: error.clone(),
                    },
                );
                return Err(error);
            }
        };
        // The suggestions wait for a click: nothing here changes a speaker label (ADR-0016).
        let suggestions: Vec<(u32, NameSuggestion)> = notes
            .speaker_suggestions
            .iter()
            .map(|s| {
                let suggestion = NameSuggestion {
                    name: s.name.clone(),
                    reason: s.reason.clone(),
                };
                (s.person, suggestion)
            })
            .collect();
        let saved = self.with_store(|store| {
            // Edited while the LLM wrote (the page lets the person edit again): keep the edit.
            if edited_since(store.meeting_notes_edited_at(id)?, started) {
                return Err(MeetingError::NotesEdited);
            }
            Ok(store.save_meeting_notes(id, template_id, &notes.markdown)?)
        });
        if let Err(error) = saved {
            self.progress(
                id,
                ProgressStage::Failed {
                    error: error.clone(),
                },
            );
            return Err(error);
        }
        // A transcription that finished meanwhile renumbered "Person N": these suggestions point
        // at the old numbers. The notes are saved either way.
        let saved = self.with_store(|store| {
            if store.meeting(id)?.transcribed_at != transcribed_at {
                log::info!("meeting {id}: transcribed again during the notes; suggestions dropped");
                return Ok(());
            }
            Ok(store.save_speaker_suggestions(id, &suggestions)?)
        });
        if let Err(e) = saved {
            log::warn!("meeting {id}: speaker suggestions not stored: {e}");
        }
        // The generated title only fills an empty one, never a title the user typed.
        if let Some(title) = &notes.title {
            // The notes are saved; a title that fails to land only leaves the default one.
            if let Err(e) =
                self.with_store(|store| Ok(store.set_generated_meeting_title(id, title)?))
            {
                log::warn!("meeting {id}: generated title not stored: {e}");
            }
        }
        self.progress(id, ProgressStage::Done);
        Ok(notes.markdown)
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use chrono::TimeZone;
    use fala_meeting::SessionMode;
    use fala_secrets::MemoryStore;

    use super::*;

    fn asr(channel: Channel, speaker: Speaker, t0_ms: u64, text: &str) -> Segment {
        Segment {
            channel,
            speaker,
            t0_ms,
            t1_ms: t0_ms + 500,
            text: text.to_string(),
        }
    }

    #[test]
    fn segments_round_trip() {
        let person = |n| Speaker::Person(NonZeroU32::new(n).unwrap());
        let stored = segments_for_store(vec![
            asr(Channel::System, person(2), 4_000, "b"),
            asr(Channel::Mic, Speaker::Me, 0, "a"),
        ]);
        assert_eq!(stored[0].channel, SegmentChannel::System);
        assert_eq!(stored[0].speaker, SegmentSpeaker::Person(2));
        assert_eq!(stored[1].speaker, SegmentSpeaker::Me);

        // As fala-storage hands them back: numbered by t0.
        let numbered = vec![
            MeetingSegment {
                seq: 1,
                ..stored[1].clone()
            },
            MeetingSegment {
                seq: 2,
                ..stored[0].clone()
            },
        ];
        let notes = notes_segments(&numbered, &HashMap::new());
        assert_eq!(notes[0].id, 1);
        assert_eq!(notes[0].speaker, fala_notes::Speaker::Me);
        assert_eq!(notes[0].channel, fala_notes::Channel::Mic);
        assert_eq!(notes[1].id, 2);
        assert_eq!(notes[1].speaker, fala_notes::Speaker::Person(2));
        // An applied name replaces "Person 2"; "me" never takes one.
        let names = HashMap::from([(2, "Ana".to_string()), (1, "Bruno".to_string())]);
        let named = notes_segments(&numbered, &names);
        assert_eq!(named[0].speaker, fala_notes::Speaker::Me);
        assert_eq!(named[1].speaker, fala_notes::Speaker::Named("Ana".into()));
        assert_eq!(line(&numbered[0]).person, None);
        assert_eq!(line(&numbered[1]).person, Some(2));
        assert_eq!(line(&numbered[1]).channel, "system");
    }

    #[test]
    fn keyterms_follow_the_setting() {
        let mut settings = crate::settings::get_default_settings();
        settings.custom_words = vec!["Fala".into(), "ADR".into()];
        let options = scribe_options(&settings);
        assert!(options.keyterms);
        assert_eq!(options.dictionary, Dictionary::new(["Fala", "ADR"]));
        settings.meeting_keyterms = false;
        assert!(!scribe_options(&settings).keyterms);
    }

    #[test]
    fn keys_come_from_the_store() {
        let store = MemoryStore::default();
        assert_eq!(scribe_key(&store), Err(MeetingError::MissingKey));
        assert!(!has_key(&store, SCRIBE_KEY));
        store
            .set(SCRIBE_KEY, &ApiKey::new("xi-segredo").unwrap())
            .unwrap();
        assert_eq!(scribe_key(&store).unwrap().expose(), "xi-segredo");
        assert!(has_key(&store, SCRIBE_KEY));

        let keys = KeyringKeys(Arc::new(MemoryStore::default()));
        use fala_notes::KeySource;
        assert_eq!(keys.api_key(fala_notes::KEY_PROVIDER), Ok(None));
        let store = MemoryStore::default();
        store
            .set(
                fala_notes::KEY_PROVIDER,
                &ApiKey::new("sk-ant-segredo").unwrap(),
            )
            .unwrap();
        let keys = KeyringKeys(Arc::new(store));
        assert_eq!(
            keys.api_key("anthropic").unwrap().unwrap().expose(),
            "sk-ant-segredo"
        );
        assert_eq!(
            MeetingError::from(NotesError::MissingKey),
            MeetingError::MissingKey
        );
    }

    #[test]
    fn retain_before_transcribing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!needs_retain(dir.path()));
        std::fs::write(dir.path().join(WORK_WAV), b"RIFF").unwrap();
        assert!(needs_retain(dir.path()));
        std::fs::write(dir.path().join(MIC_FILE), b"OggS").unwrap();
        assert!(needs_retain(dir.path()));
        std::fs::write(dir.path().join(SYSTEM_FILE), b"OggS").unwrap();
        assert!(!needs_retain(dir.path()));
        assert_eq!(
            MeetingError::from(fala_asr::AsrError::MissingChannel("sys.opus")),
            MeetingError::NoAudio
        );
    }

    #[test]
    fn notes_input_from_session() {
        let offset = FixedOffset::west_opt(3 * 3600).unwrap();
        let created = offset.with_ymd_and_hms(2026, 10, 2, 13, 50, 0).unwrap();
        let started = offset.with_ymd_and_hms(2026, 10, 2, 14, 2, 30).unwrap();
        let record = MeetingRecord {
            id: SessionId::from_parts(1_727_000_000_000, 1),
            title: "Planejamento".into(),
            mode: SessionMode::Meeting,
            local_only: false,
            created_at: created,
            started_at: Some(started),
            ended_at: None,
            recorded_ms: 0,
            stop_reason: None,
            audio_retained: true,
            // Typed in the draft, before recording: it goes to the notes as typed notes.
            annotations: "pauta: contrato".into(),
            transcribed_at: None,
            notes_md: None,
            notes_template: None,
        };
        auto_notes_cases(&record);
        let segments = vec![MeetingSegment {
            seq: 1,
            channel: SegmentChannel::Mic,
            speaker: SegmentSpeaker::Me,
            t0_ms: 0,
            t1_ms: 900,
            text: "oi".into(),
        }];
        let template = find_template(Path::new("nao-existe"), "geral").unwrap();
        let settings = NotesSettings {
            language: Language::PtBr,
            profile: "PM do Fala".into(),
            dictionary: Dictionary::new(["Fala"]),
        };
        let input = notes_input(
            &record,
            &segments,
            &settings,
            vec!["Ana".into()],
            &HashMap::new(),
            template.clone(),
        );
        assert_eq!(input.title, "Planejamento");
        assert_eq!(input.date, "2026-10-02");
        assert_eq!(input.start_time, "14:02");
        assert_eq!(input.language, Language::PtBr);
        assert_eq!(input.profile, "PM do Fala");
        assert_eq!(input.participants, ["Ana"]);
        assert_eq!(input.dictionary, Dictionary::new(["Fala"]));
        assert_eq!(input.template, template);
        assert_eq!(input.annotations, "pauta: contrato");
        assert_eq!(input.transcript.len(), 1);
        assert_eq!(input.transcript[0].id, 1);
        assert!(!input.local_only);

        let draft = MeetingRecord {
            started_at: None,
            ..record
        };
        let english = NotesSettings {
            language: Language::En,
            profile: String::new(),
            dictionary: Dictionary::default(),
        };
        assert_eq!(
            notes_input(&draft, &[], &english, Vec::new(), &HashMap::new(), template).start_time,
            "13:50"
        );
        assert_eq!(
            find_template(Path::new("nao-existe"), "nao-existe"),
            Err(MeetingError::UnknownTemplate)
        );
    }

    /// `meeting_templates`: the built-ins, then the folder's templates, and its broken files; a
    /// user file never takes a built-in id.
    #[test]
    fn meeting_templates_lists_builtin_and_user_templates() {
        let dir = tempfile::tempdir().unwrap();
        let user = |id: &str, name: &str| {
            format!(
                r#"{{"schema":1,"id":"{id}","name":"{name}","purpose":"Vender","style":"s","sections":[{{"title":"Dor","instruction":"x"}}]}}"#
            )
        };
        std::fs::write(dir.path().join("vendas.json"), user("vendas", "Vendas")).unwrap();
        std::fs::write(dir.path().join("geral.json"), user("geral", "Minha geral")).unwrap();
        std::fs::write(dir.path().join("quebrado.json"), "{").unwrap();
        let listed = templates(dir.path());
        let ids: Vec<(&str, TemplateSource)> = listed
            .templates
            .iter()
            .map(|t| (t.id.as_str(), t.source))
            .collect();
        assert_eq!(
            ids,
            [
                ("geral", TemplateSource::Builtin),
                ("um-a-um", TemplateSource::Builtin),
                ("vendas", TemplateSource::User),
            ]
        );
        assert_eq!(listed.templates[2].purpose, "Vender");
        let files: Vec<&str> = listed.errors.iter().map(|e| e.file.as_str()).collect();
        assert_eq!(files, ["geral.json", "quebrado.json"]);
        assert_eq!(find_template(dir.path(), "vendas").unwrap().name, "Vendas");
        assert_eq!(
            find_template(dir.path(), "geral").unwrap().name,
            "Reunião geral"
        );
    }

    /// Chain stop -> transcript -> notes: only when every condition holds.
    fn auto_notes_cases(record: &MeetingRecord) {
        let long = MeetingRecord {
            recorded_ms: AUTO_NOTES_MIN_MS,
            ..record.clone()
        };
        let text = "x".repeat(AUTO_NOTES_MIN_CHARS / 2);
        let line = |seq| MeetingLine {
            seq,
            channel: "system".into(),
            person: Some(1),
            name: None,
            t0_ms: 0,
            t1_ms: 1,
            text: text.clone(),
        };
        let enough = vec![line(1), line(2)];
        assert!(auto_notes_due(true, &long, true, &enough));
        assert!(!auto_notes_due(false, &long, true, &enough), "setting off");
        assert!(!auto_notes_due(true, &long, false, &enough), "no key");
        assert!(
            !auto_notes_due(true, &long, true, &enough[..1]),
            "short text"
        );
        let short = MeetingRecord {
            recorded_ms: AUTO_NOTES_MIN_MS - 1,
            ..long.clone()
        };
        assert!(!auto_notes_due(true, &short, true, &enough), "short call");
        let local = MeetingRecord {
            local_only: true,
            ..long.clone()
        };
        assert!(!auto_notes_due(true, &local, true, &enough), "local only");
        let with_notes = MeetingRecord {
            notes_md: Some("- x".into()),
            ..long
        };
        assert!(
            !auto_notes_due(true, &with_notes, true, &enough),
            "has notes"
        );
    }

    #[test]
    fn typed_name_with_at_never_goes_out() {
        let segments = vec![MeetingSegment {
            seq: 1,
            channel: SegmentChannel::System,
            speaker: SegmentSpeaker::Person(2),
            t0_ms: 0,
            t1_ms: 900,
            text: "oi".into(),
        }];
        let names = HashMap::from([(1, "Bruno".to_string()), (2, "ana@empresa.com".to_string())]);
        // "Copy Markdown" keeps the name as typed; the notes LLM gets "Person 2".
        assert_eq!(
            notes_segments(&segments, &names)[0].speaker,
            fala_notes::Speaker::Named("ana@empresa.com".into())
        );
        let outgoing = outgoing_names(names);
        assert_eq!(outgoing, HashMap::from([(1, "Bruno".to_string())]));
        assert_eq!(
            notes_segments(&segments, &outgoing)[0].speaker,
            fala_notes::Speaker::Person(2)
        );
    }

    #[test]
    fn edited_notes_are_regenerated_only_after_the_confirmation() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("fala.sqlite"), dir.path()).unwrap();
        let offset = FixedOffset::west_opt(3 * 3600).unwrap();
        let now = offset.with_ymd_and_hms(2026, 10, 2, 14, 0, 0).unwrap();
        let id = SessionId::from_parts(1_727_000_000_000, 1);
        store
            .create_meeting(&fala_storage::NewMeeting {
                id,
                title: String::new(),
                mode: SessionMode::Meeting,
                local_only: false,
                created_at: now,
            })
            .unwrap();
        assert_eq!(guard_edited_notes(&store, id, "geral", false), Ok(()));
        store.save_meeting_notes(id, "geral", "- gerada").unwrap();
        assert_eq!(guard_edited_notes(&store, id, "geral", false), Ok(()));
        store.save_edited_meeting_notes(id, "- minha", now).unwrap();
        assert_eq!(
            guard_edited_notes(&store, id, "geral", false),
            Err(MeetingError::NotesEdited)
        );
        assert_eq!(guard_edited_notes(&store, id, "geral", true), Ok(()));
        // Another template is a new version, not a regeneration: no question.
        assert_eq!(guard_edited_notes(&store, id, "um-a-um", false), Ok(()));
        let missing = SessionId::from_parts(1, 1);
        assert_eq!(
            guard_edited_notes(&store, missing, "geral", false),
            Err(MeetingError::NotFound)
        );
        let edited = store.meeting_notes_edited_at(id).unwrap();
        assert!(edited_since(edited, now));
        assert!(edited_since(
            edited,
            now + chrono::Duration::milliseconds(900)
        ));
        assert!(!edited_since(edited, now + chrono::Duration::seconds(1)));
        assert!(!edited_since(None, now));
        assert_eq!(
            serde_json::to_value(MeetingError::NotesEdited).unwrap(),
            serde_json::json!({ "kind": "notes_edited" })
        );
    }
}
