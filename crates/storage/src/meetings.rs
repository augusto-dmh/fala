//! Reuniões no `fala.sqlite` (schema 2): a sessão, as anotações que a pessoa digita antes e
//! durante a gravação, os trechos transcritos e as notas geradas.
//!
//! Uma sessão nasce como rascunho (`started_at` nulo: título e pauta, sem áudio) e passa a
//! gravada no clique de "Gravar" (ADR-0005); um rascunho nunca grava sozinho.
//!
//! Só SQLite por enquanto: o espelho `.md` da reunião e o FTS das transcrições são da F5
//! (`meeting-storage`), por migração aditiva.

use chrono::{DateTime, FixedOffset, SecondsFormat};
use fala_meeting::{SessionId, SessionMode, StopReason};
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::{StorageError, Store};

pub(crate) const SCHEMA_2: &str = "
CREATE TABLE IF NOT EXISTS meetings (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    mode TEXT NOT NULL CHECK (mode IN ('meeting', 'in_person', 'system_only', 'import')),
    local_only INTEGER NOT NULL DEFAULT 0 CHECK (local_only IN (0, 1)),
    created_at TEXT NOT NULL,
    created_ms INTEGER NOT NULL,
    started_at TEXT,
    started_ms INTEGER,
    ended_at TEXT CHECK (ended_at IS NULL OR started_at IS NOT NULL),
    recorded_ms INTEGER NOT NULL DEFAULT 0,
    stop_reason TEXT CHECK (stop_reason IN ('user', 'silence', 'cap_reached')),
    audio_retained INTEGER NOT NULL DEFAULT 0 CHECK (audio_retained IN (0, 1)),
    annotations TEXT NOT NULL DEFAULT '',
    transcribed_at TEXT,
    notes_md TEXT,
    notes_template TEXT
);
CREATE INDEX IF NOT EXISTS meetings_by_time ON meetings (created_ms DESC);
CREATE TABLE IF NOT EXISTS meeting_segments (
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    seq INTEGER NOT NULL CHECK (seq >= 1),
    channel TEXT NOT NULL CHECK (channel IN ('mic', 'system')),
    speaker TEXT NOT NULL,
    t0_ms INTEGER NOT NULL,
    t1_ms INTEGER NOT NULL,
    text TEXT NOT NULL,
    PRIMARY KEY (meeting_id, seq)
);
";

const COLUMNS: &str =
    "id, title, mode, local_only, created_at, started_at, ended_at, recorded_ms, \
     stop_reason, audio_retained, annotations, transcribed_at, notes_md, notes_template";

/// Uma sessão nova, ainda rascunho: sem áudio até [`Store::start_meeting_recording`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMeeting {
    pub id: SessionId,
    pub title: String,
    pub mode: SessionMode,
    pub local_only: bool,
    pub created_at: DateTime<FixedOffset>,
}

/// Uma sessão persistida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingRecord {
    pub id: SessionId,
    pub title: String,
    pub mode: SessionMode,
    pub local_only: bool,
    pub created_at: DateTime<FixedOffset>,
    /// `None` enquanto a sessão é rascunho (nunca gravou).
    pub started_at: Option<DateTime<FixedOffset>>,
    /// `None` enquanto grava ou se o processo morreu antes do fim.
    pub ended_at: Option<DateTime<FixedOffset>>,
    pub recorded_ms: u64,
    pub stop_reason: Option<StopReason>,
    /// `mic.opus` e `sys.opus` validados em `audio/<id>/`.
    pub audio_retained: bool,
    /// O texto digitado antes (pauta) e durante a sessão, como está.
    pub annotations: String,
    pub transcribed_at: Option<DateTime<FixedOffset>>,
    pub notes_md: Option<String>,
    pub notes_template: Option<String>,
}

/// O canal de um trecho.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentChannel {
    Mic,
    System,
}

/// Quem fala, na forma da door 4 do `meeting-asr`: `"me"` ou `{"person": N}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentSpeaker {
    Me,
    Person(u32),
}

/// Um trecho transcrito. `seq` (1 a n, na ordem de `t0_ms`) é o id que os ponteiros `^sN` das
/// notas citam; na gravação ele é recalculado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingSegment {
    pub seq: u32,
    pub channel: SegmentChannel,
    pub speaker: SegmentSpeaker,
    pub t0_ms: u64,
    pub t1_ms: u64,
    pub text: String,
}

fn mode_str(mode: SessionMode) -> &'static str {
    match mode {
        SessionMode::Meeting => "meeting",
        SessionMode::InPerson => "in_person",
        SessionMode::SystemOnly => "system_only",
        SessionMode::Import => "import",
    }
}

fn parse_mode(s: &str) -> Option<SessionMode> {
    Some(match s {
        "meeting" => SessionMode::Meeting,
        "in_person" => SessionMode::InPerson,
        "system_only" => SessionMode::SystemOnly,
        "import" => SessionMode::Import,
        _ => return None,
    })
}

