# hotkey-inject-traits verification

**Verdict**: FAIL
**Profile**: light
**Diff range**: origin/main...76292b111af8deeff926b4c3eedc6b1920f5e65d
**Round**: 2 - scoped
**Verifier**: independent sub-agent (author != verifier)

FAIL by design only: C8 and C18 are manual `TODO(windows)` runs on Windows 11 hardware and stay
UNPROVEN (plan Assumptions, `checks.md` header). Every other check, 21 of 23, is PASS at
`76292b1`.

State at this round: `git fetch origin` done. `origin/main` = `616af4f` = `git merge-base
origin/main HEAD`, so main has not moved since the rebase. Feature commits: `8ce00fc` (hotkey),
`76292b1` (inject). Round 1 ran at `134e0c6` with verdict FAIL.

Scope, per verify.md "Re-verifying after a fix": every proof re-ran in full at `HEAD` = `76292b1`
(one batched `cargo test` per crate, plus the grep, `cargo tree`, diff and cross-clippy proofs).
The CI-dependent halves (C6, C7, C9, C20, C21) come from PR #57's CI run `37979815265`. That
run's `headSha` is `76292b111af8deeff926b4c3eedc6b1920f5e65d`, equal to local `HEAD`. Citations
were refreshed for the two files the fixes touched: `crates/hotkey/src/windows.rs` and
`crates/inject/src/paste/windows.rs`. `git diff 134e0c6 HEAD --stat -- crates/` lists only those
two files. Every other citation was re-read at `HEAD` and still matches its line, so it is
marked carried.

## Checks

