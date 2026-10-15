//! A session after its recording (phase 2, F3): move it to the trash or delete it, and keep its
//! retained audio only as long as the setting says (ADR-0005: retained by default, retention
//! configurable).

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use fala_meeting::{Levels, SessionId, UnixMillis};
use fala_retention::{audio_due, RetainedSession, RetentionPolicy};
use fala_storage::{MeetingRecord, Store, TRASH_DAYS};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

use super::pipeline::MeetingSummary;
use super::{millis, parse_id, MeetingError, MeetingManager};

/// A session in the trash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct TrashedMeetingSummary {
    pub summary: MeetingSummary,
    /// When it was moved, RFC 3339 with the local offset.
    pub trashed_at: String,
}

/// How often the upkeep thread wakes up to see whether a day passed since the last retention.
/// Wall-clock: a monotonic sleep of 24 h does not advance while the PC is suspended.
const RETENTION_CHECK_EVERY: Duration = Duration::from_secs(3600);
const RETENTION_EVERY_MS: u64 = 24 * 3600 * 1000;

/// A stop shorter than this asks "Recorded by mistake?" (Fala transcribes after the stop, so it
/// has no word count to go by, unlike Wispr's 20 words).
pub(crate) const SHORT_RECORDING_MS: u64 = 30_000;
/// Or one with less speech than this, wherever it falls.
pub(crate) const LITTLE_SPEECH_MS: u64 = 5_000;
/// A second whose louder channel reaches this RMS (≈ −40 dBFS) counts as speech.
pub(crate) const SPEECH_RMS: f32 = 0.01;
/// A second of the recording counts as speech: its louder channel reached `SPEECH_RMS`. A
/// dictation zeroes the mic (ADR-0015), so then only the system channel counts.
pub(crate) fn heard_speech(levels: Levels) -> bool {
    let mic = if levels.mic_omitted { 0.0 } else { levels.mic };
    mic.max(levels.system) >= SPEECH_RMS
}

/// Under this, nothing was recorded at all.
const NOTHING_MS: u64 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mistake {
    /// No audio: discarded at once, saying "Nothing was recorded".
    NothingRecorded,
    /// Short or almost silent: "Recorded by mistake?" [Keep] [Discard].
    Ask,
}

/// What a stop the person clicked looks like. A typed agenda or title is never discarded
/// without asking.
pub(crate) fn mistake(recorded_ms: u64, voiced_ms: u64, typed: bool) -> Option<Mistake> {
    if recorded_ms < NOTHING_MS && !typed {
        Some(Mistake::NothingRecorded)
    } else if recorded_ms < SHORT_RECORDING_MS || voiced_ms < LITTLE_SPEECH_MS {
        Some(Mistake::Ask)
    } else {
        None
    }
}

/// The setting as a policy; an out-of-range number keeps the audio.
pub(crate) fn retention_policy(days: Option<u16>) -> RetentionPolicy {
    match days {
        None => RetentionPolicy::Keep,
        Some(days) => RetentionPolicy::delete_after_days(days).unwrap_or_else(|e| {
            log::warn!("meeting audio retention ignored: {e}");
            RetentionPolicy::Keep
        }),
    }
}

/// Sessions whose retained audio is due at `now`. Only audio that became Opus counts: a WAV that
/// never validated is the only copy and stays (ADR-0014). `busy` is being read right now.
pub(crate) fn audio_to_delete(
    records: &[MeetingRecord],
    policy: RetentionPolicy,
    now: UnixMillis,
    busy: Option<SessionId>,
) -> Vec<SessionId> {
    let sessions: Vec<RetainedSession> = records
        .iter()
        .filter(|r| r.audio_retained && Some(r.id) != busy)
        .map(|r| RetainedSession {
            id: r.id,
            ended_at: r.ended_at.map(millis),
            transcript_confirmed: false,
        })
        .collect();
    audio_due(policy, &sessions, now)
}

