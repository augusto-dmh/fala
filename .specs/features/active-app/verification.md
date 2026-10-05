# active-app verification

**Verdict**: FAIL
**Profile**: light
**Diff range**: 008102f..8b6f831 (ef53e87, 8b6f831)
**Round**: 1 - full
**Verifier**: independent sub-agent (author != verifier). The author is the linux-inject executor sub-agent. This Verifier was dispatched fresh by the orchestrator, built none of the code, and fixed nothing.

The verdict is FAIL only because C7 is unproven. C7 is a manual Windows check (`TODO(windows)`) that cannot run on this Linux machine, and `checks.md` Handoff already lists it as open. The other 8 checks are proven at `8b6f831` with a located assertion or command output, and the review found no code defect. The rule is that any check whose Result is not PASS makes the verdict FAIL, so the verdict stays FAIL instead of softening the C7 row. Two ways to close the gate:

- (a) Run C7 on Windows, then do a scoped round 2.
- (b) The user re-scopes C7 out of this Linux gate in `checks.md`.

## Binding sources

Not run: step 1 belongs to the `ui` profile, and this feature is `light`. The plan marks no source as binding. ADR-0004 and ADR-0007 were still read for the invariants the brief named (see `## ADR and repo invariants`).

## Checks

All proofs ran at `8b6f831` (`git rev-parse HEAD`). Each cargo command used the prefix `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2` and ran one at a time. The five unit tests ran in a single invocation: `cargo test -p fala-inject -- --exact foreground::tests::exe_path_becomes_lowercase_name foreground::tests::exe_path_without_name_is_none foreground::tests::non_windows_detection_is_unsupported foreground::tests::non_windows_context_is_unknown foreground::tests::every_error_becomes_unknown_app`. It exited 0 with `5 passed; 0 failed; 0 ignored; 0 filtered out`, and each test name appears with `... ok`.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | 6 paths map to `chrome`, `code`, `ms-teams`, `foo.bar`, `notepad`, `gnome-text-editor`, one case at a time | `foreground::tests::exe_path_becomes_lowercase_name ... ok` (batch above, exit 0). Test exists at `crates/inject/src/foreground.rs:141` | `crates/inject/src/foreground.rs:159-163` - `assert_eq!(app_name_from_exe_path(path), Some(expected.to_string()), "{path}")` inside a loop over the 6 literal pairs at `:146-157`. The pairs are byte-for-byte the AC 1 inputs and outputs. | PASS |
| C2 | `""`, `C:\dir\`, `.exe` and `C:\x\.EXE` map to `None`, one case at a time | `foreground::tests::exe_path_without_name_is_none ... ok` (batch, exit 0). Test exists at `crates/inject/src/foreground.rs:168` | `crates/inject/src/foreground.rs:170` - `assert_eq!(app_name_from_exe_path(path), None, "{path:?}")`. The loop is over the 4 AC 2 literals at `:169`. | PASS |
| C3 | no file in `crates/inject` contains `GetWindowText` | `! grep -rn "GetWindowText" crates/inject` exit 0, no output | Broader search `grep -rniE "GetWindowText\|InternalGetWindowText\|WM_GETTEXT\|GetClassName\|title\|Accessibility\|UIAutomation" crates/inject` found nothing. The only Win32 imports are at `crates/inject/src/foreground.rs:71-78`: `CloseHandle`, `GetLastError`, `HANDLE`, `OpenProcess`, `QueryFullProcessImageNameW`, `PROCESS_NAME_WIN32`, `PROCESS_QUERY_LIMITED_INFORMATION`, `GetForegroundWindow`, `GetWindowThreadProcessId`. | PASS |
| C4 | outside Windows, `try_foreground_app()` returns `Err(Unsupported(reason))` with a non-empty reason | `foreground::tests::non_windows_detection_is_unsupported ... ok` (batch, exit 0). Test exists at `crates/inject/src/foreground.rs:176`, `#[cfg(not(windows))]` at `:174` | `crates/inject/src/foreground.rs:178` - `Err(InjectError::Unsupported(reason)) => assert!(!reason.trim().is_empty())`. Every other arm panics (`:179`). | PASS |
| C5 | outside Windows, `foreground_app()` returns `AppContext { app_name: None }` | `foreground::tests::non_windows_context_is_unknown ... ok` (batch, exit 0). Test exists at `crates/inject/src/foreground.rs:185` | `crates/inject/src/foreground.rs:186` - `assert_eq!(foreground_app(), AppContext { app_name: None })` | PASS |
| C6 | `Unsupported("x")`, `NoForegroundWindow` and `Os { call: "OpenProcess", code: 5 }` all map to `AppContext::default()`; `Ok(Some("code"))` maps to itself | `foreground::tests::every_error_becomes_unknown_app ... ok` (batch, exit 0). Test exists at `crates/inject/src/foreground.rs:190` | `crates/inject/src/foreground.rs:199-203` - `assert_eq!(context_or_unknown(Err(err.clone())), AppContext::default(), "{err:?}")` over the 3 literal variants at `:192-197`; `:208` - `assert_eq!(context_or_unknown(Ok(known.clone())), known)`. `foreground_app()` is exactly `context_or_unknown(try_foreground_app())` (`:15`). | PASS |
| C7 | on Windows, with Chrome focused, the example exits 0 only if `app_name == Some("chrome")` | Not runnable here: the proof is a manual run on Windows (`TODO(windows)`, AC 7, and `checks.md` Handoff says "C7 open until the manual Windows run"). The only thing run here was the Linux negative direction: `cargo run -q -p fala-inject --example foreground_app -- chrome` printed `Err(Unsupported("esta plataforma não expõe ..."))` and `esperava "chrome", detectou None`, exit 1. | The exit-code logic is at `crates/inject/examples/foreground_app.rs:19-24`: `if detected.as_deref() == Some(expected.as_str()) { ExitCode::SUCCESS } else { ... ExitCode::FAILURE }`. C9 shows the Windows path (`crates/inject/src/foreground.rs:85-115`) type-checks and lints for msvc. Nothing shows that it returns `chrome` at runtime. | UNPROVEN (by design - TODO(windows) manual run on Windows 11; cannot execute on Linux) |
| C8 | no-tauri script exits 0; the linux-gnu normal tree has no `windows-sys` or `tauri`; the windows-msvc tree has `windows-sys v0.61` | The full proof chain exited 0 (`C8 exit=0`). `scripts/check-no-tauri-in-crates.sh` printed `ok: no tauri in crates/` | `cargo tree -p fala-inject -e normal --target x86_64-unknown-linux-gnu` lists only `fala-core` (serde, thiserror), `log v0.4.29` and `thiserror v2.0.18`. The `--target x86_64-pc-windows-msvc` tree adds `└── windows-sys v0.61.2` / `└── windows-link v0.2.1`. The manifest matches door 2 at `crates/inject/Cargo.toml:12-20`. | PASS |
| C9 | cross clippy `--all-targets -D warnings` for `x86_64-pc-windows-msvc` exits 0, compiling the `cfg(windows)` code and the example | `RUSTC_BOOTSTRAP=1 cargo clippy -Zbuild-std=std,panic_abort --target x86_64-pc-windows-msvc -p fala-inject --all-targets -- -D warnings` printed `Checking fala-inject v0.1.0 ... Finished`, exit 0 | The code it covers is the `#[cfg(windows)] mod platform` at `crates/inject/src/foreground.rs:69-134`. It can fail, as shown in a scratch worktree (see `## C9 discrimination`): an `unwrap()` injected at `crates/inject/src/foreground.rs:114` inside the Windows module failed this exact command (exit 101, `error: used unwrap() on an Option value --> crates/inject/src/foreground.rs:114:22`). Linux clippy on the same mutant exited 0, so only the msvc proof reaches that code. | PASS |

