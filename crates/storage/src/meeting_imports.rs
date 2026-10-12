//! Reuniões trazidas de outro app (hoje, o export do Granola, `granola`): cada uma vira uma
//! sessão `import` sem áudio, e `meeting_imports` liga o id da origem à sessão, para importar de
//! novo atualizar em vez de duplicar.
//!
//! Tabela aditiva, criada a cada abertura como `meeting_speakers`, sem subir `user_version`.
//! Apagar a sessão apaga a ligação (cascata): importar de novo a traz de volta.

use std::collections::BTreeMap;

use chrono::{DateTime, FixedOffset};
use fala_meeting::{SessionId, SessionMode};
use rusqlite::{params, OptionalExtension};

use crate::granola::GranolaMeeting;
use crate::meetings::{bad, channel_str, mode_str, rfc3339};
use crate::{MeetingSegment, SegmentChannel, SegmentSpeaker, StorageError, Store};

pub(crate) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meeting_imports (
    source TEXT NOT NULL,
    external_id TEXT NOT NULL,
    meeting_id TEXT NOT NULL UNIQUE REFERENCES meetings(id) ON DELETE CASCADE,
    imported_at TEXT NOT NULL,
    PRIMARY KEY (source, external_id)
);
";

/// A origem das reuniões do "Historical Data Export" do Granola.
pub const GRANOLA: &str = "granola";
/// O marcador das linhas geradas (`fala_notes::GENERATED_MARKER`): o resumo do Granola é texto
/// de IA de outro produto (ADR-0016).
const GENERATED_MARKER: &str = "<!-- fala:ia -->";
/// Rótulos da transcrição que são a própria pessoa (canal do mic).
const ME_LABELS: [&str; 5] = ["me", "you", "eu", "você", "voce"];
/// Teto de um rótulo de falante, em caracteres; acima disso o ":" é parte da frase.
const LABEL_MAX_CHARS: usize = 40;

/// O que a importação fez com uma reunião.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportOutcome {
    Created,
    /// A reunião já tinha vindo antes: a sessão dela foi reescrita com o conteúdo de agora.
    Updated,
}

