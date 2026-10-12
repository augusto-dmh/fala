//! O "Historical Data Export" do Granola (Settings › Profile › "Export historical data"): um CSV,
//! ou um `.zip` de CSVs com o mesmo cabeçalho, com uma reunião por linha.
//!
//! Colunas reconhecidas, como no parser do Wispr que lê o mesmo arquivo: `document_id`,
//! `document_title`, `workspace_name`, `document_created`, `summary`, `notes` e `transcript`;
//! só `document_id` e `document_created` são obrigatórias. O cabeçalho aceita BOM e qualquer
//! caixa. Uma linha ruim não derruba o arquivo: vai para [`GranolaExport::rejected`] com o número
//! da linha e o motivo. Um id repetido vale pela última linha.
//!
//! Nada aqui abre rede: o arquivo é local e o conteúdo não sai do computador (ADR-0008).

use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::Path;

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, Offset, TimeZone, Utc};

/// Teto do arquivo (e de cada CSV dentro do `.zip`), em bytes.
pub const MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;
/// Teto de linhas de dados numa importação, somados os CSVs do `.zip`.
pub const MAX_ROWS: usize = 5_000;
const MAX_SUMMARY_BYTES: usize = 256 * 1024;
const MAX_NOTES_BYTES: usize = 2 * 1024 * 1024;
const MAX_TRANSCRIPT_BYTES: usize = 8 * 1024 * 1024;
/// Caracteres do começo do texto que decidem se ele é UTF-8.
const SNIFF_CHARS: usize = 2_000;

const REQUIRED: [&str; 2] = ["document_id", "document_created"];

/// Uma reunião do export, já validada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GranolaMeeting {
    /// A linha da planilha em que ela está (o cabeçalho é a linha 1).
    pub line: usize,
    pub document_id: String,
    /// Aparado; vazio quando o Granola não tinha título.
    pub title: String,
    pub workspace: String,
    pub created_at: DateTime<FixedOffset>,
    /// O resumo que a IA do Granola escreveu, em Markdown.
    pub summary: String,
    /// O que a pessoa digitou no Granola.
    pub notes: String,
    pub transcript: String,
}

/// Por que uma linha ficou de fora.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectReason {
    /// `document_id` em branco.
    MissingId,
    /// `document_created` que não é uma data reconhecida; leva o texto da célula.
    InvalidDate(String),
    /// Uma célula passou do teto da coluna (`summary`, `notes` ou `transcript`).
    CellTooLarge(&'static str),
}

/// Uma linha ignorada: onde e por quê.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedRow {
    /// O CSV dentro do `.zip`; `None` num CSV avulso.
    pub file: Option<String>,
    pub line: usize,
    pub reason: RejectReason,
}

/// O export lido: as reuniões, uma por `document_id`, na ordem do arquivo, e as linhas ignoradas.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GranolaExport {
    pub meetings: Vec<GranolaMeeting>,
    pub rejected: Vec<RejectedRow>,
    /// Linhas trocadas por uma linha posterior com o mesmo `document_id`.
    pub duplicates: usize,
}

impl GranolaExport {
    /// A primeira e a última data das reuniões, se houver alguma.
    pub fn span(&self) -> Option<(DateTime<FixedOffset>, DateTime<FixedOffset>)> {
        let first = self.meetings.iter().map(|m| m.created_at).min()?;
        let last = self.meetings.iter().map(|m| m.created_at).max()?;
        Some((first, last))
    }
}