/// Deletes `audio/<id>/`. A folder that failed halfway (a file held open on Windows) no longer
/// has playable audio either, so it counts as gone; the next run retries what is left.
fn delete_audio(audio_root: &Path, id: SessionId) -> Result<(), (bool, MeetingError)> {
    fala_retention::delete_session_audio(audio_root, id)
        .map(|_| ())
        .map_err(|e| {
            let dir = fala_retention::session_audio_dir(audio_root, id);
            (
                !super::clip::audio_available(&dir),
                MeetingError::Storage(e.to_string()),
            )
        })
}

/// Deletes the folders of `due` (outside the store lock: it may be gigabytes) and returns the
/// sessions whose audio is now gone.
fn delete_due_audio(audio_root: &Path, due: Vec<SessionId>) -> Vec<SessionId> {
    due.into_iter()
        .filter(|id| match delete_audio(audio_root, *id) {
            Ok(()) => true,
            Err((gone, e)) => {
                log::warn!("meeting {id}: audio not fully deleted by the retention: {e}");
                gone
            }
        })
        .collect()
}

fn mark_audio_gone(store: &Store, gone: &[SessionId]) -> Result<(), MeetingError> {
    for id in gone {
        store.clear_meeting_audio_retained(*id)?;
        log::info!("meeting {id}: retained audio deleted by the retention setting");
    }
    Ok(())
}

/// Deletes a session whose days in the trash are up, checked again under the store lock: one
/// restored after the purge listed it stays. Its audio went when it was moved. Returns whether
/// it deleted.
fn purge(store: &Store, id: SessionId, now_ms: i64) -> Result<bool, MeetingError> {
    if !store.trash_is_due(id, now_ms)? {
        return Ok(false);
    }
    store.delete_meeting(id)?;
    Ok(true)
}

impl MeetingManager {
    /// "Delete meeting", after the literal confirmation in the UI: audio folder first, then the
    /// row and, by cascade, transcript, participants and speaker names (ADR-0005: no trash for
    /// audio). Never a session that records or is being finished, nor one a transcription or a
    /// notes generation is reading.
    pub fn delete(&self, id: &str) -> Result<(), MeetingError> {
        let id = parse_id(id)?;
        // Lock order: `retaining`, then `work`, then the store. A conversion of this session
        // finishes first (it would write the folder again), and no job can start on it while
        // the work slot is held.
        let _retaining = self.retaining.lock().unwrap_or_else(|e| e.into_inner());
        let work = self.work.lock().unwrap_or_else(|e| e.into_inner());
        if self.is_live(id) {
            return Err(MeetingError::StillRecording);
        }
        if work.involves(id) {
            return Err(MeetingError::Busy);
        }
        self.with_store(|store| Ok(store.meeting(id).map(|_| ())?))?;
        if let Err((gone, error)) = delete_audio(&self.audio_root, id) {
            if gone {
                self.with_store(|store| mark_audio_gone(store, &[id]))?;
            }
            return Err(error);
        }
        self.with_store(|store| Ok(store.delete_meeting(id)?))?;
        let _ = tauri::Emitter::emit(&self.app, super::DELETED_EVENT, id.to_string());
        self.forget_orphan(id);
        log::info!("meeting {id} deleted");
        Ok(())
    }

