# dictionary verification

**Verdict**: PASS
**Profile**: light
**Diff range**: eb0482c..1f63c57 (8f1dca4 desktop slice, 1f63c57 UI slice); HEAD `1f63c57`. `main` = `eb0482c`, so `git diff main` is the same range.
**Round**: 3 - scoped (round 2 - scoped, at eb0482c..8511599; round 1 - full, at eb0482c..8ddfe8f)
**Verifier**: independent sub-agent (author != verifier). The author is the dictionary executor. This Verifier was dispatched fresh by the orchestrator on the Windows 11 machine. It built none of the feature code and fixed nothing. The only file it wrote in the repo is this report. To check the fuzzy behaviour it compiled a throwaway probe crate in its scratchpad (outside the repo, `CARGO_TARGET_DIR=C:\f\dictprobe`). The probe includes `apps/desktop/src/audio_toolkit/text.rs` by `#[path]` and depends on `fala-core`, `fala-postproc` and `fala-secrets` by path.

## Round 3 - scoped

The stack was rewritten again, this time for round-2 Findings 5, 8 and 9. `git diff 8511599 1f63c57` touches 2 files (+4 -3). `git range-diff` shows the desktop slice `02ccdfb` became `8f1dca4` (`!`) and the UI slice `8511599` became `1f63c57` (`=`).

- `apps/desktop/src/llm_auto.rs:176-179` (test module only): `SIXTEEN` and `SIXTEEN_RULES` are back to the `eb0482c` text (`... time inteiro hoje`). `git show eb0482c:apps/desktop/src/llm_auto.rs` has the same two strings. The rules text has 16 words again, so it matches the doc comment at `:175` and C1's "16 palavras". **Finding 5 is fixed.**
- `plan.md:53`: the note now reads `` (`a charge bee` → `ChargeBee`, `o augusto` → `Augusto`) ``, with the backticks and the `a` restored. **Finding 9 is fixed.** `plan.md:54` is a new bullet that records Finding 8 (with a filler next to a term, the deferred path can differ from the no-LLM pipeline) as an accepted limit of the parity claim. **Finding 8 is accepted in the plan.** That matches its non-blocking classification.
- Neither changed file contains a control character (a byte count after stripping printable and UTF-8 bytes gives 0).

Runs at `1f63c57` (`CARGO_TARGET_DIR=C:\f\dict`, one at a time):

| Command | Result |
| --- | --- |
| `cargo test -p fala --lib -- --exact llm_auto::tests::dictionary_reaches_the_llm_prompt` (C1, now on the 16-word constant) | exit 0; 1 passed, 0 failed, 358 filtered out |
| `cargo test -p fala --lib` | exit 0; 359 passed, 0 failed, 0 ignored |
| `cargo fmt --all -- --check` | exit 0 |
| clippy | not re-run: the only Rust change is two string literals inside `#[cfg(test)]`; round 2 was exit 0 |

The 16-word constant is the exact threshold boundary (more than 15 words). Every test that uses it still passes in the 359 run: C1, C2, and the F7a tests `llm_text_is_pasted_and_recorded_as_llm`, `unknown_app_still_uses_llm` and `slow_failed_or_invalid_llm_keeps_rules_text`. Each of them expects the LLM to be asked.

**Verdict for round 3: PASS.** Nothing new found. Findings 2, 6, 7 and 8 remain accepted or noted, and none of them blocks.

## Round 2 - scoped

The stack was rewritten to act on round-1 Findings 1-5. `git range-diff eb0482c..8ddfe8f eb0482c..8511599` shows:

- Desktop slice `700dece` became `02ccdfb` (`!`). Only `apps/desktop/src/llm_auto.rs` changed in code: the new `format` branch, the C3 test rewritten to the named input, and the new C13 and C14 tests. `plan.md` changed AC 3, AC 5, AC 6, Flow 5, door 2, the Impact row, a new "inherited limit" note and the Assumptions row. `checks.md` changed C3, added C13 and C14, the Coverage row, the Swept "settings concurrency" line (Finding 2) and the observability line (Finding 4).
- UI slice `8ddfe8f` became `8511599` (`=`). It touches only `src/` (3 files, no Rust), so the cargo results at HEAD also stand for `02ccdfb` alone. The frontend is unchanged since round 1, so C10 stands as recorded.

Round 2 re-ran C3, C6, C13, C14 and the C12 gate at `8511599`. It read the new `format` code and re-ran the probe against a copy of the new logic.

**Verdict for round 2: PASS.** Every automated check (C1-C10, C12-C14) is proven. Finding 1 is fixed, Finding 3 is fixed, and Findings 2 and 4 are acted on. Finding 5 stands: the coordinator's recount is wrong, see below. There is one new non-blocking finding (8).

