//! A lixeira das reuniões: a sessão some da lista e da busca, o `.md` vai para
//! `Reuniões/.lixeira/` com `trashed_at`, e o `reindex` a devolve para a lixeira (ADR-0006).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs;
use std::path::PathBuf;

use common::{at, env, Env};
use fala_meeting::{SessionId, SessionMode, StopReason};
use fala_storage::{
    MeetingSegment, NewMeeting, SegmentChannel, SegmentSpeaker, StorageError, Store, TRASH_DAYS,
};

fn id(n: u64) -> SessionId {
    SessionId::from_parts(1_727_000_000_000 + n, u128::from(n))
}

/// Uma sessão gravada, com pauta e um trecho que a busca acha.
fn recorded(store: &mut Store, n: u64, title: &str, said: &str) -> SessionId {
    let id = id(n);
    store
        .create_meeting(&NewMeeting {
            id,
            title: title.to_string(),
            mode: SessionMode::Meeting,
            local_only: false,
            created_at: at(14, n as u32, 0),
        })
        .unwrap();
    store
        .start_meeting_recording(id, SessionMode::Meeting, at(14, n as u32, 0))
        .unwrap();
    store
        .finish_meeting(id, at(15, 0, 0), 60_000, StopReason::User)
        .unwrap();
    store.save_meeting_annotations(id, "pauta: prazo").unwrap();
    let segment = MeetingSegment {
        seq: 0,
        channel: SegmentChannel::System,
        speaker: SegmentSpeaker::Person(1),
        t0_ms: 1_000,
        t1_ms: 2_000,
        text: said.to_string(),
    };
    store
        .replace_meeting_segments(id, vec![segment], at(15, 1, 0))
        .unwrap();
    id
}

fn reunioes(e: &Env) -> PathBuf {
    e.notes.join("Reuniões")
}

fn lixeira(e: &Env) -> PathBuf {
    reunioes(e).join(".lixeira")
}

fn md_in(dir: &PathBuf) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .map(|entry| entry.unwrap())
                .filter(|entry| entry.path().is_file())
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn found(store: &Store, query: &str) -> Vec<SessionId> {
    store
        .search_meetings(query, 20)
        .unwrap()
        .into_iter()
        .map(|hit| hit.id)
        .collect()
}

fn trashed(store: &Store) -> Vec<(SessionId, String)> {
    store
        .trashed_meetings()
        .unwrap()
        .into_iter()
        .map(|t| (t.record.id, t.trashed_at.to_rfc3339()))
        .collect()
}