    /// "Move to trash": the audio goes now, with the same refusals and order as `delete`
    /// (ADR-0005: no trash for audio); transcript, annotations and notes stay `TRASH_DAYS` days.
    /// If the audio does not go, the session does not move.
    pub fn trash(&self, id: &str) -> Result<(), MeetingError> {
        let id = parse_id(id)?;
        let _retaining = self.retaining.lock().unwrap_or_else(|e| e.into_inner());
        let work = self.work.lock().unwrap_or_else(|e| e.into_inner());
        if self.is_live(id) {
            return Err(MeetingError::StillRecording);
        }
        if work.involves(id) {
            return Err(MeetingError::Busy);
        }
        self.with_store(|store| Ok(store.meeting(id).map(|_| ())?))?;
        if let Err((gone, error)) = delete_audio(&self.audio_root, id) {
            if gone {
                self.with_store(|store| mark_audio_gone(store, &[id]))?;
            }
            return Err(error);
        }
        self.with_store(|store| Ok(store.trash_meeting(id, super::now_local())?))?;
        let _ = tauri::Emitter::emit(&self.app, super::DELETED_EVENT, id.to_string());
        self.forget_orphan(id);
        log::info!("meeting {id} moved to the trash");
        Ok(())
    }

    /// "Restore" in the trash: the session is back on the page, without audio.
    pub fn restore(&self, id: &str) -> Result<(), MeetingError> {
        let id = parse_id(id)?;
        self.with_store(|store| Ok(store.restore_meeting(id)?))?;
        log::info!("meeting {id} restored from the trash");
        Ok(())
    }

    /// The sessions in the trash, last moved first.
    pub fn trashed(&self) -> Result<Vec<TrashedMeetingSummary>, MeetingError> {
        self.with_store(|store| {
            Ok(store
                .trashed_meetings()?
                .iter()
                .map(|t| TrashedMeetingSummary {
                    summary: super::pipeline::summary(&t.record),
                    trashed_at: super::pipeline::rfc3339(&t.trashed_at),
                })
                .collect())
        })
    }

    /// Deletes for good the sessions that have been in the trash for `TRASH_DAYS` days.
    pub fn purge_trash(&self) {
        let now = super::now_local().timestamp_millis();
        let due = match self.with_store(|store| Ok(store.trash_due(now)?)) {
            Ok(due) => due,
            Err(e) => {
                log::warn!("meeting trash was not emptied: {e}");
                return;
            }
        };
        for id in due {
            match self.with_store(|store| purge(store, id, now)) {
                Ok(true) => {
                    // Normally gone since the move; a folder left behind goes too.
                    if let Err((_, e)) = delete_audio(&self.audio_root, id) {
                        log::warn!("meeting {id}: audio left after the trash: {e}");
                    }
                    let _ = tauri::Emitter::emit(&self.app, super::DELETED_EVENT, id.to_string());
                    log::info!("meeting {id}: deleted after {TRASH_DAYS} days in the trash");
                }
                Ok(false) => {}
                Err(e) => log::warn!("meeting {id}: not deleted from the trash: {e}"),
            }
        }
    }

    /// After a stop the person clicked: discards an empty recording, or asks about one that
    /// looks like a mistake instead of sending it to the transcription. Returns whether it
    /// took over (no automatic transcript).
    pub(crate) fn handle_mistake(&self, id: SessionId, recorded_ms: u64, voiced_ms: u64) -> bool {
        let typed = self
            .with_store(|store| {
                let record = store.meeting(id)?;
                Ok(!record.title.trim().is_empty() || !record.annotations.trim().is_empty())
            })
            .unwrap_or(true);
        match mistake(recorded_ms, voiced_ms, typed) {
            None => false,
            Some(Mistake::NothingRecorded) => {
                if let Err(e) = self.delete(&id.to_string()) {
                    log::warn!("meeting {id}: empty recording not discarded: {e}");
                    return false;
                }
                crate::call_prompt::show_nothing_recorded(&self.app);
                true
            }
            Some(Mistake::Ask) => {
                let seconds = u32::try_from(recorded_ms / 1_000).unwrap_or(u32::MAX);
                crate::call_prompt::show_mistake(&self.app, id, seconds);
                true
            }
        }
    }

    /// [Keep] on "Recorded by mistake?": the session goes on as after any stop.
    pub fn keep_after_mistake(&self, id: &str) -> Result<(), MeetingError> {
        let id = parse_id(id)?;
        self.transcribe_if_keyed(id);
        Ok(())
    }

