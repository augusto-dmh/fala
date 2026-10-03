//! O espelho Markdown: caminho, frontmatter literal, texto hostil e escrita atômica.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs;

use common::{at, delete_db, dictation, env, files_under, md_of};
use fala_core::Editor;

#[test]
fn mirror_path_and_frontmatter_literal() {
    let env = env("mirror_path_and_frontmatter_literal");
    let store = env.open();
    let with_app = store
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
    let no_app = store
        .add(&dictation("oi", "Oi.", Editor::Rules, None), at(9, 5, 7))
        .unwrap();

    let path = env
        .ditados()
        .join("2026-10-02")
        .join(format!("143022-{}.md", with_app.id));
    let want = format!(
        "---\n\
         id: \"{}\"\n\
         created_at: \"2026-10-02T14:30:22-03:00\"\n\
         language: \"pt-BR\"\n\
         app: \"Slack\"\n\
         edited_by: \"llm\"\n\
         showing: \"final\"\n\
         raw: \"acao de amanha\"\n\
         ---\n\
         Ação de amanhã.\n",
        with_app.id
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), want);

    let path = env
        .ditados()
        .join("2026-10-02")
        .join(format!("090507-{}.md", no_app.id));
    let md = fs::read_to_string(&path).unwrap();
    assert!(md.contains("\napp: null\n"), "{md}");
    assert!(md.contains("\nedited_by: \"rules\"\n"), "{md}");
    assert!(md.ends_with("---\nOi.\n"), "{md}");
}

#[test]
fn hostile_text_round_trips() {
    let env = env("hostile_text_round_trips");
    let store = env.open();
    let cases = [
        "---",
        "## x",
        "ele disse \"aspas\" e 'simples'",
        "linha um\nlinha dois",
        "windows\r\nquebra",
        "ação, coração, pão e você",
        "termina com quebra\n",
        "",
        "---\n## x\n\"aspas\"\n---\n",
    ];
    let mut ids = Vec::new();
    for (i, text) in cases.iter().enumerate() {
        // O caso no bruto e no final, e o caso só no bruto com um final comum.
        let both = dictation(text, text, Editor::Llm, Some("Editor | x"));
        let raw_only = dictation(text, "Final comum.", Editor::Rules, None);
        ids.push(store.add(&both, at(10, i as u32, 0)).unwrap());
        ids.push(store.add(&raw_only, at(11, i as u32, 0)).unwrap());
    }
    drop(store);
    delete_db(&env);

    let mut store = env.open();
    let report = store.reindex().unwrap();
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
    assert_eq!(report.indexed, ids.len());
    for record in &ids {
        assert_eq!(&store.get(&record.id).unwrap(), record);
    }
}

#[test]
fn no_tmp_left_behind() {
    let env = env("no_tmp_left_behind");
    let mut store = env.open();
    let r = store
        .add(
            &dictation("bruto", "Final.", Editor::Llm, None),
            at(8, 0, 0),
        )
        .unwrap();
    store.undo(&r.id).unwrap();
    store.redo(&r.id).unwrap();
    let tmps = |env: &common::Env| {
        files_under(&env.ditados())
            .into_iter()
            .filter(|p| p.to_string_lossy().ends_with(".tmp"))
            .count()
    };
    assert_eq!(tmps(&env), 0);

    // Um `.tmp` que um crash deixou para trás não muda o `.md` final nem a próxima escrita.
    let md = md_of(&env, &r.id);
    let good = fs::read_to_string(&md).unwrap();
    fs::write(format!("{}.tmp", md.display()), "---\nlixo parcial").unwrap();
    assert_eq!(fs::read_to_string(&md).unwrap(), good);
    let report = store.reindex().unwrap();
    assert_eq!(report.indexed, 1);
    assert!(report.skipped.is_empty());
    store.undo(&r.id).unwrap();
    assert!(fs::read_to_string(&md)
        .unwrap()
        .contains("\nshowing: \"raw\"\n"));
    assert_eq!(tmps(&env), 0);
}
