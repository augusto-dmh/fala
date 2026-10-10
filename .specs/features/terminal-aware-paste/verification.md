# terminal-aware-paste verification

**Verdict**: PASS
**Profile**: light
**Diff range**: 3135653..feat/terminal-aware-paste (6cc5385)
**Round**: 1 - full
**Verifier**: the author, in the same session, on the Windows 11 machine (Alienware 16). Not an independent verifier: the manual proof takes over the desktop.

Every `cargo` ran with `$env:CARGO_TARGET_DIR='C:\f\pipe'`.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | terminals true, others/empty false | `cargo test -p fala-inject` at 6cc5385 | `terminal::tests::terminals_by_app_name ... ok` | PASS |
| C2 | `for_app`: terminal → `ShiftInsert`, other/`None` → `CtrlV` | same run | `terminal::tests::chord_for_app ... ok` | PASS |
| C3 | list lowercase, no `.exe`, no spaces, no duplicates | same run | `terminal::tests::list_is_normalized ... ok` | PASS |
| C4 | whole crate green | same run | `test result: ok. 14 passed; 0 failed; 4 ignored` | PASS |
| C5 | desktop: `CtrlV` → `ShiftInsert` in terminals, kept elsewhere and for unknown app | `cargo test -p fala --lib clipboard` | `clipboard::tests::ctrl_v_becomes_shift_insert_in_terminals ... ok` | PASS |
| C6 | the five explicit methods are kept in a terminal | same run | `clipboard::tests::explicit_paste_method_is_kept ... ok`; `6 passed; 0 failed; 398 filtered out` | PASS |
| C7 | switch logged at `debug` with app name, never text | read `clipboard.rs::paste`: `log::debug!("terminal em foco ({}): ...", app.app_name...)` before the `match`; the only other logs in the function are the pre-existing `info!` lines with method and delays | by inspection | PASS |
| C8 | example `--chord auto`: Windows Terminal → `chord=shift_insert result=ok`, text visible; Notepad → `chord=ctrl_v result=ok` | `paste.exe --chord auto --delay 3 <marker>` with each target focused by `AppActivate` | wt: `round=1 chord=shift_insert result=ok restored=y elapsed_ms=228`; Notepad: `round=1 chord=ctrl_v result=ok restored=y elapsed_ms=263`; both exit 0 | PASS |
| C9 | clippy and fmt clean, CI scripts | `cargo clippy -p fala-inject --all-targets -- -D warnings`; `cargo clippy -p fala --all-targets -- -D warnings`; `cargo fmt --all -- --check`; `scripts/check-no-tauri-in-crates.sh`; `scripts/check-brand.sh` | `Finished` with no warnings (5.69 s, 10.74 s); fmt clean after one `cargo fmt`; `ok: no tauri in crates/`; `ok: no Handy branding outside the allowlist` | PASS |
| C10 | Code Map says inject picks the chord per app | `grep -n "acorde por app" ARCHITECTURE.md` | line 24, the `crates/inject` row | PASS |

## Manual proof (C8)

Run on 2026-10-10 17:15 (local), machine on AC, with the person's go-ahead and hands off the keyboard. A PowerShell script started the target (`wt -w fala-c8 pwsh` / `notepad`), launched the example with `--chord auto --delay 3` and a unique marker, and focused the target through `WScript.Shell.AppActivate` one second into the countdown. The chord is chosen only after the countdown, from `foreground_app()` at that moment.

| Target | Foreground app | Chord chosen | Result | Restored | Elapsed |
| --- | --- | --- | --- | --- | --- |
| Windows Terminal (pwsh) | `windowsterminal` | `shift_insert` | `ok` (target read the clipboard) | `y` | 228 ms |
| Notepad (Windows 11) | `notepad` | `ctrl_v` | `ok` | `y` | 263 ms |

"Text visible in the target" was not checked by eye by the verifier (no screen access from the pane); `result=ok` is the delayed-rendering read receipt, and the windows were left open for the person to confirm.

## Not tested

- A live dictation from the running app into a terminal (needs the person at the keyboard); the example sends the same chord through the same `enigo` path the desktop's `paste_tx`/legacy paths use, but it is the crate's `Injector`, not `clipboard.rs::paste`.
- Terminals other than Windows Terminal (conhost, Alacritty, WezTerm, PuTTY): not installed here.
- Linux: `foreground_app` returns `None` there, so the rule never fires; the Linux CI job compiles and runs the crate tests.