### Round-2 runs

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C3 (rewritten) | `["ChargeBee"]`: `a charge bee mandou ...` becomes `A ChargeBee mandou ...` with the LLM off and with no key. Configured but not asked (`keepassxc`, 15 words): equal to the no-LLM pipeline and contains `ChargeBee mandou`. 0 requests (AC 3) | `llm_auto::tests::rules_apply_the_dictionary_without_llm ... ok` (round-2 batch) | `llm_auto.rs:329-332` now use the named input `a charge bee ...` (16 and 15 words after the rules join `charge bee`). `:341-346` assert `starts_with("A ChargeBee mandou")` and `!llm_produced` for off and keyless. `:358-360` assert `== without_llm(..)`, `contains("ChargeBee mandou")` and `!llm_produced` for the two configured cases. `:362` asserts `requests() == 0`. The round-1 failure (`CHARGEBEE ...`) would now fail at `:358` and `:359`. | PASS |
| C6 | configured, `["Augusto"]`: `agusto` becomes `Augusto` at 4 words, in `keepassxc`, and on HTTP 500 (AC 5) | `...deferred_fuzzy_fixes_text_the_llm_did_not_write ... ok` (round-2 batch) | Unchanged test, now at `llm_auto.rs:434-451`: `:442` `O Augusto mandou isso`, `:444-445` `keepassxc` with 0 requests, `:449-451` HTTP 500 with `!llm_produced` and 1 request. | PASS |
| C13 (new) | configured, `["ChargeBee", "Augusto"]`: `a charge bee mandou isso`, `o augusto mandou isso` and `o agusto mandou isso` equal the no-LLM pipeline, with no `CHARGEBEE` or `AUGUSTO`. 0 requests (AC 5) | `...deferred_fuzzy_matches_the_pipeline_without_llm ... ok` (round-2 batch) | `llm_auto.rs:366-381`. The oracle `without_llm` (`:313-323`) runs `apply_custom_words` and then the keyless formatter, which applies the rules with the dictionary and has no deferred fuzzy. That is the no-LLM order. The parity half shares its algorithm with the implementation. The independent half is `:378-379` (`!contains("CHARGEBEE")`, `!contains("AUGUSTO")`). Either the round-1 order or a missing deferred fuzzy (`o agusto` would stay) makes the test fail. | PASS |
| C14 (new) | `formatter_at(settings, false, ..)` with the LLM configured: 0 requests, `agusto` becomes `Augusto` (AC 5, RulesOnly retry) | `...rules_only_retry_keeps_the_deferred_fuzzy ... ok` (round-2 batch) | `llm_auto.rs:385-405`: a 16-word text with `use_llm = false` and `AppContext::default()`. `:399-403` assert `starts_with("O Augusto acho")`, `:404` `!llm_produced`, `:405` `requests() == 0`. Restoring F7a's `llm_enabled &= use_llm` would make `Fuzzy::deferred` `None` and fail `:399`. This closes round-1 Finding 3. | PASS |
| C12 | fmt, clippy, `cargo test -p fala --lib`, `cargo test -p fala-postproc`, `check-brand.sh` exit 0 | `cargo fmt --all -- --check` exit 0. `cargo clippy --workspace --all-targets -- -D warnings` exit 0. `cargo test -p fala --lib` exit 0. `cargo test -p fala-postproc` exit 0. `check-brand.sh` exit 0 | Clippy printed only the two build-script notes. `fala` lib: `359 passed; 0 failed; 0 ignored` (round 1 had 357, plus C13 and C14). `fala-postproc`: unit 8, `fake_gemini` 14, doc 0. Brand: `ok: no Handy branding outside the allowlist`. | PASS |

The round-2 batch was `cargo test -p fala --lib -- --exact llm_auto::tests::rules_apply_the_dictionary_without_llm llm_auto::tests::deferred_fuzzy_fixes_text_the_llm_did_not_write llm_auto::tests::deferred_fuzzy_matches_the_pipeline_without_llm llm_auto::tests::rules_only_retry_keeps_the_deferred_fuzzy`. It exited 0 with `4 passed; 0 failed; 0 ignored; 355 filtered out`. C1, C2, C4, C5 and C7-C9 sit in code that round 2 did not change. They passed again inside the full `359 passed` run.

### What was read in the new `format` (`apps/desktop/src/llm_auto.rs:103-141`)

- When `fuzzy` is `Some` and the editor is not `Llm` (`:123-135`), it runs `Rules.format(&fuzzy.apply(dictation.raw.text.clone()), &ctx)`.
  - `ctx` is built from the same `dictation.app`, `formatter.dictionary` and `dictation.raw.language` that `Postprocessor::process` used for its own rules pass (`crates/postproc/src/lib.rs:157-166`).
  - `dictation.raw.text` is the formatter's input: the TM output without the fuzzy, after OpenCC. So the second pass is exactly "fuzzy on the ASR text, then the rules", the pre-F8 order.
