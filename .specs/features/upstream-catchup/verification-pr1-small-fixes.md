# upstream-catchup PR 1 (small fixes) verification

**Verdict**: PASS
**Profile**: light
**Diff range**: eb0482c..e658f46
**Round**: 2 - scoped
**Verifier**: independent sub-agent (author != verifier)

Round 1 (below, through `## Gate`) was run at `162f90c` and is carried from `162f90c`; round 2
(`## Round 2 - scoped`, at the end) was verified at `e658f46`. Every round 1 proof was run at
`162f90c` in a detached worktree of that commit. The checks file is
`.specs/features/upstream-catchup/checks-pr1-small-fixes.md` (no `plan.md`; its `## Intent` is
the plan). One proof command (C5) is malformed as written; the claim itself holds under the
corrected invocation (finding 1).

## Binding sources

None. The checks carry no binding source (no design, no external contract). The upstream commits
were opened for comparison only (see "Upstream comparison").

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | no model loaded and `get_model_path` errors -> log `Not starting recording` and return before tray, overlay and `try_start_recording` | awk order proof, exit 0 (lines 575 < 582 < 635) | `apps/desktop/src/actions.rs:575` - `warn!("Not starting recording: no model can transcribe it ({})", e); return;` inside `if !tm.is_model_loaded()` (`:569`), before `set_tray_state(app, TrayIconState::Recording)` at `:582` and `rm.try_start_recording(` at `:635` | PASS |
| C2 | coordinator `start()` returns `is_recording()`; the existing failed-start test stays green | `cargo test -p fala --lib -- transcription_coordinator::tests audio_toolkit::text::tests` exit 0; `transcription_coordinator::tests::is_busy_follows_the_stage ... ok` | `apps/desktop/src/transcription_coordinator.rs:938` - `.is_some_and(... a.is_recording());` returned at `:942`; `apps/desktop/src/transcription_coordinator.rs:2231` - `assert!(!state.is_busy(), "a failed start rolls back to idle");` | PASS |
| C3 | desktop filter: leading filler, filler after `.`, mid-sentence filler | same desktop invocation; `audio_toolkit::text::tests::test_filter_leading_filler_keeps_sentence_capital ... ok` | `apps/desktop/src/audio_toolkit/text.rs:586` - `assert_eq!(result, "So I think we should ship it.")`; `:589` - `assert_eq!(result, "That works. Let me check.")`; `:593` - `assert_eq!(result, "He said, not today.")` | PASS |
| C4 | postproc pt-BR: `.`, `!`, `?`, `…`, two fillers in a row, mid-sentence, lowercase filler | `cargo test -p fala-postproc rules::tests::keeps_sentence_capital_after_removed_filler` exit 0, `1 passed` | `crates/postproc/src/rules.rs:270` - `pt("Isso funciona. Uhm, deixa eu ver.")` == `"Isso funciona. Deixa eu ver."`; `:274` `"Pronto! Então vamos."`; `:275` `"Será? É isso."`; `:276` `"Certo… Ótimo."`; `:278` `"Ele disse, hoje não."`; `:280` `"Isso funciona. deixa."` | PASS |
| C5 | pre-existing postproc filler tests green with no assertion edited | as written: exit 1 (`unexpected argument`); corrected `cargo test -p fala-postproc -- rules::tests::removes_pt_br_fillers rules::tests::en_keeps_pt_only_fillers` exit 0, both `... ok`; `git diff eb0482c..162f90c -- crates/postproc/src/rules.rs` adds lines only in tests | `crates/postproc/src/rules.rs:245` - `assert_eq!(pt("então, hã, eu acho"), "Então, eu acho")` (unchanged); `crates/postproc/src/rules.rs:286` - `assert_eq!(en("ahn ok"), "Ahn ok")` (unchanged) | PASS |
| C6 | "Ha Long Bay is beautiful." unchanged in English; "ha" out of the `en` list | same desktop invocation; `audio_toolkit::text::tests::test_filter_keeps_ha_in_english ... ok` | `apps/desktop/src/audio_toolkit/text.rs:599` - `assert_eq!(result, "Ha Long Bay is beautiful.")`; list at `apps/desktop/src/audio_toolkit/text.rs:316` - `"en" => &["um", "ah", "eh"]` | PASS |
| C7 | `init_transcribe_backend` no longer lists devices; `report_compute_devices` called only from pre-warm thread and headless path | both proofs exit 0; `grep -rn report_compute_devices apps/desktop/src` -> definition plus exactly 2 call sites | `apps/desktop/src/lib.rs:1048` - `crate::managers::transcription::report_compute_devices();` inside the `std::thread::spawn(` closure opened at `:1047`; `apps/desktop/src/lib.rs:932` - headless path (`if headless_mode {` at `:920`); `apps/desktop/src/managers/transcription.rs:1907` - `let devices = transcribe_compute_devices();` inside `pub fn report_compute_devices()` | PASS |
| C8 | `installerIcon` = `icons/icon.ico`, template uses `{{installer_icon}}`, no `signCommand` | jq/grep proof exit 0 | `apps/desktop/tauri.conf.json:78` - `"installerIcon": "icons/icon.ico",`; `apps/desktop/nsis/installer.nsi:48` - `!define INSTALLERICON "{{installer_icon}}"` | PASS |
| C9 | `Dropdown` calls `onOpen` only on opening; `SoundPicker` passes `onOpen={checkCustomSounds}`; no `onRefresh` left in `src/` | grep proof exit 0; `bun run lint` exit 0; `bun run build` (tsc + vite) exit 0, `built in 8.85s` | `src/components/ui/Dropdown.tsx:60` - `if (!isOpen) onOpen?.();`; `src/components/settings/SoundPicker.tsx:60` - `onOpen={checkCustomSounds}` | PASS |
| C10 | no Handy brand added in code | `scripts/check-brand.sh` exit 0 (`ok: no Handy branding outside the allowlist`); a case-insensitive grep for the brand over the added lines of the diff hits only the checks file prose (lines 9 and 74 of the added spec), none in code | `scripts/check-brand.sh:1` - script run, exit 0 | PASS |

