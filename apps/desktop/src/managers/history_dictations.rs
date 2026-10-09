//! Ligação do histórico herdado (`history.db`) ao `fala-storage` (`fala.sqlite`).
//!
//! `history.db` continua dono do áudio, da estrela, do título e da retenção; cada linha com
//! texto aponta, por `dictation_id`, para um item de `fala.sqlite` com o bruto, o texto colado,
//! quem editou e o app. As funções daqui recebem a conexão e o `Store` para que os testes rodem
//! sem `AppHandle`.

use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, FixedOffset, Local, Utc};
use fala_core::{AppContext, Dictation, Editor, Language, Transcript};
use fala_storage::{DictationRecord, Showing, StorageError, Store};
use log::{debug, error, info};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use specta::Type;

/// Quem produziu o texto final de um item, como o front o recebe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum HistoryEditor {
    None,
    Rules,
    Llm,
}

/// Qual texto o item mostra agora: o final ou, depois de desfazer, o bruto.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum HistoryShowing {
    Final,
    Raw,
}

/// O item de `fala.sqlite` ligado a uma linha do histórico.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct HistoryDictation {
    pub raw_text: String,
    pub final_text: String,
    pub editor: HistoryEditor,
    pub showing: HistoryShowing,
    pub app_name: Option<String>,
}

impl From<&DictationRecord> for HistoryDictation {
    fn from(record: &DictationRecord) -> Self {
        let d = &record.dictation;
        HistoryDictation {
            raw_text: d.raw.text.clone(),
            final_text: d.final_text.clone(),
            editor: match d.editor {
                Editor::None => HistoryEditor::None,
                Editor::Rules => HistoryEditor::Rules,
                Editor::Llm => HistoryEditor::Llm,
            },
            showing: match record.showing {
                Showing::Final => HistoryShowing::Final,
                Showing::Raw => HistoryShowing::Raw,
            },
            app_name: d.app.app_name.clone(),
        }
    }
}

/// Abre `fala.sqlite`; numa falha registra o erro e devolve `None`, e o histórico segue sem vínculo.
pub(crate) fn open_store(db: &Path, notes_dir: &Path) -> Option<Store> {
    match Store::open(db, notes_dir) {
        Ok(store) => Some(store),
        Err(e) => {
            error!(
                "fala.sqlite unavailable at {}: {e}; history keeps working without dictations",
                db.display()
            );
            None
        }
    }
}

/// `none` quando o colado é o bruto; `llm` quando o LLM produziu o colado; `rules` nos outros casos.
pub(crate) fn editor_for(raw: &str, pasted: &str, llm_produced: bool) -> Editor {
    if pasted == raw {
        Editor::None
    } else if llm_produced {
        Editor::Llm
    } else {
        Editor::Rules
    }
}

/// O idioma de `selected_language`; `pt-BR` quando não é pt nem en (`auto`, `es`, ...).
pub(crate) fn language_from_setting(setting: &str) -> Language {
    setting.parse().unwrap_or(Language::PtBr)
}

/// O instante de um `timestamp` (segundos UTC) no fuso local.
pub(crate) fn created_at(timestamp: i64) -> DateTime<FixedOffset> {
    DateTime::from_timestamp(timestamp, 0)
        .unwrap_or_else(Utc::now)
        .with_timezone(&Local)
        .fixed_offset()
}

/// O `Dictation` de um texto do ASR e do texto colado; `None` sem texto do ASR.
pub(crate) fn dictation_for(
    raw: &str,
    pasted: &str,
    llm_produced: bool,
    language: Language,
    app: AppContext,
) -> Option<Dictation> {
    if raw.is_empty() {
        return None;
    }
    Some(Dictation {
        raw: Transcript {
            text: raw.to_string(),
            language,
        },
        final_text: pasted.to_string(),
        editor: editor_for(raw, pasted, llm_produced),
        app,
    })
}