verified at 76292b1 (proofs re-run). Each row's evidence says whether its citation was refreshed
or carried from 134e0c6.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | non-Windows `platform_hotkey` is `Unsupported(non-empty)`, no event sent | `cargo test -p fala-hotkey -- <5 names> --exact` exit 0: `tests::non_windows_platform_hotkey_is_unsupported ... ok` | carried from 134e0c6, line re-read at HEAD: `crates/hotkey/src/lib.rs:105` - `Err(super::HotkeyError::Unsupported(reason)) => assert!(!reason.trim().is_empty())`; `crates/hotkey/src/lib.rs:109` - `assert!(rx.try_recv().is_err(), ..)` | PASS |
| C2 | id 7 Pressed/Released go to `transcribe` / `ctrl+shift+space`, in order | same invocation: `bindings::tests::events_route_to_binding_in_order ... ok` | carried from 134e0c6: `crates/hotkey/src/bindings.rs:86` - `assert_eq!(events, vec![event(KeyState::Pressed), event(KeyState::Released)])` | PASS |
| C3 | unknown id 9 and id 7 after `unregister` both give `None` | same invocation: `bindings::tests::unknown_or_unregistered_id_sends_nothing ... ok` | carried from 134e0c6: `crates/hotkey/src/bindings.rs:95` - `assert_eq!(bindings.event(9, KeyState::Pressed), None)`; `crates/hotkey/src/bindings.rs:102` - `assert_eq!(bindings.event(7, KeyState::Pressed), None)` | PASS |
| C4 | a duplicate `transcribe` is `AlreadyRegistered`, the backend is not called, the first accelerator stays | same invocation: `bindings::tests::duplicate_binding_is_rejected_and_keeps_first ... ok` | carried from 134e0c6: `crates/hotkey/src/bindings.rs:114` - `assert_eq!(result, Err(HotkeyError::AlreadyRegistered("transcribe".to_owned())))`; `:118` - `assert!(!backend_called, ..)`; `:122` - `assert_eq!(event.accelerator, "ctrl+shift+space")` | PASS |
| C5 | `unregister("nunca")` is `Ok(())` without a backend call | same invocation: `bindings::tests::unregister_unknown_binding_is_ok ... ok` | carried from 134e0c6: `crates/hotkey/src/bindings.rs:134` - `assert_eq!(result, Ok(()))`; `:135` - `assert!(!backend_called, ..)` | PASS |
| C6 | on Windows, `""`, `ctrl+banana` and `ctrl+a+b` are rejected as `InvalidAccelerator`; the 3 desktop accelerators parse | `gh run view 37979815265 --log --job 113987157577` (windows job, headSha `76292b1`): `test windows::tests::invalid_accelerators_are_rejected ... ok` and `test windows::tests::desktop_accelerators_parse ... ok`, in the `fala_hotkey` block, `test result: ok. 8 passed; 0 failed` | refreshed: `crates/hotkey/src/windows.rs:217` - `}) => assert_eq!(got, accelerator)` in the `Err(HotkeyError::InvalidAccelerator { accelerator: got, .. })` arm, other arms `panic!` (`:218`); `crates/hotkey/src/windows.rs:227` - `parse_accelerator(accelerator).is_ok()` inside `assert!` (`:226`). The command receiver is now dropped (`:208`), so an accepted invalid accelerator gets `Backend` and fails the test instead of hanging it | PASS |
| C7 | `HotkeyState::Pressed/Released` map to `KeyState::Pressed/Released` | same windows log: `test windows::tests::hook_states_map_to_key_state ... ok` | refreshed: `crates/hotkey/src/windows.rs:235` - `assert_eq!(key_state(HotkeyState::Pressed), KeyState::Pressed)`; `:236` - `assert_eq!(key_state(HotkeyState::Released), KeyState::Released)` | PASS |
| C8 | `hotkey_listen -- f9 60` with 60 physical F9 taps prints `pressed=60 released=60`, exit 0 | not run: `TODO(windows)`, manual on Windows 11 hardware (plan Assumptions) | carried from 134e0c6: example exists, `crates/hotkey/examples/hotkey_listen.rs:57` - `if pressed == expected && released == expected` | UNPROVEN |
| C9 | `register` after the manager thread is gone is `Backend(_)` | same windows log: `test windows::tests::register_after_thread_gone_is_backend_error ... ok` | refreshed: `crates/hotkey/src/windows.rs:247-250` - `assert!(matches!(hotkey.register("transcribe", "f9"), Err(HotkeyError::Backend(_))))`, receiver dropped at `:242` | PASS |
| C10 | non-Windows `platform_injector(default)` is `Unsupported(non-empty)` | `cargo test -p fala-inject -- <8 names> --exact` exit 0: `paste::tests::non_windows_platform_injector_is_unsupported ... ok` | carried from 134e0c6: `crates/inject/src/paste.rs:290` - `Err(InjectError::Unsupported(reason)) => assert!(!reason.trim().is_empty())` | PASS |
| C11 | `PasteConfig::default()` is CtrlV / 60 / 60 / 100 ms | same invocation: `paste::tests::default_config_matches_desktop ... ok` | carried from 134e0c6: `crates/inject/src/paste.rs:298` - `assert_eq!(PasteConfig::default(), PasteConfig { chord: PasteChord::CtrlV, delay_before: Duration::from_millis(60), delay_after: Duration::from_millis(60), modifier_hold: Duration::from_millis(100) })` | PASS |
| C12 | text saved: exact order ReadText, WriteText, Sleep, Chord, Sleep, WriteText("antes"); Ok; for CtrlV and CtrlShiftV | same invocation: `paste::tests::text_is_restored_after_paste_in_order ... ok` | carried from 134e0c6: `crates/inject/src/paste.rs:318` - `assert_eq!(run(&mut clipboard, false, &config, "ditado"), Ok(()))`; `:321` - `assert_eq!(*log.borrow(), expected, "{chord:?}")` | PASS |
| C13 | no text or empty text, with a 2x1 image: the image is read before the write and restored by `WriteImage`, no `Clear` | same invocation: `paste::tests::image_is_restored_when_there_is_no_text ... ok` | carried from 134e0c6: `crates/inject/src/paste.rs:334` - `assert_eq!(*log.borrow(), expected, "texto salvo {text:?}")` with `expected` ending in `WriteImage(image())` | PASS |
| C14 | nothing saved: the chord is sent and the sequence ends with `Clear` | same invocation: `paste::tests::empty_clipboard_is_cleared ... ok` | carried from 134e0c6: `crates/inject/src/paste.rs:348` - `assert_eq!(*log.borrow(), expected, ..)` with `expected` ending in `Op::Clear` | PASS |
| C15 | a failed chord still restores text / image / clear and returns `Keystroke` | same invocation: `paste::tests::failed_chord_still_restores ... ok` | carried from 134e0c6: `crates/inject/src/paste.rs:364` - `matches!(result, Err(InjectError::Keystroke(_)))`; `:367` - `assert_eq!(log.borrow().last(), Some(&restore))` | PASS |
| C16 | a failed write of the text returns `Clipboard` and sends no chord | same invocation: `paste::tests::failed_write_skips_chord ... ok` | carried from 134e0c6: `crates/inject/src/paste.rs:378` - `matches!(result, Err(InjectError::Clipboard(_)))`; `:382` - `!log.borrow().iter().any(\|op\| matches!(op, Op::Chord(..)))` | PASS |
| C17 | no log record carries either marker (Trace logger), and the Windows paste adapter makes no log call | same invocation: `paste::tests::paste_never_logs_text_or_clipboard_content ... ok`; `test -f crates/inject/src/paste/windows.rs && ! grep -nE '(log::\|\b(trace\|debug\|info\|warn\|error)!)' ...` exit 0 | carried from 134e0c6: `crates/inject/src/paste.rs:433` - `!line.contains(DICTATED) && !line.contains(SAVED)`, probe at `:422`. Refreshed: grep over `crates/inject/src/paste/windows.rs:1-122` (now 122 lines) found no match | PASS |
| C18 | `paste -- "teste um dois três"` with Notepad: the text appears twice, exit 0 only if the marker text and the 8x8 image come back | not run: `TODO(windows)`, manual on Windows 11 hardware (plan Assumptions) | carried from 134e0c6: example exists, `crates/inject/examples/paste.rs:76` - `if text_ok && image_ok {` | UNPROVEN |
| C19 | the Linux tree has none of `handy-keys`/`arboard`/`enigo`/`windows*`/`tauri`; the msvc tree has `handy-keys v0.3.4`, `arboard v3.6.1`, `enigo v0.6.1`; the no-tauri script passes | proof command from `checks.md`, exactly as written, exit 0 (`ok: no tauri in crates/`) | carried from 134e0c6: `crates/hotkey/Cargo.toml:15-16` - `[target.'cfg(windows)'.dependencies]` / `handy-keys = "0.3.4"`; `crates/inject/Cargo.toml:15`, `:17-18` - `arboard = "3.6.1"`, `enigo = "0.6.1"`; the Linux tree lists only `fala-core`, `log`, `thiserror` (+ serde) | PASS |
| C20 | msvc cross-clippy of both crates `--all-targets -D warnings` exit 0; PR `windows` job `success` | proof 1: `RUSTC_BOOTSTRAP=1 cargo clippy -Zbuild-std=std,panic_abort --target x86_64-pc-windows-msvc -p fala-hotkey -p fala-inject --all-targets -- -D warnings` exit 0 (`Checking fala-inject`, `Checking fala-hotkey`, `Finished`); proof 2: `gh pr checks 57` - `windows pass 5m29s` (job 113987157577, run headSha `76292b1`) | refreshed: `crates/hotkey/src/windows.rs:1` and `crates/inject/src/paste/windows.rs:1` are the `cfg(windows)` modules both proofs compile; the windows job also ran `cargo clippy --workspace --all-targets -- -D warnings` to `Finished` and `cargo test` with the 4 `windows::tests` `... ok` | PASS |
| C21 | Linux `cargo clippy --workspace --all-targets -D warnings` exit 0; PR `rust` job `success` | proof 2: `gh pr checks 57` - `rust pass 3m33s` (job 113987157920). Proof 1 was not run locally, because `--workspace` is forbidden on this machine (RAM). It is settled from the same job: on `Image: ubuntu-24.04` at headSha `76292b1`, step `Run cargo clippy --workspace --all-targets -- -D warnings` reached `Finished`, and the job passed. That step also ran `cargo test --workspace`, with all 9 crate tests `... ok` | the rust job log: `tests::non_windows_platform_hotkey_is_unsupported ... ok` and the 8 `paste::tests::* ... ok` lines, for the code at `crates/hotkey/src/lib.rs` and `crates/inject/src/paste.rs:290-433` | PASS |
| C22 | the diff from the merge-base does not touch `apps/desktop/` or `src/`, and adds no `cfg(windows\|target_os)` outside the two crates | proof command from `checks.md` (three-dot `origin/main...HEAD`), exactly as written, exit 0. The merge-base equals `origin/main` = `616af4f` | `git diff --stat origin/main...HEAD`: 14 files, all under `.specs/features/hotkey-inject-traits/`, `ARCHITECTURE.md`, `Cargo.lock`, `crates/hotkey/`, `crates/inject/`. None under `apps/desktop` or `src` | PASS |
| C23 | the hotkey doc cites `handy-keys` and not `rdev`; the inject doc has the failed-chord sentence; the ARCHITECTURE "Estado" paragraph and Code Map rows are updated | proof command from `checks.md`, exactly as written, exit 0 | carried from 134e0c6, lines re-read at HEAD: `crates/hotkey/src/lib.rs:6` - `Windows: hook WH_KEYBOARD_LL do crate handy-keys`; `crates/inject/src/lib.rs:10` - `Se o acorde falha, o clipboard volta ao conteúdo de antes`; `ARCHITECTURE.md:19` and `ARCHITECTURE.md:24` - Code Map rows citing `GlobalHotkey` and `Injector`; `ARCHITECTURE.md:35` - Estado paragraph, which after the rebase keeps main's text (`postproc`, Gemini) and adds the feature's sentence | PASS |

