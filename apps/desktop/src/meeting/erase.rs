//! "Erase stretch" on a finished session: `[t0, t1]` leaves the audio (zeroed in both channels,
//! the timeline kept) and the transcript (the lines that cross it give way to one marker), and
//! the notes' pointers follow the renumbered transcript. With the same refusals as delete:
//! never a session that records or that a transcription or notes generation is reading.

use std::path::Path;

use fala_core::Language;
use fala_meeting::SessionId;
use fala_storage::Store;
use serde::{Deserialize, Serialize};
use specta::Type;

use super::clip::TakeAudio;
use super::{parse_id, MeetingError, MeetingManager, WORK_WAV};

/// The most lines one stretch takes (Granola's limit).
pub(crate) const MAX_LINES: usize = 500;

/// What erasing a stretch did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct MeetingErased {
    /// Lines that left the transcript.
    pub removed: u32,
    /// Lines of the notes that cited one of them and now say "(trecho apagado)".
    pub notes_lines: u32,
    /// The session had audio, and it was zeroed too.
    pub audio: bool,
}

/// The stretch to erase, widened to the lines it takes: a line that crosses an edge leaves
/// whole, so its audio goes whole too (and the widening may take one more line). Refuses a
/// session in the trash and a stretch of more than [`MAX_LINES`] lines.
fn span(
    store: &Store,
    id: SessionId,
    (mut t0_ms, mut t1_ms): (u64, u64),
) -> Result<(u64, u64), MeetingError> {
    if store.meeting_trashed_at(id)?.is_some() {
        return Err(MeetingError::NotFound);
    }
    let segments = store.meeting_segments(id)?;
    let spoken: Vec<_> = segments.iter().filter(|s| !s.speaker.is_marker()).collect();
    loop {
        let taken: Vec<_> = spoken.iter().filter(|s| s.crosses(t0_ms, t1_ms)).collect();
        if taken.len() > MAX_LINES {
            return Err(MeetingError::EraseTooLong);
        }
        let wide = taken
            .iter()
            .fold((t0_ms, t1_ms), |(a, b), s| (a.min(s.t0_ms), b.max(s.t1_ms)));
        if wide == (t0_ms, t1_ms) {
            return Ok(wide);
        }
        (t0_ms, t1_ms) = wide;
    }
}

/// Zeroes the stretch in the audio the session still has: the working WAV a retention has not
/// turned into Opus yet and the two Opus files. Returns whether there was any.
fn zero_audio(dir: &Path, t0_ms: u64, t1_ms: u64) -> Result<bool, MeetingError> {
    let failed = |e: fala_retention::RetentionError| MeetingError::Storage(e.to_string());
    let wav = dir.join(WORK_WAV);
    let wav_found = wav.is_file();
    if wav_found {
        fala_retention::zero_wav(&wav, t0_ms, t1_ms).map_err(failed)?;
    }
    // Even one Opus left alone (the other lost) holds the stretch: `zero_range` takes what is there.
    let opus = [fala_retention::MIC_FILE, fala_retention::SYSTEM_FILE]
        .iter()
        .any(|name| dir.join(name).is_file());
    if opus {
        fala_retention::zero_range(dir, t0_ms, t1_ms).map_err(failed)?;
    }
    Ok(wav_found || opus)
}

/// Zeroes `[t0, t1]` (on the session's timeline) in each take it crosses, in that take's own
/// time. Returns whether any take had audio.
fn zero_takes(takes: &[TakeAudio], t0_ms: u64, t1_ms: u64) -> Result<bool, MeetingError> {
    let mut audio = false;
    for (index, take) in takes.iter().enumerate() {
        let end = takes.get(index + 1).map_or(u64::MAX, |next| next.offset_ms);
        if t1_ms <= take.offset_ms || t0_ms >= end {
            continue;
        }
        let local = |ms: u64| ms.clamp(take.offset_ms, end) - take.offset_ms;
        audio |= zero_audio(&take.dir, local(t0_ms), local(t1_ms))?;
    }
    Ok(audio)
}