## C9 discrimination

The brief required this. Fault injection belongs to `standard`, so this is the one demonstration requested for C9, not a full step 4.

| Mutation | Location | Killed |
| --- | --- | --- |
| add `let _first = buf.first().unwrap();` in the `cfg(windows)` `foreground_exe_path` | `crates/inject/src/foreground.rs:114` (scratch worktree at `8b6f831`) | yes - the C9 command exited 101 on the lib and lib-test targets. The same mutant passed Linux `cargo clippy -p fala-inject --all-targets -- -D warnings` (exit 0), which confirms the msvc run is what compiles the Windows code |

Isolation: `git worktree add --detach <scratchpad>/c9-fault HEAD`, mutate, run, then `git worktree remove --force` and `git worktree prune`. The real tree's `git status --porcelain` was empty before and after, so the baseline matched. No `git stash` was used.

## ADR and repo invariants

- **ADR-0004 (only the app name reaches the LLM).** No window-title or content API is called. C3 plus the broader search show no `GetWindowText*`, `WM_GETTEXT`, `GetClassName` or UI Automation. The Windows path reads only the foreground HWND, then the owner PID, then the process image path (`crates/inject/src/foreground.rs:87`, `:93`, `:99`, `:107-108`). Door 1 reduces that path to the file stem (`:38-47`). The module doc says why (`:3-4`).
- **ADR-0007 / AGENTS.md (`cfg` only in platform crates).** The diff adds `cfg(windows)` and `cfg(not(windows))` only in `crates/inject`: `Cargo.toml:15`, `src/foreground.rs:53`, `:69`, `:174`, `:183`. `fala-inject` is one of the platform crates AGENTS.md names (`hotkey`, `audio`, `inject`).
- **No tauri in crates.** `scripts/check-no-tauri-in-crates.sh` reports `ok: no tauri in crates/`, exit 0 (C8).
- **No `unwrap`/`expect` outside tests.** `cargo clippy -p fala-inject --all-targets -- -D warnings` exited 0 on Linux, and so did the msvc cross clippy (C9). The workspace lints deny both (`[lints] workspace = true`, `crates/inject/Cargo.toml:22-23`). The only `unwrap`-like call in non-test code is `Option::unwrap_or(path)` at `crates/inject/src/foreground.rs:39`, which cannot panic.
- **Workspace `Cargo.toml` is unchanged.** `git diff --quiet 008102f..HEAD -- Cargo.toml` exited 0. `Cargo.lock` is additive only: 3 dependency lines on `fala-inject` (`log`, `thiserror 2.0.18`, `windows-sys 0.61.2`), and no package versions moved.
- **Formatting.** `cargo fmt -p fala-inject -- --check` exited 0.
- **Logging.** The failure reason is logged at `debug` (`crates/inject/src/foreground.rs:27`) and contains no dictated content.