Counts: 21 PASS (C1-C7, C9-C17, C19-C23), 2 UNPROVEN (C8, C18), 0 FAIL.

## Round-1 findings, re-checked

verified at 76292b1

| # | Round-1 finding | Status at 76292b1 | Evidence |
| --- | --- | --- | --- |
| 1 | C22 measured as a two-dot diff while main moved | resolved | `checks.md:89-90` now uses `origin/main...HEAD`; the claim is unchanged; the proof exits 0; the branch is rebased on `616af4f` = `origin/main` |
| 2 | header said "C8 e C17" | resolved | `checks.md:6` - "C8 e C18 são `TODO(windows)`" |
| 3 | unregister dropped the `HotkeyId` before the hook accepted | resolved | `crates/hotkey/src/windows.rs:183-187` - `ids.get(&raw)`, then `manager.unregister(id)...?`, and only then `ids.remove(&raw)`; `Bindings` also removes only after the backend returns Ok (`crates/hotkey/src/bindings.rs:44-46`), so a failed unregister can be retried |
| 4 | a modifier stayed pressed when the chord failed midway | resolved | `crates/inject/src/paste/windows.rs:97-119` - counts the modifiers pressed and releases `modifiers[..pressed]` in reverse even after an error; the first error wins |
| 5 | the C6 test could hang | resolved | `crates/hotkey/src/windows.rs:207-208` - `drop(command_rx)`, so `request` fails with `Backend` at `send` |
| 6 | `thread::spawn` could panic | resolved | `crates/hotkey/src/windows.rs:46-49` - `thread::Builder::new().name("fala-hotkey")...spawn(..).map_err(\|err\| HotkeyError::Backend(..))?` |