/// Por que o arquivo inteiro foi recusado.
#[derive(Debug, thiserror::Error)]
pub enum GranolaError {
    #[error("o arquivo está vazio")]
    Empty,
    #[error("o arquivo não é texto UTF-8")]
    NotUtf8,
    #[error("o arquivo passa de {} MB", MAX_FILE_BYTES / 1024 / 1024)]
    FileTooLarge,
    #[error("o arquivo passa de {MAX_ROWS} linhas")]
    TooManyRows,
    #[error("faltam as colunas {}", .0.join(", "))]
    MissingColumns(Vec<String>),
    #[error("o .zip não tem nenhum CSV")]
    NoCsvInZip,
    #[error("os CSVs do .zip não têm as mesmas colunas")]
    ZipMismatch,
    #[error("o .zip não abriu: {0}")]
    Zip(String),
    #[error("o CSV não abriu: {0}")]
    Csv(String),
    #[error("E/S: {0}")]
    Io(#[from] std::io::Error),
}

/// Lê o export em `path`: um CSV ou um `.zip` (reconhecido pelo conteúdo, não pela extensão).
pub fn read_granola_export(path: &Path) -> Result<GranolaExport, GranolaError> {
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() > MAX_FILE_BYTES {
        return Err(GranolaError::FileTooLarge);
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(GranolaError::FileTooLarge);
    }
    if bytes.starts_with(b"PK\x03\x04") {
        parse_granola_zip(&bytes)
    } else {
        parse_granola_csv(&bytes)
    }
}

/// Lê um CSV do export.
pub fn parse_granola_csv(bytes: &[u8]) -> Result<GranolaExport, GranolaError> {
    Ok(merge(vec![parse_table(bytes, None)?]))
}

/// Lê um `.zip` do export: todos os `.csv` dentro dele, que precisam ter o mesmo cabeçalho.
pub fn parse_granola_zip(bytes: &[u8]) -> Result<GranolaExport, GranolaError> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| GranolaError::Zip(e.to_string()))?;
    let mut tables: Vec<Table> = Vec::new();
    let mut rows = 0;
    // O teto vale para o descomprimido somado: o tamanho que o `.zip` declara não é prova.
    let mut bytes_read: u64 = 0;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|e| GranolaError::Zip(e.to_string()))?;
        let name = entry.name().to_string();
        // O Finder põe uma cópia de metadados de cada arquivo em `__MACOSX/`.
        if !entry.is_file()
            || name.starts_with("__MACOSX/")
            || !name.to_lowercase().ends_with(".csv")
        {
            continue;
        }
        if entry.size() > MAX_FILE_BYTES {
            return Err(GranolaError::FileTooLarge);
        }
        let mut csv = Vec::new();
        entry
            .take(MAX_FILE_BYTES + 1 - bytes_read)
            .read_to_end(&mut csv)?;
        bytes_read += csv.len() as u64;
        if bytes_read > MAX_FILE_BYTES {
            return Err(GranolaError::FileTooLarge);
        }
        let table = parse_table(&csv, Some(&name))?;
        if tables
            .first()
            .is_some_and(|first| first.header != table.header)
        {
            return Err(GranolaError::ZipMismatch);
        }
        rows += table.rows;
        if rows > MAX_ROWS {
            return Err(GranolaError::TooManyRows);
        }
        tables.push(table);
    }
    if tables.is_empty() {
        return Err(GranolaError::NoCsvInZip);
    }
    Ok(merge(tables))
}

/// Um CSV lido, antes de juntar os ids repetidos.
struct Table {
    header: Vec<String>,
    rows: usize,
    meetings: Vec<GranolaMeeting>,
    rejected: Vec<RejectedRow>,
}

/// Os CSVs na ordem; um id repetido fica na posição e com o conteúdo da última linha.
fn merge(tables: Vec<Table>) -> GranolaExport {
    let mut export = GranolaExport::default();
    let mut slot: HashMap<String, usize> = HashMap::new();
    for table in tables {
        export.rejected.extend(table.rejected);
        for meeting in table.meetings {
            if let Some(&at) = slot.get(&meeting.document_id) {
                export.meetings[at].document_id.clear();
                export.duplicates += 1;
            }
            slot.insert(meeting.document_id.clone(), export.meetings.len());
            export.meetings.push(meeting);
        }
    }
    export.meetings.retain(|m| !m.document_id.is_empty());
    export
}

