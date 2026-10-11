//! Quem estava numa reunião: os nomes digitados em "Quem estava" e, por "Pessoa N", o nome
//! aplicado e a sugestão do LLM de notas (ADR-0016: uma sugestão nunca se aplica sozinha).
//!
//! Tabelas aditivas, criadas a cada abertura como as métricas, sem subir `user_version`: um
//! banco antigo só fica sem linhas nelas.

use fala_meeting::SessionId;
use rusqlite::{params, OptionalExtension};

use crate::{StorageError, Store};

pub(crate) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meeting_participants (
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 1),
    name TEXT NOT NULL,
    PRIMARY KEY (meeting_id, position)
);
CREATE TABLE IF NOT EXISTS meeting_speakers (
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    person INTEGER NOT NULL CHECK (person >= 1),
    name TEXT,
    suggested_name TEXT,
    suggested_reason TEXT,
    PRIMARY KEY (meeting_id, person)
);
";

/// "Pessoa `person`" de uma sessão: o nome aplicado, a sugestão pendente, ou os dois.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingSpeaker {
    pub person: u32,
    pub name: Option<String>,
    pub suggestion: Option<NameSuggestion>,
}

/// Um nome sugerido e a frase que o justifica.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameSuggestion {
    pub name: String,
    pub reason: String,
}

impl Store {
    /// Substitui os participantes da sessão, na ordem dada.
    pub fn set_meeting_participants(
        &mut self,
        id: SessionId,
        names: &[String],
    ) -> Result<(), StorageError> {
        self.meeting(id)?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM meeting_participants WHERE meeting_id = ?1",
            [id.to_string()],
        )?;
        for (index, name) in names.iter().enumerate() {
            tx.execute(
                "INSERT INTO meeting_participants (meeting_id, position, name) VALUES (?1, ?2, ?3)",
                params![id.to_string(), index as i64 + 1, name],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Os participantes da sessão, na ordem digitada.
    pub fn meeting_participants(&self, id: SessionId) -> Result<Vec<String>, StorageError> {
        self.meeting(id)?;
        let mut stmt = self.conn.prepare(
            "SELECT name FROM meeting_participants WHERE meeting_id = ?1 ORDER BY position",
        )?;
        let rows = stmt.query_map([id.to_string()], |row| row.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// As pessoas da sessão com nome ou sugestão, por número.
    pub fn meeting_speakers(&self, id: SessionId) -> Result<Vec<MeetingSpeaker>, StorageError> {
        self.meeting(id)?;
        let mut stmt = self.conn.prepare(
            "SELECT person, name, suggested_name, suggested_reason FROM meeting_speakers
             WHERE meeting_id = ?1 ORDER BY person",
        )?;
        let rows = stmt.query_map([id.to_string()], |row| {
            let suggested: Option<String> = row.get(2)?;
            let reason: Option<String> = row.get(3)?;
            Ok(MeetingSpeaker {
                person: row.get(0)?,
                name: row.get(1)?,
                suggestion: suggested.map(|name| NameSuggestion {
                    name,
                    reason: reason.unwrap_or_default(),
                }),
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Troca as sugestões da sessão pelas de uma geração nova. Nenhum nome aplicado muda, e uma
    /// pessoa que já tem nome não recebe sugestão (uma geração que começou antes do clique).
    pub fn save_speaker_suggestions(
        &mut self,
        id: SessionId,
        suggestions: &[(u32, NameSuggestion)],
    ) -> Result<(), StorageError> {
        self.meeting(id)?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE meeting_speakers SET suggested_name = NULL, suggested_reason = NULL
             WHERE meeting_id = ?1",
            [id.to_string()],
        )?;
        for (person, suggestion) in suggestions {
            tx.execute(
                "INSERT INTO meeting_speakers (meeting_id, person, suggested_name, suggested_reason)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (meeting_id, person) DO UPDATE
                 SET suggested_name = excluded.suggested_name,
                     suggested_reason = excluded.suggested_reason
                 WHERE meeting_speakers.name IS NULL",
                params![id.to_string(), person, suggestion.name, suggestion.reason],
            )?;
        }
        tx.execute(
            "DELETE FROM meeting_speakers
             WHERE meeting_id = ?1 AND name IS NULL AND suggested_name IS NULL",
            [id.to_string()],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// O clique em "aplicar": o nome sugerido vira o nome de "Pessoa `person`" e a sugestão some.
    /// Sem sugestão pendente, devolve [`StorageError::NotFound`].
    pub fn apply_speaker_suggestion(
        &mut self,
        id: SessionId,
        person: u32,
    ) -> Result<String, StorageError> {
        let tx = self.conn.transaction()?;
        let name: Option<String> = tx
            .query_row(
                "SELECT suggested_name FROM meeting_speakers
                 WHERE meeting_id = ?1 AND person = ?2 AND suggested_name IS NOT NULL",
                params![id.to_string(), person],
                |row| row.get(0),
            )
            .optional()?;
        let Some(name) = name else {
            return Err(StorageError::NotFound(format!("{id} pessoa {person}")));
        };
        tx.execute(
            "UPDATE meeting_speakers
             SET name = suggested_name, suggested_name = NULL, suggested_reason = NULL
             WHERE meeting_id = ?1 AND person = ?2",
            params![id.to_string(), person],
        )?;
        tx.commit()?;
        Ok(name)
    }
}