## Swept existing

carried from 134e0c6. The citations were refreshed because `windows.rs` moved.

| Row | Cited constraint | Found in code |
| --- | --- | --- |
| concurrency | one thread owns the `HotkeyManager`; commands arrive one at a time on a channel, as in `fala_keys.rs` | yes - `crates/hotkey/src/windows.rs:46-49` starts `manager_thread`; `crates/hotkey/src/windows.rs:134` builds `HotkeyManager::new_with_blocking()` inside that thread; `crates/hotkey/src/windows.rs:161` reads commands with `commands.recv_timeout(POLL)` (10 ms) |

The other `Swept` rows resolve to check IDs or to `n/a` (authorization, data lifecycle), which are approved policy.

## Coverage

carried from 134e0c6: not recomputed, because the profile is `light`. The `Coverage` table in
`checks.md` was read, not joined. Its row "adaptadores Windows em hardware real" (C8, C18) is
the one this report leaves UNPROVEN.

## Faults injected

None: profile `light` does not run fault injection.

## Gate

verified at 76292b1

- `cargo test -p fala-hotkey -- <5 names> --exact`: 5 passed, 0 failed, 0 filtered out
- `cargo test -p fala-inject -- <8 names> --exact`: 8 passed, 0 failed, 5 filtered out
- C17 grep: exit 0. C19 `cargo tree` and no-tauri script: exit 0. C22 (three-dot): exit 0. C23 grep: exit 0
- C20 cross-clippy (msvc, both crates, `--all-targets -D warnings`): exit 0
- CI run `37979815265`, headSha `76292b1` = local HEAD. `gh pr checks 57`: `windows` pass, `rust` pass, `deny` pass, `frontend` pass, `secrets` pass. `pr` failed on the step `Title is at most 72 characters`. The run's env shows the original 80-char title (`TITLE: feat(hotkey): register push-to-talk and paste with restore outside the app shell`). The PR title is now 66 chars. That job does not belong to any check of this feature
- windows job log: `windows::tests::invalid_accelerators_are_rejected`, `desktop_accelerators_parse`, `hook_states_map_to_key_state` and `register_after_thread_gone_is_backend_error`, each `... ok`
- `scripts/check-brand.sh`: exit 0 (supporting, not a check)

