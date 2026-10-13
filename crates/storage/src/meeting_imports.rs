//! Reuniões trazidas de outro app (hoje, o export do Granola, `granola`): cada uma vira uma
//! sessão `import` sem áudio, e `meeting_imports` liga o id da origem à sessão, para importar de
//! novo atualizar em vez de duplicar.
//!
//! Importar de novo reescreve o que é do Granola (data, resumo, transcrição) e guarda o que a
//! pessoa mudou no Fala: o título e as anotações só trocam se ainda são os da última importação
//! (`meeting_imports` guarda os dois), a nota editada à mão fica, e um nome aplicado a "Pessoa N"
//! também.
//!
//! Tabela aditiva, criada a cada abertura como `meeting_speakers`, sem subir `user_version`.
//! Apagar a sessão apaga a ligação (cascata): importar de novo a traz de volta. O `.md` da sessão
//! guarda a ligação (`import` no frontmatter) e o `reindex` a restaura: sem ela, importar de novo
//! depois de perder o banco duplicaria as reuniões.

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, FixedOffset};
use fala_meeting::{SessionId, SessionMode};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::granola::GranolaMeeting;
use crate::meeting_mirror::GENERATED_MARKER;
use crate::meetings::{bad, channel_str, mode_str, rfc3339};
use crate::{MeetingSegment, SegmentChannel, SegmentSpeaker, StorageError, Store};

pub(crate) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meeting_imports (
    source TEXT NOT NULL,
    external_id TEXT NOT NULL,
    meeting_id TEXT NOT NULL UNIQUE REFERENCES meetings(id) ON DELETE CASCADE,
    imported_at TEXT NOT NULL,
    title TEXT NOT NULL,
    annotations TEXT NOT NULL,
    PRIMARY KEY (source, external_id)
);
";

/// A origem das reuniões do "Historical Data Export" do Granola.
pub const GRANOLA: &str = "granola";
/// Rótulos da transcrição que são a própria pessoa (canal do mic).
const ME_LABELS: [&str; 5] = ["me", "you", "eu", "você", "voce"];
/// Rótulos que dizem "outra pessoa" sem dizer quem: viram "Pessoa N" sem nome.
const GENERIC_LABELS: [&str; 5] = ["them", "they", "speaker", "eles", "outro"];
/// Teto de um rótulo de falante, em caracteres; acima disso o ":" é parte da frase.
const LABEL_MAX_CHARS: usize = 40;

/// A linha de `meeting_imports` de uma sessão, como o `.md` a guarda.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct ImportLink {
    pub source: String,
    pub external_id: String,
    /// RFC 3339, como no banco.
    pub imported_at: String,
    /// O título e as anotações da última importação, para importar de novo saber o que a pessoa
    /// mudou.
    pub title: String,
    pub annotations: String,
}

impl ImportLink {
    /// As chaves em ordem alfabética: com ou sem o `preserve_order` do `serde_json` (que outro
    /// crate da árvore pode ligar), o `.md` sai igual.
    pub(crate) fn to_json(&self) -> Value {
        json!({
            "annotations": self.annotations,
            "external_id": self.external_id,
            "imported_at": self.imported_at,
            "source": self.source,
            "title": self.title,
        })
    }

    /// Lê o valor de `import` no frontmatter; o `Err` é o motivo legível para o log.
    pub(crate) fn from_json(value: &Value) -> Result<Self, String> {
        let link = Self::deserialize(value)
            .map_err(|_| "`import` não é a ligação de importação".to_string())?;
        if link.source.is_empty() || link.external_id.is_empty() {
            return Err("`import` sem `source` ou `external_id`".to_string());
        }
        DateTime::parse_from_rfc3339(&link.imported_at)
            .map_err(|e| format!("`import.imported_at` não é RFC 3339: {e}"))?;
        Ok(link)
    }
}

/// A ligação da sessão, se ela foi importada.
pub(crate) fn link(conn: &Connection, id: SessionId) -> rusqlite::Result<Option<ImportLink>> {
    conn.query_row(
        "SELECT source, external_id, imported_at, title, annotations FROM meeting_imports
         WHERE meeting_id = ?1",
        [id.to_string()],
        |row| {
            Ok(ImportLink {
                source: row.get(0)?,
                external_id: row.get(1)?,
                imported_at: row.get(2)?,
                title: row.get(3)?,
                annotations: row.get(4)?,
            })
        },
    )
    .optional()
}

