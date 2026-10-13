//! Os trechos que a pessoa apagou de uma reunião, em ms da gravação, no molde de
//! `meeting_omissions`: as falas que cruzam o intervalo saem, uma linha marcadora
//! "[trecho apagado]" fica no lugar, os trechos são renumerados 1..n e os ponteiros das notas
//! seguem o número novo (`fala_notes::remap_pointers`). Toda transcrição depois recoloca os
//! marcadores e descarta o que o ASR devolver dentro de um intervalo apagado.
//!
//! Tabela aditiva, criada a cada abertura como `meeting_omissions`, sem subir `user_version`.

use std::collections::BTreeMap;

use fala_meeting::SessionId;
use rusqlite::{params, Connection, OptionalExtension};

use crate::meetings::{
    insert_segments, ms, overlaps, read_segment, sort_segments, MeetingSegment, SegmentChannel,
    SegmentSpeaker,
};
use crate::{StorageError, Store};

pub(crate) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meeting_erasures (
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    t0_ms INTEGER NOT NULL,
    t1_ms INTEGER NOT NULL CHECK (t1_ms >= t0_ms)
);
";

/// O texto da linha marcadora, igual na transcrição, no `.md` e no payload das notas.
pub const ERASED_TEXT: &str = "[trecho apagado]";

/// A linha marcadora de um trecho apagado; `seq` é numerado na gravação.
pub fn erased_segment(t0_ms: u64, t1_ms: u64) -> MeetingSegment {
    MeetingSegment {
        seq: 0,
        channel: SegmentChannel::Mic,
        speaker: SegmentSpeaker::Erased,
        t0_ms,
        t1_ms,
        text: ERASED_TEXT.to_string(),
    }
}

/// O que apagar um trecho tirou da sessão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Erasure {
    /// Falas que saíram.
    pub removed: usize,
    /// Linhas das notas que citavam uma delas e agora dizem "(trecho apagado)".
    pub notes_lines: usize,
}

impl Store {
    /// Apaga o texto de `[t0_ms, t1_ms]` nos dois canais, numa transação: as falas que cruzam
    /// o intervalo saem, entra uma linha "[trecho apagado]", os trechos são renumerados e
    /// `remap` reescreve as notas pelo mapa número antigo → novo (`None` se saiu), devolvendo o
    /// texto e quantas linhas citavam um trecho apagado (`fala_notes::remap_pointers`, que fica
    /// fora da árvore deste crate). Os nomes de "Pessoa N" ficam. O áudio é de quem chama.
    pub fn erase_meeting_range(
        &mut self,
        id: SessionId,
        (t0_ms, t1_ms): (u64, u64),
        remap: impl FnOnce(&str, &BTreeMap<u32, Option<u32>>) -> (String, usize),
    ) -> Result<Erasure, StorageError> {
        let t1_ms = t1_ms.max(t0_ms);
        let tx = self.conn.transaction()?;
        let notes: Option<Option<String>> = tx
            .query_row(
                "SELECT notes_md FROM meetings WHERE id = ?1",
                [id.to_string()],
                |row| row.get(0),
            )
            .optional()?;
        let Some(notes) = notes else {
            return Err(StorageError::NotFound(id.to_string()));
        };
        let old: Vec<MeetingSegment> = tx
            .prepare(
                "SELECT seq, channel, speaker, t0_ms, t1_ms, text FROM meeting_segments
                 WHERE meeting_id = ?1 ORDER BY seq",
            )?
            .query_map([id.to_string()], read_segment)?
            .collect::<Result<_, _>>()?;
        let gone = |s: &MeetingSegment| !s.speaker.is_marker() && overlaps(s, t0_ms, t1_ms);
        let removed = old.iter().filter(|s| gone(s)).count();
        let mut segments: Vec<MeetingSegment> = old.iter().filter(|s| !gone(s)).cloned().collect();
        segments.push(erased_segment(t0_ms, t1_ms));
        sort_segments(&mut segments);

        // O número antigo de cada trecho: o novo é a posição na lista ordenada.
        let mut map: BTreeMap<u32, Option<u32>> = old.iter().map(|s| (s.seq, None)).collect();
        for (index, segment) in segments.iter().enumerate() {
            if segment.seq > 0 {
                map.insert(segment.seq, Some(index as u32 + 1));
            }
        }
        tx.execute(
            "INSERT INTO meeting_erasures (meeting_id, t0_ms, t1_ms) VALUES (?1, ?2, ?3)",
            params![id.to_string(), ms(t0_ms), ms(t1_ms)],
        )?;
        tx.execute(
            "DELETE FROM meeting_segments WHERE meeting_id = ?1",
            [id.to_string()],
        )?;
        insert_segments(&tx, id, &segments)?;
        let mut notes_lines = 0;
        if let Some(md) = notes {
            let (md, lines) = remap(&md, &map);
            notes_lines = lines;
            // A nota editada à mão continua editada: só os ponteiros mudaram.
            tx.execute(
                "UPDATE meetings SET notes_md = ?2 WHERE id = ?1",
                params![id.to_string(), md],
            )?;
        }
        tx.commit()?;
        log::debug!("reunião {id}: {removed} trechos apagados");
        self.after_meeting_change(id);
        Ok(Erasure {
            removed,
            notes_lines,
        })
    }

    /// Os trechos apagados da sessão, por início.
    pub fn meeting_erasures(&self, id: SessionId) -> Result<Vec<(u64, u64)>, StorageError> {
        Ok(erasures(&self.conn, id)?)
    }
}

pub(crate) fn erasures(conn: &Connection, id: SessionId) -> rusqlite::Result<Vec<(u64, u64)>> {
    let mut stmt = conn.prepare(
        "SELECT t0_ms, t1_ms FROM meeting_erasures WHERE meeting_id = ?1 ORDER BY t0_ms, t1_ms",
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