/// Grava um item (sensível por `Store::add_sensitive`) e devolve o id; com falha só no espelho,
/// o id que o erro informa.
pub(crate) fn add_dictation(
    store: &Store,
    dictation: &Dictation,
    timestamp: i64,
    sensitive: bool,
) -> Option<String> {
    let added = if sensitive {
        store.add_sensitive(dictation, created_at(timestamp))
    } else {
        store.add(dictation, created_at(timestamp))
    };
    match added {
        Ok(record) => Some(record.id),
        Err(StorageError::Mirror { id, path, source }) => {
            error!(
                "dictation {id} saved, but its mirror {} failed: {source}",
                path.display()
            );
            Some(id)
        }
        Err(e) => {
            error!("Failed to save dictation to fala.sqlite: {e}");
            None
        }
    }
}

/// Apaga um item. `NotFound` e falha só no espelho contam como apagado (a linha já não existe).
pub(crate) fn delete_dictation(store: &Store, id: &str) -> Result<(), StorageError> {
    match store.delete(id) {
        Ok(()) => Ok(()),
        Err(StorageError::NotFound(_)) => {
            debug!("dictation {id} was already gone from fala.sqlite");
            Ok(())
        }
        Err(StorageError::Mirror { path, source, .. }) => {
            error!(
                "dictation {id} deleted, but its mirror {} could not be removed: {source}",
                path.display()
            );
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// O item ligado a uma linha, se o store está aberto e o item existe.
pub(crate) fn view(store: Option<&Store>, dictation_id: Option<&str>) -> Option<HistoryDictation> {
    let (store, id) = (store?, dictation_id?);
    match store.get(id) {
        Ok(record) => Some(HistoryDictation::from(&record)),
        Err(e) => {
            debug!("dictation {id} not readable: {e}");
            None
        }
    }
}

/// Copia para `fala.sqlite` cada linha com texto e sem vínculo (door 4 do plano) e grava o id.
pub(crate) fn backfill(conn: &Connection, store: &Store, language: Language) -> Result<usize> {
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, transcription_text, post_processed_text, post_process_requested
         FROM transcription_history
         WHERE dictation_id IS NULL AND transcription_text != ''
         ORDER BY id",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, bool>(4)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut copied = 0;
    for (id, timestamp, raw, post_processed, requested) in rows {
        let editor = match (&post_processed, requested) {
            (None, _) => Editor::None,
            (Some(_), true) => Editor::Llm,
            (Some(_), false) => Editor::Rules,
        };
        let dictation = Dictation {
            final_text: post_processed.unwrap_or_else(|| raw.clone()),
            raw: Transcript {
                text: raw,
                language,
            },
            editor,
            app: AppContext::default(),
        };
        let Some(dictation_id) = add_dictation(store, &dictation, timestamp, false) else {
            anyhow::bail!("backfill stopped at history entry {id}");
        };
        conn.execute(
            "UPDATE transcription_history SET dictation_id = ?1 WHERE id = ?2",
            params![dictation_id, id],
        )?;
        copied += 1;
    }
    if copied > 0 {
        info!("Copied {copied} history entries to fala.sqlite");
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managers::history::{
        EntryTexts, HistoryEntry, HistoryManager, NewEntry, MIGRATIONS,
    };
    use chrono::{Offset, TimeZone};
    use rusqlite_migration::Migrations;
    use std::fs;
    use std::path::PathBuf;

    struct Env {
        _dir: tempfile::TempDir,
        db: PathBuf,
        notes: PathBuf,
        recordings: PathBuf,
    }

    fn scratch() -> Env {
        let dir = tempfile::tempdir().unwrap();
        let recordings = dir.path().join("recordings");
        fs::create_dir_all(&recordings).unwrap();
        Env {
            db: dir.path().join("fala.sqlite"),
            notes: dir.path().join("notas"),
            recordings,
            _dir: dir,
        }
    }

    impl Env {
        fn store(&self) -> Store {
            Store::open(&self.db, &self.notes).unwrap()
        }

        /// Toda operação do store passa a falhar com `Db`.
        fn break_store(&self) {
            Connection::open(&self.db)
                .unwrap()
                .execute_batch("DROP TABLE dictations;")
                .unwrap();
        }

        fn md_of(&self, id: &str) -> Option<PathBuf> {
            fn walk(dir: &Path, id: &str) -> Option<PathBuf> {
                for entry in fs::read_dir(dir).ok()? {
                    let path = entry.ok()?.path();
                    if path.is_dir() {
                        if let Some(hit) = walk(&path, id) {
                            return Some(hit);
                        }
                    } else if path.to_string_lossy().ends_with(&format!("-{id}.md")) {
                        return Some(path);
                    }
                }
                None
            }
            walk(&self.notes.join("Ditados"), id)
        }

        fn wav(&self, name: &str) -> PathBuf {
            let path = self.recordings.join(name);
            fs::write(&path, b"RIFF").unwrap();
            path
        }
    }

    fn history() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::new(MIGRATIONS.to_vec())
            .to_latest(&mut conn)
            .unwrap();
        conn
    }

    fn entry(
        file: &str,
        raw: &str,
        pasted: &str,
        requested: bool,
        post_processed: Option<&str>,
        app: Option<&str>,
    ) -> NewEntry {
        NewEntry {
            file_name: file.to_string(),
            post_process_requested: requested,
            texts: EntryTexts {
                transcription_text: raw.to_string(),
                post_processed_text: post_processed.map(str::to_string),
                post_process_prompt: None,
                pasted_text: pasted.to_string(),
                llm_produced: requested && post_processed.is_some(),
            },
            app: AppContext {
                app_name: app.map(str::to_string),
            },
            sensitive: false,
        }
    }

    /// Uma entrada editada pelo LLM no app "notepad".
    fn llm_entry(file: &str) -> NewEntry {
        entry(
            file,
            "acao de amanha",
            "Ação de amanhã.",
            true,
            Some("Ação de amanhã."),
            Some("notepad"),
        )
    }

    fn save(conn: &Connection, store: Option<&Store>, e: NewEntry, ts: i64) -> HistoryEntry {
        HistoryManager::save_entry_with(conn, store, e, Language::PtBr, ts).unwrap()
    }

    fn link(conn: &Connection, id: i64) -> Option<String> {
        conn.query_row(
            "SELECT dictation_id FROM transcription_history WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn rows(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM transcription_history", [], |r| {
            r.get(0)
        })
        .unwrap()
    }

    fn insert_legacy(
        conn: &Connection,
        ts: i64,
        raw: &str,
        post_processed: Option<&str>,
        requested: bool,
    ) -> i64 {
        conn.execute(
            "INSERT INTO transcription_history (file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, post_process_requested)
             VALUES (?1, ?2, 0, ?3, ?4, ?5, NULL, ?6)",
            params![format!("fala-{ts}.wav"), ts, format!("Recording {ts}"), raw, post_processed, requested],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn not_found(store: &Store, id: &str) -> bool {
        matches!(store.get(id), Err(StorageError::NotFound(_)))
    }

    #[test]
    fn disabled_app_marks_sensitive() {
        let mut settings = crate::settings::get_default_settings();
        settings.llm_disabled_apps = vec!["keepassxc".to_string()];
        for (app, sensitive) in [("KeePassXC", true), ("notepad", false)] {
            let env = scratch();
            let store = env.store();
            let conn = history();
            let mut e = entry(
                "fala-1.wav",
                "senha nova",
                "Senha nova",
                false,
                None,
                Some(app),
            );
            e.sensitive = crate::llm_auto::is_disabled_app(&settings, &e.app);

            let saved = save(&conn, Some(&store), e, 1);

            let record = store.get(&link(&conn, saved.id).unwrap()).unwrap();
            assert_eq!(record.sensitive, sensitive, "{app}");
        }
    }

    #[test]
    fn undo_after_late_edit() {
        let env = scratch();
        let store = env.store();
        let conn = history();
        let saved = save(
            &conn,
            Some(&store),
            entry("fala-1.wav", "bruto", "Bruto", false, Some("Bruto"), None),
            1,
        );
        let id = link(&conn, saved.id).unwrap();

        let late =
            HistoryManager::apply_late_edit_with(&conn, Some(&store), saved.id, "Bruto, editado.")
                .unwrap();
        let shown = late.dictation.unwrap();
        assert_eq!(shown.editor, HistoryEditor::Llm);
        assert_eq!(shown.showing, HistoryShowing::Raw);

        let applied =
            HistoryManager::set_showing_with(&conn, Some(&store), saved.id, Showing::Final)
                .unwrap();
        assert_eq!(applied.dictation.unwrap().showing, HistoryShowing::Final);
        assert_eq!(store.get(&id).unwrap().shown_text(), "Bruto, editado.");

        let undone =
            HistoryManager::set_showing_with(&conn, Some(&store), saved.id, Showing::Raw).unwrap();
        let undone = undone.dictation.unwrap();
        assert_eq!(undone.showing, HistoryShowing::Raw);
        assert_eq!(undone.editor, HistoryEditor::Llm);
        assert_eq!(store.get(&id).unwrap().showing, Showing::Raw);
    }

    #[test]
    fn editor_follows_pasted_text_and_llm() {
        for (raw, pasted, llm, want) in [
            ("a", "a", false, Editor::None),
            ("a", "a", true, Editor::None),
            ("a", "A.", true, Editor::Llm),
            ("a", "A.", false, Editor::Rules),
        ] {
            assert_eq!(
                editor_for(raw, pasted, llm),
                want,
                "{raw:?} {pasted:?} {llm}"
            );
        }
    }

    #[test]
    fn save_links_new_dictation() {
        let env = scratch();
        let store = env.store();
        let conn = history();

        let saved = HistoryManager::save_entry_with(
            &conn,
            Some(&store),
            llm_entry("fala-1.wav"),
            Language::En,
            1_790_000_000,
        )
        .unwrap();

        assert_eq!(rows(&conn), 1);
        let id = link(&conn, saved.id).expect("the new row is linked");
        assert_eq!(saved.dictation_id.as_deref(), Some(id.as_str()));
        let record = store.get(&id).unwrap();
        assert_eq!(record.dictation.raw.text, "acao de amanha");
        assert_eq!(record.dictation.final_text, "Ação de amanhã.");
        assert_eq!(record.dictation.editor, Editor::Llm);
        assert_eq!(record.dictation.raw.language, Language::En);
        assert_eq!(record.dictation.app.app_name.as_deref(), Some("notepad"));
        assert_eq!(record.showing, Showing::Final);
    }

    #[test]
    fn language_falls_back_to_pt_br() {
        for (setting, want) in [
            ("pt-BR", Language::PtBr),
            ("pt", Language::PtBr),
            ("en", Language::En),
            ("auto", Language::PtBr),
            ("es", Language::PtBr),
        ] {
            assert_eq!(language_from_setting(setting), want, "{setting}");
        }
    }

    #[test]
    fn save_without_store_keeps_row_unlinked() {
        let conn = history();
        let saved = save(&conn, None, llm_entry("fala-1.wav"), 1);
        assert_eq!(saved.transcription_text, "acao de amanha");
        assert_eq!(saved.dictation_id, None);
        assert_eq!(link(&conn, saved.id), None);

        let env = scratch();
        let store = env.store();
        env.break_store();
        let saved = save(&conn, Some(&store), llm_entry("fala-2.wav"), 2);
        assert_eq!(saved.transcription_text, "acao de amanha");
        assert_eq!(saved.dictation_id, None);
        assert_eq!(link(&conn, saved.id), None);
        assert_eq!(rows(&conn), 2);
    }

    #[test]
    fn save_mirror_failure_links_reported_id() {
        let env = scratch();
        fs::create_dir_all(&env.notes).unwrap();
        fs::write(env.notes.join("Ditados"), "não é pasta").unwrap();
        let store = env.store();
        let conn = history();

        let saved = save(
            &conn,
            Some(&store),
            entry("fala-1.wav", "reunião", "reunião", false, None, None),
            1,
        );

        let id = link(&conn, saved.id).expect("linked despite the mirror failure");
        assert_eq!(store.get(&id).unwrap().dictation.raw.text, "reunião");
    }

    #[test]
    fn failed_transcription_saves_unlinked_row() {
        let env = scratch();
        let store = env.store();
        let conn = history();

        let saved = save(
            &conn,
            Some(&store),
            entry("fala-1.wav", "", "", true, None, None),
            1,
        );

        assert_eq!(saved.transcription_text, "");
        assert_eq!(link(&conn, saved.id), None);
        assert!(store.search("", 10).unwrap().is_empty());
    }

    #[test]
    fn backfill_maps_rows_by_door_four() {
        let env = scratch();
        let store = env.store();
        let conn = history();
        let plain = insert_legacy(&conn, 1_790_000_100, "so bruto", None, false);
        let llm = insert_legacy(&conn, 1_790_000_200, "bruto llm", Some("Bruto LLM."), true);
        let rules = insert_legacy(&conn, 1_790_000_300, "ni hao", Some("你好"), false);
        let empty = insert_legacy(&conn, 1_790_000_400, "", None, false);

        assert_eq!(backfill(&conn, &store, Language::En).unwrap(), 3);

        for (row, ts, raw, final_text, editor) in [
            (plain, 1_790_000_100, "so bruto", "so bruto", Editor::None),
            (llm, 1_790_000_200, "bruto llm", "Bruto LLM.", Editor::Llm),
            (rules, 1_790_000_300, "ni hao", "你好", Editor::Rules),
        ] {
            let id = link(&conn, row).unwrap_or_else(|| panic!("row {row} linked"));
            let record = store.get(&id).unwrap();
            assert_eq!(record.dictation.raw.text, raw);
            assert_eq!(record.dictation.final_text, final_text);
            assert_eq!(record.dictation.editor, editor, "row {row}");
            assert_eq!(record.dictation.app.app_name, None);
            assert_eq!(record.dictation.raw.language, Language::En);
            assert_eq!(record.created_at.timestamp(), ts);
            let local = Local.timestamp_opt(ts, 0).unwrap().offset().fix();
            assert_eq!(*record.created_at.offset(), local);
        }
        assert_eq!(link(&conn, empty), None);
    }

    #[test]
    fn backfill_is_idempotent() {
        let env = scratch();
        let store = env.store();
        let conn = history();
        let a = insert_legacy(&conn, 10, "um", None, false);
        let b = insert_legacy(&conn, 20, "dois", Some("Dois."), true);
        let c = insert_legacy(&conn, 30, "tres", None, false);

        backfill(&conn, &store, Language::PtBr).unwrap();
        let first: Vec<_> = [a, b, c].iter().map(|id| link(&conn, *id)).collect();
        assert_eq!(backfill(&conn, &store, Language::PtBr).unwrap(), 0);
        let second: Vec<_> = [a, b, c].iter().map(|id| link(&conn, *id)).collect();

        assert_eq!(store.search("", 100).unwrap().len(), 3);
        assert_eq!(first, second);
    }

    #[test]
    fn open_store_failure_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arquivo");
        fs::write(&file, "não é pasta").unwrap();
        assert!(open_store(&file.join("fala.sqlite"), &dir.path().join("notas")).is_none());
        assert!(open_store(&dir.path().join("fala.sqlite"), &dir.path().join("notas")).is_some());
    }

    #[test]
    fn delete_linked_removes_dictation_row_and_wav() {
        let env = scratch();
        let store = env.store();
        let conn = history();
        let wav = env.wav("fala-1.wav");
        let saved = save(&conn, Some(&store), llm_entry("fala-1.wav"), 1);
        let id = link(&conn, saved.id).unwrap();
        let md = env.md_of(&id).expect("the dictation has a mirror");

        HistoryManager::delete_entry_with(&conn, Some(&store), &env.recordings, saved.id).unwrap();

        assert!(not_found(&store, &id));
        assert!(!md.exists());
        assert_eq!(rows(&conn), 0);
        assert!(!wav.exists());
    }

    #[test]
    fn delete_failure_keeps_row_and_wav() {
        let env = scratch();
        let store = env.store();
        let conn = history();
        let wav = env.wav("fala-1.wav");
        let saved = save(&conn, Some(&store), llm_entry("fala-1.wav"), 1);
        env.break_store();

        assert!(
            HistoryManager::delete_entry_with(&conn, Some(&store), &env.recordings, saved.id)
                .is_err()
        );
        assert_eq!(rows(&conn), 1);
        assert!(wav.exists());

        let env = scratch();
        let store = env.store();
        let conn = history();
        let wav = env.wav("fala-2.wav");
        let saved = save(&conn, Some(&store), llm_entry("fala-2.wav"), 2);
        store.delete(&link(&conn, saved.id).unwrap()).unwrap();

        HistoryManager::delete_entry_with(&conn, Some(&store), &env.recordings, saved.id).unwrap();
        assert_eq!(rows(&conn), 0);
        assert!(!wav.exists());
    }

    #[test]
    fn retention_deletes_dictations_or_keeps_entry() {
        let env = scratch();
        let store = env.store();
        let conn = history();
        let mut ids = Vec::new();
        for ts in [10, 20, 30] {
            let file = format!("fala-{ts}.wav");
            env.wav(&file);
            let saved = save(&conn, Some(&store), llm_entry(&file), ts);
            ids.push(link(&conn, saved.id).unwrap());
        }

        HistoryManager::cleanup_by_count_with(&conn, Some(&store), &env.recordings, 1).unwrap();
        assert_eq!(rows(&conn), 1);
        assert!(not_found(&store, &ids[0]));
        assert!(not_found(&store, &ids[1]));
        assert!(store.get(&ids[2]).is_ok());

        HistoryManager::cleanup_by_time_with(&conn, Some(&store), &env.recordings, 31).unwrap();
        assert_eq!(rows(&conn), 0);
        assert!(not_found(&store, &ids[2]));

        let env = scratch();
        let store = env.store();
        let conn = history();
        for ts in [10, 20, 30] {
            let file = format!("fala-{ts}.wav");
            env.wav(&file);
            save(&conn, Some(&store), llm_entry(&file), ts);
        }
        env.break_store();
        HistoryManager::cleanup_by_count_with(&conn, Some(&store), &env.recordings, 1).unwrap();
        assert_eq!(rows(&conn), 3);
        HistoryManager::cleanup_by_time_with(&conn, Some(&store), &env.recordings, 100).unwrap();
        assert_eq!(rows(&conn), 3);
    }

    #[test]
    fn retry_replaces_dictation_keeping_app() {
        let env = scratch();
        let store = env.store();
        let conn = history();
        let saved = save(
            &conn,
            Some(&store),
            entry(
                "fala-1.wav",
                "velho",
                "Velho.",
                true,
                Some("Velho."),
                Some("slack"),
            ),
            1,
        );
        let old = link(&conn, saved.id).unwrap();

        HistoryManager::update_transcription_with(
            &conn,
            Some(&store),
            saved.id,
            EntryTexts {
                transcription_text: "novo".to_string(),
                post_processed_text: Some("Novo.".to_string()),
                post_process_prompt: None,
                pasted_text: "Novo.".to_string(),
                llm_produced: true,
            },
            Language::PtBr,
        )
        .unwrap();

        let new = link(&conn, saved.id).unwrap();
        assert_ne!(new, old);
        let record = store.get(&new).unwrap();
        assert_eq!(record.dictation.raw.text, "novo");
        assert_eq!(record.dictation.final_text, "Novo.");
        assert_eq!(record.dictation.editor, Editor::Llm);
        assert_eq!(record.dictation.app.app_name.as_deref(), Some("slack"));
        assert!(not_found(&store, &old));
    }

    #[test]
    fn undo_then_redo_switches_showing() {
        let env = scratch();
        let store = env.store();
        let conn = history();
        let saved = save(&conn, Some(&store), llm_entry("fala-1.wav"), 1);
        let id = link(&conn, saved.id).unwrap();

        let undone =
            HistoryManager::set_showing_with(&conn, Some(&store), saved.id, Showing::Raw).unwrap();
        assert_eq!(undone.dictation.unwrap().showing, HistoryShowing::Raw);
        assert_eq!(store.get(&id).unwrap().showing, Showing::Raw);
        let md = fs::read_to_string(env.md_of(&id).unwrap()).unwrap();
        assert!(md.contains("showing: \"raw\""), "{md}");

        let redone =
            HistoryManager::set_showing_with(&conn, Some(&store), saved.id, Showing::Final)
                .unwrap();
        assert_eq!(redone.dictation.unwrap().showing, HistoryShowing::Final);
        assert_eq!(store.get(&id).unwrap().showing, Showing::Final);
        let md = fs::read_to_string(env.md_of(&id).unwrap()).unwrap();
        assert!(md.contains("showing: \"final\""), "{md}");
    }

    #[test]
    fn undo_errors_change_nothing() {
        let env = scratch();
        let store = env.store();
        let conn = history();

        // Id inexistente.
        assert!(HistoryManager::set_showing_with(&conn, Some(&store), 999, Showing::Raw).is_err());

        // Entrada sem vínculo.
        let unlinked = save(&conn, None, llm_entry("fala-1.wav"), 1);
        assert!(
            HistoryManager::set_showing_with(&conn, Some(&store), unlinked.id, Showing::Raw)
                .is_err()
        );

        // Item sem edição.
        let plain = save(
            &conn,
            Some(&store),
            entry("fala-2.wav", "oi", "oi", false, None, None),
            2,
        );
        let plain_id = link(&conn, plain.id).unwrap();
        let plain_md = fs::read_to_string(env.md_of(&plain_id).unwrap()).unwrap();
        assert!(
            HistoryManager::set_showing_with(&conn, Some(&store), plain.id, Showing::Raw).is_err()
        );
        assert_eq!(store.get(&plain_id).unwrap().showing, Showing::Final);
        assert_eq!(
            fs::read_to_string(env.md_of(&plain_id).unwrap()).unwrap(),
            plain_md
        );

        // Store fechado.
        let edited = save(&conn, Some(&store), llm_entry("fala-3.wav"), 3);
        let edited_id = link(&conn, edited.id).unwrap();
        assert!(HistoryManager::set_showing_with(&conn, None, edited.id, Showing::Raw).is_err());
        assert_eq!(store.get(&edited_id).unwrap().showing, Showing::Final);
    }

    #[test]
    fn entries_carry_their_dictation() {
        let env = scratch();
        let store = env.store();
        let conn = history();
        save(&conn, Some(&store), llm_entry("fala-1.wav"), 1);
        save(&conn, None, llm_entry("fala-2.wav"), 2);

        let page = HistoryManager::page_with(&conn, Some(&store), None, Some(10)).unwrap();

        assert_eq!(page.entries.len(), 2);
        assert_eq!(page.entries[0].file_name, "fala-2.wav");
        assert_eq!(page.entries[0].dictation, None);
        assert_eq!(
            page.entries[1].dictation,
            Some(HistoryDictation {
                raw_text: "acao de amanha".to_string(),
                final_text: "Ação de amanhã.".to_string(),
                editor: HistoryEditor::Llm,
                showing: HistoryShowing::Final,
                app_name: Some("notepad".to_string()),
            })
        );
    }
}