- The `.ok()` fallback (`:136`, back to the first rules text) cannot fire: `Rules::format` has no `Err` path (`crates/postproc/src/rules.rs` contains no `Err(`).
- The LLM-produced path and the not-configured path are unchanged. With `fuzzy = None`, `refixed` is `None` and `final_text` is the rules or LLM text, as in round 1.
- The word-count threshold is still taken on the first rules pass, before the fuzzy. That matches the plan's Assumptions row.
- **Probe (mirror of the new `format`, configured, `notepad`, `["ChargeBee", "Augusto"]`).** It compares against the no-LLM pipeline (fuzzy, then the rules with the dictionary). All 7 inputs are equal, and none has an all-caps term. The inputs include the capitalized ASR forms that the C13 test does not cover:
  - `A charge bee mandou o relatório.` gives `ChargeBee mandou o relatório.`
  - `O Augusto mandou isso.` gives `Augusto mandou isso.`
  - `a charge bee mandou ... da empresa` (15 words) gives `ChargeBee mandou ...`
  - `o agusto mandou isso` gives `O Augusto mandou isso`
- The swallowed one-letter word and `Em agosto o Augusto` turning into `Em Augusto Augusto` remain, identical to the no-LLM pipeline. The plan now records this as an inherited matcher limit (Landing note under the doors; Out of scope "algoritmo da fuzzy"). That classification matches round 1's probe: both behaviours happen before F8 and without the LLM.

### Status of round-1 findings

1. **Fixed (`02ccdfb`, C3, C13).** See above. AC 3 and AC 5 are reworded to the parity contract plus "no term in uppercase", and the code meets them.
2. **Accepted.** Recorded in `checks.md` Swept ("concorrência de settings"). Unchanged in code.
3. **Fixed (C14).**
4. **Fixed.** The `checks.md` Swept observability line now names the `error!` (`llm_auto.rs:94`) and the `debug!` (`:117`). `grep -nE "(debug|info|warn|error)!" apps/desktop/src/llm_auto.rs` gives exactly those two lines.
5. **Stands (non-blocking). The coordinator's recount is not right.** `SIXTEEN` is not F7a's untouched constant. `git show eb0482c:apps/desktop/src/llm_auto.rs` has `... para o time inteiro hoje` (16 words after `hã` is dropped). `700dece` and `02ccdfb` both append `todo` (`... time inteiro todo hoje`). The rules text `Eu acho que a gente pode mandar o relatório amanhã cedo para o time inteiro todo hoje` is 17 words. The doc comment at `llm_auto.rs:175` and C1's "16 palavras" are therefore off by one. The 16-word boundary is still exercised by C3's `long`, C6's `long` and C14's text. Either revert `todo` or fix the comment.
6. **Note, unchanged** (`--bench` printout).
7. **Note, unchanged** (UI optimistic list).

### New in round 2

8. **Non-blocking - "parity with the no-LLM pipeline" holds up to the TM's step order.** Without the LLM, the TM runs the fuzzy *before* filler removal, `normalize_transcription_output` and text-based language detection (`transcription.rs:1779-1818`). On the deferred path the fuzzy runs *after* them, because `raw.text` is the TM output. A filler or a normalization change next to a term can therefore produce different n-grams on the two paths. The 3 C13 inputs and the 7 probe inputs contain no filler, so they cannot show this. The difference is rare and may sometimes favour the deferred path (fillers are removed before matching). AC 5's "same text" is met for filler-free text. In general it is "same order of fuzzy and rules".
9. **Cosmetic.** The new plan note reads `( charge bee → ChargeBee, o augusto → Augusto)` without backticks, and the `a` of `a charge bee` is missing (`plan.md:53`).

### Round-2 gate

| Command | Result |
| --- | --- |
| `cargo test -p fala --lib -- --exact <C3, C6, C13, C14>` | exit 0; 4 passed, 0 failed, 355 filtered out |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0; only the two build-script notes |
| `cargo test -p fala --lib` | exit 0; 359 passed, 0 failed, 0 ignored |
| `cargo test -p fala-postproc` | exit 0; 8 + 14 passed, doc 0 |
| `scripts/check-brand.sh` (Git Bash) | exit 0 |
| Probe, mirror of the new `format` (outside the repo, `C:\f\dictprobe`) | 7 of 7 inputs equal to the no-LLM pipeline; no `CHARGEBEE` or `AUGUSTO` |
| Frontend (`lint`, `format:check`, `check:translations`) | not re-run: UI slice unchanged (`range-diff` `=`); round 1 exit 0 / 0 / 0 |
| Per-slice compile | not run separately. `8511599` touches no Rust, so HEAD's cargo results are `02ccdfb`'s |

