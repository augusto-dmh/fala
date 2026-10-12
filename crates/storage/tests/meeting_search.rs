//! Busca nas reuniões (`meetings_fts`): sem acento, em título, anotações, notas e trechos.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{at, env};
use fala_meeting::{SessionId, SessionMode};
use fala_storage::{
    MeetingSegment, NewMeeting, SegmentChannel, SegmentSpeaker, Store, MATCH_END, MATCH_START,
};
use rusqlite::Connection;

fn create(store: &Store, n: u64, title: &str, hour: u32) -> SessionId {
    let id = SessionId::from_parts(1_727_000_000_000 + n, u128::from(n));
    store
        .create_meeting(&NewMeeting {
            id,
            title: title.to_string(),
            mode: SessionMode::Meeting,
            local_only: false,
            created_at: at(hour, 0, 0),
        })
        .unwrap();
    id
}

fn transcribe(store: &mut Store, id: SessionId, texts: &[&str]) {
    let segments = texts
        .iter()
        .enumerate()
        .map(|(i, text)| MeetingSegment {
            seq: 0,
            channel: SegmentChannel::System,
            speaker: SegmentSpeaker::Person(1),
            t0_ms: i as u64 * 1_000,
            t1_ms: i as u64 * 1_000 + 900,
            text: text.to_string(),
        })
        .collect();
    store
        .replace_meeting_segments(id, segments, at(18, 0, 0))
        .unwrap();
}

fn ids(store: &Store, query: &str) -> Vec<SessionId> {
    store
        .search_meetings(query, 20)
        .unwrap()
        .into_iter()
        .map(|hit| hit.id)
        .collect()
}

#[test]
fn finds_every_part_without_accents_newest_first() {
    let e = env("meeting_search_parts");
    let mut store = e.open();
    let old = create(&store, 1, "Orçamento anual", 9);
    let new = create(&store, 2, "Daily", 10);
    store
        .save_meeting_annotations(old, "revisar a previsão")
        .unwrap();
    transcribe(
        &mut store,
        new,
        &["A reunião de orçamento ficou para sexta."],
    );
    store
        .save_meeting_notes(new, "geral", "## Notas\n\n- Decisão: contratar.\n")
        .unwrap();

    // Título e trecho, sem acento, prefixo; a mais nova primeiro.
    assert_eq!(ids(&store, "orcam"), [new, old]);
    assert_eq!(ids(&store, "PREVISAO"), [old]);
    assert_eq!(ids(&store, "decisao contrat"), [new]);
    // Todos os termos precisam casar, em qualquer coluna.
    assert_eq!(ids(&store, "daily sexta"), [new]);
    assert!(ids(&store, "daily previsao").is_empty());
    // Sintaxe do FTS5 é texto; consulta vazia ou só de pontuação não acha nada.
    assert!(ids(&store, "orc* OR \"").is_empty());
    assert!(ids(&store, "   ").is_empty());

    let hit = &store.search_meetings("sexta", 20).unwrap()[0];
    assert!(
        hit.snippet
            .contains(&format!("{MATCH_START}sexta{MATCH_END}")),
        "{:?}",
        hit.snippet
    );
}

#[test]
fn changes_and_deletes_reach_the_index() {
    let e = env("meeting_search_changes");
    let mut store = e.open();
    let id = create(&store, 1, "Kickoff", 9);
    transcribe(&mut store, id, &["primeira versão"]);
    assert_eq!(ids(&store, "primeira"), [id]);

    transcribe(&mut store, id, &["segunda versão"]);
    assert!(ids(&store, "primeira").is_empty());
    assert_eq!(ids(&store, "segunda"), [id]);

    store.set_meeting_title(id, "Retrospectiva").unwrap();
    assert!(ids(&store, "kickoff").is_empty());
    assert_eq!(ids(&store, "retro"), [id]);

    store.delete_meeting(id).unwrap();
    assert!(ids(&store, "retro").is_empty());
}

#[test]
fn an_index_with_gaps_is_rebuilt() {
    let e = env("meeting_search_rebuild");
    let mut store = e.open();
    let id = create(&store, 1, "Planejamento", 9);
    assert_eq!(store.index_meetings_for_search().unwrap(), 0);

    // Um banco de antes da busca: as sessões existem, o índice não.
    Connection::open(&e.db)
        .unwrap()
        .execute("DELETE FROM meetings_fts", [])
        .unwrap();
    assert!(ids(&store, "planejamento").is_empty());
    assert_eq!(store.index_meetings_for_search().unwrap(), 1);
    assert_eq!(ids(&store, "planejamento"), [id]);
}

#[test]
fn reindexed_meetings_are_searchable() {
    let e = env("meeting_search_reindex");
    let store = e.open();
    let id = create(&store, 1, "Revisão trimestral", 9);
    drop(store);
    std::fs::remove_file(&e.db).unwrap();
    let mut store = e.open();
    assert_eq!(store.reindex_meetings().unwrap().indexed, 1);
    assert_eq!(ids(&store, "revisao"), [id]);
}
