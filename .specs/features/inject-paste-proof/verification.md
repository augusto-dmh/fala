# inject-paste-proof verification

**Verdict**: PASS
**Profile**: light
**Diff range**: eb0482c..feat/inject-paste-proof-example (1ee9d8c offer, fe3b949 sequence, the example commit on top)
**Round**: 1 - full
**Verifier**: the author, in the same session, on the Windows 11 machine (Alienware 16). Not an independent verifier: the manual proofs take over the desktop, and repeating them in another pane would interfere with the person using the machine again (see "Interference" below).

Every `cargo` ran with `$env:CARGO_TARGET_DIR='C:\f\inj'`.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | read paste: `Ok`, full order, 3 chords x 5 saved contents | `cargo test -p fala-inject` at fe3b949 | `paste::tests::read_paste_restores_in_order ... ok` | PASS |
| C2 | unread paste: `PasteNotRead`, restore, `EndOffer` | same run | `paste::tests::unread_paste_is_reported_and_restored ... ok` | PASS |
| C3 | clipboard changed: no restore, `Ok`/`PasteNotRead` kept | same run | `paste::tests::changed_clipboard_is_not_restored ... ok` | PASS |
| C4 | failed chord: no `WaitRead`, restore, `Keystroke` | same run | `paste::tests::failed_chord_still_restores ... ok` | PASS |
| C5 | failed offer: `Clipboard`, no chord, `EndOffer` | same run | `paste::tests::failed_offer_skips_chord ... ok` | PASS |
| C6 | default config CtrlV / 60 / 60 / 100 / 1500 ms | same run | `paste::tests::default_config_matches_desktop ... ok` | PASS |
| C7 | no content in logs; "changed" path logs at DEBUG | same run | `paste::tests::paste_never_logs_text_or_clipboard_content ... ok` | PASS |
| C8 | chord to keys, Shift+Insert = Shift + 0x2D | same run | `paste::windows::tests::chords_map_to_keys ... ok` | PASS |
| C9 | Notepad, Windows Terminal, VS Code: 3 pastes each, read, restored, visible | example, 3 rounds per app (table below) | 9/9 `result=ok restored=y` on clean runs; screenshots show the text 3x in each app | PASS |
| C10 | window with no field: `not_read` in ~1.5 s, restored | `paste.exe --blank` | `round=1 result=not_read restored=y elapsed_ms=1735` (before the refactor) and `elapsed_ms=1730` (final code) | PASS |
| C11 | pasted text is not in Win+V history | history on for one paste, read through WinRT `Clipboard.GetHistoryItemsAsync` | `history_enabled=True`, items: `FALA-EXEMPLO-27100`, `CONTROLE-COPIA-NORMAL`; the pasted `DITADO-SECRETO-WINV` is absent | PASS |
| C12 | workspace clippy clean on Windows | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, `Finished dev profile ... in 7m 52s` (run before the offer was split into `offer.rs`; after the split, `cargo clippy -p fala-inject --all-targets -- -D warnings` exit 0 on every commit of the stack) | PASS |
| C13 | fmt, no-tauri, brand | `cargo fmt --all -- --check`, `scripts/check-no-tauri-in-crates.sh`, `scripts/check-brand.sh` | exit 0; `ok: no tauri in crates/`; `ok: no Handy branding outside the allowlist` | PASS |
| C14 | whole crate green | `cargo test -p fala-inject` | `test result: ok. 11 passed; 0 failed; 4 ignored` | PASS |
| C15 | offer on the real clipboard | `cargo test -p fala-inject offer -- --ignored --test-threads=1` at 1ee9d8c and fe3b949 | `test result: ok. 4 passed; 0 failed` (`offer_is_rendered_when_read`, `foreign_write_is_detected`, `offer_carries_exclusion_formats`, `unread_offer_is_rendered_on_drop`) | PASS |

## Manual proof (C9, C10, C11)

Each target was focused by `WScript.Shell.AppActivate` from a PowerShell job while the example counted down. The screenshots were checked by eye.

| Target | Chord | Pastes | `result=ok` | `restored=y` | Text visible | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| Notepad (Windows 11) | Ctrl+V | 3 | 3 | 3 | 3 | 225-283 ms per paste, including the 60 + 100 + 60 ms waits |
| Windows Terminal (powershell) | Ctrl+V | 3 | 3 | 3 | 3 | clean re-run; see Interference for the first run |
| VS Code 1.140 | Ctrl+V | 3 | 3 | 3 | 3 | a first attempt lost focus to the Terminal (`activate=False`) and pasted 3x there; repeated with the editor focused |
| Windows Terminal | Shift+Insert | 1 | 1 | 1 | 1 | |
| window with no field (`--blank`) | Ctrl+V | 1 + 1 | `not_read` 2/2 | 2/2 | n/a | 1735 ms and 1730 ms |
| VS Code, final code after the refactor | Ctrl+V | 1 | 1 | 1 | 1 | |

Total in real apps: 14 pastes read, 14 restored, 0 `not_read`. With no field: 2 `not_read`, 2 restored.

Win+V: clipboard history is off on this machine (Win+V shows "Turn on"). It was turned on through `HKCU\Software\Microsoft\Clipboard\EnableClipboardHistory = 1` for one paste and one control copy, read through WinRT, and then turned off again (value set to 0, then removed, which was the original state; `IsHistoryEnabled()` back to `False`). The Win+V screenshot from that moment was blocked by a Fala window that took focus, so the evidence is the WinRT listing, not a picture.

## Interference

The machine was in use during the first Terminal run: a dictation from the running Fala app ("I know it's") landed in the test terminal between rounds 2 and 3, and round 2 reported `restored=n`. That is the expected behavior when someone else writes to the clipboard: the injector skipped the restore instead of overwriting the foreign copy. From then on every run waited for 15-20 s without user input (`GetLastInputInfo`) before taking focus. The run was repeated cleanly (3/3).

## Not tested

- The `cfg(not(windows))` test `non_windows_platform_injector_is_unsupported` does not compile on Windows; the Linux CI job runs it.
- `PasteChord::CtrlShiftV` in a real app.
- A clipboard monitor that ignores `ExcludeClipboardContentFromMonitorProcessing` and reads before the target, which would mask a failed paste (door 2). No such monitor runs here.
- An app that reads the clipboard and does not paste (door 2): counts as `Ok` by design.
- Elevated targets (UIPI blocks `SendInput`); unchanged from #57.
- The desktop still pastes through `apps/desktop/src/clipboard.rs`; nothing here reaches the app yet.