/// Restaura a ligação de uma sessão lida do `.md`. Se o banco já liga a mesma origem a outra
/// sessão, aquela fica (o banco é a fonte de verdade) e esta entra sem ligação; devolve se gravou.
pub(crate) fn restore(
    conn: &Connection,
    id: SessionId,
    link: &ImportLink,
) -> rusqlite::Result<bool> {
    let changed = conn.execute(
        "INSERT INTO meeting_imports
             (source, external_id, meeting_id, imported_at, title, annotations)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT DO NOTHING",
        params![
            link.source,
            link.external_id,
            id.to_string(),
            link.imported_at,
            link.title,
            link.annotations
        ],
    )?;
    Ok(changed > 0)
}

/// O que a importação fez com uma reunião.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportOutcome {
    Created,
    /// A reunião já tinha vindo antes: a sessão dela foi atualizada com o conteúdo de agora.
    Updated,
}

/// O resumo como notas: cada linha não vazia com o marcador de IA (o resumo é texto de IA de
/// outro produto, ADR-0016), menos onde o comentário quebraria o Markdown (bloco de código e
/// linha de tabela), e o rodapé da importação. Resumo em branco: sem notas.
pub(crate) fn summary_notes(summary: &str, imported_at: &DateTime<FixedOffset>) -> Option<String> {
    let mut fenced = false;
    let mut lines = Vec::new();
    for line in summary.lines().map(str::trim_end) {
        let trimmed = line.trim_start();
        let fence = trimmed.starts_with("```") || trimmed.starts_with("~~~");
        if fence {
            fenced = !fenced;
        }
        let plain = trimmed.is_empty() || fence || fenced || trimmed.starts_with('|');
        lines.push(if plain {
            line.to_string()
        } else {
            format!("{line} {GENERATED_MARKER}")
        });
    }
    let body = lines.join("\n");
    let body = body.trim_matches('\n');
    if body.trim().is_empty() {
        return None;
    }
    Some(format!(
        "{body}\n\n_Importado do Granola em {}._",
        imported_at.format("%Y-%m-%d")
    ))
}

/// `Rótulo: fala` → o rótulo, se ele parece um nome: começa com letra, é curto e tem espaço
/// depois do ":" (assim `10:30` e `https://` ficam no texto).
fn split_label(line: &str) -> Option<(&str, &str)> {
    let (label, text) = line.split_once(':')?;
    let label = label.trim();
    let named = label.chars().next().is_some_and(char::is_alphabetic)
        && label.chars().count() <= LABEL_MAX_CHARS;
    (named && (text.is_empty() || text.starts_with(char::is_whitespace))).then_some((label, text))
}

/// Quem um rótulo é: a própria pessoa, uma "Pessoa N" sem nome (`Them`), uma com o rótulo como
/// nome, ou ninguém (o ":" é parte da frase).
#[derive(Clone, Copy)]
enum Label {
    Me,
    Other { named: bool },
}

/// A transcrição do CSV em trechos, um por linha não vazia, sem tempo (o CSV não tem): "Me:" e
/// "You:" são o mic; "Them:" e cada nome são uma "Pessoa N", na ordem em que aparecem, com o
/// nome aplicado. Um nome só conta como falante se abre mais de uma linha (assim "Próximos
/// passos: …" fica no texto); uma linha sem rótulo continua com quem falava antes.
pub(crate) fn transcript_segments(text: &str) -> (Vec<MeetingSegment>, BTreeMap<u32, String>) {
    let mut opens: HashMap<&str, usize> = HashMap::new();
    for (label, _) in text.lines().filter_map(split_label) {
        *opens.entry(label).or_default() += 1;
    }
    let kind = |label: &str| {
        let lower = label.to_lowercase();
        if ME_LABELS.contains(&lower.as_str()) {
            Some(Label::Me)
        } else if GENERIC_LABELS.contains(&lower.as_str()) {
            Some(Label::Other { named: false })
        } else if opens.get(label).is_some_and(|n| *n > 1) {
            Some(Label::Other { named: true })
        } else {
            None
        }
    };
    let mut segments = Vec::new();
    let mut people: Vec<&str> = Vec::new();
    let mut names: BTreeMap<u32, String> = BTreeMap::new();
    let mut speaker = (SegmentChannel::System, SegmentSpeaker::Person(1));
    for line in text.lines() {
        let labelled =
            split_label(line).and_then(|(label, said)| Some((label, said, kind(label)?)));
        let said = match labelled {
            Some((_, said, Label::Me)) => {
                speaker = (SegmentChannel::Mic, SegmentSpeaker::Me);
                said
            }
            Some((label, said, Label::Other { named })) => {
                let index = people.iter().position(|p| *p == label).unwrap_or_else(|| {
                    people.push(label);
                    people.len() - 1
                });
                let person = index as u32 + 1;
                if named {
                    names.insert(person, label.to_string());
                }
                speaker = (SegmentChannel::System, SegmentSpeaker::Person(person));
                said
            }
            None => line,
        };
        let said = said.trim();
        if said.is_empty() {
            continue;
        }
        segments.push(MeetingSegment {
            seq: segments.len() as u32 + 1,
            channel: speaker.0,
            speaker: speaker.1,
            t0_ms: 0,
            t1_ms: 0,
            text: said.to_string(),
        });
    }
    (segments, names)
}

