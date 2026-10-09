//! Reuniões no `fala.sqlite` (schema 2): checks C1-C9 do `meeting-panel`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{at, env, unedited};
use fala_meeting::{SessionId, SessionMode, StopReason};
use fala_storage::{MeetingSegment, NewMeeting, SegmentChannel, SegmentSpeaker, StorageError};
use rusqlite::Connection;

fn id(n: u64) -> SessionId {
    SessionId::from_parts(1_727_000_000_000 + n, u128::from(n))
}

fn new_meeting(n: u64, hour: u32) -> NewMeeting {
    NewMeeting {
        id: id(n),
        title: format!("Reunião {n}"),
        mode: SessionMode::InPerson,
        local_only: false,
        created_at: at(hour, 0, 0),
    }
}

/// Cria a sessão e a põe para gravar no mesmo instante.
fn recorded(store: &fala_storage::Store, n: u64, hour: u32) -> NewMeeting {
    let meeting = new_meeting(n, hour);
    store.create_meeting(&meeting).unwrap();
    store
        .start_meeting_recording(meeting.id, SessionMode::InPerson, at(hour, 0, 30))
        .unwrap();
    meeting
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
        t1_ms: t0_ms + 1_000,
        text: text.to_string(),
    }
}

fn user_version(db: &std::path::Path) -> i64 {
    Connection::open(db)
        .unwrap()
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap()
}

