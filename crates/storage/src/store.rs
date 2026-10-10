//! O `Store`: `fala.sqlite` com FTS5 e o espelho escrito depois de cada mudança.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, FixedOffset};
use fala_core::{AppContext, Dictation, Editor, Language, Transcript};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::metrics::{self, DictationMetrics, MetricsSummary};
use crate::mirror::{self, DITADOS};
use crate::{DictationRecord, ReindexReport, Showing, Skipped, StorageError};

/// Versão do schema em `PRAGMA user_version`; migrações futuras sobem esse número.
///
/// 1: as tabelas de `SCHEMA_1`. 2: as mesmas, com `auto_vacuum = INCREMENTAL`.
const SCHEMA_VERSION: i64 = 2;

const SCHEMA_1: &str = "
CREATE TABLE dictations (
    rowid INTEGER PRIMARY KEY,
    id TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    created_ms INTEGER NOT NULL,
    language TEXT NOT NULL,
    app TEXT,
    edited_by TEXT NOT NULL CHECK (edited_by IN ('none', 'rules', 'llm')),
    showing TEXT NOT NULL CHECK (showing IN ('final', 'raw')),
    raw TEXT NOT NULL,
    final TEXT NOT NULL,
    sensitive INTEGER NOT NULL DEFAULT 0 CHECK (sensitive IN (0, 1))
);
CREATE INDEX dictations_by_time ON dictations (created_ms DESC, id DESC);
CREATE VIRTUAL TABLE dictations_fts USING fts5(raw, final, content='dictations', content_rowid='rowid', tokenize='unicode61 remove_diacritics 2');
CREATE TRIGGER dictations_ai AFTER INSERT ON dictations BEGIN
    INSERT INTO dictations_fts (rowid, raw, final) VALUES (new.rowid, new.raw, new.final);
END;
CREATE TRIGGER dictations_ad AFTER DELETE ON dictations BEGIN
    INSERT INTO dictations_fts (dictations_fts, rowid, raw, final) VALUES ('delete', old.rowid, old.raw, old.final);
END;
CREATE TRIGGER dictations_au AFTER UPDATE OF raw, final ON dictations BEGIN
    INSERT INTO dictations_fts (dictations_fts, rowid, raw, final) VALUES ('delete', old.rowid, old.raw, old.final);
    INSERT INTO dictations_fts (rowid, raw, final) VALUES (new.rowid, new.raw, new.final);
END;
";

const COLUMNS: &str =
    "d.id, d.created_at, d.language, d.app, d.edited_by, d.showing, d.raw, d.final, d.sensitive";

/// O histórico de ditados: `fala.sqlite` mais o espelho em `<notes_dir>/Ditados/`.
#[derive(Debug)]
pub struct Store {
    conn: Connection,
    notes_dir: PathBuf,
}

impl Store {
    /// Abre (ou cria) o banco em WAL com `busy_timeout` de 5 s e aplica o schema.
    ///
    /// Banco novo nasce com `auto_vacuum = INCREMENTAL`, que só vale antes da primeira tabela.
    /// Banco na versão 1 passa por um `VACUUM` uma vez; se outro processo o segura, a migração
    /// fica para a próxima abertura. Com a versão em dia, as páginas livres voltam ao SO.
    pub fn open(db: &Path, notes_dir: &Path) -> Result<Self, StorageError> {
        if let Some(dir) = db.parent().filter(|d| !d.as_os_str().is_empty()) {
            fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(db)?;
        conn.busy_timeout(std::time::Duration::from_millis(5000))?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version == 0 {
            conn.pragma_update(None, "auto_vacuum", "INCREMENTAL")?;
        }
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get::<_, String>(0))?;
        match version {
            0 => conn.execute_batch(&format!(
                "BEGIN; {SCHEMA_1} PRAGMA user_version = {SCHEMA_VERSION}; COMMIT;"
            ))?,
            1 => {
                // `VACUUM` não roda dentro de transação; se cair entre ele e a versão, roda de novo.
                let migrated = conn.execute_batch(&format!(
                    "PRAGMA auto_vacuum = INCREMENTAL; VACUUM; PRAGMA user_version = {SCHEMA_VERSION};"
                ));
                if let Err(e) = migrated {
                    log::warn!("fala.sqlite: VACUUM da migração adiado: {e}");
                }
            }
            _ => {
                // Sem página livre, nada de pedir a trava de escrita (o MCP só lê).
                let free: i64 = conn.pragma_query_value(None, "freelist_count", |r| r.get(0))?;
                if free > 0
                    && let Err(e) = incremental_vacuum(&conn)
                {
                    log::warn!("fala.sqlite: incremental_vacuum adiado: {e}");
                }
            }
        }
        conn.execute_batch(metrics::SCHEMA)?;
        Ok(Store {
            conn,
            notes_dir: notes_dir.to_path_buf(),
        })
    }

