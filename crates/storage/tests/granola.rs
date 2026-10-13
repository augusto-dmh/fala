//! O export do Granola lido do disco e importado como reuniões: sessão `import` sem áudio, notas
//! marcadas como IA, trechos sem tempo, e importar de novo não duplica (`meeting_imports`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::io::Write;

use common::{at, env};
use fala_meeting::SessionMode;
use fala_storage::{
    read_granola_export, ImportOutcome, SegmentChannel, SegmentSpeaker, Store, GRANOLA,
};

const HEADER: &str =
    "document_id,document_title,workspace_name,document_created,summary,notes,transcript\n";

fn csv(title: &str) -> String {
    format!(
        "{HEADER}g-1,{title},Acme,2025-03-14 10:30:00 BST,\"### Próximos passos\n- Ana manda o contrato\",\"pauta: preço\",\"Me: Bom dia.\nAna: Bom dia, vamos ao preço?\nAna: Começo pelo contrato.\"\n\
         g-2,,Acme,2025-03-15T09:00:00Z,,,\n"
    )
}

fn import_all(store: &mut Store, path: &std::path::Path) -> Vec<ImportOutcome> {
    let export = read_granola_export(path).unwrap();
    export
        .meetings
        .iter()
        .map(|m| store.import_granola_meeting(m, at(21, 0, 0)).unwrap().1)
        .collect()
}

