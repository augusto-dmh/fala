# shortcut-gestures verification

**Verdict**: PASS - 22 of 23 checks proven at HEAD on Linux; C17 is deferred by design (TODO(windows), manual), see `## Deferred`
**Profile**: light
**Diff range**: 881e77c..001a553
**Round**: 1 - full
**Verifier**: independent sub-agent with a fresh context (author != verifier). The author is the desktop-phase1 executor sub-agent; this report was written by a separate Verifier sub-agent that did not build any of the 4 commits.

Every proof ran at `001a553` (`feat/shortcut-gestures`, tree clean). Cargo ran with
`CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`, one command at a time.
The read-only rule held: no code, test, plan or checks file was changed. `checks.md` is unchanged
since the planning commit `6fdba31`; later commits only appended to `## Handoff`.

## Binding sources

n/a under `light`: step 1 runs only under `ui`. The plan's `Sources` (pitch and delta under
`fala-research/`) are outside the repo and are not marked binding.

## Checks

The Rust proofs ran in one invocation: `cargo test -p fala --lib -- transcription_coordinator settings::tests` (76 passed, 0 failed, exit 0). Each named test below shows up by name as `... ok` in that output.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | key-down while idle emits `Start` and sets the stage to `Recording` (AC 1) | `double_tap_press_starts_recording_on_key_down` ok | `apps/desktop/src/transcription_coordinator.rs:1781` - `Some(Effect::Start { binding_id, .. }) => assert_eq!(binding_id, BINDING)`; `:1784` - `assert_eq!(state.stage, Stage::Recording(BINDING.to_string()))` | PASS |
| C2 | a 300 ms hold emits nothing on release, then `Stop` when the grace runs out; stage `Processing` (AC 2) | `double_tap_long_hold_stops_after_release_grace` ok | `transcription_coordinator.rs:1796` - `state.on_input(dt(false), t0 + ms(300)).is_none()`; `:1800-1803` - `matches!(state.on_deadline(t0 + ms(300) + RELEASE_GRACE), Some(Effect::Stop { .. }))`; `:1807` stage `Processing` | PASS |
| C3 | a 299 ms tap emits nothing even after the grace, stays `Recording`, not locked, `next_deadline() == t0+500ms` (AC 3) | `double_tap_short_tap_keeps_recording_and_arms_window` ok | `transcription_coordinator.rs:1815` - `dt_tap(&mut state, t0, ms(299)).is_none()`; `:1820` - `!state.is_locked()`; `:1821` - `assert_eq!(state.next_deadline(), Some(t0 + ms(500)))` | PASS |
| C4 | second key-down at 499 and at 500 ms locks, emits nothing, clears the window (AC 4) | `double_tap_second_press_inside_window_locks` ok | `transcription_coordinator.rs:1826` - `for second_ms in [499, 500]`; `:1831` - `state.on_input(dt(true), t0 + ms(second_ms)).is_none()`; `:1835` - `assert!(state.is_locked())`; `:1836` - `assert_eq!(state.next_deadline(), None, ...)` | PASS |
| C5 | locked session: key-up emits nothing, `grace_deadline()` stays `None` (AC 5) | `double_tap_locked_session_ignores_release` ok | `transcription_coordinator.rs:1851` - `state.on_input(dt(false), t0 + ms(380)).is_none()`; `:1852` - `assert_eq!(state.grace_deadline(), None, ...)` | PASS |
| C6 | locked session: next key-down emits `Stop`, stage `Processing`, and its key-up leaves nothing for the drain (AC 6) | `double_tap_press_on_locked_session_stops` ok | `transcription_coordinator.rs:1864-1866` - `matches!(state.on_input(dt(true), t0 + ms(5000)), Some(Effect::Stop { .. }))`; `:1868` stage `Processing`; `:1871` - `state.on_processing_finished().is_none()` | PASS |
| C7 | a 100 ms tap with no second press: `on_deadline(t0+500ms)` emits `Discard{binding}`, stage `Idle`, no `Stop` anywhere (AC 7) | `double_tap_window_expiry_discards` ok | `transcription_coordinator.rs:1888` - `assert_eq!(effects.len(), 1, ...)` through t0+499ms; `:1891` - `Some(Effect::Discard { binding_id }) => assert_eq!(binding_id, BINDING)`; `:1894` - `!effects.iter().any(...)` over `matches!(e, Effect::Stop { .. })`; `:1898` stage `Idle` | PASS |
| C8 | second key-down at 501 ms, before the deadline is processed, emits `Discard` and leaves `Idle` (AC 8) | `double_tap_late_second_press_discards` ok | `transcription_coordinator.rs:1909-1910` - `match state.on_input(dt(true), t0 + ms(501)) { Some(Effect::Discard { binding_id }) => assert_eq!(binding_id, BINDING)`; `:1914` - `assert_eq!(state.stage, Stage::Idle)` | PASS |
| C9 | window open: a stray key-up emits nothing, defers no release, window stays at t0+500ms (AC 9) | `double_tap_release_inside_window_is_ignored` ok | `transcription_coordinator.rs:1930` - `.is_none()`; `:1931` - `assert_eq!(state.grace_deadline(), None, ...)`; `:1932` - `assert_eq!(state.next_deadline(), Some(t0 + ms(500)))` | PASS |
| C10 | a 40 ms tap during `Processing` leaves nothing: `on_processing_finished()` is `None`, stage `Idle` (AC 10) | `double_tap_tap_during_processing_nets_noop` ok | `transcription_coordinator.rs:1949` - `state.on_processing_finished().is_none()`; `:1953` - `assert_eq!(state.stage, Stage::Idle)` | PASS |
| C11 | ~600 ms hold under 5 ms X11 auto-repeat pairs gives exactly 1 `Start` and 1 `Stop` (AC 11) | `double_tap_autorepeat_hold_is_one_hold` ok | `transcription_coordinator.rs:1989` - `assert_eq!((starts, stops, effects.len()), (1, 1, 2), ...)`; `:1970`/`:1975` no window and no lock in the middle of the burst | PASS |
| C12 | `on_cancel(true)` with the window open and with a locked session goes to `Idle`, clears lock and deadline; next key-down emits `Start` (AC 12) | `double_tap_cancel_clears_window_and_locked_session` ok | `transcription_coordinator.rs:2000-2005` (window case) and `:2013-2018` (locked case) - `assert_eq!(state.stage, Stage::Idle)`, `assert!(!state.is_locked())`, `assert_eq!(state.next_deadline(), None)`, `matches!(state.on_input(dt(true), ...), Some(Effect::Start { .. }))` | PASS |
| C13 | `next_deadline()` returns the earlier of grace and window in both orders, the only one when one is set, `None` with neither (AC 13) | `next_deadline_is_the_earliest_pending` ok | `transcription_coordinator.rs:2026` (`None`), `:2042` (only grace), `:2049` (grace earlier), `:2056` (window earlier), `:2059` (only window). Loop wiring read at `:673` - `if let Some(deadline) = state.next_deadline()` and `:677` - `state.on_deadline(Instant::now())` | PASS |
| C14 | inherited coordinator tests stay green with no assertion edited (door 1 regression) | `cargo test -p fala --lib transcription_coordinator` subset: all 49 coordinator tests ok, including `hold_or_toggle_*`, `toggle_*`, `push_to_talk_*`, `x11_autorepeat_burst_does_not_toggle_recording`, `failed_start_rolls_back_to_idle` | `git diff 881e77c..HEAD` removes no line inside `mod tests` of `transcription_coordinator.rs` (only `-` lines are in non-test code). The one test-side change is a new arm in the `drive` helper, `transcription_coordinator.rs:1051` - `Some(Effect::Discard { .. }) => panic!("push-to-talk never discards")`; the helper's counting assertions are unchanged (e.g. `:1331` - `assert!(matches!(effect, Some(Effect::Start { .. })))`, `:1334` - `assert_eq!(state.stage, Stage::Idle)`) | PASS |
| C15 | `run_effect` sends `Effect::Discard` to `utils::abort_current_operation`, whose body does not call `notify_cancel` (AC 15) | both proofs exit 0 (grep for `abort_current_operation` after `Effect::Discard`; negated awk/grep for `notify_cancel`) | `apps/desktop/src/transcription_coordinator.rs:807-810` - `Effect::Discard { binding_id } => { ... crate::utils::abort_current_operation(app); }`; `apps/desktop/src/utils.rs:104` - `pub fn abort_current_operation(app: &AppHandle) -> bool` (22-line body, no `notify_cancel`) | PASS |
| C16 | `cancel_current_operation` runs the same teardown via `abort_current_operation` and then notifies the coordinator (AC 14, no cancel regression) | both proofs exit 0 | `apps/desktop/src/utils.rs:89` - `let recording_was_active = abort_current_operation(app);`; `utils.rs:93` - `coordinator.notify_cancel(recording_was_active);` | PASS |
| C18 | `get_default_settings()` and `from_value(json!({}))` both give `PushToTalkDoubleTap` (AC 16) | `settings::tests::default_shortcut_activation_is_push_to_talk_double_tap` ok | `apps/desktop/src/settings.rs:1743-1745` - `assert_eq!(get_default_settings().shortcut_activation, ShortcutActivation::PushToTalkDoubleTap)`; `settings.rs:1749-1751` - `assert_eq!(from_empty.shortcut_activation, ShortcutActivation::PushToTalkDoubleTap)` | PASS |
| C19 | `PushToTalkDoubleTap` serializes to `"push_to_talk_double_tap"` and parses back (AC 17) | `settings::tests::push_to_talk_double_tap_round_trips_through_serde` ok | `settings.rs:1758` - `assert_eq!(value, serde_json::json!("push_to_talk_double_tap"))`; `settings.rs:1761` - `assert_eq!(parsed, ShortcutActivation::PushToTalkDoubleTap)` | PASS |
| C20 | on Windows/Linux `transcribe` = `ctrl+shift+space` and `transcribe_with_post_process` = `ctrl+space`, default and current (AC 18) | `settings::tests::default_bindings_put_dictation_on_ctrl_shift_space` ok (cfg windows/linux, ran on Linux) | `settings.rs:1769-1770` - `assert_eq!(transcribe.default_binding, "ctrl+shift+space")` / `current_binding`; `settings.rs:1772-1773` - `assert_eq!(post_process.default_binding, "ctrl+space")` / `current_binding` | PASS |
| C21 | stores with `"hold_or_toggle"`, `"push_to_talk"`, `"toggle"` parse strictly and keep the value after migrations (AC 19) | `settings::tests::stored_activation_modes_load_unchanged` ok | `settings.rs:1796` - `assert_eq!(settings.shortcut_activation, expected, "stored '{stored}'")` inside the 3-row table at `settings.rs:1780-1784` | PASS |
| C22 | first option `value` is `push_to_talk_double_tap`, `selected` falls back to it, pt "Segurar ou dois toques" / en "Hold or double-tap", descriptions non-empty in both (AC 20, 21) | 3 grep/python proofs exit 0; `bun run check:translations` exit 0 ("PT: All keys present"); `bun run lint` exit 0; `bunx tsc --noEmit` exit 0 | `src/components/settings/ShortcutActivation.tsx:20` - `value: "push_to_talk_double_tap",` (first option; next at `:29` is `hold_or_toggle`); `ShortcutActivation.tsx:52` - `"push_to_talk_double_tap") as ShortcutActivation;`; `src/i18n/locales/pt/translation.json:201` - `"pushToTalkDoubleTap": "Segurar ou dois toques"`; `src/i18n/locales/en/translation.json:201` - `"pushToTalkDoubleTap": "Hold or double-tap"`; descriptions at `pt/translation.json:207` and `en/translation.json:207` | PASS |
| C23 | generated `src/bindings.ts` has `"push_to_talk_double_tap"` in `ShortcutActivation` (door 1) | awk/grep proof exit 0 | `src/bindings.ts:1137` - `"push_to_talk_double_tap"` inside `export type ShortcutActivation =` at `src/bindings.ts:1117` | PASS |

