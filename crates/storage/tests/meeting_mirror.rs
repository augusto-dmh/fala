//! O espelho `Reuniões/*.md` (ADR-0006): escrito depois do banco, apagado com a sessão, e
//! `reindex_meetings` reconstrói o banco a partir dele.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs;
use std::path::PathBuf;

use common::{at, env, Env};
use fala_core::{Dictionary, Language};
use fala_meeting::{SessionId, SessionMode, StopReason};
use fala_storage::{
    MeetingSegment, NameSuggestion, NewMeeting, SegmentChannel, SegmentSpeaker, StorageError, Store,
};

fn id(n: u64) -> SessionId {
    SessionId::from_parts(1_727_000_000_000 + n, u128::from(n))
}

fn segment(t0_ms: u64, speaker: SegmentSpeaker, text: &str) -> MeetingSegment {
    MeetingSegment {
        seq: 0,
        channel: if speaker == SegmentSpeaker::Me {
            SegmentChannel::Mic
        } else {
            SegmentChannel::System
        },
        speaker,
        t0_ms,
        t1_ms: t0_ms + 1_500,
        text: text.to_string(),
    }
}

fn create(store: &Store, n: u64, title: &str) -> SessionId {
    store
        .create_meeting(&NewMeeting {
            id: id(n),
            title: title.to_string(),
            mode: SessionMode::Meeting,
            local_only: false,
            created_at: at(14, 0, 0),
        })
        .unwrap();
    id(n)
}

/// Uma sessão com tudo o que o espelho guarda: pauta, gravação com pausa (45 min de relógio,
/// 30 gravados), trechos, participantes, um falante renomeado, notas e uma edição à mão.
fn full_session(store: &mut Store, n: u64) -> SessionId {
    let id = create(store, n, "Planejamento");
    store
        .save_meeting_annotations(id, "decidir data\n\nAna: contrato")
        .unwrap();
    store
        .start_meeting_recording(id, SessionMode::Meeting, at(14, 2, 0))
        .unwrap();
    store
        .finish_meeting(id, at(14, 47, 0), 1_800_000, StopReason::User)
        .unwrap();
    store.set_meeting_audio_retained(id).unwrap();
    store
        .replace_meeting_segments(
            id,
            vec![
                segment(65_000, SegmentSpeaker::Person(1), "Proponho 15/11."),
                segment(0, SegmentSpeaker::Me, "Vamos falar da data."),
                segment(70_000, SegmentSpeaker::Person(2), "Concordo."),
            ],
            at(14, 48, 0),
        )
        .unwrap();
    store
        .set_meeting_participants(id, &["Ana".to_string(), "Bruno".to_string()])
        .unwrap();
    store.set_meeting_speaker_name(id, 1, Some("Ana")).unwrap();
    store
        .save_meeting_notes(
            id,
            "geral",
            "## Anotações\n\ndecidir data ^a1\n\n## Notas · Reunião geral\n\n- Data: 15/11. [[#^s2|01:05]] <!-- fala:ia -->\n",
        )
        .unwrap();
    store
        .save_edited_meeting_notes(
            id,
            "## Anotações\n\ndecidir data ^a1\n\n## Notas · Reunião geral\n\n- Data: 15/11, confirmada. [[#^s2|01:05]]\n",
            at(15, 0, 0),
        )
        .unwrap();
    id
}

fn reunioes(e: &Env) -> PathBuf {
    e.notes.join("Reuniões")
}

