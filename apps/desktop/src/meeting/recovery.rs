//! A recording the process did not close (phase 2, F3; bug bar: "audio never lost in a crash").
//!
//! The recorder rewrites the WAV header every second (`fala_audio::FLUSH_EVERY`), so a killed
//! process leaves a WAV readable up to the last second. At startup a session that started, has no
//! end and still has its `recording.wav` becomes an orphan; nothing happens to it until the
//! person clicks "recover", because recovering may send the audio to the transcription
//! (ADR-0005). A session that did stop but whose WAV never became Opus is retained in the
//! background, as the stop would have done.

use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

use chrono::{DateTime, FixedOffset, Local};
use fala_meeting::SessionId;
use fala_storage::{MeetingRecord, MeetingTake, Store};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Manager;

use super::{parse_id, MeetingError, MeetingManager, WORK_WAV};

/// A recording the process left without an end: "Recover the recording from 14:02 (12 min)?".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct MeetingOrphan {
    pub id: String,
    pub title: String,
    /// RFC 3339 with the local offset at the time.
    pub started_at: String,
    /// What the WAV header holds.
    pub recorded_ms: u64,
}

/// Sessions that started, never ended and still have their working WAV (of the first take, or
/// of the take a resumed session was recording).
pub(crate) fn find_orphans(records: &[MeetingRecord], audio_root: &Path) -> Vec<SessionId> {
    records
        .iter()
        .filter(|r| r.started_at.is_some() && r.ended_at.is_none())
        .filter(|r| work_wav(audio_root, r.id).is_some())
        .map(|r| r.id)
        .collect()
}

/// The working WAV left behind: the one of the highest take `t<N>/` (the take that was
/// recording), or else the first take's. A first-take WAV whose conversion failed may stay
/// next to a later take: the take is the recording that did not end.
fn work_wav(audio_root: &Path, id: SessionId) -> Option<std::path::PathBuf> {
    let dir = fala_retention::session_audio_dir(audio_root, id);
    let first = Some(dir.join(WORK_WAV)).filter(|wav| wav.is_file());
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return first;
    };
    entries
        .filter_map(|entry| {
            let name = entry.ok()?.file_name();
            let take: u32 = name.to_str()?.strip_prefix('t')?.parse().ok()?;
            Some((take, dir.join(name).join(WORK_WAV)))
        })
        .filter(|(_, wav)| wav.is_file())
        .max_by_key(|(take, _)| *take)
        .map(|(_, wav)| wav)
        .or(first)
}

/// Recorded milliseconds the WAV header announces.
pub(crate) fn wav_recorded_ms(wav: &Path) -> Result<u64, MeetingError> {
    let reader = hound::WavReader::open(wav).map_err(|e| MeetingError::Storage(e.to_string()))?;
    Ok(super::recorder::recorded_ms(u64::from(reader.duration())))
}

/// When the WAV was last written: the end of a recording nobody stopped.
fn wav_ended_at(wav: &Path) -> DateTime<FixedOffset> {
    let modified = std::fs::metadata(wav)
        .and_then(|m| m.modified())
        .unwrap_or_else(|_| SystemTime::now());
    DateTime::<Local>::from(modified).fixed_offset()
}

/// The banner of an orphan; `take` is the take a resumed session was recording, whose start
/// goes with the minutes of its WAV.
pub(crate) fn orphan(
    record: &MeetingRecord,
    take: Option<&MeetingTake>,
    audio_root: &Path,
) -> MeetingOrphan {
    let started = take
        .map(|take| take.started_at)
        .or(record.started_at)
        .unwrap_or(record.created_at);
    MeetingOrphan {
        id: record.id.to_string(),
        title: record.title.clone(),
        started_at: started.to_rfc3339_opts(chrono::SecondsFormat::Secs, false),
        recorded_ms: work_wav(audio_root, record.id)
            .and_then(|wav| wav_recorded_ms(&wav).ok())
            .unwrap_or(0),
    }
}

/// Closes an orphan in the store from its WAV: end at the last write, duration from the header.
/// A take left open is closed as a take (returned), so the session keeps what it had before.
pub(crate) fn recover(
    store: &mut Store,
    audio_root: &Path,
    id: SessionId,
) -> Result<Option<MeetingTake>, MeetingError> {
    let wav = work_wav(audio_root, id).ok_or(MeetingError::NoAudio)?;
    let recorded_ms = wav_recorded_ms(&wav)?;
    let open = store
        .meeting_takes(id)?
        .into_iter()
        .any(|take| take.ended_at.is_none());
    let take = if open {
        Some(store.finish_meeting_take(id, wav_ended_at(&wav), recorded_ms, None)?)
    } else {
        store.recover_meeting(id, wav_ended_at(&wav), recorded_ms)?;
        None
    };
    log::info!("meeting {id}: recovered ({recorded_ms} ms)");
    Ok(take)
}