    /// Rebuilds the search index if it misses a session, then writes the `.md` of each meeting
    /// that is missing or differs from `fala.sqlite` (ADR-0006): sessions saved before the
    /// mirror, and writes that failed. The store is taken once per meeting for the files, so the
    /// page's commands are not held for the whole run.
    pub fn sync_mirrors(&self) {
        if let Err(e) = self.with_store(|store| Ok(store.index_meetings_for_search()?)) {
            log::warn!("meeting search index was not rebuilt: {e}");
        }
        let ids = match self.with_store(|store| Ok(store.meetings()?)) {
            Ok(meetings) => meetings.into_iter().map(|m| m.id),
            Err(e) => {
                log::warn!("meeting mirror sync did not run: {e}");
                return;
            }
        };
        let written = ids
            .filter(|id| {
                self.with_store(|store| Ok(store.sync_meeting_mirror(*id)))
                    .unwrap_or(false)
            })
            .count();
        if written > 0 {
            log::info!("meeting mirror: {written} files written");
        }
    }

    /// Deletes the retained audio the retention setting says is due now.
    pub fn run_retention(&self) {
        let days = crate::settings::get_settings(&self.app).meeting_audio_retention_days;
        let policy = retention_policy(days);
        if policy == RetentionPolicy::Keep {
            return;
        }
        // Held throughout: no transcription starts reading a folder this run deletes.
        let _retaining = self.retaining.lock().unwrap_or_else(|e| e.into_inner());
        let work = self.work.lock().unwrap_or_else(|e| e.into_inner());
        let now = millis(super::now_local());
        let result = self
            .with_store(|store| {
                Ok(audio_to_delete(
                    &store.meetings()?,
                    policy,
                    now,
                    work.transcribing(),
                ))
            })
            .map(|due| delete_due_audio(&self.audio_root, due))
            .and_then(|gone| self.with_store(|store| mark_audio_gone(store, &gone)));
        if let Err(e) = result {
            log::warn!("meeting audio retention did not run: {e}");
        }
    }

