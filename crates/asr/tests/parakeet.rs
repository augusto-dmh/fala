//! O Parakeet de verdade.
//!
//! O teste `#[ignore]` lê `FALA_TEST_PARAKEET_DIR` (pasta `parakeet-tdt-0.6b-v3-int8`) e
//! `FALA_TEST_SPEECH_WAV` (fala real em pt-BR, 16 kHz mono i16), e falha se faltarem.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use fala_asr::{AsrError, Parakeet, Transcriber};
use fala_core::{DictationAudio, Language};

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set")))
}

#[test]
fn missing_model_names_the_path() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("asr-missing-model");
    let _ = std::fs::remove_dir_all(&root);
    let empty = root.join("vazia");
    std::fs::create_dir_all(&empty).unwrap();
    for dir in [root.join("nao-existe"), empty] {
        let err = Parakeet::load(&dir).err().expect("load devia falhar");
        assert!(matches!(err, AsrError::ModelLoad { .. }), "{err:?}");
        let text = err.to_string();
        assert!(
            text.contains(&dir.display().to_string()),
            "a mensagem não cita {}: {text}",
            dir.display()
        );
    }
}

#[test]
#[ignore = "needs FALA_TEST_PARAKEET_DIR and FALA_TEST_SPEECH_WAV"]
fn transcribes_real_speech() {
    let mut reader = hound::WavReader::open(env_path("FALA_TEST_SPEECH_WAV")).unwrap();
    assert_eq!(reader.spec().sample_rate, 16_000);
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| f32::from(s.unwrap()) / 32_768.0)
        .collect();
    let audio = DictationAudio::new(samples);
    let mut model = Parakeet::load(&env_path("FALA_TEST_PARAKEET_DIR")).unwrap();
    for language in [Language::PtBr, Language::En] {
        let t = model.transcribe(&audio, language).unwrap();
        assert!(!t.text.trim().is_empty(), "texto vazio para {language}");
        assert_eq!(t.language, language);
    }
}
