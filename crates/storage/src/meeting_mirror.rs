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
//! Uma sessão importada leva também `import`, a linha de `meeting_imports`, que o `reindex`
//! restaura; as outras não têm a chave. Uma sessão na lixeira leva `trashed_at` e mora em
//! `Reuniões/.lixeira/`, que o Obsidian não mostra (pasta com ponto).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, FixedOffset};
use fala_meeting::{SessionId, SessionMode};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::meeting_imports::{self, ImportLink};
use crate::meetings::{channel_str, mode_str, ms, parse_mode, parse_reason, reason_str, rfc3339};
use crate::mirror::{frontmatter, write_atomic};
use crate::{meeting_search, meeting_trash};
use crate::{
    MeetingRecord, MeetingSegment, ReindexReport, SegmentChannel, SegmentSpeaker, Skipped,
    StorageError, Store,
};

pub(crate) const REUNIOES: &str = "Reuniões";
/// A pasta da lixeira, dentro de `Reuniões/`.
pub(crate) const LIXEIRA: &str = ".lixeira";

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
    /// De onde a sessão foi importada, se foi.
    pub import: Option<ImportLink>,
    /// Quando a sessão foi para a lixeira, se está nela.
    pub trashed_at: Option<DateTime<FixedOffset>>,
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
        SegmentSpeaker::Me | SegmentSpeaker::Omitted => "Eu".to_string(),
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
    if let Some(link) = &doc.import {
        out.push_str(&format!("import: {}\n", link.to_json()));
    }
    if let Some(at) = &doc.trashed_at {
        out.push_str(&format!("trashed_at: {}\n", Value::from(rfc3339(at))));
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
            SegmentSpeaker::Omitted => "omitted".to_string(),
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

/// Lê um `.md` de volta; o `Err` é o motivo legível para o `ReindexReport`.
pub(crate) fn parse(content: &str) -> Result<MeetingDoc, String> {
    let (keys, body) = frontmatter(content)?;
    let get = |key: &str| {
        keys.get(key)
            .ok_or_else(|| format!("falta a chave `{key}`"))
    };
    let text = |key: &str| match get(key)? {
        Value::String(s) => Ok(s.clone()),
        _ => Err(format!("`{key}` não é texto")),
    };
    let optional = |key: &str| match get(key)? {
        Value::Null => Ok(None),
        Value::String(s) => Ok(Some(s.clone())),
        _ => Err(format!("`{key}` não é texto nem null")),
    };
    let flag = |key: &str| {
        get(key)?
            .as_bool()
            .ok_or_else(|| format!("`{key}` não é `true` nem `false`"))
    };
    let time = |key: &str, value: &str| {
        DateTime::parse_from_rfc3339(value).map_err(|e| format!("`{key}` não é RFC 3339: {e}"))
    };
    let optional_time = |key: &str| optional(key)?.map(|v| time(key, &v)).transpose();

    let id: SessionId = text("id")?
        .parse()
        .map_err(|_| "`id` não é id de sessão".to_string())?;
    let mode = text("mode")?;
    let started_at = optional_time("started_at")?;
    let ended_at = optional_time("ended_at")?;
    if ended_at.is_some() && started_at.is_none() {
        return Err("`ended_at` sem `started_at`".to_string());
    }
    let stop_reason = optional("stop_reason")?
        .map(|s| parse_reason(&s).ok_or_else(|| format!("`stop_reason` inválido: {s:?}")))
        .transpose()?;
    let participants: Vec<String> = serde_json::from_value(get("participants")?.clone())
        .map_err(|_| "`participants` não é lista de textos".to_string())?;
    let speakers: BTreeMap<String, String> = serde_json::from_value(get("speakers")?.clone())
        .map_err(|_| "`speakers` não é objeto de textos".to_string())?;
    let mut names = BTreeMap::new();
    for (person, name) in speakers {
        let n: u32 = person
            .parse()
            .ok()
            .filter(|n| *n >= 1)
            .ok_or_else(|| format!("pessoa inválida em `speakers`: {person:?}"))?;
        names.insert(n, name);
    }
    let mut record = MeetingRecord {
        id,
        title: text("title")?,
        mode: parse_mode(&mode).ok_or_else(|| format!("`mode` inválido: {mode:?}"))?,
        local_only: flag("local_only")?,
        created_at: time("created_at", &text("created_at")?)?,
        started_at,
        ended_at,
        recorded_ms: get("recorded_ms")?
            .as_u64()
            .ok_or_else(|| "`recorded_ms` não é inteiro".to_string())?,
        stop_reason,
        audio_retained: flag("audio_retained")?,
        annotations: text("annotations")?,
        transcribed_at: optional_time("transcribed_at")?,
        notes_md: None,
        notes_template: optional("template")?,
    };

    let (head, segments) = match body.rfind(SEGMENTS_OPEN) {
        None => (body, Vec::new()),
        Some(at) => {
            let rows = body[at + SEGMENTS_OPEN.len()..]
                .strip_suffix(SEGMENTS_CLOSE)
                .ok_or_else(|| "bloco `fala:segments` sem o fechamento".to_string())?;
            let before = &body[..at];
            let heading = before
                .rfind(TRANSCRIPT_HEADING)
                .ok_or_else(|| "trechos sem a seção `## Transcrição`".to_string())?;
            let lines = &before[heading + TRANSCRIPT_HEADING.len()..];
            (&before[..heading], parse_segments(rows, lines, &names)?)
        }
    };
    if flag("notes")? {
        let notes = head
            .strip_prefix(header(&record).as_str())
            .and_then(|rest| rest.strip_prefix('\n'))
            .map(str::trim)
            .filter(|md| !md.is_empty())
            .ok_or_else(|| "`notes` verdadeiro sem notas depois do cabeçalho".to_string())?;
        record.notes_md = Some(format!("{notes}\n"));
    }
    let imported = record.mode == SessionMode::Import;
    Ok(MeetingDoc {
        record,
        segments,
        participants,
        names,
        notes_edited_at: optional_time("notes_edited_at")?,
        import: import_link(&keys, imported, id),
        trashed_at: trashed_at(&keys, id),
    })
}

/// `trashed_at` do frontmatter. Inválido, fica de fora e a pasta decide (ver `reindex_meetings`),
/// em vez de a sessão ficar fora do `reindex`.
fn trashed_at(keys: &HashMap<&str, Value>, id: SessionId) -> Option<DateTime<FixedOffset>> {
    let value = keys.get("trashed_at")?;
    let parsed = value
        .as_str()
        .and_then(|at| DateTime::parse_from_rfc3339(at).ok());
    if parsed.is_none() {
        log::warn!("reindex: {id} tem `trashed_at` inválido; a pasta decide se está na lixeira");
    }
    parsed
}

/// A ligação de importação do frontmatter. Só a sessão importada tem a chave; numa gravada ela é
/// ignorada (importar de novo apagaria a transcrição dela), e um valor inválido também: a
/// reunião entra sem a ligação, que é opcional, em vez de ficar fora do `reindex`.
fn import_link(keys: &HashMap<&str, Value>, imported: bool, id: SessionId) -> Option<ImportLink> {
    let value = keys.get("import")?;
    if !imported {
        log::warn!("reindex: {id} não é importada e tem `import`; a chave fica de fora");
        return None;
    }
    ImportLink::from_json(value)
        .map_err(|reason| log::warn!("reindex: {id} entra sem a ligação de importação: {reason}"))
        .ok()
}

/// Junta cada linha do bloco `fala:segments` (`sN canal falante t0 t1`) com a linha da
/// transcrição de mesma ordem, que precisa ter o prefixo e a âncora que o trecho renderiza.
fn parse_segments(
    rows: &str,
    lines: &str,
    names: &BTreeMap<u32, String>,
) -> Result<Vec<MeetingSegment>, String> {
    let lines: Vec<&str> = lines.lines().collect();
    let rows: Vec<&str> = rows.lines().collect();
    if lines.len() != rows.len() {
        return Err(format!(
            "{} linhas na transcrição e {} no bloco `fala:segments`",
            lines.len(),
            rows.len()
        ));
    }
    let mut out = Vec::with_capacity(rows.len());
    for (index, (row, line)) in rows.iter().zip(lines).enumerate() {
        let bad = || format!("trecho {} inválido no bloco `fala:segments`", index + 1);
        let fields: Vec<&str> = row.split(' ').collect();
        let [seq, channel, speaker, t0, t1] = fields[..] else {
            return Err(bad());
        };
        let seq: u32 = seq
            .strip_prefix('s')
            .and_then(|n| n.parse().ok())
            .ok_or_else(bad)?;
        if seq as usize != index + 1 {
            return Err(format!("trechos fora da ordem 1..n em s{seq}"));
        }
        let mut segment = MeetingSegment {
            seq,
            channel: match channel {
                "mic" => SegmentChannel::Mic,
                "system" => SegmentChannel::System,
                _ => return Err(bad()),
            },
            speaker: match speaker {
                "me" => SegmentSpeaker::Me,
                "omitted" => SegmentSpeaker::Omitted,
                n => SegmentSpeaker::Person(n.parse().map_err(|_| bad())?),
            },
            t0_ms: t0.parse().map_err(|_| bad())?,
            t1_ms: t1.parse().map_err(|_| bad())?,
            text: String::new(),
        };
        segment.text = line
            .strip_prefix(line_prefix(&segment, names).as_str())
            .and_then(|rest| rest.strip_suffix(format!(" ^s{seq}").as_str()))
            .ok_or_else(|| format!("a linha de ^s{seq} não confere com o bloco"))?
            .to_string();
        out.push(segment);
    }
    Ok(out)
}

/// Grava uma sessão lida de um `.md`, com o nome do arquivo de onde veio.
fn insert(conn: &Connection, doc: &MeetingDoc, file_name: &str) -> rusqlite::Result<()> {
    let r = &doc.record;
    let id = r.id.to_string();
    conn.execute(
        "INSERT INTO meetings (id, title, mode, local_only, created_at, created_ms, started_at,
             started_ms, ended_at, recorded_ms, stop_reason, audio_retained, annotations,
             transcribed_at, notes_md, notes_template)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            id,
            r.title,
            mode_str(r.mode),
            r.local_only,
            rfc3339(&r.created_at),
            r.created_at.timestamp_millis(),
            r.started_at.as_ref().map(rfc3339),
            r.started_at.map(|t| t.timestamp_millis()),
            r.ended_at.as_ref().map(rfc3339),
            ms(r.recorded_ms),
            r.stop_reason.map(reason_str),
            r.audio_retained,
            r.annotations,
            r.transcribed_at.as_ref().map(rfc3339),
            r.notes_md,
            r.notes_template,
        ],
    )?;
    for s in &doc.segments {
        let speaker = serde_json::to_string(&s.speaker)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
        conn.execute(
            "INSERT INTO meeting_segments (meeting_id, seq, channel, speaker, t0_ms, t1_ms, text)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                s.seq,
                channel_str(s.channel),
                speaker,
                ms(s.t0_ms),
                ms(s.t1_ms),
                s.text
            ],
        )?;
        // The recorder's spans live only in the markers here: a later transcription keeps them.
        if s.speaker == SegmentSpeaker::Omitted {
            conn.execute(
                "INSERT INTO meeting_omissions (meeting_id, t0_ms, t1_ms) VALUES (?1, ?2, ?3)",
                params![id, ms(s.t0_ms), ms(s.t1_ms)],
            )?;
        }
    }
    for (index, name) in doc.participants.iter().enumerate() {
        conn.execute(
            "INSERT INTO meeting_participants (meeting_id, position, name) VALUES (?1, ?2, ?3)",
            params![id, index as i64 + 1, name],
        )?;
    }
    for (person, name) in &doc.names {
        conn.execute(
            "INSERT INTO meeting_speakers (meeting_id, person, name) VALUES (?1, ?2, ?3)",
            params![id, person, name],
        )?;
    }
    if let Some(at) = &doc.notes_edited_at {
        conn.execute(
            "INSERT INTO meeting_note_edits (meeting_id, edited_at) VALUES (?1, ?2)",
            params![id, rfc3339(at)],
        )?;
    }
    if let Some(link) = &doc.import
        && !meeting_imports::restore(conn, r.id, link)?
    {
        log::info!("reindex: {id} entra sem a ligação de importação, que é de outra sessão");
    }
    if let Some(at) = &doc.trashed_at {
        meeting_trash::insert(conn, r.id, at)?;
    }
    conn.execute(
        "INSERT INTO meeting_files (meeting_id, file_name) VALUES (?1, ?2)",
        params![id, file_name],
    )?;
    Ok(())
}