/// The text, after the audio: an audio that fails leaves the text, and trying again zeroes the
/// same samples once more.
fn erase_text(
    store: &mut Store,
    id: SessionId,
    t0_ms: u64,
    t1_ms: u64,
    audio: bool,
    language: Language,
) -> Result<MeetingErased, MeetingError> {
    let erased = fala_notes::erased_pointer(language);
    let erasure = store.erase_meeting_range(id, (t0_ms, t1_ms), |md, map| {
        fala_notes::remap_pointers(md, map, erased)
    })?;
    Ok(MeetingErased {
        removed: u32::try_from(erasure.removed).unwrap_or(u32::MAX),
        notes_lines: u32::try_from(erasure.notes_lines).unwrap_or(u32::MAX),
        audio,
    })
}

/// Clears the erase mark of `Work` however the erase ends.
struct Erasing<'a>(&'a std::sync::Mutex<super::pipeline::Work>);

impl Drop for Erasing<'_> {
    fn drop(&mut self) {
        let mut work = self.0.lock().unwrap_or_else(|e| e.into_inner());
        work.set_erasing(None);
    }
}

impl MeetingManager {
    /// "Erase stretch", after its confirmation. The audio is re-encoded outside the store lock
    /// (an hour takes seconds), with no job allowed to start on the session meanwhile.
    pub fn erase_range(
        &self,
        id: &str,
        t0_ms: u64,
        t1_ms: u64,
    ) -> Result<MeetingErased, MeetingError> {
        let id = parse_id(id)?;
        let (t0_ms, t1_ms) = (t0_ms.min(t1_ms), t0_ms.max(t1_ms));
        // Lock order as in `delete`: `retaining`, then `work`, then the store. `retaining` is held
        // to the end (no conversion or retention touches the folder meanwhile); `work` only marks
        // the session, so pages and jobs of other sessions do not wait for the re-encode.
        let _retaining = self.retaining.lock().unwrap_or_else(|e| e.into_inner());
        let (t0_ms, t1_ms) = {
            let mut work = self.work.lock().unwrap_or_else(|e| e.into_inner());
            if self.is_live(id) {
                return Err(MeetingError::StillRecording);
            }
            if work.involves(id) {
                return Err(MeetingError::Busy);
            }
            let wide = self.with_store(|store| span(store, id, (t0_ms, t1_ms)))?;
            work.set_erasing(Some(id));
            wide
        };
        let _erasing = Erasing(&self.work);
        let audio = zero_takes(&self.takes_audio(id)?, t0_ms, t1_ms)?;
        let settings = crate::settings::get_settings(&self.app);
        let language = super::context::notes_settings(&settings).language;
        let erased =
            self.with_store(|store| erase_text(store, id, t0_ms, t1_ms, audio, language))?;
        log::info!(
            "meeting {id}: {} lines erased, audio {}",
            erased.removed,
            if audio { "zeroed" } else { "absent" }
        );
        Ok(erased)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, TimeZone};
    use fala_meeting::{SessionMode, StopReason};
    use fala_storage::{MeetingSegment, NewMeeting, SegmentChannel, SegmentSpeaker};

    fn segment(t0_ms: u64, text: &str) -> MeetingSegment {
        MeetingSegment {
            seq: 0,
            channel: SegmentChannel::System,
            speaker: SegmentSpeaker::Person(1),
            t0_ms,
            t1_ms: t0_ms + 900,
            text: text.to_string(),
        }
    }

