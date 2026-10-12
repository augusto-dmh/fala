//! O espelho Markdown de uma reunião: `Reuniões/<AAAA-MM-DD> <Título>.md`.
//!
//! Frontmatter como o dos ditados (`chave: <valor JSON>` por linha). O corpo é o documento do
//! "Copiar Markdown" (`fala_notes::note_document`, rótulos pt-BR): título, data e hora, as notas
//! (ou, sem elas, o bloco de anotações) e a transcrição com as âncoras `^sN` que os ponteiros das
//! notas citam. Depois dela, um comentário HTML (escondido no Obsidian) guarda canal, falante e
//! tempos de cada trecho, para o `reindex` voltar sem perda. Nunca um caminho de disco
//! (ADR-0011): o caminho revela o usuário e as pastas.
//!
//! O nome em uso fica em `meeting_files` (tabela aditiva), para renomear quando o título muda.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, FixedOffset};
use fala_meeting::SessionId;
use rusqlite::{params, OptionalExtension};
use serde_json::Value;

use crate::meetings::{channel_str, mode_str, reason_str, rfc3339};
use crate::mirror::write_atomic;
use crate::{MeetingRecord, MeetingSegment, SegmentSpeaker, StorageError, Store};

pub(crate) const REUNIOES: &str = "Reuniões";

pub(crate) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meeting_files (
    meeting_id TEXT PRIMARY KEY REFERENCES meetings(id) ON DELETE CASCADE,
    file_name TEXT NOT NULL UNIQUE COLLATE NOCASE
);
";

const UNTITLED: &str = "Reunião";
const TRANSCRIPT_HEADING: &str = "\n## Transcrição\n\n";
const SEGMENTS_OPEN: &str = "\n<!-- fala:segments\n";
const SEGMENTS_CLOSE: &str = "-->\n";
/// O marcador das linhas geradas (`fala_notes::GENERATED_MARKER`); sai do texto dos trechos.
pub(crate) const GENERATED_MARKER: &str = "<!-- fala:ia -->";
/// Caracteres que o Windows não aceita num nome de arquivo ou que quebram um link do Obsidian.
const FORBIDDEN: &str = "<>:\"/\\|?*#^[]";
/// Teto do título no nome do arquivo, em caracteres.
const STEM_TITLE_MAX: usize = 80;

/// Tudo o que o `.md` de uma sessão guarda.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MeetingDoc {
    pub record: MeetingRecord,
    pub segments: Vec<MeetingSegment>,
    pub participants: Vec<String>,
    /// O nome aplicado a cada "Pessoa N".
    pub names: BTreeMap<u32, String>,
    pub notes_edited_at: Option<DateTime<FixedOffset>>,
}

/// As notas que entram no corpo: aparadas, e só se não estão em branco.
fn notes_text(record: &MeetingRecord) -> Option<&str> {
    record
        .notes_md
        .as_deref()
        .map(str::trim)
        .filter(|md| !md.is_empty())
}

fn start(record: &MeetingRecord) -> DateTime<FixedOffset> {
    record.started_at.unwrap_or(record.created_at)
}

/// `<AAAA-MM-DD> <Título>`, sem extensão: a data local do início (ou da criação, no rascunho) e o
/// título sem os caracteres proibidos, numa linha e cortado.
pub(crate) fn file_stem(record: &MeetingRecord) -> String {
    let cleaned: String = record
        .title
        .chars()
        .map(|c| {
            if c.is_control() || FORBIDDEN.contains(c) {
                ' '
            } else {
                c
            }
        })
        .collect();
    let title: String = cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(STEM_TITLE_MAX)
        .collect();
    let title = title.trim_end_matches(['.', ' ']);
    let title = if title.is_empty() { UNTITLED } else { title };
    format!("{} {title}", start(record).format("%Y-%m-%d"))
}

