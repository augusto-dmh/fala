//! Reconhecimento de fala plugável.
//!
//! Trait `Transcriber`: uma utterance de ditado entra, o texto bruto sai, no idioma pedido.
//! Única implementação: Parakeet-TDT-0.6B-v3 int8 local pelo `transcribe-rs` (ADR-0003, com o
//! runtime ajustado pela ADR-0009). Backends de nuvem, só para reunião (ADR-0005).
//! Trocar de backend não deve exigir mudança fora deste crate.
//! Ainda por vir: `transcribe_file(path)`, quando houver decodificação de arquivo.

use std::path::{Path, PathBuf};

use fala_core::{DictationAudio, Language, Transcript};
use transcribe_rs::onnx::parakeet::ParakeetModel;
use transcribe_rs::onnx::Quantization;
use transcribe_rs::{SpeechModel, TranscribeOptions};

/// Utterances mais curtas que isto (100 ms a 16 kHz) nem chegam ao modelo.
pub const MIN_SAMPLES: usize = 1_600;

/// Transforma uma utterance de ditado em texto bruto.
pub trait Transcriber: Send {
    fn transcribe(
        &mut self,
        audio: &DictationAudio,
        language: Language,
    ) -> Result<Transcript, AsrError>;
}

/// Erros de `fala-asr`.
#[derive(Debug, thiserror::Error)]
pub enum AsrError {
    #[error("não consegui carregar o modelo de {}: {reason}", path.display())]
    ModelLoad { path: PathBuf, reason: String },
    #[error("inferência: {0}")]
    Inference(String),
}

/// Parakeet-TDT-0.6B-v3 int8. O modelo detecta o idioma sozinho; `language` só rotula o
/// `Transcript`.
pub struct Parakeet {
    model: Box<ParakeetModel>,
}

impl Parakeet {
    /// Carrega a pasta `parakeet-tdt-0.6b-v3-int8`.
    pub fn load(dir: &Path) -> Result<Self, AsrError> {
        let model_load = |reason: String| AsrError::ModelLoad {
            path: dir.to_path_buf(),
            reason,
        };
        if !dir.is_dir() {
            return Err(model_load("a pasta não existe".to_owned()));
        }
        let model =
            ParakeetModel::load(dir, &Quantization::Int8).map_err(|e| model_load(e.to_string()))?;
        Ok(Self {
            model: Box::new(model),
        })
    }
}

impl Transcriber for Parakeet {
    fn transcribe(
        &mut self,
        audio: &DictationAudio,
        language: Language,
    ) -> Result<Transcript, AsrError> {
        transcribe_with(audio, language, |samples| {
            self.model
                .transcribe(samples, &TranscribeOptions::default())
                .map(|result| result.text)
                .map_err(|e| AsrError::Inference(e.to_string()))
        })
    }
}

/// O que todo backend faz em volta da inferência: pula áudio curto demais e rotula o idioma.
fn transcribe_with(
    audio: &DictationAudio,
    language: Language,
    infer: impl FnOnce(&[f32]) -> Result<String, AsrError>,
) -> Result<Transcript, AsrError> {
    let text = if audio.samples().len() < MIN_SAMPLES {
        String::new()
    } else {
        infer(audio.samples())?
    };
    Ok(Transcript { text, language })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_audio_skips_inference() {
        let mut calls = 0;
        let short = DictationAudio::new(vec![0.1; 1_599]);
        let t = transcribe_with(&short, Language::PtBr, |_| {
            calls += 1;
            Ok("não devia rodar".to_owned())
        })
        .unwrap();
        assert_eq!(calls, 0);
        assert_eq!(t.text, "");
        assert_eq!(t.language, Language::PtBr);

        let enough = DictationAudio::new(vec![0.1; 1_600]);
        let t = transcribe_with(&enough, Language::En, |samples| {
            calls += 1;
            assert_eq!(samples.len(), 1_600);
            Ok("rodou".to_owned())
        })
        .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(t.text, "rodou");
        assert_eq!(t.language, Language::En);
    }
}
