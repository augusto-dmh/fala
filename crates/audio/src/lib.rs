//! Captura de áudio, VAD e resample.
//!
//! Ditado: `Mic` (microfone via `cpal`) entrega mono f32 na taxa do dispositivo;
//! `DictationCapture` reamostra para 16 kHz com `rubato`, mantém o pré-buffer de 300 ms e usa o
//! Silero VAD v4 (`vad-rs`, ADR-0009) para fechar utterances de até 15 s, entregues como
//! `fala_core::DictationAudio` (que não implementa serialização, ADR-0003), e, para o ditado
//! longo, blocos de 25-40 s fechados no silêncio.
//! Reunião: `meeting::MeetingRecorder` grava mic e sistema (`SystemAudio`) num WAV estéreo 48 kHz
//! à prova de crash, com o relógio de parede decidindo os frames (ver o módulo).
//! O desktop grava o ditado por aqui (`Mic::open_device` com o dispositivo e o canal dos
//! settings). Ainda por vir: Opus no fim da reunião.
//! Atividade: `platform_activity` diz quais processos capturam o microfone agora (só leitura), o
//! sinal da detecção de call.

mod activity;
mod capture;
mod meeting;
mod mic;
mod resample;
mod vad;

use std::path::PathBuf;

pub use activity::{platform_activity, AudioActivity, MicHolder};
pub use capture::{
    DictationCapture, BLOCK_MAX_SAMPLES, BLOCK_MIN_SAMPLES, HANGOVER_FRAMES, MAX_UTTERANCE_SAMPLES,
    ONSET_FRAMES, PREBUFFER_SAMPLES, PRE_ROLL_FRAMES,
};
pub use meeting::{
    ChannelStats, MeetingRecorder, MeetingWav, SystemAudio, FLUSH_EVERY, LAG_FRAMES,
    MAX_BACKLOG_FRAMES, MEETING_RATE,
};
pub use meeting::{OutputDevice, OutputMeter, OutputPeak};
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
    #[error("stream: {0}")]
    Stream(String),
    #[error("áudio do sistema `{name}` não encontrado; disponíveis:\n  {}", available.join("\n  "))]
    NoSystem {
        name: String,
        available: Vec<String>,
    },
    /// O SO atual ainda não tem esta função (ADR-0007: os adaptadores Linux chegam na fase 3).
    #[error("sem suporte neste sistema: {0}")]
    Unsupported(&'static str),
    #[error("WAV {}: {reason}", path.display())]
    Wav { path: PathBuf, reason: String },
}
