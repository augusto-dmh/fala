# pill-polish verification

Windows 11 (Alienware 16), `CARGO_TARGET_DIR=C:\f\pill-polish`, 2026-10-12.

| Check | Evidence | Result |
| --- | --- | --- |
| C1 | `cargo test -p fala --lib`: `actions::tests::failed_paste_keeps_the_pill_up_to_say_so`, `last_dictation::tests::failed_paste_is_kept_until_the_pill_ends`, `last_dictation::tests::paste_is_ignored_while_busy_and_without_history` | pass |
| C2 | `cargo test -p fala --lib`: `overlay::tests::paste_failed_pill` | pass |
| C3 | `bun src/overlay/pill.test.tsx`: `pill-polish C3` | pass |
| C4 | `bun src/overlay/pill.test.tsx`: `pill-polish C4`, `upstream-catchup PR2 C7` (still two awaits before the pill shows) | pass |
| C5 | `bun src/overlay/pill.test.tsx`: `pill-polish C5`; `cargo test -p fala --lib`: `overlay::tests::meeting_pill_opens_its_session` | pass |
| C6 | `bun run lint`, `bun run format:check`, `bunx tsc --noEmit`, `bun run check:translations`, `cargo clippy --workspace --all-targets -- -D warnings` | clean |

Totals: `cargo test -p fala --lib` 494 passed, 0 failed, 1 ignored; `bun src/overlay/pill.test.tsx`
32 checks ok; `bun src/overlay/notice.test.tsx` ok.

`src/bindings.ts` regenerated from the debug build (`fala.exe --list-devices`, portable): three
commands added (`openMeetingFromPill`, `endPasteFailed`, `copyFailedPaste`), nothing else changed.

## Not tested

Manual on Windows (needs Augusto, about 5 minutes):

1. Dictate with the focus on the desktop (no field takes the paste): the pill says "Não deu para
   colar." for 5 s; with the pointer on it, it stays; the clipboard still holds what it had before.
2. Click the pill: "Copiado"; Ctrl+V in Notepad pastes the dictation.
3. With a "Colar o último ditado" shortcut set, the second line names it; press it with the focus
   in Notepad while the pill is up: the dictation goes in.
4. With a meeting recording, click the pill: the main window opens on Reuniões with the session.
