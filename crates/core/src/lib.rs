//! Tipos compartilhados do Fala.
//!
//! Define o contrato que os crates de `crates/` trocam: `Language`, `DictationAudio`,
//! `Transcript`, `AppContext`, `Dictionary`, `Editor`, `Dictation` e `CoreError`.
//! `Event`, `Settings`, `Utterance` e `Session` entram por adição quando um consumidor precisar.
//! Sem dependência de plataforma nem de `tauri` (invariante de arquitetura).

mod audio;
mod dictation;
mod dictionary;
mod language;

pub use audio::DictationAudio;
pub use dictation::{AppContext, Dictation, Editor, Transcript};
pub use dictionary::Dictionary;
pub use language::Language;

/// Erros de `fala-core`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoreError {
    /// A tag de idioma não é uma das que o Fala aceita (`pt-BR`, `pt`, `en`).
    #[error("idioma desconhecido: {0:?}")]
    UnknownLanguage(String),
}
