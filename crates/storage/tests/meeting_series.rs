//! As reuniões de uma série: pelo título, fora da lixeira, da mais recente para a mais antiga, e
//! renomear muda a série na hora.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{at, env};
use fala_meeting::{SessionId, SessionMode};
use fala_storage::{series_of, NewMeeting, Store};

fn meeting(store: &Store, n: u64, hour: u32, title: &str) -> SessionId {
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
    store
        .start_meeting_recording(id, SessionMode::Meeting, at(hour, 5, 0))
        .unwrap();
    id
}

#[test]
fn a_series_lists_its_meetings_newest_first() {
    let e = env("series");
    let mut store = e.open();
    let first = meeting(&store, 1, 9, "Weekly produto 06/10");
    let second = meeting(&store, 2, 11, "weekly Produto 13/10");
    let third = meeting(&store, 3, 13, "Weekly produto — 20/10");
    meeting(&store, 4, 14, "Weekly vendas 20/10");
    let trashed = meeting(&store, 5, 15, "Weekly produto 27/10");
    store.trash_meeting(trashed, at(16, 0, 0)).unwrap();

    let key = series_of("Weekly produto 13/10").unwrap().key;
    assert_eq!(store.series_meetings(&key).unwrap(), [third, second, first]);

    store.set_meeting_title(second, "Retro 13/10").unwrap();
    assert_eq!(store.series_meetings(&key).unwrap(), [third, first]);
    assert!(store.series_meetings("ninguem").unwrap().is_empty());
}