## Deferred

| Check | Claim | Status | Reason |
| --- | --- | --- | --- |
| C17 | on the Windows build, a lone short tap on `Ctrl+Shift+Space` shows and hides the pill, plays no stop sound, pastes nothing, adds no history entry; Esc during a double-tap-locked recording does the same (AC 14, AC 12 on the real hook) | Unproven-by-design | `checks.md` approved C17 as a `TODO(windows)` manual proof (10 lone taps, 10 double-taps with speech, 5 double-taps cancelled by Esc; expected 0, 10, 0 pastes/history entries). It needs the Windows hook, overlay, paste and history on the phase-1 target, and this Linux machine cannot run it. It was not run, so it is not PASS. The PASS above covers the 22 checks this machine can settle. The behaviour parts of AC 14 (no paste, no history, no stop sound) have only structural proof here (C15, C16: `Discard` never reaches `stop()`, and the teardown is shared with cancel), until C17 runs |

## Coverage

Under `light` the join is read, not recomputed (that is `standard`). Read against the code: every
member in `checks.md`'s Coverage table points at a check that has a located assertion above.
Boundary pairs are asserted on both sides: 299/300 ms (C3 `:1815`, C2 `:1800`) and 499/500/501 ms
(C4 `:1826`, C8 `:1909`). The one member whose proof is pending is the AC 14 behaviour on the real
hook, which lands on C17 (deferred, above).

