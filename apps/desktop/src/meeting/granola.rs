//! "Import" → "Granola history": the "Historical Data Export" that Granola mails (a CSV, or a
//! `.zip` of them) becomes finished import sessions without audio (`fala_storage::granola`).
//! A preview reads and checks the file and writes nothing; the import then saves one meeting at
//! a time, so a re-import updates the same sessions. The file is read here and nothing of it
//! leaves the machine: no Granola login (ADR-0008).

use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use fala_media::CancelToken;
use fala_storage::{
    GranolaError, GranolaExport, GranolaMeeting, ImportOutcome, RejectReason, StorageError,
};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use super::pipeline::rfc3339;
use super::{now_local, MeetingManager};

/// Least time between two progress events.
const PROGRESS_EVERY: Duration = Duration::from_millis(250);

/// Why a Granola import stopped, serialized as `{"kind": "...", "detail": ...}` like
/// `MeetingError`, so the dialog picks the sentence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum GranolaImportError {
    Empty,
    NotUtf8,
    FileTooLarge,
    TooManyRows,
    /// The required columns that are missing, comma separated.
    MissingColumns(String),
    NoCsvInZip,
    ZipMismatch,
    /// The file could not be opened or parsed; the detail is the reader's message.
    Unreadable(String),
    /// Another import (a file, a URL or Granola) is running.
    Busy,
    /// Cancelled: the meetings saved before the click stay.
    Cancelled,
    Storage(String),
}

impl From<GranolaError> for GranolaImportError {
    fn from(error: GranolaError) -> Self {
        match error {
            GranolaError::Empty => Self::Empty,
            GranolaError::NotUtf8 => Self::NotUtf8,
            GranolaError::FileTooLarge => Self::FileTooLarge,
            GranolaError::TooManyRows => Self::TooManyRows,
            GranolaError::MissingColumns(columns) => Self::MissingColumns(columns.join(", ")),
            GranolaError::NoCsvInZip => Self::NoCsvInZip,
            GranolaError::ZipMismatch => Self::ZipMismatch,
            GranolaError::Zip(detail) | GranolaError::Csv(detail) => Self::Unreadable(detail),
            GranolaError::Io(e) => Self::Unreadable(e.to_string()),
        }
    }
}

impl From<StorageError> for GranolaImportError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error.to_string())
    }
}

/// Why a row was left out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum GranolaRejection {
    MissingId,
    /// The `document_created` text.
    InvalidDate(String),
    /// The column over its size cap.
    CellTooLarge(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GranolaRejected {
    /// The CSV inside the `.zip`, if the file was one.
    pub file: Option<String>,
    /// The spreadsheet row (the header is row 1).
    pub line: u32,
    pub reason: GranolaRejection,
}

/// What the file holds, before anything is saved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GranolaPreview {
    pub meetings: u32,
    /// RFC 3339 of the oldest and the newest meeting; `None` without meetings.
    pub first: Option<String>,
    pub last: Option<String>,
    /// Rows replaced by a later row with the same id.
    pub duplicates: u32,
    pub rejected: Vec<GranolaRejected>,
}

/// How the import ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GranolaImported {
    pub created: u32,
    /// Meetings imported before, rewritten with the file's content.
    pub updated: u32,
    pub rejected: u32,
}

/// "Importing `done` of `total`".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Event)]
pub struct GranolaProgress {
    pub done: u32,
    pub total: u32,
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Frees the one-import slot however the import ends, a panic included.
struct Slot<'a>(&'a Mutex<Option<CancelToken>>);

impl Drop for Slot<'_> {
    fn drop(&mut self) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

pub(crate) fn preview(export: &GranolaExport) -> GranolaPreview {
    let span = export.span();
    GranolaPreview {
        meetings: count(export.meetings.len()),
        first: span.map(|(first, _)| rfc3339(&first)),
        last: span.map(|(_, last)| rfc3339(&last)),
        duplicates: count(export.duplicates),
        rejected: export
            .rejected
            .iter()
            .map(|row| GranolaRejected {
                file: row.file.clone(),
                line: count(row.line),
                reason: match &row.reason {
                    RejectReason::MissingId => GranolaRejection::MissingId,
                    RejectReason::InvalidDate(text) => GranolaRejection::InvalidDate(text.clone()),
                    RejectReason::CellTooLarge(column) => {
                        GranolaRejection::CellTooLarge(column.to_string())
                    }
                },
            })
            .collect(),
    }
}

/// Saves every meeting of `export` with `save`, in file order, reporting `(done, total)` after
/// each one. A cancel stops between two meetings.
pub(crate) fn import_rows(
    export: &GranolaExport,
    cancel: &CancelToken,
    mut save: impl FnMut(&GranolaMeeting) -> Result<ImportOutcome, GranolaImportError>,
    mut progress: impl FnMut(u32, u32),
) -> Result<GranolaImported, GranolaImportError> {
    let total = count(export.meetings.len());
    let mut imported = GranolaImported {
        created: 0,
        updated: 0,
        rejected: count(export.rejected.len()),
    };
    progress(0, total);
    for (index, meeting) in export.meetings.iter().enumerate() {
        if cancel.is_cancelled() {
            return Err(GranolaImportError::Cancelled);
        }
        match save(meeting)? {
            ImportOutcome::Created => imported.created += 1,
            ImportOutcome::Updated => imported.updated += 1,
        }
        progress(count(index + 1), total);
    }
    Ok(imported)
}

