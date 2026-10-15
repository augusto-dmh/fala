//! De onde veio a transcrição de uma sessão: sem linha, da nuvem (Scribe, ADR-0005); com linha,
//! do Parakeet local (o fallback da ADR-0005, e sem chave pela ADR-0023). A reunião mostra a
//! local como tal e oferece transcrever na nuvem.
//!
//! Tabela aditiva, criada a cada abertura como `meeting_trash`, sem subir `user_version`.

use fala_meeting::SessionId;
use rusqlite::{params, OptionalExtension};

use crate::{StorageError, Store};

pub(crate) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meeting_local_transcripts (
    meeting_id TEXT PRIMARY KEY REFERENCES meetings(id) ON DELETE CASCADE
);
";

impl Store {
    /// Marca a transcrição da sessão como local (`true`) ou da nuvem.
    pub fn set_meeting_transcript_local(
        &self,
        id: SessionId,
        local: bool,
    ) -> Result<(), StorageError> {
        if local {
            self.conn.execute(
                "INSERT INTO meeting_local_transcripts (meeting_id) VALUES (?1)
                 ON CONFLICT (meeting_id) DO NOTHING",
                params![id.to_string()],
            )?;
        } else {
            self.conn.execute(
                "DELETE FROM meeting_local_transcripts WHERE meeting_id = ?1",
                params![id.to_string()],
            )?;
        }
        Ok(())
    }

    /// Se a transcrição da sessão (ou parte dela, numa tomada) saiu do caminho local.
    pub fn meeting_transcript_local(&self, id: SessionId) -> Result<bool, StorageError> {
        Ok(self
            .conn
            .query_row(
                "SELECT 1 FROM meeting_local_transcripts WHERE meeting_id = ?1",
                [id.to_string()],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }
}
