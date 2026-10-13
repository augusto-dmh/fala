# dictation-during-meeting verification

Windows 11 (Alienware), `CARGO_TARGET_DIR=C:\f\dictation-during-meeting`, 2026-10-12, on
`origin/main` `58fd6a2` (#146, the power guard, merged into the recorder thread). Three stacked PRs.

Verdict: **PASS** for C1-C7. M1-M4 are **Not tested** (they need a real call) and do not change
the verdict.

| Check | Proof | Result |
| --- | --- | --- |
| C1 | `cargo test -p fala-audio hub::` | 5 ok |
| C2 | `cargo test -p fala-audio mic::` | ok |
| C3 | `cargo test -p fala-audio meeting::recorder::` (6 omit tests + the 8 adapted to the 700 ms lag) | ok |
| C4 | `cargo test -p fala --lib` (`dictation_is_refused_only_in_person`, `a_dictation_in_a_pause_zeroes_only_what_was_recorded`, `meeting_overlay_states`); `bun src/overlay/pill.test.tsx` (`dictation-during-meeting C4 ok`) | ok |
| C5 | `cargo test -p fala-meeting` (`a_long_dictation_neither_counts_as_silence_nor_resets_it`, `a_dictation_is_neither_silence_nor_sound_on_the_mic`); `cargo test -p fala --lib` (`a_dictation_is_not_speech_of_the_meeting`) | ok |
| C6 | `cargo test -p fala-storage` (`a_dictation_marker_reaches_the_segments_and_the_md`), `cargo test -p fala-notes` (`speaker_labels_follow_language`), `cargo test -p fala --lib` (`segments_round_trip`, `omitted_spans_are_reported_in_ms`), `bun src/components/meeting/meeting.test.tsx` (`dictation-during-meeting C6 ok`) | ok |
| C7 | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p fala --lib` (536 passed, 0 failed, 1 ignored), `-p fala-audio` (54), `-p fala-meeting` (72), `-p fala-storage` (99), `-p fala-notes` (44), `bun run lint`, `bun run format:check`, `bunx tsc --noEmit`, `bun run check:translations`, `scripts/check-brand.sh`, `scripts/check-no-tauri-in-crates.sh` | all exit 0 |

## Settled mid-build

- Size: 1060 lines in the first slice; split into the hub, the recorder, and the desktop with the
  marker.
- Review: `BeginOmit` carries the instant the dictation started, so a late message still reaches
  the pre-buffer (the hold covers 50 ms of delay; more is counted in `omit_late` and logged).
- Review: "mute while recording" no longer mutes the PC during a meeting (it would cut the call
  out of the user's ears and of the system channel).
- Review: the card resizes only the recording pill, never the live streaming card; a stop in
  the middle of a dictation shrinks it back.
- Review: a failed mic stream moves the meeting to the hub's new stream (same rate), so dictation
  and the meeting never end up on two streams.
- Review: the transcript marker does not open or close a speaker's turn; reindex from the `.md`
  rebuilds `meeting_omissions`, so a later transcription keeps the markers.

## Not tested

M1-M4: dictating in the Teams chat during a recorded call (one short dictation, one ~1 min),
checking the marker in the transcript, the silence in the played span and the card over the
pill, and the in-person refusal. These need a real call, which this round cannot record.
