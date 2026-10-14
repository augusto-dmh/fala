//! Retomar uma reunião encerrada: cada gravação a mais é uma tomada. A tomada 1 é a gravação de
//! sempre; a N (N ≥ 2) grava em `audio/<id>/t<N>/` e começa na linha do tempo da sessão em
//! `offset_ms`, a soma do que as anteriores gravaram (sem o buraco do intervalo). Uma linha
//! marcadora "[gravação retomada às HH:MM]" fica no ponto, e os trechos transcritos da tomada
//! entram depois dos que já existem, com o deslocamento.
//!
//! Tabela aditiva, criada a cada abertura como `meeting_erasures`, sem subir `user_version`. O
//! `.md` leva a tabela na chave `takes` (só com tomadas), para o `reindex`.

use chrono::{DateTime, FixedOffset};
use fala_meeting::{SessionId, SessionMode, StopReason};
use rusqlite::{params, Connection, OptionalExtension};

use crate::meetings::{
    bad, insert_segments, mode_str, ms, read_segment, reason_str, rfc3339, with_markers,
    MeetingSegment, SegmentChannel, SegmentSpeaker,
};
use crate::{StorageError, Store};

pub(crate) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meeting_takes (
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    take INTEGER NOT NULL CHECK (take >= 2),
    offset_ms INTEGER NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    recorded_ms INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (meeting_id, take)
);
";

/// Uma gravação a mais de uma sessão (a 2, a 3...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingTake {
    pub take: u32,
    /// Onde a tomada começa na linha do tempo da sessão, em ms.
    pub offset_ms: u64,
    pub started_at: DateTime<FixedOffset>,
    /// `None` enquanto grava ou se o processo morreu antes do fim.
    pub ended_at: Option<DateTime<FixedOffset>>,
    pub recorded_ms: u64,
}

impl MeetingTake {
    /// A tomada como objeto JSON, para a chave `takes` do `.md`.
    pub(crate) fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "take": self.take,
            "offset_ms": self.offset_ms,
            "started_at": rfc3339(&self.started_at),
            "ended_at": self.ended_at.as_ref().map(rfc3339),
            "recorded_ms": self.recorded_ms,
        })
    }

    pub(crate) fn from_json(value: &serde_json::Value) -> Option<Self> {
        let time = |key: &str| {
            value[key]
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        };
        Some(MeetingTake {
            take: u32::try_from(value["take"].as_u64()?)
                .ok()
                .filter(|n| *n >= 2)?,
            offset_ms: value["offset_ms"].as_u64()?,
            started_at: time("started_at")?,
            ended_at: match value["ended_at"].is_null() {
                true => None,
                false => Some(time("ended_at")?),
            },
            recorded_ms: value["recorded_ms"].as_u64()?,
        })
    }
}

/// A linha marcadora de uma tomada: "[gravação retomada às 15:42]", no início dela.
pub fn resumed_segment(take: &MeetingTake) -> MeetingSegment {
    MeetingSegment {
        seq: 0,
        channel: SegmentChannel::Mic,
        speaker: SegmentSpeaker::Resumed,
        t0_ms: take.offset_ms,
        t1_ms: take.offset_ms,
        text: format!("[gravação retomada às {}]", take.started_at.format("%H:%M")),
    }
}

