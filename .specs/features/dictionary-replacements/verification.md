# dictionary-replacements verification

**Verdict**: PASS for C1-C10 and C12; C11 PASS for the automated part, manual part unproven (no screen)
**Profile**: light
**Diff range**: cca0a87..feat/dictionary-replacements (f6c5433)
**Round**: 1 - full
**Verifier**: the author, in the same session, on the Windows 11 machine (Alienware 16), unattended overnight. Not an independent verifier.

Every `cargo` ran with `CARGO_TARGET_DIR=C:\f\dictionary-replacements`.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | pair replaces case/accent/hyphen variants, keeps surrounding punctuation | `cargo test -p fala-postproc` | `rules::tests::replacement_matches_spelling_variants ... ok` | PASS |
| C2 | never inside a word, never across punctuation, one-word spoken never joins two | same run | `rules::tests::replacement_only_whole_words ... ok` | PASS |
| C3 | pair wins over term at the same spot; written text not re-read | same run | `rules::tests::replacement_wins_and_is_not_reread ... ok` | PASS |
| C4 | LLM off: written form in the final text; fake server: written form in text and dictionary block, spoken form absent | same run | `replacement_applies_without_llm ... ok`, `replacement_reaches_llm_as_written ... ok`; `fake_gemini` 22 passed | PASS |
| C5 | `Dictionary` normalization, written form in `terms()`, old JSON loads, round-trip | `cargo test -p fala-core` | `dictionary::tests::{normalizes_terms, deserializing_normalizes_terms, replacements_are_normalized, replacements_round_trip} ... ok`; 14 passed | PASS |
| C6 | `Formatted.replaced` one index per occurrence | C4 run | `assert_eq!(out.replaced, [0, 0])` in `replacement_applies_without_llm`; also `rules::tests::replacement_hits_once_per_occurrence` (`[1, 0, 1]`) | PASS |
| C7 | crates green | `cargo test -p fala-core -p fala-postproc -p fala-cli` | all `test result: ok`; `fala-postproc` lib 28, `fake_gemini` 22, `fala-cli` format tests incl. `prints_final_text_and_editor` with a `fala sql -> Fala.sqlite` line | PASS |
| C8 | v0.9 store: `custom_words` intact, no pairs | `cargo test -p fala --lib` | `settings::tests::frozen_v0_9_store_parses_strictly_then_migrates_device_index ... ok` | PASS |
| C9 | command normalization keeps stored `uses` by spoken form | same run | `dictionary::tests::normalize_keeps_stored_uses ... ok` | PASS |
| C10 | uses +1 per occurrence; desktop formatter reports applied spoken forms, from the second pass when the deferred fuzzy re-fixes | same run | `dictionary::tests::count_uses_adds_one_per_occurrence ... ok`, `dictionary_joins_terms_and_pairs ... ok`, `llm_auto::tests::replacements_are_applied_and_reported ... ok`; `431 passed; 0 failed; 1 ignored` | PASS |
| C11 | page: search, terms and pairs with counts, i18n | `bun run lint`; `bun run check:translations`; prettier check (pre-commit); `bunx tsc --noEmit`; `bun src/components/shell.test.tsx` | eslint clean; `All keys present`; `All matched files use Prettier code style!`; tsc exit 0; `C29 ok` | PASS (automated); manual unproven |
| C12 | clippy, fmt, CI scripts | `cargo clippy --workspace --all-targets -- -D warnings`; `cargo fmt --all -- --check`; both scripts | `Finished` with no warnings; fmt clean; `ok: no tauri in crates/`; `ok: no Handy branding outside the allowlist` | PASS |

## No-UI proof with the AI off

`fala-cli format --dictionary dict.txt` with `ChargeBee`, `fala sql -> Fala.sqlite` and `btw -> by the way`, stdin `hã btw o FALA-SQL abriu, mas o falasqlx não`:

```
By the way o Fala.sqlite abriu, mas o falasqlx não
editor: regras
```

## Not tested

- The running app: no live dictation, and the Dictionary page was never opened. The manual script is in the PR body.
- Linux: nothing platform-specific was added; the Linux CI job covers compilation and tests.
