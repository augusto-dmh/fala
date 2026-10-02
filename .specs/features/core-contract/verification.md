# core-contract verification

**Verdict**: PASS
**Profile**: light
**Diff range**: 881e77c..91f4e0d (`origin/main..HEAD`, one commit)
**Round**: 1 - full
**Verifier**: independent sub-agent (author != verifier) - a fresh Verifier context, distinct from the Lux session that authored the plan, the checks and commit 91f4e0d

## Binding sources

n/a under `light` (step 1 runs only under `ui`). The plan has no `Observable` surface and no route in `Surface`. Its `Sources` name `fala-research/HANDOFF-fase-1-linux.md` and `ARCHITECTURE.md` § Invariantes. Neither is marked binding, so nothing was compared.

## Checks

The proofs ran at HEAD 91f4e0d with `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=3`, in a single invocation that passed all 11 test paths:
`cargo test -p fala-core -- --exact language::tests::parses_known_tags language::tests::rejects_unknown_tags language::tests::serializes_as_bcp47_tag language::tests::tag_matches_serialized_form audio::tests::dictation_audio_is_not_serializable audio::tests::one_second_of_samples audio::tests::empty_audio dictation::tests::editor_serialized_values dictation::tests::dictation_round_trips_json dictation::tests::unedited_keeps_raw_text dictionary::tests::normalizes_terms`
It exited 0 with `11 passed; 0 failed; 0 filtered out`, and each of the 11 names appears individually as `... ok`. The unfiltered `cargo test -p fala-core` also gave 11 passed, 0 failed. All 11 tests were added in this diff, since the four `.rs` files are new in 91f4e0d.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | `pt-BR`, `PT-br`, `pt` -> `PtBr`; `en` -> `En` | `language::tests::parses_known_tags ... ok` | `crates/core/src/language.rs:52-55` - `assert_eq!("pt-BR".parse::<Language>(), Ok(Language::PtBr))`, same for `"PT-br"`, `"pt"`, and `"en"` -> `Ok(Language::En)` | PASS |
| C2 | `es`, `pt-PT`, `""` -> `UnknownLanguage(input)` | `language::tests::rejects_unknown_tags ... ok` | `crates/core/src/language.rs:60-63` - `for input in ["es", "pt-PT", ""]` / `assert_eq!(input.parse::<Language>(), Err(CoreError::UnknownLanguage(input.to_string())))` | PASS |
| C3 | JSON `"pt-BR"` / `"en"`, both directions | `language::tests::serializes_as_bcp47_tag ... ok` | `crates/core/src/language.rs:70-71` - `assert_eq!(serde_json::to_string(&Language::PtBr).unwrap(), "\"pt-BR\"")`, `... En ... "\"en\""`; `:72-79` - `from_str::<Language>("\"pt-BR\"")` == `PtBr`, `("\"en\"")` == `En` | PASS |
| C4 | `tag()` is `"pt-BR"` / `"en"` and equals the serialized form | `language::tests::tag_matches_serialized_form ... ok` | `crates/core/src/language.rs:84-85` - `assert_eq!(Language::PtBr.tag(), "pt-BR")`, `assert_eq!(Language::En.tag(), "en")`; `:88` - `assert_eq!(json.trim_matches('"'), lang.tag())` | PASS |
| C5 | `DictationAudio` is not `Serialize` (compile-time) | `audio::tests::dictation_audio_is_not_serializable ... ok` (the test build compiled) | `crates/core/src/audio.rs:43` - `static_assertions::assert_not_impl_any!(DictationAudio: serde::Serialize);`; `audio.rs:7` derives only `Debug, Clone, PartialEq, Default` | PASS |
| C6 | 16 000 samples -> `Duration::from_secs(1)`; `SAMPLE_RATE_HZ == 16_000` | `audio::tests::one_second_of_samples ... ok` | `crates/core/src/audio.rs:49-50` - `assert_eq!(audio.duration(), Duration::from_secs(1))`, `assert_eq!(DictationAudio::SAMPLE_RATE_HZ, 16_000)` | PASS |
| C7 | empty -> `Duration::ZERO`, `is_empty()` | `audio::tests::empty_audio ... ok` | `crates/core/src/audio.rs:56-57` - `assert_eq!(audio.duration(), Duration::ZERO)`, `assert!(audio.is_empty())` | PASS |
| C8 | `Editor` <-> `"none"`, `"rules"`, `"llm"` | `dictation::tests::editor_serialized_values ... ok` | `crates/core/src/dictation.rs:57-63` - table `(Editor::None, "\"none\"")`, `(Editor::Rules, "\"rules\"")`, `(Editor::Llm, "\"llm\"")`; `assert_eq!(serde_json::to_string(&editor).unwrap(), json)` and `assert_eq!(serde_json::from_str::<Editor>(json).unwrap(), editor)` | PASS |
| C9 | the stated `Dictation` round-trips JSON to an equal value | `dictation::tests::dictation_round_trips_json ... ok` | `crates/core/src/dictation.rs:71-77` - fixture `"oi tudo bem"`, `Language::PtBr`, `"Oi, tudo bem?"`, `Editor::Llm`, `Some("Slack")`; `:81` - `assert_eq!(serde_json::from_str::<Dictation>(&json).unwrap(), dictation)` | PASS |
| C10 | `unedited` -> `final_text == raw.text`, `editor == None` | `dictation::tests::unedited_keeps_raw_text ... ok` | `crates/core/src/dictation.rs:91-92` - `assert_eq!(dictation.final_text, transcript.text)`, `assert_eq!(dictation.editor, Editor::None)` | PASS |
| C11 | `[" Fala ", "fala", "", "ADR", "  "]` -> exactly `["Fala", "ADR"]` | `dictionary::tests::normalizes_terms ... ok` | `crates/core/src/dictionary.rs:40-41` - `Dictionary::new([" Fala ", "fala", "", "ADR", "  "])` / `assert_eq!(dictionary.terms(), ["Fala", "ADR"])` | PASS |
| C12 | no tauri, no `cfg(target_os)` / `cfg(windows)` in `crates/core` | `scripts/check-no-tauri-in-crates.sh` exit 0 (`ok: no tauri in crates/`); the proof's `rg -n` over `crates/core` for `cfg(target_os` or `cfg(windows` exit 1, no output (the exact command line is in checks.md C12); corroborated by an equivalent `grep -rnE` exit 1 | `scripts/check-no-tauri-in-crates.sh:7` - greps `crates/*/Cargo.toml` for a direct `tauri` dependency; `:14` - `cargo tree -p "$pkg" --target all -e normal,build` piped to `grep -E '^tauri'`; `crates/core/Cargo.toml:10-16` lists only `serde`, `thiserror` (deps) and `serde_json`, `static_assertions` (dev-deps) | PASS |