impl Store {
    /// Grava uma reunião do Granola: sessão `import` terminada, sem áudio, no instante de
    /// `document_created`; `notes` vira as anotações, `summary` as notas (marcadas como IA) e
    /// `transcript` os trechos. O título vazio fica vazio: a UI mostra "Reunião" e a data no
    /// idioma dela. Se o `document_id` já veio antes, a mesma sessão é atualizada, sem desfazer
    /// o que a pessoa mudou no Fala (ver o módulo).
    ///
    /// Tudo numa transação; a linha de busca e o `.md` da sessão vêm depois do commit.
    pub fn import_granola_meeting(
        &mut self,
        meeting: &GranolaMeeting,
        imported_at: DateTime<FixedOffset>,
    ) -> Result<(SessionId, ImportOutcome), StorageError> {
        let (segments, names) = transcript_segments(&meeting.transcript);
        let notes = summary_notes(&meeting.summary, &imported_at);
        let transcribed = (!segments.is_empty()).then(|| rfc3339(&imported_at));
        let created = rfc3339(&meeting.created_at);
        let created_ms = meeting.created_at.timestamp_millis();
        let tx = self.conn.transaction()?;
        let known: Option<String> = tx
            .query_row(
                "SELECT meeting_id FROM meeting_imports WHERE source = ?1 AND external_id = ?2",
                params![GRANOLA, meeting.document_id],
                |row| row.get(0),
            )
            .optional()?;
        let (id, outcome) = match known {
            Some(id) => {
                let id: SessionId = id
                    .parse()
                    .map_err(|_| bad(0, format!("id de sessão inválido {id:?}")))?;
                tx.execute(
                    "UPDATE meetings SET created_at = ?2, created_ms = ?3, started_at = ?2,
                         started_ms = ?3, ended_at = ?2, transcribed_at = ?4,
                         title = CASE WHEN title = (SELECT i.title FROM meeting_imports i
                             WHERE i.meeting_id = ?1) THEN ?5 ELSE title END,
                         annotations = CASE WHEN annotations = (SELECT i.annotations
                             FROM meeting_imports i WHERE i.meeting_id = ?1)
                             THEN ?6 ELSE annotations END,
                         notes_md = CASE WHEN EXISTS (SELECT 1 FROM meeting_note_edits e
                             WHERE e.meeting_id = ?1) THEN notes_md ELSE ?7 END
                     WHERE id = ?1",
                    params![
                        id.to_string(),
                        created,
                        created_ms,
                        transcribed,
                        meeting.title,
                        meeting.notes,
                        notes
                    ],
                )?;
                tx.execute(
                    "DELETE FROM meeting_segments WHERE meeting_id = ?1",
                    [id.to_string()],
                )?;
                (id, ImportOutcome::Updated)
            }
            None => {
                // 80 bits de entropia sem pedir ao SO de novo: o UUID v7 já os tem.
                let entropy = uuid::Uuid::now_v7().as_u128();
                let id = SessionId::from_parts(u64::try_from(created_ms).unwrap_or(0), entropy);
                tx.execute(
                    "INSERT INTO meetings (id, title, mode, local_only, created_at, created_ms,
                         started_at, started_ms, ended_at, annotations, transcribed_at, notes_md)
                     VALUES (?1, ?2, ?3, 0, ?4, ?5, ?4, ?5, ?4, ?6, ?7, ?8)",
                    params![
                        id.to_string(),
                        meeting.title,
                        mode_str(SessionMode::Import),
                        created,
                        created_ms,
                        meeting.notes,
                        transcribed,
                        notes
                    ],
                )?;
                (id, ImportOutcome::Created)
            }
        };
        for segment in &segments {
            let speaker = serde_json::to_string(&segment.speaker)
                .map_err(|e| StorageError::Db(bad(3, e.to_string())))?;
            tx.execute(
                "INSERT INTO meeting_segments (meeting_id, seq, channel, speaker, t0_ms, t1_ms, text)
                 VALUES (?1, ?2, ?3, ?4, 0, 0, ?5)",
                params![
                    id.to_string(),
                    segment.seq,
                    channel_str(segment.channel),
                    speaker,
                    segment.text
                ],
            )?;
        }
        // Um nome que a pessoa já aplicou fica; os do Granola só preenchem quem está sem nome.
        for (person, name) in &names {
            tx.execute(
                "INSERT INTO meeting_speakers (meeting_id, person, name) VALUES (?1, ?2, ?3)
                 ON CONFLICT (meeting_id, person) DO UPDATE SET name = excluded.name
                 WHERE meeting_speakers.name IS NULL",
                params![id.to_string(), person, name],
            )?;
        }
        tx.execute(
            "INSERT INTO meeting_imports
                 (source, external_id, meeting_id, imported_at, title, annotations)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (source, external_id) DO UPDATE SET imported_at = excluded.imported_at,
                 title = excluded.title, annotations = excluded.annotations",
            params![
                GRANOLA,
                meeting.document_id,
                id.to_string(),
                rfc3339(&imported_at),
                meeting.title,
                meeting.notes
            ],
        )?;
        tx.commit()?;
        log::debug!("reunião {id} importada do Granola ({outcome:?})");
        self.after_meeting_change(id);
        Ok((id, outcome))
    }

    /// De onde cada sessão importada veio (`granola`), numa consulta só, para a lista.
    pub fn meeting_import_sources(&self) -> Result<HashMap<SessionId, String>, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT meeting_id, source FROM meeting_imports")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut sources = HashMap::new();
        for row in rows {
            let (id, source) = row?;
            if let Ok(id) = id.parse() {
                sources.insert(id, source);
            }
        }
        Ok(sources)
    }

    /// De onde a sessão foi importada (`granola`), se foi.
    pub fn meeting_import_source(&self, id: SessionId) -> Result<Option<String>, StorageError> {
        Ok(link(&self.conn, id)?.map(|link| link.source))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(
        channel: SegmentChannel,
        speaker: SegmentSpeaker,
        seq: u32,
        text: &str,
    ) -> MeetingSegment {
        MeetingSegment {
            seq,
            channel,
            speaker,
            t0_ms: 0,
            t1_ms: 0,
            text: text.to_string(),
        }
    }

    #[test]
    fn transcript_lines_become_untimed_segments_by_speaker() {
        let text = "Me: Bom dia.\r\n\nThem: Oi, tudo bem?\n  continua aqui\n\
                    Ana Souza: Às 10:30 então.\nThem: Próximos passos: contrato\n\
                    You: Fechado: https://x.y\nAna Souza: certo\n\
                    Próximos passos: não é falante\n10:30 sem rótulo\nThem:\n";
        let (segments, names) = transcript_segments(text);
        use SegmentChannel::{Mic, System};
        use SegmentSpeaker::{Me, Person};
        assert_eq!(
            segments,
            [
                seg(Mic, Me, 1, "Bom dia."),
                seg(System, Person(1), 2, "Oi, tudo bem?"),
                seg(System, Person(1), 3, "continua aqui"),
                seg(System, Person(2), 4, "Às 10:30 então."),
                seg(System, Person(1), 5, "Próximos passos: contrato"),
                seg(Mic, Me, 6, "Fechado: https://x.y"),
                seg(System, Person(2), 7, "certo"),
                seg(System, Person(2), 8, "Próximos passos: não é falante"),
                seg(System, Person(2), 9, "10:30 sem rótulo"),
            ]
        );
        // "Them" fica "Pessoa 1"; só o nome que abre mais de uma fala vira nome aplicado.
        assert_eq!(names, BTreeMap::from([(2, "Ana Souza".to_string())]));
        // Sem rótulo nenhum, tudo é a "Pessoa 1", sem nome.
        let (segments, names) = transcript_segments("primeira linha\nsegunda");
        assert_eq!(segments.len(), 2);
        assert!(segments.iter().all(|s| s.speaker == Person(1)));
        assert!(names.is_empty());
    }

    #[test]
    fn the_summary_is_marked_as_ai_line_by_line() {
        let at = DateTime::parse_from_rfc3339("2026-10-12T21:00:00-03:00").unwrap();
        let summary = "\n### Decisões\n\n- Lançar em novembro  \n```\ncódigo\n```\n| a | b |\n";
        assert_eq!(
            summary_notes(summary, &at).unwrap(),
            "### Decisões <!-- fala:ia -->\n\n- Lançar em novembro <!-- fala:ia -->\n\
             ```\ncódigo\n```\n| a | b |\n\n_Importado do Granola em 2026-10-12._"
        );
        assert_eq!(summary_notes(" \n\n", &at), None);
    }
}