    #[test]
    fn erase_zeroes_the_working_wav_and_takes_the_text() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("fala.sqlite"), dir.path()).unwrap();
        let at = FixedOffset::west_opt(3 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, 10, 2, 14, 0, 0)
            .unwrap();
        let id = SessionId::from_parts(1_800_000_000_000, 7);
        store
            .create_meeting(&NewMeeting {
                id,
                title: "r".into(),
                mode: SessionMode::Meeting,
                local_only: false,
                created_at: at,
            })
            .unwrap();
        store
            .start_meeting_recording(id, SessionMode::Meeting, at)
            .unwrap();
        store
            .finish_meeting(id, at, 3_000, StopReason::User)
            .unwrap();
        let lines = (0..=MAX_LINES as u64)
            .map(|i| segment(i * 1_000, "oi"))
            .collect();
        store.replace_meeting_segments(id, lines, at).unwrap();

        // One line over the limit is refused before anything is touched.
        let all = (0, (MAX_LINES as u64 + 1) * 1_000);
        assert_eq!(span(&store, id, all), Err(MeetingError::EraseTooLong));
        // A stretch that cuts two lines in half takes both whole, audio included.
        assert_eq!(span(&store, id, (1_500, 2_100)), Ok((1_000, 2_900)));

        // The retention has not run: the WAV is the audio, zeroed in place.
        let session = dir.path().join("audio");
        std::fs::create_dir_all(&session).unwrap();
        let wav = session.join(WORK_WAV);
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&wav, spec).unwrap();
        for _ in 0..3 * 48_000 * 2 {
            writer.write_sample(1_000i16).unwrap();
        }
        writer.finalize().unwrap();
        assert!(zero_audio(&session, 1_000, 2_000).unwrap());
        let samples: Vec<i16> = hound::WavReader::open(&wav)
            .unwrap()
            .into_samples::<i16>()
            .map(Result::unwrap)
            .collect();
        assert_eq!(samples[48_000 * 2 - 1], 1_000);
        assert!(samples[48_000 * 2..96_000 * 2].iter().all(|&s| s == 0));
        assert_eq!(samples[96_000 * 2], 1_000);
        assert!(!zero_audio(&dir.path().join("none"), 0, 10).unwrap());

        let erased = erase_text(&mut store, id, 1_000, 2_900, true, Language::PtBr).unwrap();
        assert_eq!(
            erased,
            MeetingErased {
                removed: 2,
                notes_lines: 0,
                audio: true
            }
        );
        // In the trash, the session is not found.
        store.trash_meeting(id, at).unwrap();
        assert_eq!(span(&store, id, (0, 10)), Err(MeetingError::NotFound));
    }

    #[test]
    fn a_stretch_across_two_takes_is_zeroed_in_each() {
        let dir = tempfile::tempdir().unwrap();
        let wav = |dir: &Path| {
            std::fs::create_dir_all(dir).unwrap();
            let spec = hound::WavSpec {
                channels: 2,
                sample_rate: 48_000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            let mut writer = hound::WavWriter::create(dir.join(WORK_WAV), spec).unwrap();
            for _ in 0..3 * 48_000 * 2 {
                writer.write_sample(1_000i16).unwrap();
            }
            writer.finalize().unwrap();
        };
        let takes = [
            TakeAudio {
                offset_ms: 0,
                dir: dir.path().to_path_buf(),
            },
            TakeAudio {
                offset_ms: 3_000,
                dir: dir.path().join("t2"),
            },
        ];
        for take in &takes {
            wav(&take.dir);
        }
        // 2-4 s of the session: 2-3 s of take 1 and 0-1 s of take 2.
        assert!(zero_takes(&takes, 2_000, 4_000).unwrap());
        let zeroed = |dir: &Path| -> Vec<bool> {
            let samples: Vec<i16> = hound::WavReader::open(dir.join(WORK_WAV))
                .unwrap()
                .into_samples::<i16>()
                .map(Result::unwrap)
                .collect();
            (0..3).map(|s| samples[s * 96_000 + 10] == 0).collect()
        };
        assert_eq!(zeroed(&takes[0].dir), [false, false, true]);
        assert_eq!(zeroed(&takes[1].dir), [true, false, false]);
    }
}
