# snippets checks

Profile: light
Plan: `.specs/features/snippets/plan.md`

10 checks in 2 slices · 3 one-way doors · 0 open, of which 0 block

## Checks

Prefixo de todo `cargo` abaixo: `CARGO_TARGET_DIR=C:\f\snippets`.

### S1 - o motor (PR 1: `fala-core`, `fala-postproc`, `fala-cli format`)

**C1** - o gatilho vira `⟦Sn⟧` por ocorrência, sem caixa/acento, e o texto das regras tem a expansão; não casa através de pontuação nem com parte de palavra (AC 1)
Proof: `cargo test -p fala-postproc rules::tests::snippet` (2 testes)

**C2** - ditado que é só o gatilho vira exatamente a expansão e não faz request (AC 2)
Proof: `cargo test -p fala-postproc rules::tests::whole_dictation_is_the_text_as_is -- --exact` e `cargo test -p fala-postproc --test fake_gemini snippet_without_llm_and_whole_dictation -- --exact`

**C3** - o corpo da request tem o marcador e a linha dos marcadores no prompt, e nunca o gatilho nem a expansão (AC 3, invariante ADR-0004)
Proof: `cargo test -p fala-postproc --test fake_gemini snippet_reaches_the_llm_only_as_a_marker -- --exact`

**C4** - resposta com marcador faltando, repetido ou alterado fica com o texto das regras e `InvalidResponse`; a tardia também (AC 4, door 3)
Proof: `cargo test -p fala-postproc rules::tests::restore_needs_every_marker_once -- --exact`, `--test fake_gemini altered_marker_keeps_the_rules_text` e `--test fake_gemini late_answer_gets_the_snippet_back`

**C5** - `Snippet` exige duas palavras e texto; gatilho repetido some; snippets fora de `terms()` e do JSON (AC 5)
Proof: `cargo test -p fala-core snippet` e `cargo test -p fala-core dictionary::tests::replacements_round_trip -- --exact`

**C6** - `{data}`, `{data_extenso}`, `{hora}` (AC 6)
Proof: `cargo test -p fala-core snippet::tests::renders_the_variables -- --exact`

**C7** - sem UI: `fala-cli format --dictionary` com `gatilho => texto` insere o texto
Proof: `cargo test -p fala-cli format::tests::prints_final_text_and_editor -- --exact`

### S2 - o app (PR 2: store, comando, contagem, página)

**C8** - normalização com duas palavras e `uses` do store; variáveis resolvidas no ditado e fora dos termos; contagem por gatilho (AC 7)
Proof: `cargo test -p fala --lib dictionary` (6 testes)

**C9** - no desktop: snippet aplicado com o LLM configurado (fuzzy adiada + regras de novo) e ditado inteiro sem request (AC 1, 2, 7)
Proof: `cargo test -p fala --lib llm_auto::tests::snippets_are_applied_and_reported_with_or_without_the_llm -- --exact`

**C10** - página Dicionário com a lista de snippets, busca e validação de duas palavras, strings em pt e en (AC 8)
Proof: `bun run lint`, `bunx tsc --noEmit`, `bun run check:translations`, `bun test src/components/shell.test.tsx`