fn md_files(e: &Env) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(reunioes(e))
        .map(|dir| {
            dir.map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn read(e: &Env, name: &str) -> String {
    fs::read_to_string(reunioes(e).join(name)).unwrap()
}

/// Tudo o que o banco sabe de uma sessão, para comparar antes e depois do `reindex`.
fn snapshot(store: &Store, id: SessionId) -> String {
    format!(
        "{:?}\n{:?}\n{:?}\n{:?}\n{:?}",
        store.meeting(id).unwrap(),
        store.meeting_segments(id).unwrap(),
        store.meeting_participants(id).unwrap(),
        store.meeting_speakers(id).unwrap(),
        store.meeting_notes_edited_at(id).unwrap(),
    )
}

#[test]
fn every_change_rewrites_the_md() {
    let e = env("meeting_mirror_every_change");
    let mut store = e.open();
    let id = full_session(&mut store, 1);
    assert_eq!(md_files(&e), ["2026-10-02 Planejamento.md"]);
    let md = read(&e, "2026-10-02 Planejamento.md");
    assert!(md.contains(&format!("\nid: \"{id}\"\n")), "{md}");
    for want in [
        "\ntitle: \"Planejamento\"\ndate: \"2026-10-02\"\nmode: \"meeting\"\nlocal_only: false\n",
        "\nstarted_at: \"2026-10-02T14:02:00-03:00\"\nended_at: \"2026-10-02T14:47:00-03:00\"\n",
        "\nrecorded_ms: 1800000\nstop_reason: \"user\"\naudio_retained: true\n",
        "\ntemplate: \"geral\"\nnotes: true\nnotes_edited_at: \"2026-10-02T15:00:00-03:00\"\n",
        "\nparticipants: [\"Ana\",\"Bruno\"]\nspeakers: {\"1\":\"Ana\"}\n",
        "\nannotations: \"decidir data\\n\\nAna: contrato\"\n---\n# Planejamento\n\n2026-10-02 14:02\n",
        "- Data: 15/11, confirmada. [[#^s2|01:05]]\n\n## Transcrição\n\n",
        "- **[01:05] Ana:** Proponho 15/11. ^s2\n- **[01:10] Pessoa 2:** Concordo. ^s3\n",
        "^s3\n\n<!-- fala:segments\ns1 mic me 0 1500\ns2 system 1 65000 66500\ns3 system 2 70000 71500\n-->\n",
    ] {
        assert!(md.contains(want), "{want}\n---\n{md}");
    }
    // Nenhum caminho de disco (ADR-0011).
    assert!(!md.contains("audio/") && !md.contains(".opus"), "{md}");
    assert!(!md.contains(&e.dir.display().to_string()), "{md}");

    store
        .set_meeting_speaker_name(id, 2, Some("Bruno"))
        .unwrap();
    let md = read(&e, "2026-10-02 Planejamento.md");
    assert!(md.contains("- **[01:10] Bruno:** Concordo. ^s3\n"), "{md}");
}

#[test]
fn round_trip_through_reindex_is_identical() {
    let e = env("meeting_mirror_round_trip");
    let mut store = e.open();
    let id = full_session(&mut store, 1);
    let draft = create(&store, 2, "");
    store
        .save_meeting_annotations(draft, "pauta: orçamento")
        .unwrap();
    let before = (snapshot(&store, id), snapshot(&store, draft));
    let files = md_files(&e);
    let contents: Vec<String> = files.iter().map(|f| read(&e, f)).collect();
    assert_eq!(
        files,
        ["2026-10-02 Planejamento.md", "2026-10-02 Reunião.md"]
    );

    drop(store);
    fs::remove_file(&e.db).unwrap();
    let mut store = e.open();
    assert!(store.meetings().unwrap().is_empty());
    let report = store.reindex_meetings().unwrap();
    assert_eq!(report.indexed, 2, "{report:?}");
    assert!(report.skipped.is_empty(), "{report:?}");
    assert_eq!((snapshot(&store, id), snapshot(&store, draft)), before);

    // De volta ao `.md`: o mesmo arquivo, byte a byte, e nenhum reescrito.
    assert_eq!(store.sync_meeting_mirrors().unwrap(), 0);
    assert_eq!(md_files(&e), files);
    let again: Vec<String> = files.iter().map(|f| read(&e, f)).collect();
    assert_eq!(again, contents);

    // Depois do reindex, uma mudança reescreve o mesmo arquivo.
    store.set_meeting_title(draft, "Orçamento").unwrap();
    assert_eq!(
        md_files(&e),
        ["2026-10-02 Orçamento.md", "2026-10-02 Planejamento.md"]
    );
}

#[test]
fn the_md_follows_the_commit() {
    let e = env("meeting_mirror_after_commit");
    let store = e.open();
    // O banco recusa: nenhum `.md`.
    assert!(matches!(
        store.set_meeting_title(id(9), "nada"),
        Err(StorageError::NotFound(_))
    ));
    assert!(md_files(&e).is_empty());

    // O espelho falha: a mudança já está no banco e não volta como erro.
    fs::create_dir_all(&e.notes).unwrap();
    fs::write(reunioes(&e), "um arquivo no lugar da pasta").unwrap();
    let id = create(&store, 1, "Daily");
    store.set_meeting_title(id, "Daily de terça").unwrap();
    assert_eq!(store.meeting(id).unwrap().title, "Daily de terça");

    // A próxima sincronização escreve o que faltou.
    fs::remove_file(reunioes(&e)).unwrap();
    assert_eq!(store.sync_meeting_mirrors().unwrap(), 1);
    assert_eq!(md_files(&e), ["2026-10-02 Daily de terça.md"]);
    assert_eq!(store.sync_meeting_mirrors().unwrap(), 0);
}

#[test]
fn title_changes_rename_and_same_names_get_a_number() {
    let e = env("meeting_mirror_names");
    let store = e.open();
    let a = create(&store, 1, "");
    let b = create(&store, 2, "");
    let c = create(&store, 3, "");
    assert_eq!(
        md_files(&e),
        [
            "2026-10-02 Reunião (2).md",
            "2026-10-02 Reunião (3).md",
            "2026-10-02 Reunião.md"
        ]
    );
    assert!(store
        .set_generated_meeting_title(b, "Kickoff: cliente/X")
        .unwrap());
    store.set_meeting_title(a, "Kickoff cliente X").unwrap();
    assert_eq!(
        md_files(&e),
        [
            "2026-10-02 Kickoff cliente X (2).md",
            "2026-10-02 Kickoff cliente X.md",
            "2026-10-02 Reunião (3).md"
        ]
    );
    assert!(read(&e, "2026-10-02 Kickoff cliente X.md").contains(&format!("\nid: \"{b}\"\n")));
    // Só a caixa mudou: o mesmo arquivo no Windows, sem `(2)`.
    store.set_meeting_title(c, "reunião").unwrap();
    assert!(md_files(&e).contains(&"2026-10-02 Reunião (3).md".to_string()));
}

#[test]
fn deleting_a_meeting_deletes_its_md() {
    let e = env("meeting_mirror_delete");
    let mut store = e.open();
    let gone = full_session(&mut store, 1);
    create(&store, 2, "Fica");
    store.delete_meeting(gone).unwrap();
    assert_eq!(md_files(&e), ["2026-10-02 Fica.md"]);
    assert!(matches!(
        store.delete_meeting(gone),
        Err(StorageError::NotFound(_))
    ));

    // O `.md` não sai (uma pasta no lugar dele): a sessão fica, e apagar de novo funciona.
    let stuck = create(&store, 3, "Presa");
    let path = reunioes(&e).join("2026-10-02 Presa.md");
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(matches!(
        store.delete_meeting(stuck),
        Err(StorageError::MeetingMirror { .. })
    ));
    assert_eq!(store.meeting(stuck).unwrap().title, "Presa");
    fs::remove_dir(&path).unwrap();
    store.delete_meeting(stuck).unwrap();
    assert!(store.meeting(stuck).is_err());

    // O `reindex` não traz as apagadas de volta.
    assert_eq!(store.reindex_meetings().unwrap().indexed, 0);
    assert!(store.meeting(gone).is_err());
}

#[test]
fn reindex_never_touches_what_the_db_has_and_reports_bad_files() {
    let e = env("meeting_mirror_reindex_db_has");
    let mut store = e.open();
    let id = full_session(&mut store, 1);
    // Uma sugestão pendente: o `.md` não a guarda, e o banco não a perde.
    let suggestion = NameSuggestion {
        name: "Bruno".to_string(),
        reason: "falou do contrato".to_string(),
    };
    store
        .save_speaker_suggestions(id, &[(2, suggestion)])
        .unwrap();
    let before = snapshot(&store, id);
    // Um `.md` desatualizado da mesma sessão também não reescreve o banco.
    let name = "2026-10-02 Planejamento.md";
    let stale = read(&e, name).replace("Concordo.", "Discordo.");
    fs::write(reunioes(&e).join(name), stale).unwrap();
    fs::write(reunioes(&e).join("solto.md"), "sem frontmatter\n").unwrap();
    fs::write(reunioes(&e).join("leia-me.txt"), "não é .md").unwrap();

    let report = store.reindex_meetings().unwrap();
    assert_eq!(report.indexed, 0, "{report:?}");
    assert_eq!(report.skipped.len(), 1, "{report:?}");
    assert!(report.skipped[0].path.ends_with("solto.md"));
    assert_eq!(snapshot(&store, id), before);
}

#[test]
fn reindex_prefers_the_file_named_after_the_title() {
    let e = env("meeting_mirror_reindex_duplicate");
    let store = e.open();
    let id = create(&store, 1, "Original");
    store.save_meeting_annotations(id, "a pauta certa").unwrap();
    // Uma cópia velha que o Obsidian nomeou e que vem antes na ordem dos nomes.
    let copy = read(&e, "2026-10-02 Original.md").replace("a pauta certa", "pauta velha");
    fs::write(reunioes(&e).join("2026-10-02 Original 1.md"), copy).unwrap();
    drop(store);
    fs::remove_file(&e.db).unwrap();

    let mut store = e.open();
    let report = store.reindex_meetings().unwrap();
    assert_eq!(report.indexed, 1);
    assert_eq!(report.skipped.len(), 1);
    assert!(report.skipped[0].path.ends_with("2026-10-02 Original 1.md"));
    assert_eq!(report.skipped[0].reason, "id duplicado");
    assert_eq!(store.meeting(id).unwrap().annotations, "a pauta certa");
}

/// O corpo do espelho é o documento que "Copiar Markdown" monta com `fala_notes::note_document`.
#[test]
fn body_is_the_copied_note_document() {
    let e = env("meeting_mirror_note_document");
    let mut store = e.open();
    let id = full_session(&mut store, 1);
    let record = store.meeting(id).unwrap();
    let names = store.meeting_speakers(id).unwrap();
    let transcript = store
        .meeting_segments(id)
        .unwrap()
        .into_iter()
        .map(|s| fala_notes::Segment {
            id: s.seq,
            channel: match s.channel {
                SegmentChannel::Mic => fala_notes::Channel::Mic,
                SegmentChannel::System => fala_notes::Channel::System,
            },
            speaker: match s.speaker {
                SegmentSpeaker::Me => fala_notes::Speaker::Me,
                SegmentSpeaker::Omitted => fala_notes::Speaker::Omitted,
                SegmentSpeaker::Erased => fala_notes::Speaker::Erased,
                SegmentSpeaker::Resumed => fala_notes::Speaker::Resumed,
                SegmentSpeaker::Person(n) => names
                    .iter()
                    .find(|p| p.person == n)
                    .and_then(|p| p.name.clone())
                    .map_or(fala_notes::Speaker::Person(n), fala_notes::Speaker::Named),
            },
            t0_ms: s.t0_ms,
            t1_ms: s.t1_ms,
            text: s.text,
        })
        .collect();
    let input = fala_notes::NotesInput {
        title: record.title.clone(),
        date: "2026-10-02".to_string(),
        start_time: "14:02".to_string(),
        language: Language::PtBr,
        profile: String::new(),
        dictionary: Dictionary::default(),
        participants: store.meeting_participants(id).unwrap(),
        template: fala_notes::builtin_templates().unwrap().remove(0),
        annotations: record.annotations.clone(),
        transcript,
        local_only: false,
    };
    let expected = fala_notes::note_document(&input, record.notes_md.as_deref());
    let md = read(&e, "2026-10-02 Planejamento.md");
    let body = md.splitn(3, "---\n").nth(2).unwrap();
    let body = &body[..body.find("\n<!-- fala:segments\n").unwrap()];
    assert_eq!(body, expected);

    // Sem notas: o bloco de anotações, como no documento copiado.
    let draft = create(&store, 2, "Pauta");
    store
        .save_meeting_annotations(draft, "decidir data\n\n  \nAna: contrato")
        .unwrap();
    let mut input = input;
    input.title = "Pauta".to_string();
    input.start_time = "14:00".to_string();
    input.annotations = "decidir data\n\n  \nAna: contrato".to_string();
    input.transcript.clear();
    let md = read(&e, "2026-10-02 Pauta.md");
    let body = md.splitn(3, "---\n").nth(2).unwrap();
    assert_eq!(body, fala_notes::note_document(&input, None));
}

#[test]
fn a_dictation_marker_reaches_the_segments_and_the_md() {
    let e = env("meeting_mirror_omitted");
    let mut store = e.open();
    let id = create(&store, 1, "Planejamento");
    store
        .start_meeting_recording(id, SessionMode::Meeting, at(14, 2, 0))
        .unwrap();
    // O gravador anota o ditado na hora; a transcrição vem depois do fim.
    store.add_meeting_omission(id, 30_000, 42_500).unwrap();
    store
        .finish_meeting(id, at(14, 47, 0), 1_800_000, StopReason::User)
        .unwrap();
    // Um marcador vindo de quem transcreve não vale: só o do gravador entra.
    let fake = fala_storage::omitted_segment(1_000, 2_000);
    store
        .replace_meeting_segments(
            id,
            vec![
                segment(65_000, SegmentSpeaker::Person(1), "Proponho 15/11."),
                segment(0, SegmentSpeaker::Me, "Vamos falar da data."),
                fake,
            ],
            at(14, 48, 0),
        )
        .unwrap();
    let segments = store.meeting_segments(id).unwrap();
    let marker = MeetingSegment {
        seq: 2,
        ..fala_storage::omitted_segment(30_000, 42_500)
    };
    assert_eq!(segments.len(), 3, "{segments:?}");
    assert_eq!(segments[1], marker);
    assert_eq!(store.meeting_omissions(id).unwrap(), [(30_000, 42_500)]);

    let md = read(&e, "2026-10-02 Planejamento.md");
    for want in [
        "- **[00:30] Eu:** [trecho de ditado omitido] ^s2\n",
        "\ns2 mic omitted 30000 42500\n",
    ] {
        assert!(md.contains(want), "{want}\n---\n{md}");
    }

    // O `.md` reconstrói o marcador.
    let before = snapshot(&store, id);
    drop(store);
    fs::remove_file(&e.db).unwrap();
    let mut store = e.open();
    store.reindex_meetings().unwrap();
    assert_eq!(snapshot(&store, id), before);
    // E uma transcrição nova depois do reindex ainda tem o marcador.
    assert_eq!(store.meeting_omissions(id).unwrap(), [(30_000, 42_500)]);
    store
        .replace_meeting_segments(
            id,
            vec![segment(0, SegmentSpeaker::Me, "Oi.")],
            at(15, 0, 0),
        )
        .unwrap();
    let again = store.meeting_segments(id).unwrap();
    assert_eq!(again[1].speaker, SegmentSpeaker::Omitted, "{again:?}");
}
