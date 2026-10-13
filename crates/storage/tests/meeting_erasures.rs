//! Apagar um trecho do texto de uma reunião: as falas que cruzam o intervalo saem nos dois
//! canais, um marcador "[trecho apagado]" fica, os ponteiros das notas seguem os números novos, e
//! o `reindex` e uma transcrição nova mantêm o apagado (ADR-0006).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs;

use common::{at, env};
use fala_meeting::{SessionId, SessionMode, StopReason};
use fala_storage::{
    erased_segment, MeetingSegment, NewMeeting, SegmentChannel, SegmentSpeaker, Store, ERASED_TEXT,
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
        t1_ms: t0_ms + 4_000,
        text: text.to_string(),
    }
}

/// s1 Eu 0 s, s2 Pessoa 1 10 s, s3 Eu 20 s (a senha), s4 Pessoa 1 25 s, s5 Eu 40 s.
fn session(store: &mut Store) -> SessionId {
    let id = id(1);
    store
        .create_meeting(&NewMeeting {
            id,
            title: "Planejamento".to_string(),
            mode: SessionMode::Meeting,
            local_only: false,
            created_at: at(14, 0, 0),
        })
        .unwrap();
    store
        .start_meeting_recording(id, SessionMode::Meeting, at(14, 0, 0))
        .unwrap();
    store
        .finish_meeting(id, at(14, 1, 0), 60_000, StopReason::User)
        .unwrap();
    let person = SegmentSpeaker::Person(1);
    store
        .replace_meeting_segments(
            id,
            vec![
                line(0, SegmentSpeaker::Me, "Vamos começar."),
                line(10_000, person, "Proponho sexta."),
                line(20_000, SegmentSpeaker::Me, "A senha é hipopótamo."),
                line(25_000, person, "Anotado, hipopótamo."),
                line(40_000, SegmentSpeaker::Me, "Fechado."),
            ],
            at(14, 2, 0),
        )
        .unwrap();
    store.set_meeting_speaker_name(id, 1, Some("Ana")).unwrap();
    store
        .save_meeting_notes(id, "geral", "## Notas\n\n- Sexta. [[#^s2|00:10]] <!-- fala:ia -->\n- Senha. [[#^s3|00:20]] [[#^s4|00:25]] <!-- fala:ia -->\n- Fim. [[#^s5|00:40]] <!-- fala:ia -->\n")
        .unwrap();
    store
        .save_edited_meeting_notes(
            id,
            "## Notas\n\n- Sexta. [[#^s2|00:10]] <!-- fala:ia -->\n- Senha. [[#^s3|00:20]] [[#^s4|00:25]] <!-- fala:ia -->\n- Fim, confirmado. [[#^s5|00:40]]\n",
            at(14, 3, 0),
        )
        .unwrap();
    id
}

fn texts(store: &Store, id: SessionId) -> Vec<(u32, String)> {
    store
        .meeting_segments(id)
        .unwrap()
        .into_iter()
        .map(|s| (s.seq, s.text))
        .collect()
}

#[test]
fn erasing_takes_both_channels_and_remaps_the_notes() {
    let e = env("meeting_erase_range");
    let mut store = e.open();
    let id = session(&mut store);

    // Da fala do mic às 20 s até o fim da do sistema às 29 s.
    let erased = store
        .erase_meeting_range(id, (20_000, 29_000), fala_notes::remap_pointers)
        .unwrap();
    assert_eq!(erased.removed, 2);
    assert_eq!(erased.notes_lines, 1);
    assert_eq!(
        texts(&store, id),
        [
            (1, "Vamos começar.".to_string()),
            (2, "Proponho sexta.".to_string()),
            (3, ERASED_TEXT.to_string()),
            (4, "Fechado.".to_string()),
        ]
    );
    let segments = store.meeting_segments(id).unwrap();
    assert_eq!(
        segments[2],
        MeetingSegment {
            seq: 3,
            ..erased_segment(20_000, 29_000)
        }
    );
    let notes = store.meeting(id).unwrap().notes_md.unwrap();
    assert_eq!(
        notes,
        "## Notas\n\n- Sexta. [[#^s2|00:10]] <!-- fala:ia -->\n- Senha. (trecho apagado) (trecho apagado) <!-- fala:ia -->\n- Fim, confirmado. [[#^s4|00:40]]\n"
    );
    // A nota editada continua editada, e o nome dado a "Pessoa 1" fica.
    assert!(store.meeting_notes_edited(id).unwrap());
    assert_eq!(
        store.meeting_speakers(id).unwrap()[0].name.as_deref(),
        Some("Ana")
    );
    assert!(store.search_meetings("hipopotamo", 10).unwrap().is_empty());
    assert_eq!(store.meeting_erasures(id).unwrap(), [(20_000, 29_000)]);

    let md =
        fs::read_to_string(e.notes.join("Reuniões").join("2026-10-02 Planejamento.md")).unwrap();
    for want in [
        "- **[00:20] Trecho apagado:** [trecho apagado] ^s3\n",
        "\ns3 mic erased 20000 29000\n",
    ] {
        assert!(md.contains(want), "{want}\n---\n{md}");
    }
    assert!(!md.contains("hipopótamo"), "{md}");
}

#[test]
fn a_new_transcript_keeps_the_erased_stretch_out() {
    let e = env("meeting_erase_retranscribe");
    let mut store = e.open();
    let id = session(&mut store);
    store
        .erase_meeting_range(id, (20_000, 29_000), fala_notes::remap_pointers)
        .unwrap();

    // O ASR devolve texto dentro do trecho (e um marcador de mentira): nada disso entra.
    store
        .replace_meeting_segments(
            id,
            vec![
                line(0, SegmentSpeaker::Me, "Vamos começar."),
                line(21_000, SegmentSpeaker::Me, "hipopótamo"),
                line(26_000, SegmentSpeaker::Person(1), "hipopótamo"),
                erased_segment(1_000, 2_000),
                line(40_000, SegmentSpeaker::Me, "Fechado."),
            ],
            at(15, 0, 0),
        )
        .unwrap();
    assert_eq!(
        texts(&store, id),
        [
            (1, "Vamos começar.".to_string()),
            (2, ERASED_TEXT.to_string()),
            (3, "Fechado.".to_string()),
        ]
    );
}

#[test]
fn reindex_brings_back_the_marker_and_keeps_it_on_the_next_transcript() {
    let e = env("meeting_erase_reindex");
    let mut store = e.open();
    let id = session(&mut store);
    store
        .erase_meeting_range(id, (20_000, 29_000), fala_notes::remap_pointers)
        .unwrap();
    let before = (
        store.meeting_segments(id).unwrap(),
        store.meeting(id).unwrap().notes_md,
        store.search_meetings("sexta", 10).unwrap().len(),
    );

    drop(store);
    fs::remove_file(&e.db).unwrap();
    let mut store = e.open();
    assert_eq!(store.reindex_meetings().unwrap().indexed, 1);
    let after = (
        store.meeting_segments(id).unwrap(),
        store.meeting(id).unwrap().notes_md,
        store.search_meetings("sexta", 10).unwrap().len(),
    );
    assert_eq!(after, before);
    assert_eq!(store.meeting_erasures(id).unwrap(), [(20_000, 29_000)]);
    store
        .replace_meeting_segments(
            id,
            vec![line(22_000, SegmentSpeaker::Me, "hipopótamo")],
            at(15, 0, 0),
        )
        .unwrap();
    assert_eq!(texts(&store, id), [(1, ERASED_TEXT.to_string())]);
}