/// O resumo como notas: cada linha não vazia com o marcador de IA, e o rodapé da importação.
/// Resumo em branco: sem notas.
pub(crate) fn summary_notes(summary: &str, imported_at: &DateTime<FixedOffset>) -> Option<String> {
    let lines: Vec<String> = summary
        .lines()
        .map(str::trim_end)
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                format!("{line} {GENERATED_MARKER}")
            }
        })
        .collect();
    let body = lines.join("\n");
    let body = body.trim_matches('\n');
    if body.is_empty() {
        return None;
    }
    Some(format!(
        "{body}\n\n_Importado do Granola em {}._",
        imported_at.format("%d/%m/%Y")
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

/// A transcrição do CSV em trechos, um por linha não vazia, sem tempo (o CSV não tem): "Me:" e
/// "You:" são o mic; cada outro rótulo é uma "Pessoa N", na ordem em que aparece, com o rótulo
/// como nome. Uma linha sem rótulo continua com quem falava antes.
pub(crate) fn transcript_segments(text: &str) -> (Vec<MeetingSegment>, BTreeMap<u32, String>) {
    let mut segments = Vec::new();
    let mut names: BTreeMap<u32, String> = BTreeMap::new();
    let mut speaker = (SegmentChannel::System, SegmentSpeaker::Person(1));
    for line in text.lines() {
        let (label, said) = match split_label(line) {
            Some((label, said)) => (Some(label), said.trim()),
            None => (None, line.trim()),
        };
        if let Some(label) = label {
            speaker = if ME_LABELS.contains(&label.to_lowercase().as_str()) {
                (SegmentChannel::Mic, SegmentSpeaker::Me)
            } else {
                let person = names
                    .iter()
                    .find(|(_, name)| name.as_str() == label)
                    .map(|(n, _)| *n)
                    .unwrap_or(names.len() as u32 + 1);
                names.insert(person, label.to_string());
                (SegmentChannel::System, SegmentSpeaker::Person(person))
            };
        }
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

fn title(meeting: &GranolaMeeting) -> String {
    if meeting.title.is_empty() {
        format!(
            "Reunião do Granola de {}",
            meeting.created_at.format("%d/%m/%Y")
        )
    } else {
        meeting.title.clone()
    }
}

impl Store {
    /// Grava uma reunião do Granola: sessão `import` terminada, sem áudio, no instante de
    /// `document_created`; `notes` vira as anotações, `summary` as notas (marcadas como IA) e
    /// `transcript` os trechos. Se o `document_id` já veio antes, a mesma sessão é reescrita.
    ///
    /// Tudo numa transação; o `.md` da sessão vem depois do commit.
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
                    "UPDATE meetings SET title = ?2, created_at = ?3, created_ms = ?4,
                         started_at = ?3, started_ms = ?4, ended_at = ?3, annotations = ?5,
                         transcribed_at = ?6, notes_md = ?7
                     WHERE id = ?1",
                    params![
                        id.to_string(),
                        title(meeting),
                        created,
                        created_ms,
                        meeting.notes,
                        transcribed,
                        notes
                    ],
                )?;
                for table in ["meeting_segments", "meeting_speakers", "meeting_note_edits"] {
                    tx.execute(
                        &format!("DELETE FROM {table} WHERE meeting_id = ?1"),
                        [id.to_string()],
                    )?;
                }
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
                        title(meeting),
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
        for (person, name) in &names {
            tx.execute(
                "INSERT INTO meeting_speakers (meeting_id, person, name) VALUES (?1, ?2, ?3)",
                params![id.to_string(), person, name],
            )?;
        }
        tx.execute(
            "INSERT INTO meeting_imports (source, external_id, meeting_id, imported_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (source, external_id) DO UPDATE SET imported_at = excluded.imported_at",
            params![
                GRANOLA,
                meeting.document_id,
                id.to_string(),
                rfc3339(&imported_at)
            ],
        )?;
        tx.commit()?;
        log::debug!("reunião {id} importada do Granola ({outcome:?})");
        self.mirror_meeting(id);
        Ok((id, outcome))
    }

    /// De onde a sessão foi importada (`granola`), se foi.
    pub fn meeting_import_source(&self, id: SessionId) -> Result<Option<String>, StorageError> {
        Ok(self
            .conn
            .query_row(
                "SELECT source FROM meeting_imports WHERE meeting_id = ?1",
                [id.to_string()],
                |row| row.get(0),
            )
            .optional()?)
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
        let text = "Me: Bom dia.\r\n\nThem: Oi, tudo bem?\n  continua aqui\nAna Souza: Às 10:30 então.\nYou: Fechado: https://x.y\n10:30 sem rótulo\nThem:\n";
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
                seg(Mic, Me, 5, "Fechado: https://x.y"),
                seg(Mic, Me, 6, "10:30 sem rótulo"),
            ]
        );
        assert_eq!(
            names,
            BTreeMap::from([(1, "Them".to_string()), (2, "Ana Souza".to_string())])
        );
        // Sem rótulo nenhum, tudo é a "Pessoa 1", sem nome.
        let (segments, names) = transcript_segments("primeira linha\nsegunda");
        assert_eq!(segments.len(), 2);
        assert!(segments.iter().all(|s| s.speaker == Person(1)));
        assert!(names.is_empty());
    }

    #[test]
    fn the_summary_is_marked_as_ai_line_by_line() {
        let at = DateTime::parse_from_rfc3339("2026-10-12T21:00:00-03:00").unwrap();
        let notes = summary_notes("\n### Decisões\n\n- Lançar em novembro  \n", &at).unwrap();
        assert_eq!(
            notes,
            "### Decisões <!-- fala:ia -->\n\n- Lançar em novembro <!-- fala:ia -->\n\n\
             _Importado do Granola em 12/10/2026._"
        );
        assert_eq!(summary_notes(" \n\n", &at), None);
    }
}