All cargo commands in the repo used `CARGO_TARGET_DIR=C:\f\dict`, one at a time. Before this rewrite, `git status --porcelain` showed only this report as untracked.

## Round 1 - full

The rows below are the round-1 record at `8ddfe8f`, unchanged. Their `llm_auto.rs` line numbers are from that revision.

10 of the 11 automated checks (C1, C2, C4-C10, C12) are proven with a located assertion or command output. C11 is the manual Windows UI check. It sits outside the verdict (L-004) and is listed under `## Manual (outside the verdict)`.

**C3 fails as written.** Its proof test passes, but the test was written with a different input from the one the check names. With the check's own input (`a charge bee ...`), the two configured-but-not-requested cases (app on the list, 15 words) paste `CHARGEBEE ...`, not `A ChargeBee ...`. That breaks AC 3 for those cases (Finding 1). Everything else in the diff reads correct against the plan and the two one-way doors.

## Checks

All proofs ran at `8ddfe8f` (`git rev-parse HEAD`) in `C:\dev\fala\.houston\worktrees\dictionary`, with `CARGO_TARGET_DIR=C:\f\dict` and one cargo command at a time. The 9 named unit tests ran in one invocation: `cargo test -p fala --lib -- --exact llm_auto::tests::dictionary_reaches_the_llm_prompt llm_auto::tests::empty_dictionary_keeps_the_prompt llm_auto::tests::rules_apply_the_dictionary_without_llm managers::transcription::tests::fuzzy_waits_for_the_llm_while_it_is_configured llm_auto::tests::llm_text_is_not_fuzzy_corrected llm_auto::tests::deferred_fuzzy_fixes_text_the_llm_did_not_write llm_auto::tests::no_second_fuzzy_without_llm actions::tests::legacy_keeps_the_fuzzy_while_llm_is_configured llm_auto::tests::custom_words_are_normalized`. It exited 0 with `9 passed; 0 failed; 0 ignored; 348 filtered out`, and each name printed `... ok`. In the table, "batch" means this run.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | `[" Augusto ", "augusto", "ChargeBee"]`, key set, more than 15 words: 1 request whose `systemInstruction.parts[0].text` is `SYSTEM_PROMPT + "\n\nDicionário pessoal:\n- Augusto\n- ChargeBee"` (AC 1, door 1) | `llm_auto::tests::dictionary_reaches_the_llm_prompt ... ok` (batch) | `apps/desktop/src/llm_auto.rs:271` sets the list with a padded duplicate. `:275-280` assert `llm_produced`, `requests() == 1` and that the parsed body field (`system_prompt`, `:254-261`, reads `systemInstruction.parts[0].text`) equals the exact string. The dictionary is built in production code at `llm_auto.rs:49` (`Dictionary::new(&settings.custom_words)`) and passed at `:111-113`. The string is assembled in the crate (`crates/postproc/src/gemini.rs:102-104`). The store-side half of door 1 (an old store with duplicates is normalized at use) is proven by the duplicate in the input. Note: `SIXTEEN` is now 17 words after the rules, not 16 (Finding 5). | PASS |
| C2 | `[]` and `["", "  "]`: `systemInstruction` is exactly `SYSTEM_PROMPT` (AC 2) | `...empty_dictionary_keeps_the_prompt ... ok` (batch) | `llm_auto.rs:286-292`: both cases go to one server, and `system_prompt(&server, index) == SYSTEM_PROMPT` with `llm_produced` for each. Because the comparison is exact, an empty `Dicionário pessoal:` header would fail it. | PASS |
| C3 | `["ChargeBee"]`, server up: **`a charge bee ...` becomes `A ChargeBee ...`** with the LLM off, with no key, in `keepassxc` and at 15 words; 0 requests (AC 3) | `...rules_apply_the_dictionary_without_llm ... ok` (batch). Probe with the check's own input (see Finding 1) | The test (`llm_auto.rs:296-324`) uses `ontem charge bee ...` and asserts `starts_with("Ontem ChargeBee mandou")`, 4 times `!llm_produced` and `requests() == 0`. For that input it proves the rules get the dictionary in all 4 cases. With `Dictionary::default()` the "off" and "keyless" cases would fail. **The claim as written does not hold.** The probe mirrors `formatter_at` and `format` (`enabled: true`, key set, `disabled_apps = ["keepassxc"]`, rules plus `Dictionary::new`, then `apply_custom_words(.., 0.18)` when the editor is not `Llm`). It prints `CHARGEBEE mandou o relatório amanhã cedo para o time inteiro de vendas da empresa hoje` for `keepassxc` and `CHARGEBEE mandou ... da empresa` for the 15-word case. Both have `llm_produced = false`. The "off" and "keyless" cases have no deferred fuzzy, so they give `A ChargeBee ...` as claimed. | **FAIL** (2 of 4 cases with the named input) |
| C4 | `post_process_transcription_text` with `["Augusto"]`: key and `llm_enabled` keep `agusto`; no key, or `llm_enabled = false`, give `Augusto` (AC 4, AC 6, door 2) | `managers::transcription::tests::fuzzy_waits_for_the_llm_while_it_is_configured ... ok` (batch) | `apps/desktop/src/managers/transcription.rs:2244` (no key gives `Augusto mandou`), `:2249` (key plus enabled gives `agusto mandou`), `:2251` (enabled off gives `Augusto mandou`). The skip is at `transcription.rs:1779-1781`. It is the same function on the batch path (`:1496`) and the streaming finalize path (`:1139`, which passes `custom_words_already_prompted = false`). If the predicate were missing, `:2249` would fail. If it read only `llm_enabled`, `:2244` would fail. | PASS |
| C5 | LLM configured, `["Augusto"]`, server answers `O agusto mandou o relatório.` to a 16-word dictation with `agusto`: the body has `agusto`, the pasted text is the answer unchanged, `llm_produced = true` (AC 4) | `...llm_text_is_not_fuzzy_corrected ... ok` (batch) | `llm_auto.rs:335-336` (16 words, counted by hand), `:340` (1 request), `:341` (body contains `O agusto acho`, so no fuzzy before the LLM), `:342-348` (the answer pasted unchanged, `llm_produced: true`, so no fuzzy after the LLM). The production guard is `llm_auto.rs:117-120` (`Some(fuzzy) if !llm_produced`). | PASS |
| C6 | configured, `["Augusto"]`: `agusto` becomes `Augusto` at 4 words (0 requests), in `keepassxc` (0 requests) and at 16 words with HTTP 500 (1 request) (AC 5) | `...deferred_fuzzy_fixes_text_the_llm_did_not_write ... ok` (batch) | `llm_auto.rs:359-360` (`O Augusto mandou isso`), `:361-363` (`keepassxc`, then `quiet.requests() == 0`), `:365-369` (HTTP 500: `O Augusto acho`, `!llm_produced`, `failing.requests() == 1`). The test has no timeout case. A timeout takes the same `editor != Llm` branch (`checks.md` Coverage accepts this). Without `Fuzzy::deferred` (`llm_auto.rs:78-83`) every assertion fails. | PASS |
| C7 | no key: `Fuzzy::deferred` is `None`, and `agusto mandou` comes out `Agusto mandou` (AC 6) | `...no_second_fuzzy_without_llm ... ok` (batch) | `llm_auto.rs:376` (`is_none()`), `:379-381` (`Agusto mandou`, 0 requests). Together with C4 `:2244`, this shows the fuzzy runs exactly once when the LLM is not configured. | PASS |
| C8 | `legacy_post_process`, LLM configured, `["Augusto"]`, no legacy provider: `agusto mandou` becomes `Augusto mandou` (AC 7) | `actions::tests::legacy_keeps_the_fuzzy_while_llm_is_configured ... ok` (batch) | `apps/desktop/src/actions.rs:1088-1090` (`Augusto mandou`, `post_processed_text == None`, `!llm_produced`). Production: `actions.rs:517-521` runs `Fuzzy::deferred` before `post_process_transcription` (`:524`). It is reached from `OutputMode::Legacy` at `:488-489`. Without the deferred fuzzy, `:1088` fails. | PASS |
| C9 | `normalize_words([" Fala ", "fala", "", "ChargeBee", "  "]) == ["Fala", "ChargeBee"]`, and `update_custom_words` stores through it (AC 8, door 1) | `llm_auto::tests::custom_words_are_normalized ... ok` (batch). `grep -n "normalize_words" apps/desktop/src/shortcut/mod.rs` printed `885:    settings.custom_words = crate::llm_auto::normalize_words(words);` | `llm_auto.rs:385-390`. The code is `llm_auto.rs:129-131` (`Dictionary::new(words).terms().to_vec()`; the crate dedups by `to_lowercase` and keeps the first spelling, `crates/core/src/dictionary.rs:22`). Command: `shortcut/mod.rs:883-888`, signature unchanged, so `bindings.ts` is untouched. The command itself has no test (it needs an `AppHandle`). It is one line, proven by reading. | PASS |
| C10 | `CustomWords.tsx` compares duplicates with `toLowerCase()`; `customWords.title`/`.description` changed in pt and en; `bun run lint`, `format:check`, `check:translations` exit 0 (AC 9, AC 10) | `bun run lint` exit 0 (`eslint src`, no output). `bun run format:check` exit 0 (`All matched files use Prettier code style!`, then `cargo fmt --all -- --check`). `bun run check:translations` exit 0 (`Reference has 464 keys`, `PT: All keys present`). `git diff main -- src/` read (3 files, +6 -5) | `src/components/settings/CustomWords.tsx:30-31` (`lowered`, `some(word => word.toLowerCase() === lowered)`), then the existing `duplicate` toast at `:32-37`. `src/i18n/locales/pt/translation.json:403-404` (`Dicionário pessoal`, new description) and `en/translation.json:403-404` (`Personal dictionary`). No new literal in JSX: title and description go through `t(...)` (`CustomWords.tsx:61-62`). The i18n key stays `settings.advanced.customWords` (plan Out of scope). | PASS |
| C12 | fmt, clippy, `cargo test -p fala --lib`, `cargo test -p fala-postproc`, `check-brand.sh` exit 0 | `cargo fmt --all -- --check` exit 0. `cargo clippy --workspace --all-targets -- -D warnings` exit 0. `cargo test -p fala --lib` exit 0. `cargo test -p fala-postproc` exit 0. `& 'C:\Program Files\Git\bin\bash.exe' scripts/check-brand.sh` exit 0 (`ok: no Handy branding outside the allowlist`) | Clippy printed only the two build-script notes (`Generated tray translations: 2 languages, 11 fields`, `Staged 13 transcribe-cpp runtime library file(s)`), with no lint. `fala` lib: `357 passed; 0 failed; 0 ignored`. `fala-postproc`: unit `8 passed`, `tests/fake_gemini.rs` `14 passed`, doc-tests 0. | PASS |

