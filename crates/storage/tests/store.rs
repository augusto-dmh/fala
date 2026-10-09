//! Gravar, buscar, desfazer e reaplicar pelo `Store`, conferindo banco e espelho.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs;
use std::thread;
use std::time::Duration;

use common::{at, dictation, env, md_of, unedited};
use fala_core::Editor;
use fala_storage::{Showing, StorageError, Store};
use rusqlite::Connection;

#[test]
fn add_returns_v7_record_showing_final() {
    let env = env("add_returns_v7_record_showing_final");
    let store = env.open();
    let d = dictation(
        "acao de amanha",
        "Ação de amanhã.",
        Editor::Llm,
        Some("Slack"),
    );
    let record = store.add(&d, at(14, 30, 22)).unwrap();

    let uuid = uuid::Uuid::parse_str(&record.id).unwrap();
    assert_eq!(uuid.get_version_num(), 7);
    assert_eq!(record.id, uuid.hyphenated().to_string());
    assert_eq!(record.created_at, at(14, 30, 22));
    assert_eq!(record.dictation, d);
    assert_eq!(record.showing, Showing::Final);
    assert_eq!(store.get(&record.id).unwrap(), record);
}

#[test]
fn mirror_failure_keeps_row() {
    let env = env("mirror_failure_keeps_row");
    fs::create_dir_all(&env.notes).unwrap();
    // `Ditados` como arquivo comum: a pasta do dia não pode ser criada.
    fs::write(env.ditados(), "não é pasta").unwrap();
    let store = env.open();

    let err = store
        .add(&unedited("reunião às três"), at(9, 0, 0))
        .unwrap_err();
    let StorageError::Mirror { id, .. } = err else {
        panic!("esperava Mirror, veio {err:?}");
    };
    let record = store.get(&id).unwrap();
    assert_eq!(record.dictation.final_text, "reunião às três");
    let hits = store.search("reuniao", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, id);
}

#[test]
fn search_ignores_accents() {
    let env = env("search_ignores_accents");
    let store = env.open();
    let r = store
        .add(
            &dictation("x", "Ação de amanhã.", Editor::Rules, None),
            at(10, 0, 0),
        )
        .unwrap();
    store.add(&unedited("outra coisa"), at(10, 0, 1)).unwrap();

    let hits = store.search("acao", 10).unwrap();
    assert_eq!(hits.iter().map(|h| &h.id).collect::<Vec<_>>(), [&r.id]);
}

#[test]
fn search_matches_raw_only_word() {
    let env = env("search_matches_raw_only_word");
    let store = env.open();
    let r = store
        .add(
            &dictation(
                "então tipo manda o relatório",
                "Manda o relatório.",
                Editor::Llm,
                None,
            ),
            at(10, 0, 0),
        )
        .unwrap();
    let hits = store.search("tipo", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, r.id);
}

#[test]
fn search_terms_are_anded_prefixes() {
    let env = env("search_terms_are_anded_prefixes");
    let store = env.open();
    let both = store
        .add(&unedited("reunião de amanhã cedo"), at(10, 0, 0))
        .unwrap();
    store
        .add(&unedited("reunião de ontem"), at(10, 0, 1))
        .unwrap();
    store
        .add(&unedited("amanhã eu vejo"), at(10, 0, 2))
        .unwrap();

    let hits = store.search("reuniao amanh", 10).unwrap();
    assert_eq!(hits.iter().map(|h| &h.id).collect::<Vec<_>>(), [&both.id]);
}

