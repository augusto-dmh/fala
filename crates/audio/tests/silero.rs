//! O Silero v4 versionado dentro do `DictationCapture`.
//!
//! O teste `#[ignore]` lê fala real de `FALA_TEST_SPEECH_WAV` (16 kHz mono i16, falha se faltar).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use fala_audio::{DictationCapture, SileroVad};

fn silero() -> SileroVad {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/desktop/resources/models/silero_vad_v4.onnx");
    SileroVad::load(&path).unwrap()
}

fn utterances(rate: u32, samples: &[f32]) -> usize {
    let mut capture = DictationCapture::new(rate, Box::new(silero())).unwrap();
    let mut count = capture.start().unwrap().len();
    for piece in samples.chunks(rate as usize / 100) {
        count += capture.feed(piece).unwrap().len();
    }
    count + capture.stop().unwrap().len()
}

#[test]
fn silence_gives_no_utterance() {
    assert_eq!(utterances(16_000, &vec![0.0; 48_000]), 0);
}

#[test]
#[ignore = "needs real speech in FALA_TEST_SPEECH_WAV"]
fn speech_gives_an_utterance() {
    let path = std::env::var("FALA_TEST_SPEECH_WAV").expect("FALA_TEST_SPEECH_WAV is not set");
    let mut reader = hound::WavReader::open(path).unwrap();
    let rate = reader.spec().sample_rate;
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| f32::from(s.unwrap()) / 32_768.0)
        .collect();
    assert!(utterances(rate, &samples) >= 1);
}