## Correctness reading (against the plan and the doors)

- **(a) Same predicate, skip and defer.** The TM skips when `!custom_words.is_empty() && !custom_words_already_prompted && !llm_configured(settings)` is false (`transcription.rs:1779-1781`). The formatter defers when `llm_configured(settings) && !custom_words.is_empty()` (`llm_auto.rs:79`).
  - For non-Whisper models (`already_prompted = false`), exactly one of the two runs, for any `custom_words`, including a blank-only list (both see `!is_empty()`).
  - `llm_configured` (`llm_auto.rs:64-66`) and the `Postprocessor`'s Gemini client (`:42`) both use the same `gemini_key`, so a key that `ApiKey::new` rejects counts as "no key" on both sides.
  - The two sides read two `get_settings` snapshots: one in the TM (`transcription.rs:1136` and the batch path) and one in `process_transcription_output` (`actions.rs:472`). See Finding 2.
- **(b) RulesOnly and Legacy keep the fuzzy.**
  - F7a used to set `settings.llm_enabled &= use_llm` before building the postprocessor. That mutation is gone. `formatter_at` now puts `use_llm` only into `LlmConfig.enabled` (`llm_auto.rs:45`), while `Fuzzy::deferred` reads the real settings (`:50`). A `RulesOnly` retry (`commands/history.rs:90` re-runs `tm.transcribe`, which skips, and then the formatter defers) therefore gets the fuzzy once, and `llm_produced` is false.
  - `Legacy` returns before the formatter (`actions.rs:488-489`) and applies the deferred fuzzy itself (`:518-521`). Without the LLM configured, `deferred` is `None` and the TM already ran it, so nothing changes.
  - The RulesOnly arm has no unit test (Finding 3).
  - Small ordering change: on both paths the fuzzy now runs after OpenCC (`actions.rs:479-483`) instead of before it. The matcher skips non-ASCII keys (`text.rs:58-60`), so this has no visible effect.
