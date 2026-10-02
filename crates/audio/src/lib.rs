//! Captura de áudio, VAD e resample.
//!
//! Ditado: `Mic` (microfone via `cpal`) entrega mono f32 na taxa do dispositivo;
//! `DictationCapture` reamostra para 16 kHz com `rubato`, mantém o pré-buffer de 300 ms e usa o
//! Silero VAD v4 (`vad-rs`, ADR-0009) para fechar utterances de até 15 s, entregues como
//! `fala_core::DictationAudio` (que não implementa serialização, ADR-0003).
//! Ainda por vir: loopback do sistema e o gravador WAV/Opus de reunião (fase 2), e o
//! `audio_toolkit` do desktop, que migra para cá quando o desktop for ligado aos crates.

mod capture;
mod mic;
mod resample;
mod vad;

use std::path::PathBuf;

pub use capture::{
    DictationCapture, HANGOVER_FRAMES, MAX_UTTERANCE_SAMPLES, ONSET_FRAMES, PREBUFFER_SAMPLES,
    PRE_ROLL_FRAMES,
};
pub use mic::Mic;
pub use resample::{Resampler, OUT_RATE};
pub use vad::{SileroVad, VoiceDetector, FRAME_SAMPLES, SILERO_THRESHOLD};

/// Erros de `fala-audio`.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("não consegui carregar o VAD de {path}: {reason}")]
    VadLoad { path: PathBuf, reason: String },
    #[error("VAD: {0}")]
    Vad(String),
    #[error("resample: {0}")]
    Resample(String),
    #[error("nenhum dispositivo de entrada contém `{needle}`; dispositivos de entrada:\n  {}", available.join("\n  "))]
    NoDevice {
        needle: String,
        available: Vec<String>,
    },
    #[error("dispositivo de entrada: {0}")]
    Device(String),
    #[error("configuração do dispositivo: {0}")]
    UnsupportedConfig(String),
    #[error("stream do microfone: {0}")]
    Stream(String),
}
