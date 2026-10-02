# core-contract checks

Profile: light
Plan: `.specs/features/core-contract/plan.md`

12 checks in 3 slices · 4 one-way doors · 0 open, of which 0 block

## Checks

Unit proofs: `cargo test -p fala-core <path> -- --exact`, with `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=3`. Tests live next to the code in `crates/core/src/`.

### S1 - idioma · 3 files · 4 KB · ~1k

**C1** - `Language::from_str` maps `pt-BR` → `PtBr`, `PT-br` → `PtBr`, `pt` → `PtBr`, `en` → `En`, asserted case by case (AC 1)
Proof: `cargo test -p fala-core language::tests::parses_known_tags -- --exact`

**C2** - `Language::from_str` on `es`, `pt-PT` and `""` returns `CoreError::UnknownLanguage(s)` with `s` equal to each input (AC 2)
Proof: `cargo test -p fala-core language::tests::rejects_unknown_tags -- --exact`

**C3** - `serde_json::to_string(&Language::PtBr)` is `"\"pt-BR\""`, `Language::En` is `"\"en\""`, and `serde_json::from_str` of each string returns the same variant (AC 3, door 1)
Proof: `cargo test -p fala-core language::tests::serializes_as_bcp47_tag -- --exact`

**C4** - `Language::PtBr.tag()` is `"pt-BR"` and `Language::En.tag()` is `"en"`, each equal to its serialized string without quotes (AC 4)
Proof: `cargo test -p fala-core language::tests::tag_matches_serialized_form -- --exact`

### S2 - áudio do ditado · 2 files · 3 KB · ~1k

**C5** - `static_assertions::assert_not_impl_any!(DictationAudio: serde::Serialize)` compiles inside the test module, so the crate's test build fails if `Serialize` is ever implemented (AC 5, door 3)
Proof: `cargo test -p fala-core audio::tests::dictation_audio_is_not_serializable -- --exact`

**C6** - `DictationAudio::new(vec![0.0; 16_000]).duration()` equals `Duration::from_secs(1)` and `DictationAudio::SAMPLE_RATE_HZ == 16_000` (AC 6)
Proof: `cargo test -p fala-core audio::tests::one_second_of_samples -- --exact`

**C7** - `DictationAudio::new(vec![])` has `duration() == Duration::ZERO` and `is_empty() == true` (AC 7)
Proof: `cargo test -p fala-core audio::tests::empty_audio -- --exact`

### S3 - ditado, editor e dicionário · 3 files · 6 KB · ~2k

**C8** - `Editor::None`, `Editor::Rules`, `Editor::Llm` serialize to `"none"`, `"rules"`, `"llm"` and each string deserializes back to its variant (AC 8, door 2)
Proof: `cargo test -p fala-core dictation::tests::editor_serialized_values -- --exact`

**C9** - A `Dictation` with raw `"oi tudo bem"` (`PtBr`), final `"Oi, tudo bem?"`, editor `Llm`, app `Some("Slack")` round-trips through `serde_json` to a value `==` to the original (AC 9)
Proof: `cargo test -p fala-core dictation::tests::dictation_round_trips_json -- --exact`

**C10** - `Dictation::unedited(transcript, app)` has `final_text == transcript.text` and `editor == Editor::None` (AC 10)
Proof: `cargo test -p fala-core dictation::tests::unedited_keeps_raw_text -- --exact`

**C11** - `Dictionary::new([" Fala ", "fala", "", "ADR", "  "]).terms()` is exactly `["Fala", "ADR"]` (AC 11)
Proof: `cargo test -p fala-core dictionary::tests::normalizes_terms -- --exact`

**C12** - `scripts/check-no-tauri-in-crates.sh` exits 0 and `rg -n 'cfg\(target_os|cfg\(windows' crates/core` prints nothing (exit 1) (AC 12)
Proof: `scripts/check-no-tauri-in-crates.sh && ! rg -n 'cfg\(target_os|cfg\(windows' crates/core`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| accepted language inputs (4) | `pt-BR` C1 · `PT-br` C1 · `pt` C1 · `en` C1 | - |
| rejected language inputs (3) | `es` C2 · `pt-PT` C2 · empty C2 | - |
| `Language` variants, serialized form (2) | `PtBr` C3 · `En` C3 | - |
| `Editor` variants (3) | `none` C8 · `rules` C8 · `llm` C8 | - |
| `Dictionary` normalization rules (4) | trim C11 · drop empty C11 · case-insensitive dedup C11 · first spelling and order kept C11 | - |
| one-way doors (4) | door 1 C3 · door 2 C8 · door 3 C5 · door 4 (dependencies) C5, C3 | - |

- No check claims more than the cases its proof exercises

## Swept

- validation: C1, C2, C11
- failure modes: C2
- idempotency: n/a - pure value types, no operation that can be repeated with effect
- authorization: n/a - library types with no caller identity
- concurrency: n/a - no shared state; types are plain values
- data lifecycle: n/a - nothing persists in this feature; C (storage) owns persistence of the serialized forms fixed by C3 and C8
- dependency failure: n/a - no external dependency at runtime
- state transitions: n/a - `Editor` is a value, not a state machine; who moves a `Dictation` from `none` to `llm` is B
- observability: n/a - no logging in value types; ditado content must not be logged above debug anyway (AGENTS.md)

## Handoff

- S1-S3 = ~4k, all in `fala-core`, under the 150k budget - one builder