    /// At startup, in the background: the mirror catches up, the retention runs, the audio a
    /// previous run left as WAV is retained (and transcribed, as after a stop), the transcripts
    /// that waited for a retry go back to the queue, then the retention runs once a day.
    pub fn spawn_upkeep(app: &AppHandle) {
        let app = app.clone();
        let spawned = std::thread::Builder::new()
            .name("meeting-upkeep".to_string())
            .spawn(move || {
                let Some(manager) = app.try_state::<Arc<MeetingManager>>() else {
                    return;
                };
                manager.sync_mirrors();
                let mut last = millis(super::now_local());
                manager.run_retention();
                manager.purge_trash();
                manager.retain_stopped();
                manager.resume_pending_transcripts();
                loop {
                    std::thread::sleep(RETENTION_CHECK_EVERY);
                    let now = millis(super::now_local());
                    if now.0.saturating_sub(last.0) >= RETENTION_EVERY_MS {
                        last = now;
                        manager.run_retention();
                        manager.purge_trash();
                    }
                }
            });
        if let Err(e) = spawned {
            log::error!("meeting upkeep thread did not start: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, FixedOffset, TimeZone};
    use fala_meeting::{SessionMode, StopReason};
    use fala_storage::NewMeeting;

    const DAY_MS: u64 = 86_400_000;

    fn at(day: u32, h: u32, m: u32) -> DateTime<FixedOffset> {
        FixedOffset::west_opt(3 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, 10, day, h, m, 0)
            .unwrap()
    }

    /// What `run_retention` does, without the locks.
    fn apply_retention(
        store: &Store,
        audio: &Path,
        policy: RetentionPolicy,
        now: UnixMillis,
        busy: Option<SessionId>,
    ) -> Result<Vec<SessionId>, MeetingError> {
        let gone = delete_due_audio(
            audio,
            audio_to_delete(&store.meetings()?, policy, now, busy),
        );
        mark_audio_gone(store, &gone)?;
        Ok(gone)
    }

    /// What `MeetingManager::delete` does, without the locks and the live/busy guards.
    fn delete(store: &Store, audio: &Path, id: SessionId) -> Result<(), MeetingError> {
        store.meeting(id)?;
        delete_audio(audio, id).map_err(|(_, e)| e)?;
        Ok(store.delete_meeting(id)?)
    }

    fn setup() -> (tempfile::TempDir, Store, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("fala.sqlite"), dir.path()).unwrap();
        let audio = dir.path().join("audio");
        (dir, store, audio)
    }

    fn session(store: &Store, n: u64, started: Option<DateTime<FixedOffset>>) -> SessionId {
        let id = SessionId::from_parts(1_800_000_000_000 + n, u128::from(n));
        store
            .create_meeting(&NewMeeting {
                id,
                title: format!("r{n}"),
                mode: SessionMode::Meeting,
                local_only: false,
                created_at: at(1, 9, 0),
            })
            .unwrap();
        if let Some(started) = started {
            store
                .start_meeting_recording(id, SessionMode::Meeting, started)
                .unwrap();
        }
        id
    }

    #[test]
    fn retention_deletes_only_retained_audio_past_its_days() {
        let (_dir, store, audio) = setup();
        let now = millis(at(30, 12, 0));
        let ended = |n: u64, day: u32, retained: bool| {
            let id = session(&store, n, Some(at(day, 9, 0)));
            store
                .finish_meeting(id, at(day, 10, 0), 1_000, StopReason::User)
                .unwrap();
            let dir = fala_retention::session_audio_dir(&audio, id);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(fala_retention::MIC_FILE), b"opus").unwrap();
            if retained {
                store.set_meeting_audio_retained(id).unwrap();
            }
            id
        };
        let old = ended(1, 2, true);
        let recent = ended(2, 29, true);
        let old_unretained = ended(3, 2, false);
        let old_busy = ended(4, 3, true);

        let keep = retention_policy(None);
        assert!(apply_retention(&store, &audio, keep, now, None)
            .unwrap()
            .is_empty());
        // An out-of-range setting keeps everything too.
        assert_eq!(retention_policy(Some(0)), RetentionPolicy::Keep);

        let week = retention_policy(Some(7));
        let deleted = apply_retention(&store, &audio, week, now, Some(old_busy)).unwrap();
        assert_eq!(deleted, vec![old]);
        assert!(!fala_retention::session_audio_dir(&audio, old).exists());
        assert!(!store.meeting(old).unwrap().audio_retained);
        for kept in [recent, old_unretained, old_busy] {
            assert!(fala_retention::session_audio_dir(&audio, kept).exists());
        }
        // The row stays: only the audio goes.
        assert_eq!(store.meeting(old).unwrap().title, "r1");
        // A second run finds nothing left to delete for `old`.
        let again = apply_retention(&store, &audio, week, now, None).unwrap();
        assert_eq!(again, vec![old_busy]);

        // Exactly N days after the end is due; a minute less is not.
        let day = |ms: u64| UnixMillis(millis(at(29, 10, 0)).0 + ms);
        let records = store.meetings().unwrap();
        let one = retention_policy(Some(1));
        assert!(audio_to_delete(&records, one, day(DAY_MS - 60_000), None).is_empty());
        assert_eq!(
            audio_to_delete(&records, one, day(DAY_MS), None),
            vec![recent]
        );
    }

    #[test]
    fn a_short_or_silent_stop_looks_like_a_mistake() {
        assert_eq!(mistake(0, 0, false), Some(Mistake::NothingRecorded));
        // An agenda or a title typed before recording is never discarded without asking.
        assert_eq!(mistake(0, 0, true), Some(Mistake::Ask));
        assert_eq!(mistake(29_999, 29_999, false), Some(Mistake::Ask));
        assert_eq!(mistake(600_000, 4_999, false), Some(Mistake::Ask));
        assert_eq!(mistake(30_000, 5_000, false), None);
        assert_eq!(mistake(3_600_000, 1_200_000, true), None);
    }

    #[test]
    fn a_dictation_is_not_speech_of_the_meeting() {
        let levels = |mic, system, mic_omitted| Levels {
            mic,
            system,
            mic_omitted,
        };
        assert!(heard_speech(levels(0.2, 0.0, false)));
        assert!(
            !heard_speech(levels(0.2, 0.0, true)),
            "the dictation's voice"
        );
        assert!(
            heard_speech(levels(0.2, 0.2, true)),
            "the call still speaks"
        );
        assert!(!heard_speech(levels(0.0, 0.0, false)));
    }

    #[test]
    fn delete_removes_audio_and_row() {
        let (_dir, store, audio) = setup();
        let id = session(&store, 1, Some(at(2, 14, 0)));
        let other = session(&store, 2, Some(at(2, 15, 0)));
        for session in [id, other] {
            let dir = fala_retention::session_audio_dir(&audio, session);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(super::super::WORK_WAV), b"wav").unwrap();
            std::fs::write(dir.join(fala_retention::MIC_FILE), b"opus").unwrap();
        }
        delete(&store, &audio, id).unwrap();
        assert!(!fala_retention::session_audio_dir(&audio, id).exists());
        assert!(matches!(
            store.meeting(id),
            Err(fala_storage::StorageError::NotFound(_))
        ));
        assert!(fala_retention::session_audio_dir(&audio, other).exists());
        assert_eq!(delete(&store, &audio, id), Err(MeetingError::NotFound));
        // A draft has no audio folder and is deleted all the same.
        let draft = session(&store, 3, None);
        delete(&store, &audio, draft).unwrap();
    }

