use anyhow::{anyhow, Result};
use chrono::{DateTime, Local, Utc};
use log::{debug, error, info};
use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use tauri::AppHandle;
use tauri_specta::Event;

use crate::managers::history_dictations::{self, HistoryDictation};
use fala_core::{AppContext, Language};
use fala_storage::{Showing, StorageError, Store};

/// Database migrations for transcription history.
/// Each migration is applied in order. The library tracks which migrations
/// have been applied using SQLite's user_version pragma.
///
/// Note: For users upgrading from tauri-plugin-sql, migrate_from_tauri_plugin_sql()
/// converts the old _sqlx_migrations table tracking to the user_version pragma,
/// ensuring migrations don't re-run on existing databases.
pub(crate) static MIGRATIONS: &[M] = &[
    M::up(
        "CREATE TABLE IF NOT EXISTS transcription_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_name TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            saved BOOLEAN NOT NULL DEFAULT 0,
            title TEXT NOT NULL,
            transcription_text TEXT NOT NULL
        );",
    ),
    M::up("ALTER TABLE transcription_history ADD COLUMN post_processed_text TEXT;"),
    M::up("ALTER TABLE transcription_history ADD COLUMN post_process_prompt TEXT;"),
    M::up("ALTER TABLE transcription_history ADD COLUMN post_process_requested BOOLEAN NOT NULL DEFAULT 0;"),
    // The id of the matching dictation in fala.sqlite (fala-storage); null when the
    // transcription failed or the store was unavailable.
    M::up("ALTER TABLE transcription_history ADD COLUMN dictation_id TEXT;"),
    // The paste of this dictation failed: the text never reached the app.
    M::up("ALTER TABLE transcription_history ADD COLUMN paste_failed BOOLEAN NOT NULL DEFAULT 0;"),
];

/// The columns `map_history_entry` reads, in every query that returns entries.
const ENTRY_COLUMNS: &str = "id, file_name, timestamp, saved, title, transcription_text, \
     post_processed_text, post_process_prompt, post_process_requested, dictation_id, paste_failed";

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct PaginatedHistory {
    pub entries: Vec<HistoryEntry>,
    pub has_more: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(tag = "action")]
pub enum HistoryUpdatePayload {
    #[serde(rename = "added")]
    Added { entry: HistoryEntry },
    #[serde(rename = "updated")]
    Updated { entry: HistoryEntry },
    #[serde(rename = "deleted")]
    Deleted { id: i64 },
    #[serde(rename = "toggled")]
    Toggled { id: i64 },
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct HistoryEntry {
    pub id: i64,
    pub file_name: String,
    pub timestamp: i64,
    pub saved: bool,
    pub title: String,
    pub transcription_text: String,
    pub post_processed_text: Option<String>,
    pub post_process_prompt: Option<String>,
    pub post_process_requested: bool,
    /// The linked dictation in fala.sqlite, if any.
    pub dictation_id: Option<String>,
    /// That dictation (raw, final, editor, what it shows, app), when the store can read it.
    pub dictation: Option<HistoryDictation>,
    /// The paste failed, so the text never reached the app.
    pub paste_failed: bool,
    /// The dictation was not delivered and has text to recover (see `is_discarded`).
    pub discarded: bool,
}

/// The texts of one dictation, as the pipeline produced them.
pub struct EntryTexts {
    pub transcription_text: String,
    pub post_processed_text: Option<String>,
    pub post_process_prompt: Option<String>,
    /// The text that was pasted (the pipeline's final text); empty when transcription failed.
    pub pasted_text: String,
    /// The LLM produced `pasted_text`; the dictation is then recorded as `Editor::Llm`.
    pub llm_produced: bool,
}

/// A history entry about to be saved.
pub struct NewEntry {
    pub file_name: String,
    pub post_process_requested: bool,
    pub texts: EntryTexts,
    /// The app that had focus when the user released the shortcut.
    pub app: AppContext,
    /// The app is in `llm_disabled_apps`: the dictation is saved as sensitive.
    pub sensitive: bool,
    /// `utils::paste` failed for this dictation.
    pub paste_failed: bool,
}

pub struct HistoryManager {
    app_handle: AppHandle,
    recordings_dir: PathBuf,
    db_path: PathBuf,
    store: Option<Mutex<Store>>,
}

impl HistoryManager {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        // Create recordings directory in app data dir
        let app_data_dir = crate::portable::app_data_dir(app_handle)?;
        let recordings_dir = app_data_dir.join("recordings");
        let db_path = app_data_dir.join("history.db");