#[test]
fn trash_moves_the_md_hides_from_search_and_restore_brings_it_back() {
    let e = env("meeting_trash_move");
    let mut store = e.open();
    let gone = recorded(&mut store, 1, "Contrato", "o contrato sai dia 15");
    let stays = recorded(&mut store, 2, "Daily", "o contrato da Ana");
    assert_eq!(found(&store, "contrato"), [stays, gone]);

    store.trash_meeting(gone, at(16, 0, 0)).unwrap();
    assert_eq!(md_in(&reunioes(&e)), ["2026-10-02 Daily.md"]);
    assert_eq!(md_in(&lixeira(&e)), ["2026-10-02 Contrato.md"]);
    let md = fs::read_to_string(lixeira(&e).join("2026-10-02 Contrato.md")).unwrap();
    assert!(
        md.contains("\ntrashed_at: \"2026-10-02T16:00:00-03:00\"\n---\n"),
        "{md}"
    );
    assert_eq!(found(&store, "contrato"), [stays]);
    // `meetings()` ainda vê tudo: o espelho, o `reindex` e a retenção precisam.
    assert_eq!(store.meetings().unwrap().len(), 2);
    assert_eq!(
        trashed(&store),
        [(gone, "2026-10-02T16:00:00-03:00".to_string())]
    );
    assert_eq!(store.meeting_trashed_at(stays).unwrap(), None);

    // Mover de novo não muda o prazo.
    store.trash_meeting(gone, at(17, 0, 0)).unwrap();
    assert_eq!(
        trashed(&store),
        [(gone, "2026-10-02T16:00:00-03:00".to_string())]
    );
    // Uma mudança na lixeira reescreve o `.md` de lá.
    store.set_meeting_title(gone, "Contrato velho").unwrap();
    assert_eq!(md_in(&lixeira(&e)), ["2026-10-02 Contrato velho.md"]);
    assert_eq!(md_in(&reunioes(&e)), ["2026-10-02 Daily.md"]);

    // Um arquivo de fora tomou o nome em `Reuniões/`: o restaurado ganha `(2)`.
    fs::write(reunioes(&e).join("2026-10-02 Contrato velho.md"), "meu").unwrap();
    store.restore_meeting(gone).unwrap();
    assert!(md_in(&lixeira(&e)).is_empty());
    assert_eq!(
        md_in(&reunioes(&e)),
        [
            "2026-10-02 Contrato velho (2).md",
            "2026-10-02 Contrato velho.md",
            "2026-10-02 Daily.md"
        ]
    );
    let md = fs::read_to_string(reunioes(&e).join("2026-10-02 Contrato velho (2).md")).unwrap();
    assert!(!md.contains("trashed_at"), "{md}");
    assert_eq!(
        fs::read_to_string(reunioes(&e).join("2026-10-02 Contrato velho.md")).unwrap(),
        "meu"
    );
    assert_eq!(found(&store, "contrato"), [stays, gone]);
    assert!(trashed(&store).is_empty());
    assert!(matches!(
        store.restore_meeting(gone),
        Err(StorageError::NotFound(_))
    ));
    assert!(matches!(
        store.trash_meeting(id(9), at(16, 0, 0)),
        Err(StorageError::NotFound(_))
    ));
}

#[test]
fn a_copy_left_in_the_other_folder_goes_on_the_next_sync() {
    let e = env("meeting_trash_stale_copy");
    let mut store = e.open();
    let id = recorded(&mut store, 1, "Contrato", "oi");
    let name = "2026-10-02 Contrato.md";
    // Salvo com CRLF por um editor: ainda é a cópia da sessão.
    let old = fs::read_to_string(reunioes(&e).join(name))
        .unwrap()
        .replace('\n', "\r\n");
    store.trash_meeting(id, at(16, 0, 0)).unwrap();
    // O processo morreu entre escrever na lixeira e apagar o de `Reuniões/`.
    fs::write(reunioes(&e).join(name), &old).unwrap();
    assert_eq!(store.sync_meeting_mirrors().unwrap(), 1);
    assert!(md_in(&reunioes(&e)).is_empty());
    assert_eq!(md_in(&lixeira(&e)), [name]);
    assert_eq!(store.sync_meeting_mirrors().unwrap(), 0);

    // Com as duas cópias, um `reindex` fica com a de fora da lixeira: nada que talvez tenha
    // sido restaurado vai para a purga.
    fs::write(reunioes(&e).join(name), old.replace("\r\n", "\n")).unwrap();
    drop(store);
    fs::remove_file(&e.db).unwrap();
    let mut store = e.open();
    let report = store.reindex_meetings().unwrap();
    assert_eq!(report.indexed, 1, "{report:?}");
    assert_eq!(store.meeting_trashed_at(id).unwrap(), None);
}

#[test]
fn a_file_of_the_person_with_the_same_name_is_never_deleted() {
    let e = env("meeting_trash_foreign_file");
    let mut store = e.open();
    let id = recorded(&mut store, 1, "Contrato", "oi");
    let name = "2026-10-02 Contrato.md";
    store.trash_meeting(id, at(16, 0, 0)).unwrap();
    fs::write(reunioes(&e).join(name), "minha nota").unwrap();
    store.sync_meeting_mirrors().unwrap();
    store.set_meeting_title(id, "Contrato").unwrap();
    store.delete_meeting(id).unwrap();
    assert!(md_in(&lixeira(&e)).is_empty());
    assert_eq!(
        fs::read_to_string(reunioes(&e).join(name)).unwrap(),
        "minha nota"
    );
}

