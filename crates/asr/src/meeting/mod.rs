//! Transcrição de reunião (ADR-0005): os dois arquivos retidos de uma sessão viram uma lista de
//! [`Segment`] "Eu / Pessoa N", ordenada por tempo.
//!
//! Separado de [`crate::Transcriber`], que é só do ditado. O único tipo que um backend de rede
//! aceita é [`MeetingRecording`], construído a partir da pasta da sessão; nenhum backend de rede
//! implementa `Transcriber`, então áudio de ditado não chega a um cliente HTTP por construção
//! (ADR-0003).

mod elevenlabs;
mod guard;
mod local;
mod segments;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub use elevenlabs::{ElevenLabsScribe, ScribeOptions, SCRIBE_BASE_URL};
pub use guard::{MeetingRecording, MIC_FILE, SYSTEM_FILE};
pub use local::{transcribe_with_fallback, LocalMeeting, MeetingRoute, WINDOW_MS};
pub use segments::{Channel, Segment, Speaker};

use crate::AsrError;

/// Um backend de transcrição de reunião (Scribe na nuvem, Parakeet local em janelas).
pub trait MeetingTranscriber: Send {
    /// Transcreve os canais da sessão. `on_progress` é chamado no thread de quem chama, com
    /// intervalo de no máximo 1 s enquanto um canal está em andamento; `cancel` interrompe a
    /// espera e devolve [`AsrError::Cancelled`].
    fn transcribe_session(
        &mut self,
        recording: &MeetingRecording,
        cancel: &CancelToken,
        on_progress: &mut dyn FnMut(Progress),
    ) -> Result<Vec<Segment>, AsrError>;
}

/// Pedido de cancelamento compartilhado entre quem chama e a transcrição em andamento.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Onde está a transcrição de um canal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub channel: Channel,
    pub stage: Stage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Bytes da requisição do canal (multipart com o arquivo) já entregues ao cliente HTTP.
    Uploading { sent: u64, total: u64 },
    /// Corpo enviado; esperando a resposta do provedor.
    Waiting,
    /// Caminho local: áudio do canal já passado pelo modelo.
    Transcribing { done_ms: u64, total_ms: u64 },
}