    /// O `busy_timeout` da conexão, em ms.
    pub fn busy_timeout_ms(&self) -> Result<i64, StorageError> {
        Ok(self
            .conn
            .pragma_query_value(None, "busy_timeout", |r| r.get(0))?)
    }

    /// Grava um ditado: a linha e o FTS numa transação, depois o `.md`.
    ///
    /// Se só o espelho falhar, devolve `StorageError::Mirror` com o id; a linha fica no banco.
    pub fn add(
        &self,
        dictation: &Dictation,
        created_at: DateTime<FixedOffset>,
    ) -> Result<DictationRecord, StorageError> {
        self.add_marked(dictation, created_at, false)
    }

    /// Como `add`, com o item marcado como sensível.
    pub fn add_sensitive(
        &self,
        dictation: &Dictation,
        created_at: DateTime<FixedOffset>,
    ) -> Result<DictationRecord, StorageError> {
        self.add_marked(dictation, created_at, true)
    }

    fn add_marked(
        &self,
        dictation: &Dictation,
        created_at: DateTime<FixedOffset>,
        sensitive: bool,
    ) -> Result<DictationRecord, StorageError> {
        let record = DictationRecord {
            id: uuid::Uuid::now_v7().hyphenated().to_string(),
            created_at,
            dictation: dictation.clone(),
            showing: Showing::Final,
            sensitive,
        };
        insert(&self.conn, &record)?;
        log::debug!("ditado {} gravado", record.id);
        self.write_mirror(&record)?;
        Ok(record)
    }