## Level, sampling and swept

- **Level.** This feature is a library inside the workspace (plan `Surface`: none), so unit tests are the right level for C1, C2, C4, C5 and C6. C6 tests the private `context_or_unknown`, not `foreground_app()` itself. That is acceptable because `foreground_app()` is that one call (`foreground.rs:15`), and C5 exercises the public path on Linux.
- **Sampling.** AC 1 names 6 paths and C1 proves all 6. AC 2 names 4 and C2 proves 4. AC 6 names 3 variants and C6 proves 3. Nothing is under-sampled.
- **Swept existing.** No `Swept` row in `checks.md` resolves to existing code. Each row cites a check (C1, C2, C4, C6) or `n/a`, so there was nothing to re-read against the code.

## Findings (non-blocking, for the record)

1. **C7 is the only open obligation, and it is open by design.** On Linux, nothing shows that the Windows path returns `chrome` at runtime. That needs the manual run in AC 7. The code's `TODO(windows)` note also asks for an elevated app and a UWP app (`crates/inject/src/foreground.rs:67-68`). C9 proves only that this path compiles and lints.
2. **C4 and C5 are `#[cfg(not(windows))]`** (`foreground.rs:174`, `:183`), so the Windows CI job does not compile them and has no replacement. `checks.md` already says this.
3. **Windows-only edge.** If `GetWindowThreadProcessId` writes `pid = 0`, `last_error` is read right away (`foreground.rs:94-95`). `OpenProcess` failure is handled before the handle is used (`:100-101`). The plan's assumption that `PROCESS_QUERY_LIMITED_INFORMATION` works across integrity levels can only be verified on Windows (`TODO(windows)`).

## Gate

`cargo test -p fala-inject -- --exact <5 names>`: 5 passed, 0 failed. `! grep -rn GetWindowText crates/inject`: exit 0. C8 chain: exit 0. C9 msvc cross clippy: exit 0. Linux `cargo clippy -p fala-inject --all-targets -- -D warnings`: exit 0. C7: not run (manual, Windows).