/// Se `name` é `<stem>.md` ou `<stem> (N).md`.
/// Sem diferença de caixa: no Windows, `plano.md` e `Plano.md` são o mesmo arquivo.
fn fits(name: &str, stem: &str) -> bool {
    let (name, stem) = (name.to_lowercase(), stem.to_lowercase());
    let Some(rest) = name.strip_prefix(stem.as_str()) else {
        return false;
    };
    rest == ".md"
        || rest
            .strip_prefix(" (")
            .and_then(|r| r.strip_suffix(").md"))
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

/// `mm:ss`, ou `h:mm:ss` a partir de uma hora, como na transcrição das notas.
fn timestamp(ms: u64) -> String {
    let seconds = ms / 1_000;
    let (hours, minutes, seconds) = (seconds / 3_600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

/// O texto de um trecho numa linha só, sem o marcador das linhas geradas.
fn one_line(text: &str) -> String {
    text.replace(GENERATED_MARKER, " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Numa linha só: um nome com quebra não parte a linha do trecho.
fn label(speaker: SegmentSpeaker, names: &BTreeMap<u32, String>) -> String {
    match speaker {
        SegmentSpeaker::Me => "Eu".to_string(),
        SegmentSpeaker::Person(n) => names
            .get(&n)
            .map(|name| one_line(name))
            .unwrap_or_else(|| format!("Pessoa {n}")),
    }
}

fn line_prefix(segment: &MeetingSegment, names: &BTreeMap<u32, String>) -> String {
    format!(
        "- **[{}] {}:** ",
        timestamp(segment.t0_ms),
        label(segment.speaker, names)
    )
}

fn header(record: &MeetingRecord) -> String {
    let title = match record.title.trim() {
        "" => UNTITLED,
        title => title,
    };
    let at = start(record);
    format!(
        "# {title}\n\n{} {}\n",
        at.format("%Y-%m-%d"),
        at.format("%H:%M")
    )
}

/// O bloco de anotações das notas: cada linha não vazia, aparada, com a âncora `^aN`.
fn annotations_block(annotations: &str) -> String {
    let lines: Vec<String> = annotations
        .lines()
        .map(|line| line.replace(GENERATED_MARKER, " ").trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        return String::new();
    }
    let mut out = String::from("## Anotações\n");
    for (index, line) in lines.iter().enumerate() {
        out.push_str(&format!("\n{line} ^a{}\n", index + 1));
    }
    out
}

/// O conteúdo do `.md` de uma sessão.
pub(crate) fn render(doc: &MeetingDoc) -> String {
    let r = &doc.record;
    let time = |at: Option<&DateTime<FixedOffset>>| at.map_or(Value::Null, |t| rfc3339(t).into());
    let speakers: serde_json::Map<String, Value> = doc
        .names
        .iter()
        .map(|(person, name)| (person.to_string(), Value::from(name.as_str())))
        .collect();
    let notes = notes_text(r);
    let lines: [(&str, Value); 18] = [
        ("id", r.id.to_string().into()),
        ("title", r.title.as_str().into()),
        ("date", start(r).format("%Y-%m-%d").to_string().into()),
        ("mode", mode_str(r.mode).into()),
        ("local_only", r.local_only.into()),
        ("created_at", rfc3339(&r.created_at).into()),
        ("started_at", time(r.started_at.as_ref())),
        ("ended_at", time(r.ended_at.as_ref())),
        ("recorded_ms", r.recorded_ms.into()),
        (
            "stop_reason",
            r.stop_reason.map_or(Value::Null, |s| reason_str(s).into()),
        ),
        ("audio_retained", r.audio_retained.into()),
        ("transcribed_at", time(r.transcribed_at.as_ref())),
        (
            "template",
            r.notes_template.as_deref().map_or(Value::Null, Value::from),
        ),
        ("notes", notes.is_some().into()),
        ("notes_edited_at", time(doc.notes_edited_at.as_ref())),
        ("participants", doc.participants.clone().into()),
        ("speakers", Value::Object(speakers)),
        ("annotations", r.annotations.as_str().into()),
    ];
    let mut out = String::from("---\n");
    for (key, value) in lines {
        out.push_str(&format!("{key}: {value}\n"));
    }
    out.push_str("---\n");

    out.push_str(&header(r));
    let block = match notes {
        Some(md) => format!("{md}\n"),
        None => annotations_block(&r.annotations),
    };
    if !block.is_empty() {
        out.push('\n');
        out.push_str(&block);
    }
    if doc.segments.is_empty() {
        return out;
    }
    out.push_str(TRANSCRIPT_HEADING);
    for s in &doc.segments {
        out.push_str(&format!(
            "{}{} ^s{}\n",
            line_prefix(s, &doc.names),
            one_line(&s.text),
            s.seq
        ));
    }
    out.push_str(SEGMENTS_OPEN);
    for s in &doc.segments {
        let speaker = match s.speaker {
            SegmentSpeaker::Me => "me".to_string(),
            SegmentSpeaker::Person(n) => n.to_string(),
        };
        out.push_str(&format!(
            "s{} {} {speaker} {} {}\n",
            s.seq,
            channel_str(s.channel),
            s.t0_ms,
            s.t1_ms
        ));
    }
    out.push_str(SEGMENTS_CLOSE);
    out
}

fn not_found_ok(result: std::io::Result<()>) -> std::io::Result<()> {
    match result {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

impl Store {
    fn meetings_dir(&self) -> PathBuf {
        self.notes_dir.join(REUNIOES)
    }

    /// O nome do `.md` da sessão, se já foi escrito.
    pub(crate) fn meeting_file(&self, id: SessionId) -> Result<Option<String>, StorageError> {
        Ok(self
            .conn
            .query_row(
                "SELECT file_name FROM meeting_files WHERE meeting_id = ?1",
                [id.to_string()],
                |row| row.get(0),
            )
            .optional()?)
    }

    fn meeting_doc(&self, id: SessionId) -> Result<MeetingDoc, StorageError> {
        Ok(MeetingDoc {
            record: self.meeting(id)?,
            segments: self.meeting_segments(id)?,
            participants: self.meeting_participants(id)?,
            names: self
                .meeting_speakers(id)?
                .into_iter()
                .filter_map(|s| Some((s.person, s.name?)))
                .collect(),
            notes_edited_at: self.meeting_notes_edited_at(id)?,
        })
    }

    /// O primeiro `<stem>.md`, `<stem> (2).md`, ... que nenhuma outra sessão usa e que não existe.
    fn free_file_name(
        &self,
        dir: &Path,
        stem: &str,
        id: SessionId,
    ) -> Result<String, StorageError> {
        for n in 1u32.. {
            let name = match n {
                1 => format!("{stem}.md"),
                n => format!("{stem} ({n}).md"),
            };
            let taken: bool = self.conn.query_row(
                "SELECT EXISTS (SELECT 1 FROM meeting_files WHERE file_name = ?1 AND meeting_id <> ?2)",
                params![name, id.to_string()],
                |row| row.get(0),
            )?;
            if !taken && !dir.join(&name).exists() {
                return Ok(name);
            }
        }
        Err(StorageError::NotFound(format!("nome livre para {id}")))
    }

    /// Reescreve o `.md` da sessão se o conteúdo ou o nome mudou; devolve se escreveu.
    fn write_meeting_mirror(&self, id: SessionId) -> Result<bool, StorageError> {
        let doc = self.meeting_doc(id)?;
        let content = render(&doc);
        let dir = self.meetings_dir();
        let current = self.meeting_file(id)?;
        let stem = file_stem(&doc.record);
        let name = match &current {
            Some(name) if fits(name, &stem) => name.clone(),
            _ => self.free_file_name(&dir, &stem, id)?,
        };
        let path = dir.join(&name);
        let failed = |source| StorageError::MeetingMirror {
            id: id.to_string(),
            path: path.clone(),
            source,
        };
        let renamed = current.as_deref() != Some(name.as_str());
        if !renamed && fs::read_to_string(&path).is_ok_and(|old| old == content) {
            return Ok(false);
        }
        write_atomic(&path, &content).map_err(failed)?;
        if renamed {
            self.conn.execute(
                "INSERT INTO meeting_files (meeting_id, file_name) VALUES (?1, ?2)
                 ON CONFLICT (meeting_id) DO UPDATE SET file_name = excluded.file_name",
                params![id.to_string(), name],
            )?;
            if let Some(old) = current {
                not_found_ok(fs::remove_file(dir.join(old))).map_err(failed)?;
            }
        }
        Ok(true)
    }

    /// Depois de uma mudança já gravada: a falha do espelho vai para o log, sem o caminho (que
    /// tem o título), e a próxima [`Store::sync_meeting_mirrors`] tenta de novo.
    pub(crate) fn mirror_meeting(&self, id: SessionId) {
        if let Err(e) = self.write_meeting_mirror(id) {
            log::warn!("{e}");
        }
    }

    /// Reescreve o `.md` de cada sessão que falta ou diverge do banco; devolve quantos escreveu.
    /// Uma sessão que falha vai para o log e não para as outras.
    pub fn sync_meeting_mirrors(&self) -> Result<usize, StorageError> {
        let mut written = 0;
        for meeting in self.meetings()? {
            if self.sync_meeting_mirror(meeting.id) {
                written += 1;
            }
        }
        Ok(written)
    }

    /// [`Store::sync_meeting_mirrors`] de uma sessão só, para quem segura o `Store` por sessão;
    /// devolve se escreveu.
    pub fn sync_meeting_mirror(&self, id: SessionId) -> bool {
        self.write_meeting_mirror(id).unwrap_or_else(|e| {
            log::warn!("{e}");
            false
        })
    }

    /// Apaga o `.md` de uma sessão que está saindo do banco (dentro da transação que a apaga).
    pub(crate) fn remove_meeting_mirror(
        &self,
        id: SessionId,
        file: Option<String>,
    ) -> Result<(), StorageError> {
        let Some(name) = file else {
            return Ok(());
        };
        let path = self.meetings_dir().join(name);
        not_found_ok(fs::remove_file(&path)).map_err(|source| StorageError::MeetingMirror {
            id: id.to_string(),
            path,
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fala_meeting::{SessionMode, StopReason};

    fn at(s: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(s).unwrap()
    }

    fn record(title: &str) -> MeetingRecord {
        MeetingRecord {
            id: SessionId::from_parts(1_760_288_400_000, 42),
            title: title.to_string(),
            mode: SessionMode::Meeting,
            local_only: false,
            created_at: at("2026-10-12T14:00:00-03:00"),
            started_at: Some(at("2026-10-12T14:02:00-03:00")),
            ended_at: Some(at("2026-10-12T14:40:00-03:00")),
            recorded_ms: 2_100_000,
            stop_reason: Some(StopReason::User),
            audio_retained: true,
            annotations: "decidir data\n\n  Ana: contrato  \n".to_string(),
            transcribed_at: Some(at("2026-10-12T14:41:00-03:00")),
            notes_md: None,
            notes_template: Some("geral".to_string()),
        }
    }

    #[test]
    fn file_stem_is_safe_for_windows_and_obsidian() {
        let r = record("  Plano: Q4 / 2026? [rascunho] #ops.  ");
        assert_eq!(file_stem(&r), "2026-10-12 Plano Q4 2026 rascunho ops");
        assert_eq!(file_stem(&record("\t")), "2026-10-12 Reunião");
        let mut draft = record("Pauta");
        draft.started_at = None;
        draft.created_at = at("2026-10-11T23:59:00-03:00");
        assert_eq!(file_stem(&draft), "2026-10-11 Pauta");
        assert_eq!(
            file_stem(&record(&"a".repeat(200))).chars().count(),
            "2026-10-12 ".len() + STEM_TITLE_MAX
        );
        // O nome numerado e o de outra caixa ainda são da sessão; outro título, não.
        assert!(fits("2026-10-12 Plano.md", "2026-10-12 Plano"));
        assert!(fits("2026-10-12 Plano (3).md", "2026-10-12 Plano"));
        assert!(!fits("2026-10-12 Plano B.md", "2026-10-12 Plano"));
        assert!(!fits("2026-10-12 Plano ().md", "2026-10-12 Plano"));
        assert!(fits("2026-10-12 plano.md", "2026-10-12 Plano"));
    }
}
