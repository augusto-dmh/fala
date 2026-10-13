# pill-menu verification

Windows 11 (Alienware), `CARGO_TARGET_DIR=C:\f\pill-menu-drag`, 2026-10-12.

Verdict: **PASS** for C1-C5. The Windows manual checks ran with scripted input on the portable
debug app (below); M2 and M3 are **Not tested** and do not change the verdict.

| Check | Proof | Result |
| --- | --- | --- |
| C1 | `cargo test -p fala --lib tray::tests::pill_menu_items_and_their_handlers` | ok |
| C2, C3 | `cargo test -p fala --lib overlay::tests::snooze_hides_only_the_idle_pill_for_an_hour` | ok |
| C4 | `bun src/overlay/pill.test.tsx` (`pill-menu C4 ok`) | ok |
| C5 | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p fala --lib` (522 passed, 0 failed, 1 ignored), `bun run lint`, `bun run format:check`, `bunx tsc --noEmit`, `bun run check:translations`, `scripts/check-brand.sh`, `bun src/overlay/notice.test.tsx` | all exit 0 |

The build log shows the tray strings now carry 20 fields (`hidePillHour` added).

## Settled mid-build

- Review: the snooze timer first slept one hour on the monotonic clock while the end was kept on
  the wall clock, so a clock change could strand the pill hidden. It is now one timer that sleeps
  toward the end and wakes at least every minute (`snooze_wait`); a second snooze reuses it.
- Review: "Configurações" opens the general view, as the rail does, not the last sub-view.
- `src/bindings.ts` regenerated with the portable debug build (`fala.exe --list-devices`): only
  `openPillMenu` was added.

## Manual run (portable debug app, scripted mouse and keys)

`fala.exe` from `C:\f\pill-menu-drag\debug` in portable mode with the Vite dev server, idle pill
on, no model. Input by `SetCursorPos`/`mouse_event`/`keybd_event`, windows read with
`GetWindowRect`/`GetForegroundWindow`, screenshots with `CopyFromScreen`.

| Manual | Result |
| --- | --- |
| M1 | Right click opened the native menu at the cursor: Paste Last Dictation, Dictation language ▸, Microphone ▸, Hide for 1 hour, Settings... (English: the portable store had `app_language: en-US`). The pill held the foreground during the menu; after Esc the foreground went back to the terminal that had it. |
| M2 | Not tested: no dictation in the portable store to paste. |
| M3 | Not tested: switching the microphone would need a person to check the device. |
| M4 | "Hide for 1 hour" hid the pill (`IsWindowVisible` false; log `idle pill hidden for 60 min`). The dictation half is not tested: no model is installed in the portable store. |
| M5 | "Settings..." opened the main window in front (foreground `Fala`) on Configurações → General. |

Found by the run and fixed: the menu holds the pointer, so the page never heard it leave and the
shortcut card stayed open after the menu closed with the cursor elsewhere. `open_pill_menu` now
puts the pill back to `idle` when the cursor is off it.
Re-run after the fix: hover window 315×67, right click, cursor moved away, Esc: the window went
back to the resting 67×21 and the foreground back to the terminal.
