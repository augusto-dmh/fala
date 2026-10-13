//! Os trechos do mic que um ditado zerou durante a gravação (ADR-0015). O gravador os anota na
//! hora, em ms da gravação, e cada transcrição os põe entre os trechos como uma linha marcadora
//! "trecho de ditado omitido" do canal do mic: o marcador vem do gravador, não do ASR, e passa
//! pelo SQLite, pelo espelho e pelo payload das notas (ADR-0016).
//!
//! Tabela aditiva, criada a cada abertura como `meeting_speakers`, sem subir `user_version`.

use fala_meeting::SessionId;
use rusqlite::{params, Transaction};

use crate::meetings::{ms, MeetingSegment, SegmentChannel, SegmentSpeaker};
use crate::{StorageError, Store};

pub(crate) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meeting_omissions (
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    t0_ms INTEGER NOT NULL,
    t1_ms INTEGER NOT NULL CHECK (t1_ms >= t0_ms)
);
";

/// O texto da linha marcadora, igual na transcrição, no `.md` e no payload das notas.
pub const OMITTED_TEXT: &str = "[trecho de ditado omitido]";

/// A linha marcadora de um trecho omitido; `seq` é numerado na gravação.
pub fn omitted_segment(t0_ms: u64, t1_ms: u64) -> MeetingSegment {
    MeetingSegment {
        seq: 0,
        channel: SegmentChannel::Mic,
        speaker: SegmentSpeaker::Omitted,
        t0_ms,
        t1_ms,
        text: OMITTED_TEXT.to_string(),
    }
}

impl Store {
    /// Anota um trecho do mic zerado por um ditado, em ms da gravação.
    pub fn add_meeting_omission(
        &mut self,
        id: SessionId,
        t0_ms: u64,
        t1_ms: u64,
    ) -> Result<(), StorageError> {
        self.meeting(id)?;
        self.conn.execute(
            "INSERT INTO meeting_omissions (meeting_id, t0_ms, t1_ms) VALUES (?1, ?2, ?3)",
            params![id.to_string(), ms(t0_ms), ms(t1_ms.max(t0_ms))],
        )?;
        Ok(())
    }

    /// Os trechos omitidos da sessão, por início.
    pub fn meeting_omissions(&self, id: SessionId) -> Result<Vec<(u64, u64)>, StorageError> {
        Ok(omissions(&self.conn, id)?)
    }
}

/// As linhas marcadoras da sessão, para entrar entre os trechos de uma transcrição.
pub(crate) fn omitted_segments(
    tx: &Transaction<'_>,
    id: SessionId,
) -> Result<Vec<MeetingSegment>, StorageError> {
    Ok(omissions(tx, id)?
        .into_iter()
        .map(|(t0, t1)| omitted_segment(t0, t1))
        .collect())
}

fn omissions(conn: &rusqlite::Connection, id: SessionId) -> rusqlite::Result<Vec<(u64, u64)>> {
    let mut stmt = conn.prepare(
        "SELECT t0_ms, t1_ms FROM meeting_omissions WHERE meeting_id = ?1 ORDER BY t0_ms, t1_ms",
    )?;
    let rows = stmt.query_map([id.to_string()], |row| {
        let (t0, t1): (i64, i64) = (row.get(0)?, row.get(1)?);
        Ok((
            u64::try_from(t0).unwrap_or(0),
            u64::try_from(t1).unwrap_or(0),
        ))
    })?;
    rows.collect()
}
