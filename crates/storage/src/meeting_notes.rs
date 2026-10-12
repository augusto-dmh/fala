//! A nota de reunião editada à mão: o texto vai para `meetings.notes_md` e a marca de editada
//! para `meeting_note_edits`, que uma geração nova apaga. Com a marca, "Gerar notas" pede
//! confirmação antes de descartar a edição (ADR-0016, consequências).
//!
//! Tabela aditiva, criada a cada abertura como `meeting_speakers`, sem subir `user_version`.

use chrono::{DateTime, FixedOffset, SecondsFormat};
use fala_meeting::SessionId;
use rusqlite::{params, OptionalExtension};

use crate::{StorageError, Store};

pub(crate) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meeting_note_edits (
    meeting_id TEXT PRIMARY KEY REFERENCES meetings(id) ON DELETE CASCADE,
    edited_at TEXT NOT NULL
);
";

impl Store {
    /// Grava a nota editada e marca a sessão como editada. Sem nota gerada antes, devolve
    /// [`StorageError::NotFound`]: só se edita o que já foi gerado.
    pub fn save_edited_meeting_notes(
        &mut self,
        id: SessionId,
        markdown: &str,
        edited_at: DateTime<FixedOffset>,
    ) -> Result<(), StorageError> {
        let tx = self.conn.transaction()?;
        let changed = tx.execute(
            "UPDATE meetings SET notes_md = ?2 WHERE id = ?1 AND notes_md IS NOT NULL",
            params![id.to_string(), markdown],
        )?;
        if changed == 0 {
            return Err(StorageError::NotFound(id.to_string()));
        }
        tx.execute(
            "INSERT INTO meeting_note_edits (meeting_id, edited_at) VALUES (?1, ?2)
             ON CONFLICT (meeting_id) DO UPDATE SET edited_at = excluded.edited_at",
            params![
                id.to_string(),
                edited_at.to_rfc3339_opts(SecondsFormat::Secs, false)
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Se a nota da sessão foi editada depois da última geração.
    pub fn meeting_notes_edited(&self, id: SessionId) -> Result<bool, StorageError> {
        self.meeting(id)?;
        Ok(self
            .conn
            .query_row(
                "SELECT 1 FROM meeting_note_edits WHERE meeting_id = ?1",
                [id.to_string()],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }
}
