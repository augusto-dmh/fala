# meeting-detection-polish verification

Windows 11 (Alienware), `CARGO_TARGET_DIR=C:\f\meeting-detection-polish`, 2026-10-12.

| Check | Result | Evidence |
| --- | --- | --- |
| C1 | pass | `cargo test -p fala-meeting`: `browser_titles_name_the_web_call`, lib `detect::tests` (`words_need_a_boundary`, `every_browser_is_a_call_app`) |
| C2 | pass | `a_browser_offers_only_once_a_window_shows_a_meeting` |
| C3 | pass | `switching_tabs_during_a_recorded_meet_is_not_leaving_it`, `asks_only_when_every_call_app_left` |
| C4 | pass | `ask_stop_waits_three_minutes_and_stops_after_two` |
| C5 | pass | `cargo test -p fala-meeting`: lib 13, `detect` 14, `route` 10, `session` 25, others 3 — 65 passed, 0 failed |
| C6 | pass | `cargo test -p fala --lib call_prompt`: 8 passed (`answer_payload_names_the_meeting_in_the_browser` new) |
| C7 | pass | `bun src/callprompt/callprompt.test.tsx`: 4 sections ok, incl. "meeting-detection-polish web title ok" |
| C8 | open | manual with real calls; script below |

Also: `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets -- -D warnings`
clean; `bun run lint` clean; `bunx prettier --check .` clean; `bunx tsc --noEmit` exit 0;
`bun run check:translations` complete. `src/bindings.ts` regenerated from the debug build
(`fala.exe --list-devices`, portable): only `web_name` added.

## Review (code-review, high)

Fixed: a browser hold without a meeting was settled during a recording, so a Meet opened later in
the same hold was never offered (test added to C2); titles were re-read every second after the
meeting was known (the watcher now reuses the detector's); the browser list was kept apart from
`CALL_APPS` (now a `browser` flag in the table); the info log printed the whole `CallPrompt`
(now kind, app id and meeting name). Kept as decisions: titles of every window of the browser,
not only the tab holding the mic (the title approach cannot tell which tab captures); word
markers like "Microsoft Teams" or "Zoom" can match a non-call page while the browser holds the
mic for something else; [Não avisar] mutes the browser; a Meet answered inside Gmail or
Calendar is no longer offered.

## Not tested (C8, needs Augusto and real calls)

1. claude.ai voice mode in Edge for 30 s: no card.
2. Join a Meet in Edge: within ~2 s, "Parece uma chamada do Google Meet no Edge". Confirm the
   window title starts with "Meet - " or "Meet – " (the marker the code relies on).
3. Record over 3 min, leave the call, rejoin and leave twice more inside the recording:
   at most two "Parar a gravação?" cards, the second at least 3 min after the first.
4. During a recorded Meet, switch Edge to another tab for 10 s: no "Parar a gravação?".
