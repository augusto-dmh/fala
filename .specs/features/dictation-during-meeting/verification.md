# dictation-during-meeting verification

Windows 11 (Alienware), `CARGO_TARGET_DIR=C:\f\dictation-during-meeting`, 2026-10-13, PR 3 on
`origin/main` with #153, #154, #147 (sleep) and #150 merged in.

Verdict: **PASS** for C1-C7. M1-M4 are **Not tested** (they need a real call).

| Check | Proof | Result |
| --- | --- | --- |
| C1, C2 | `cargo test -p fala-audio hub:: mic::` (with `renew_opens_a_new_stream_and_moves_everyone_to_it`) | ok |
| C3 | `cargo test -p fala-audio meeting::recorder::` (omit tests, the mic rate switch included) | ok |
| C4, C5 | `cargo test -p fala --lib` (540 passed, 1 ignored), `cargo test -p fala-meeting` (74), `bun src/overlay/pill.test.tsx` | ok |
| C6 | `cargo test -p fala-storage` (101), `-p fala-notes` (44), `bun src/components/meeting/meeting.test.tsx` | ok |
| C7 | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `bun run lint`, `format:check`, `bunx tsc --noEmit`, `check:translations`, the 13 `src/components/meeting/*.test.tsx` | all exit 0 |

## Settled mid-build

Review fixes (start instant on `BeginOmit`, no PC mute in a meeting, card only on the recording
pill, failed stream moves, turns, reindex refill). After a sleep (#147) the mic reopens with
`MicHub::renew`, so dictation follows and the device keeps one stream.

**Not tested** (M1-M4, need a real call): dictating in the Teams chat during a recording.
