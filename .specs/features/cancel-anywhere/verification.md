# cancel-anywhere verification

**Verdict**: FAIL
**Profile**: light
**Diff range**: 907d447..481633b
**Round**: 1 - full
**Verifier**: independent sub-agent (author != verifier). The author was the desktop-phase1b executor sub-agent; this report was written by a separate Verifier sub-agent with no build context

The only failing row is C9: its Linux part is green, and its physical Windows part (`TODO(windows)`) has not been run, so it is Unproven. C1-C8 are proven with located evidence.

## Where the proofs ran

The worktree `/home/augusto/projects/fala-wt/desktop-phase1` is on `feat/language-picker` at `fa615bf`, and that commit contains `481633b`. The language-picker commits do not touch any file in this feature's diff: `git diff --quiet 481633b HEAD -- apps/desktop/src/{actions.rs,transcription_coordinator.rs,shortcut/handler.rs,utils.rs}` exits 0. So the tests compiled at `HEAD` run the same code these four files have at `481633b`. The grep and awk proofs read the same files. `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`, one cargo command at a time.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | `is_busy()` matches the stage across 7 transitions | `cargo test -p fala --lib -- transcription_coordinator actions::tests shortcut::handler::tests` exit 0; `transcription_coordinator::tests::is_busy_follows_the_stage ... ok` | `apps/desktop/src/transcription_coordinator.rs:2119` `assert!(!state.is_busy(), "a new coordinator is idle")`; `:2124` `assert!(state.is_busy(), "recording is busy")`; `:2130-2131` `assert_eq!(state.stage, Stage::Processing)` + `assert!(state.is_busy(), ...)`; `:2133` drained idle; `:2143` discard idle; `:2152` `on_cancel(true)` idle; `:2161` `on_start_result(BINDING, false)` idle | PASS |
| C2 | cancel during Processing keeps it busy until the drain, and the drain starts nothing despite a remembered press | same invocation; `cancel_during_processing_stays_busy_until_drained ... ok` | `apps/desktop/src/transcription_coordinator.rs:2175` `assert!(state.is_busy(), "the pipeline is still running; ...")`; `:2180` `assert!(state.on_processing_finished().is_none(), ...)`; `:2184` `assert!(!state.is_busy())`; precondition (press remembered before the cancel) `:2170` | PASS |
| C3 | a drain with a remembered press goes Processing -> Recording and stays busy | same invocation; `drain_with_remembered_press_stays_busy ... ok` | `apps/desktop/src/transcription_coordinator.rs:2193` `assert!(state.is_busy())`; `:2196` `assert!(matches!(effect, Some(Effect::Start { .. })))`; `:2197` `assert_eq!(state.stage, Stage::Recording(BINDING.to_string()))`; `:2198` `assert!(state.is_busy(), ...)` | PASS |
| C4 | `cancel_key_change` acts only when busy flips; only the coordinator calls register/unregister | same invocation; `cancel_key_changes_only_when_busy_flips ... ok`; grep proof exit 0 (the only call sites outside `shortcut/` are `transcription_coordinator.rs:683-684`, inside `sync_cancel_key`) | `apps/desktop/src/transcription_coordinator.rs:2213` `assert_eq!(cancel_key_change(armed, busy), expected, ...)` over all 4 pairs `(false,false,None) (false,true,Some(true)) (true,false,Some(false)) (true,true,None)`; call sites `apps/desktop/src/transcription_coordinator.rs:683` / `:684` | PASS |
| C5 | the 1.F1 and inherited coordinator tests stay green with no assertion edited | same invocation: 53 `transcription_coordinator::tests::*` ok (49 at `907d447` + 4 new); `git diff 907d447..481633b -- apps/desktop/src/transcription_coordinator.rs` removes one line only, `Self { tx }`, and no test line; `#[test]` count 49 -> 53, `assert` count 229 -> 256 (27 added, 0 removed) | `apps/desktop/src/transcription_coordinator.rs:2104` is the old end of the test module; everything after it is added, and nothing before it changed in the test region | PASS |
| C6 | `cancel_key_fires` is true only for (press, busy), and the cancel handler checks `is_busy()` | same invocation; `shortcut::handler::tests::cancel_key_fires_only_on_press_while_busy ... ok`; awk proof exit 0 | `apps/desktop/src/shortcut/handler.rs:98` `assert_eq!(cancel_key_fires(is_pressed, busy), expected, ...)` over the 4 pairs; handler `apps/desktop/src/shortcut/handler.rs:62-64` `app.try_state::<TranscriptionCoordinator>().is_some_and(\|c\| c.is_busy())` | PASS |
| C7 | `deliver_unless_cancelled`: if cancelled, neither paste nor save; if not, paste then save, once each | same invocation; `actions::tests::delivery_is_all_or_nothing_after_the_last_check ... ok` | `apps/desktop/src/actions.rs:1076-1077` `assert!(!delivered)` + `assert!(calls.borrow().is_empty(), ...)`; `apps/desktop/src/actions.rs:1086-1087` `assert!(delivered)` + `assert_eq!(*calls.borrow(), vec!["paste", "save"])`. One ordered log covers both the order and the call counts | PASS |
| C8 | in `stop`, history is saved only through `deliver_unless_cancelled`, in both branches | grep `-c 'deliver_unless_cancelled('` = 4 (`-ge 3`) exit 0; `! grep -q 'save_history()'` exit 0 | production call sites `apps/desktop/src/actions.rs:830` (empty-text branch: `\|\| {}`, `save_history`) and `apps/desktop/src/actions.rs:843` (main thread: `utils::paste`, `save_history`), both inside `TranscribeAction::stop` (`:649`); `save_history` is used only at `:815` (definition), `:833` and `:855` (passed as an argument) | PASS |
| C9 | Windows: Esc while transcribing or in the LLM hides the pill, pastes nothing, writes no history; Esc after the paste reaches Notepad | Linux part: same invocation; `actions::tests::pending_operation_stops_after_cancellation ... ok`. Physical part: `TODO(windows)` manual run (10 + 5 + 5 dictations) **not executed** | Linux part: `apps/desktop/src/actions.rs:1028` `assert_eq!(result, None)` (`complete_unless_cancelled` gives up once cancelled; inherited test, untouched by the diff). Physical part: no evidence | UNPROVEN - Windows manual part not run |

