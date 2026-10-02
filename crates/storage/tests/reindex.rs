//! `reindex`: reconstruir o banco a partir dos `.md` (confirmação da ADR-0006).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs;

use common::{at, delete_db, dictation, env, md_of, unedited};
use fala_core::{Editor, Language};
use fala_storage::{Showing, StorageError};
use rusqlite::Connection;

#[test]
fn reindex_restores_every_field() {
    let env = env("reindex_restores_every_field");
    let store = env.open();
    let a = store
        .add(
            &dictation(
                "acao de amanha",
                "Ação de amanhã.",
                Editor::Llm,
                Some("Slack"),
            ),
            at(14, 30, 22),
        )
        .unwrap();
    let mut en = dictation("hello there", "Hello there.", Editor::Rules, None);
    en.raw.language = Language::En;
    let b = store.add(&en, at(14, 31, 0)).unwrap();
    let b = store.undo(&b.id).unwrap();
    let c = store.add(&unedited("sem edição"), at(14, 32, 0)).unwrap();
    drop(store);
    delete_db(&env);
    assert!(!env.db.exists());

    // C44: o banco não existe; abrir e reindexar o cria.
    let mut store = env.open();
    let report = store.reindex().unwrap();
    assert!(env.db.exists());
    assert_eq!(report.indexed, 3);
    assert!(report.skipped.is_empty());
    for want in [&a, &b, &c] {
        assert_eq!(&store.get(&want.id).unwrap(), want);
    }
    assert_eq!(store.get(&b.id).unwrap().showing, Showing::Raw);
}

#[test]
fn delete_db_then_reindex_gives_same_search() {
    let env = env("delete_db_then_reindex_gives_same_search");
    let store = env.open();
    let texts = [
        (
            "reunião de amanhã cedo",
            "Reunião de amanhã cedo.",
            Editor::Llm,
        ),
        ("manda o relatório", "Manda o relatório.", Editor::Rules),
        ("acao pendente", "Ação pendente.", Editor::Llm),
        ("compra pão", "compra pão", Editor::None),
        (
            "ligar pro joão amanhã",
            "Ligar para o João amanhã.",
            Editor::Llm,
        ),
        (
            "relatório da reunião",
            "Relatório da reunião.",
            Editor::Rules,
        ),
    ];
    let mut ids = Vec::new();
    for (i, (raw, fin, editor)) in texts.iter().enumerate() {
        let app = (i % 2 == 0).then_some("Slack");
        ids.push(
            store
                .add(&dictation(raw, fin, *editor, app), at(9, i as u32, 0))
                .unwrap()
                .id,
        );
    }
    store.undo(&ids[0]).unwrap();
    store.undo(&ids[4]).unwrap();
    let queries = ["", "reuniao", "amanh", "relatorio reuniao"];
    let before: Vec<_> = queries
        .iter()
        .map(|q| store.search(q, 20).unwrap())
        .collect();
    assert!(before.iter().all(|hits| !hits.is_empty()), "{before:?}");
    drop(store);
    delete_db(&env);

    let mut store = env.open();
    store.reindex().unwrap();
    let after: Vec<_> = queries
        .iter()
        .map(|q| store.search(q, 20).unwrap())
        .collect();
    assert_eq!(after, before);
}