fn tables(db: &std::path::Path) -> Vec<String> {
    let conn = Connection::open(db).unwrap();
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap();
    stmt.query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// O schema 1 literal, como o `fala-storage` o criava antes da reunião.
const SCHEMA_1: &str = "
CREATE TABLE dictations (
    rowid INTEGER PRIMARY KEY,
    id TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    created_ms INTEGER NOT NULL,
    language TEXT NOT NULL,
    app TEXT,
    edited_by TEXT NOT NULL CHECK (edited_by IN ('none', 'rules', 'llm')),
    showing TEXT NOT NULL CHECK (showing IN ('final', 'raw')),
    raw TEXT NOT NULL,
    final TEXT NOT NULL,
    sensitive INTEGER NOT NULL DEFAULT 0 CHECK (sensitive IN (0, 1))
);
CREATE INDEX dictations_by_time ON dictations (created_ms DESC, id DESC);
CREATE VIRTUAL TABLE dictations_fts USING fts5(raw, final, content='dictations', content_rowid='rowid', tokenize='unicode61 remove_diacritics 2');
CREATE TRIGGER dictations_ai AFTER INSERT ON dictations BEGIN
    INSERT INTO dictations_fts (rowid, raw, final) VALUES (new.rowid, new.raw, new.final);
END;
PRAGMA user_version = 1;
";

#[test]
fn migrates_v1_to_v2_keeping_dictations() {
    let e = env("meetings_migrate");
    {
        let conn = Connection::open(&e.db).unwrap();
        conn.execute_batch(SCHEMA_1).unwrap();
        for (n, text) in ["primeiro ditado", "segundo ditado"].iter().enumerate() {
            conn.execute(
                "INSERT INTO dictations (id, created_at, created_ms, language, app, edited_by, showing, raw, final)
                 VALUES (?1, '2026-10-02T09:00:00-03:00', ?2, 'pt-BR', NULL, 'none', 'final', ?3, ?3)",
                rusqlite::params![format!("0192f000-0000-7000-8000-00000000000{n}"), n as i64, text],
            )
            .unwrap();
        }
    }
    assert_eq!(user_version(&e.db), 1);

    let store = e.open();
    assert_eq!(user_version(&e.db), 2);
    let names = tables(&e.db);
    assert!(names.contains(&"meetings".to_string()), "{names:?}");
    assert!(names.contains(&"meeting_segments".to_string()), "{names:?}");
    assert_eq!(
        store
            .get("0192f000-0000-7000-8000-000000000000")
            .unwrap()
            .shown_text(),
        "primeiro ditado"
    );
    assert_eq!(
        store
            .get("0192f000-0000-7000-8000-000000000001")
            .unwrap()
            .shown_text(),
        "segundo ditado"
    );
    drop(store);

    // Um banco novo abre direto em 2 e aceita ditados e reuniões.
    let fresh = env("meetings_migrate_fresh");
    let store = fresh.open();
    assert_eq!(user_version(&fresh.db), 2);
    store.add(&unedited("oi"), at(9, 0, 0)).unwrap();
    store.create_meeting(&new_meeting(1, 10)).unwrap();
    drop(store);
    // Reabrir um banco em 2 não reaplica nada.
    let store = fresh.open();
    assert_eq!(store.meetings().unwrap().len(), 1);
}

#[test]
fn create_then_read_meeting() {
    let e = env("meetings_create");
    let store = e.open();
    let meeting = new_meeting(1, 14);
    store.create_meeting(&meeting).unwrap();

    let record = store.meeting(meeting.id).unwrap();
    assert_eq!(record.id, meeting.id);
    assert_eq!(record.title, "Reunião 1");
    assert_eq!(record.mode, SessionMode::InPerson);
    assert!(!record.local_only);
    assert_eq!(record.created_at, at(14, 0, 0));
    assert_eq!(record.started_at, None);
    assert_eq!(record.ended_at, None);
    assert_eq!(record.recorded_ms, 0);
    assert_eq!(record.stop_reason, None);
    assert!(!record.audio_retained);
    assert_eq!(record.annotations, "");
    assert_eq!(record.transcribed_at, None);
    assert_eq!(record.notes_md, None);
    assert_eq!(record.notes_template, None);
}

#[test]
fn finish_meeting_records_end() {
    let e = env("meetings_finish");
    let store = e.open();
    let meeting = recorded(&store, 1, 14);

    store
        .finish_meeting(meeting.id, at(14, 1, 1), 61_000, StopReason::CapReached)
        .unwrap();
    store.set_meeting_audio_retained(meeting.id).unwrap();

    let record = store.meeting(meeting.id).unwrap();
    assert_eq!(record.ended_at, Some(at(14, 1, 1)));
    assert_eq!(record.recorded_ms, 61_000);
    assert_eq!(record.stop_reason, Some(StopReason::CapReached));
    assert!(record.audio_retained);

    let raw: String = Connection::open(&e.db)
        .unwrap()
        .query_row("SELECT stop_reason FROM meetings", [], |r| r.get(0))
        .unwrap();
    assert_eq!(raw, "cap_reached");
}

#[test]
fn annotations_survive_reopen() {
    let e = env("meetings_annotations");
    let meeting = new_meeting(1, 14);
    {
        let store = e.open();
        store.create_meeting(&meeting).unwrap();
        store
            .save_meeting_annotations(meeting.id, "decidir data\nAna: contrato")
            .unwrap();
    }
    let store = e.open();
    assert_eq!(
        store.meeting(meeting.id).unwrap().annotations,
        "decidir data\nAna: contrato"
    );
}

#[test]
fn segments_sorted_numbered_and_replaced() {
    let e = env("meetings_segments");
    let mut store = e.open();
    let meeting = recorded(&store, 1, 14);

    store
        .replace_meeting_segments(
            meeting.id,
            vec![
                segment(9_000, SegmentSpeaker::Me, "terceiro"),
                segment(0, SegmentSpeaker::Person(2), "primeiro"),
                segment(4_000, SegmentSpeaker::Person(1), "segundo"),
            ],
            at(15, 0, 0),
        )
        .unwrap();

    let segments = store.meeting_segments(meeting.id).unwrap();
    let got: Vec<(u32, u64, SegmentSpeaker, &str)> = segments
        .iter()
        .map(|s| (s.seq, s.t0_ms, s.speaker, s.text.as_str()))
        .collect();
    assert_eq!(
        got,
        vec![
            (1, 0, SegmentSpeaker::Person(2), "primeiro"),
            (2, 4_000, SegmentSpeaker::Person(1), "segundo"),
            (3, 9_000, SegmentSpeaker::Me, "terceiro"),
        ]
    );
    assert_eq!(segments[2].channel, SegmentChannel::Mic);
    assert_eq!(
        store.meeting(meeting.id).unwrap().transcribed_at,
        Some(at(15, 0, 0))
    );

    let conn = Connection::open(&e.db).unwrap();
    let speakers: Vec<String> = conn
        .prepare("SELECT speaker FROM meeting_segments ORDER BY seq")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        speakers,
        vec![r#"{"person":2}"#, r#"{"person":1}"#, r#""me""#]
    );

    store
        .replace_meeting_segments(
            meeting.id,
            vec![segment(500, SegmentSpeaker::Me, "único")],
            at(16, 0, 0),
        )
        .unwrap();
    let segments = store.meeting_segments(meeting.id).unwrap();
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].seq, 1);
    assert_eq!(segments[0].text, "único");
}

#[test]
fn notes_saved() {
    let e = env("meetings_notes");
    let store = e.open();
    let meeting = new_meeting(1, 14);
    store.create_meeting(&meeting).unwrap();
    store
        .save_meeting_notes(meeting.id, "geral", "## Notas")
        .unwrap();
    let record = store.meeting(meeting.id).unwrap();
    assert_eq!(record.notes_template.as_deref(), Some("geral"));
    assert_eq!(record.notes_md.as_deref(), Some("## Notas"));
}