#[test]
fn an_imported_meeting_is_a_finished_session_without_audio() {
    let env = env("granola-import");
    let path = env.dir.join("granola.csv");
    std::fs::write(&path, csv("Preço com a Acme")).unwrap();
    let mut store = env.open();
    assert_eq!(
        import_all(&mut store, &path),
        [ImportOutcome::Created, ImportOutcome::Created]
    );

    let meetings = store.meetings().unwrap();
    assert_eq!(meetings.len(), 2);
    let m = meetings
        .iter()
        .find(|m| m.title == "Preço com a Acme")
        .unwrap();
    assert_eq!(m.mode, SessionMode::Import);
    assert!(!m.audio_retained);
    assert_eq!(m.recorded_ms, 0);
    assert_eq!(m.created_at.to_rfc3339(), "2025-03-14T10:30:00+01:00");
    assert_eq!(m.started_at, Some(m.created_at));
    assert_eq!(m.ended_at, Some(m.created_at));
    assert_eq!(m.annotations, "pauta: preço");
    assert_eq!(
        m.notes_md.as_deref(),
        Some(
            "### Próximos passos <!-- fala:ia -->\n- Ana manda o contrato <!-- fala:ia -->\n\n\
             _Importado do Granola em 2026-10-02._"
        )
    );
    assert!(m.transcribed_at.is_some());
    assert_eq!(
        store.meeting_import_source(m.id).unwrap().as_deref(),
        Some(GRANOLA)
    );

    let segments = store.meeting_segments(m.id).unwrap();
    let shape: Vec<_> = segments
        .iter()
        .map(|s| (s.seq, s.channel, s.speaker, s.t0_ms, s.text.as_str()))
        .collect();
    assert_eq!(
        shape,
        [
            (1, SegmentChannel::Mic, SegmentSpeaker::Me, 0, "Bom dia."),
            (
                2,
                SegmentChannel::System,
                SegmentSpeaker::Person(1),
                0,
                "Bom dia, vamos ao preço?"
            ),
            (
                3,
                SegmentChannel::System,
                SegmentSpeaker::Person(1),
                0,
                "Começo pelo contrato."
            ),
        ]
    );
    let speakers = store.meeting_speakers(m.id).unwrap();
    assert_eq!(speakers[0].name.as_deref(), Some("Ana"));
    // A transcrição importada entra na busca das reuniões, sem acento.
    let hits = store.search_meetings("comeco contrato", 10).unwrap();
    assert_eq!(hits.iter().map(|h| h.id).collect::<Vec<_>>(), [m.id]);

    // Sem título, sem resumo e sem transcrição: a UI dá o título no idioma dela.
    let empty = meetings.iter().find(|e| e.id != m.id).unwrap();
    assert_eq!(empty.title, "");
    assert_eq!(empty.notes_md, None);
    assert_eq!(empty.transcribed_at, None);

    // O `.md` de cada sessão sai como o de qualquer reunião.
    let files: Vec<String> = std::fs::read_dir(env.notes.join("Reuniões"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        files.contains(&"2025-03-14 Preço com a Acme.md".to_string()),
        "{files:?}"
    );
}

#[test]
fn importing_again_updates_instead_of_duplicating() {
    let env = env("granola-again");
    let path = env.dir.join("granola.csv");
    std::fs::write(&path, csv("Primeiro título")).unwrap();
    let mut store = env.open();
    import_all(&mut store, &path);
    let first: Vec<_> = store.meetings().unwrap().iter().map(|m| m.id).collect();
    let find = |store: &Store, title: &str| {
        store
            .meetings()
            .unwrap()
            .into_iter()
            .find(|m| m.title == title)
            .unwrap()
            .id
    };
    let id = find(&store, "Primeiro título");
    let untitled = find(&store, "");

    // O que a pessoa muda no Fala sobrevive a importar de novo.
    store.set_meeting_title(untitled, "Meu título").unwrap();
    store
        .save_meeting_annotations(untitled, "minhas anotações")
        .unwrap();
    store
        .save_edited_meeting_notes(id, "editada", at(22, 0, 0))
        .unwrap();
    store
        .set_meeting_speaker_name(id, 1, Some("Ana Souza"))
        .unwrap();

    std::fs::write(&path, csv("Título revisado")).unwrap();
    assert_eq!(
        import_all(&mut store, &path),
        [ImportOutcome::Updated, ImportOutcome::Updated]
    );
    let again = store.meetings().unwrap();
    assert_eq!(again.iter().map(|m| m.id).collect::<Vec<_>>(), first);
    let m = store.meeting(id).unwrap();
    assert_eq!(m.title, "Título revisado");
    assert_eq!(m.notes_md.as_deref(), Some("editada"));
    assert!(store.meeting_notes_edited(id).unwrap());
    assert_eq!(store.meeting_segments(id).unwrap().len(), 3);
    let speakers = store.meeting_speakers(id).unwrap();
    assert_eq!(speakers.len(), 1);
    assert_eq!(speakers[0].name.as_deref(), Some("Ana Souza"));
    let kept = store.meeting(untitled).unwrap();
    assert_eq!(kept.title, "Meu título");
    assert_eq!(kept.annotations, "minhas anotações");
    let sources = store.meeting_import_sources().unwrap();
    assert_eq!(sources.len(), 2);
    assert_eq!(sources.get(&id).map(String::as_str), Some(GRANOLA));
    let names: Vec<String> = std::fs::read_dir(env.notes.join("Reuniões"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 2, "{names:?}");

    // Apagada no Fala, a reunião volta numa importação nova, com outra sessão.
    store.delete_meeting(id).unwrap();
    assert_eq!(
        import_all(&mut store, &path),
        [ImportOutcome::Created, ImportOutcome::Updated]
    );
    assert_eq!(store.meetings().unwrap().len(), 2);
}

#[test]
fn importing_again_after_losing_the_db_and_reindexing_does_not_duplicate() {
    let env = env("granola-reindex");
    let path = env.dir.join("granola.csv");
    std::fs::write(&path, csv("Preço com a Acme")).unwrap();
    let mut store = env.open();
    import_all(&mut store, &path);
    let mut ids: Vec<_> = store.meetings().unwrap().iter().map(|m| m.id).collect();
    ids.sort();
    let renamed = store
        .meetings()
        .unwrap()
        .into_iter()
        .find(|m| m.title.is_empty())
        .unwrap()
        .id;
    store.set_meeting_title(renamed, "Meu título").unwrap();
    let md_names = |env: &common::Env| {
        let mut names: Vec<String> = std::fs::read_dir(env.notes.join("Reuniões"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    };
    let files = md_names(&env);

    drop(store);
    std::fs::remove_file(&env.db).unwrap();
    let mut store = env.open();
    let report = store.reindex_meetings().unwrap();
    assert_eq!(report.indexed, 2, "{report:?}");
    assert!(report.skipped.is_empty(), "{report:?}");
    // O `.md` já trazia a ligação: o reindex não muda nenhum arquivo.
    assert_eq!(store.sync_meeting_mirrors().unwrap(), 0);
    let sources = store.meeting_import_sources().unwrap();
    assert_eq!(sources.len(), 2);
    assert!(sources.values().all(|s| s == GRANOLA));

    // O mesmo CSV de novo: atualiza as duas, sem nenhuma reunião ou `.md` a mais.
    assert_eq!(
        import_all(&mut store, &path),
        [ImportOutcome::Updated, ImportOutcome::Updated]
    );
    let mut again: Vec<_> = store.meetings().unwrap().iter().map(|m| m.id).collect();
    again.sort();
    assert_eq!(again, ids);
    assert_eq!(md_names(&env), files);
    // O título que a pessoa deu antes de perder o banco continua sendo dela.
    assert_eq!(store.meeting(renamed).unwrap().title, "Meu título");
}

#[test]
fn a_zip_named_csv_is_read_by_its_content() {
    let env = env("granola-zip");
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    writer
        .start_file("Granola export/Acme.csv", options)
        .unwrap();
    writer.write_all(csv("Do zip").as_bytes()).unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    let path = env.dir.join("granola-export.csv");
    std::fs::write(&path, bytes).unwrap();
    let export = read_granola_export(&path).unwrap();
    assert_eq!(export.meetings.len(), 2);
    assert_eq!(export.meetings[0].title, "Do zip");
}
