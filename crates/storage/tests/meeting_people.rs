//! Quem estava numa reunião: participantes, nomes aplicados e sugestões (ADR-0016).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{at, env};
use fala_meeting::{SessionId, SessionMode};
use fala_storage::{
    MeetingSegment, MeetingSpeaker, NameSuggestion, NewMeeting, SegmentChannel, SegmentSpeaker,
    StorageError, Store,
};
use rusqlite::Connection;

fn id(n: u64) -> SessionId {
    SessionId::from_parts(1_727_000_000_000 + n, u128::from(n))
}

fn meeting(store: &Store, n: u64, title: &str) -> SessionId {
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

fn segment(t0_ms: u64, speaker: SegmentSpeaker) -> MeetingSegment {
    MeetingSegment {
        seq: 0,
        channel: SegmentChannel::System,
        speaker,
        t0_ms,
        t1_ms: t0_ms + 1_000,
        text: "oi".to_string(),
    }
}

fn suggestion(name: &str, reason: &str) -> NameSuggestion {
    NameSuggestion {
        name: name.to_string(),
        reason: reason.to_string(),
    }
}

#[test]
fn tables_exist_after_open() {
    let e = env("people_tables");
    drop(e.open());
    let conn = Connection::open(&e.db).unwrap();
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap();
    let names: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(
        names.contains(&"meeting_participants".to_string()),
        "{names:?}"
    );
    assert!(names.contains(&"meeting_speakers".to_string()), "{names:?}");
    // Reabrir não falha nem apaga nada.
    drop(e.open());
}

#[test]
fn participants_are_replaced_in_order() {
    let e = env("people_participants");
    let mut store = e.open();
    let id = meeting(&store, 1, "Planejamento");
    assert!(store.meeting_participants(id).unwrap().is_empty());
    store
        .set_meeting_participants(id, &["Ana".to_string(), "Bruno Lima".to_string()])
        .unwrap();
    assert_eq!(
        store.meeting_participants(id).unwrap(),
        ["Ana", "Bruno Lima"]
    );
    store
        .set_meeting_participants(id, &["Carla".to_string()])
        .unwrap();
    assert_eq!(store.meeting_participants(id).unwrap(), ["Carla"]);

    let missing = SessionId::from_parts(1, 1);
    assert!(matches!(
        store.set_meeting_participants(missing, &[]),
        Err(StorageError::NotFound(_))
    ));
}

#[test]
fn suggestion_becomes_a_name_only_when_applied() {
    let e = env("people_apply");
    let mut store = e.open();
    let id = meeting(&store, 1, "Planejamento");
    let segments = vec![
        segment(0, SegmentSpeaker::Person(1)),
        segment(5_000, SegmentSpeaker::Person(2)),
    ];
    store
        .replace_meeting_segments(id, segments, at(15, 0, 0))
        .unwrap();
    store
        .save_speaker_suggestions(id, &[(1, suggestion("Ana", "Cuida do contrato."))])
        .unwrap();
    assert_eq!(
        store.meeting_speakers(id).unwrap(),
        [MeetingSpeaker {
            person: 1,
            name: None,
            suggestion: Some(suggestion("Ana", "Cuida do contrato.")),
        }]
    );
    // Os trechos continuam "Pessoa 1" até o clique.
    assert_eq!(
        store.meeting_segments(id).unwrap()[0].speaker,
        SegmentSpeaker::Person(1)
    );

    assert_eq!(store.apply_speaker_suggestion(id, 1).unwrap(), "Ana");
    assert!(matches!(
        store.apply_speaker_suggestion(id, 1),
        Err(StorageError::NotFound(_))
    ));
    assert!(matches!(
        store.apply_speaker_suggestion(id, 2),
        Err(StorageError::NotFound(_))
    ));

    // Uma geração nova troca as sugestões e não mexe no nome aplicado; uma sugestão para quem
    // já tem nome (geração que começou antes do clique) não entra.
    store
        .save_speaker_suggestions(
            id,
            &[
                (1, suggestion("Outra", "chegou depois do clique")),
                (2, suggestion("Bruno", "Fala do jurídico.")),
            ],
        )
        .unwrap();
    assert_eq!(
        store.meeting_speakers(id).unwrap(),
        [
            MeetingSpeaker {
                person: 1,
                name: Some("Ana".to_string()),
                suggestion: None,
            },
            MeetingSpeaker {
                person: 2,
                name: None,
                suggestion: Some(suggestion("Bruno", "Fala do jurídico.")),
            },
        ]
    );
    store.save_speaker_suggestions(id, &[]).unwrap();
    assert_eq!(store.meeting_speakers(id).unwrap().len(), 1);
}

#[test]
fn transcribing_again_forgets_names() {
    let e = env("people_retranscribe");
    let mut store = e.open();
    let id = meeting(&store, 1, "Planejamento");
    store
        .replace_meeting_segments(
            id,
            vec![segment(0, SegmentSpeaker::Person(1))],
            at(15, 0, 0),
        )
        .unwrap();
    store
        .save_speaker_suggestions(id, &[(1, suggestion("Ana", "x"))])
        .unwrap();
    store.apply_speaker_suggestion(id, 1).unwrap();
    // Uma sugestão ainda pendente também não sobrevive à transcrição nova.
    store
        .save_speaker_suggestions(id, &[(2, suggestion("Bruno", "y"))])
        .unwrap();
    assert_eq!(store.meeting_speakers(id).unwrap().len(), 2);
    store
        .replace_meeting_segments(
            id,
            vec![segment(0, SegmentSpeaker::Person(1))],
            at(16, 0, 0),
        )
        .unwrap();
    assert!(store.meeting_speakers(id).unwrap().is_empty());
}

#[test]
fn typed_name_replaces_the_suggestion_and_can_be_erased() {
    let e = env("people_typed_name");
    let mut store = e.open();
    let id = meeting(&store, 1, "Planejamento");
    store
        .save_speaker_suggestions(
            id,
            &[(1, suggestion("Ana", "x")), (2, suggestion("Bruno", "y"))],
        )
        .unwrap();

    store
        .set_meeting_speaker_name(id, 2, Some("Bruno Lima"))
        .unwrap();
    store
        .set_meeting_speaker_name(id, 3, Some("Carla"))
        .unwrap();
    assert_eq!(
        store.meeting_speakers(id).unwrap(),
        [
            MeetingSpeaker {
                person: 1,
                name: None,
                suggestion: Some(suggestion("Ana", "x")),
            },
            MeetingSpeaker {
                person: 2,
                name: Some("Bruno Lima".to_string()),
                suggestion: None,
            },
            MeetingSpeaker {
                person: 3,
                name: Some("Carla".to_string()),
                suggestion: None,
            },
        ]
    );

    // Apagar o nome devolve "Pessoa N": a linha sem nome nem sugestão some.
    store.set_meeting_speaker_name(id, 3, None).unwrap();
    store.set_meeting_speaker_name(id, 1, None).unwrap();
    assert_eq!(
        store
            .meeting_speakers(id)
            .unwrap()
            .iter()
            .map(|s| s.person)
            .collect::<Vec<_>>(),
        [2]
    );

    let missing = SessionId::from_parts(1, 1);
    assert!(matches!(
        store.set_meeting_speaker_name(missing, 1, Some("Ana")),
        Err(StorageError::NotFound(_))
    ));
    assert!(store.set_meeting_speaker_name(id, 0, Some("Ana")).is_err());
}