#[test]
fn list_newest_first() {
    let e = env("meetings_list");
    let store = e.open();
    store.create_meeting(&new_meeting(2, 11)).unwrap();
    store.create_meeting(&new_meeting(1, 15)).unwrap();
    store.create_meeting(&new_meeting(3, 9)).unwrap();
    let titles: Vec<String> = store
        .meetings()
        .unwrap()
        .into_iter()
        .map(|m| m.title)
        .collect();
    assert_eq!(titles, vec!["Reunião 1", "Reunião 2", "Reunião 3"]);
}

fn assert_not_found<T: std::fmt::Debug>(result: Result<T, StorageError>, missing: SessionId) {
    match result {
        Err(StorageError::NotFound(got)) => assert_eq!(got, missing.to_string()),
        other => panic!("esperava NotFound, veio {other:?}"),
    }
}

#[test]
fn unknown_id_is_not_found() {
    let e = env("meetings_not_found");
    let mut store = e.open();
    store.create_meeting(&new_meeting(1, 14)).unwrap();
    let missing = id(99);
    assert_not_found(store.meeting(missing), missing);
    assert_not_found(
        store.finish_meeting(missing, at(15, 0, 0), 1, StopReason::User),
        missing,
    );
    assert_not_found(store.save_meeting_annotations(missing, "x"), missing);
    assert_not_found(
        store.replace_meeting_segments(missing, vec![], at(15, 0, 0)),
        missing,
    );
    assert_not_found(store.save_meeting_notes(missing, "geral", "x"), missing);
    assert_not_found(store.set_meeting_audio_retained(missing), missing);
    assert_not_found(store.meeting_segments(missing), missing);
    assert_not_found(
        store.start_meeting_recording(missing, SessionMode::Meeting, at(15, 0, 0)),
        missing,
    );
    assert_not_found(store.set_meeting_title(missing, "x"), missing);
}

#[test]
fn draft_takes_agenda_and_records_once() {
    let e = env("meetings_draft");
    let store = e.open();
    let meeting = new_meeting(1, 9);
    store.create_meeting(&meeting).unwrap();
    store.set_meeting_title(meeting.id, "1:1 com Ana").unwrap();
    store
        .save_meeting_annotations(meeting.id, "pauta: contrato\nprazo")
        .unwrap();

    let draft = store.meeting(meeting.id).unwrap();
    assert_eq!(draft.started_at, None);
    assert_eq!(draft.title, "1:1 com Ana");
    assert_eq!(draft.annotations, "pauta: contrato\nprazo");

    // Um rascunho não termina sem ter começado.
    assert!(store
        .finish_meeting(meeting.id, at(9, 30, 0), 1, StopReason::User)
        .is_err());

    store
        .start_meeting_recording(meeting.id, SessionMode::Meeting, at(10, 0, 0))
        .unwrap();
    let started = store.meeting(meeting.id).unwrap();
    assert_eq!(started.started_at, Some(at(10, 0, 0)));
    assert_eq!(started.mode, SessionMode::Meeting);
    assert_eq!(started.annotations, "pauta: contrato\nprazo");

    match store.start_meeting_recording(meeting.id, SessionMode::Meeting, at(11, 0, 0)) {
        Err(StorageError::MeetingAlreadyStarted(got)) => assert_eq!(got, meeting.id.to_string()),
        other => panic!("esperava MeetingAlreadyStarted, veio {other:?}"),
    }
    assert_eq!(
        store.meeting(meeting.id).unwrap().started_at,
        Some(at(10, 0, 0))
    );
}

#[test]
fn schema_constraints() {
    let e = env("meetings_constraints");
    let mut store = e.open();
    let meeting = recorded(&store, 1, 14);
    store
        .replace_meeting_segments(
            meeting.id,
            vec![segment(0, SegmentSpeaker::Me, "oi")],
            at(15, 0, 0),
        )
        .unwrap();
    for mode in [
        SessionMode::Meeting,
        SessionMode::SystemOnly,
        SessionMode::Import,
    ] {
        let mut other = new_meeting(2, 10);
        other.id = SessionId::from_parts(1_800_000_000_000, mode as u128);
        other.mode = mode;
        store.create_meeting(&other).unwrap();
        assert_eq!(store.meeting(other.id).unwrap().mode, mode);
    }
    drop(store);

    let conn = Connection::open(&e.db).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
    let bad = conn.execute(
        "INSERT INTO meetings (id, title, mode, created_at, created_ms) VALUES ('X', '', 'outro', '', 0)",
        [],
    );
    assert!(bad.is_err(), "o CHECK de mode devia recusar 'outro'");
    conn.execute(
        "DELETE FROM meetings WHERE id = ?1",
        [meeting.id.to_string()],
    )
    .unwrap();
    let left: i64 = conn
        .query_row("SELECT count(*) FROM meeting_segments", [], |r| r.get(0))
        .unwrap();
    assert_eq!(left, 0);
}