fn reason_str(reason: StopReason) -> &'static str {
    match reason {
        StopReason::User => "user",
        StopReason::Silence => "silence",
        StopReason::CapReached => "cap_reached",
    }
}

fn parse_reason(s: &str) -> Option<StopReason> {
    Some(match s {
        "user" => StopReason::User,
        "silence" => StopReason::Silence,
        "cap_reached" => StopReason::CapReached,
        _ => return None,
    })
}

fn channel_str(channel: SegmentChannel) -> &'static str {
    match channel {
        SegmentChannel::Mic => "mic",
        SegmentChannel::System => "system",
    }
}

fn rfc3339(at: &DateTime<FixedOffset>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, false)
}

fn bad(idx: usize, what: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(idx, rusqlite::types::Type::Text, what.into())
}

fn parse_time(idx: usize, s: &str) -> rusqlite::Result<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(s).map_err(|e| bad(idx, format!("data inválida {s:?}: {e}")))
}

fn read_meeting(row: &Row<'_>) -> rusqlite::Result<MeetingRecord> {
    let id: String = row.get(0)?;
    let mode: String = row.get(2)?;
    let created_at: String = row.get(4)?;
    let started_at: Option<String> = row.get(5)?;
    let ended_at: Option<String> = row.get(6)?;
    let recorded_ms: i64 = row.get(7)?;
    let stop_reason: Option<String> = row.get(8)?;
    let transcribed_at: Option<String> = row.get(11)?;
    Ok(MeetingRecord {
        id: id
            .parse()
            .map_err(|_| bad(0, format!("id de sessão inválido {id:?}")))?,
        title: row.get(1)?,
        mode: parse_mode(&mode).ok_or_else(|| bad(2, format!("modo inválido {mode:?}")))?,
        local_only: row.get(3)?,
        created_at: parse_time(4, &created_at)?,
        started_at: started_at
            .as_deref()
            .map(|s| parse_time(5, s))
            .transpose()?,
        ended_at: ended_at.as_deref().map(|s| parse_time(6, s)).transpose()?,
        recorded_ms: u64::try_from(recorded_ms).unwrap_or(0),
        stop_reason: stop_reason
            .as_deref()
            .map(|s| parse_reason(s).ok_or_else(|| bad(8, format!("motivo inválido {s:?}"))))
            .transpose()?,
        audio_retained: row.get(9)?,
        annotations: row.get(10)?,
        transcribed_at: transcribed_at
            .as_deref()
            .map(|s| parse_time(11, s))
            .transpose()?,
        notes_md: row.get(12)?,
        notes_template: row.get(13)?,
    })
}

fn read_segment(row: &Row<'_>) -> rusqlite::Result<MeetingSegment> {
    let seq: i64 = row.get(0)?;
    let channel: String = row.get(1)?;
    let speaker: String = row.get(2)?;
    let t0_ms: i64 = row.get(3)?;
    let t1_ms: i64 = row.get(4)?;
    Ok(MeetingSegment {
        seq: u32::try_from(seq).map_err(|_| bad(0, format!("seq inválido {seq}")))?,
        channel: match channel.as_str() {
            "mic" => SegmentChannel::Mic,
            "system" => SegmentChannel::System,
            other => return Err(bad(1, format!("canal inválido {other:?}"))),
        },
        speaker: serde_json::from_str(&speaker)
            .map_err(|_| bad(2, format!("falante inválido {speaker:?}")))?,
        t0_ms: u64::try_from(t0_ms).unwrap_or(0),
        t1_ms: u64::try_from(t1_ms).unwrap_or(0),
        text: row.get(5)?,
    })
}

