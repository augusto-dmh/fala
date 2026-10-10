# upstream-catchup PR 3 shortcuts verification

**Verdict**: PASS - C1 to C7, C9 and C10 proven at af7a6a5 with located evidence; C8 (manual, Windows) not run and listed separately below
**Profile**: light
**Diff range**: eb0482c..af7a6a5
**Round**: 2 - scoped
**Verifier**: independent sub-agent (author != verifier)

Checks file: `.specs/features/upstream-catchup/checks-pr3-shortcuts.md` (no plan.md; `## Intent` holds the delegated decisions). Every proof ran in a detached worktree at `6c9a9eb`. Cargo compiled `fala` from that worktree's sources (log line `Compiling fala v0.1.0 (.../wt-pr3/apps/desktop)`), so no stale artifact from another worktree was reused.

## Binding sources

None. The checks name no binding source and there is no plan.md. Step 1 runs only under `ui` anyway. The upstream commits `b38987a` (#2190) and `f5c27e6` (#2158) were compared as reference below. They are not design sources.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | requests true then false, both passes run after (stop's first): ends unregistered, zero backend calls | `cargo test -p fala --lib -- shortcut:: transcription_coordinator::tests::cancel_key_changes_only_when_busy_flips` exit 0, line `shortcut::tests::cancel_key_ends_unregistered_when_passes_run_out_of_order ... ok` | `apps/desktop/src/shortcut/mod.rs:1585` - `assert!(!*backend.registered.borrow())`; `apps/desktop/src/shortcut/mod.rs:1586` - `assert!(backend.calls.borrow().is_empty())` | PASS |
| C2 | each pass applies the latest request once; a repeat pass makes no call; calls = [true, false] | same invocation, line `shortcut::tests::cancel_key_follows_the_latest_request ... ok` | `apps/desktop/src/shortcut/mod.rs:1601` - `assert_eq!(*backend.calls.borrow(), vec![true, false])` (the repeat pass sits between 1595 and 1600) | PASS |
| C3 | a failed register leaves state unregistered; the next pass registers; calls on the working backend = [true] | same invocation, line `shortcut::tests::cancel_key_retries_after_a_failed_registration ... ok` | `apps/desktop/src/shortcut/mod.rs:1611` - `assert!(*backend.registered.borrow())`; `apps/desktop/src/shortcut/mod.rs:1612` - `assert_eq!(*backend.calls.borrow(), vec![true])` | PASS |
| C4 | no per-backend cancel functions remain; the only spawn in shortcut/mod.rs is in `schedule_cancel_reconcile` | the C4 shell proof from the checks file, exit 0 | `apps/desktop/src/shortcut/mod.rs:128` - `tauri::async_runtime::spawn(async move {` inside `fn schedule_cancel_reconcile` (mod.rs:126); `rg` finds the cancel fns only at mod.rs:107 and mod.rs:117; the only external callers are `apps/desktop/src/transcription_coordinator.rs:745` and `:746` | PASS |
| C5 | `validate_shortcut` rejects `option_left+space` and `ctrl_right+space`, accepts the 6 defaults | same invocation, lines `shortcut::tauri_impl::tests::rejects_side_specific_modifiers_the_parser_cannot_register ... ok` and `shortcut::tauri_impl::tests::accepts_the_default_shortcuts ... ok` | `apps/desktop/src/shortcut/tauri_impl.rs:178` - `assert!(validate_shortcut(raw).is_err(), ...)` over both rejected chords; `apps/desktop/src/shortcut/tauri_impl.rs:192` - `assert_eq!(validate_shortcut(raw), Ok(()), ...)` over all 6 accepted ones | PASS |
| C6 | existing shortcut and coordinator tests stay green with no assertion edited | same invocation: `shortcut::tests::empty_binding_is_unset`, `shortcut::tests::compound_shortcut_keys_parse_on_both_backends`, `shortcut::handler::tests::cancel_key_fires_only_on_press_while_busy`, `transcription_coordinator::tests::cancel_key_changes_only_when_busy_flips` all `ok`; 9 passed, 0 failed | `apps/desktop/src/transcription_coordinator.rs:2283` - `assert_eq!(cancel_key_change(armed, busy), expected, ...)`; `apps/desktop/src/shortcut/mod.rs:1641` - `assert!(key.parse::<Shortcut>().is_ok(), ...)`. `git diff -U0 eb0482c..6c9a9eb` removes no line in any test module, and `transcription_coordinator.rs` is outside the diff | PASS |
| C7 | no upstream brand enters the code; the #2158 comments say `fala_keys` | `scripts/check-brand.sh && grep -q 'the fala_keys recorder saves' ...` exit 0, output `ok: no Handy branding outside the allowlist` | `apps/desktop/src/shortcut/tauri_impl.rs:71` - comment `which the fala_keys recorder saves but the accelerator` | PASS |

## Manual proofs not run

| Check | Claim | Proof | Status |
| --- | --- | --- | --- |
| C8 | Windows build: 20 short discarded recordings, then Esc in Notepad, and Esc reaches Notepad 20/20 | `TODO(windows)` manual count | not run (manual, Windows). It cannot run on this Linux host, where `apply_cancel_shortcut` returns before any backend call (`apps/desktop/src/shortcut/mod.rs:135`) |

Do C1 to C7 still support the claim without C8? Partly. They prove the reconciliation rule on the `CancelKey` type through a closure, and they prove that the old per-backend spawns are gone. No automated proof touches the real glue in `schedule_cancel_reconcile` and `apply_cancel_shortcut` (mod.rs:126-151): the binding lookup, the dispatch to `register_shortcut`/`unregister_shortcut`, and the real hook. C8 is the only check that covers that layer. Until someone runs it on Windows, the user-facing claim "Esc no longer stays captured" rests on unit-level evidence plus reading the code, which I did (see Findings 1 and 2).

## Coverage

Under `light`, the Coverage join is not recomputed. I spot-checked the author's table anyway:

| Set (size) | Checked against | Member -> proof | Note |
| --- | --- | --- | --- |
| pass orders of a short dictation (2) | `CancelKey::reconcile`, mod.rs:87 | stop-first C1, start-first C2 | holds |
| backend result (2) | `match apply(requested)`, mod.rs:94 | Ok C2, Err C3 (register only) | an unregister Err is not tested; see Finding 3 |
| backends that had their own cancel fns (2) | `git show 94094ac` | fala_keys C4, tauri_impl C4 | holds |
| #2158 shortcuts (8) | test arrays, tauri_impl.rs:177 and :184 | all 8 members C5 | holds |
| Esc platforms (2) | mod.rs:135-151 | Linux C4 (structural only), Windows C8 | Windows member is pending C8, not run |

Level gap (light): C1 to C3 test the decision layer only. The tests cover no async scheduling and no real backend. That is the design the author chose and recorded in Intent. It leaves C8 as the only proof at the hook level.

Swept rows: none resolves to "existing". All rows cite new checks (C1 to C5) or `n/a`, so there was no existing constraint to re-read. The observability row (C3, "the backend error is logged with the direction") has no assertion. The log call exists at `apps/desktop/src/shortcut/mod.rs:96` (`"Failed to {} cancel shortcut: {}"`), but C3's test does not prove it. This is a precision gap in the Swept row, not a failed check.

## Faults injected

none - profile light (fault injection runs only under standard and ui, so no mutant was tried).

## Upstream reconciliation

- **#2190 (`94094ac` vs `b38987a`).** The two are semantically the same.
  - Same three rules: the latest request wins (an `AtomicBool` stored synchronously before the spawn, mod.rs:112/120), the lock serializes passes and the request is read under it (mod.rs:88-89), and a failed call leaves the recorded state (mod.rs:94-100).
  - The difference is structural only: one `CancelKey` type with a closure instead of two loose statics.
  - Kept from the old per-backend functions: the `secure_input` register/unregister fallback calls (mod.rs:110, mod.rs:118), and Linux disabled with no backend call (mod.rs:135-139).
  - Behaviour change, and not upstream's: a missing `cancel` binding used to be a silent no-op, and upstream still returns silently. Fala now returns `Err` and logs at error level (mod.rs:143-145). See Finding 2.
  - The old fala_keys path called `state.register` directly. It now goes through `fala_keys::register_shortcut`, which does the same `try_state` + `register` (fala_keys.rs:464-469) but errors where the state is missing. That is the same as upstream.
- **Deadlock.** I found no deadlock path.
  - The pass holds the `std::sync::Mutex` while it calls the backend inside a tokio task. That call blocks: the Tauri plugin does `run_on_main_thread` + `rx.recv()` (tauri-plugin-global-shortcut-2.3.2 `src/lib.rs:83-84`), and fala_keys does `rx.recv()` on its manager thread (fala_keys.rs:258).
  - Nothing that main or the manager thread runs takes `CANCEL_KEY`. `register_cancel_shortcut` and `unregister_cancel_shortcut` only store the atomic and spawn, and the coordinator calls them from its own thread (transcription_coordinator.rs:745-746).
  - The worst case is a second pass parking one tokio worker on the mutex until the first returns. Upstream has the same shape.
- **#2158 (`89bdc56` vs `f5c27e6`).** The cherry-pick is the same as upstream except for comments.
  - The validator change and both tests match line for line.
  - The `handy-keys` to `fala_keys` rename is in the two new comment lines.
  - It also renames one pre-existing comment that upstream did not touch (tauri_impl.rs:118, "Mirrors the fala_keys event log line"). The commit message announces this hunk, it is consistent with ADR brand policy, and it changes comments only. See Finding 4.
  - The conflict (the cancel fns that 94094ac had already removed) is resolved by keeping them removed, which is correct.

## Findings

1. **The stuck state after a failed unregister is inherited from upstream and not new to Fala (note, no check violated).**
   - What happens: if an unregister fails, `registered` stays `true` (mod.rs:94-100). One way to get there: the person edits the cancel binding while a dictation runs. `change_binding` for `cancel` only rewrites settings (mod.rs:226), so the stop pass unregisters the new binding, which was never registered, and Tauri errors.
   - The effect: every later start is a no-op (`requested == registered`), every later stop retries the failing unregister, and the old binding stays captured until the app restarts.
   - Compared with before: the pre-PR code ignored unregister errors and re-registered on every start, so it recovered on the next dictation.
   - Upstream b38987a behaves exactly like Fala here. This is a candidate follow-up, not a regression against the reimplementation target.
2. **A missing cancel binding now logs at error level (minor deviation from upstream).** `apply_cancel_shortcut` returns `Err("no cancel binding in settings")` (mod.rs:143-145). Upstream returns silently, and so did the old code. Since the state never flips, this logs once per dictation start. The `cancel` binding is in the defaults (settings.rs:1049), so this only matters for hand-edited settings. Not covered by a check.
3. **Precision gap: C3 and the Swept rows cover only the failed register.** The failed unregister path, which is the one in Finding 1, has no test, and no row in Coverage's "backend result" set names it.
4. **One cherry-pick hunk is not in upstream:** the comment rename at tauri_impl.rs:118. It is harmless, comment-only, and announced ("the comments name fala_keys"). Recorded because the brief asked whether the cherry-pick is otherwise identical.
5. **Style nit:** in the test module, the new `CancelKey` tests come before the module's existing `use handy_keys::Hotkey; use tauri_plugin_global_shortcut::Shortcut;` (mod.rs:1614-1615), so those `use` lines now sit after items. It compiles. apps/desktop is inherited code and is not linted under the workspace rules.
6. **C8 not run (manual, Windows).** It is the only hook-level proof. The feature should not be called done for Windows until C8's 20/20 count is recorded.

## Gate

`CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2 cargo test -p fala --lib -- shortcut:: transcription_coordinator::tests::cancel_key_changes_only_when_busy_flips` - 9 passed, 0 failed, 345 filtered out (exit 0). C4 shell proof exit 0. C7 `scripts/check-brand.sh` + grep exit 0.

## Round 2 - scoped

Verified at `af7a6a5` (detached worktree, clean `git status --porcelain`). The fix diff is `git diff 6c9a9eb af7a6a5`: `apps/desktop/src/shortcut/mod.rs` (+99 -44) and the checks file (C9, C10, Intent, Coverage). Scope: C1, C2, C3, C4, C6, C9, C10, because every one of them touches `shortcut/mod.rs`. Per the procedure, all proofs re-ran in one invocation at the new head, so C5 also ran again and is green. C7 also ran again because it is a cheap shell check.

**Cherry-pick carried.** `5ff011a` is the same patch as round 1's `89bdc56`:
- `git range-diff eb0482c..6c9a9eb eb0482c..af7a6a5` prints `2:  89bdc56 = 2:  5ff011a`.
- `git patch-id` gives `88be9da67dee7d750dafe19078a4e89ca1a0ae82` for both.
- `diff <(git diff 89bdc56^ 89bdc56) <(git diff 5ff011a^ 5ff011a)` is empty.
- `git diff 89bdc56 5ff011a` touches only `shortcut/mod.rs`, which is the rewritten parent `ffa46bc`.

The upstream reconciliation for #2158 and the tauri_impl citations therefore carry from `6c9a9eb`.

**Build provenance.** `Compiling fala v0.1.0 (.../scratchpad/wt-pr3/apps/desktop)` appears in the log after a `touch` of `shortcut/mod.rs`, so no stale artifact from another worktree was reused. The run used `CARGO_BUILD_JOBS=2` and had already finished (exit 0) when the orchestrator changed the rule to JOBS=1. No further cargo run was needed.

### Checks (round 2)

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | requests true then false, stop's pass first: ends unregistered, zero backend calls | `cargo test -p fala --lib -- shortcut:: transcription_coordinator::tests::cancel_key_changes_only_when_busy_flips` exit 0, line `shortcut::tests::cancel_key_ends_unregistered_when_passes_run_out_of_order ... ok` (verified at af7a6a5) | `apps/desktop/src/shortcut/mod.rs:1615` - `assert!(!*backend.registered.borrow())`; `apps/desktop/src/shortcut/mod.rs:1616` - `assert!(backend.calls.borrow().is_empty())` | PASS |
| C2 | latest request applied once; repeat pass makes no call; calls = [true, false] | same invocation, line `shortcut::tests::cancel_key_follows_the_latest_request ... ok` | `apps/desktop/src/shortcut/mod.rs:1631` - `assert_eq!(*backend.calls.borrow(), vec![true, false])` (the repeat pass is at 1627) | PASS |
| C3 | failed register leaves unregistered; next pass registers; calls = [true] | same invocation, line `shortcut::tests::cancel_key_retries_after_a_failed_registration ... ok` | `apps/desktop/src/shortcut/mod.rs:1641` - `assert!(*backend.registered.borrow())`; `apps/desktop/src/shortcut/mod.rs:1642` - `assert_eq!(*backend.calls.borrow(), vec![true])` | PASS |
| C4 | no per-backend cancel fns; the only spawn in shortcut/mod.rs is in `schedule_cancel_reconcile` | C4 shell proof from the checks file, exit 0 | `apps/desktop/src/shortcut/mod.rs:134` - `tauri::async_runtime::spawn(async move {` inside `fn schedule_cancel_reconcile` (mod.rs:132); the cancel fns exist only at mod.rs:113 and mod.rs:123; external callers only `apps/desktop/src/transcription_coordinator.rs:745` and `:746` | PASS |
| C5 | `validate_shortcut` rejects 2 side-specific chords, accepts the 6 defaults (carried from 6c9a9eb, re-run here) | same invocation, lines `shortcut::tauri_impl::tests::rejects_side_specific_modifiers_the_parser_cannot_register ... ok` and `shortcut::tauri_impl::tests::accepts_the_default_shortcuts ... ok` | `apps/desktop/src/shortcut/tauri_impl.rs:178` - `assert!(validate_shortcut(raw).is_err(), ...)`; `apps/desktop/src/shortcut/tauri_impl.rs:192` - `assert_eq!(validate_shortcut(raw), Ok(()), ...)` | PASS |
| C6 | existing shortcut and coordinator tests green, no assertion edited | same invocation: `empty_binding_is_unset`, `compound_shortcut_keys_parse_on_both_backends`, `handler::tests::cancel_key_fires_only_on_press_while_busy`, `cancel_key_changes_only_when_busy_flips` all `ok`; 11 passed, 0 failed | `apps/desktop/src/transcription_coordinator.rs:2283` - `assert_eq!(cancel_key_change(armed, busy), expected, ...)`; `apps/desktop/src/shortcut/mod.rs:1696` - `assert!(key.parse::<Shortcut>().is_ok(), ...)`. `git diff -U0 eb0482c..af7a6a5 -- apps/desktop/src` removes no `assert` or `#[test]` line, and `transcription_coordinator.rs` is outside the diff | PASS |
| C7 | no upstream brand; #2158 comments say `fala_keys` (carried from 6c9a9eb, re-run here) | `scripts/check-brand.sh && grep -q 'the fala_keys recorder saves' ...` exit 0, output `ok: no Handy branding outside the allowlist` | `apps/desktop/src/shortcut/tauri_impl.rs:71` - comment `which the fala_keys recorder saves but the accelerator` | PASS |
| C9 | unregister gets the binding register returned: registered `escape`, settings now `ctrl+q`, stop unregisters `["escape"]` and the key ends free | same invocation, line `shortcut::tests::cancel_key_unregisters_the_binding_it_registered ... ok` | `apps/desktop/src/shortcut/mod.rs:1655` - `assert_eq!(*backend.unregistered.borrow(), vec!["escape".to_string()])`; `apps/desktop/src/shortcut/mod.rs:1654` - `assert!(!*backend.registered.borrow())` | PASS |
| C10 | a failed unregister leaves the state registered; the next pass unregisters; calls on the working backend = [true, false] | same invocation, line `shortcut::tests::cancel_key_retries_after_a_failed_unregistration ... ok` | `apps/desktop/src/shortcut/mod.rs:1669` - `assert_eq!(*backend.calls.borrow(), vec![true, false])`; `apps/desktop/src/shortcut/mod.rs:1668` - `assert!(!*backend.registered.borrow())` | PASS |

### Assertion strength, C1 to C3 (compared with `git show 6c9a9eb:apps/desktop/src/shortcut/mod.rs`)

All five assertions in C1 to C3 are textually identical to round 1 (6c9a9eb mod.rs:1585-1586, 1601, 1611-1612 -> af7a6a5 mod.rs:1615-1616, 1631, 1641-1642). Only the call sites changed:
- `key.reconcile(|r| backend.apply(r))` became `backend.reconcile(&key, "escape")`, a helper at mod.rs:1600-1602 that routes `register` and `unregister` to the same recording fake.
- C3's failing pass `|_| Err(..)` became `key.reconcile(|| Err(..), |_| Ok(()))`. The request is `true`, so only the register closure can run, and the unregister `Ok` is unreachable. Same strength.
- The fake still records every call in `calls`, so the zero-call and `[true, false]` assertions are as strong as before.

No assertion got weaker.

### Manual proofs not run (round 2)

| Check | Claim | Proof | Status |
| --- | --- | --- | --- |
| C8 | Windows build: 20 short discarded recordings, then Esc in Notepad, and Esc reaches Notepad 20/20 | `TODO(windows)` manual count | not run (manual, Windows). On Linux neither `register_cancel_binding` nor `unregister_cancel_binding` calls a backend (mod.rs:151, mod.rs:157-161). C8 is still the only hook-level proof, and C9 and C10 are closure-level like C1 to C3. |

### Design judgment: remembered binding

- **Does it fix round 1's finding 1?** Yes.
  - The stop pass no longer reads settings. `reconcile` hands the stored `Some(binding)` to `unregister` (mod.rs:101), and `unregister_cancel_binding` passes that same binding to the backend (mod.rs:165).
  - A cancel binding edited mid-dictation (`change_binding` still only rewrites settings, mod.rs:241) therefore unregisters the key that was actually registered. C9 proves this at the decision layer.
  - The Tauri backend unregisters by parsing `binding.current_binding` (tauri_impl.rs:147). Before, it parsed the new string and errored. Now it parses the old string.
  - This is a deliberate departure from upstream b38987a, and the Intent records it.
- **Linux path.** Neither direction calls a backend.
  - `register_cancel_binding` now reads settings on Linux too and stores `Some(binding)` (mod.rs:145-153). Tracking that state is harmless.
  - New: on Linux, a missing `cancel` binding now hits the error log at every dictation start (`Failed to register cancel shortcut: no cancel binding in settings`, mod.rs:99/150). Before, the code returned `Ok` before any lookup. See finding R2-2.
- **Missing binding.** Only the register path needs it now. The stop path cannot fail for a missing binding anymore, because it uses the stored one. That is better than round 1, where both directions errored.
- **Lock held across backend calls.** Same shape as round 1.
  - `get_settings` and the blocking backend call run under the `std::sync::Mutex` (mod.rs:94-105).
  - Nothing else takes `CANCEL_KEY`, and `register_cancel_shortcut`/`unregister_cancel_shortcut` only store the atomic and spawn (mod.rs:113-127).
  - I found no new deadlock path. The worst case is still one tokio worker parked on the mutex.
- **What the binding does not remember: the backend.** `unregister_shortcut` dispatches on the current `keyboard_implementation` (mod.rs:179-184), and `unregister_all_shortcuts` skips `cancel` on a switch (mod.rs:541). See finding R2-3.

### Coverage (round 2, light: spot-check only)

| Set (size) | Checked against | Member -> proof | Note |
| --- | --- | --- | --- |
| backend result (4) | `match (requested, registered.as_ref())`, mod.rs:96-105 | register Ok C2 · register Err C3 · unregister Ok C2, C9 · unregister Err C10 | holds; round 1 finding 3 closed |
| binding unregistered (2) | mod.rs:101, mod.rs:165 | the registered one C9 · the current settings one never C9 | holds at the closure level; the real glue (mod.rs:137-138) has no automated proof |
| reconcile match arms (3) | mod.rs:97, 101, 105 | (true, None) C2/C3 · (false, Some) C2/C9/C10 · no-op `_` C1 (both passes no-op) and C2 (repeat pass) | holds |

Swept: the observability row still names C3 for "the error is logged with the direction". The log now carries the direction in two separate literals (mod.rs:99, mod.rs:103), and no test asserts either one. The precision gap from round 1 carries.

### Faults injected (round 2)

none - profile light (fault injection runs only under standard and ui).

### Findings (round 2)

1. **Round 1 findings closed.**
   - Finding 1 (stuck after a failed unregister of an edited binding) is fixed by design and proven by C9.
   - Finding 3 (no failed-unregister test) is closed by C10.
   - Finding 5 (`use` lines after items) is fixed: the lines now sit at the top of the test module (mod.rs:1573, mod.rs:1575).
2. **R2-2, minor, changed from round 1's finding 2: a missing cancel binding is now an error log on Linux too.** `register_cancel_binding` looks up settings before the `cfg` gate (mod.rs:146-150). On Linux, a hand-edited settings file without `cancel` now logs at error level on every dictation start, where round 1's code returned silently on Linux. No check covers it. It is harmless apart from the log.
3. **R2-3, note, not new: a keyboard implementation switched mid-dictation sends the unregister to the wrong backend.**
   - The stored `ShortcutBinding` does not say which backend registered it.
   - The unregister then fails on the new backend and is retried at every later stop, so the key stays captured on the old backend until someone switches back or restarts the app.
   - Round 1's code, the pre-PR code and upstream all share this. Candidate follow-up, outside this PR's checks.
4. **Precision note on C10.** The mid-test `assert!(*backend.registered.borrow())` (mod.rs:1666) reads the fake's flag, which the failing closure never touches, so it would hold even if `CancelKey` had dropped its state. The claim "the state stays registered" is settled instead by mod.rs:1669: a second `false` call only happens if the state was still `Some`. The check is proven, but that one assertion adds nothing.
5. **A persistent unregister failure still leaves the key captured between dictations, until the next stop retries.** This is inherent to retry-on-next-pass, and the Intent decided it. Upstream and the pre-PR code do the same.
6. **C8 not run (manual, Windows).** Unchanged from round 1. Do not call the feature done for Windows until the 20/20 count is recorded.

### Gate (round 2)

`flock -w 7200 /home/augusto/projects/fala/target/.builder.lock env CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2 cargo test -p fala --lib -- shortcut:: transcription_coordinator::tests::cancel_key_changes_only_when_busy_flips` - 11 passed, 0 failed, 345 filtered out (exit 0) at af7a6a5. C4 shell proof exit 0. C7 `scripts/check-brand.sh` + grep exit 0.
