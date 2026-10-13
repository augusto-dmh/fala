//! As pessoas das reuniões: derivadas de "Quem estava" e dos nomes de "Pessoa N", uma vez por
//! reunião, sem a lixeira nem rascunhos vazios, e iguais depois de um `reindex` (ADR-0006).
//! A lixeira volta pelo `reindex` na mesma pasta: `meeting_trash.rs` prova.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs;

use common::{at, env};
use fala_meeting::{SessionId, SessionMode, StopReason};
use fala_storage::{MeetingSegment, NewMeeting, SegmentChannel, SegmentSpeaker, Store};

fn id(n: u64) -> SessionId {
    SessionId::from_parts(1_727_000_000_000 + n, u128::from(n))
}

/// Um rascunho criado às `hour`:00, com "Quem estava".
fn draft(store: &mut Store, n: u64, hour: u32, people: &[&str]) -> SessionId {
    let id = id(n);
    store
        .create_meeting(&NewMeeting {
            id,
            title: format!("Reunião {n}"),
            mode: SessionMode::Meeting,
            local_only: false,
            created_at: at(hour, 0, 0),
        })
        .unwrap();
    let names: Vec<String> = people.iter().map(|p| p.to_string()).collect();
    store.set_meeting_participants(id, &names).unwrap();
    id
}

/// Uma sessão gravada às `hour`:00, com "Quem estava" e a "Pessoa 1" chamada `speaker`.
fn recorded(
    store: &mut Store,
    n: u64,
    hour: u32,
    people: &[&str],
    speaker: Option<&str>,
) -> SessionId {
    let id = draft(store, n, hour, people);
    store
        .start_meeting_recording(id, SessionMode::Meeting, at(hour, 5, 0))
        .unwrap();
    store
        .finish_meeting(id, at(hour, 30, 0), 60_000, StopReason::User)
        .unwrap();
    let segment = MeetingSegment {
        seq: 0,
        channel: SegmentChannel::System,
        speaker: SegmentSpeaker::Person(1),
        t0_ms: 1_000,
        t1_ms: 2_000,
        text: "oi".to_string(),
    };
    store
        .replace_meeting_segments(id, vec![segment], at(hour, 31, 0))
        .unwrap();
    if let Some(name) = speaker {
        store.set_meeting_speaker_name(id, 1, Some(name)).unwrap();
    }
    id
}

/// Cada pessoa como `chave|nome|hora da última reunião|reuniões`.
fn rows(store: &Store) -> Vec<String> {
    use chrono::Timelike;
    let people = store.meeting_directory().unwrap();
    people
        .iter()
        .map(|p| {
            format!(
                "{}|{}|{}|{}",
                p.key,
                p.name,
                p.last_meeting_at.hour(),
                p.meetings
            )
        })
        .collect()
}

#[test]
fn people_count_once_per_meeting_and_survive_a_reindex() {
    let e = env("directory-count");
    let mut store = e.open();
    let first = recorded(&mut store, 1, 9, &["ana", "Bruno"], None);
    // Na mesma reunião, "Ana" em "Quem estava" e como falante conta uma vez.
    let second = recorded(
        &mut store,
        2,
        11,
        &["Aná ", "Carla@Example.com"],
        Some("Ana"),
    );
    let third = recorded(&mut store, 3, 13, &["Ana Souza"], Some("Bruno"));

    assert_eq!(
        rows(&store),
        [
            "ana souza|Ana Souza|13|1",
            "bruno|Bruno|13|2",
            "ana|Aná|11|2",
            "carla@example.com|Carla@Example.com|11|1",
        ]
    );
    assert_eq!(store.person_meeting_ids("ana").unwrap(), [second, first]);
    assert_eq!(store.person_meeting_ids("bruno").unwrap(), [third, first]);
    assert!(store.person_meeting_ids("ninguem").unwrap().is_empty());

    // Apagar o banco e reindexar os `.md` dá o mesmo diretório.
    let before = rows(&store);
    drop(store);
    fs::remove_file(&e.db).unwrap();
    let mut store = e.open();
    assert_eq!(store.reindex_meetings().unwrap().indexed, 3);
    assert_eq!(rows(&store), before);
    assert_eq!(store.person_meeting_ids("ana").unwrap(), [second, first]);
}

#[test]
fn trash_and_empty_drafts_stay_out() {
    let e = env("directory-out");
    let mut store = e.open();
    let kept = recorded(&mut store, 1, 9, &["Ana"], None);
    let trashed = recorded(&mut store, 2, 10, &["Ana", "Bruno"], None);
    store.trash_meeting(trashed, at(12, 0, 0)).unwrap();
    // Só título e "Quem estava": uma reunião planejada, não uma que aconteceu.
    draft(&mut store, 3, 14, &["Carla"]);
    // Com pauta, o rascunho conta.
    let blank = draft(&mut store, 5, 16, &["Eva"]);
    store.save_meeting_annotations(blank, "\n\t \r\n").unwrap();
    let agenda = draft(&mut store, 4, 15, &["Davi"]);
    store.save_meeting_annotations(agenda, "pauta").unwrap();

    assert_eq!(rows(&store), ["davi|Davi|15|1", "ana|Ana|9|1"]);
    assert_eq!(store.person_meeting_ids("ana").unwrap(), [kept]);

    store.restore_meeting(trashed).unwrap();
    assert_eq!(rows(&store)[1], "ana|Ana|10|2");
}
