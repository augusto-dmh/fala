# postproc-spoken-punctuation verification

**Verdict**: PASS
**Profile**: light
**Diff range**: origin/main..d218bcbe67286dec8212842336d9c1a915d2944d (1 commit, `d218bcb`)
**Round**: 1 - full
**Verifier**: independent sub-agent (author != verifier)

No `plan.md` (checks-only feature) and no binding sources: step 1 does not apply under `light`.
Coverage recompute, `Test policy` verdicts and fault injection are `standard`/`ui` steps and did
not run under `light`.

## Proof runs (all at HEAD `d218bcb`)

The shared target dir reuses artifacts across worktrees and the test binaries' dep-info carries
only relative paths, so before running I touched (mtime only) `crates/postproc/src/*.rs` and
`crates/postproc/tests/*.rs`. Cargo then recompiled `fala-postproc` from this worktree
(`Compiling fala-postproc v0.1.0 (/home/augusto/projects/fala/.claude/worktrees/agent-a29363e00c2ab9ba7/crates/postproc)`).
`git status --porcelain` stayed empty.

1. `locked.sh cargo test -p fala-postproc` exit 0. Unit tests: 20 passed, 0 failed (all 13
   `rules::tests::spoken_*` and the 7 pre-existing rules tests listed individually as `ok`).
   `tests/fake_gemini.rs`: 14 passed. `tests/spoken_punctuation.rs`: 1 passed
   (`postprocessor_honors_the_flag ... ok`). Doc-tests: 0.
2. `locked.sh cargo clippy -p fala-postproc --all-targets -- -D warnings` exit 0.
3. `cargo fmt --all -- --check` exit 0.
4. C16: the harness refused the one-line form (git inside `$(...)`), so I ran the same logic in two
   steps: `git diff --name-only origin/main...HEAD > c16-names.txt` (exit 0), then
   `test -z "$(cat c16-names.txt | grep -v '^crates/postproc/' | grep -v '^.specs/features/postproc-spoken-punctuation/')"`
   exit 0. Paths: `.specs/features/postproc-spoken-punctuation/checks.md`,
   `crates/postproc/src/lib.rs`, `crates/postproc/src/rules.rs`,
   `crates/postproc/tests/spoken_punctuation.rs`.