#[test]
fn search_fts_syntax_is_literal() {
    let env = env("search_fts_syntax_is_literal");
    let store = env.open();
    let cases = [
        ("ele disse \"oi\" ontem", "\"oi\""),
        ("tarefa urgente agora", "urgente*"),
        ("(reunião) cancelada", "(reunião"),
        ("NEAR do escritório", "NEAR"),
        ("bom-dia pessoal", "bom-dia"),
    ];
    let mut ids = Vec::new();
    for (i, (text, _)) in cases.iter().enumerate() {
        ids.push(store.add(&unedited(text), at(11, 0, i as u32)).unwrap().id);
    }
    for (i, (_, query)) in cases.iter().enumerate() {
        let hits = store
            .search(query, 10)
            .unwrap_or_else(|e| panic!("{query:?} deu erro: {e}"));
        assert!(
            hits.iter().any(|h| h.id == ids[i]),
            "{query:?} não achou o item {i}"
        );
    }
    for query in ["\"", "*", "(", "-", "NEAR(", "a AND", "OR"] {
        assert!(store.search(query, 10).is_ok(), "{query:?} deu erro");
    }
}

#[test]
fn search_empty_returns_recent() {
    let env = env("search_empty_returns_recent");
    let store = env.open();
    let a = store.add(&unedited("um"), at(8, 0, 0)).unwrap();
    let b = store.add(&unedited("dois"), at(8, 0, 1)).unwrap();
    let c = store.add(&unedited("três"), at(8, 0, 2)).unwrap();
    for query in ["", "   "] {
        let hits = store.search(query, 2).unwrap();
        assert_eq!(
            hits.iter().map(|h| &h.id).collect::<Vec<_>>(),
            [&c.id, &b.id]
        );
    }
    assert_eq!(store.search("", 10).unwrap().len(), 3);
    let _ = a;
}

#[test]
fn search_orders_newest_first_and_limits() {
    let env = env("search_orders_newest_first_and_limits");
    let store = env.open();
    let order = [3, 1, 4, 0, 2];
    let mut by_minute = vec![String::new(); 5];
    for m in order {
        let r = store
            .add(&unedited(&format!("nota número {m}")), at(12, m, 0))
            .unwrap();
        by_minute[m as usize] = r.id;
    }
    let all = store.search("nota", 10).unwrap();
    let got: Vec<_> = all.iter().map(|h| h.id.clone()).collect();
    let want: Vec<_> = by_minute.iter().rev().cloned().collect();
    assert_eq!(got, want);
    let two = store.search("nota", 2).unwrap();
    assert_eq!(
        two.iter().map(|h| h.id.clone()).collect::<Vec<_>>(),
        want[..2]
    );
}

