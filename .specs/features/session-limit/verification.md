# session-limit verification

**Verdict**: FAIL
**Profile**: light
**Diff range**: 0b92c67..058ed22
**Round**: 1 - full
**Verifier**: independent sub-agent (author != verifier) - verifier sub-agent dispatched by the orchestrator; the author is the desktop-phase1c executor sub-agent

The only check not proven is C15, the manual 20-minute Windows run (`TODO(windows)`). It has no
proof that can run on this Linux machine, so it is Unproven and the verdict is FAIL until that
session is recorded. C1-C14, C16 and C17 are proven at `058ed22`.

## Binding sources

None under `light` - step 1 runs only under `ui`. The plan's `Sources` (pitch, design doc §3.3,
delta plan) were not opened.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | warns at 19 min, not 1 ms before, stays `Recording`, all 4 modes | `cargo test -p fala --lib -- transcription_coordinator ...` - `session_limit_warns_at_nineteen_minutes_in_every_mode ... ok` | `apps/desktop/src/transcription_coordinator.rs:2346` - `assert_eq!(state.on_deadline(t0 + mins(19) - ms(1)), None, ..)`; `:2351` - `assert_eq!(state.on_deadline(t0 + mins(19)), limit_warning(), ..)`; `:2356` - `assert_eq!(state.stage, Stage::Recording(BINDING.to_string()), ..)`; loop over `LIMITED_MODES` (4 modes) | PASS |
| C2 | warning once; next mark becomes 20 min | same run - `session_limit_warns_once ... ok` | `transcription_coordinator.rs:2371` / `:2375` - `assert_eq!(state.on_deadline(t0 + mins(19) + Duration::from_secs(1|30)), None)`; `:2379` - `assert_eq!(state.session_deadline(), Some(t0 + mins(20)))` | PASS |
| C3 | cut at 20 min (not 1 ms before) is `Stop` with binding and hotkey, to `Processing`, no `Discard`, all 4 modes | same run - `session_limit_cuts_at_twenty_minutes_with_stop ... ok` | `transcription_coordinator.rs:2391` - `assert_eq!(state.on_deadline(t0 + mins(20) - ms(1)), None, ..)`; `:2397` - `assert_eq!(cut, Some(Effect::Stop { binding_id: BINDING.., hotkey_string: BINDING.. }), ..)`; `:2406` - `assert_eq!(state.stage, Stage::Processing, ..)`; `:2407` - `assert!(!effects.iter().any(\|e\| matches!(e, Effect::Discard { .. })), ..)` | PASS |
| C4 | overslept wake at 25 min cuts with `Stop`, no warning; then `None` | same run - `session_limit_overslept_cuts_without_warning ... ok` | `transcription_coordinator.rs:2420` - `assert!(matches!(state.on_deadline(t0 + mins(25)), Some(Effect::Stop { .. })), ..)`; `:2424` - `assert_eq!(state.stage, Stage::Processing)`; `:2425` - `assert_eq!(state.on_deadline(t0 + mins(25) + ms(1)), None)` | PASS |
| C5 | `wake_deadline()` = earliest of grace, window, session mark; `next_deadline()` keeps F1 meaning | same run - `wake_deadline_is_the_earliest_of_input_and_session ... ok` | `transcription_coordinator.rs:2439` - `assert_eq!(state.next_deadline(), None)`; `:2440` - `assert_eq!(state.wake_deadline(), Some(t0 + mins(19)))`; `:2443` - `Some(t0 + ms(1000) + RELEASE_GRACE)`; `:2452` - `Some(t0 + ms(500))`; `:2457` - locked `next_deadline() == None`; `:2462` - `Some(t0 + mins(19))`; `:2465` - `Some(t0 + mins(20))` | PASS |
| C6 | the loop sleeps until `wake_deadline()` | `grep -q "let cmd = if let Some(deadline) = state.wake_deadline()" ...` exit 0; fails (exit 1) when the loop is reverted to `next_deadline()` in a scratch worktree | `apps/desktop/src/transcription_coordinator.rs:768` - `let cmd = if let Some(deadline) = state.wake_deadline() {`; the only other non-test caller of `next_deadline()` is `wake_deadline` itself (`:305`) | PASS |
| C7 | no session deadline after stop, discard, cancel, failed start, or new state; `on_deadline(t0+20min)` is `None` in all 4 | same run - `session_deadline_clears_when_the_recording_ends ... ok` | `transcription_coordinator.rs:2472` - `assert_eq!(CoordinatorState::new().session_deadline(), None)`; `:2485`/`:2486` "stop", `:2495`/`:2496` "discard", `:2502`/`:2503` "cancel", `:2512`/`:2513` "failed start" - each `assert_eq!(state.session_deadline(), None, ..)` and `assert_eq!(state.on_deadline(t0 + mins(20)), None, ..)` | PASS |
| C8 | drained recording measures both marks from the remembered key-down | same run - `drained_recording_measures_limit_from_key_down ... ok` | `transcription_coordinator.rs:2529` - `assert_eq!(state.session_deadline(), Some(t1 + mins(19)))`; `:2530` - `assert_eq!(state.on_deadline(t1 + mins(19)), limit_warning())`; `:2531` - `assert!(matches!(state.on_deadline(t1 + mins(20)), Some(Effect::Stop { .. })))` | PASS |
| C9 | held PTT key-up after the cut: no effect, no drain, `Idle` | same run - `release_after_limit_cut_starts_nothing ... ok` | `transcription_coordinator.rs:2550` - `assert_eq!(state.on_input(input(ptt, false), released), None)`; `:2551` - grace `None`; `:2552` - `assert_eq!(state.on_processing_finished(), None)`; `:2553` - `assert_eq!(state.stage, Stage::Idle)` | PASS |
| C10 | double-tap tap after the cut: no effect, no drain, `Idle` | same run - `tap_after_limit_cut_starts_nothing ... ok` | `transcription_coordinator.rs:2568`/`:2569`/`:2570` - press, release, grace each `None`; `:2571` - `assert_eq!(state.on_processing_finished(), None)`; `:2572` - `assert_eq!(state.stage, Stage::Idle)` | PASS |
| C11 | inherited coordinator tests green with no assertion edited | same run - 62 `transcription_coordinator` tests ok (53 inherited + 9 new), 0 failed | `git diff 0b92c67..HEAD` removes exactly 3 lines in `apps/` and `src/` (the `use log` line, the loop's `next_deadline()` line, the old `.sdot` span) - no test line removed; the only edit in the inherited test region is the new `panic!` arm in `drive` at `transcription_coordinator.rs:1165`-`:1167`; assertion count in lines 1-2290 equals the base file's (unchanged); F1 `next_deadline() == None` asserts still present e.g. `:2015`, `:2118`, `:2131`, `:2142` | PASS |
| C12 | `LimitWarning` -> `play_limit_warning` + `emit_recording_limit_warning`; two `play_test_sound` 150 ms apart, no `play_feedback_sound` | 5 grep/awk proofs, all exit 0; each fails (exit 1) under its own fault in a scratch worktree | `apps/desktop/src/transcription_coordinator.rs:922` - `crate::audio_feedback::play_limit_warning(app);`; `:923` - `crate::utils::emit_recording_limit_warning(app);`; `apps/desktop/src/audio_feedback.rs:82` and `:84` - `play_test_sound(&app, SoundType::Start);`; `:83` - `thread::sleep(Duration::from_millis(150));`; `play_test_sound` (`audio_feedback.rs:69`-`:73`) has no `audio_feedback` gate, unlike `play_feedback_sound` (`:49`-`:52`) | PASS |
| C13 | emitter skips when overlay disabled; else emits only to `recording_overlay` | 2 awk/grep proofs exit 0; each fails under its fault | `apps/desktop/src/overlay.rs:617` - `if !OVERLAY_ENABLED.load(Ordering::Relaxed) {`; `:622` - `let _ = handle.emit_to("recording_overlay", "recording-limit-warning", ());` | PASS |
| C14 | the cut is the shortcut's `Stop`; all-or-nothing delivery still green | grep proof exit 0 (fails when the arm calls another function); `actions::tests::delivery_is_all_or_nothing_after_the_last_check ... ok` | `transcription_coordinator.rs:914` - `} => stop(app, &binding_id, &hotkey_string),` (the cut returns `begin_processing(..)`, the same `Effect::Stop` builder the release path uses); `apps/desktop/src/actions.rs:1076`-`:1077` - `assert!(!delivered); assert!(calls.borrow().is_empty(), ..)`; `:1086`-`:1087` - `assert!(delivered); assert_eq!(*calls.borrow(), vec!["paste", "save"])` | PASS |
| C15 | Windows: amber dot + two chimes at 19:00, cut at 20:00, text in Notepad, 1 history entry | not run - `TODO(windows)` manual 20-minute session; no Windows machine in this verification | no evidence - manual check pending | Unproven |
| C16 | listener sets `limit` class on `.sdot`; reset on show recording/streaming and hide; `.sdot.limit` uses `--color-warning` with own animation | 4 grep/awk proofs exit 0 (each fails under its fault); `bun run lint` exit 0; `bunx tsc --noEmit` exit 0 | `src/overlay/RecordingOverlay.tsx:102` - `listen("recording-limit-warning", () => { setLimitWarning(true); })`; `:67` (inside `overlayState === "recording" \|\| "streaming"`) and `:99` (`hide-overlay`) - `setLimitWarning(false);`; `:219` - `${limitWarning ? " limit" : ""}`; `src/overlay/RecordingOverlay.css:338` - `background: var(--color-warning);`, `:340` - `animation: sdot-limit-pulse 0.9s infinite;` | PASS |
| C17 | front formatted, translations complete | `bun run format:check` exit 0 (Prettier + `cargo fmt --all -- --check`); `bun run check:translations` exit 0 | `package.json` scripts; output "All matched files use Prettier code style!" and "All 1 languages have complete translations!"; no visible string added - `git diff 0b92c67..HEAD -- src/` adds no JSX text literal (only a class name and listeners) | PASS |

## Level and sampling

- C1 and C3 assert both sides of each edge (19 min - 1 ms / 19 min; 20 min - 1 ms / 20 min) in
  all 4 modes, table-driven over `LIMITED_MODES`. No claim covers more cases than its proof runs.
- C6, C12, C13, C14 (first proof) and C16 (first four) are structural greps over source, not
  behaviour. That is the level the checks chose for `run_effect` and the Tauri/React side, which
  have no test harness here. Each grep was shown to fail under a targeted fault (see below).
  Precision note, not a failure: C16's `setLimitWarning(false)` count (`-ge 2`) does not say
  where the two resets sit; I read them at `RecordingOverlay.tsx:67` (show recording/streaming)
  and `:99` (hide), which matches AC 14.
- Under `light` I did not inject behavioural faults into the coordinator, so a coordinator test
  passing under a wrong implementation stays undetected, as the profile's row says.

## Structural proofs shown to fail (scratch worktree)

Not required under `light`; the orchestrator asked for it. Scratch worktree
`/tmp/claude-1000/verify-sl` at `058ed22` (detached), one fault per proof, proof re-run, file
restored with `git checkout -- .` before the next fault. Worktree removed afterwards; the real
tree's `git status --porcelain` matched the baseline (empty) before and after.

| Proof | Fault | Proof exit under fault |
| --- | --- | --- |
| C6 | loop back to `state.next_deadline()` | 1 |
| C12 a | delete the `play_limit_warning(app)` call | 1 |
| C12 b | delete the `emit_recording_limit_warning(app)` call | 1 |
| C12 c | drop one of the two chimes | 1 |
| C12 d | gap `150` -> `300` ms | 1 |
| C12 e | first chime through `play_feedback_sound` | 1 |
| C13 a | remove the `OVERLAY_ENABLED` guard | 1 |
| C13 b | `emit_to("recording_overlay", ..)` -> broadcast `emit(..)` | 1 |
| C14 a | `Stop` arm calls `limit_stop(..)` | 1 |
| C16 a | listener renamed to `recording-limit` | 1 |
| C16 b | remove the reset on `hide-overlay` | 1 |
| C16 c | class expression replaced by `""` | 1 |
| C16 d | `.sdot.limit` background -> `var(--s-accent)` | 1 |

## Scrutiny requested by the orchestrator

- **Inherited assertions.** `drive` gained only `Some(Effect::LimitWarning { .. }) => panic!(..)`
  (`transcription_coordinator.rs:1165`-`:1167`); the diff removes no test line. `next_deadline()`
  is unchanged and still asserted `None` while locked by F1 tests; the loop uses the new
  `wake_deadline()` (`:304`-`:309`, `:768`).
- **One stop path.** `on_session_mark` (`:345`-`:361`) returns `self.begin_processing(binding_id,
  hotkey_string)` at the limit, the same `Effect::Stop` producer as a key release; `run_effect`'s
  `Stop` arm is unchanged (`:911`-`:914`) and there is no limit-specific branch. It also clears
  `pending_release`, as the Handoff records. `delivery_is_all_or_nothing_after_the_last_check`
  passed at HEAD.
- **overlay.rs.** The only hunk adds `emit_recording_limit_warning` (`overlay.rs:613`-`:624`). No
  window builder, size, position, focus, `always_on_top` or flag line is touched (`git diff
  0b92c67..HEAD -- apps/desktop/src/overlay.rs` is a single `@@ -610,6 +610,19 @@` hunk of added
  lines).
- **AGENTS.md.** No reformatting outside touched hunks (format check green; 3 removed lines, all
  intended). `scripts/check-brand.sh` exit 0 ("ok: no Handy branding outside the allowlist").
  No visible string added, so no i18n key owed. Logs: cut at `info!` and warning at `debug!`,
  binding id only, no dictated content. `cargo clippy -p fala --all-targets -- -D warnings`:
  exit 0, no warnings (only the build script's `fala@0.1.0: Staged 18 transcribe-cpp runtime library file(s)` notice).

## Swept existing

- dependency failure: "falha ao abrir a saída de áudio ou achar o som é logada por
  `play_sound_blocking`" - present: `play_test_sound` resolves the path with `if let Some(path)`
  and calls `play_sound_blocking` (`audio_feedback.rs:69`-`:73`), which logs a failure with `error!` (`audio_feedback.rs:98`-`:99`) and reads the feedback volume
  and output device (`audio_feedback.rs:105`-`:107`); the warning runs on its
  own spawned thread (`audio_feedback.rs:81`), so it cannot block the cut.
- Observable "Esc durante o processamento ... regra da 1.F2" - present: delivery goes through the
  all-or-nothing path proven by `actions.rs:1076`-`:1087`.

## Gate

`CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2 cargo test -p fala --lib -- transcription_coordinator actions::tests::delivery_is_all_or_nothing_after_the_last_check` - exit 0, `test result: ok. 63 passed; 0 failed; 0 ignored; 246 filtered out` (62 coordinator + 1 delivery); each of the 10 named tests listed individually as `... ok`
`bun run lint`, `bunx tsc --noEmit`, `bun run format:check`, `bun run check:translations`, `scripts/check-brand.sh` - all exit 0

## Ranked gaps

1. C15 Unproven - manual 20-minute Windows session (`TODO(windows)`): warning instant (19:00 ± 1 s),
   cut instant (20:00 ± 1 s), non-empty paste in Notepad, one new history entry - no evidence.