Each named test exists (`rg -n`): `rules.rs:645` spoken_pt_br_each_trigger, `:652`
spoken_en_each_trigger, `:659` spoken_spacing, `:673` spoken_capitalizes_after_sentence_end,
`:685` spoken_does_not_double_asr_punctuation, `:695` spoken_only_whole_words, `:714`
spoken_pt_br_exceptions, `:738` spoken_en_exceptions, `:753` spoken_bare_ponto_needs_pause,
`:764` spoken_follows_language, `:770` spoken_flag_off_and_default_on, `:787`
spoken_order_with_other_rules; pre-existing `:509`, `:534`, `:550`, `:557`, `:566`, `:577`,
`:583`; `tests/spoken_punctuation.rs:19` postprocessor_honors_the_flag. All new tests are added by
this diff.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | each pt-BR trigger becomes its mark (14 pairs incl. singular "parêntese") | `cargo test -p fala-postproc` exit 0; `rules::tests::spoken_pt_br_each_trigger ... ok` | `crates/postproc/src/rules.rs:647` - `assert_eq!(pt(input), *want, "{input}")` over `PT_BR_TRIGGERS` at `rules.rs:600-619`, every pair literal (e.g. `:607` `("oi nova linha tchau", "Oi\nTchau")`, `:615-616` singular parêntese, `:619` `("isso travessão aquilo", "Isso — aquilo")`) | PASS |
| C2 | each English trigger becomes its mark (16 pairs incl. newline, paren, quotes) | same run; `rules::tests::spoken_en_each_trigger ... ok` | `crates/postproc/src/rules.rs:654` - `assert_eq!(en(input), *want, "{input}")` over `EN_TRIGGERS` at `rules.rs:623-641` (e.g. `:632` `("hi newline bye", "Hi\nBye")`, `:638` open/close paren, `:640` open/close quotes, `:641` `("wait dash that", "Wait — that")`) | PASS |
| C3 | spacing of closing, break, parens, dash | same run; `rules::tests::spoken_spacing ... ok` | `crates/postproc/src/rules.rs:665` - `"X, b. C? D! E: f; g"`; `:667` `assert_eq!(pt("x nova linha y"), "X\nY")`; `:668` `"X (y) z"`; `:669` `"X — y"` | PASS |
| C4 | capital after . ? ! break paragraph, never after , : ; | same run; `rules::tests::spoken_capitalizes_after_sentence_end ... ok` | `crates/postproc/src/rules.rs:674-678` - `assert_eq!(pt("sim ponto final não"), "Sim. Não")` ... `assert_eq!(pt("oi novo parágrafo tchau"), "Oi\n\nTchau")`; `:679-681` `"Sim, não"`, `"Lista: um"`, `"X; b"` | PASS |
| C5 | no doubled sign next to ASR punctuation | same run; `rules::tests::spoken_does_not_double_asr_punctuation ... ok` | `crates/postproc/src/rules.rs:686-691` - `assert_eq!(pt("eu acho, vírgula que sim"), "Eu acho, que sim")` ... `assert_eq!(en("Done. Period."), "Done.")`, all 6 pairs literal | PASS |
| C6 | whole words only (7 inputs) | same run; `rules::tests::spoken_only_whole_words ... ok` | `crates/postproc/src/rules.rs:701` - `assert_eq!(pt(input), want, "{input}")` over `:697-699`; `:709` `assert_eq!(en(input), want, "{input}")` over `:704-707` | PASS |
| C7 | pt-BR exceptions stay (10 inputs) | same run; `rules::tests::spoken_pt_br_exceptions ... ok` | `crates/postproc/src/rules.rs:733` - `assert_eq!(pt(input), want, "{input}")` over `:717-731` (all 10 listed literally) | PASS |
| C8 | English exceptions stay (7 inputs) | same run; `rules::tests::spoken_en_exceptions ... ok` | `crates/postproc/src/rules.rs:748` - `assert_eq!(en(input), want, "{input}")` over `:740-746` (all 7 literal) | PASS |
| C9 | bare ponto/period only at a pause | same run; `rules::tests::spoken_bare_ponto_needs_pause ... ok` | `crates/postproc/src/rules.rs:754-760` - `assert_eq!(pt("fim ponto nova linha depois"), "Fim.\nDepois")`, `assert_eq!(pt("vamos ponto depois"), "Vamos ponto depois")`, `assert_eq!(en("we left period then"), "We left period then")` (all 7 pairs) | PASS |
| C10 | language picks the table | same run; `rules::tests::spoken_follows_language ... ok` | `crates/postproc/src/rules.rs:765` - `assert_eq!(en("sim vírgula não"), "Sim vírgula não")`; `:766` `assert_eq!(pt("yes comma no"), "Yes comma no")` | PASS |
| C11 | default on; off returns C1/C2 inputs with first letter capitalized | same run; `rules::tests::spoken_flag_off_and_default_on ... ok` | `crates/postproc/src/rules.rs:771` - `assert!(Rules::default().spoken_punctuation)`; `:782` `assert_eq!(off.apply(input, language, &[]), want, "{input}")` over both tables (`:775-778`) | PASS |
| C12 | order with fillers, repeats, dictionary, first-letter capital | same run; `rules::tests::spoken_order_with_other_rules ... ok` | `crates/postproc/src/rules.rs:788` - `"Então, eu acho"`; `:789` `assert_eq!(pt("fim ponto ponto ponto"), "Fim.")`; `:792` `"ChargeBee, Itaú."`; `:794` `"Itaú — Itaú"`; `:795` `"(Itaú)"`; `:796` `assert_eq!(pt("nova linha oi"), "\nOi")` | PASS |
| C13 | 7 pre-existing tests pass, assertions unmodified | same run; all 7 `... ok`; `git diff origin/main..HEAD -- crates/postproc/src/rules.rs` | tests at `crates/postproc/src/rules.rs:509,534,550,557,566,577,583`; the diff of `mod tests` touches only `rules.rs:500` and `:505` (`apply(...)` -> `Rules::default().apply(...)` in `pt`/`with_dict`) and `:535` (the `en` closure); no `assert` line is removed or edited (e.g. `:510` `assert_eq!(pt("então, hã, eu acho"), "Então, eu acho")` unchanged) | PASS |
| C14 | `Postprocessor` honors the flag end to end | same run; `tests/spoken_punctuation.rs` `postprocessor_honors_the_flag ... ok` | `crates/postproc/tests/spoken_punctuation.rs:21-24` - `assert_eq!(final_text(&on, "sim vírgula não"), ("Sim, não".to_string(), Editor::Rules))`; `:29-32` with `.with_rules(Rules { spoken_punctuation: false })` -> `("Sim vírgula não".to_string(), Editor::Rules)`; wiring at `crates/postproc/src/lib.rs:171` `self.rules.format(&raw.text, &ctx)` | PASS |
| C15 | clippy -D warnings and fmt check exit 0 | `locked.sh cargo clippy -p fala-postproc --all-targets -- -D warnings` exit 0; `cargo fmt --all -- --check` exit 0 | command exit codes (no assertion surface): clippy `Finished dev profile`, no warnings; fmt printed nothing, exit 0. Lint config the run enforced: `crates/postproc/src/rules.rs:70-75` new code uses `unwrap_or`/`map_or`, no `unwrap`/`expect` outside tests | PASS |
| C16 | nothing outside crates/postproc and the spec folder changes | two-step equivalent of the checks.md proof (harness refused the one-liner), exit 0 | `git diff --name-only origin/main...HEAD` -> `.specs/features/postproc-spoken-punctuation/checks.md`, `crates/postproc/src/lib.rs`, `crates/postproc/src/rules.rs`, `crates/postproc/tests/spoken_punctuation.rs` (4 paths, all under the two allowed prefixes) | PASS |