impl Store {
    /// Abre a próxima tomada de uma sessão encerrada, em `at`: a sessão volta a gravar (sem
    /// fim nem motivo) e a tomada começa onde a gravação parou. Uma sessão que não terminou, um
    /// rascunho e uma importação não retomam: [`StorageError::NotResumable`].
    pub fn start_meeting_take(
        &mut self,
        id: SessionId,
        at: DateTime<FixedOffset>,
    ) -> Result<MeetingTake, StorageError> {
        let tx = self.conn.transaction()?;
        let row: Option<(String, Option<String>, i64)> = tx
            .query_row(
                "SELECT mode, ended_at, recorded_ms FROM meetings WHERE id = ?1",
                [id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let Some((mode, ended_at, recorded_ms)) = row else {
            return Err(StorageError::NotFound(id.to_string()));
        };
        // Nem com uma tomada que ficou aberta (o processo morreu nela), nem na lixeira.
        let open = takes(&tx, id)?.iter().any(|t| t.ended_at.is_none());
        let trashed = crate::meeting_trash::trashed_at(&tx, id)?.is_some();
        if ended_at.is_none() || mode == mode_str(SessionMode::Import) || open || trashed {
            return Err(StorageError::NotResumable(id.to_string()));
        }
        let take: u32 = tx.query_row(
            "SELECT coalesce(max(take), 1) + 1 FROM meeting_takes WHERE meeting_id = ?1",
            [id.to_string()],
            |row| row.get(0),
        )?;
        let take = MeetingTake {
            take,
            offset_ms: u64::try_from(recorded_ms).unwrap_or(0),
            started_at: at,
            ended_at: None,
            recorded_ms: 0,
        };
        insert(&tx, id, &take)?;
        tx.execute(
            "UPDATE meetings SET ended_at = NULL, stop_reason = NULL WHERE id = ?1",
            [id.to_string()],
        )?;
        tx.commit()?;
        log::debug!("reunião {id}: tomada {} aberta", take.take);
        self.after_meeting_change(id);
        Ok(take)
    }

    /// Fecha a tomada aberta: o fim da sessão é o dela, e o `recorded_ms` da sessão soma o da
    /// tomada. `reason` é `None` numa tomada recuperada depois de o processo morrer.
    pub fn finish_meeting_take(
        &mut self,
        id: SessionId,
        ended_at: DateTime<FixedOffset>,
        recorded_ms: u64,
        reason: Option<StopReason>,
    ) -> Result<MeetingTake, StorageError> {
        let tx = self.conn.transaction()?;
        let open = takes(&tx, id)?
            .into_iter()
            .find(|t| t.ended_at.is_none())
            .ok_or_else(|| StorageError::NotFound(format!("{id} sem tomada aberta")))?;
        tx.execute(
            "UPDATE meeting_takes SET ended_at = ?3, recorded_ms = ?4
             WHERE meeting_id = ?1 AND take = ?2",
            params![
                id.to_string(),
                open.take,
                rfc3339(&ended_at),
                ms(recorded_ms)
            ],
        )?;
        tx.execute(
            "UPDATE meetings SET ended_at = ?2, recorded_ms = ?3, stop_reason = ?4 WHERE id = ?1",
            params![
                id.to_string(),
                rfc3339(&ended_at),
                ms(open.offset_ms + recorded_ms),
                reason.map(reason_str)
            ],
        )?;
        tx.commit()?;
        self.after_meeting_change(id);
        Ok(MeetingTake {
            ended_at: Some(ended_at),
            recorded_ms,
            ..open
        })
    }

    /// As tomadas 2.. da sessão, por número; sem retomada, nenhuma.
    pub fn meeting_takes(&self, id: SessionId) -> Result<Vec<MeetingTake>, StorageError> {
        takes(&self.conn, id)
    }

    /// Acrescenta a transcrição de uma tomada (tempos dela) depois dos trechos que existem: os
    /// tempos ganham `offset_ms`, a "Pessoa N" do sistema vira a N depois da maior da sessão (os
    /// nomes já dados ficam), e nada entra dentro de um trecho apagado. Os marcadores da sessão
    /// voltam das tabelas, inclusive as omissões do ditado da tomada. Os trechos que existem,
    /// todos antes do início da tomada, guardam o número.
    pub fn append_meeting_segments(
        &mut self,
        id: SessionId,
        segments: Vec<MeetingSegment>,
        offset_ms: u64,
        transcribed_at: DateTime<FixedOffset>,
    ) -> Result<(), StorageError> {
        let tx = self.conn.transaction()?;
        let changed = tx.execute(
            "UPDATE meetings SET transcribed_at = ?2 WHERE id = ?1",
            params![id.to_string(), rfc3339(&transcribed_at)],
        )?;
        if changed == 0 {
            return Err(StorageError::NotFound(id.to_string()));
        }
        let mut all: Vec<MeetingSegment> = tx
            .prepare(
                "SELECT seq, channel, speaker, t0_ms, t1_ms, text FROM meeting_segments
                 WHERE meeting_id = ?1 ORDER BY seq",
            )?
            .query_map([id.to_string()], read_segment)?
            .collect::<Result<_, _>>()?;
        all.retain(|s| !s.speaker.is_marker());
        let spoken = segments
            .into_iter()
            .filter(|s| !s.speaker.is_marker())
            .collect();
        let all = with_markers(&tx, id, on_timeline(all, spoken, offset_ms))?;
        tx.execute(
            "DELETE FROM meeting_segments WHERE meeting_id = ?1",
            [id.to_string()],
        )?;
        insert_segments(&tx, id, &all)?;
        tx.commit()?;
        self.after_meeting_change(id);
        Ok(())
    }
}

/// A transcrição de uma tomada depois da que já existe: os tempos ganham `offset_ms`, e a
/// "Pessoa N" da tomada (transcrita sozinha) vira a N depois da maior de antes.
pub fn on_timeline(
    mut before: Vec<MeetingSegment>,
    take: Vec<MeetingSegment>,
    offset_ms: u64,
) -> Vec<MeetingSegment> {
    let people = before
        .iter()
        .filter_map(|s| match s.speaker {
            SegmentSpeaker::Person(n) => Some(n),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    before.extend(take.into_iter().map(|s| MeetingSegment {
        speaker: match s.speaker {
            SegmentSpeaker::Person(n) => SegmentSpeaker::Person(people + n),
            other => other,
        },
        t0_ms: s.t0_ms + offset_ms,
        t1_ms: s.t1_ms + offset_ms,
        ..s
    }));
    before
}

pub(crate) fn insert(conn: &Connection, id: SessionId, take: &MeetingTake) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO meeting_takes (meeting_id, take, offset_ms, started_at, ended_at, recorded_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id.to_string(),
            take.take,
            ms(take.offset_ms),
            rfc3339(&take.started_at),
            take.ended_at.as_ref().map(rfc3339),
            ms(take.recorded_ms)
        ],
    )?;
    Ok(())
}

pub(crate) fn takes(conn: &Connection, id: SessionId) -> Result<Vec<MeetingTake>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT take, offset_ms, started_at, ended_at, recorded_ms FROM meeting_takes
         WHERE meeting_id = ?1 ORDER BY take",
    )?;
    let rows = stmt.query_map([id.to_string()], |row| {
        let time = |idx: usize, s: String| {
            DateTime::parse_from_rfc3339(&s).map_err(|e| bad(idx, format!("data inválida: {e}")))
        };
        let (offset, recorded): (i64, i64) = (row.get(1)?, row.get(4)?);
        Ok(MeetingTake {
            take: row.get(0)?,
            offset_ms: u64::try_from(offset).unwrap_or(0),
            started_at: time(2, row.get(2)?)?,
            ended_at: row
                .get::<_, Option<String>>(3)?
                .map(|s| time(3, s))
                .transpose()?,
            recorded_ms: u64::try_from(recorded).unwrap_or(0),
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}