/// O texto do arquivo, sem BOM. Como o Wispr: lido com perda, e recusado só se mais de 5 % dos
/// primeiros caracteres não forem texto (`U+FFFD` ou NUL), o que um CSV em Latin-1 ou UTF-16 dá.
fn decode(bytes: &[u8]) -> Result<String, GranolaError> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let text = String::from_utf8_lossy(bytes);
    if text.trim().is_empty() {
        return Err(GranolaError::Empty);
    }
    let (mut seen, mut bad) = (0usize, 0usize);
    for c in text.chars().take(SNIFF_CHARS) {
        seen += 1;
        if c == '\u{FFFD}' || c == '\0' {
            bad += 1;
        }
    }
    if bad * 20 > seen {
        return Err(GranolaError::NotUtf8);
    }
    Ok(text.into_owned())
}

fn parse_table(bytes: &[u8], file: Option<&str>) -> Result<Table, GranolaError> {
    let text = decode(bytes)?;
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(text.as_bytes());
    let header: Vec<String> = reader
        .headers()
        .map_err(|e| GranolaError::Csv(e.to_string()))?
        .iter()
        .map(|h| h.trim().to_lowercase())
        .collect();
    let missing: Vec<String> = REQUIRED
        .iter()
        .filter(|name| !header.iter().any(|h| h == *name))
        .map(|name| name.to_string())
        .collect();
    if !missing.is_empty() {
        return Err(GranolaError::MissingColumns(missing));
    }
    let column = |name: &str| header.iter().position(|h| h == name);
    let at = Columns {
        id: column("document_id"),
        title: column("document_title"),
        workspace: column("workspace_name"),
        created: column("document_created"),
        summary: column("summary"),
        notes: column("notes"),
        transcript: column("transcript"),
    };
    let mut table = Table {
        header: header.clone(),
        rows: 0,
        meetings: Vec::new(),
        rejected: Vec::new(),
    };
    for (index, record) in reader.records().enumerate() {
        let record = record.map_err(|e| GranolaError::Csv(e.to_string()))?;
        table.rows += 1;
        if table.rows > MAX_ROWS {
            return Err(GranolaError::TooManyRows);
        }
        let line = index + 2;
        match row(&record, &at, line) {
            Ok(meeting) => table.meetings.push(meeting),
            Err(reason) => table.rejected.push(RejectedRow {
                file: file.map(str::to_string),
                line,
                reason,
            }),
        }
    }
    Ok(table)
}

/// A posição de cada coluna reconhecida no cabeçalho.
struct Columns {
    id: Option<usize>,
    title: Option<usize>,
    workspace: Option<usize>,
    created: Option<usize>,
    summary: Option<usize>,
    notes: Option<usize>,
    transcript: Option<usize>,
}

fn row(
    record: &csv::StringRecord,
    at: &Columns,
    line: usize,
) -> Result<GranolaMeeting, RejectReason> {
    let cell = |index: Option<usize>| index.and_then(|i| record.get(i)).unwrap_or_default();
    let limited = |index: Option<usize>, name: &'static str, max: usize| {
        let value = cell(index);
        if value.len() > max {
            Err(RejectReason::CellTooLarge(name))
        } else {
            Ok(value.to_string())
        }
    };
    let document_id = cell(at.id).trim();
    if document_id.is_empty() {
        return Err(RejectReason::MissingId);
    }
    let created = cell(at.created).trim();
    let created_at =
        parse_date(created).ok_or_else(|| RejectReason::InvalidDate(created.to_string()))?;
    Ok(GranolaMeeting {
        line,
        document_id: document_id.to_string(),
        title: cell(at.title).trim().to_string(),
        workspace: cell(at.workspace).trim().to_string(),
        created_at,
        summary: limited(at.summary, "summary", MAX_SUMMARY_BYTES)?,
        notes: limited(at.notes, "notes", MAX_NOTES_BYTES)?,
        transcript: limited(at.transcript, "transcript", MAX_TRANSCRIPT_BYTES)?,
    })
}