- **(c) Streaming finalize.** `finalize_stream` (`transcription.rs:1136-1145`) calls the same `post_process_transcription_text` with `false`, so it skips under the same predicate. Its output goes to `process_transcription_output` (`actions.rs:797`, `:861`) like the batch path. No path skips the fuzzy without a formatter after it, except the dev `--bench` printout (`lib.rs:585`, Finding 6).
- **(d) Can the tests pass with the feature broken?**
  - C1, C2, C4-C9 each fail if their part of the feature is removed (see the Evidence column).
  - C3 is the exception. Its input was moved away from the case the check names, so it cannot see Finding 1.
  - No test drives `formatter_at(.., false, ..)` (Finding 3).
- **Whisper.** With the LLM configured, Whisper now gets the deferred fuzzy on text the LLM did not write (`already_prompted` stops only the TM side). The plan's Landing accepts this. It is noted here, not a defect.
- **Invariants.**
  - Every new `unwrap` is under `#[cfg(test)]` (`llm_auto.rs:147+`, the `actions.rs` and `transcription.rs` test modules). Workspace clippy with `-D warnings` exits 0.
  - The only new log line is `llm_auto.rs:92` `error!("Custom-word correction panicked; keeping the text")`. It contains no text, term or key. `grep -nE "(debug|info|warn|error)!" apps/desktop/src/llm_auto.rs` gives `:92` and `:115` (Finding 4).
  - The test key literal is `"chave-de-teste"`.
  - The diff touches nothing under `crates/` and adds no `#[cfg(target_os)]` and no dependency.