## Findings

The fixes introduced no new defect. `git diff 134e0c6 HEAD -- crates/` touches only
`crates/hotkey/src/windows.rs` (+11/-3) and `crates/inject/src/paste/windows.rs` (+25/-19), with
no rebase noise inside `crates/`. Reviewed:

- `crates/hotkey/src/windows.rs:46-49`: when the spawn fails, the closure (and `ready_tx`) is dropped and an error comes back. The handle is not leaked.
- `crates/hotkey/src/windows.rs:182-190`: no binding or id can be lost. If `ids` lacks `raw`, the backend is skipped and the binding is removed. That happens only for a binding whose registration never reached `ids`, and `register_with` rules that out (`:169-173` inserts before returning the id).
- `crates/inject/src/paste/windows.rs:114`: `modifiers[..pressed]` cannot go out of bounds (`pressed <= modifiers.len()`).
- The first error is kept over any release error (`:116-118`).
- `Key` is `Copy` (the msvc cross-clippy compiled `for &modifier`).
- No `unwrap`/`expect` outside `#[cfg(test)]` (`rg` shows only `crates/hotkey/src/bindings.rs:70,101,121` and `crates/inject/src/paste.rs:408,420`, all in test modules).
- `ARCHITECTURE.md:35` resolves the rebase conflict by keeping main's #53 text.

Observations, not defects of this fix and not blocking:

1. `crates/inject/src/paste/windows.rs:107-110`: if the `V` click fails, the code still sleeps `modifier_hold` (100 ms) before releasing. This is harmless.
2. `crates/inject/src/paste/windows.rs:108`: `Direction::Click` on `V` is press + release inside `enigo`. If the release half fails, `V` could stay down. This is inherited from the desktop (`apps/desktop/src/input.rs:193`), where the same call has the same exposure. Minor.

Remaining gap, by design: C8 and C18 need a manual run on Windows 11 hardware
(`cargo run -p fala-hotkey --example hotkey_listen -- f9 60`; `cargo run -p fala-inject --example
paste -- "teste um dois três"` with Notepad in focus). Until those runs are recorded, the verdict
stays FAIL.