## Level and sampling

- C1-C11 are value-level claims about library types, and unit tests next to the code are the right level for them. `Surface` is `None`, so no claim names a boundary that a unit proof sits below.
- C2 samples 3 inputs for an AC that reads "any other string". The check itself claims only those three, and the implementation's closed `_ =>` arm at `crates/core/src/language.rs:41` makes the sample representative. This is not a gap.
- C5 is a compile-time proof. The test body is a const assertion, so the `ok` line only shows that the test target compiled, and that is the claim. It guards the test build only: a non-test `cargo build` would not fail on a future `Serialize`. The check's wording ("the crate's test build fails") matches that, so this is not a gap.
- C6 asserts exact `Duration` equality, which matches AC 6's "exatamente".

## Swept existing

No `Swept` row resolves to an existing constraint in the code. `validation` and `failure modes` point to C1, C2 and C11, which are proven above. The other seven rows are `n/a` with reasons, which is approved policy. There was nothing to re-read against the code.

## Project invariants touched

- No tauri in `crates/`: `scripts/check-no-tauri-in-crates.sh` exit 0.
- No `cfg(target_os)` / `cfg(windows)` in `crates/core`: `rg` and `grep` both exit 1 with no output. The only `tauri` substring in `crates/core` is the doc comment at `crates/core/src/lib.rs:6`.
- No `unwrap`/`expect` outside tests: `cargo clippy -p fala-core --all-targets -- -D warnings` exit 0 under `[workspace.lints.clippy] unwrap_used = "deny"`, `expect_used = "deny"`. All 9 `unwrap()` calls in `crates/core/src` are inside `#[cfg(test)] mod tests` (`language.rs:70,71,73,77,87`, `dictation.rs:62,63,80,81`). `clippy.toml:2-3` sets `allow-unwrap-in-tests` and `allow-expect-in-tests`.
- Formatting: `cargo fmt --all --check` exit 0.
- No crate was created, renamed or removed, so the `ARCHITECTURE.md` Code Map does not need an update.
- Door 4: `serde`, `serde_json` and `static_assertions` are in `[workspace.dependencies]` (`Cargo.toml:24-26`). `fala-core` uses `serde` as a normal dependency and the other two only as `[dev-dependencies]` (`crates/core/Cargo.toml:11-16`). The plan's literal shape names only `serde` for the workspace table, but listing the dev-only crates there too is required for `.workspace = true` and does not contradict the "only as dev-dependencies" constraint on `fala-core`.

## Findings (non-blocking)

1. **C12's proof command can pass without running.** `! rg ...` treats a missing `rg` as success. On this machine `rg` exists only as a shell function of the Claude Code shell, and `which rg` exits 1. Run under plain `bash -c`, the C12 proof printed `rg: comando não encontrado` and still exited 0, because the 127 was negated. The claim still holds: the interactive `rg` exited 1 with no matches, and an independent `grep -rnE` agrees. But the proof as written would stay green in CI or a fresh shell whatever the code says. This is a precision gap in the check's proof. `! grep -rnE 'cfg\((target_os|windows)' crates/core`, or a guard on `command -v rg`, would make the command able to fail.
2. **`Dictionary` derives `Deserialize`** (`crates/core/src/dictionary.rs:5`). A `Dictionary` read from JSON skips the normalization in `Dictionary::new`, so it can hold terms with surrounding spaces, empty terms or case duplicates. No check or AC covers deserialization, so this does not affect the verdict. It matters for C (storage), which will read dictionaries back.
3. **`Cargo.lock` re-resolves `windows-sys` outside `fala-core`.** Besides the 4 new `fala-core` dependency lines, the commit moves about 10 third-party packages (for example `arboard`, `dirs-sys`, `errno`, `rustix`, `tempfile`, `rustls-platform-verifier`) from `windows-sys` 0.45/0.48/0.59 to 0.60.2/0.61.2. This is plausibly Cargo unifying range requirements when the graph changed. It only affects Windows targets (TODO(windows)), and no check covers it.

## Coverage

Not recomputed: under `light`, step 3's Coverage recompute does not run. The table in `checks.md` was read and not re-derived.

## Test policy rows

`checks.md` has no `Test policy` section (`grep -n 'Test policy'` exit 1), so there is nothing to judge.

## Faults injected

None: fault injection runs only under `standard` and `ui`, and this feature was approved under `light`.

## Gate

`cargo test -p fala-core` at 91f4e0d - 11 passed, 0 failed, 0 ignored. `cargo clippy -p fala-core --all-targets -- -D warnings` exit 0. `cargo fmt --all --check` exit 0. `scripts/check-no-tauri-in-crates.sh` exit 0.