    #[test]
    fn trash_deletes_the_audio_now_and_the_rest_after_thirty_days() {
        let (_dir, mut store, audio) = setup();
        let id = session(&store, 1, Some(at(2, 14, 0)));
        let other = session(&store, 2, Some(at(2, 15, 0)));
        for session in [id, other] {
            store
                .finish_meeting(session, at(2, 16, 0), 1_000, StopReason::User)
                .unwrap();
            let dir = fala_retention::session_audio_dir(&audio, session);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(fala_retention::MIC_FILE), b"opus").unwrap();
            store.set_meeting_audio_retained(session).unwrap();
        }
        // What `MeetingManager::trash` does, without the locks and the live/busy guards.
        delete_audio(&audio, id).map_err(|(_, e)| e).unwrap();
        store.trash_meeting(id, at(3, 9, 0)).unwrap();

        assert!(!fala_retention::session_audio_dir(&audio, id).exists());
        assert!(fala_retention::session_audio_dir(&audio, other).exists());
        let record = store.meeting(id).unwrap();
        assert!(!record.audio_retained);
        assert_eq!(record.title, "r1");
        let listed: Vec<String> = crate::meeting::pipeline::list_in(&store)
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(listed, [other.to_string()]);

        // The purge takes the session only once its days are up, then deletes it for good.
        let ms = at(3, 9, 0).timestamp_millis() + TRASH_DAYS * 86_400_000;
        assert!(store.trash_due(ms - 3_600_000).unwrap().is_empty());
        assert!(!purge(&store, id, ms - 3_600_000).unwrap());
        assert_eq!(store.trash_due(ms).unwrap(), [id]);
        // Restored after the purge listed it: it stays.
        store.restore_meeting(id).unwrap();
        assert!(!purge(&store, id, ms).unwrap());
        store.trash_meeting(id, at(3, 9, 0)).unwrap();
        assert!(purge(&store, id, ms).unwrap());
        assert!(store.trashed_meetings().unwrap().is_empty());
        assert!(store.meeting(id).is_err());
    }
}
