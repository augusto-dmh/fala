# paste-last verification

**Verdict**: PASS (automated checks); C11 not run
**Profile**: light
**Diff range**: cca0a87..83dc1d0
**Round**: 1 - full
**Verifier**: the author, in the same session, on the Windows 11 machine (Alienware 16). Not an independent verifier. The person was not at the keyboard, so the manual check (C11) was left as a script in the PR.

Every `cargo` ran with `$env:CARGO_TARGET_DIR='C:\f\shortcuts-last-text'`.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | old store gets both bindings with no key; other keys kept; a chosen key survives | `cargo test -p fala --lib` at 83dc1d0 | `settings::tests::old_store_gets_last_dictation_bindings_without_a_key ... ok` | PASS |
| C2 | `to_paste`: busy → none, no history → none, idle → text | same run | `last_dictation::tests::paste_is_ignored_while_busy_and_without_history ... ok` | PASS |
| C3 | `text_of`: final, raw after undo, raw when emptied, history.db text, none for blank | same run | `last_dictation::tests::text_is_what_the_history_shows ... ok` | PASS |
| C4 | `ACTION_MAP` has `paste_last` (stop) and `copy_last` (start) | grep and reading `actions.rs` | both entries; `PasteLastAction::stop` calls `paste_last(app, false)`, `CopyLastAction::start` calls `copy_last` | PASS |
| C5 | paste through `utils::paste` on the main thread, `paste-error` on failure, no text in logs | reading `last_dictation.rs::paste_last` | `run_on_main_thread(move \|\| match crate::utils::paste(text, ...))`; `Err` emits `paste-error`; the log lines carry no text | PASS |
| C6 | whole crate green, clippy, fmt, scripts | `cargo test -p fala --lib`; `cargo clippy -p fala --all-targets -- -D warnings`; `cargo fmt --all -- --check`; `scripts/check-brand.sh`; `scripts/check-no-tauri-in-crates.sh` | `428 passed; 0 failed; 1 ignored`; clippy `Finished` with no warnings; fmt exit 0; both scripts `ok` | PASS |
| C7 | idle menu: paste before copy; busy: copy only | same test run | `tray::tests::paste_last_is_offered_only_while_idle ... ok` | PASS |
| C8 | General renders both rows; pt and en strings | grep; `bun run check:translations` | both `ShortcutInput` lines found; `All 1 languages have complete translations` | PASS |
| C9 | an unset chip shows `settings.general.shortcut.unset` in both inputs | grep | found in `FalaKeysShortcutInput.tsx` and `GlobalShortcutInput.tsx` | PASS |
| C10 | frontend clean | `bun run lint`; `bunx prettier --check .`; `bun run build`; `bun src/components/shell.test.tsx` | lint exit 0; `All matched files use Prettier code style!`; `built in 50.42s`; shell test `C28`–`C32 ok` | PASS |
| C11 | Windows manual: Notepad, Windows Terminal, tray, during a dictation | not run | needs the keyboard and screen | NOT RUN |

## Not tested

- C11 as a whole: no live paste-last into Notepad or Windows Terminal, no tray click. The focus hand-back after the tray menu and the wait for the modifiers are untested assumptions (plan § Assumptions).
- Linux: the tray item and shortcuts compile there; pasting from the tray does not hand the focus back (Windows only).