#[test]
fn open_sets_wal_timeout_and_version() {
    let env = env("open_sets_wal_timeout_and_version");
    let store = env.open();
    assert_eq!(store.busy_timeout_ms().unwrap(), 5000);

    let conn = Connection::open(&env.db).unwrap();
    let mode: String = conn
        .pragma_query_value(None, "journal_mode", |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    let version: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(version, 1);

    // Um escritor segurando o lock por 300 ms não faz o `add` falhar: ele espera.
    let db = env.db.clone();
    let (locked, wait_lock) = std::sync::mpsc::channel();
    let handle = thread::spawn(move || {
        let mut other = Connection::open(&db).unwrap();
        let tx = other
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();
        locked.send(()).unwrap();
        thread::sleep(Duration::from_millis(300));
        tx.commit().unwrap();
    });
    wait_lock.recv().unwrap();
    store
        .add(&unedited("esperou o lock"), at(13, 0, 0))
        .unwrap();
    handle.join().unwrap();
}

#[test]
fn undo_shows_raw_and_keeps_final() {
    let env = env("undo_shows_raw_and_keeps_final");
    let store = env.open();
    for (i, editor) in [Editor::Rules, Editor::Llm].into_iter().enumerate() {
        let r = store
            .add(
                &dictation("acho que sim", "Acho que sim.", editor, None),
                at(15, 0, i as u32),
            )
            .unwrap();
        let undone = store.undo(&r.id).unwrap();
        assert_eq!(undone.showing, Showing::Raw);
        assert_eq!(undone.dictation.final_text, "Acho que sim.");
        assert_eq!(undone.shown_text(), "acho que sim");
        assert_eq!(store.get(&r.id).unwrap(), undone);
        let md = fs::read_to_string(md_of(&env, &r.id)).unwrap();
        assert!(md.contains("\nshowing: \"raw\"\n"), "{md}");
        assert!(md.ends_with("---\nAcho que sim.\n"), "{md}");
    }
}

#[test]
fn redo_shows_final_again() {
    let env = env("redo_shows_final_again");
    let store = env.open();
    let r = store
        .add(
            &dictation("acho que sim", "Acho que sim.", Editor::Llm, None),
            at(15, 0, 0),
        )
        .unwrap();
    store.undo(&r.id).unwrap();
    let redone = store.redo(&r.id).unwrap();
    assert_eq!(redone.showing, Showing::Final);
    assert_eq!(redone.shown_text(), "Acho que sim.");
    assert_eq!(store.get(&r.id).unwrap().showing, Showing::Final);
    let md = fs::read_to_string(md_of(&env, &r.id)).unwrap();
    assert!(md.contains("\nshowing: \"final\"\n"), "{md}");
}

#[test]
fn undo_and_redo_are_idempotent() {
    let env = env("undo_and_redo_are_idempotent");
    let store = env.open();
    let r = store
        .add(
            &dictation("bruto", "Final.", Editor::Llm, None),
            at(15, 0, 0),
        )
        .unwrap();
    let path = md_of(&env, &r.id);

    let md_final = fs::read_to_string(&path).unwrap();
    assert_eq!(store.redo(&r.id).unwrap(), r);
    assert_eq!(fs::read_to_string(&path).unwrap(), md_final);

    let once = store.undo(&r.id).unwrap();
    let md_raw = fs::read_to_string(&path).unwrap();
    assert_eq!(store.undo(&r.id).unwrap(), once);
    assert_eq!(fs::read_to_string(&path).unwrap(), md_raw);
}

#[test]
fn unedited_has_nothing_to_undo() {
    let env = env("unedited_has_nothing_to_undo");
    let store = env.open();
    let r = store.add(&unedited("sem edição"), at(15, 0, 0)).unwrap();
    let path = md_of(&env, &r.id);
    let before = fs::read_to_string(&path).unwrap();
    assert!(matches!(
        store.undo(&r.id),
        Err(StorageError::NothingToUndo(id)) if id == r.id
    ));
    assert!(matches!(
        store.redo(&r.id),
        Err(StorageError::NothingToUndo(_))
    ));
    assert_eq!(store.get(&r.id).unwrap(), r);
    assert_eq!(fs::read_to_string(&path).unwrap(), before);
}

#[test]
fn unknown_id_is_not_found() {
    let env = env("unknown_id_is_not_found");
    let store = env.open();
    store.add(&unedited("existe"), at(15, 0, 0)).unwrap();
    let missing = "0199a3f2-5c1e-7b3a-9d4e-2f6a8c0b1e27";
    assert!(matches!(store.get(missing), Err(StorageError::NotFound(id)) if id == missing));
    assert!(matches!(
        store.undo(missing),
        Err(StorageError::NotFound(_))
    ));
    assert!(matches!(
        store.redo(missing),
        Err(StorageError::NotFound(_))
    ));
}

#[test]
fn open_under_a_file_is_io_error() {
    let env = env("open_under_a_file_is_io_error");
    let file = env.dir.join("arquivo");
    fs::write(&file, "x").unwrap();
    let err = Store::open(&file.join("fala.sqlite"), &env.notes).unwrap_err();
    assert!(matches!(err, StorageError::Io(_)), "{err:?}");
}

#[test]
fn editor_literals_in_db() {
    let env = env("editor_literals_in_db");
    let store = env.open();
    let mut ids = Vec::new();
    for (i, editor) in [Editor::None, Editor::Rules, Editor::Llm]
        .into_iter()
        .enumerate()
    {
        let d = dictation("a", "b", editor, None);
        ids.push(store.add(&d, at(16, 0, i as u32)).unwrap().id);
    }
    let conn = Connection::open(&env.db).unwrap();
    for (id, want) in ids.iter().zip(["none", "rules", "llm"]) {
        let got: String = conn
            .query_row(
                "SELECT edited_by FROM dictations WHERE id = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(got, want);
        let showing: String = conn
            .query_row("SELECT showing FROM dictations WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(showing, "final");
    }
}

#[test]
fn leaves_history_db_alone() {
    let env = env("leaves_history_db_alone");
    let history = env.dir.join("history.db");
    {
        let conn = Connection::open(&history).unwrap();
        conn.execute_batch(
            "CREATE TABLE transcription_history (id INTEGER PRIMARY KEY, transcription_text TEXT);
             INSERT INTO transcription_history (transcription_text) VALUES ('antigo');
             PRAGMA user_version = 4;",
        )
        .unwrap();
    }
    let before = fs::read(&history).unwrap();
    let store = env.open();
    store.add(&unedited("novo"), at(17, 0, 0)).unwrap();
    drop(store);
    assert_eq!(fs::read(&history).unwrap(), before);
    assert!(env.db.exists());
}

#[test]
fn fts_table_shape() {
    let env = env("fts_table_shape");
    drop(env.open());
    let conn = Connection::open(&env.db).unwrap();
    let sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name = 'dictations_fts'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(sql.contains("fts5(raw, final"), "{sql}");
    assert!(sql.contains("content='dictations'"), "{sql}");
    assert!(sql.contains("content_rowid='rowid'"), "{sql}");
    assert!(
        sql.contains("tokenize='unicode61 remove_diacritics 2'"),
        "{sql}"
    );
}

#[test]
fn delete_removes_row_fts_and_mirror() {
    let env = env("delete_removes_row_fts_and_mirror");
    let store = env.open();
    let kept = store.add(&unedited("caju maduro"), at(11, 0, 0)).unwrap();
    let gone = store
        .add(&unedited("jabuticaba no pé"), at(11, 0, 1))
        .unwrap();
    let gone_md = md_of(&env, &gone.id);

    store.delete(&gone.id).unwrap();

    assert!(matches!(store.get(&gone.id), Err(StorageError::NotFound(id)) if id == gone.id));
    assert!(store.search("jabuticaba", 10).unwrap().is_empty());
    assert!(!gone_md.exists(), "o .md apagado ainda existe: {gone_md:?}");
    assert_eq!(store.get(&kept.id).unwrap(), kept);
    assert!(md_of(&env, &kept.id).exists());
}

#[test]
fn delete_unknown_id_is_not_found() {
    let env = env("delete_unknown_id_is_not_found");
    let store = env.open();
    let kept = store.add(&unedited("pitanga"), at(11, 5, 0)).unwrap();
    let kept_md = fs::read_to_string(md_of(&env, &kept.id)).unwrap();
    let unknown = uuid::Uuid::now_v7().hyphenated().to_string();

    let err = store.delete(&unknown).unwrap_err();

    assert!(
        matches!(err, StorageError::NotFound(ref id) if *id == unknown),
        "{err:?}"
    );
    assert_eq!(store.get(&kept.id).unwrap(), kept);
    assert_eq!(fs::read_to_string(md_of(&env, &kept.id)).unwrap(), kept_md);
    assert_eq!(store.search("pitanga", 10).unwrap().len(), 1);
}

#[test]
fn reindex_after_delete_does_not_resurrect() {
    let env = env("reindex_after_delete_does_not_resurrect");
    let mut store = env.open();
    store.add(&unedited("acerola"), at(11, 10, 0)).unwrap();
    let gone = store.add(&unedited("graviola"), at(11, 10, 1)).unwrap();
    store.delete(&gone.id).unwrap();

    let report = store.reindex().unwrap();

    assert_eq!(report.indexed, 1);
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
    assert!(matches!(
        store.get(&gone.id),
        Err(StorageError::NotFound(_))
    ));
}

#[test]
fn delete_tolerates_missing_mirror() {
    let env = env("delete_tolerates_missing_mirror");
    let store = env.open();
    let gone = store.add(&unedited("umbu"), at(11, 15, 0)).unwrap();
    fs::remove_file(md_of(&env, &gone.id)).unwrap();

    store.delete(&gone.id).unwrap();

    assert!(matches!(
        store.get(&gone.id),
        Err(StorageError::NotFound(_))
    ));
}

#[test]
fn apply_late_edit_sets_final_editor_and_showing() {
    let env = env("apply_late_edit_sets_final_editor_and_showing");
    let store = env.open();
    let before = store
        .add_sensitive(
            &dictation(
                "a charge bee cobra",
                "A ChargeBee cobra",
                Editor::Rules,
                Some("notepad"),
            ),
            at(16, 0, 0),
        )
        .unwrap();

    let applied = store.apply_late_edit(&before.id, "texto do llm").unwrap();

    assert_eq!(applied.dictation.final_text, "texto do llm");
    assert_eq!(applied.dictation.editor, Editor::Llm);
    assert_eq!(applied.showing, Showing::Raw);
    assert_eq!(applied.id, before.id);
    assert_eq!(applied.dictation.raw, before.dictation.raw);
    assert_eq!(applied.dictation.app, before.dictation.app);
    assert_eq!(applied.created_at, before.created_at);
    assert!(applied.sensitive);
    assert_eq!(store.get(&before.id).unwrap(), applied);
}

#[test]
fn apply_late_edit_rewrites_mirror_and_survives_reindex() {
    let env = env("apply_late_edit_rewrites_mirror_and_survives_reindex");
    let mut store = env.open();
    let before = store
        .add(
            &dictation("bruto do asr", "Bruto do ASR.", Editor::Rules, None),
            at(16, 5, 0),
        )
        .unwrap();

    let applied = store.apply_late_edit(&before.id, "Texto do LLM.").unwrap();

    let md = fs::read_to_string(md_of(&env, &before.id)).unwrap();
    assert!(md.contains("\nedited_by: \"llm\"\n"), "{md}");
    assert!(md.contains("\nshowing: \"raw\"\n"), "{md}");
    assert!(md.ends_with("---\nTexto do LLM.\n"), "{md}");

    store.reindex().unwrap();
    assert_eq!(store.get(&before.id).unwrap(), applied);
}

#[test]
fn apply_late_edit_rejects_unknown_id_and_blank_text() {
    let env = env("apply_late_edit_rejects_unknown_id_and_blank_text");
    let store = env.open();
    let r = store
        .add(
            &dictation("bruto", "Bruto.", Editor::Rules, None),
            at(16, 10, 0),
        )
        .unwrap();
    let path = md_of(&env, &r.id);
    let md_before = fs::read_to_string(&path).unwrap();

    assert!(matches!(
        store.apply_late_edit("nao-existe", "texto do llm"),
        Err(StorageError::NotFound(id)) if id == "nao-existe"
    ));
    assert_eq!(store.get(&r.id).unwrap(), r);
    assert_eq!(fs::read_to_string(&path).unwrap(), md_before);
    assert_eq!(common::files_under(&env.ditados()).len(), 1);

    for blank in ["", "   "] {
        assert!(
            matches!(store.apply_late_edit(&r.id, blank), Err(StorageError::EmptyEdit(id)) if id == r.id),
            "{blank:?}"
        );
        assert_eq!(store.get(&r.id).unwrap(), r, "{blank:?}");
        assert_eq!(fs::read_to_string(&path).unwrap(), md_before, "{blank:?}");
    }

    Connection::open(&env.db)
        .unwrap()
        .execute_batch("DROP TABLE dictations;")
        .unwrap();
    assert!(matches!(
        store.apply_late_edit(&r.id, "texto do llm"),
        Err(StorageError::Db(_))
    ));
    assert_eq!(fs::read_to_string(&path).unwrap(), md_before);
}
