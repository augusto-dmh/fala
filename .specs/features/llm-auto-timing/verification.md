# llm-auto-timing verification

Windows 11, `CARGO_TARGET_DIR='C:\f\followup-fixes'`, 2026-10-13.

| Check | Result | Evidence |
| --- | --- | --- |
| C1 | pass | `cargo test -p fala --lib llm_auto::tests::a_slow_block_keeps_its_rules_until_the_late_answer` 10 runs in a row: 10/10 passed. Then 4 runs of `cargo test -p fala --lib llm_auto` at once, as load: 4 × 24 passed |
| C2 | pass | `cargo test -p fala --lib llm_auto`: 24 passed, 0 failed |
| C3 | pass | `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` via the pre-push hook |

The exact 2 s insert deadline is still checked by `crates/postproc/tests/fake_gemini.rs`
(`slow_response_falls_back_at_two_seconds`, `late_response_is_delivered_as_late_edit`) and by
`slow_failed_or_invalid_llm_keeps_rules_text` here. The last one has the same kind of wall-clock
bound (2.5 s) and was not touched.
