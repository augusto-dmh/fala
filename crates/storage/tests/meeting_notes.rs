//! A nota de reunião editada à mão e a marca que faz "Gerar notas" pedir confirmação.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{at, env};
use fala_meeting::{SessionId, SessionMode};
use fala_storage::{NewMeeting, StorageError, Store};
use rusqlite::Connection;

fn meeting(store: &Store, n: u64) -> SessionId {
    let id = SessionId::from_parts(1_727_000_000_000 + n, u128::from(n));
    store
        .create_meeting(&NewMeeting {
            id,
            title: "Planejamento".to_string(),
            mode: SessionMode::Meeting,
            local_only: false,
            created_at: at(14, 0, 0),
        })
        .unwrap();
    id
}

#[test]
fn edit_marks_the_notes_and_a_generation_clears_the_mark() {
    let e = env("notes_edit_mark");
    let mut store = e.open();
    let id = meeting(&store, 1);
    assert!(!store.meeting_notes_edited(id).unwrap());

    // Sem nota gerada não há o que editar.
    assert!(matches!(
        store.save_edited_meeting_notes(id, "minha nota", at(15, 0, 0)),
        Err(StorageError::NotFound(_))
    ));
    assert!(!store.meeting_notes_edited(id).unwrap());

    store
        .save_meeting_notes(id, "geral", "- gerada <!-- fala:ia -->")
        .unwrap();
    assert!(!store.meeting_notes_edited(id).unwrap());
    store
        .save_edited_meeting_notes(id, "- minha versão", at(15, 0, 0))
        .unwrap();
    store
        .save_edited_meeting_notes(id, "- minha versão final", at(15, 1, 0))
        .unwrap();
    let record = store.meeting(id).unwrap();
    assert_eq!(record.notes_md.as_deref(), Some("- minha versão final"));
    assert_eq!(record.notes_template.as_deref(), Some("geral"));
    assert!(store.meeting_notes_edited(id).unwrap());

    // A marca sobrevive a reabrir o banco.
    drop(store);
    let store = e.open();
    assert!(store.meeting_notes_edited(id).unwrap());

    store
        .save_meeting_notes(id, "geral", "- outra <!-- fala:ia -->")
        .unwrap();
    assert!(!store.meeting_notes_edited(id).unwrap());

    let missing = SessionId::from_parts(1, 1);
    assert!(matches!(
        store.meeting_notes_edited(missing),
        Err(StorageError::NotFound(_))
    ));
}

#[test]
fn mark_goes_with_the_meeting() {
    let e = env("notes_edit_cascade");
    let mut store = e.open();
    let id = meeting(&store, 1);
    store.save_meeting_notes(id, "geral", "- gerada").unwrap();
    store
        .save_edited_meeting_notes(id, "- minha", at(15, 0, 0))
        .unwrap();
    drop(store);

    let conn = Connection::open(&e.db).unwrap();
    conn.pragma_update(None, "foreign_keys", true).unwrap();
    conn.execute("DELETE FROM meetings WHERE id = ?1", [id.to_string()])
        .unwrap();
    let left: i64 = conn
        .query_row("SELECT count(*) FROM meeting_note_edits", [], |r| r.get(0))
        .unwrap();
    assert_eq!(left, 0);
}