/// Horas a leste de UTC de cada sigla de fuso aceita. Sem as ambíguas (`IST`, `CST` da China…):
/// `CST` vale como o horário central dos EUA, como no `Date` do JavaScript.
const ZONES: [(&str, i32); 30] = [
    ("UTC", 0),
    ("GMT", 0),
    ("WET", 0),
    ("WEST", 60),
    ("BST", 60),
    ("CET", 60),
    ("CEST", 120),
    ("EET", 120),
    ("EEST", 180),
    ("MSK", 180),
    ("EST", -300),
    ("EDT", -240),
    ("CST", -360),
    ("CDT", -300),
    ("MST", -420),
    ("MDT", -360),
    ("PST", -480),
    ("PDT", -420),
    ("AKST", -540),
    ("AKDT", -480),
    ("HST", -600),
    ("BRT", -180),
    ("ART", -180),
    ("JST", 540),
    ("KST", 540),
    ("SGT", 480),
    ("HKT", 480),
    ("AEST", 600),
    ("AEDT", 660),
    ("NZDT", 780),
];

/// Com o offset colado na hora (`+01`, `+0100`, `+01:00`), que o RFC 3339 do chrono não aceita.
const ATTACHED: [&str; 4] = [
    "%Y-%m-%dT%H:%M:%S%.f%#z",
    "%Y-%m-%d %H:%M:%S%.f%#z",
    "%Y-%m-%dT%H:%M%#z",
    "%Y-%m-%d %H:%M%#z",
];

const NAIVE: [&str; 4] = [
    "%Y-%m-%dT%H:%M:%S%.f",
    "%Y-%m-%d %H:%M:%S%.f",
    "%Y-%m-%dT%H:%M",
    "%Y-%m-%d %H:%M",
];

/// `document_created`: ISO 8601 com offset ou `Z`, ou data e hora seguidas de uma sigla de fuso
/// (`2025-03-14 10:30:00 BST`) ou de um offset, separado ou colado (`+01:00`, `+0100`, `+01`).
/// Sem fuso, vale UTC; só a data, meia-noite.
pub(crate) fn parse_date(text: &str) -> Option<DateTime<FixedOffset>> {
    let text = text.trim();
    if let Ok(at) = DateTime::parse_from_rfc3339(text) {
        return Some(at);
    }
    let zulu = match text.strip_suffix(['Z', 'z']) {
        Some(rest) => format!("{rest}+00:00"),
        None => text.to_string(),
    };
    if let Some(at) = ATTACHED
        .iter()
        .find_map(|format| DateTime::parse_from_str(&zulu, format).ok())
    {
        return Some(at);
    }
    let (rest, offset) = match text.rsplit_once(' ') {
        Some((rest, zone)) => match zone_offset(zone) {
            Some(offset) => (rest.trim_end(), offset),
            None => (text, utc()),
        },
        None => (text, utc()),
    };
    let naive = NAIVE
        .iter()
        .find_map(|format| NaiveDateTime::parse_from_str(rest, format).ok())
        .or_else(|| {
            NaiveDate::parse_from_str(rest, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(0, 0, 0))
        })?;
    offset.from_local_datetime(&naive).single()
}

fn utc() -> FixedOffset {
    Utc.fix()
}

