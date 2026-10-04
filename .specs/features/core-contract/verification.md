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

---

## Round 2

**Verdict**: PASS
**Profile**: light
**Diff range**: 91f4e0d..ba3b6df (one commit, `fix(core): normalize dictionary terms when deserializing`)
**Round**: 2 - scoped
**Verifier**: independent sub-agent (author != verifier) - a fresh Verifier context that did not author ba3b6df, the plan or the checks

Scope: the diff 91f4e0d..ba3b6df touches `crates/core/src/dictionary.rs` (+24), `checks.md` (C12 proof `rg` -> `grep`, new C13, coverage rows, Handoff) and `verification.md` (round 1 report committed). Round 1 had no non-PASS verdict, so the scope is C12 (proof changed), C13 (new) and citations in `dictionary.rs`. Everything else carries forward from 91f4e0d and says so below. Proofs for all 13 checks re-ran at ba3b6df. The tree was clean (`git status --porcelain` empty) before and after this round.

### Binding sources (carried from 91f4e0d)

n/a under `light`. ba3b6df touches no interface the plan marks binding.

### Proofs re-run (verified at ba3b6df)

One invocation, with `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=3`:
`cargo test -p fala-core -- --exact language::tests::parses_known_tags language::tests::rejects_unknown_tags language::tests::serializes_as_bcp47_tag language::tests::tag_matches_serialized_form audio::tests::dictation_audio_is_not_serializable audio::tests::one_second_of_samples audio::tests::empty_audio dictation::tests::editor_serialized_values dictation::tests::dictation_round_trips_json dictation::tests::unedited_keeps_raw_text dictionary::tests::normalizes_terms dictionary::tests::deserializing_normalizes_terms`
Exit 0, `12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`. Each of the 12 names printed its own `... ok` line: `audio::tests::dictation_audio_is_not_serializable`, `dictation::tests::dictation_round_trips_json`, `audio::tests::one_second_of_samples`, `dictation::tests::editor_serialized_values`, `audio::tests::empty_audio`, `dictation::tests::unedited_keeps_raw_text`, `language::tests::parses_known_tags`, `language::tests::tag_matches_serialized_form`, `language::tests::rejects_unknown_tags`, `dictionary::tests::deserializing_normalizes_terms`, `language::tests::serializes_as_bcp47_tag`, `dictionary::tests::normalizes_terms`. C13's test is new in this diff (`grep -rn deserializing_normalizes_terms crates/` -> only `crates/core/src/dictionary.rs:58`).

### Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | `pt-BR`, `PT-br`, `pt` -> `PtBr`; `en` -> `En` | `language::tests::parses_known_tags ... ok` at ba3b6df | `crates/core/src/language.rs:52-55` (citation carried from 91f4e0d; `language.rs` not in the diff) | PASS |
| C2 | `es`, `pt-PT`, `""` -> `UnknownLanguage(input)` | `language::tests::rejects_unknown_tags ... ok` at ba3b6df | `crates/core/src/language.rs:60-63` (carried from 91f4e0d) | PASS |
| C3 | JSON `"pt-BR"` / `"en"`, both directions | `language::tests::serializes_as_bcp47_tag ... ok` at ba3b6df | `crates/core/src/language.rs:70-79` (carried from 91f4e0d) | PASS |
| C4 | `tag()` equals the serialized form | `language::tests::tag_matches_serialized_form ... ok` at ba3b6df | `crates/core/src/language.rs:84-88` (carried from 91f4e0d) | PASS |
| C5 | `DictationAudio` is not `Serialize` | `audio::tests::dictation_audio_is_not_serializable ... ok` at ba3b6df | `crates/core/src/audio.rs:43` (carried from 91f4e0d; `audio.rs` not in the diff) | PASS |
| C6 | 16 000 samples -> 1 s; `SAMPLE_RATE_HZ == 16_000` | `audio::tests::one_second_of_samples ... ok` at ba3b6df | `crates/core/src/audio.rs:49-50` (carried from 91f4e0d) | PASS |
| C7 | empty -> `Duration::ZERO`, `is_empty()` | `audio::tests::empty_audio ... ok` at ba3b6df | `crates/core/src/audio.rs:56-57` (carried from 91f4e0d) | PASS |
| C8 | `Editor` <-> `"none"`, `"rules"`, `"llm"` | `dictation::tests::editor_serialized_values ... ok` at ba3b6df | `crates/core/src/dictation.rs:57-63` (carried from 91f4e0d; `dictation.rs` not in the diff) | PASS |
| C9 | the stated `Dictation` round-trips JSON | `dictation::tests::dictation_round_trips_json ... ok` at ba3b6df | `crates/core/src/dictation.rs:71-81` (carried from 91f4e0d) | PASS |
| C10 | `unedited` keeps raw text, editor `None` | `dictation::tests::unedited_keeps_raw_text ... ok` at ba3b6df | `crates/core/src/dictation.rs:91-92` (carried from 91f4e0d) | PASS |
| C11 | `[" Fala ", "fala", "", "ADR", "  "]` -> exactly `["Fala", "ADR"]` | `dictionary::tests::normalizes_terms ... ok` at ba3b6df | `crates/core/src/dictionary.rs:53-54` (refreshed; the file is in the diff, lines unchanged) - `Dictionary::new([" Fala ", "fala", "", "ADR", "  "])` / `assert_eq!(dictionary.terms(), ["Fala", "ADR"])` | PASS |
| C12 | `check-no-tauri-in-crates.sh` exit 0 and no `cfg(target_os` / `cfg(windows` in `crates/core` | Under `env -i ... bash --noprofile --norc -c`: `command -v grep` -> `/usr/bin/grep` (GNU grep 3.11, `type -t grep` = `file`, not a shell function); the exact C12 proof line from `checks.md` (the script, then the negated `grep -rnE` over `crates/core` for `cfg(target_os` or `cfg(windows`) printed `ok: no tauri in crates/`, no grep output, exit 0. Positive control: the same negated `grep -rnE` over a scratch `crates/core/src/x.rs` holding `#[cfg(target_os = "linux")]` and `#[cfg(windows)]` printed both lines and exited 1, so the proof can fail | `scripts/check-no-tauri-in-crates.sh:7` (greps `crates/*/Cargo.toml` for a direct `tauri` dependency) and `:14` (`cargo tree -p "$pkg" --target all -e normal,build` piped to `grep -E '^tauri'`); the proof's alternation pattern matches both spellings named in the claim, shown by the control | PASS |
| C13 | deserializing `{"terms":[" Fala ","fala","","ADR","  "]}` -> `terms() == ["Fala", "ADR"]`, and serializes back to `{"terms":["Fala","ADR"]}` | `dictionary::tests::deserializing_normalizes_terms ... ok` at ba3b6df | `crates/core/src/dictionary.rs:59-61` - `let json = r#"{"terms":[" Fala ","fala","","ADR","  "]}"#;` / `serde_json::from_str(json).unwrap()` / `assert_eq!(dictionary.terms(), ["Fala", "ADR"])`; `:62-65` - `assert_eq!(serde_json::to_string(&dictionary).unwrap(), r#"{"terms":["Fala","ADR"]}"#)`. Mechanism: `dictionary.rs:7` `#[serde(from = "DictionaryTerms")]` and `:41-44` `impl From<DictionaryTerms> for Dictionary { ... Dictionary::new(raw.terms) }`, so deserialization goes through the same constructor C11 proves | PASS |