fn read(path: &str) -> Result<GranolaExport, GranolaImportError> {
    Ok(fala_storage::read_granola_export(Path::new(path))?)
}

impl MeetingManager {
    /// Reads and checks the export at `path`; writes nothing.
    pub fn preview_granola(&self, path: &str) -> Result<GranolaPreview, GranolaImportError> {
        let export = read(path)?;
        log::info!(
            "granola export: {} meetings, {} rows rejected",
            export.meetings.len(),
            export.rejected.len()
        );
        Ok(preview(&export))
    }

    /// Imports the export at `path`, sharing the one-import-at-a-time slot (and its Cancel) with
    /// the file and URL import. Progress comes as `GranolaProgress` events.
    ///
    /// Not while a transcription or a notes generation runs: a re-import rewrites the segments
    /// that job may be reading.
    pub fn import_granola(&self, path: &str) -> Result<GranolaImported, GranolaImportError> {
        let export = read(path)?;
        if self.work.lock().unwrap_or_else(|e| e.into_inner()).busy() {
            return Err(GranolaImportError::Busy);
        }
        let cancel = CancelToken::new();
        {
            let mut importing = self.importing.lock().unwrap_or_else(|e| e.into_inner());
            if importing.is_some() {
                return Err(GranolaImportError::Busy);
            }
            *importing = Some(cancel.clone());
        }
        let _slot = Slot(&self.importing);
        let imported_at = now_local();
        let mut last: Option<Instant> = None;
        let result = import_rows(
            &export,
            &cancel,
            |meeting| {
                let mut guard = self.store.lock().unwrap_or_else(|e| e.into_inner());
                let store = guard.as_mut().ok_or_else(|| {
                    GranolaImportError::Storage("fala.sqlite is unavailable".to_string())
                })?;
                Ok(store.import_granola_meeting(meeting, imported_at)?.1)
            },
            |done, total| {
                if done < total && last.is_some_and(|at| at.elapsed() < PROGRESS_EVERY) {
                    return;
                }
                last = Some(Instant::now());
                if let Err(e) = (GranolaProgress { done, total }).emit(&self.app) {
                    log::debug!("granola progress not emitted: {e}");
                }
            },
        );
        match &result {
            Ok(imported) => log::info!("granola import: {imported:?}"),
            Err(error) => log::warn!("granola import stopped: {error:?}"),
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fala_storage::{parse_granola_csv, Store};

    const CSV: &str = "document_id,document_title,document_created,summary,transcript\n\
        a,Um,2025-03-14 10:30:00 CEST,- resumo,Me: oi\n\
        b,Dois,2025-03-20T09:00:00Z,,\n\
        ,Sem id,2025-03-21T09:00:00Z,,\n\
        c,Data ruim,ontem,,\n";

    #[test]
    fn the_preview_counts_and_names_the_rejected_rows() {
        let export = parse_granola_csv(CSV.as_bytes()).unwrap();
        let preview = preview(&export);
        assert_eq!(preview.meetings, 2);
        assert_eq!(preview.first.as_deref(), Some("2025-03-14T10:30:00+02:00"));
        assert_eq!(preview.last.as_deref(), Some("2025-03-20T09:00:00+00:00"));
        assert_eq!(
            preview.rejected,
            [
                GranolaRejected {
                    file: None,
                    line: 4,
                    reason: GranolaRejection::MissingId
                },
                GranolaRejected {
                    file: None,
                    line: 5,
                    reason: GranolaRejection::InvalidDate("ontem".into())
                },
            ]
        );
        assert_eq!(
            serde_json::to_value(&preview.rejected[1].reason).unwrap(),
            serde_json::json!({ "kind": "invalid_date", "detail": "ontem" })
        );
        assert_eq!(
            GranolaImportError::from(GranolaError::MissingColumns(vec![
                "document_id".into(),
                "document_created".into()
            ])),
            GranolaImportError::MissingColumns("document_id, document_created".into())
        );
    }

    #[test]
    fn importing_reports_progress_and_counts_a_second_run_as_updates() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("granola.sqlite"), dir.path()).unwrap();
        let export = parse_granola_csv(CSV.as_bytes()).unwrap();
        // The preview writes nothing.
        let _ = preview(&export);
        assert!(store.meetings().unwrap().is_empty());

        let at = now_local();
        let mut seen = Vec::new();
        let run = |store: &mut Store, seen: &mut Vec<(u32, u32)>| {
            import_rows(
                &export,
                &CancelToken::new(),
                |m| Ok(store.import_granola_meeting(m, at)?.1),
                |done, total| seen.push((done, total)),
            )
            .unwrap()
        };
        let first = run(&mut store, &mut seen);
        assert_eq!(
            first,
            GranolaImported {
                created: 2,
                updated: 0,
                rejected: 2
            }
        );
        assert_eq!(seen, [(0, 2), (1, 2), (2, 2)]);
        let again = run(&mut store, &mut seen);
        assert_eq!((again.created, again.updated), (0, 2));
        assert_eq!(store.meetings().unwrap().len(), 2);

        let cancel = CancelToken::new();
        cancel.cancel();
        let stopped = import_rows(&export, &cancel, |_| unreachable!(), |_, _| {});
        assert_eq!(stopped, Err(GranolaImportError::Cancelled));
    }
}
