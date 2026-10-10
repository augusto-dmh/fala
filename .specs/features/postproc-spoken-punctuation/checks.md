# postproc-spoken-punctuation - checks

Profile: light
Plan: none - checks-only change (one-sentence diff: the local rules turn spoken punctuation into marks); decisions delegated by the round brief of 2026-10-10 (Confirmed? y — delegado)

## Intent

With the LLM off, under the 15-word threshold, or in an app on the no-AI list, the text Fala inserts
comes only from `Rules` (`crates/postproc/src/rules.rs`), and `Rules` keeps "vírgula", "ponto
final", "nova linha" as words. Win+H and Wispr turn them into marks; a Brazilian user dictating in
an app without AI gets "Oi vírgula tudo bem ponto final". When this ships, `Rules` converts spoken
punctuation in pt-BR and English (the language comes from the transcript's `Language`), behind a
`spoken_punctuation: bool` field on `Rules`, on by default and reachable from `Postprocessor` by
`with_rules`. The LLM still receives the rules text, now with the marks already in place.

Decided here (delegated):

- **Pipeline order:** fillers → repeats → spoken punctuation → dictionary → join → first-letter
  capital. Fillers and repeats go first so "então hã vírgula" and "ponto ponto ponto" reach the
  rule clean; the dictionary goes after so a term next to a mark keeps the mark ("charge bee
  vírgula" → "ChargeBee,"). The dictionary now skips windows holding a token with no word (the
  dash, a lone parenthesis), so "itau travessão itau" never glues the dash to the term.
- **Exception heuristic** (the brief's "trigger alone between pauses", made concrete):
  1. **Whole words only:** the trigger is 1 to 3 adjacent tokens with no punctuation between them,
     compared without case and accent; hyphenated forms match ("ponto-e-vírgula"); a word that
     merely contains a trigger ("apontou", "dashboard") never matches. Longest trigger first.
  2. **Noun marker before:** no trigger fires when the word right before it (no punctuation
     between) is an article, demonstrative, possessive, quantifier or one of a short list of
     prepositions ("a vírgula", "o ponto", "no travessão", "os dois pontos", "com vírgula", "the
     comma", "a dash", "to dash"). The bare forms of rule 4 are also blocked by a short list of
     pre-nominal adjectives ("um bom ponto", "último ponto", "trial period", "long period"); the
     other triggers are not, so "primeiro ponto e vírgula segundo" still fires.
  3. **Word after:** "nova linha", "quebra de linha", "novo parágrafo" do not fire before
     de/do/da/dos/das; "new line", "new paragraph" do not fire before "of"; "dois pontos" does not
     fire before "percentuais", "percentual", "de", "do", "da", "acima", "abaixo" (not before
     articles: "segue dois pontos a pauta" is a colon).
  4. **Pause only for the bare forms:** "ponto" and "period" fire only at a pause - end of text,
     punctuation the ASR put right after them, or another trigger right after ("ponto nova
     linha"). "ponto de vista", "ponto de encontro", "ponto de partida" never fire because "de"
     is not a pause. Mid-sentence, the user says "ponto final" / "full stop".
- **Accepted false positives:** a sentence-final "ponto"/"period" after a word outside the marker
  list ("é ponto." would lose the word); "dois pontos" as a score followed by a verb, an adverb or
  nothing ("perdemos dois pontos hoje" → "Perdemos: hoje", "subiu dois pontos." → "Subiu:");
  "colon" as the organ without an article; "travessão"/"dash" as nouns after a word outside the
  list ("hit the crossbar" style phrases without article).
- **Accepted false negatives:** bare "ponto"/"period" mid-sentence; any trigger right after a
  marker word ("I said that period", "número com vírgula" stay as words).
- **"Início de frase"** in the brief is read as "the ASR already marked the boundary with a sign
  after the trigger"; a capitalized next word is not used as a signal (proper names, English "I").
- **Spacing:** `, . ? ! : ;` and the closing `)` and `"` glue to the previous word; `(` and the
  opening `"` glue to the next word; the dash is " — " (U+2014) between spaces; "nova linha" is
  `\n` and "novo parágrafo" `\n\n`, with no space around them. Quotes are straight `"`.
- **ASR punctuation:** the sign the ASR put after a trigger is dropped (the spoken mark replaces
  it); a spoken closing mark equal to the sign already ending the previous word is skipped, and a
  different one replaces it ("acho, vírgula" → "acho,"; "sim, ponto final" → "sim.").
- **Capital after:** the next word after a spoken `.`, `?`, `!`, new line or paragraph gets a
  capital. The capitalization rule that exists today only covers the first letter, so this one
  applies only after marks this rule inserts; the ASR's own signs keep today's behaviour.
- A trigger at the very start (nothing before it) still becomes its mark (a dictation of just
  "vírgula" inserts ","), so a short dictation can add punctuation to text already in the field.

16 checks in 2 slices · 0 one-way doors · 0 open

## Checks

Unit proofs: `CARGO_TARGET_DIR=/home/augusto/projects/fala/target cargo test -p fala-postproc --lib <path>`.
Integration proof: the same with `--test spoken_punctuation <name>`.

### S1 - rule · 1 file · 12 KB · ~3k

**C1** - Under `Language::PtBr`, each pt-BR trigger becomes its mark, asserted per input: "sim vírgula não" → "Sim, não"; "fim ponto" → "Fim."; "fim ponto final" → "Fim."; "tudo bem ponto de interrogação" → "Tudo bem?"; "que ótimo ponto de exclamação" → "Que ótimo!"; "lista dois pontos um" → "Lista: um"; "primeiro ponto e vírgula segundo" → "Primeiro; segundo"; "oi nova linha tchau" → "Oi\nTchau"; "oi quebra de linha tchau" → "Oi\nTchau"; "oi novo parágrafo tchau" → "Oi\n\nTchau"; "veja abre parênteses nota fecha parênteses aqui" and the singular "parêntese" → "Veja (nota) aqui"; "ele disse abre aspas oi fecha aspas" → "Ele disse \"oi\""; "isso travessão aquilo" → "Isso — aquilo"
Proof: `cargo test -p fala-postproc --lib rules::tests::spoken_pt_br_each_trigger`

**C2** - Under `Language::En`, each English trigger becomes its mark: "yes comma no" → "Yes, no"; "done period" → "Done."; "done full stop" → "Done."; "really question mark" → "Really?"; "great exclamation mark" and "great exclamation point" → "Great!"; "list colon one" → "List: one"; "first semicolon second" → "First; second"; "hi new line bye" and "hi newline bye" → "Hi\nBye"; "hi new paragraph bye" → "Hi\n\nBye"; "see open parenthesis note close parenthesis here" and "open paren … close paren" → "See (note) here"; "he said open quote hi close quote" and "open quotes … close quotes" → "He said \"hi\""; "wait dash that" → "Wait — that"
Proof: `cargo test -p fala-postproc --lib rules::tests::spoken_en_each_trigger`

**C3** - Spacing: "x vírgula b ponto final c ponto de interrogação d ponto de exclamação e dois pontos f ponto e vírgula g" → "X, b. C? D! E: f; g" (no space before a closing mark, exactly one after); "x nova linha y" → "X\nY" (no space around `\n`); "x abre parênteses y fecha parênteses z" → "X (y) z"; "x travessão y" → "X — y"
Proof: `cargo test -p fala-postproc --lib rules::tests::spoken_spacing`

**C4** - Capital after a spoken `.` `?` `!`, new line and paragraph ("sim ponto final não" → "Sim. Não"; "sério ponto de interrogação sim" → "Sério? Sim"; "uau ponto de exclamação sim" → "Uau! Sim"; "oi nova linha tchau" → "Oi\nTchau"; "oi novo parágrafo tchau" → "Oi\n\nTchau"), never after `,` `:` `;` ("sim vírgula não" → "Sim, não"; "lista dois pontos um" → "Lista: um"; "x ponto e vírgula b" → "X; b")
Proof: `cargo test -p fala-postproc --lib rules::tests::spoken_capitalizes_after_sentence_end`

**C5** - No doubled sign next to ASR punctuation: "eu acho, vírgula que sim" → "Eu acho, que sim"; "Tudo bem. Ponto final." → "Tudo bem."; "sim vírgula, não" → "Sim, não"; "sim, ponto final não" → "Sim. Não"; "sim vírgula vírgula não" → "Sim, não"; "Done. Period." → "Done."
Proof: `cargo test -p fala-postproc --lib rules::tests::spoken_does_not_double_asr_punctuation`

**C6** - Whole words only; these stay as dictated (first letter capitalized): "ele apontou isso", "isso é ponto-chave", "foi virgulado", "commas are fine", "periodically", "colonial era", "dashboard ready"
Proof: `cargo test -p fala-postproc --lib rules::tests::spoken_only_whole_words`

**C7** - pt-BR exceptions stay as dictated: "do meu ponto de vista está certo", "o ponto de encontro é aqui", "esse é o ponto de partida", "subiu dois pontos percentuais", "temos nova linha de produto", "a vírgula está errada", "esse é o ponto.", "foi um bom ponto.", "bateu no travessão", "ganhamos os dois pontos"
Proof: `cargo test -p fala-postproc --lib rules::tests::spoken_pt_br_exceptions`

**C8** - English exceptions stay as dictated: "it was a long period.", "the trial period.", "add a comma here", "new line of products", "a dash of salt", "I have to dash.", "the colon is an organ"
Proof: `cargo test -p fala-postproc --lib rules::tests::spoken_en_exceptions`

**C9** - Bare "ponto"/"period" fire only at a pause: "fim ponto" → "Fim."; "Fim ponto. Depois" → "Fim. Depois"; "fim ponto nova linha depois" → "Fim.\nDepois"; "vamos ponto depois" stays "Vamos ponto depois"; "we left period. Then" → "We left. Then"; "we left period new line then" → "We left.\nThen"; "we left period then" stays "We left period then"
Proof: `cargo test -p fala-postproc --lib rules::tests::spoken_bare_ponto_needs_pause`

**C10** - The language picks the table: under `En`, "sim vírgula não" → "Sim vírgula não"; under `PtBr`, "yes comma no" → "Yes comma no"
Proof: `cargo test -p fala-postproc --lib rules::tests::spoken_follows_language`

**C11** - `Rules::default().spoken_punctuation` is `true`; with `Rules { spoken_punctuation: false }` every C1 and C2 input comes out equal to its input with only the first letter capitalized
Proof: `cargo test -p fala-postproc --lib rules::tests::spoken_flag_off_and_default_on`

**C12** - Order with the existing rules: "então hã vírgula eu acho" → "Então, eu acho" (filler first); "fim ponto ponto ponto" → "Fim." (repeats first, no "..."); with the dictionary ["ChargeBee", "Itaú", "Fala Cloud Sync"], "charge bee vírgula itau ponto final" → "ChargeBee, Itaú." and "itau travessão itau" → "Itaú — Itaú" and "abre parênteses itau fecha parênteses" → "(Itaú)"; "nova linha oi" → "\nOi" (first-letter capital still applies)
Proof: `cargo test -p fala-postproc --lib rules::tests::spoken_order_with_other_rules`

**C13** - The 7 pre-existing rules tests pass with their assertions unmodified (only the helper that builds `Rules` changes): `removes_pt_br_fillers`, `en_keeps_pt_only_fillers`, `collapses_three_or_more_repeats`, `dictionary_replaces_spelling_variants`, `dictionary_does_not_join_across_punctuation`, `dictionary_ignores_partial_words`, `normalizes_spacing_and_capitalizes`
Proof: `cargo test -p fala-postproc --lib rules::tests`

### S2 - wiring · 2 files · 10 KB · ~3k

**C14** - `Postprocessor::new(LlmConfig::default())` turns "sim vírgula não" (pt-BR) into final text "Sim, não" with editor `Rules`; the same with `.with_rules(Rules { spoken_punctuation: false })` gives "Sim vírgula não"
Proof: `cargo test -p fala-postproc --test spoken_punctuation postprocessor_honors_the_flag`

**C15** - `cargo clippy -p fala-postproc --all-targets -- -D warnings` and `cargo fmt --all -- --check` exit 0
Proof: `CARGO_TARGET_DIR=/home/augusto/projects/fala/target cargo clippy -p fala-postproc --all-targets -- -D warnings && cargo fmt --all -- --check`

**C16** - Nothing outside `crates/postproc` and this spec folder changes: `git diff --name-only origin/main...HEAD` lists only paths under `crates/postproc/` and `.specs/features/postproc-spoken-punctuation/`
Proof: `test -z "$(git diff --name-only origin/main...HEAD | grep -v '^crates/postproc/' | grep -v '^.specs/features/postproc-spoken-punctuation/')"`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| pt-BR triggers (17) | `vírgula` C1 · `ponto` C1, C9 · `ponto final` C1 · `ponto de interrogação` C1 · `ponto de exclamação` C1 · `dois pontos` C1 · `ponto e vírgula` C1 · `nova linha` C1 · `quebra de linha` C1 · `novo parágrafo` C1 · `abre parênteses` C1 · `abre parêntese` C1 · `fecha parênteses` C1 · `fecha parêntese` C1 · `abre aspas` C1 · `fecha aspas` C1 · `travessão` C1 | - |
| English triggers (20) | `comma` C2 · `period` C2, C9 · `full stop` C2 · `question mark` C2 · `exclamation mark` C2 · `exclamation point` C2 · `colon` C2 · `semicolon` C2 · `new line` C2 · `newline` C2 · `new paragraph` C2 · `open parenthesis` C2 · `open paren` C2 · `close parenthesis` C2 · `close paren` C2 · `open quote` C2 · `open quotes` C2 · `close quote` C2 · `close quotes` C2 · `dash` C2 | - |
| brief exceptions (5) | `ponto de vista` C7 · `ponto de encontro` C7 · `ponto de partida` C7 · `dois pontos percentuais` C7 · `nova linha de produto` C7 | - |
| heuristic rules (4) | whole word C6 · marker before C7, C8 · word after C7, C8 · pause for bare forms C9 | - |
| pause kinds (3) | end of text C9 · ASR sign after C9 · trigger after C9 | - |
| mark kinds (6) | closing `, . ? ! : ;` C3 · open `(` `"` C1, C3 · close `)` `"` C1, C3 · dash C3 · new line C3 · paragraph C1, C4 | - |
| existing rules order (3) | fillers C12 · repeats C12 · dictionary C12 | - |
| flag states (2) | on (default) C11, C14 · off C11, C14 | - |
| languages (2) | `PtBr` C1, C10 · `En` C2, C10 | - |

- Every claim is an input → output pair asserted literally; the rule has no I/O and no state, and
  its only boundary is `Postprocessor::process` (C14).
- English "end quote" and the plural "parentheses" variants are accepted by the table but not
  asserted one by one (spellings of C2 members).

## Swept

- validation: C6, C7, C8, C9 (what is and is not a trigger)
- failure modes: n/a - pure function over a string, no error path (the `Formatter` stays `Ok`)
- idempotency: C5 (a sign already present is not doubled)
- authorization: n/a - local text transform
- concurrency: n/a - `Rules` is `Copy` and stateless
- data lifecycle: n/a - nothing persisted
- dependency failure: n/a - no dependency; the LLM path is untouched
- state transitions: n/a - no stateful entity
- observability: n/a - no new log; dictated content is never logged

## Out of scope

- A setting in `apps/desktop` or a `fala-cli format` flag for `spoken_punctuation`: the desktop has
  no rules settings yet and the CLI flag is not needed while the default is on; left for another PR.
- Spoken editing commands ("apaga isso", "send it"): C13 of the competitors report, separate pitch.
- Symbols beyond the listed set ("barra", "slash", "arroba", "hífen").

## Handoff

- S1-S2 touch `crates/postproc/src/rules.rs` (12 KB), `crates/postproc/src/lib.rs` (9 KB) and a new
  `crates/postproc/tests/spoken_punctuation.rs` ≈ 25 KB read + ≈ 20 KB written ≈ 12k, under the
  150k budget - one builder
- Mechanism: one builder (fits)