/// `BST`, `CEST`, `+01:00`, `-0300`, `+01` ou `Z`.
fn zone_offset(zone: &str) -> Option<FixedOffset> {
    if zone.eq_ignore_ascii_case("z") {
        return Some(utc());
    }
    if let Some(&(_, minutes)) = ZONES
        .iter()
        .find(|(name, _)| zone.eq_ignore_ascii_case(name))
    {
        return FixedOffset::east_opt(minutes * 60);
    }
    let sign = match zone.chars().next()? {
        '+' => 1,
        '-' => -1,
        _ => return None,
    };
    let digits: String = zone[1..].chars().filter(|c| *c != ':').collect();
    if !matches!(digits.len(), 2 | 4) || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let hours: i32 = digits[..2].parse().ok()?;
    let minutes: i32 = digits
        .get(2..)
        .filter(|m| !m.is_empty())
        .map_or(Ok(0), str::parse)
        .ok()?;
    FixedOffset::east_opt(sign * (hours * 3_600 + minutes * 60))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str =
        "document_id,document_title,workspace_name,document_created,summary,notes,transcript\n";

    fn at(text: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(text).unwrap()
    }

    #[test]
    fn bom_uppercase_header_quotes_and_line_breaks() {
        let csv = "\u{FEFF}DOCUMENT_ID, Document_Title ,WORKSPACE_NAME,Document_Created,SUMMARY,NOTES,TRANSCRIPT\n\
                   g1,\"Planejamento, Q4\",Acme,2025-03-14T10:30:00Z,\"### Decisões\n- Lançar em \"\"novembro\"\"\",minhas notas,\"Me: oi\nThem: olá\"\n";
        let export = parse_granola_csv(csv.as_bytes()).unwrap();
        assert!(export.rejected.is_empty());
        let m = &export.meetings[0];
        assert_eq!(m.line, 2);
        assert_eq!(m.document_id, "g1");
        assert_eq!(m.title, "Planejamento, Q4");
        assert_eq!(m.workspace, "Acme");
        assert_eq!(m.created_at, at("2025-03-14T10:30:00Z"));
        assert_eq!(m.summary, "### Decisões\n- Lançar em \"novembro\"");
        assert_eq!(m.notes, "minhas notas");
        assert_eq!(m.transcript, "Me: oi\nThem: olá");
    }

    #[test]
    fn missing_required_columns_are_named() {
        let error = parse_granola_csv(b"document_title,summary\nx,y\n").unwrap_err();
        assert!(
            matches!(&error, GranolaError::MissingColumns(c) if c == &["document_id", "document_created"]),
            "{error:?}"
        );
        assert_eq!(
            error.to_string(),
            "faltam as colunas document_id, document_created"
        );
        // Só as obrigatórias: um export sem `transcript` nem `workspace_name` serve.
        let export = parse_granola_csv(b"document_id,document_created\na,2025-01-02\n").unwrap();
        assert_eq!(export.meetings[0].transcript, "");
        assert_eq!(export.meetings[0].created_at, at("2025-01-02T00:00:00Z"));
    }

    #[test]
    fn dates_take_zone_abbreviations_and_offsets() {
        assert_eq!(
            parse_date("2025-07-01 14:00:00 CEST"),
            Some(at("2025-07-01T14:00:00+02:00"))
        );
        assert_eq!(
            parse_date("2025-07-01 09:15 BST"),
            Some(at("2025-07-01T09:15:00+01:00"))
        );
        assert_eq!(
            parse_date("2025-07-01T09:15:00.250 pdt"),
            Some(at("2025-07-01T09:15:00.250-07:00"))
        );
        assert_eq!(
            parse_date("2025-07-01 09:15:00 -0300"),
            Some(at("2025-07-01T09:15:00-03:00"))
        );
        assert_eq!(
            parse_date("2025-07-01 09:15:00+05:30"),
            Some(at("2025-07-01T09:15:00+05:30"))
        );
        assert_eq!(
            parse_date("2025-07-01 09:15:00.123+00"),
            Some(at("2025-07-01T09:15:00.123Z"))
        );
        assert_eq!(
            parse_date("2025-07-01 09:15:00+0100"),
            Some(at("2025-07-01T09:15:00+01:00"))
        );
        assert_eq!(
            parse_date("2025-07-01T09:15Z"),
            Some(at("2025-07-01T09:15:00Z"))
        );
        assert_eq!(
            parse_date("2025-07-01 09:15:00 +01"),
            Some(at("2025-07-01T09:15:00+01:00"))
        );
        assert_eq!(
            parse_date("2025-07-01 09:15:00"),
            Some(at("2025-07-01T09:15:00Z"))
        );
        for bad in [
            "",
            "ontem",
            "2025-13-01",
            "2025-07-01 09:15 XYZ",
            "1/7/2025",
        ] {
            assert_eq!(parse_date(bad), None, "{bad}");
        }
    }

    #[test]
    fn bad_rows_are_rejected_with_line_and_reason() {
        let big = "x".repeat(MAX_SUMMARY_BYTES + 1);
        let csv = format!(
            "{HEADER}ok,Um,,2025-01-01T09:00:00Z,,,\n,Sem id,,2025-01-01T09:00:00Z,,,\n\
             d,Data ruim,,amanhã,,,\nbig,Grande,,2025-01-01T09:00:00Z,{big},,\nok2,Dois,,2025-01-02T09:00:00Z,,,\n"
        );
        let export = parse_granola_csv(csv.as_bytes()).unwrap();
        let ids: Vec<&str> = export
            .meetings
            .iter()
            .map(|m| m.document_id.as_str())
            .collect();
        assert_eq!(ids, ["ok", "ok2"]);
        let rejected: Vec<(usize, RejectReason)> = export
            .rejected
            .iter()
            .map(|r| (r.line, r.reason.clone()))
            .collect();
        assert_eq!(
            rejected,
            [
                (3, RejectReason::MissingId),
                (4, RejectReason::InvalidDate("amanhã".into())),
                (5, RejectReason::CellTooLarge("summary")),
            ]
        );
        assert_eq!(
            export.span(),
            Some((at("2025-01-01T09:00:00Z"), at("2025-01-02T09:00:00Z")))
        );
    }

    #[test]
    fn a_repeated_id_keeps_the_last_row() {
        let csv = format!(
            "{HEADER}a,Primeira,,2025-01-01T09:00:00Z,,,\nb,B,,2025-01-02T09:00:00Z,,,\na,Revisada,,2025-01-01T09:00:00Z,,,\n"
        );
        let export = parse_granola_csv(csv.as_bytes()).unwrap();
        let titles: Vec<&str> = export.meetings.iter().map(|m| m.title.as_str()).collect();
        assert_eq!(titles, ["B", "Revisada"]);
        assert_eq!(export.duplicates, 1);
    }

    #[test]
    fn empty_non_utf8_and_too_many_rows_refuse_the_file() {
        assert!(matches!(parse_granola_csv(b""), Err(GranolaError::Empty)));
        assert!(matches!(
            parse_granola_csv(b"\xEF\xBB\xBF \n"),
            Err(GranolaError::Empty)
        ));
        // UTF-16 LE: metade dos bytes é NUL.
        let utf16: Vec<u8> = HEADER.encode_utf16().flat_map(u16::to_le_bytes).collect();
        assert!(matches!(
            parse_granola_csv(&utf16),
            Err(GranolaError::NotUtf8)
        ));
        let mut csv = HEADER.to_string();
        for n in 0..=MAX_ROWS {
            csv.push_str(&format!("id{n},,,2025-01-01,,,\n"));
        }
        assert!(matches!(
            parse_granola_csv(csv.as_bytes()),
            Err(GranolaError::TooManyRows)
        ));
    }

    fn zip_of(files: &[(&str, &str)]) -> Vec<u8> {
        use std::io::Write;
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, content) in files {
            writer.start_file(*name, options).unwrap();
            writer.write_all(content.as_bytes()).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn a_zip_of_csvs_with_the_same_header_is_one_export() {
        let one = format!("{HEADER}a,A,W1,2025-01-01T09:00:00Z,,,\n,,,,,,\n");
        let two = format!("{HEADER}b,B,W2,2025-01-02T09:00:00Z,,,\n");
        let bytes = zip_of(&[
            ("export/W1.csv", &one),
            ("__MACOSX/export/._W1.csv", "lixo"),
            ("export/leia-me.txt", "x"),
            ("export/W2.CSV", &two),
        ]);
        let export = parse_granola_zip(&bytes).unwrap();
        let ids: Vec<&str> = export
            .meetings
            .iter()
            .map(|m| m.document_id.as_str())
            .collect();
        assert_eq!(ids, ["a", "b"]);
        assert_eq!(export.rejected[0].file.as_deref(), Some("export/W1.csv"));
        assert_eq!(export.rejected[0].line, 3);

        let other = "document_id,document_created,summary\nc,2025-01-03,\n";
        let mismatch = zip_of(&[("a.csv", &one), ("b.csv", other)]);
        assert!(matches!(
            parse_granola_zip(&mismatch),
            Err(GranolaError::ZipMismatch)
        ));
        assert!(matches!(
            parse_granola_zip(&zip_of(&[("leia-me.txt", "x")])),
            Err(GranolaError::NoCsvInZip)
        ));
    }
}