        // Ensure recordings directory exists
        if !recordings_dir.exists() {
            fs::create_dir_all(&recordings_dir)?;
            debug!("Created recordings directory: {:?}", recordings_dir);
        }

        let store = history_dictations::open_store(
            &app_data_dir.join("fala.sqlite"),
            &app_data_dir.join("notas"),
        );

        let manager = Self {
            app_handle: app_handle.clone(),
            recordings_dir,
            db_path,
            store: store.map(Mutex::new),
        };

        // Initialize database and run migrations synchronously
        manager.init_database()?;
        manager.backfill_dictations();

        Ok(manager)
    }

    fn lock_store(&self) -> Option<MutexGuard<'_, Store>> {
        self.store
            .as_ref()
            .map(|m| m.lock().unwrap_or_else(|poisoned| poisoned.into_inner()))
    }

    fn selected_language(&self) -> Language {
        history_dictations::language_from_setting(
            &crate::settings::get_settings(&self.app_handle).selected_language,
        )
    }

    /// Copies the rows that predate fala.sqlite into it, once; a failure is logged and retried
    /// on the next start.
    fn backfill_dictations(&self) {
        let Some(store) = self.lock_store() else {
            return;
        };
        let language = self.selected_language();
        let result = self
            .get_connection()
            .and_then(|conn| history_dictations::backfill(&conn, &store, language));
        if let Err(e) = result {
            error!("History backfill to fala.sqlite failed: {}", e);
        }
    }

    fn init_database(&self) -> Result<()> {
        info!("Initializing database at {:?}", self.db_path);

        let mut conn = Connection::open(&self.db_path)?;

        // Handle migration from tauri-plugin-sql to rusqlite_migration
        // tauri-plugin-sql used _sqlx_migrations table, rusqlite_migration uses user_version pragma
        self.migrate_from_tauri_plugin_sql(&conn)?;

        // Create migrations object and run to latest version
        let migrations = Migrations::new(MIGRATIONS.to_vec());

        // Validate migrations in debug builds
        #[cfg(debug_assertions)]
        migrations.validate().expect("Invalid migrations");

        // Get current version before migration
        let version_before: i32 =
            conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        debug!("Database version before migration: {}", version_before);

        // Apply any pending migrations
        migrations.to_latest(&mut conn)?;

        // Get version after migration
        let version_after: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

        if version_after > version_before {
            info!(
                "Database migrated from version {} to {}",
                version_before, version_after
            );
        } else {
            debug!("Database already at latest version {}", version_after);
        }

        Ok(())
    }

    /// Migrate from tauri-plugin-sql's migration tracking to rusqlite_migration's.
    /// tauri-plugin-sql used a _sqlx_migrations table, while rusqlite_migration uses
    /// SQLite's user_version pragma. This function checks if the old system was in use
    /// and sets the user_version accordingly so migrations don't re-run.
    fn migrate_from_tauri_plugin_sql(&self, conn: &Connection) -> Result<()> {
        // Check if the old _sqlx_migrations table exists
        let has_sqlx_migrations: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !has_sqlx_migrations {
            return Ok(());
        }

        // Check current user_version
        let current_version: i32 =
            conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

        if current_version > 0 {
            // Already migrated to rusqlite_migration system
            return Ok(());
        }

        // Get the highest version from the old migrations table
        let old_version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations WHERE success = 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if old_version > 0 {
            info!(
                "Migrating from tauri-plugin-sql (version {}) to rusqlite_migration",
                old_version
            );

            // Set user_version to match the old migration state
            conn.pragma_update(None, "user_version", old_version)?;

            // Optionally drop the old migrations table (keeping it doesn't hurt)
            // conn.execute("DROP TABLE IF EXISTS _sqlx_migrations", [])?;

            info!(
                "Migration tracking converted: user_version set to {}",
                old_version
            );
        }

        Ok(())
    }

    fn get_connection(&self) -> Result<Connection> {
        Ok(Connection::open(&self.db_path)?)
    }

    fn map_history_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> {
        Ok(HistoryEntry {
            id: row.get("id")?,
            file_name: row.get("file_name")?,
            timestamp: row.get("timestamp")?,
            saved: row.get("saved")?,
            title: row.get("title")?,
            transcription_text: row.get("transcription_text")?,
            post_processed_text: row.get("post_processed_text")?,
            post_process_prompt: row.get("post_process_prompt")?,
            post_process_requested: row.get("post_process_requested")?,
            dictation_id: row.get("dictation_id")?,
            dictation: None,
            paste_failed: row.get("paste_failed")?,
            discarded: false,
        })
    }

    /// Attaches the entry's dictation from the store and classifies it.
    fn attach_dictation(entry: &mut HistoryEntry, store: Option<&Store>) {
        entry.dictation = history_dictations::view(store, entry.dictation_id.as_deref());
        entry.discarded =
            history_dictations::is_discarded(entry.paste_failed, entry.dictation.as_ref());
    }

    pub fn recordings_dir(&self) -> &std::path::Path {
        &self.recordings_dir
    }

    /// Save a new history entry to the database, with its dictation in fala.sqlite.
    /// The WAV file should already have been written to the recordings directory.
    pub fn save_entry(&self, entry: NewEntry) -> Result<HistoryEntry> {
        let timestamp = Utc::now().timestamp();
        let language = self.selected_language();
        let conn = self.get_connection()?;
        let entry = {
            let store = self.lock_store();
            Self::save_entry_with(&conn, store.as_deref(), entry, language, timestamp)?
        };

        debug!("Saved history entry with id {}", entry.id);

        self.cleanup_old_entries()?;

        // Emit typed event for real-time frontend updates
        if let Err(e) = (HistoryUpdatePayload::Added {
            entry: entry.clone(),
        })
        .emit(&self.app_handle)
        {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(entry)
    }

    pub(crate) fn save_entry_with(
        conn: &Connection,
        store: Option<&Store>,
        entry: NewEntry,
        language: Language,
        timestamp: i64,
    ) -> Result<HistoryEntry> {
        let NewEntry {
            file_name,
            post_process_requested,
            texts,
            app,
            sensitive,
            paste_failed,
        } = entry;
        let title = Self::format_timestamp_title(timestamp);
        let dictation_id = store.and_then(|store| {
            let dictation = history_dictations::dictation_for(
                &texts.transcription_text,
                &texts.pasted_text,
                texts.llm_produced,
                language,
                app,
            )?;
            history_dictations::add_dictation(store, &dictation, timestamp, sensitive)
        });

        conn.execute(
            "INSERT INTO transcription_history (
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                post_process_requested,
                dictation_id,
                paste_failed
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                &file_name,
                timestamp,
                false,
                &title,
                &texts.transcription_text,
                &texts.post_processed_text,
                &texts.post_process_prompt,
                post_process_requested,
                &dictation_id,
                paste_failed,
            ],
        )?;

        let mut entry = HistoryEntry {
            id: conn.last_insert_rowid(),
            file_name,
            timestamp,
            saved: false,
            title,
            transcription_text: texts.transcription_text,
            post_processed_text: texts.post_processed_text,
            post_process_prompt: texts.post_process_prompt,
            post_process_requested,
            dictation_id,
            dictation: None,
            paste_failed,
            discarded: false,
        };
        Self::attach_dictation(&mut entry, store);
        Ok(entry)
    }

    /// Update an existing history entry with new transcription results (used by retry).
    pub fn update_transcription(&self, id: i64, texts: EntryTexts) -> Result<HistoryEntry> {
        let language = self.selected_language();
        let conn = self.get_connection()?;
        let entry = {
            let store = self.lock_store();
            Self::update_transcription_with(&conn, store.as_deref(), id, texts, language)?
        };

        debug!("Updated transcription for history entry {}", id);

        if let Err(e) = (HistoryUpdatePayload::Updated {
            entry: entry.clone(),
        })
        .emit(&self.app_handle)
        {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(entry)
    }

    /// Retry: rewrites the texts and replaces the linked dictation with one built from them,
    /// keeping the previous dictation's app.
    pub(crate) fn update_transcription_with(
        conn: &Connection,
        store: Option<&Store>,
        id: i64,
        texts: EntryTexts,
        language: Language,
    ) -> Result<HistoryEntry> {
        let updated = conn.execute(
            "UPDATE transcription_history
             SET transcription_text = ?1,
                 post_processed_text = ?2,
                 post_process_prompt = ?3
             WHERE id = ?4",
            params![
                texts.transcription_text,
                texts.post_processed_text,
                texts.post_process_prompt,
                id
            ],
        )?;

        if updated == 0 {
            return Err(anyhow!("History entry {} not found", id));
        }

        if let Some(store) = store {
            let (old_link, timestamp): (Option<String>, i64) = conn.query_row(
                "SELECT dictation_id, timestamp FROM transcription_history WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            // The replacement keeps the app and the sensitive mark of the dictation it replaces.
            let (app, sensitive) = old_link
                .as_deref()
                .and_then(|old| store.get(old).ok())
                .map(|record| (record.dictation.app, record.sensitive))
                .unwrap_or_default();
            let new_link = history_dictations::dictation_for(
                &texts.transcription_text,
                &texts.pasted_text,
                texts.llm_produced,
                language,
                app,
            )
            .and_then(|dictation| {
                history_dictations::add_dictation(store, &dictation, timestamp, sensitive)
            });
            if let Some(new_link) = new_link {
                conn.execute(
                    "UPDATE transcription_history SET dictation_id = ?1 WHERE id = ?2",
                    params![new_link, id],
                )?;
                if let Some(old) = old_link {
                    if let Err(e) = history_dictations::delete_dictation(store, &old) {
                        error!("Failed to delete replaced dictation {}: {}", old, e);
                    }
                }
            }
        }

        let mut entry = Self::get_entry_by_id_with(conn, id)?
            .ok_or_else(|| anyhow!("History entry {} not found", id))?;
        Self::attach_dictation(&mut entry, store);
        Ok(entry)
    }

    pub fn cleanup_old_entries(&self) -> Result<()> {
        let retention_period = crate::settings::get_recording_retention_period(&self.app_handle);

        match retention_period {
            crate::settings::RecordingRetentionPeriod::Never => {
                // Don't delete anything
                Ok(())
            }
            crate::settings::RecordingRetentionPeriod::PreserveLimit => {
                // Use the old count-based logic with history_limit
                let limit = crate::settings::get_history_limit(&self.app_handle);
                self.cleanup_by_count(limit)
            }
            _ => {
                // Use time-based logic
                self.cleanup_by_time(retention_period)
            }
        }
    }

    /// Deletes each entry's dictation, row and WAV. An entry whose dictation cannot be
    /// deleted stays for the next cleanup.
    fn delete_entries_and_files_with(
        conn: &Connection,
        store: Option<&Store>,
        recordings_dir: &Path,
        entries: &[(i64, String, Option<String>)],
    ) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        let mut deleted_count = 0;

        for (id, file_name, dictation_id) in entries {
            if let (Some(store), Some(dictation_id)) = (store, dictation_id) {
                if let Err(e) = history_dictations::delete_dictation(store, dictation_id) {
                    error!(
                        "Keeping history entry {}: its dictation could not be deleted: {}",
                        id, e
                    );
                    continue;
                }
            }

            // Delete database entry
            conn.execute(
                "DELETE FROM transcription_history WHERE id = ?1",
                params![id],
            )?;

            // Delete WAV file
            let file_path = recordings_dir.join(file_name);
            if file_path.exists() {
                if let Err(e) = fs::remove_file(&file_path) {
                    error!("Failed to delete WAV file {}: {}", file_name, e);
                } else {
                    debug!("Deleted old WAV file: {}", file_name);
                    deleted_count += 1;
                }
            }
        }

        Ok(deleted_count)
    }

    fn cleanup_by_count(&self, limit: usize) -> Result<()> {
        let conn = self.get_connection()?;
        let store = self.lock_store();
        Self::cleanup_by_count_with(&conn, store.as_deref(), &self.recordings_dir, limit)
    }

    pub(crate) fn cleanup_by_count_with(
        conn: &Connection,
        store: Option<&Store>,
        recordings_dir: &Path,
        limit: usize,
    ) -> Result<()> {
        // Get all entries that are not saved, ordered by timestamp desc
        let mut stmt = conn.prepare(
            "SELECT id, file_name, dictation_id FROM transcription_history WHERE saved = 0 ORDER BY timestamp DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>("id")?,
                row.get::<_, String>("file_name")?,
                row.get::<_, Option<String>>("dictation_id")?,
            ))
        })?;

        let mut entries: Vec<(i64, String, Option<String>)> = Vec::new();
        for row in rows {
            entries.push(row?);
        }

        if entries.len() > limit {
            let entries_to_delete = &entries[limit..];
            let deleted_count = Self::delete_entries_and_files_with(
                conn,
                store,
                recordings_dir,
                entries_to_delete,
            )?;

            if deleted_count > 0 {
                debug!("Cleaned up {} old history entries by count", deleted_count);
            }
        }

        Ok(())
    }

    fn cleanup_by_time(
        &self,
        retention_period: crate::settings::RecordingRetentionPeriod,
    ) -> Result<()> {
        let conn = self.get_connection()?;

        // Calculate cutoff timestamp (current time minus retention period)
        let now = Utc::now().timestamp();
        let cutoff_timestamp = match retention_period {
            crate::settings::RecordingRetentionPeriod::Days3 => now - (3 * 24 * 60 * 60), // 3 days in seconds
            crate::settings::RecordingRetentionPeriod::Weeks2 => now - (2 * 7 * 24 * 60 * 60), // 2 weeks in seconds
            crate::settings::RecordingRetentionPeriod::Months3 => now - (3 * 30 * 24 * 60 * 60), // 3 months in seconds (approximate)
            _ => unreachable!("Should not reach here"),
        };

        let store = self.lock_store();
        Self::cleanup_by_time_with(
            &conn,
            store.as_deref(),
            &self.recordings_dir,
            cutoff_timestamp,
        )
    }

    pub(crate) fn cleanup_by_time_with(
        conn: &Connection,
        store: Option<&Store>,
        recordings_dir: &Path,
        cutoff_timestamp: i64,
    ) -> Result<()> {
        // Get all unsaved entries older than the cutoff timestamp
        let mut stmt = conn.prepare(
            "SELECT id, file_name, dictation_id FROM transcription_history WHERE saved = 0 AND timestamp < ?1",
        )?;

        let rows = stmt.query_map(params![cutoff_timestamp], |row| {
            Ok((
                row.get::<_, i64>("id")?,
                row.get::<_, String>("file_name")?,
                row.get::<_, Option<String>>("dictation_id")?,
            ))
        })?;

        let mut entries_to_delete: Vec<(i64, String, Option<String>)> = Vec::new();
        for row in rows {
            entries_to_delete.push(row?);
        }

        let deleted_count =
            Self::delete_entries_and_files_with(conn, store, recordings_dir, &entries_to_delete)?;

        if deleted_count > 0 {
            debug!(
                "Cleaned up {} old history entries based on retention period",
                deleted_count
            );
        }

        Ok(())
    }

    pub async fn get_history_entries(
        &self,
        cursor: Option<i64>,
        limit: Option<usize>,
    ) -> Result<PaginatedHistory> {
        let conn = self.get_connection()?;
        let store = self.lock_store();
        Self::page_with(&conn, store.as_deref(), cursor, limit)
    }

    /// One page of entries, newest first, each linked entry carrying its dictation.
    pub(crate) fn page_with(
        conn: &Connection,
        store: Option<&Store>,
        cursor: Option<i64>,
        limit: Option<usize>,
    ) -> Result<PaginatedHistory> {
        let limit = limit.map(|l| l.min(100));

        let mut entries: Vec<HistoryEntry> = match (cursor, limit) {
            (Some(cursor_id), Some(lim)) => {
                let fetch_count = (lim + 1) as i64;
                let mut stmt = conn.prepare(&format!(
                    "SELECT {ENTRY_COLUMNS}
                     FROM transcription_history
                     WHERE id < ?1
                     ORDER BY id DESC
                     LIMIT ?2"
                ))?;
                let result = stmt
                    .query_map(params![cursor_id, fetch_count], Self::map_history_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                result
            }
            (None, Some(lim)) => {
                let fetch_count = (lim + 1) as i64;
                let mut stmt = conn.prepare(&format!(
                    "SELECT {ENTRY_COLUMNS}
                     FROM transcription_history
                     ORDER BY id DESC
                     LIMIT ?1"
                ))?;
                let result = stmt
                    .query_map(params![fetch_count], Self::map_history_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                result
            }
            (_, None) => {
                let mut stmt = conn.prepare(&format!(
                    "SELECT {ENTRY_COLUMNS}
                     FROM transcription_history
                     ORDER BY id DESC"
                ))?;
                let result = stmt
                    .query_map([], Self::map_history_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                result
            }
        };

        let has_more = limit.is_some_and(|lim| entries.len() > lim);
        if has_more {
            entries.pop();
        }
        for entry in &mut entries {
            Self::attach_dictation(entry, store);
        }

        Ok(PaginatedHistory { entries, has_more })
    }

    #[cfg(test)]
    fn get_latest_entry_with_conn(conn: &Connection) -> Result<Option<HistoryEntry>> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {ENTRY_COLUMNS}
             FROM transcription_history
             ORDER BY timestamp DESC
             LIMIT 1"
        ))?;

        let entry = stmt.query_row([], Self::map_history_entry).optional()?;
        Ok(entry)
    }

    /// Get the latest entry with non-empty transcription text.
    pub fn get_latest_completed_entry(&self) -> Result<Option<HistoryEntry>> {
        let conn = self.get_connection()?;
        Self::get_latest_completed_entry_with_conn(&conn)
    }

    fn get_latest_completed_entry_with_conn(conn: &Connection) -> Result<Option<HistoryEntry>> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {ENTRY_COLUMNS}
             FROM transcription_history
             WHERE transcription_text != ''
             ORDER BY timestamp DESC
             LIMIT 1"
        ))?;

        let entry = stmt.query_row([], Self::map_history_entry).optional()?;
        Ok(entry)
    }

    pub async fn toggle_saved_status(&self, id: i64) -> Result<()> {
        let conn = self.get_connection()?;

        // Get current saved status
        let current_saved: bool = conn.query_row(
            "SELECT saved FROM transcription_history WHERE id = ?1",
            params![id],
            |row| row.get("saved"),
        )?;

        let new_saved = !current_saved;

        conn.execute(
            "UPDATE transcription_history SET saved = ?1 WHERE id = ?2",
            params![new_saved, id],
        )?;

        debug!("Toggled saved status for entry {}: {}", id, new_saved);

        // Emit history updated event
        if let Err(e) = (HistoryUpdatePayload::Toggled { id }).emit(&self.app_handle) {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(())
    }

    pub fn get_audio_file_path(&self, file_name: &str) -> PathBuf {
        self.recordings_dir.join(file_name)
    }

    pub async fn get_entry_by_id(&self, id: i64) -> Result<Option<HistoryEntry>> {
        let conn = self.get_connection()?;
        Self::get_entry_by_id_with(&conn, id)
    }

    fn get_entry_by_id_with(conn: &Connection, id: i64) -> Result<Option<HistoryEntry>> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {ENTRY_COLUMNS}
             FROM transcription_history
             WHERE id = ?1"
        ))?;

        let entry = stmt.query_row([id], Self::map_history_entry).optional()?;

        Ok(entry)
    }

    pub async fn delete_entry(&self, id: i64) -> Result<()> {
        let conn = self.get_connection()?;
        {
            let store = self.lock_store();
            Self::delete_entry_with(&conn, store.as_deref(), &self.recordings_dir, id)?;
        }

        debug!("Deleted history entry with id: {}", id);

        // Emit history updated event
        if let Err(e) = (HistoryUpdatePayload::Deleted { id }).emit(&self.app_handle) {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(())
    }

    /// Deletes the linked dictation first; if that fails, the row and the WAV stay.
    pub(crate) fn delete_entry_with(
        conn: &Connection,
        store: Option<&Store>,
        recordings_dir: &Path,
        id: i64,
    ) -> Result<()> {
        // Get the entry to find the file name
        if let Some(entry) = Self::get_entry_by_id_with(conn, id)? {
            if let Some(dictation_id) = entry.dictation_id.as_deref() {
                match store {
                    Some(store) => history_dictations::delete_dictation(store, dictation_id)
                        .map_err(|e| {
                            anyhow!("Failed to delete dictation {}: {}", dictation_id, e)
                        })?,
                    None => error!(
                        "Dictation {} stays in fala.sqlite: the store is unavailable",
                        dictation_id
                    ),
                }
            }

            // Delete the audio file first
            let file_path = recordings_dir.join(&entry.file_name);
            if file_path.exists() {
                if let Err(e) = fs::remove_file(&file_path) {
                    error!("Failed to delete audio file {}: {}", entry.file_name, e);
                    // Continue with database deletion even if file deletion fails
                }
            }
        }

        // Delete from database
        conn.execute(
            "DELETE FROM transcription_history WHERE id = ?1",
            params![id],
        )?;

        Ok(())
    }

    /// Keeps the LLM answer that arrived after the paste on the entry's dictation (see
    /// `Store::apply_late_edit`); history.db and the pasted text stay as they were.
    pub fn apply_late_edit(&self, id: i64, text: &str) -> Result<HistoryEntry> {
        let conn = self.get_connection()?;
        let store = self.lock_store();
        Self::apply_late_edit_with(&conn, store.as_deref(), id, text)
    }

    pub(crate) fn apply_late_edit_with(
        conn: &Connection,
        store: Option<&Store>,
        id: i64,
        text: &str,
    ) -> Result<HistoryEntry> {
        let store = store.ok_or_else(|| anyhow!("fala.sqlite is not available"))?;
        let mut entry = Self::get_entry_by_id_with(conn, id)?
            .ok_or_else(|| anyhow!("History entry {} not found", id))?;
        let dictation_id = entry
            .dictation_id
            .clone()
            .ok_or_else(|| anyhow!("History entry {} has no dictation", id))?;
        let record = match store.apply_late_edit(&dictation_id, text) {
            Ok(record) => record,
            Err(StorageError::Mirror { path, source, .. }) => {
                error!(
                    "Late edit of dictation {} saved, but its mirror {} failed: {}",
                    dictation_id,
                    path.display(),
                    source
                );
                store.get(&dictation_id).map_err(|e| anyhow!("{}", e))?
            }
            Err(e) => return Err(anyhow!("{}", e)),
        };
        entry.dictation = Some(HistoryDictation::from(&record));
        entry.discarded =
            history_dictations::is_discarded(entry.paste_failed, entry.dictation.as_ref());
        Ok(entry)
    }

    /// Tells the front an entry changed (`history-update-payload` `updated`).
    pub fn announce_updated(&self, entry: HistoryEntry) {
        if let Err(e) = (HistoryUpdatePayload::Updated { entry }).emit(&self.app_handle) {
            error!("Failed to emit history-updated event: {}", e);
        }
    }

    /// Undo (`Showing::Raw`) or redo (`Showing::Final`) the edit of an entry's dictation.
    pub fn set_showing(&self, id: i64, showing: Showing) -> Result<HistoryEntry> {
        let conn = self.get_connection()?;
        let entry = {
            let store = self.lock_store();
            Self::set_showing_with(&conn, store.as_deref(), id, showing)?
        };
        if let Err(e) = (HistoryUpdatePayload::Updated {
            entry: entry.clone(),
        })
        .emit(&self.app_handle)
        {
            error!("Failed to emit history-updated event: {}", e);
        }
        Ok(entry)
    }

    pub(crate) fn set_showing_with(
        conn: &Connection,
        store: Option<&Store>,
        id: i64,
        showing: Showing,
    ) -> Result<HistoryEntry> {
        let store = store.ok_or_else(|| anyhow!("fala.sqlite is not available"))?;
        let mut entry = Self::get_entry_by_id_with(conn, id)?
            .ok_or_else(|| anyhow!("History entry {} not found", id))?;
        let dictation_id = entry
            .dictation_id
            .clone()
            .ok_or_else(|| anyhow!("History entry {} has no dictation", id))?;
        let record = match showing {
            Showing::Raw => store.undo(&dictation_id),
            Showing::Final => store.redo(&dictation_id),
        }
        .map_err(|e| anyhow!("{}", e))?;
        entry.dictation = Some(HistoryDictation::from(&record));
        entry.discarded =
            history_dictations::is_discarded(entry.paste_failed, entry.dictation.as_ref());
        Ok(entry)
    }

    /// "Recuperar": the discarded entry becomes a normal one, showing the text it has.
    pub fn recover(&self, id: i64) -> Result<HistoryEntry> {
        let conn = self.get_connection()?;
        let entry = {
            let store = self.lock_store();
            Self::recover_with(&conn, store.as_deref(), id)?
        };
        if let Err(e) = (HistoryUpdatePayload::Updated {
            entry: entry.clone(),
        })
        .emit(&self.app_handle)
        {
            error!("Failed to emit history-updated event: {}", e);
        }
        Ok(entry)
    }

    /// Clears the paste failure and, when the pipeline emptied the text, shows the raw one.
    /// An entry that is not discarded is an error and nothing changes.
    pub(crate) fn recover_with(
        conn: &Connection,
        store: Option<&Store>,
        id: i64,
    ) -> Result<HistoryEntry> {
        let mut entry = Self::get_entry_by_id_with(conn, id)?
            .ok_or_else(|| anyhow!("History entry {} not found", id))?;
        Self::attach_dictation(&mut entry, store);
        if !entry.discarded {
            return Err(anyhow!("History entry {} is not discarded", id));
        }
        if let (Some(store), Some(dictation_id), Some(dictation)) = (
            store,
            entry.dictation_id.as_deref(),
            entry.dictation.as_ref(),
        ) {
            if history_dictations::shows_emptied_text(dictation) {
                store.undo(dictation_id).map_err(|e| anyhow!("{}", e))?;
            }
        }
        conn.execute(
            "UPDATE transcription_history SET paste_failed = 0 WHERE id = ?1",
            params![id],
        )?;
        entry.paste_failed = false;
        Self::attach_dictation(&mut entry, store);
        Ok(entry)
    }

    fn format_timestamp_title(timestamp: i64) -> String {
        if let Some(utc_datetime) = DateTime::from_timestamp(timestamp, 0) {
            // Convert UTC to local timezone
            let local_datetime = utc_datetime.with_timezone(&Local);
            local_datetime.format("%B %e, %Y - %l:%M%p").to_string()
        } else {
            format!("Recording {}", timestamp)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE transcription_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_name TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                saved BOOLEAN NOT NULL DEFAULT 0,
                title TEXT NOT NULL,
                transcription_text TEXT NOT NULL,
                post_processed_text TEXT,
                post_process_prompt TEXT,
                post_process_requested BOOLEAN NOT NULL DEFAULT 0,
                dictation_id TEXT,
                paste_failed BOOLEAN NOT NULL DEFAULT 0
            );",
        )
        .expect("create transcription_history table");
        conn
    }

    fn insert_entry(conn: &Connection, timestamp: i64, text: &str, post_processed: Option<&str>) {
        conn.execute(
            "INSERT INTO transcription_history (
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                post_process_requested
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                format!("fala-{}.wav", timestamp),
                timestamp,
                false,
                format!("Recording {}", timestamp),
                text,
                post_processed,
                Option::<String>::None,
                false,
            ],
        )
        .expect("insert history entry");
    }

    #[test]
    fn migration_five_adds_nullable_dictation_id() {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        Migrations::new(MIGRATIONS[..4].to_vec())
            .to_latest(&mut conn)
            .expect("migrate to version 4");
        insert_entry(&conn, 100, "antes", None);
        let version: i32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 4);

        Migrations::new(MIGRATIONS[..5].to_vec())
            .to_latest(&mut conn)
            .expect("migrate to version 5");

        let version: i32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 5);
        let notnull: i64 = conn
            .query_row(
                "SELECT \"notnull\" FROM pragma_table_info('transcription_history') WHERE name = 'dictation_id'",
                [],
                |row| row.get(0),
            )
            .expect("dictation_id column exists");
        assert_eq!(notnull, 0);
        let link: Option<String> = conn
            .query_row(
                "SELECT dictation_id FROM transcription_history WHERE timestamp = 100",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(link, None);
    }

    #[test]
    fn migration_six_adds_paste_failed() {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        Migrations::new(MIGRATIONS[..5].to_vec())
            .to_latest(&mut conn)
            .expect("migrate to version 5");
        insert_entry(&conn, 100, "antes", None);

        Migrations::new(MIGRATIONS.to_vec())
            .to_latest(&mut conn)
            .expect("migrate to latest");

        let version: i32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 6);
        let (notnull, default): (i64, String) = conn
            .query_row(
                "SELECT \"notnull\", dflt_value FROM pragma_table_info('transcription_history') WHERE name = 'paste_failed'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("paste_failed column exists");
        assert_eq!((notnull, default.as_str()), (1, "0"));
        let entry = HistoryManager::get_latest_entry_with_conn(&conn)
            .unwrap()
            .expect("the old row survives");
        assert_eq!(entry.transcription_text, "antes");
        assert!(!entry.paste_failed);
    }

    #[test]
    fn get_latest_entry_returns_none_when_empty() {
        let conn = setup_conn();
        let entry = HistoryManager::get_latest_entry_with_conn(&conn).expect("fetch latest entry");
        assert!(entry.is_none());
    }

    #[test]
    fn get_latest_entry_returns_newest_entry() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "first", None);
        insert_entry(&conn, 200, "second", Some("processed"));

        let entry = HistoryManager::get_latest_entry_with_conn(&conn)
            .expect("fetch latest entry")
            .expect("entry exists");

        assert_eq!(entry.timestamp, 200);
        assert_eq!(entry.transcription_text, "second");
        assert_eq!(entry.post_processed_text.as_deref(), Some("processed"));
    }

    #[test]
    fn get_latest_completed_entry_skips_empty_entries() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "completed", None);
        insert_entry(&conn, 200, "", None);

        let entry = HistoryManager::get_latest_completed_entry_with_conn(&conn)
            .expect("fetch latest completed entry")
            .expect("completed entry exists");

        assert_eq!(entry.timestamp, 100);
        assert_eq!(entry.transcription_text, "completed");
    }
}
