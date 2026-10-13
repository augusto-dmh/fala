//! Busca nas reuniões: `meetings_fts` (`unicode61 remove_diacritics 2`, como os ditados) com uma
//! linha por sessão, de título, anotações, notas e trechos.
//!
//! Tabela aditiva com o conteúdo copiado (os trechos ficam em outra tabela, e o `rowid` de
//! `meetings` não é estável num `VACUUM`), atualizada depois de cada mudança da sessão, junto com
//! o espelho. [`Store::index_meetings_for_search`] reconstrói o índice quando falta alguma sessão.

use fala_meeting::SessionId;
use rusqlite::{params, Connection};

use crate::store::match_expr;
use crate::{StorageError, Store};

pub(crate) const SCHEMA: &str = "
CREATE VIRTUAL TABLE IF NOT EXISTS meetings_fts USING fts5(
    meeting_id UNINDEXED, title, annotations, notes, transcript,
    tokenize = 'unicode61 remove_diacritics 2'
);
";

/// A linha de busca de cada sessão de `filter` (`WHERE` sobre `m`), lida do banco agora.
const ROWS: &str = "
INSERT INTO meetings_fts (meeting_id, title, annotations, notes, transcript)
SELECT m.id, m.title, m.annotations, coalesce(m.notes_md, ''),
       coalesce((SELECT group_concat(text, ' ')
                 FROM (SELECT text FROM meeting_segments s WHERE s.meeting_id = m.id ORDER BY seq)),
                '')
FROM meetings m";

/// Início e fim de um termo achado dentro de [`MeetingHit::snippet`].
pub const MATCH_START: char = '\u{2}';
pub const MATCH_END: char = '\u{3}';

/// Uma sessão achada pela busca.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingHit {
    pub id: SessionId,
    /// O trecho da coluna que casou, com cada termo entre [`MATCH_START`] e [`MATCH_END`].
    pub snippet: String,
}

/// Troca a linha de busca da sessão pela de agora; sessão apagada fica sem linha.
pub(crate) fn refresh(conn: &Connection, id: SessionId) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM meetings_fts WHERE meeting_id = ?1",
        [id.to_string()],
    )?;
    conn.execute(&format!("{ROWS} WHERE m.id = ?1"), [id.to_string()])?;
    Ok(())
}

impl Store {
    /// Reconstrói o índice de busca das reuniões se ele não tem uma linha por sessão (banco de
    /// antes da busca, ou uma atualização que falhou); devolve quantas linhas escreveu.
    pub fn index_meetings_for_search(&mut self) -> Result<usize, StorageError> {
        let tx = self.conn.transaction()?;
        let indexed: bool = tx.query_row(
            "SELECT (SELECT count(*) FROM meetings) = (SELECT count(*) FROM meetings_fts)
                AND NOT EXISTS (SELECT 1 FROM meetings m WHERE NOT EXISTS
                    (SELECT 1 FROM meetings_fts f WHERE f.meeting_id = m.id))",
            [],
            |row| row.get(0),
        )?;
        if indexed {
            return Ok(0);
        }
        tx.execute("DELETE FROM meetings_fts", [])?;
        let written = tx.execute(ROWS, [])?;
        tx.commit()?;
        log::info!("busca de reuniões reconstruída: {written} sessões");
        Ok(written)
    }

    /// Busca sem acento no título, nas anotações, nas notas e na transcrição das reuniões, das
    /// criadas mais recentemente para as mais antigas.
    ///
    /// Cada termo casa como prefixo e todos precisam casar, em qualquer coluna; a sintaxe do
    /// FTS5 é tratada como texto. Consulta vazia não acha nada.
    pub fn search_meetings(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MeetingHit>, StorageError> {
        let terms: Vec<&str> = query.split_whitespace().collect();
        let Some(fts_query) = match_expr(&terms).filter(|_| !terms.is_empty()) else {
            return Ok(Vec::new());
        };
        let mut stmt = self.conn.prepare(&format!(
            "SELECT f.meeting_id, snippet(meetings_fts, -1, '{MATCH_START}', '{MATCH_END}', '…', 16)
             FROM meetings_fts f JOIN meetings m ON m.id = f.meeting_id
             WHERE meetings_fts MATCH ?1
             ORDER BY m.created_ms DESC, m.id DESC LIMIT ?2"
        ))?;
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let rows = stmt.query_map(params![fts_query, limit], |row| {
            let id: String = row.get(0)?;
            Ok((id, row.get::<_, String>(1)?))
        })?;
        let mut hits = Vec::new();
        for row in rows {
            let (id, snippet) = row?;
            let id = id.parse().map_err(|_| {
                StorageError::Db(rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    format!("id de sessão inválido {id:?}").into(),
                ))
            })?;
            hits.push(MeetingHit { id, snippet });
        }
        Ok(hits)
    }
}