- **Commits.**
  - Both subjects are lowercase, imperative `type(scope): description` (`feat(desktop): ...` 68 chars, `feat(ui): ...` 55 chars).
  - Both bodies explain why and end with `Assisted-by: Claude Code`. There is no `Co-Authored-By` or `Signed-off-by`.
  - Each slice carries its own tests. `normalize_words` arrives in 700dece together with its only caller.

## Findings

1. **Blocking - with the LLM configured but not asked, the deferred fuzzy overwrites the rules' dictionary spelling, so AC 3 / C3 fail for a sentence-initial one-letter word before a term.**
   - **What happens.** On the `editor != Llm` path, `format` runs `apply_custom_words` on the rules output (`llm_auto.rs:117-119`). The inherited matcher considers only n-grams that start at the current word (`text.rs:174-200`). For `A ChargeBee` it scores the 2-gram `achargebee` against `chargebee` at 1/10 = 0.1, under the 0.18 default (`settings.rs:740-742`). It consumes the `A`, and `preserve_case_pattern("A", ..)` uppercases the whole term (`text.rs:224-225`).
   - **Probe results.** The probe compared three orders on the same inputs: pre-F8 (fuzzy, then rules with an empty dictionary); F8 not configured (fuzzy, then rules with the dictionary); F8 deferred (rules with the dictionary, then fuzzy).

     | raw | pre-F8 | F8 not configured | F8 deferred | rules alone |
     | --- | --- | --- | --- | --- |
     | `a charge bee mandou o relatório` | `ChargeBee mandou ...` | `ChargeBee mandou ...` | `CHARGEBEE mandou ...` | `A ChargeBee mandou ...` |
     | `A charge bee mandou o relatório.` | `CHARGEBEE mandou ...` | `ChargeBee mandou ...` | `CHARGEBEE mandou ...` | `A ChargeBee mandou ...` |
     | `o augusto mandou isso` | `Augusto mandou isso` | `Augusto mandou isso` | `AUGUSTO mandou isso` | `O Augusto mandou isso` |
     | `O Augusto mandou isso.` | `AUGUSTO mandou isso.` | `Augusto mandou isso.` | `AUGUSTO mandou isso.` | `O Augusto mandou isso.` |
     | `ontem a charge bee mandou ...` | `Ontem ChargeBee mandou ...` | same | same | `Ontem a ChargeBee mandou ...` |

   - **The author's "pre-existing" note: partly confirmed, partly refuted.**
     - *Confirmed:* the one-letter word is swallowed in every order, before and after F8. That is inherited matcher behaviour, and the plan puts the algorithm out of scope.
     - *Confirmed:* the all-caps output existed before F8, but only when the ASR text itself starts with a capital (`O Augusto ...` gave `AUGUSTO ...`).
     - *Refuted:* that F8 adds nothing. On the deferred path the fuzzy now runs **after** the rules, so it gets the last word. The rules alone produce the correct `A ChargeBee` / `O Augusto`. The fuzzy then turns lowercase ASR text into all caps too (`o augusto mandou isso` gives `AUGUSTO mandou isso`, where pre-F8 gave `Augusto mandou isso`). It also damages a name the ASR heard correctly.
     - *Improvement:* on the not-configured path F8 is better than before, because the rules with the dictionary repair the case.
   - **Why it blocks.** AC 3 says the pasted text carries the dictionary spelling of the variants the rules recognize (`charge bee` → `ChargeBee`) whenever the LLM is not asked. `checks.md` C3 names exactly `a charge bee ...` → `A ChargeBee ...` for the `keepassxc` and 15-word cases. The production path gives `CHARGEBEE ...` for both (probe, C3 row). The test passes only because it uses `ontem charge bee ...`. The pattern (article + name at the start of a short dictation) is ordinary Portuguese.
   - **Options (the author's or maintainer's call; not applied here):**
     - (a) re-apply the rules' dictionary spelling after the deferred fuzzy. The not-configured column shows the rules turn `CHARGEBEE` back into `ChargeBee`, though the article is still lost.
     - (b) skip the deferred fuzzy for a span the rules already spelled exactly as a term.
     - (c) amend the plan (AC 3, and Landing door 2 or Assumptions) and C3 to accept this limitation explicitly, and make C3's test use the named input with the expected output.