/// Se `path` é um `.md` da sessão `id` (uma cópia que uma mudança de pasta deixou).
fn holds(path: &Path, id: SessionId) -> bool {
    fs::read_to_string(path).is_ok_and(|content| content.contains(&format!("\nid: \"{id}\"\n")))
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

    fn trash_dir(&self) -> PathBuf {
        self.meetings_dir().join(LIXEIRA)
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
            import: meeting_imports::link(&self.conn, id)?,
            trashed_at: meeting_trash::trashed_at(&self.conn, id)?,
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

    /// Reescreve o `.md` da sessão se o conteúdo, o nome ou a pasta mudou; devolve se escreveu.
    /// Entrar na lixeira ou sair dela muda a pasta: o arquivo novo é escrito e o da outra pasta
    /// sai depois, então uma falha no meio deixa duas cópias, e a próxima sincronização apaga a
    /// velha.
    fn write_meeting_mirror(&self, id: SessionId) -> Result<bool, StorageError> {
        let doc = self.meeting_doc(id)?;
        let content = render(&doc);
        let (dir, other) = match doc.trashed_at {
            Some(_) => (self.trash_dir(), self.meetings_dir()),
            None => (self.meetings_dir(), self.trash_dir()),
        };
        let current = self.meeting_file(id)?;
        let stem = file_stem(&doc.record);
        let stale = current
            .as_deref()
            .map(|name| other.join(name))
            .filter(|path| path.is_file());
        // Mudando de pasta, o nome pode estar tomado por um arquivo que não é desta sessão.
        let moving = stale.is_some()
            && current
                .as_deref()
                .is_some_and(|name| !holds(&dir.join(name), id));
        let name = match &current {
            Some(name) if fits(name, &stem) && !moving => name.clone(),
            _ => self.free_file_name(&dir, &stem, id)?,
        };
        let path = dir.join(&name);
        let failed = |source| StorageError::MeetingMirror {
            id: id.to_string(),
            path: path.clone(),
            source,
        };
        let renamed = current.as_deref() != Some(name.as_str());
        if !renamed && stale.is_none() && fs::read_to_string(&path).is_ok_and(|old| old == content)
        {
            return Ok(false);
        }
        write_atomic(&path, &content).map_err(failed)?;
        if renamed {
            self.conn.execute(
                "INSERT INTO meeting_files (meeting_id, file_name) VALUES (?1, ?2)
                 ON CONFLICT (meeting_id) DO UPDATE SET file_name = excluded.file_name",
                params![id.to_string(), name],
            )?;
            // Mudando de pasta, o nome velho na pasta nova é de outro arquivo.
            if let Some(old) = current.filter(|_| !moving) {
                not_found_ok(fs::remove_file(dir.join(old))).map_err(failed)?;
            }
        }
        if let Some(stale) = &stale {
            not_found_ok(fs::remove_file(stale)).map_err(failed)?;
        }
        Ok(true)
    }

    /// Depois de uma mudança já gravada: a linha de busca e o `.md` da sessão. Uma falha vai
    /// para o log, sem o caminho (que tem o título); a próxima abertura do desktop refaz os dois.
    pub(crate) fn after_meeting_change(&self, id: SessionId) {
        if let Err(e) = meeting_search::refresh(&self.conn, id) {
            log::warn!("busca da reunião {id}: {e}");
        }
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

    /// Apaga o `.md` de uma sessão que está saindo do banco (dentro da transação que a apaga),
    /// em `Reuniões/` ou na lixeira.
    pub(crate) fn remove_meeting_mirror(
        &self,
        id: SessionId,
        file: Option<String>,
    ) -> Result<(), StorageError> {
        let Some(name) = file else {
            return Ok(());
        };
        for dir in [self.meetings_dir(), self.trash_dir()] {
            let path = dir.join(&name);
            not_found_ok(fs::remove_file(&path)).map_err(|source| StorageError::MeetingMirror {
                id: id.to_string(),
                path,
                source,
            })?;
        }
        Ok(())
    }

    /// Acrescenta ao banco as reuniões de `<notes_dir>/Reuniões/*.md` e da lixeira
    /// (`Reuniões/.lixeira/*.md`) que faltam nele, numa única transação; devolve quantas entraram.
    ///
    /// Um `.md` da lixeira sem `trashed_at` entra nela com agora: o prazo recomeça, nunca encurta.
    ///
    /// Uma sessão que o banco já tem não muda: ele é a fonte de verdade e guarda o que o `.md`
    /// não guarda (sugestões de nome pendentes, tempos em ms). Com id repetido vale o arquivo
    /// cujo nome bate com o título. Um `.md` inválido, repetido ou que o banco recusa entra no
    /// relatório; os outros entram.
    pub fn reindex_meetings(&mut self) -> Result<ReindexReport, StorageError> {
        let now = chrono::Local::now().fixed_offset();
        let mut files = Vec::new();
        for (dir, trashed) in [(self.meetings_dir(), false), (self.trash_dir(), true)] {
            if dir.is_dir() {
                for entry in fs::read_dir(&dir)? {
                    let path = entry?.path();
                    if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                        files.push((path, trashed));
                    }
                }
            }
        }
        files.sort();

        let mut report = ReindexReport::default();
        let mut docs = Vec::new();
        for (path, trashed) in files {
            let parsed = fs::read_to_string(&path)
                .map_err(|e| format!("não deu para ler: {e}"))
                .and_then(|content| parse(&content));
            match parsed {
                Ok(mut doc) => {
                    if trashed && doc.trashed_at.is_none() {
                        doc.trashed_at = Some(now);
                    }
                    docs.push((file_name(&path), path, doc));
                }
                Err(reason) => report.skipped.push(Skipped { path, reason }),
            }
        }
        // Os arquivos com o nome do título primeiro; `sort_by_key` é estável.
        docs.sort_by_key(|(name, _, doc)| !fits(name, &file_stem(&doc.record)));
        let known: HashSet<SessionId> = self.meetings()?.into_iter().map(|m| m.id).collect();
        let mut seen = HashSet::new();

        let mut tx = self.conn.transaction()?;
        for (name, path, doc) in docs {
            if !seen.insert(doc.record.id) {
                report.skipped.push(Skipped {
                    path,
                    reason: "id duplicado".to_string(),
                });
                continue;
            }
            if known.contains(&doc.record.id) {
                continue;
            }
            let sp = tx.savepoint()?;
            match insert(&sp, &doc, &name)
                .and_then(|()| meeting_search::refresh(&sp, doc.record.id))
            {
                Ok(()) => {
                    sp.commit()?;
                    report.indexed += 1;
                }
                // O savepoint desfaz a sessão inteira ao sair de escopo.
                Err(e) => report.skipped.push(Skipped {
                    path,
                    reason: format!("o banco recusou: {e}"),
                }),
            }
        }
        tx.commit()?;
        log::info!(
            "reindex: {} reuniões, {} arquivos ignorados",
            report.indexed,
            report.skipped.len()
        );
        Ok(report)
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fala_meeting::StopReason;

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

    fn segment(seq: u32, speaker: SegmentSpeaker, t0_ms: u64, text: &str) -> MeetingSegment {
        MeetingSegment {
            seq,
            channel: match speaker {
                SegmentSpeaker::Me | SegmentSpeaker::Omitted => SegmentChannel::Mic,
                SegmentSpeaker::Person(_) => SegmentChannel::System,
            },
            speaker,
            t0_ms,
            t1_ms: t0_ms + 4_200,
            text: text.to_string(),
        }
    }

    fn doc() -> MeetingDoc {
        let mut record = record("Planejamento");
        record.notes_md = Some(
            "## Anotações\n\ndecidir data ^a1\n\n## Notas · Reunião geral\n\n### Resumo\n\n\
             - Lançamento em 15/11. [[#^s2|01:05]] <!-- fala:ia -->\n"
                .to_string(),
        );
        MeetingDoc {
            record,
            segments: vec![
                segment(1, SegmentSpeaker::Me, 0, "Vamos falar da data."),
                segment(
                    2,
                    SegmentSpeaker::Person(1),
                    65_000,
                    "Proponho 15 de novembro.",
                ),
                segment(3, SegmentSpeaker::Person(2), 3_725_000, "Fechado."),
            ],
            participants: vec!["Ana".to_string(), "Bruno".to_string()],
            names: BTreeMap::from([(1, "Ana".to_string())]),
            notes_edited_at: Some(at("2026-10-12T15:00:00-03:00")),
            import: None,
            trashed_at: None,
        }
    }

    #[test]
    fn renders_the_documented_file() {
        let md = render(&doc());
        assert!(md.starts_with("---\nid: \""), "{md}");
        assert!(md.contains("\ndate: \"2026-10-12\"\n"), "{md}");
        assert!(md.contains("\nspeakers: {\"1\":\"Ana\"}\n"), "{md}");
        assert!(md.contains("\nparticipants: [\"Ana\",\"Bruno\"]\n"), "{md}");
        assert!(
            md.contains("---\n# Planejamento\n\n2026-10-12 14:02\n\n## Anotações\n"),
            "{md}"
        );
        assert!(md.contains(
            "\n## Transcrição\n\n- **[00:00] Eu:** Vamos falar da data. ^s1\n\
             - **[01:05] Ana:** Proponho 15 de novembro. ^s2\n\
             - **[1:02:05] Pessoa 2:** Fechado. ^s3\n\n<!-- fala:segments\n\
             s1 mic me 0 4200\ns2 system 1 65000 69200\ns3 system 2 3725000 3729200\n-->\n"
        ));
        assert!(md.ends_with("-->\n"), "{md}");
        assert!(!md.contains("audio/"), "{md}");
    }

    #[test]
    fn round_trips_md_to_doc_to_md() {
        let original = doc();
        let md = render(&original);
        let parsed = parse(&md).unwrap();
        assert_eq!(render(&parsed), md);
        assert_eq!(parsed.segments, original.segments);
        assert_eq!(parsed.names, original.names);
        assert_eq!(parsed.participants, original.participants);
        assert_eq!(parsed.record.annotations, original.record.annotations);
        assert_eq!(parsed.notes_edited_at, original.notes_edited_at);
        assert_eq!(
            parsed.record.notes_md.as_deref().map(str::trim),
            original.record.notes_md.as_deref().map(str::trim)
        );
    }

    #[test]
    fn the_import_link_round_trips_and_only_imports_have_it() {
        assert!(!render(&doc()).contains("\nimport:"));
        let mut d = doc();
        d.record.mode = SessionMode::Import;
        d.import = Some(ImportLink {
            source: "granola".to_string(),
            external_id: "g-1".to_string(),
            imported_at: "2026-10-12T21:00:00-03:00".to_string(),
            title: "Título do Granola".to_string(),
            annotations: "pauta: \"preço\"\n---\nfim".to_string(),
        });
        let md = render(&d);
        let line = "import: {\"annotations\":\"pauta: \\\"preço\\\"\\n---\\nfim\",\
                    \"external_id\":\"g-1\",\"imported_at\":\"2026-10-12T21:00:00-03:00\",\
                    \"source\":\"granola\",\"title\":\"Título do Granola\"}";
        assert!(md.contains(&format!("\n{line}\n---\n")), "{md}");
        let parsed = parse(&md).unwrap();
        assert_eq!(parsed.import, d.import);
        assert_eq!(render(&parsed), md);

        // Numa reunião gravada a chave fica de fora: importar de novo apagaria a transcrição.
        let recorded = md.replace("\nmode: \"import\"", "\nmode: \"meeting\"");
        assert_eq!(parse(&recorded).unwrap().import, None);

        // Inválida, a ligação fica de fora e a reunião entra assim mesmo.
        for (bad, reason) in [
            ("{\"source\":\"granola\"}", "não é a ligação"),
            (
                "{\"source\":\"\",\"external_id\":\"g\",\"imported_at\":\
                 \"2026-10-12T21:00:00-03:00\",\"title\":\"\",\"annotations\":\"\"}",
                "sem `source`",
            ),
            (
                "{\"source\":\"granola\",\"external_id\":\"g\",\"imported_at\":\
                 \"ontem\",\"title\":\"\",\"annotations\":\"\"}",
                "não é RFC 3339",
            ),
        ] {
            let value: Value = serde_json::from_str(bad).unwrap();
            let err = ImportLink::from_json(&value).unwrap_err();
            assert!(err.contains(reason), "{reason}: {err}");
            let parsed = parse(&md.replace(line, &format!("import: {bad}"))).unwrap();
            assert_eq!(parsed.import, None, "{reason}");
            assert_eq!(parsed.segments, d.segments, "{reason}");
        }
    }

    #[test]
    fn draft_without_notes_or_segments_round_trips() {
        let mut d = doc();
        d.record.started_at = None;
        d.record.ended_at = None;
        d.record.stop_reason = None;
        d.record.transcribed_at = None;
        d.record.notes_md = Some("  \n".to_string());
        d.record.title = String::new();
        d.segments.clear();
        d.names.clear();
        d.notes_edited_at = None;
        let md = render(&d);
        assert!(md.contains("\nnotes: false\n"), "{md}");
        assert!(md.contains("---\n# Reunião\n\n2026-10-12 14:00\n\n## Anotações\n\ndecidir data ^a1\n\nAna: contrato ^a2\n"), "{md}");
        assert!(!md.contains("Transcrição"), "{md}");
        let parsed = parse(&md).unwrap();
        assert_eq!(parsed.record.notes_md, None);
        assert_eq!(render(&parsed), md);
    }

    #[test]
    fn notes_mentioning_the_transcript_heading_still_parse() {
        let mut d = doc();
        d.record.notes_md = Some("## Transcrição\n\n- citação\n".to_string());
        let md = render(&d);
        let parsed = parse(&md).unwrap();
        assert_eq!(
            parsed.record.notes_md.as_deref(),
            Some("## Transcrição\n\n- citação\n")
        );
        assert_eq!(parsed.segments.len(), 3);
    }

    #[test]
    fn rejects_inconsistent_files() {
        let md = render(&doc());
        let cases = [
            (md.replace("^s2\n", "^s9\n"), "não confere"),
            (
                md.replace("s3 system 2 3725000 3729200\n", ""),
                "linhas na transcrição",
            ),
            (
                md.replace("\nmode: \"meeting\"", "\nmode: \"podcast\""),
                "`mode` inválido",
            ),
            (md.replace("\nnotes: true", ""), "falta a chave `notes`"),
            (
                md.replace(
                    "started_at: \"2026-10-12T14:02:00-03:00\"",
                    "started_at: null",
                ),
                "sem `started_at`",
            ),
            (
                md.replace("speakers: {\"1\"", "speakers: {\"0\""),
                "pessoa inválida",
            ),
        ];
        for (content, reason) in cases {
            let err = parse(&content).unwrap_err();
            assert!(err.contains(reason), "{reason}: {err}");
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