Every check names its input -> output pairs and each one is asserted literally; no precision or
coverage gap among C1-C14. Declared and accepted in checks.md: "end quote" and the plural
"parentheses" English spellings (`rules.rs:277,280,286`) are in the table but not asserted.

## Swept (re-read)

- validation -> C6-C9: proven above.
- idempotency -> C5: proven above.
- concurrency `n/a - Rules is Copy and stateless`: holds, `crates/postproc/src/rules.rs:25`
  `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` on a struct with one `bool` field (`:26-29`).
- failure modes `n/a ... the Formatter stays Ok`: holds, `rules.rs:41`
  `Ok(self.apply(text, *ctx.language, ctx.dictionary.terms()))`.
- observability `n/a - no new log`: holds; grep of added lines for `log::`, `tracing`, `println`,
  `debug!/info!/warn!/error!` found nothing (grep exit 1).
- The other rows are `n/a` policy; nothing in the code to contradict.

## Behaviour notes (outside the checks; do not change the verdict)

Found by reading `spoken_punctuation` / `trigger_at` / `place` (`rules.rs:321-449`) and tracing
inputs by hand; none of these inputs was executed, since the tree is read-only. No check is
violated.

- No panic path found. `lexical_trigger` bounds its window with
  `MAX_TRIGGER_WORDS.min(tokens.len().saturating_sub(i))` (`:394`), so the lookahead call at
  `i + n == len` (`:381`) gets an empty range; `tokens[i + n - 1]` (`:371`) stays inside the window
  just matched; `place` does no indexing; `Token::parse` slices at char boundaries (`:70-75`). The
  only byte slice is `input[..1]` in the test at `:781`, ASCII-only inputs.
1. Medium. A leading closing mark followed by words gets its next word capitalized: "vírgula e
   depois" -> `, E depois`, "dois pontos x" -> `: X`. `spoken_punctuation` correctly leaves
   `capitalize` false (`:333-334`), but `capitalize_first` over the joined string (`:56`, `:482`)
   then uppercases the first letter anywhere in it. That works against the stated purpose of
   start-of-text triggers ("a short dictation can add punctuation to text already in the field").
   C12 asserts only "nova linha oi" -> "\nOi", where the capital is wanted.
2. Low (design trade-off the checks accept as a false-negative class, but the word lists reach
   further than the examples suggest). `PT_NOUN_MARKERS` includes adverbs and pronouns that often
   end a clause, which blocks every trigger after them, not just the bare forms: "é isso mesmo
   ponto final" and "um pouco mais vírgula" stay as words; "opção um vírgula" too. In English
   "one", "no", "that" do the same ("no comma I don't think so", "step one colon"). The bare-form
   adjectives block common endings: "tá bom ponto", "tá certo ponto", "ótimo ponto" keep "ponto"
   as a word (`:292-302`).
3. Low. "dois pontos" as a score after a preposition that is not in the marker list fires: "venceu
   por dois pontos" -> "Venceu por:", "dividido em dois pontos" -> "Dividido em:" ("por" and "em"
   are missing from `PT_NOUN_MARKERS`). Close to the accepted "dois pontos as a score"
   false-positive class, but those examples have a verb, adverb or nothing after; these have a
   preposition before.
4. Low. A spoken closing mark after an ASR ellipsis pops only one char (`:440-443`): "espera...
   vírgula" -> `Espera..,`.
5. Low. The pause lookahead for bare "ponto"/"period" is lexical only (`:381`): it counts a
   following trigger that `trigger_at` would itself reject (e.g. "ponto nova linha de ..." fires
   the "." even though "nova linha de" stays as words).
6. Cosmetic. An opening mark followed directly by a break or closing mark leaves the opening sign
   orphaned before it ("abre aspas nova linha oi" -> `"` + `\n` + `Oi`), because `place` flushes
   `pending` as its own token (`:431-432`). Garbage-in.

## Gate

`cargo test -p fala-postproc` at `d218bcb` - 35 passed, 0 failed (20 unit + 14 fake_gemini + 1
spoken_punctuation); clippy exit 0; fmt exit 0; C16 exit 0. 16/16 checks proven with located
evidence.