    /// Lê um item pelo id.
    pub fn get(&self, id: &str) -> Result<DictationRecord, StorageError> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM dictations d WHERE d.id = ?1"),
                [id],
                read_row,
            )
            .optional()?
            .ok_or_else(|| StorageError::NotFound(id.to_string()))?
    }

    /// Busca sem acento no bruto e no final, mais recentes primeiro.
    ///
    /// Cada termo da consulta casa como prefixo e todos precisam casar; a sintaxe do FTS5 é
    /// tratada como texto. Consulta vazia devolve os mais recentes.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<DictationRecord>, StorageError> {
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let terms: Vec<&str> = query.split_whitespace().collect();
        let mut rows = Vec::new();
        if terms.is_empty() {
            let mut stmt = self.conn.prepare(&format!(
                "SELECT {COLUMNS} FROM dictations d ORDER BY d.created_ms DESC, d.id DESC LIMIT ?1"
            ))?;
            for row in stmt.query_map([limit], read_row)? {
                rows.push(row??);
            }
            return Ok(rows);
        }
        // Um termo sem letra nem dígito não vira token no `unicode61`: não há o que casar.
        if terms.iter().any(|t| !t.chars().any(char::is_alphanumeric)) {
            return Ok(rows);
        }
        let fts_query = terms
            .iter()
            .map(|t| format!("\"{}\"*", t.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" ");
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {COLUMNS} FROM dictations_fts f JOIN dictations d ON d.rowid = f.rowid
             WHERE dictations_fts MATCH ?1
             ORDER BY d.created_ms DESC, d.id DESC LIMIT ?2"
        ))?;
        for row in stmt.query_map(params![fts_query, limit], read_row)? {
            rows.push(row??);
        }
        Ok(rows)
    }

    /// Desfaz a edição: o item passa a mostrar o bruto. Sem mudança se já mostra.
    pub fn undo(&self, id: &str) -> Result<DictationRecord, StorageError> {
        self.set_showing(id, Showing::Raw)
    }

    /// Reaplica a edição: o item volta a mostrar o final. Sem mudança se já mostra.
    pub fn redo(&self, id: &str) -> Result<DictationRecord, StorageError> {
        self.set_showing(id, Showing::Final)
    }

    /// Guarda a resposta do LLM que chegou depois da colagem: o item passa a ter `final = text`,
    /// `editor = llm` e a mostrar o bruto, que é o que foi colado, até a pessoa aplicar a edição
    /// com `redo`. Bruto, app, data e marca de sensível ficam.
    ///
    /// Id desconhecido devolve `NotFound` e texto em branco devolve `EmptyEdit`, sem mudar nada.
    pub fn apply_late_edit(&self, id: &str, text: &str) -> Result<DictationRecord, StorageError> {
        if text.trim().is_empty() {
            return Err(StorageError::EmptyEdit(id.to_string()));
        }
        let mut record = self.get(id)?;
        self.conn.execute(
            "UPDATE dictations SET final = ?1, edited_by = ?2, showing = ?3 WHERE id = ?4",
            params![
                text,
                mirror::editor_str(Editor::Llm),
                Showing::Raw.as_str(),
                id
            ],
        )?;
        record.dictation.final_text = text.to_string();
        record.dictation.editor = Editor::Llm;
        record.showing = Showing::Raw;
        log::debug!("edição tardia aplicada ao ditado {id}");
        self.write_mirror(&record)?;
        Ok(record)
    }

    /// Apaga um item: a linha (o FTS sai pelo gatilho) e depois o `.md`.
    ///
    /// Id desconhecido devolve `NotFound` sem mudar nada; `.md` já ausente não é erro. Se só a
    /// remoção do `.md` falhar, devolve `StorageError::Mirror` com a linha já apagada.
    pub fn delete(&self, id: &str) -> Result<(), StorageError> {
        let record = self.get(id)?;
        self.conn
            .execute("DELETE FROM dictations WHERE id = ?1", [id])?;
        log::debug!("ditado {id} apagado");
        let path = mirror::path_for(&self.notes_dir, &record);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(StorageError::Mirror {
                id: id.to_string(),
                path,
                source,
            }),
        }
    }

    /// Reconstrói o banco a partir de `<notes_dir>/Ditados/**/*.md`, numa única transação.
    ///
    /// Um `.md` inválido ou com id repetido é ignorado e entra no relatório; os outros entram.
    pub fn reindex(&mut self) -> Result<ReindexReport, StorageError> {
        let root = self.notes_dir.join(DITADOS);
        let mut files = Vec::new();
        collect_md(&root, &mut files)?;
        files.sort();

        let mut report = ReindexReport::default();
        let mut records = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for path in files {
            let parsed = fs::read_to_string(&path)
                .map_err(|e| format!("não deu para ler: {e}"))
                .and_then(|content| mirror::parse(&content));
            match parsed {
                Ok(record) if !seen.insert(record.id.clone()) => report.skipped.push(Skipped {
                    path,
                    reason: "id duplicado".to_string(),
                }),
                Ok(record) => records.push(record),
                Err(reason) => report.skipped.push(Skipped { path, reason }),
            }
        }

        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM dictations", [])?;
        for record in &records {
            insert(&tx, record)?;
        }
        tx.commit()?;
        report.indexed = records.len();
        log::info!(
            "reindex: {} ditados, {} arquivos ignorados",
            report.indexed,
            report.skipped.len()
        );
        Ok(report)
    }

    /// Grava as métricas de um ditado (`dictation_metrics`); nunca texto.
    pub fn add_metrics(&self, metrics: &DictationMetrics) -> Result<(), StorageError> {
        metrics::insert(&self.conn, metrics)
    }

    /// Contagens e p50/p90 dos ditados dos últimos `days` dias.
    pub fn metrics_summary(&self, days: u32) -> Result<MetricsSummary, StorageError> {
        metrics::summary(&self.conn, days, chrono::Utc::now().timestamp_millis())
    }

    fn set_showing(&self, id: &str, showing: Showing) -> Result<DictationRecord, StorageError> {
        let mut record = self.get(id)?;
        if record.dictation.editor == Editor::None {
            return Err(StorageError::NothingToUndo(id.to_string()));
        }
        if record.showing == showing {
            return Ok(record);
        }
        self.conn.execute(
            "UPDATE dictations SET showing = ?1 WHERE id = ?2",
            params![showing.as_str(), id],
        )?;
        record.showing = showing;
        self.write_mirror(&record)?;
        Ok(record)
    }

    fn write_mirror(&self, record: &DictationRecord) -> Result<(), StorageError> {
        mirror::write(&self.notes_dir, record).map_err(|(path, source)| StorageError::Mirror {
            id: record.id.clone(),
            path,
            source,
        })
    }
}