## Scrutiny requested by the orchestrator

- **No inherited assertion was weakened or removed.** In `transcription_coordinator.rs`, the only removed line in the whole diff is `Self { tx }`. In `actions.rs`, the test module changes only the `use super::{...}` import list and adds one test. No removed line contains `assert`.
- **The history and paste ordering is proven by a test, not only stated in prose.** C7 records the calls into an ordered `RefCell<Vec>` log. It asserts the empty log when cancelled and `["paste", "save"]` when not, which rules out reversed, duplicated and partial delivery. C8 ties `stop` to that function. Reading the code confirms there is no other history write: the old unconditional `hm.save_entry` before the main-thread hop became the `save_history` closure (`actions.rs:815`), and that closure is only ever passed to `deliver_unless_cancelled`.
- **AGENTS.md rules for `apps/desktop`:**
  - No gratuitous reformatting. The re-indented history and paste blocks moved into closures because they had to; `rustfmt --edition 2021 --check` on the 4 files and `cargo fmt -p fala -- --check` both exit 0.
  - No new visible string. The only literals added are a moved `debug!` message and test failure messages, and nothing under `src/` changed in this diff.
  - No `unwrap`, `expect` or `println!` added. The only `cfg` added is `#[cfg(test)]`.
  - `cargo clippy -p fala --all-targets -- -D warnings` exit 0.
  - `scripts/check-brand.sh` prints `ok: no Handy branding outside the allowlist`, exit 0.
  - `scripts/check-no-tauri-in-crates.sh` exit 0.

## Findings

1. **C9 is Unproven**, and that is why the verdict is FAIL. Still to do: on the Windows build with Notepad in focus, cancel 10 dictations with Esc while transcribing, cancel 5 during the LLM call, and press Esc 5 times after the paste. Expected: 0 pastes and 0 new history entries for the first two groups, 5 pastes with 5 entries for the third, and the post-paste Esc reaching Notepad 5 of 5 times.
2. **The C8 proof cannot catch a missing call site. This is a gap in the check, not in the code.** `grep -c 'deliver_unless_cancelled('` also counts the 2 calls in the test (`actions.rs:1071`, `:1081`). The total is 4, so deleting one of the two production calls leaves 3 and the `-ge 3` proof still passes. I proved the claim directly by locating both call sites (`:830`, `:843`). A stronger proof would count only above `mod tests` (`:978`), or require `-ge 4`.
3. **Observation, outside the checks:** `FinishGuard` (`actions.rs:689`) drops when the async task ends, which is right after `run_on_main_thread` queues the paste closure. So `ProcessingFinished` can reach the coordinator, and Esc can be disarmed, a moment before the last check runs on the main thread. During that gap Esc does nothing, which is a missed cancel and never an orphan history entry, so AC 6 holds. Flow step 5 of the plan is consistent with it ("o Esc já não está armado quando o coordenador recebe `ProcessingFinished`"). The physical Windows run in C9 is where this would show up, if it shows up at all.
4. **Observation:** `hm.save_entry`, a SQLite insert, now runs on the main thread after the paste. The plan's Impact table says so. One more path changes: if `run_on_main_thread` itself fails (`actions.rs:863`), the history entry is no longer saved, where before it was. This failure path is rare, and the new behaviour still follows AC 6.
5. **Light profile:** no Coverage recompute, Test policy or fault injection, as the profile allows. `sync_cancel_key` itself, which needs an `AppHandle`, is covered only through its pure decision `cancel_key_change` (C4) and the grep for its call sites. No test exercises it.

## Gate

`cargo test -p fala --lib -- transcription_coordinator actions::tests shortcut::handler::tests`: 62 passed, 0 failed (238 filtered out), and all 7 named tests appear individually as `ok`. `cargo test -p fala --lib`: 300 passed, 0 failed. `cargo clippy -p fala --all-targets -- -D warnings`: exit 0.