## Coverage

Profile light: the join is not recomputed (that step runs under `standard`/`ui`). Read only, as a
sanity pass:

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| upstream commits in this PR (6) | not recomputed - profile light; `git log eb0482c..162f90c` shows the 6 | as in checks (C1-C4, C6-C9) | - |

## Level and sampling gaps

- C1, C7, C8 and C9 are structural (order and presence in source), as the checks declare; no
  behaviour test reaches `TranscribeAction::start`, the pre-warm thread or the NSIS bundle. C2's
  proof is a pre-existing state-machine test (`on_start_result(BINDING, false)`), untouched by
  the diff; the link from C1's early `return` to the rollback is the `is_recording()` read at
  `transcription_coordinator.rs:938`, verified by reading, not by a test. Accepted under light.
- The Intent says "Isso funciona. Uhm, deixa eu ver" becomes "Isso funciona. Deixa eu ver" "nos
  dois filtros". The pt-BR sentence is only asserted in the postproc filter (C4). The desktop
  filter is asserted on English strings only (C3); "uhm" is a universal filler there, so the
  same code path applies, but no desktop test exercises a pt-BR sentence. Sampling gap, not a
  failure.
- C5 claims "no assertion edited" for postproc only. In the desktop filter the cherry-pick of
  upstream #2157 does edit two pre-existing assertions (`text.rs:552` "This is a test" and
  `text.rs:580` "So I was, thinking about this"). Both edits are byte-identical to upstream
  eb49dc0 and follow from the intended behaviour (a capitalized leading filler now hands its
  capital on). No check covers that; noted so the reviewer is not surprised.

## Swept rows re-read