## Level and sampling

- C13: the unit test proves `next_deadline()` picks the minimum. The thread loop that consumes it (`transcription_coordinator.rs:673-677`) needs an `AppHandle`, so no test runs it; I checked it by reading. This is a level gap the checks accept, not a failure.
- C15/C16 are structural grep proofs, as approved. C15's second proof (`! awk ... | grep -q notify_cancel`) would also pass if `abort_current_operation` did not exist. C15's first proof and `utils.rs:104` rule that case out. **Precision gap** in the check's wording, not in the code.
- C13 sets `pending_release` and `tap_deadline` directly rather than reaching both through inputs. That is acceptable for a claim about `next_deadline()` alone.

## Rulings on the scrutiny items

- **(a) Two inherited settings assertions now expect `PushToTalkDoubleTap`. Legitimate, not weakened.** Plan AC 16 says: "The settings SHALL default `shortcut_activation` to `push_to_talk_double_tap`, both in `get_default_settings()` and when the stored object lacks the key". The approved Landing door 1 moves `#[default]` to the new variant (`settings.rs:174-175`). `empty_store_parses_with_defaults` (`settings.rs:1279-1281`) parses `{}`, which is the "stored object lacks the key" case. `shortcut_activation_defaults_to_push_to_talk_double_tap_without_legacy_key` (`settings.rs:1625-1627`) starts from `get_default_settings()` and migrates a raw store that has no key at all. Both expectations are now exactly what AC 16 requires. The assertions keep the same form (exact `assert_eq!` on the variant), so the expected value changed but the strength did not. The `HoldOrToggle` assertions that guard old stores remain untouched: `shortcut_activation_migration_respects_explicit_new_key` (`settings.rs:1603`, still `HoldOrToggle` at `:1613-1615`) and the legacy `push_to_talk` mappings. The new C21 (`settings.rs:1796`) adds an explicit 3-mode no-migration table. The author disclosed the change before any code, in `checks.md` at the planning commit `6fdba31`, which is the approved artifact, and repeated it in the `9653ae2` commit body.
- **(b) `drive` helper arm. Confirmed, no inherited assertion changed.** The only `-` lines that `git diff 881e77c..HEAD -- apps/desktop/src/transcription_coordinator.rs` shows are in non-test code (classify_busy_input, hold_to_talk, finish_hold call, Hold literal, loop). The test module gains one line at `:1051` and a new block from `:1747` on. Before the change the exhaustive `match` could not compile with the new variant. The new arm panics on `Discard`, so the helper is stricter: a push-to-talk sequence that discarded would fail the test instead of being silently uncounted. All 49 coordinator tests pass.
- **(c) `src/bindings.ts`. Confirmed: one hunk, the `ShortcutActivation` one only.** `git diff 881e77c..HEAD -- src/bindings.ts` has exactly 1 `@@` hunk (`-1128,7 +1128,13`). It turns `"hold_or_toggle"` into `"hold_or_toggle" | ` and appends the doc comment and `"push_to_talk_double_tap"`. The doc text matches the Rust doc comment at `settings.rs:171-173` word for word, and the formatting (`" | "` with a trailing space, `/** */` per member) matches the existing generated members (`src/bindings.ts:1117-1137`). No `isLaptop` or other unrelated hunk is present. I did not re-run the generator myself, so "identical to the generator output" rests on that format and text match, not on a byte diff.

