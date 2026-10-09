//! What happens to a stopped meeting: the transcript from ElevenLabs Scribe (ADR-0005), the
//! notes from the notes LLM (ADR-0016), and the views the Meetings page reads.
//!
//! Keys come only from the OS keyring (ADR-0008): `elevenlabs` for the transcript and the
//! `anthropic` key the post-processing settings already store, for the notes.

use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, FixedOffset};
use fala_asr::meeting::{
    CancelToken, Channel, ElevenLabsScribe, MeetingRecording, MeetingTranscriber, Progress,
    ScribeOptions, Segment, Speaker, Stage, MIC_FILE, SYSTEM_FILE,
};
use fala_core::{Dictionary, Language};
use fala_meeting::SessionId;
use fala_notes::{builtin_templates, generate_notes, Claude, NotesError, NotesInput, Template};
use fala_secrets::{ApiKey, SecretStore};
use fala_storage::{MeetingRecord, MeetingSegment, SegmentChannel, SegmentSpeaker};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use super::{parse_id, MeetingError, MeetingManager, WORK_WAV};

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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct MeetingTemplate {
    pub id: String,
    pub name: String,
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

/// The stored segments as the notes crate reads them: `seq` is the id the `^sN` pointers cite.
pub(crate) fn notes_segments(segments: &[MeetingSegment]) -> Vec<fala_notes::Segment> {
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
                SegmentSpeaker::Person(n) => fala_notes::Speaker::Person(n),
            },
            t0_ms: segment.t0_ms,
            t1_ms: segment.t1_ms,
            text: segment.text.clone(),
        })
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

pub(crate) fn find_template(id: &str) -> Result<Template, MeetingError> {
    builtin_templates()
        .map_err(|e| MeetingError::Notes(e.to_string()))?
        .into_iter()
        .find(|t| t.id == id)
        .ok_or(MeetingError::UnknownTemplate)
}

pub(crate) fn templates() -> Vec<MeetingTemplate> {
    builtin_templates()
        .unwrap_or_default()
        .into_iter()
        .map(|t| MeetingTemplate {
            id: t.id,
            name: t.name,
        })
        .collect()
}

/// What the notes LLM receives, from the stored session (ADR-0016 list: title, local date and
/// start time, language, dictionary, template, the typed notes, the transcript).
pub(crate) fn notes_input(
    record: &MeetingRecord,
    segments: &[MeetingSegment],
    language: Language,
    dictionary: Dictionary,
    template: Template,
) -> NotesInput {
    let start = record.started_at.unwrap_or(record.created_at);
    NotesInput {
        title: record.title.clone(),
        date: start.format("%Y-%m-%d").to_string(),
        start_time: start.format("%H:%M").to_string(),
        language,
        dictionary,
        template,
        annotations: record.annotations.clone(),
        transcript: notes_segments(segments),
        local_only: record.local_only,
    }
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

fn settings_language(app: &tauri::AppHandle) -> (Language, Dictionary) {
    let settings = crate::settings::get_settings(app);
    (
        crate::managers::history_dictations::language_from_setting(&settings.selected_language),
        Dictionary::new(settings.custom_words),
    )
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
        self.with_store(|store| {
            let record = store.meeting(id)?;
            let lines = store.meeting_segments(id)?.iter().map(line).collect();
            Ok(MeetingDetail {
                summary: summary(&record),
                annotations: record.annotations,
                lines,
                notes_md: record.notes_md,
                notes_template: record.notes_template,
            })
        })
    }

    /// "Copy Markdown": the note document with its transcript anchors.
    pub fn markdown(&self, id: &str) -> Result<String, MeetingError> {
        let id = parse_id(id)?;
        let (language, dictionary) = settings_language(&self.app);
        self.with_store(|store| {
            let record = store.meeting(id)?;
            let segments = store.meeting_segments(id)?;
            let template = find_template(record.notes_template.as_deref().unwrap_or("geral"))
                .or_else(|_| find_template("geral"))?;
            let input = notes_input(&record, &segments, language, dictionary, template);
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
        let (language, dictionary) = settings_language(&self.app);
        let mut scribe = ElevenLabsScribe::new(
            key,
            ScribeOptions {
                language: Some(language),
                keyterms: true,
                dictionary,
            },
        );
        let segments = scribe.transcribe_session(&recording, cancel, &mut |progress| {
            self.progress(id, progress_stage(progress));
        })?;
        let now = chrono::Local::now().fixed_offset();
        self.with_store(|store| {
            store.replace_meeting_segments(id, segments_for_store(segments), now)?;
            Ok(store.meeting_segments(id)?.iter().map(line).collect())
        })
    }

    /// Cancels the transcription in flight, if any; it returns `cancelled` within a second.
    pub fn cancel_transcription(&self) {
        let work = self.work.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((id, cancel)) = &work.transcribing {
            log::info!("meeting {id}: transcription cancelled");
            cancel.cancel();
        }
    }

    /// Generates the notes with a built-in template and stores the Markdown (ADR-0016).
    pub fn generate_notes(&self, id: &str, template_id: &str) -> Result<String, MeetingError> {
        let id = parse_id(id)?;
        let template = find_template(template_id)?;
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
        let (record, segments) =
            self.with_store(|store| Ok((store.meeting(id)?, store.meeting_segments(id)?)))?;
        let (language, dictionary) = settings_language(&self.app);
        let input = notes_input(&record, &segments, language, dictionary, template);
        self.progress(id, ProgressStage::Notes);
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
        self.with_store(|store| Ok(store.save_meeting_notes(id, template_id, &notes.markdown)?))?;
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
        let notes = notes_segments(&numbered);
        assert_eq!(notes[0].id, 1);
        assert_eq!(notes[0].speaker, fala_notes::Speaker::Me);
        assert_eq!(notes[0].channel, fala_notes::Channel::Mic);
        assert_eq!(notes[1].id, 2);
        assert_eq!(notes[1].speaker, fala_notes::Speaker::Person(2));
        assert_eq!(line(&numbered[0]).person, None);
        assert_eq!(line(&numbered[1]).person, Some(2));
        assert_eq!(line(&numbered[1]).channel, "system");
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
        let segments = vec![MeetingSegment {
            seq: 1,
            channel: SegmentChannel::Mic,
            speaker: SegmentSpeaker::Me,
            t0_ms: 0,
            t1_ms: 900,
            text: "oi".into(),
        }];
        let template = find_template("geral").unwrap();
        let input = notes_input(
            &record,
            &segments,
            Language::PtBr,
            Dictionary::new(["Fala"]),
            template.clone(),
        );
        assert_eq!(input.title, "Planejamento");
        assert_eq!(input.date, "2026-10-02");
        assert_eq!(input.start_time, "14:02");
        assert_eq!(input.language, Language::PtBr);
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
        assert_eq!(
            notes_input(&draft, &[], Language::En, Dictionary::default(), template).start_time,
            "13:50"
        );
        assert_eq!(
            find_template("nao-existe"),
            Err(MeetingError::UnknownTemplate)
        );
        assert!(templates().iter().any(|t| t.id == "um-a-um"));
    }
}