impl MeetingManager {
    /// The recordings found without an end at startup that still have none.
    pub fn orphans(&self) -> Result<Vec<MeetingOrphan>, MeetingError> {
        let ids = self
            .orphans
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        self.with_store(|store| {
            Ok(ids
                .iter()
                .filter_map(|id| store.meeting(*id).ok())
                .filter(|record| record.ended_at.is_none())
                .map(|record| {
                    let takes = store.meeting_takes(record.id).unwrap_or_default();
                    let open = takes.iter().find(|take| take.ended_at.is_none());
                    orphan(&record, open, &self.audio_root)
                })
                .collect())
        })
    }

    /// "Recover": the end comes from the WAV, then the session goes on as after a stop
    /// (retained audio, then the transcript when the key is there).
    pub fn recover(&self, id: &str) -> Result<(), MeetingError> {
        let id = parse_id(id)?;
        if !self.is_orphan(id) {
            return Err(MeetingError::NotFound);
        }
        let take = self.with_store(|store| recover(store, &self.audio_root, id))?;
        self.forget_orphan(id);
        let app = self.app.clone();
        let spawned = std::thread::Builder::new()
            .name("meeting-recover".to_string())
            .spawn(move || {
                if let Some(manager) = app.try_state::<Arc<MeetingManager>>() {
                    match &take {
                        Some(take) => {
                            let dir = manager.session_dir(id).join(format!("t{}", take.take));
                            manager.retain(id, &dir);
                            manager.transcribe_take_if_keyed(id, take);
                        }
                        None => {
                            manager.retain(id, &manager.session_dir(id));
                            manager.transcribe_if_keyed(id);
                        }
                    }
                }
            });
        if let Err(e) = spawned {
            // The end is saved; "transcribe" retains the WAV first.
            log::error!("meeting {id}: recovery thread did not start: {e}");
        }
        Ok(())
    }

    /// Sessions that stopped but whose WAV never became Opus (the process died while
    /// retaining, or "Stop and quit") are retained now and, without a transcript, go on as
    /// after a stop.
    pub(crate) fn retain_stopped(&self) {
        let ended = self.with_store(|store| {
            Ok(store
                .meetings()?
                .into_iter()
                .filter(|record| record.ended_at.is_some())
                .collect::<Vec<_>>())
        });
        for record in ended.unwrap_or_default() {
            let id = record.id;
            let dir = self.session_dir(id);
            if dir.join(WORK_WAV).is_file() {
                // Also a WAV left next to validated Opus files (killed before deleting it):
                // `retain` validates again and only then deletes it (ADR-0014).
                log::info!("meeting {id}: retaining the audio a previous run left as WAV");
                self.retain(id, &dir);
                // A WAV that still does not convert is not sent anywhere.
                if record.transcribed_at.is_none() && !super::pipeline::needs_retain(&dir) {
                    self.transcribe_if_keyed(id);
                }
            } else if !record.audio_retained && super::clip::audio_available(&dir) {
                // Killed between the Opus rename and the store update.
                if let Err(e) = self.with_store(|store| Ok(store.set_meeting_audio_retained(id)?)) {
                    log::warn!("meeting {id}: retained audio not marked: {e}");
                }
            }
            // The takes of a resumed session (take 2 is always the first): a WAV left in `t<N>/` is
            // retained too and, if its transcript never came, transcribed as after its stop.
            if dir.join("t2").is_dir() {
                self.retain_stopped_takes(id);
            }
        }
    }

    fn retain_stopped_takes(&self, id: SessionId) {
        let takes = self
            .with_store(|store| Ok(store.meeting_takes(id)?))
            .unwrap_or_default();
        for take in takes.iter().filter(|take| take.ended_at.is_some()) {
            let dir = self.session_dir(id).join(format!("t{}", take.take));
            if !dir.join(WORK_WAV).is_file() {
                continue;
            }
            log::info!(
                "meeting {id}: retaining take {} a previous run left as WAV",
                take.take
            );
            self.retain(id, &dir);
            let transcribed = self
                .with_store(|store| Ok(store.meeting_segments(id)?))
                .unwrap_or_default()
                .iter()
                .any(|s| !s.speaker.is_marker() && s.t0_ms >= take.offset_ms);
            if !transcribed && !super::pipeline::needs_retain(&dir) {
                self.transcribe_take_if_keyed(id, take);
            }
        }
    }

    fn is_orphan(&self, id: SessionId) -> bool {
        self.orphans
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(&id)
    }