All `Swept` rows cite checks (C1, C2, C7, C8) or are `n/a`; none says "existing". Re-read
against the code anyway: the `observability` row (C1's `warn!` logs no dictated content) holds -
`actions.rs:575` logs only the model-path error.

## Upstream comparison

- The five cherry-picks (69bf487, f079182, 5e59d1b, d9e29b2, ade6329) carry the upstream author
  and `(cherry picked from commit ...)`. Their `+`/`-` hunks are byte-identical to upstream
  8ef8dd4, eb49dc0, eea1f5f, 29bd2c0 and 345caa8 respectively (diffed hunk by hunk).
- 4221a06 takes only the `installerIcon` line of upstream #2117, without `signCommand`, as the
  Intent decides (ADR-0008).
- Postproc reimplementation (9db9457) against upstream eb49dc0: same rule. A removed filler that
  starts uppercase and opens a sentence (nothing before it, or the previous kept text ends in
  `.`, `!`, `?`, `…`) sets a debt (`|=`, so a following lowercase filler keeps it); the next
  word with content is capitalized and the debt clears. Upstream looks at the last visible char
  of the kept text; postproc looks at the previous kept token's `trail`. These agree because the
  tokenizer glues loose closing punctuation onto the previous token. Upstream settles on the
  first alphanumeric char; postproc skips empty-`core` tokens and capitalizes the first
  alphabetic char of `core` (digits pass through unchanged in both). One divergence I found, not
  a regression: a filler token that carries the period itself ("ok Uhm. então") drops that
  period along with the filler, so the next word is not capitalized. That was already how
  postproc behaved before this diff.

## Faults injected

none - profile light

| Mutation | Location | Killed |
| --- | --- | --- |
| none - profile light | - | - |

## Findings

1. **C5's proof command is malformed.** `cargo test -p fala-postproc rules::tests::removes_pt_br_fillers rules::tests::en_keeps_pt_only_fillers`
   exits 1 with `error: unexpected argument 'rules::tests::en_keeps_pt_only_fillers' found`.
   Cargo takes one positional TESTNAME. The working form is
   `cargo test -p fala-postproc -- rules::tests::removes_pt_br_fillers rules::tests::en_keeps_pt_only_fillers`.
   This defect is in the checks file, not the code. Fix the command before anyone re-runs the
   checks.
2. Sampling gap: the pt-BR example in the Intent is not asserted in the desktop filter (see Level
   and sampling gaps). Non-blocking.
3. The C10 grep hits "Handy" only in the prose of the checks file under `.specs/`, which
   `scripts/check-brand.sh` accepts. No code, config or UI string adds the brand.
4. Upstream #2157 changed two existing desktop test expectations (see above). The change is
   intended, but no check names it.

## Gate

`cargo test -p fala --lib -- transcription_coordinator::tests audio_toolkit::text::tests` - 107 passed, 0 failed (244 filtered out)
`cargo test -p fala-postproc` - 9 + 14 passed, 0 failed (whole crate, no regressions)
`bun run lint` exit 0 · `bun run build` exit 0 · `scripts/check-brand.sh` exit 0 · structural proofs C1, C7 (x2), C8, C9 exit 0

The report gate (`validate_verification.py`) resolves a feature directory and reads
`verification.md` and `checks.md` from it. Pointed at this file it exits 2 (`feature not
found`). It exits 0 when run on a scratch directory holding this report as `verification.md` and
the checks as `checks.md`.

## Round 2 - scoped

Verified at `e658f46`, in a detached worktree of that commit. Scope: the diff `162f90c..e658f46`
(two commits: `e2f31ae`, which restricts the asset protocol scope for upstream issue #1384, and
`e658f46`, which adds C11 to the checks file and fixes C5's command) plus every round 1 item
that was not a clean pass (finding 1, C5's malformed command). C1, C7 and C9 are carried from
`162f90c`: the diff touches none of `actions.rs`, `lib.rs`, `transcription.rs`,
`Dropdown.tsx`, `SoundPicker.tsx` or `package.json`. C2, C3, C4 and C6 are not in scope, but
their tests ran again in the same invocations at `e658f46` and pass.

### What else the diff touches

`git diff --name-only 162f90c..e658f46` lists exactly three files:
`.specs/features/upstream-catchup/checks-pr1-small-fixes.md`,
`apps/desktop/src/managers/history.rs` and `apps/desktop/tauri.conf.json`. In `tauri.conf.json`
the only changed line is `apps/desktop/tauri.conf.json:20` (`"allow": ["**"]` became
`"allow": ["$APPDATA/recordings/**"]`). `installerIcon` (C8) and the absence of `signCommand`
are untouched. `history.rs` adds a `warn` import, a `Manager` import, the call at
`apps/desktop/src/managers/history.rs:119`, the helper at `:934` and the test at `:952`. No
other check's code path is involved. In the checks file only the Intent, the count line, C5's
command, the new C11, two Coverage rows and the `authorization` Swept row changed. No other
check's claim or proof was edited.

### Checks (round 2)

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C5 | the postproc filler tests that already existed stay green, with no assertion edited | corrected command as now written, `cargo test -p fala-postproc -- rules::tests::removes_pt_br_fillers rules::tests::en_keeps_pt_only_fillers`, exit 0, `2 passed`, both `... ok`; `git diff eb0482c..e658f46 -- crates/postproc/src/rules.rs` removes 0 lines containing `assert`, and no file under `crates/` changed in `162f90c..e658f46` | `crates/postproc/src/rules.rs:245` - `assert_eq!(pt("então, hã, eu acho"), "Então, eu acho")`; `crates/postproc/src/rules.rs:286` - `assert_eq!(en("ahn ok"), "Ahn ok")` | PASS |
| C8 | `installerIcon` = `icons/icon.ico`, the template uses `{{installer_icon}}`, no `signCommand` | jq/grep proof exit 0 at `e658f46` | `apps/desktop/tauri.conf.json:78` - `"installerIcon": "icons/icon.ico",`; `apps/desktop/nsis/installer.nsi:48` - `!define INSTALLERICON "{{installer_icon}}"` | PASS |
| C10 | no Handy brand added in code | `scripts/check-brand.sh` exit 0 (`ok: no Handy branding outside the allowlist`); a grep over the added lines of `162f90c..e658f46` hits one code line, the provenance comment `cjpais/Handy#1384`, which the script's `cjpais/` allowlist entry admits (same form as `apps/desktop/src/tray.rs:12` and `apps/desktop/src/lib.rs:978`) | `apps/desktop/src/managers/history.rs:933` - `/// Nothing else on disk is readable from the WebView (cjpais/Handy#1384).` | PASS |
| C11 | `assetProtocol.scope.allow` is exactly `["$APPDATA/recordings/**"]` with `enable: true`, and `HistoryManager::new` calls `asset_protocol_scope().allow_directory` on the `recordings` folder it created, recursively | `cargo test -p fala --lib -- managers::history::tests::asset_protocol_reads_only_the_recordings_folder` (same invocation as the C2/C3/C6 filters; `fala` compiled from the `e658f46` worktree), `managers::history::tests::asset_protocol_reads_only_the_recordings_folder ... ok`, 4 passed; awk/grep structural proof exit 0 | `apps/desktop/src/managers/history.rs:956` - `assert_eq!(scope["allow"], serde_json::json!(["$APPDATA/recordings/**"]), ...)`; `apps/desktop/src/managers/history.rs:961` - `assert_eq!(conf["app"]["security"]["assetProtocol"]["enable"], true)`; `apps/desktop/src/managers/history.rs:119` - `allow_recordings_in_asset_scope(app_handle, &recordings_dir);` inside `pub fn new` (`:108`), after `create_dir_all` at `:116`; `apps/desktop/src/managers/history.rs:937` - `.allow_directory(recordings_dir, true)` | PASS |

Also re-run, out of scope, in the same invocations: `transcription_coordinator::tests::is_busy_follows_the_stage`,
`audio_toolkit::text::tests::test_filter_leading_filler_keeps_sentence_capital`,
`audio_toolkit::text::tests::test_filter_keeps_ha_in_english` and
`rules::tests::keeps_sentence_capital_after_removed_filler`, all `... ok`.

### Upstream issue #1384: does the scope still play recordings and close the hole?

Read against the code at `e658f46` and the vendored `tauri-2.11.5` source (`src/scope/fs.rs`,
`src/protocol/asset.rs`).

- **Only one user of the asset protocol.** `grep -rn 'convertFileSrc|asset://|asset.localhost|asset_protocol' src apps/desktop/src`
  finds only `src/components/settings/history/HistorySettings.tsx:202`
  (`convertFileSrc(result.data, "asset")`) and the new code in `history.rs`. Nothing else in
  the app goes through the asset protocol, so the narrower scope breaks nothing else.
- **Normal mode.** `get_audio_file_path` (`apps/desktop/src/commands/history.rs:41`) returns
  `recordings_dir.join(file_name)` (`apps/desktop/src/managers/history.rs:792`), where
  `recordings_dir` is `portable::app_data_dir(app)/recordings` (`:111`). Outside portable mode
  that is `app.path().app_data_dir()` (`apps/desktop/src/portable.rs:69`), the
  same directory Tauri resolves `$APPDATA` to when it builds the static scope. Both the static
  pattern and the runtime `allow_directory` cover it. Playback keeps working.
- **Portable mode.** `recordings_dir` becomes `<exe dir>/Data/recordings`, which `$APPDATA`
  does not cover. The runtime `allow_directory(recordings_dir, true)` adds both the directory
  and `<dir>/**` (`tauri-2.11.5/src/scope/fs.rs:351-361`). `HistoryManager::new` runs once in
  setup (`apps/desktop/src/lib.rs:207`), before the history page can ask for a file. Playback
  keeps working.
- **Linux does not use the asset protocol at all.** On Linux, `HistorySettings.tsx:197-200`
  reads the file through `@tauri-apps/plugin-fs` `readFile` and plays a blob URL. That path is
  governed by the `fs:scope` capability (`apps/desktop/capabilities/default.json:21-22`,
  `$APPDATA` and `$APPDATA/**/*`), which this diff does not touch. The change affects playback
  only on Windows (the phase 1 target) and macOS.
- **The read-anything hole is closed for the asset protocol.** Before serving a file, the
  handler rejects any path that `SafePathBuf` refuses (one with a `..` component,
  `tauri-2.11.5/src/protocol/asset.rs:41`). It then checks `scope.is_allowed`, which resolves a
  symlink and canonicalizes the path before matching (`src/scope/fs.rs:419-481`). An absolute
  path, a `..` traversal or a symlink planted in `recordings/` that points outside it all
  resolve outside the scope and get a 403. Any path still reachable through
  `get_audio_file_path` (for example `recordings_dir.join("/etc/passwd")`) is denied by that
  same check. The static scope's `**` was the only thing that let everything through, and it
  is gone.

### Level and sampling gaps (round 2)

- C11's behaviour proof pins the config value only. The runtime `allow_directory` call is
  proven structurally (awk/grep), and no test drives the asset handler to show a 403 outside
  the folder and a 200 inside it, because that needs an `AppHandle` and a WebView. The scope
  semantics above were checked by reading the Tauri source, not by a test. Accepted under
  `light`. The test name (`asset_protocol_reads_only_the_recordings_folder`) promises more than
  its assertions cover.
- The runtime call logs and continues on error (`warn!` at `history.rs:939`). If
  `allow_directory` ever failed in portable mode, playback would fail silently, with only that
  log line. The message carries no dictated content.

### Findings (round 2)

5. Round 1 finding 1 is resolved: C5's command now passes the filters after `--` and runs as
   written (exit 0, 2 passed).
6. The read-anything hole is still open through another channel. `fs:scope` still grants
   `readFile` over all of `$APPDATA/**/*` (`apps/desktop/capabilities/default.json:22`). That
   covers the history database, `fala.sqlite`, the `notas` folder and settings, but not
   arbitrary disk paths, and API keys live in the OS keyring (ADR-0008). Present before this
   PR, outside #1384's scope and not a check failure. Noted because the commit message's
   rationale (a future XSS reading files) applies to it at a smaller scale.
7. Pre-existing, not a regression: on Linux in portable mode, playback reads
   `<exe dir>/Data/recordings` through `readFile`, which `fs:scope` (`$APPDATA` only) does not
   cover. Untouched by this diff. Portable mode is a Windows feature in practice.
8. Sampling gap: C11 has no runtime test of the scope (see above). Non-blocking under `light`.

### Gate (round 2)

`cargo test -p fala-postproc -- rules::tests::removes_pt_br_fillers rules::tests::en_keeps_pt_only_fillers` - 2 passed, 0 failed (C5 as written)
`cargo test -p fala-postproc -- rules::tests::removes_pt_br_fillers rules::tests::en_keeps_pt_only_fillers rules::tests::keeps_sentence_capital_after_removed_filler` - 3 passed, 0 failed
`cargo test -p fala --lib -- managers::history::tests::asset_protocol_reads_only_the_recordings_folder transcription_coordinator::tests::is_busy_follows_the_stage audio_toolkit::text::tests::test_filter_leading_filler_keeps_sentence_capital audio_toolkit::text::tests::test_filter_keeps_ha_in_english` - 4 passed, 0 failed (348 filtered out)
C8 jq/grep proof exit 0 · C11 awk/grep proof exit 0 · `scripts/check-brand.sh` exit 0
