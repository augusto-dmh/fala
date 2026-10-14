//! A stretch of a meeting's retained audio as an in-memory WAV, for the ▶ of the meetings page
//! (phase 2 F9). Read-only: the audio never leaves the machine (ADR-0005, ADR-0014).

use std::io::Cursor;
use std::path::{Path, PathBuf};

use fala_retention::{decode_range, MIC_FILE, SAMPLE_RATE_HZ, SYSTEM_FILE};
use fala_storage::MeetingTake;

use super::MeetingError;

/// Played before and after the segment, so the first and last words are not cut.
pub(crate) const MARGIN_MS: u64 = 500;
/// Longest clip, margins included.
pub(crate) const MAX_CLIP_MS: u64 = 60_000;

/// Both retained channels are on disk (deleted by retention, or never retained, otherwise).
pub(crate) fn audio_available(dir: &Path) -> bool {
    dir.join(MIC_FILE).is_file() && dir.join(SYSTEM_FILE).is_file()
}

/// Where a take's audio lives and where it starts on the session's timeline: take 1 in
/// `audio/<id>/`, take N of a resumed session in `audio/<id>/t<N>/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TakeAudio {
    pub offset_ms: u64,
    pub dir: PathBuf,
}

/// Every take of the session, the first one included, in order.
pub(crate) fn takes_audio(root: &Path, takes: &[MeetingTake]) -> Vec<TakeAudio> {
    std::iter::once(TakeAudio {
        offset_ms: 0,
        dir: root.to_path_buf(),
    })
    .chain(takes.iter().map(|take| TakeAudio {
        offset_ms: take.offset_ms,
        dir: root.join(format!("t{}", take.take)),
    }))
    .collect()
}

/// Every take has both retained channels.
pub(crate) fn session_audio_available(takes: &[TakeAudio]) -> bool {
    takes.iter().all(|take| audio_available(&take.dir))
}

/// The clip of `[t0, t1]` from the take `t0` falls in (the last one starting at or before it),
/// cut at the start of the next: a clip never crosses takes.
pub(crate) fn clip_session(
    takes: &[TakeAudio],
    t0_ms: u64,
    t1_ms: u64,
) -> Result<Vec<u8>, MeetingError> {
    let index = takes
        .iter()
        .rposition(|take| take.offset_ms <= t0_ms)
        .unwrap_or(0);
    let take = takes.get(index).ok_or(MeetingError::NoAudio)?;
    let end = takes.get(index + 1).map_or(u64::MAX, |next| next.offset_ms);
    clip_wav(
        &take.dir,
        t0_ms.saturating_sub(take.offset_ms),
        t1_ms.min(end).saturating_sub(take.offset_ms),
    )
}

/// `[start, end)` in ms of the clip for a segment `[t0, t1]`: the margins, capped at a minute.
pub(crate) fn window(t0_ms: u64, t1_ms: u64) -> (u64, u64) {
    let start = t0_ms.saturating_sub(MARGIN_MS);
    let end = t1_ms
        .max(t0_ms)
        .saturating_add(MARGIN_MS)
        .min(start + MAX_CLIP_MS);
    (start, end)
}

/// Mono 48 kHz 16-bit WAV of `mic.opus` + `sys.opus` around `[t0, t1]`. A stretch where
/// dictation was omitted plays the silence that is in the file (ADR-0015).
pub(crate) fn clip_wav(dir: &Path, t0_ms: u64, t1_ms: u64) -> Result<Vec<u8>, MeetingError> {
    if !audio_available(dir) {
        return Err(MeetingError::NoAudio);
    }
    let (start, end) = window(t0_ms, t1_ms);
    let samples = |ms: u64| ms * u64::from(SAMPLE_RATE_HZ) / 1_000;
    let channel = |file: &str| {
        decode_range(&dir.join(file), samples(start), samples(end))
            .map_err(|e| MeetingError::Storage(e.to_string()))
    };
    wav(&mix(&channel(MIC_FILE)?, &channel(SYSTEM_FILE)?))
}

/// Sum of the two channels, saturated: "me" and "them" rarely overlap, and an average would
/// halve each voice.
fn mix(mic: &[i16], system: &[i16]) -> Vec<i16> {
    (0..mic.len().max(system.len()))
        .map(|i| {
            let a = mic.get(i).copied().unwrap_or(0);
            let b = system.get(i).copied().unwrap_or(0);
            a.saturating_add(b)
        })
        .collect()
}