2. **Non-blocking - the skip and the deferral read two settings snapshots.** The TM reads `get_settings` when it transcribes or finalizes. `process_transcription_output` reads it again afterwards (`actions.rs:472`). If `llm_enabled` or the Gemini key changes in between (a toggle or a key edit in the UI), the fuzzy runs twice or not at all for that one dictation. A flaky keyring does not cause it: `KeyVault::hydrate` caches the first result, including a failed read (`settings.rs:393-406`). The window is one dictation long. Passing the TM's decision along with the transcription would close it.
3. **Non-blocking - `RulesOnly` with the deferred fuzzy has no test.** Every `llm_auto` test calls `formatter_at(settings, true, ..)` (`llm_auto.rs:246`). The retry case (`use_llm = false` with the LLM configured, so no request but the fuzzy still runs) is proven only by reading `llm_auto.rs:45`, `:50`. It needs no `AppHandle`, so `formatter_at(&settings, false, ..)` could pin it. That matters because F7a's old `llm_enabled &= use_llm` mutation would have silently dropped the fuzzy here.
4. **Non-blocking - `checks.md` Swept "observability" is out of date.** It says `grep -nE "(debug|info|warn|error)!" apps/desktop/src/llm_auto.rs` still shows only the `editor` line. There is now also `llm_auto.rs:92` (`error!`, a fixed message without text, term or key). The plan's Observable ("no new log with text or term") holds. Only the check's wording is off.
5. **Non-blocking - the `SIXTEEN` constant has 17 words after the rules.** 700dece appended `todo`. `SIXTEEN_RULES` (`llm_auto.rs:162-163`) counts 17, but the doc comment (`:159`) and C1's text still say 16. The exact 16-word boundary is still covered: C6's `long` (`:353-354`, 16 words, 1 request) and C3's `long` (16 words after joining `charge bee`).
6. **Non-blocking, note - outputs that bypass the formatter.** The `--bench` printout (`apps/desktop/src/lib.rs:585`, `:627`) shows the raw TM output. With the LLM configured, that output now lacks the fuzzy. It is a dev tool, but bench texts taken with and without a key are not comparable. Whisper's accepted change is described above.
7. **Non-blocking - the UI keeps an unnormalized list until reload.** `updateSetting` sets the optimistic value and does not re-read it (`src/stores/settingsStore.ts:339-345`). If an old store holds `["Fala", "fala"]`, the chips keep showing both after an add, while the store keeps `["Fala", ...]`. The next settings load fixes it. This is cosmetic.

## Manual (outside the verdict)

Per L-004 and `checks.md` Coverage, these are a checklist for the session with the maintainer:

- [ ] **C11** - in `bun run tauri dev` on Windows: the screen shows "Dicionário pessoal" with the new description (pt and en). Adding `augusto` when `Augusto` exists shows the `duplicate` toast. A dictation of more than 15 words with a list term comes out with the dictionary spelling. While there, also dictate a short `a charge bee mandou o relatório` with the LLM configured (Finding 1). Check whether Parakeet's raw output is capitalized (this decides how visible the change from pre-F8 is).
- [ ] Old store with `["Fala", "fala"]`: the next edit in the screen stores `["Fala", ...]` in `settings_store.json` (C9 covers the function; the real store is the maintainer's call).

## Gate

| Command | Result |
| --- | --- |
| `cargo test -p fala --lib -- --exact <9 check tests>` | exit 0; 9 passed, 0 failed, 348 filtered out |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0; only the two build-script notes, no lint |
| `cargo test -p fala --lib` | exit 0; 357 passed, 0 failed, 0 ignored |
| `cargo test -p fala-postproc` | exit 0; unit 8 passed, `fake_gemini` 14 passed, doc 0 |
| `scripts/check-brand.sh` (Git Bash) | exit 0, `ok: no Handy branding outside the allowlist` |
| `bun run lint` / `bun run format:check` / `bun run check:translations` | exit 0 / 0 / 0 (464 keys, PT complete) |
| `grep -n "normalize_words" apps/desktop/src/shortcut/mod.rs` | `885` |
| `grep -nE "(debug\|info\|warn\|error)!" apps/desktop/src/llm_auto.rs` | `92` (`error!`, no text), `115` (`debug!` editor) |
| Scratch probe (outside the repo, `CARGO_TARGET_DIR=C:\f\dictprobe`) | C3 input, configured: `keepassxc` and 15 words both give `CHARGEBEE ...`, `llm_produced = false` |
| `cargo deny check`, `scripts/check-no-tauri-in-crates.sh` | not run (not in C12; the diff adds no dependency and touches nothing in `crates/`) |
| C11 | not run (manual, outside the verdict) |

All cargo commands in the repo used `CARGO_TARGET_DIR=C:\f\dict`. `git status --porcelain` was empty before this report was written.
