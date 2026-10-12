# snippets verification

Windows 11 (Alienware 16), `CARGO_TARGET_DIR=C:\f\snippets`, 2026-10-12.

| Check | Evidence | Result |
| --- | --- | --- |
| C1 | `cargo test -p fala-postproc --lib`: `snippet_becomes_a_marker_and_then_its_text`, `snippet_only_whole_words_without_punctuation_between` (32 passed) | pass |
| C2 | `whole_dictation_is_the_text_as_is`; `--test fake_gemini snippet_without_llm_and_whole_dictation` (0 requests) | pass |
| C3 | `--test fake_gemini snippet_reaches_the_llm_only_as_a_marker`: body has `⟦S1⟧` and the placeholder line; none of "augusto", "segredo", "4815", "fala.dev", "minha assinatura" | pass |
| C4 | `restore_needs_every_marker_once`; `altered_marker_keeps_the_rules_text` (3 broken answers → rules text, `InvalidResponse`); `late_answer_gets_the_snippet_back` | pass |
| C5 | `cargo test -p fala-core`: 16 passed (`trigger_needs_two_words`, `replacements_round_trip` with snippets out of terms and JSON) | pass |
| C6 | `renders_the_variables`: `02/10/2026 · 2 de outubro de 2026 · 14:05` | pass |
| C7 | `cargo test -p fala-cli format`: 16 passed, `prints_final_text_and_editor` expands `minha assinatura => Augusto\nfala.dev` | pass |
| C8 | `cargo test -p fala --lib -- dictionary`: 10 passed (`snippets_need_two_words_and_keep_stored_uses`, `dictionary_resolves_snippet_variables_and_keeps_them_off_the_terms`) | pass |
| C9 | `llm_auto::tests::snippets_are_applied_and_reported_with_or_without_the_llm` (30 passed with `dictionary` + `llm_auto`) | pass |
| C10 | `bun run lint`, `bunx tsc --noEmit`, `bun run check:translations`, `bunx prettier --check`, `bun src/components/shell.test.tsx` (C1–C32 ok) | pass |

Workspace: `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test --workspace`: 992 passed, 0 failed, 55 ignored.

## Review

`code-review` (high) on the whole diff found 10 items. Fixed: triggers with spoken punctuation never matched (snippets now run before spoken punctuation, and the pairs/terms pass never rereads a marker); an LLM answer containing `⟦S` without any snippet was rejected; the marker counter rescanned the hits. Kept, with the reason in the PR: the legacy post-process shortcut runs no rules at all; the fuzzy correction can change a trigger word, as it can a pair; duplicate detection by case only, as in #108; a snippet at the start of a dictation keeps its own capitalization; a trigger split across two long-dictation blocks does not match.

## Not tested

- `fake_gemini` `no_late_edit_unless_timeout` and `response_under_two_seconds_is_used` (a 1.5 s reply against the 2 s deadline) failed in some local runs while other worktrees were compiling, also with the new tests skipped; they pass alone.
- Dictating a trigger in the running app with the microphone, and the page by hand: no one to dictate tonight. Script: Dicionário → Snippets → add "minha assinatura" with two lines → dictate "minha assinatura" (the two lines, as they are) and "segue minha assinatura obrigado" (expanded mid-sentence) → the list shows 2 uses.
- A real Gemini answer with a marker: no paid calls.