    pub(crate) fn forget_orphan(&self, id: SessionId) {
        self.orphans
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|orphan| *orphan != id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use fala_meeting::{SessionMode, StopReason};
    use fala_storage::NewMeeting;

    fn at(h: u32, m: u32) -> DateTime<FixedOffset> {
        FixedOffset::west_opt(3 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, 10, 2, h, m, 0)
            .unwrap()
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
                created_at: at(9, 0),
            })
            .unwrap();
        if let Some(started) = started {
            store
                .start_meeting_recording(id, SessionMode::Meeting, started)
                .unwrap();
        }
        id
    }

    fn first_wav(audio: &Path, id: SessionId) -> std::path::PathBuf {
        fala_retention::session_audio_dir(audio, id).join(WORK_WAV)
    }

    /// A working WAV of `seconds` as the recorder leaves it when killed: the header of the last
    /// flush, then frames written after it that the header never announced.
    fn killed_wav(path: &Path, seconds: u32) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for _ in 0..seconds * 48_000 {
            writer.write_sample(1_000i16).unwrap();
            writer.write_sample(-1_000i16).unwrap();
        }
        writer.flush().unwrap();
        for _ in 0..24_000 {
            writer.write_sample(7i16).unwrap();
            writer.write_sample(7i16).unwrap();
        }
        // The process dies here: no finalize, no header update for the last half second.
        std::mem::forget(writer);
    }

    #[test]
    fn orphan_is_a_started_unended_session_with_its_wav() {
        let (_dir, store, audio) = setup();
        let orphan = session(&store, 1, Some(at(14, 2)));
        let ended = session(&store, 2, Some(at(15, 0)));
        store
            .finish_meeting(ended, at(16, 0), 1_000, StopReason::User)
            .unwrap();
        let _no_wav = session(&store, 3, Some(at(17, 0)));
        let draft = session(&store, 4, None);
        for id in [orphan, ended, draft] {
            killed_wav(&first_wav(&audio, id), 1);
        }
        assert_eq!(
            find_orphans(&store.meetings().unwrap(), &audio),
            vec![orphan]
        );
    }

    #[test]
    fn recovery_keeps_what_the_header_announced() {
        let (_dir, mut store, audio) = setup();
        let id = session(&store, 1, Some(at(14, 2)));
        let wav = first_wav(&audio, id);
        killed_wav(&wav, 10);
        let shown = orphan(&store.meeting(id).unwrap(), None, &audio);
        assert_eq!(shown.recorded_ms, 10_000);
        assert_eq!(shown.started_at, "2026-10-02T14:02:00-03:00");

        recover(&mut store, &audio, id).unwrap();
        let record = store.meeting(id).unwrap();
        assert_eq!(record.recorded_ms, 10_000);
        assert_eq!(record.stop_reason, None);
        assert!(record.ended_at.is_some());
        // The WAV stays for the retention to turn into Opus.
        assert!(wav.is_file());
        assert!(find_orphans(&store.meetings().unwrap(), &audio).is_empty());
        // Recovering twice is refused, and a session without its WAV has nothing to recover.
        assert!(recover(&mut store, &audio, id).is_err());
        let gone = session(&store, 2, Some(at(15, 0)));
        assert_eq!(
            recover(&mut store, &audio, gone),
            Err(MeetingError::NoAudio)
        );
    }

    #[test]
    fn a_killed_wav_becomes_opus_without_losing_more_than_a_second() {
        let (_dir, mut store, audio) = setup();
        let id = session(&store, 1, Some(at(14, 2)));
        let wav = first_wav(&audio, id);
        killed_wav(&wav, 3);
        recover(&mut store, &audio, id).unwrap();
        let retained = fala_retention::retain_wav(&wav, wav.parent().unwrap()).unwrap();
        // 3 s announced, half a second after the last flush lost: within the 1 s budget.
        assert_eq!(retained.samples, 3 * 48_000);
        assert!(!wav.exists());
    }

    #[test]
    fn a_take_left_open_is_recovered_as_a_take() {
        let (_dir, mut store, audio) = setup();
        let id = session(&store, 1, Some(at(14, 2)));
        store
            .finish_meeting(id, at(14, 3), 60_000, StopReason::User)
            .unwrap();
        let take = store.start_meeting_take(id, at(15, 42)).unwrap();
        // The process dies 4 s into take 2: its WAV is in `t2/`.
        let wav = fala_retention::session_audio_dir(&audio, id)
            .join("t2")
            .join(WORK_WAV);
        killed_wav(&wav, 4);
        assert_eq!(find_orphans(&store.meetings().unwrap(), &audio), vec![id]);
        // The banner gives the take's start with the take's minutes.
        let shown = orphan(&store.meeting(id).unwrap(), Some(&take), &audio);
        assert_eq!(shown.recorded_ms, 4_000);
        assert_eq!(shown.started_at, "2026-10-02T15:42:00-03:00");

        let closed = recover(&mut store, &audio, id).unwrap().unwrap();
        assert_eq!((closed.take, closed.offset_ms), (take.take, 60_000));
        let record = store.meeting(id).unwrap();
        assert_eq!(record.recorded_ms, 64_000);
        assert!(record.ended_at.is_some());
        assert!(find_orphans(&store.meetings().unwrap(), &audio).is_empty());
    }
}
