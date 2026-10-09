//! Sessão de gravação de reunião (fase 2, pitch `fase-2-reuniao-videos` 2.F1).
//!
//! Lógica pura: o chamador entrega cada [`Input`] com o instante de parede e executa os
//! [`Effect`] devolvidos, em ordem, no gravador (`crates/audio`) e na UI. O crate não lê o
//! relógio, não abre áudio e não loga.
//!
//! ADR-0005: a sessão só sai de `Idle` por [`UserAction::StartRecording`], só volta a gravar
//! por [`UserAction::ResumeRecording`], e os estados que capturam ou aguardam carregam um
//! [`Indicator`], que só este crate constrói. Todo fim manda processar; nada é descartado.
//! Sem dependência de plataforma nem de `tauri` (ADR-0002, ADR-0007).

mod cap;
mod id;
mod mute;
mod session;

pub use cap::RecordingCap;
pub use id::{SessionId, SessionMode};
pub use mute::{MuteWatch, Muted, MUTE_WARN_AFTER};
pub use session::{
    Effect, Gap, GapKind, Indicator, IndicatorKind, Input, Levels, MeetingSession, SessionConfig,
    SessionState, StopReason, UnixMillis, UserAction, LOW_DISK_WARN_BYTES, MIN_FREE_DISK_BYTES,
    SILENCE_RMS, SILENCE_STOP_AFTER,
};

/// Erros de `fala-meeting`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SessionError {
    /// A ação do usuário não vale no estado atual; o estado não mudou.
    #[error("a ação {action:?} não vale no estado {state:?}")]
    InvalidTransition {
        state: SessionState,
        action: UserAction,
    },
    /// Espaço livre abaixo do mínimo para começar a gravar.
    #[error("espaço em disco insuficiente: {free_bytes} bytes livres, faltam {missing_bytes}")]
    InsufficientDisk { free_bytes: u64, missing_bytes: u64 },
    /// O modo da sessão não captura áudio ao vivo (importação).
    #[error("o modo {0:?} não grava")]
    ModeDoesNotRecord(SessionMode),
    /// Teto fora de 1 a 8 h.
    #[error("teto de {0} h fora do intervalo de 1 a 8 h")]
    CapOutOfRange(u8),
    /// O teto já está no máximo de 8 h.
    #[error("o teto já está no máximo de 8 h")]
    CapAtMaximum,
    /// Texto que não é um ULID de 26 caracteres Crockford maiúsculos.
    #[error("id de sessão inválido: {0:?}")]
    InvalidSessionId(String),
    /// O SO não entregou entropia para gerar o id.
    #[error("sem entropia do SO: {0}")]
    Entropy(String),
}

/// Provas de compilação da ADR-0005, rodadas por `cargo test -p fala-meeting --doc compile_fail`.
///
/// O caminho permitido compila: a sessão nasce por [`MeetingSession::new`], em `Idle`.
///
/// ```
/// use fala_meeting::{MeetingSession, RecordingCap, SessionConfig, SessionId, SessionMode, SessionState};
/// let session = MeetingSession::new(SessionConfig {
///     id: SessionId::from_parts(1_727_000_000_000, 0),
///     mode: SessionMode::Meeting,
///     title: String::new(),
///     local_only: false,
///     cap: RecordingCap::default(),
/// });
/// assert_eq!(session.state(), SessionState::Idle);
/// assert!(session.indicator().is_none());
/// ```
///
/// Trocar o estado de uma sessão por fora de `apply` não compila (campo privado):
///
/// ```compile_fail
/// use fala_meeting::{MeetingSession, SessionState};
/// fn force(session: &mut MeetingSession, state: SessionState) {
///     session.state = state;
/// }
/// ```
///
/// Fabricar um indicador fora do crate, e com ele um estado `Recording`, não compila:
///
/// ```compile_fail
/// use fala_meeting::{Indicator, IndicatorKind, SessionState};
/// let _ = SessionState::Recording { indicator: Indicator { kind: IndicatorKind::Recording } };
/// ```
#[doc(hidden)]
pub mod compile_fail {}