fn wav(pcm: &[i16]) -> Result<Vec<u8>, MeetingError> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE_HZ,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let failed = |e: hound::Error| MeetingError::Storage(e.to_string());
    let mut out = Cursor::new(Vec::with_capacity(44 + pcm.len() * 2));
    let mut writer = hound::WavWriter::new(&mut out, spec).map_err(failed)?;
    for &sample in pcm {
        writer.write_sample(sample).map_err(failed)?;
    }
    writer.finalize().map_err(failed)?;
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 4 s working WAV with a constant level on each channel, retained as the two Opus files.
    fn session(dir: &Path) {
        let wav = dir.join("work.wav");
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: SAMPLE_RATE_HZ,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&wav, spec).unwrap();
        for i in 0..4 * SAMPLE_RATE_HZ {
            let tone = ((i as f32 * 0.13).sin() * 8_000.0) as i16;
            writer.write_sample(tone).unwrap();
            writer.write_sample(tone / 2).unwrap();
        }
        writer.finalize().unwrap();
        fala_retention::retain_wav(&wav, dir).unwrap();
    }

    #[test]
    fn window_adds_margins_and_caps_at_a_minute() {
        assert_eq!(window(10_000, 12_000), (9_500, 12_500));
        assert_eq!(window(200, 900), (0, 1_400));
        assert_eq!(window(5_000, 4_000), (4_500, 5_500), "t1 before t0");
        assert_eq!(window(10_000, 200_000), (9_500, 69_500));
        assert_eq!(window(0, 120_000), (0, MAX_CLIP_MS));
    }

    #[test]
    fn clip_is_a_mono_48k_wav_of_the_window() {
        let tmp = tempfile::tempdir().unwrap();
        session(tmp.path());
        assert!(audio_available(tmp.path()));
        let bytes = clip_wav(tmp.path(), 1_000, 2_000).unwrap();
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..16], b"WAVEfmt ");
        let reader = hound::WavReader::new(Cursor::new(&bytes)).unwrap();
        let spec = reader.spec();
        assert_eq!(
            (spec.channels, spec.sample_rate, spec.bits_per_sample),
            (1, 48_000, 16)
        );
        // 0.5 s + 1 s + 0.5 s.
        assert_eq!(reader.duration(), 2 * SAMPLE_RATE_HZ);
        let peak = reader
            .into_samples::<i16>()
            .map(|s| s.unwrap().unsigned_abs())
            .max()
            .unwrap();
        assert!(peak > 9_000, "mic and system mixed, peak {peak}");

        // Clipped at the end of the 4 s file.
        let tail = clip_wav(tmp.path(), 3_000, 30_000).unwrap();
        let reader = hound::WavReader::new(Cursor::new(&tail)).unwrap();
        assert_eq!(reader.duration(), 3 * SAMPLE_RATE_HZ / 2);
    }

    #[test]
    fn a_line_plays_from_its_own_take() {
        let tmp = tempfile::tempdir().unwrap();
        session(tmp.path());
        let at = chrono::DateTime::parse_from_rfc3339("2026-10-02T15:42:00-03:00").unwrap();
        let take = MeetingTake {
            take: 2,
            offset_ms: 4_000,
            started_at: at,
            ended_at: None,
            recorded_ms: 4_000,
        };
        let takes = takes_audio(tmp.path(), std::slice::from_ref(&take));
        assert_eq!(takes[1].dir, tmp.path().join("t2"));
        // The second take has no audio yet: the session is not playable.
        assert!(!session_audio_available(&takes));
        std::fs::create_dir_all(&takes[1].dir).unwrap();
        session(&takes[1].dir);
        assert!(session_audio_available(&takes));

        let duration = |bytes: Vec<u8>| {
            hound::WavReader::new(Cursor::new(bytes))
                .unwrap()
                .duration()
        };
        // 5-6 s is 1-2 s of take 2, with both margins.
        assert_eq!(
            duration(clip_session(&takes, 5_000, 6_000).unwrap()),
            2 * SAMPLE_RATE_HZ
        );
        // A line at the end of take 1 is cut where take 2 starts (and at the end of its file).
        assert_eq!(
            duration(clip_session(&takes, 3_500, 4_800).unwrap()),
            SAMPLE_RATE_HZ
        );
    }

    #[test]
    fn session_without_audio_is_no_audio() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!audio_available(tmp.path()));
        assert_eq!(clip_wav(tmp.path(), 0, 1_000), Err(MeetingError::NoAudio));
        std::fs::write(tmp.path().join(MIC_FILE), b"OggS").unwrap();
        assert!(!audio_available(tmp.path()), "one channel is not enough");
        assert_eq!(clip_wav(tmp.path(), 0, 1_000), Err(MeetingError::NoAudio));
    }

    #[test]
    fn mix_saturates() {
        assert_eq!(
            mix(&[30_000, -30_000, 5], &[10_000, -10_000]),
            [i16::MAX, i16::MIN, 5]
        );
    }
}