#[test]
fn language_round_trips() {
    let env = env("language_round_trips");
    let store = env.open();
    let pt = store.add(&unedited("olá"), at(8, 0, 0)).unwrap();
    let mut en = unedited("hello");
    en.raw.language = Language::En;
    let en = store.add(&en, at(8, 0, 1)).unwrap();

    let md = fs::read_to_string(md_of(&env, &pt.id)).unwrap();
    assert!(md.contains("\nlanguage: \"pt-BR\"\n"), "{md}");
    let md = fs::read_to_string(md_of(&env, &en.id)).unwrap();
    assert!(md.contains("\nlanguage: \"en\"\n"), "{md}");
    let conn = Connection::open(&env.db).unwrap();
    let stored: String = conn
        .query_row(
            "SELECT language FROM dictations WHERE id = ?1",
            [&en.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stored, "en");
    drop(conn);
    drop(store);
    delete_db(&env);

    let mut store = env.open();
    store.reindex().unwrap();
    assert_eq!(
        store.get(&pt.id).unwrap().dictation.raw.language,
        Language::PtBr
    );
    assert_eq!(
        store.get(&en.id).unwrap().dictation.raw.language,
        Language::En
    );
}

#[test]
fn invalid_files_are_skipped_with_reason() {
    let env = env("invalid_files_are_skipped_with_reason");
    let store = env.open();
    let good = store
        .add(&dictation("bom", "Bom.", Editor::Llm, None), at(8, 0, 0))
        .unwrap();
    let template = fs::read_to_string(md_of(&env, &good.id)).unwrap();
    drop(store);

    let id_line = format!("id: \"{}\"\n", good.id);
    let fake_id = |n: u32| format!("id: \"0199a3f2-5c1e-7b3a-9d4e-2f6a8c0b1e{n:02}\"\n");
    // Cada caso traz um trecho do motivo esperado, para provar que caiu na regra certa.
    let reasons = [
        ("sem-frontmatter", "sem frontmatter"),
        ("sem-fechamento", "fechamento"),
        ("sem-chave", "falta a chave `raw`"),
        ("nao-json", "não é JSON"),
        ("editor-invalido", "`edited_by`"),
        ("showing-invalido", "`showing`"),
        ("idioma-invalido", "`language`"),
    ];
    let cases: Vec<(&str, String)> = vec![
        ("sem-frontmatter", "Só um texto.\n".to_string()),
        (
            "sem-fechamento",
            template
                .replacen(&id_line, &fake_id(1), 1)
                .replace("---\nBom.\n", ""),
        ),
        (
            "sem-chave",
            template
                .replacen(&id_line, &fake_id(2), 1)
                .replace("raw: \"bom\"\n", ""),
        ),
        (
            "nao-json",
            template
                .replacen(&id_line, &fake_id(3), 1)
                .replace("raw: \"bom\"", "raw: bom sem aspas"),
        ),
        (
            "editor-invalido",
            template
                .replacen(&id_line, &fake_id(4), 1)
                .replace("edited_by: \"llm\"", "edited_by: \"gpt\""),
        ),
        (
            "showing-invalido",
            template
                .replacen(&id_line, &fake_id(5), 1)
                .replace("showing: \"final\"", "showing: \"undone\""),
        ),
        (
            "idioma-invalido",
            template
                .replacen(&id_line, &fake_id(6), 1)
                .replace("language: \"pt-BR\"", "language: \"es\""),
        ),
    ];
    let day = env.ditados().join("2026-10-02");
    for (name, content) in &cases {
        assert_ne!(content, &template, "{name} não mudou nada");
        fs::write(day.join(format!("{name}.md")), content).unwrap();
    }

    delete_db(&env);
    let mut store = env.open();
    let report = store.reindex().unwrap();
    assert_eq!(report.indexed, 1);
    assert_eq!(report.skipped.len(), cases.len(), "{:?}", report.skipped);
    for (name, _) in &cases {
        let skip = report
            .skipped
            .iter()
            .find(|s| s.path == day.join(format!("{name}.md")))
            .unwrap_or_else(|| panic!("{name} não aparece no relatório"));
        let want = reasons.iter().find(|(n, _)| n == name).unwrap().1;
        assert!(
            skip.reason.contains(want),
            "{name}: motivo {:?} não contém {want:?}",
            skip.reason
        );
    }
    assert_eq!(store.get(&good.id).unwrap(), good);
    assert_eq!(store.search("", 10).unwrap().len(), 1);
}

#[test]
fn duplicate_id_keeps_one() {
    let env = env("duplicate_id_keeps_one");
    let store = env.open();
    let r = store.add(&unedited("único"), at(8, 0, 0)).unwrap();
    drop(store);
    let md = md_of(&env, &r.id);
    let copy = env.ditados().join("2026-10-03");
    fs::create_dir_all(&copy).unwrap();
    fs::copy(&md, copy.join(md.file_name().unwrap())).unwrap();

    delete_db(&env);
    let mut store = env.open();
    let report = store.reindex().unwrap();
    assert_eq!(report.indexed, 1);
    assert_eq!(report.skipped.len(), 1);
    assert_eq!(report.skipped[0].reason, "id duplicado");
    assert_eq!(store.get(&r.id).unwrap(), r);
}

#[test]
fn reindex_is_one_transaction() {
    let env = env("reindex_is_one_transaction");
    let mut store = env.open();
    let a = store.add(&unedited("primeiro"), at(8, 0, 0)).unwrap();
    let b = store.add(&unedited("segundo"), at(8, 0, 1)).unwrap();
    let orphan = store.add(&unedited("órfão"), at(8, 0, 2)).unwrap();
    fs::remove_file(md_of(&env, &orphan.id)).unwrap();

    let conn = Connection::open(&env.db).unwrap();
    conn.execute_batch(&format!(
        "CREATE TRIGGER boom BEFORE INSERT ON dictations WHEN NEW.id = '{}'
         BEGIN SELECT RAISE(ABORT, 'falha injetada'); END;",
        b.id
    ))
    .unwrap();
    let err = store.reindex().unwrap_err();
    assert!(matches!(err, StorageError::Db(_)), "{err:?}");
    let ids = |store: &fala_storage::Store| {
        let mut ids: Vec<_> = store
            .search("", 10)
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect();
        ids.sort();
        ids
    };
    let mut want = vec![a.id.clone(), b.id.clone(), orphan.id.clone()];
    want.sort();
    assert_eq!(ids(&store), want);
    assert_eq!(store.get(&orphan.id).unwrap(), orphan);

    conn.execute_batch("DROP TRIGGER boom").unwrap();
    store.reindex().unwrap();
    let mut want = vec![a.id.clone(), b.id.clone()];
    want.sort();
    assert_eq!(ids(&store), want);
    assert!(matches!(
        store.get(&orphan.id),
        Err(StorageError::NotFound(_))
    ));
    assert!(store.search("orfao", 10).unwrap().is_empty());
}

#[test]
fn non_md_files_are_not_counted() {
    let env = env("non_md_files_are_not_counted");
    let store = env.open();
    let r = store.add(&unedited("válido"), at(8, 0, 0)).unwrap();
    drop(store);
    let day = env.ditados().join("2026-10-02");
    fs::write(day.join("notas.txt"), "texto solto").unwrap();
    fs::write(day.join("x.md.tmp"), "---\nlixo").unwrap();
    fs::write(day.join("y.MD.bak"), "---\nlixo").unwrap();

    delete_db(&env);
    let mut store = env.open();
    let report = store.reindex().unwrap();
    assert_eq!(report.indexed, 1);
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
    assert_eq!(store.get(&r.id).unwrap(), r);
}
