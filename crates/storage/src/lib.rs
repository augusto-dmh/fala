//! Persistência local: SQLite com FTS5 e espelho Markdown.
//!
//! SQLite (`rusqlite` bundled, WAL, FTS5 `unicode61 remove_diacritics 2`) é a fonte de verdade (ADR-0006).
//! Os `.md` com frontmatter são o espelho para leitura e sync por pasta; `reindex` reconstrói o banco.
//! Escrita no SQLite primeiro, espelho depois.
//!
//! O banco é `fala.sqlite`, separado do `history.db` do desktop: cada um tem o seu `user_version`.
//! A cópia de `transcription_history` para cá fica para quando o desktop virar fachada.

mod mirror;
mod store;

use std::path::PathBuf;

use chrono::{DateTime, FixedOffset};
use fala_core::Dictation;
use serde::{Deserialize, Serialize};

pub use store::Store;

/// Qual dos dois textos de um item vale agora.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Showing {
    /// O texto final (regras ou LLM); o estado de todo item recém-gravado.
    Final,
    /// O bruto do ASR, depois de "desfazer edição".
    Raw,
}

impl Showing {
    /// O literal gravado no banco e no frontmatter.
    pub fn as_str(self) -> &'static str {
        match self {
            Showing::Final => "final",
            Showing::Raw => "raw",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "final" => Some(Showing::Final),
            "raw" => Some(Showing::Raw),
            _ => None,
        }
    }
}

/// Um `Dictation` persistido.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictationRecord {
    /// UUID v7 em minúsculas com hífens; o mesmo no banco, no nome do `.md` e no frontmatter.
    pub id: String,
    /// Instante do ditado, com o offset local de quem ditou.
    pub created_at: DateTime<FixedOffset>,
    pub dictation: Dictation,
    pub showing: Showing,
}

impl DictationRecord {
    /// O texto que o item mostra agora: o final ou, depois de desfazer, o bruto.
    pub fn shown_text(&self) -> &str {
        match self.showing {
            Showing::Final => &self.dictation.final_text,
            Showing::Raw => &self.dictation.raw.text,
        }
    }
}

/// Um `.md` que o `reindex` não aceitou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub path: PathBuf,
    pub reason: String,
}

/// O resultado de um `reindex`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReindexReport {
    /// Linhas gravadas, uma por `.md` válido.
    pub indexed: usize,
    /// Arquivos `.md` ignorados, com o motivo.
    pub skipped: Vec<Skipped>,
}

/// Erros de `fala-storage`.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("ditado não encontrado: {0}")]
    NotFound(String),
    #[error("o ditado {0} não tem edição para desfazer")]
    NothingToUndo(String),
    /// A linha foi gravada no banco, mas o `.md` não.
    #[error("ditado {id} gravado no banco, mas o espelho {path} falhou: {source}")]
    Mirror {
        id: String,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("banco: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("E/S: {0}")]
    Io(#[from] std::io::Error),
}
