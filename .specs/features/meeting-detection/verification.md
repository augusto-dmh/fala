# meeting-detection verification

**Verdict**: PASS for C1, C3-C15 · C2 partial (one reading on this machine) · C16 open (manual, real calls)
**Profile**: light
**Diff range**: `cca0a87..feat/meeting-detection-prompt` (PR 1 `feat/meeting-detection`, PR 2 stacked)
**Round**: 1 - full
**Verifier**: the author (executor pane). No independent verifier ran; that is a gap of this round, not a claim of independence

Windows 11 (Alienware), `CARGO_TARGET_DIR=C:\f\meeting-detection`, one heavy cargo command at a time
under the shared cargo lock.

## Checks

| Check | Claim | Proof run | Result |
| --- | --- | --- | --- |
| C1 | ancestors stop at a gap, a cycle or 4 levels; exe names lose path and case | `cargo test -p fala-audio --lib activity` → 4 passed | PASS |
| C2 | `mic_holders` lists who captures the mic, opens no stream | `cargo run -p fala-audio --example mic_holders` → `voicemeeterpro.exe <- explorer.exe` (Voicemeeter holds the default input on this machine) | PARTIAL: no call app was running; Teams/WebView2 chains not seen live |
| C3-C11 | identify, one offer per call, flicker, gap, muted, recording, ask-stop rules, ADR-0005 | `cargo test -p fala-meeting` → 49 passed (10 in `tests/detect.rs`, 1 in `detect::tests`) | PASS |
| C12 | placement from work area and scale | `call_prompt::tests::placement_top_right ... ok` | PASS |
| C13 | answers map to actions; stale answers do nothing; muted ids once | `call_prompt::tests::answer_*` → 5 ok; `cargo test -p fala --lib` → 433 passed, 1 ignored | PASS |
| C14 | `UserAction::StartRecording` only in `meeting/mod.rs` | `grep -rn "UserAction::StartRecording" apps/desktop/src` → no hit outside `meeting/mod.rs` | PASS |
| C15 | 20 s countdown pauses on hover, card buttons, keys in pt/en | `bun src/callprompt/callprompt.test.tsx` → `C15 countdown ok`, `C15 card ok`, `C15 keys ok`; `bun run check:translations`, `bun run lint`, `bunx tsc --noEmit`, `bunx prettier --check .` clean | PASS |
| C16 | real calls: Teams, Meet in Edge, WhatsApp ≤ 3 s; muted WhatsApp silent; "Parar?" after hanging up | not run: the run had no free PC and no live calls, and showing windows on the desktop was out of bounds | OPEN |

Also: `cargo clippy --workspace --all-targets -- -D warnings` clean.

## Not tested

- The prompt window itself was never shown: position on a real monitor, no focus stolen, hover
  pausing the countdown in WebView2, and the buttons in a non-focusable window.
- The non-Windows `Unsupported` path runs only in the Linux CI job.
- How Teams, WhatsApp and the browsers name their capturing process on this Windows build (the
  table follows Granola's list and the WebView2 parent rule).