#[test]
fn reindex_brings_the_trash_back_with_the_same_date_and_search() {
    let e = env("meeting_trash_reindex");
    let mut store = e.open();
    let kept = recorded(&mut store, 1, "Daily", "o contrato da Ana");
    let gone = recorded(&mut store, 2, "Contrato", "o contrato sai dia 15");
    store.trash_meeting(gone, at(16, 0, 30)).unwrap();
    let before = (
        store.meetings().unwrap(),
        trashed(&store),
        found(&store, "contrato"),
    );

    drop(store);
    fs::remove_file(&e.db).unwrap();
    let mut store = e.open();
    let report = store.reindex_meetings().unwrap();
    assert_eq!(report.indexed, 2, "{report:?}");
    assert!(report.skipped.is_empty(), "{report:?}");
    let after = (
        store.meetings().unwrap(),
        trashed(&store),
        found(&store, "contrato"),
    );
    assert_eq!(after, before);
    assert_eq!(after.2, [kept]);
    // Nada a reescrever: o `.md` da lixeira é o mesmo.
    assert_eq!(store.sync_meeting_mirrors().unwrap(), 0);
}

#[test]
fn reindex_puts_a_file_of_the_trash_folder_without_a_date_in_the_trash_now() {
    let e = env("meeting_trash_reindex_no_date");
    let mut store = e.open();
    let id = recorded(&mut store, 1, "Contrato", "oi");
    let name = "2026-10-02 Contrato.md";
    // Alguém moveu o `.md` à mão para a lixeira.
    fs::create_dir_all(lixeira(&e)).unwrap();
    fs::rename(reunioes(&e).join(name), lixeira(&e).join(name)).unwrap();
    drop(store);
    fs::remove_file(&e.db).unwrap();

    let mut store = e.open();
    let before = chrono::Local::now().timestamp();
    assert_eq!(store.reindex_meetings().unwrap().indexed, 1);
    let at = store.meeting_trashed_at(id).unwrap().unwrap();
    assert!(at.timestamp() >= before - 1, "{at}");
    // O espelho ganha a data, e o prazo conta dela.
    store.sync_meeting_mirrors().unwrap();
    let md = fs::read_to_string(lixeira(&e).join(name)).unwrap();
    assert!(md.contains("\ntrashed_at: "), "{md}");
}

#[test]
fn due_after_thirty_days_and_delete_removes_the_md_of_the_trash() {
    let e = env("meeting_trash_due");
    let mut store = e.open();
    let old = recorded(&mut store, 1, "Velha", "oi");
    let young = recorded(&mut store, 2, "Nova", "oi");
    recorded(&mut store, 3, "Fora", "oi");
    let day_ms = 86_400_000;
    store.trash_meeting(old, at(10, 0, 0)).unwrap();
    store.trash_meeting(young, at(11, 0, 0)).unwrap();
    let base = at(10, 0, 0).timestamp_millis();
    let days = |d: i64, ms: i64| base + d * day_ms + ms;
    assert_eq!(TRASH_DAYS, 30);
    // 29 dias e 23 h depois, nada vence.
    assert!(store
        .trash_due(days(29, 23 * 3_600_000))
        .unwrap()
        .is_empty());
    assert_eq!(store.trash_due(days(30, 0)).unwrap(), [old]);
    assert_eq!(store.trash_due(days(30, 3_600_000)).unwrap(), [old, young]);

    store.delete_meeting(old).unwrap();
    assert_eq!(md_in(&lixeira(&e)), ["2026-10-02 Nova.md"]);
    assert_eq!(trashed(&store).len(), 1);
    assert_eq!(store.reindex_meetings().unwrap().indexed, 0);
}