## AGENTS.md invariants (apps/desktop)

- New `cfg`: the only one is `#[cfg(any(target_os = "windows", target_os = "linux"))]` on a test in `apps/desktop/src/settings.rs:1764`, which is an allowed place (ADR-0007). Nothing outside `apps/desktop`, `src/` and `.specs/` changed.
- i18n: both new visible strings (option label, description) go through `t(...)` at `ShortcutActivation.tsx:21-26` and exist in pt and en (C22). `check:translations` and `lint` (which rejects literal strings in JSX) pass.
- Branding: `scripts/check-brand.sh` gives "ok: no Handy branding outside the allowlist" (exit 0), and no added line mentions Handy. `scripts/check-no-tauri-in-crates.sh` exit 0.
- `cargo clippy -p fala --all-targets -- -D warnings` exit 0. `cargo fmt -p fala -- --check` exit 0. `bun run format:check` (prettier + `cargo fmt --all --check`) exit 0.
- `unwrap`/`expect` show up only in added test code (`settings.rs` tests, coordinator `next_deadline_is_the_earliest_pending`).
- Logging: the new lines are `debug!` with binding ids and durations only, no dictated content.

## Swept existing

- dependency failure (existing): `on_start_result` is at `transcription_coordinator.rs:604-608` (rolls back to `Idle` and clears `hold` when start fails), and `failed_start_rolls_back_to_idle` asserts it at `:1334`. It ran green. The constraint is present.
- The other Swept rows resolve to checks proven above (C8, C10, C12, C13, C15, C21) or are `n/a` policy (authorization, observability).

## Test policy rows

None - `checks.md` has no `Test policy` section (`light`).

## Faults injected

None - not run under `light` (fault injection is a `standard`/`ui` step). Because of that, this report does not show that the new tests would fail under a wrong implementation.

## Gate

`cargo test -p fala --lib -- transcription_coordinator settings::tests` - 76 passed, 0 failed (212 filtered out)
`bun run check:translations` / `bun run lint` / `bunx tsc --noEmit` - exit 0 each
grep/awk/python proofs for C15, C16, C22, C23 - 9 of 9 exit 0
`cargo clippy -p fala --all-targets -- -D warnings` - exit 0; `scripts/check-brand.sh` - exit 0