/// Devolve todas as páginas livres. O pragma libera uma página por passo, então é lido até o fim.
fn incremental_vacuum(conn: &Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("PRAGMA incremental_vacuum")?;
    let mut rows = stmt.query([])?;
    while rows.next()?.is_some() {}
    Ok(())
}

fn insert(conn: &Connection, record: &DictationRecord) -> Result<(), StorageError> {
    let d = &record.dictation;
    conn.execute(
        "INSERT INTO dictations (id, created_at, created_ms, language, app, edited_by, showing, raw, final, sensitive)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            record.id,
            mirror::rfc3339(record),
            record.created_at.timestamp_millis(),
            d.raw.language.tag(),
            d.app.app_name,
            mirror::editor_str(d.editor),
            record.showing.as_str(),
            d.raw.text,
            d.final_text,
            record.sensitive,
        ],
    )?;
    Ok(())
}

/// Lê uma linha; um valor fora das portas (banco editado à mão) vira `Db(FromSqlConversionFailure)`.
fn read_row(row: &Row<'_>) -> rusqlite::Result<Result<DictationRecord, StorageError>> {
    let bad = |idx: usize, what: String| {
        StorageError::Db(rusqlite::Error::FromSqlConversionFailure(
            idx,
            rusqlite::types::Type::Text,
            what.into(),
        ))
    };
    let id: String = row.get(0)?;
    let created_at: String = row.get(1)?;
    let language: String = row.get(2)?;
    let app_name: Option<String> = row.get(3)?;
    let edited_by: String = row.get(4)?;
    let showing: String = row.get(5)?;
    let raw: String = row.get(6)?;
    let final_text: String = row.get(7)?;
    let sensitive: bool = row.get(8)?;

    let Ok(created_at) = DateTime::parse_from_rfc3339(&created_at) else {
        return Ok(Err(bad(1, format!("created_at inválido em {id}"))));
    };
    let Ok(language) = serde_json::from_value::<Language>(serde_json::Value::String(language))
    else {
        return Ok(Err(bad(2, format!("language inválido em {id}"))));
    };
    let Some(editor) = mirror::parse_editor(&edited_by) else {
        return Ok(Err(bad(4, format!("edited_by inválido em {id}"))));
    };
    let Some(showing) = Showing::parse(&showing) else {
        return Ok(Err(bad(5, format!("showing inválido em {id}"))));
    };
    Ok(Ok(DictationRecord {
        id,
        created_at,
        dictation: Dictation {
            raw: Transcript {
                text: raw,
                language,
            },
            final_text,
            editor,
            app: AppContext { app_name },
        },
        showing,
        sensitive,
    }))
}

/// Junta os `.md` sob `dir`, recursivo. Outras extensões (inclusive `.md.tmp`) ficam de fora.
fn collect_md(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), StorageError> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_md(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
    Ok(())
}
