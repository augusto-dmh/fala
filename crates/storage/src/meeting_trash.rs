//! A lixeira das reuniões: uma sessão movida para lá some da lista, da busca e do MCP, mas guarda
//! transcrição, anotações e notas por [`TRASH_DAYS`] dias; o áudio sai na hora, antes de ela
//! entrar (ADR-0005: não há lixeira de áudio). O `.md` dela vai para `Reuniões/.lixeira/`, com
//! `trashed_at` no frontmatter, e o `reindex` a traz de volta para a lixeira.
//!
//! Tabela aditiva, criada a cada abertura como `meeting_people`, sem subir `user_version`.

use std::collections::HashSet;

use chrono::{DateTime, FixedOffset};
use fala_meeting::SessionId;
use rusqlite::{params, Connection, OptionalExtension};

use crate::meetings::{bad, rfc3339};
use crate::{MeetingRecord, StorageError, Store};

pub(crate) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meeting_trash (
    meeting_id TEXT PRIMARY KEY REFERENCES meetings(id) ON DELETE CASCADE,
    trashed_at TEXT NOT NULL,
    trashed_ms INTEGER NOT NULL
);
";

/// Quantos dias uma sessão fica na lixeira antes de sair sozinha.
pub const TRASH_DAYS: i64 = 30;
const DAY_MS: i64 = 86_400_000;

/// Uma sessão na lixeira e quando ela entrou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrashedMeeting {
    pub record: MeetingRecord,
    pub trashed_at: DateTime<FixedOffset>,
}

/// Grava a sessão na lixeira; uma que já está lá fica com o `trashed_at` que tinha.
pub(crate) fn insert(
    conn: &Connection,
    id: SessionId,
    at: &DateTime<FixedOffset>,
) -> rusqlite::Result<()> {
    // Em segundos, como no frontmatter: o `reindex` devolve o mesmo `trashed_ms`.
    conn.execute(
        "INSERT INTO meeting_trash (meeting_id, trashed_at, trashed_ms) VALUES (?1, ?2, ?3)
         ON CONFLICT (meeting_id) DO NOTHING",
        params![id.to_string(), rfc3339(at), at.timestamp() * 1000],
    )?;
    Ok(())
}

/// Quando a sessão foi para a lixeira, se foi.
pub(crate) fn trashed_at(
    conn: &Connection,
    id: SessionId,
) -> Result<Option<DateTime<FixedOffset>>, StorageError> {
    let at: Option<String> = conn
        .query_row(
            "SELECT trashed_at FROM meeting_trash WHERE meeting_id = ?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .optional()?;
    at.map(|at| parse(&at)).transpose()
}

fn parse(at: &str) -> Result<DateTime<FixedOffset>, StorageError> {
    DateTime::parse_from_rfc3339(at)
        .map_err(|e| StorageError::Db(bad(1, format!("trashed_at inválido {at:?}: {e}"))))
}

impl Store {
    /// Move a sessão para a lixeira em `at`. O áudio já precisa ter saído: quem chama apaga
    /// `audio/<id>/` antes, e `audio_retained` volta a falso aqui. Uma sessão que já está na
    /// lixeira fica com o `trashed_at` que tinha.
    pub fn trash_meeting(
        &mut self,
        id: SessionId,
        at: DateTime<FixedOffset>,
    ) -> Result<(), StorageError> {
        let tx = self.conn.transaction()?;
        if tx.execute(
            "UPDATE meetings SET audio_retained = 0 WHERE id = ?1",
            [id.to_string()],
        )? == 0
        {
            return Err(StorageError::NotFound(id.to_string()));
        }
        insert(&tx, id, &at)?;
        tx.commit()?;
        log::debug!("reunião {id} na lixeira");
        self.after_meeting_change(id);
        Ok(())
    }

    /// Tira a sessão da lixeira; o `.md` volta para `Reuniões/`. Uma sessão fora da lixeira é
    /// [`StorageError::NotFound`].
    pub fn restore_meeting(&self, id: SessionId) -> Result<(), StorageError> {
        let removed = self.conn.execute(
            "DELETE FROM meeting_trash WHERE meeting_id = ?1",
            [id.to_string()],
        )?;
        if removed == 0 {
            return Err(StorageError::NotFound(id.to_string()));
        }
        log::debug!("reunião {id} restaurada");
        self.after_meeting_change(id);
        Ok(())
    }

    /// Quando a sessão foi para a lixeira; `None` fora dela.
    pub fn meeting_trashed_at(
        &self,
        id: SessionId,
    ) -> Result<Option<DateTime<FixedOffset>>, StorageError> {
        trashed_at(&self.conn, id)
    }

    /// Os ids das sessões da lixeira, para quem lista as outras.
    pub fn trashed_meeting_ids(&self) -> Result<HashSet<SessionId>, StorageError> {
        let mut stmt = self.conn.prepare("SELECT meeting_id FROM meeting_trash")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut ids = HashSet::new();
        for id in rows {
            ids.insert(parse_id(&id?)?);
        }
        Ok(ids)
    }

    /// Se a sessão está na lixeira há [`TRASH_DAYS`] dias ou mais em `now_ms`.
    pub fn trash_is_due(&self, id: SessionId, now_ms: i64) -> Result<bool, StorageError> {
        Ok(self.conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM meeting_trash WHERE meeting_id = ?1 AND trashed_ms <= ?2)",
            params![id.to_string(), due_before(now_ms)],
            |row| row.get(0),
        )?)
    }

    /// As sessões da lixeira, da que entrou por último para a primeira.
    pub fn trashed_meetings(&self) -> Result<Vec<TrashedMeeting>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT meeting_id, trashed_at FROM meeting_trash ORDER BY trashed_ms DESC, meeting_id",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut out = Vec::with_capacity(rows.len());
        for (id, at) in rows {
            out.push(TrashedMeeting {
                record: self.meeting(parse_id(&id)?)?,
                trashed_at: parse(&at)?,
            });
        }
        Ok(out)
    }

    /// As sessões que estão na lixeira há [`TRASH_DAYS`] dias ou mais em `now_ms`.
    pub fn trash_due(&self, now_ms: i64) -> Result<Vec<SessionId>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT meeting_id FROM meeting_trash WHERE trashed_ms <= ?1 ORDER BY trashed_ms",
        )?;
        let rows = stmt.query_map([due_before(now_ms)], |row| row.get::<_, String>(0))?;
        let mut due = Vec::new();
        for id in rows {
            due.push(parse_id(&id?)?);
        }
        Ok(due)
    }
}

/// O `trashed_ms` mais novo que já venceu em `now_ms`.
fn due_before(now_ms: i64) -> i64 {
    now_ms.saturating_sub(TRASH_DAYS * DAY_MS)
}

fn parse_id(id: &str) -> Result<SessionId, StorageError> {
    id.parse()
        .map_err(|_| StorageError::Db(bad(0, format!("id de sessão inválido {id:?}"))))
}
