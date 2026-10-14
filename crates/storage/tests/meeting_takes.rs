//! Retomar uma reunião: cada gravação a mais é uma tomada com deslocamento na linha do tempo, a
//! transcrição dela entra depois da que existe, e o `.md` leva as tomadas para o `reindex`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs;

use common::{at, env};
use fala_meeting::{SessionId, SessionMode, StopReason};
use fala_storage::{
    MeetingSegment, NewMeeting, SegmentChannel, SegmentSpeaker, StorageError, Store,
};

fn id(n: u64) -> SessionId {
    SessionId::from_parts(1_727_000_000_000 + n, u128::from(n))
}

fn line(t0_ms: u64, speaker: SegmentSpeaker, text: &str) -> MeetingSegment {
    MeetingSegment {
        seq: 0,
        channel: if speaker == SegmentSpeaker::Me {
            SegmentChannel::Mic
        } else {
            SegmentChannel::System
        },
        speaker,
        t0_ms,
        t1_ms: t0_ms + 2_000,
        text: text.to_string(),
    }
}

fn create(store: &Store, n: u64, mode: SessionMode) -> SessionId {
    store
        .create_meeting(&NewMeeting {
            id: id(n),
            title: "Planejamento".to_string(),
            mode,
            local_only: false,
            created_at: at(14, 0, 0),
        })
        .unwrap();
    id(n)
}

/// Uma reunião de 60 s encerrada às 14:01, com Eu e "Ana" (Pessoa 1).
fn recorded(store: &mut Store) -> SessionId {
    let id = create(store, 1, SessionMode::Meeting);
    store
        .start_meeting_recording(id, SessionMode::Meeting, at(14, 0, 0))
        .unwrap();
    store
        .finish_meeting(id, at(14, 1, 0), 60_000, StopReason::User)
        .unwrap();
    store
        .replace_meeting_segments(
            id,
            vec![
                line(0, SegmentSpeaker::Me, "Vamos começar."),
                line(10_000, SegmentSpeaker::Person(1), "Proponho sexta."),
            ],
            at(14, 2, 0),
        )
        .unwrap();
    store.set_meeting_speaker_name(id, 1, Some("Ana")).unwrap();
    id
}

fn texts(store: &Store, id: SessionId) -> Vec<(u32, u64, SegmentSpeaker, String)> {
    store
        .meeting_segments(id)
        .unwrap()
        .into_iter()
        .map(|s| (s.seq, s.t0_ms, s.speaker, s.text))
        .collect()
}

#[test]
fn a_take_continues_the_timeline_and_its_transcript_comes_after() {
    let e = env("meeting_takes_append");
    let mut store = e.open();
    let id = recorded(&mut store);

    // Só uma sessão encerrada, gravada e não importada retoma.
    let draft = create(&store, 2, SessionMode::Meeting);
    let import = create(&store, 3, SessionMode::Import);
    store
        .start_meeting_recording(import, SessionMode::Import, at(14, 0, 0))
        .unwrap();
    store
        .finish_meeting(import, at(14, 1, 0), 0, StopReason::User)
        .unwrap();
    for refused in [draft, import] {
        assert!(matches!(
            store.start_meeting_take(refused, at(15, 0, 0)),
            Err(StorageError::NotResumable(_))
        ));
    }

    let take = store.start_meeting_take(id, at(15, 42, 0)).unwrap();
    assert_eq!((take.take, take.offset_ms), (2, 60_000));
    let open = store.meeting(id).unwrap();
    assert_eq!((open.ended_at, open.stop_reason), (None, None));
    // Gravando a tomada, não retoma de novo.
    assert!(matches!(
        store.start_meeting_take(id, at(15, 43, 0)),
        Err(StorageError::NotResumable(_))
    ));

    let done = store
        .finish_meeting_take(id, at(15, 50, 0), 30_000, Some(StopReason::User))
        .unwrap();
    assert_eq!(done.recorded_ms, 30_000);
    let record = store.meeting(id).unwrap();
    assert_eq!(record.recorded_ms, 90_000);
    assert_eq!(record.ended_at, Some(at(15, 50, 0)));
    assert_eq!(record.stop_reason, Some(StopReason::User));
    assert_eq!(
        store.meeting_takes(id).unwrap(),
        std::slice::from_ref(&done)
    );

    // A transcrição da tomada, em tempos dela: a Pessoa 1 dela é outra pessoa (a 2 da sessão).
    store
        .append_meeting_segments(
            id,
            vec![
                line(0, SegmentSpeaker::Me, "Voltamos."),
                line(5_000, SegmentSpeaker::Person(1), "Eu cheguei agora."),
            ],
            done.offset_ms,
            at(15, 51, 0),
        )
        .unwrap();
    assert_eq!(
        texts(&store, id),
        [
            (1, 0, SegmentSpeaker::Me, "Vamos começar.".to_string()),
            (
                2,
                10_000,
                SegmentSpeaker::Person(1),
                "Proponho sexta.".to_string()
            ),
            (
                3,
                60_000,
                SegmentSpeaker::Resumed,
                "[gravação retomada às 15:42]".to_string()
            ),
            (4, 60_000, SegmentSpeaker::Me, "Voltamos.".to_string()),
            (
                5,
                65_000,
                SegmentSpeaker::Person(2),
                "Eu cheguei agora.".to_string()
            ),
        ]
    );
    // O nome dado à Pessoa 1 fica.
    assert_eq!(
        store.meeting_speakers(id).unwrap()[0].name.as_deref(),
        Some("Ana")
    );

    // Uma transcrição nova da sessão inteira mantém o marcador da tomada.
    store
        .replace_meeting_segments(
            id,
            vec![line(61_000, SegmentSpeaker::Me, "Voltamos.")],
            at(16, 0, 0),
        )
        .unwrap();
    assert_eq!(
        texts(&store, id)
            .into_iter()
            .map(|(_, _, speaker, _)| speaker)
            .collect::<Vec<_>>(),
        [SegmentSpeaker::Resumed, SegmentSpeaker::Me]
    );
}

#[test]
fn reindex_brings_the_takes_back() {
    let e = env("meeting_takes_reindex");
    let mut store = e.open();
    let id = recorded(&mut store);
    store.start_meeting_take(id, at(15, 42, 0)).unwrap();
    store
        .finish_meeting_take(id, at(15, 50, 0), 30_000, None)
        .unwrap();
    store
        .append_meeting_segments(
            id,
            vec![line(0, SegmentSpeaker::Me, "Voltamos.")],
            60_000,
            at(15, 51, 0),
        )
        .unwrap();
    let md =
        fs::read_to_string(e.notes.join("Reuniões").join("2026-10-02 Planejamento.md")).unwrap();
    assert!(md.contains("\ntakes: [{\""), "{md}");
    assert!(
        md.contains("- **[01:00] Gravação retomada:** [gravação retomada às 15:42] ^s3\n"),
        "{md}"
    );
    let before = (
        store.meeting(id).unwrap(),
        store.meeting_takes(id).unwrap(),
        store.meeting_segments(id).unwrap(),
    );

    drop(store);
    fs::remove_file(&e.db).unwrap();
    let mut store = e.open();
    assert_eq!(store.reindex_meetings().unwrap().indexed, 1);
    let after = (
        store.meeting(id).unwrap(),
        store.meeting_takes(id).unwrap(),
        store.meeting_segments(id).unwrap(),
    );
    assert_eq!(after, before);
    // A próxima tomada é a 3, depois dos 90 s.
    let next = store.start_meeting_take(id, at(16, 0, 0)).unwrap();
    assert_eq!((next.take, next.offset_ms), (3, 90_000));
}
