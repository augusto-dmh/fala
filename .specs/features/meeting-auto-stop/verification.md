# meeting-auto-stop verification

Windows 11 (Alienware), branch `feat/meeting-auto-stop`, `CARGO_TARGET_DIR=C:\f\meeting-auto-stop`.

| Check | Result | Evidence |
| --- | --- | --- |
| C1 | pass | `cargo test -p fala-meeting --test detect`: `an_expired_ask_stops_only_if_the_system_stayed_quiet` ok |
| C2 | pass | same run: `a_call_that_ended_stops_as_silence_and_only_while_recording` ok |
| C3 | pass | `cargo test -p fala-meeting`: 76 passed (detect 16 with `ask_stop_waits_three_minutes_and_stops_after_two` and `never_starts_recording_without_a_click`) |
| C4 | pass | `cargo test -p fala --lib -- call_prompt meeting::`: 73 passed, incl. `answer_stops_a_quiet_call_when_nobody_answers`, `answer_maps_each_stop_button` |
| C5 | pass | same run: `answer_resumes_the_session_the_call_stopped`, `meeting::call_end::tests::the_transcription_waits_only_for_its_own_offer` |
| C6 | pass | `bun src/callprompt/callprompt.test.tsx`: 7 sections ok, incl. "meeting-auto-stop card ok" and "meeting-auto-stop update ok" (a re-sent card keeps countdown and hover); `bun run check:translations` ok |
| C7 | open | manual with "Meet now", needs Augusto (no real call from the agent) |

Measurement behind `CALL_QUIET_RMS`: the three retained `sys.opus` of 2026-10-14 (ffmpeg `astats`, 250 ms windows):
calls at −45…−55 dBFS (p50 −54/−56/−61), 25–33 % of windows between −60 and −70, idle at ≈ −100,
longest quiet run inside a call 3.75 s against a ≥ 20 s decision window.

`/code-review high`: 8 findings. Fixed: in-person mic counts as sound, [Retomar] retries Busy for
2 min, no "Gravou por engano?" after a call-end stop, re-sent card keeps countdown and hover,
[Continuar gravando] disarms, one disarm path for pause and sleep. Kept: quitting during the
[Retomar] window skips the automatic transcription (as quitting during one does today); bounded
250 ms polling instead of a channel.

Also run: `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check`,
`bun run lint`, `bun run format:check`, `bunx tsc --noEmit` clean; `src/bindings.ts` from `fala.exe --list-devices`.