### Level and sampling (verified at ba3b6df for C12, C13; rest carried from 91f4e0d)

- C13 is a value-level claim about serde on a library type, so a unit test next to the code is the right level. Its fixture is the same five inputs as C11, so it exercises all four normalization rules (trim, drop empty, case-insensitive dedup, first spelling and order kept). That matches the two rows the `checks.md` Coverage table attributes to C13.
- C12: the proof now uses `grep`, which is a real binary at `/usr/bin/grep` under a non-interactive shell, and the positive control shows `! grep` turns into exit 1 when a match exists. That closes round 1 finding 1.

### Swept existing (carried from 91f4e0d)

No `Swept` row resolves to an existing constraint. ba3b6df did not change the `Swept` section.

### Project invariants touched (verified at ba3b6df)

- `cargo clippy -p fala-core --all-targets -- -D warnings` exit 0. The two new `unwrap()` calls are at `crates/core/src/dictionary.rs:60` and `:63`, both inside `#[cfg(test)] mod tests` (starts at `:47`).
- `cargo fmt --all --check` exit 0.
- `scripts/check-no-tauri-in-crates.sh` exit 0. ba3b6df adds no dependency and does not touch `Cargo.toml` or `Cargo.lock`.
- No crate created, renamed or removed.

### Round 1 findings, status at ba3b6df

1. C12's proof could pass without running: **resolved**. The proof uses `grep`, which is present, and the control shows it can fail.
2. `Dictionary` deserialized without normalization: **resolved** by `#[serde(from = "DictionaryTerms")]` and proven by C13.
3. `Cargo.lock` re-resolves `windows-sys`: unchanged. ba3b6df does not touch `Cargo.lock`, and the finding stays non-blocking (TODO(windows)).

### Findings (non-blocking)

1. **C12's claim text still names `rg`.** The claim reads "`rg -n 'cfg\(target_os|cfg\(windows' crates/core` prints nothing (exit 1)" while the proof runs `grep -rnE 'cfg\((target_os|windows)'`. Both patterns match the same two spellings, so the obligation is unchanged, as the Handoff states. But the claim names a command that is not what gets run. This is wording drift in `checks.md`, not a gap in the proof.
2. **`! grep` still passes vacuously if `grep` is absent.** Under `env -i PATH=/nonexistent bash -c "! grep ..."` it printed `grep: command not found` and exited 0. `grep` is part of the base system and `check-no-tauri-in-crates.sh` already depends on it, so the risk is far smaller than with `rg`. A `command -v grep >/dev/null &&` guard would remove the risk completely.

### Coverage

Not recomputed: `light` profile. The two coverage rows ba3b6df changed (`Dictionary` normalization rules, `Dictionary` construction paths) were read against the C13 fixture above, and they match.

### Test policy rows

`checks.md` has no `Test policy` section, so there is nothing to judge.

### Faults injected

None: fault injection runs only under `standard` and `ui`.

### Gate

`cargo test -p fala-core` at ba3b6df - 12 passed, 0 failed, 0 ignored (doc-tests: 0). `cargo clippy -p fala-core --all-targets -- -D warnings` exit 0. `cargo fmt --all --check` exit 0. C12 proof under plain `bash -c` exit 0.
