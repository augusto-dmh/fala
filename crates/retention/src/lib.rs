//! Áudio de reunião retido (fase 2, pitch `fase-2-reuniao-videos` 2.F3; ADR-0014).
//!
//! O WAV estéreo de trabalho (L = mic, R = sistema, 48 kHz i16) vira `audio/<id>/mic.opus` e
//! `audio/<id>/sys.opus`, Ogg Opus mono de 24 kbps. O WAV só é apagado depois de os dois
//! arquivos decodificarem com a contagem exata de amostras. A política de retenção decide, com
//! o relógio do chamador, quais sessões têm o áudio vencido. Sem `cfg` de plataforma nem `tauri`.

mod encode;
mod policy;

use std::io;
use std::path::{Path, PathBuf};

use fala_meeting::SessionId;

pub use encode::{retain_wav, validate_opus, RetainedAudio, BITRATE_BPS, SAMPLE_RATE_HZ};
pub use policy::{audio_due, RetainedSession, RetentionDays, RetentionPolicy};

/// Nome do arquivo do canal do mic (L do WAV) dentro da pasta da sessão.
pub const MIC_FILE: &str = "mic.opus";
/// Nome do arquivo do canal do sistema (R do WAV) dentro da pasta da sessão.
pub const SYSTEM_FILE: &str = "sys.opus";

/// Erros de `fala-retention`.
#[derive(Debug, thiserror::Error)]
pub enum RetentionError {
    /// O WAV não é o de trabalho da trilha F (48 kHz, estéreo, 16 bits).
    #[error("WAV não suportado: {sample_rate} Hz, {channels} canais, {bits} bits")]
    UnsupportedWav {
        sample_rate: u32,
        channels: u16,
        bits: u16,
    },
    #[error("WAV ilegível em {path:?}: {reason}")]
    Wav { path: PathBuf, reason: String },
    #[error("erro de I/O em {path:?}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("erro do libopus: {0}")]
    Opus(String),
    /// O Opus não prova que guarda o WAV inteiro; o WAV não é apagado.
    #[error("validação de {path:?} falhou: {reason}")]
    ValidationFailed { path: PathBuf, reason: String },
    /// Dias de retenção fora de 1 a 3650.
    #[error("{0} dias fora do intervalo de 1 a 3650")]
    DaysOutOfRange(u16),
}

impl RetentionError {
    fn io(path: &Path, source: io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}

/// A pasta do áudio retido de uma sessão: `<audio_root>/<id>/`.
pub fn session_audio_dir(audio_root: &Path, id: SessionId) -> PathBuf {
    audio_root.join(id.to_string())
}

/// Apaga `<audio_root>/<id>/` com o conteúdo. Devolve `false` se a pasta não existe.
///
/// O id é um ULID de 26 caracteres Crockford, então o caminho nunca sai de `audio_root`.
pub fn delete_session_audio(audio_root: &Path, id: SessionId) -> Result<bool, RetentionError> {
    let dir = session_audio_dir(audio_root, id);
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(RetentionError::io(&dir, e)),
    }
}