fn ms(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

impl Store {
    /// Grava uma sessão nova como rascunho: título, modo e criação, sem áudio.
    pub fn create_meeting(&self, meeting: &NewMeeting) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO meetings (id, title, mode, local_only, created_at, created_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                meeting.id.to_string(),
                meeting.title,
                mode_str(meeting.mode),
                meeting.local_only,
                rfc3339(&meeting.created_at),
                meeting.created_at.timestamp_millis(),
            ],
        )?;
        log::debug!("reunião {} criada como rascunho", meeting.id);
        Ok(())
    }

    /// Rascunho vira gravação: grava o início e o modo escolhido no clique.
    ///
    /// Uma sessão que já começou não recomeça: devolve [`StorageError::MeetingAlreadyStarted`].
    pub fn start_meeting_recording(
        &self,
        id: SessionId,
        mode: SessionMode,
        started_at: DateTime<FixedOffset>,
    ) -> Result<(), StorageError> {
        let changed = self.conn.execute(
            "UPDATE meetings SET mode = ?2, started_at = ?3, started_ms = ?4
             WHERE id = ?1 AND started_at IS NULL",
            params![
                id.to_string(),
                mode_str(mode),
                rfc3339(&started_at),
                started_at.timestamp_millis()
            ],
        )?;
        if changed == 0 {
            self.meeting(id)?;
            return Err(StorageError::MeetingAlreadyStarted(id.to_string()));
        }
        Ok(())
    }

    /// Troca o título.
    pub fn set_meeting_title(&self, id: SessionId, title: &str) -> Result<(), StorageError> {
        self.update(
            id,
            "UPDATE meetings SET title = ?2 WHERE id = ?1",
            params![id.to_string(), title],
        )
    }

    /// A sessão `id`.
    pub fn meeting(&self, id: SessionId) -> Result<MeetingRecord, StorageError> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM meetings WHERE id = ?1"),
                [id.to_string()],
                read_meeting,
            )
            .optional()?
            .ok_or_else(|| StorageError::NotFound(id.to_string()))
    }

    /// Todas as sessões, rascunhos inclusive, da criada mais recentemente para a mais antiga.
    pub fn meetings(&self) -> Result<Vec<MeetingRecord>, StorageError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {COLUMNS} FROM meetings ORDER BY created_ms DESC, id DESC"
        ))?;
        let rows = stmt.query_map([], read_meeting)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Fim da gravação: instante, duração gravada (sem pausas) e motivo.
    pub fn finish_meeting(
        &self,
        id: SessionId,
        ended_at: DateTime<FixedOffset>,
        recorded_ms: u64,
        reason: StopReason,
    ) -> Result<(), StorageError> {
        self.update(
            id,
            "UPDATE meetings SET ended_at = ?2, recorded_ms = ?3, stop_reason = ?4 WHERE id = ?1",
            params![
                id.to_string(),
                rfc3339(&ended_at),
                ms(recorded_ms),
                reason_str(reason)
            ],
        )
    }

    /// Marca que `mic.opus` e `sys.opus` da sessão foram validados.
    pub fn set_meeting_audio_retained(&self, id: SessionId) -> Result<(), StorageError> {
        self.update(
            id,
            "UPDATE meetings SET audio_retained = 1 WHERE id = ?1",
            params![id.to_string()],
        )
    }

    /// Substitui as anotações da sessão pelo texto inteiro.
    pub fn save_meeting_annotations(&self, id: SessionId, text: &str) -> Result<(), StorageError> {
        self.update(
            id,
            "UPDATE meetings SET annotations = ?2 WHERE id = ?1",
            params![id.to_string(), text],
        )
    }

    /// Grava as notas geradas e o template usado.
    pub fn save_meeting_notes(
        &self,
        id: SessionId,
        template: &str,
        markdown: &str,
    ) -> Result<(), StorageError> {
        self.update(
            id,
            "UPDATE meetings SET notes_template = ?2, notes_md = ?3 WHERE id = ?1",
            params![id.to_string(), template, markdown],
        )
    }

    /// Substitui os trechos da sessão, numa transação: ordena por `t0_ms` (o mic antes do
    /// sistema no empate), numera `seq` de 1 a n e grava `transcribed_at`.
    pub fn replace_meeting_segments(
        &mut self,
        id: SessionId,
        mut segments: Vec<MeetingSegment>,
        transcribed_at: DateTime<FixedOffset>,
    ) -> Result<(), StorageError> {
        segments.sort_by_key(|s| (s.t0_ms, s.channel == SegmentChannel::System));
        let tx = self.conn.transaction()?;
        let changed = tx.execute(
            "UPDATE meetings SET transcribed_at = ?2 WHERE id = ?1",
            params![id.to_string(), rfc3339(&transcribed_at)],
        )?;
        if changed == 0 {
            return Err(StorageError::NotFound(id.to_string()));
        }
        tx.execute(
            "DELETE FROM meeting_segments WHERE meeting_id = ?1",
            [id.to_string()],
        )?;
        for (index, segment) in segments.iter().enumerate() {
            let speaker = serde_json::to_string(&segment.speaker)
                .map_err(|e| StorageError::Db(bad(3, e.to_string())))?;
            tx.execute(
                "INSERT INTO meeting_segments (meeting_id, seq, channel, speaker, t0_ms, t1_ms, text)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    id.to_string(),
                    index as i64 + 1,
                    channel_str(segment.channel),
                    speaker,
                    ms(segment.t0_ms),
                    ms(segment.t1_ms),
                    segment.text,
                ],
            )?;
        }
        tx.commit()?;
        log::debug!("reunião {id}: {} trechos gravados", segments.len());
        Ok(())
    }

    /// Os trechos da sessão, por `seq`.
    pub fn meeting_segments(&self, id: SessionId) -> Result<Vec<MeetingSegment>, StorageError> {
        // Distingue "sessão sem trechos" de "sessão inexistente".
        self.meeting(id)?;
        let mut stmt = self.conn.prepare(
            "SELECT seq, channel, speaker, t0_ms, t1_ms, text FROM meeting_segments
             WHERE meeting_id = ?1 ORDER BY seq",
        )?;
        let rows = stmt.query_map([id.to_string()], read_segment)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    fn update(
        &self,
        id: SessionId,
        sql: &str,
        params: impl rusqlite::Params,
    ) -> Result<(), StorageError> {
        if self.conn.execute(sql, params)? == 0 {
            return Err(StorageError::NotFound(id.to_string()));
        }
        Ok(())
    }
}
