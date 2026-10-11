# dictionary-replacements checks

Profile: light
Plan: `.specs/features/dictionary-replacements/plan.md`

12 checks in 2 slices · 3 one-way doors · 0 open, of which 0 block

## Checks

Prefixo de todo `cargo` abaixo: `CARGO_TARGET_DIR=C:\f\dictionary-replacements`.

### S1 - o motor e o dicionário

**C1** - par troca o falado com outra caixa, acento ou hífen e mantém a pontuação em volta (AC 1)
Proof: `cargo test -p fala-postproc rules::tests::replacement_matches_spelling_variants -- --exact`

**C2** - par não casa dentro de outra palavra nem atravessa pontuação (AC 2)
Proof: `cargo test -p fala-postproc rules::tests::replacement_only_whole_words -- --exact`

**C3** - par vence termo na mesma posição e o escrito não é relido (AC 3, door 2)
Proof: `cargo test -p fala-postproc rules::tests::replacement_wins_and_is_not_reread -- --exact`

**C4** - com o LLM desligado o texto final tem o escrito; com o servidor falso, o corpo tem o escrito no texto e no dicionário e não tem o falado (AC 4)
Proof: `cargo test -p fala-postproc --test fake_gemini replacement_reaches_llm_as_written -- --exact` e `cargo test -p fala-postproc --test fake_gemini replacement_applies_without_llm -- --exact`

**C5** - `Dictionary`: par vazio e falado repetido somem, o escrito entra em `terms()`, JSON antigo carrega sem pares e o novo faz round-trip (AC 5)
Proof: `cargo test -p fala-core dictionary`

**C6** - `Formatted.replaced` tem um índice por ocorrência (AC 6)
Proof: `cargo test -p fala-postproc --test fake_gemini replacement_applies_without_llm -- --exact` (asserta `replaced == [0, 0]` para duas ocorrências)

**C7** - os crates inteiros continuam verdes
Proof: `cargo test -p fala-core -p fala-postproc -p fala-cli`

### S2 - store, contagem e página

**C8** - store v0.9.0 sem o campo: `custom_words == ["Fala", "cjpais"]` e pares vazios (AC 7)
Proof: `cargo test -p fala --lib settings::tests::frozen_v0_9_store_parses_strictly_then_migrates_device_index -- --exact`

**C9** - normalização do comando: trim, vazio fora, falado repetido fora, `uses` do store mantido pelo falado sem caixa, novo com 0 (AC 8, door 3)
Proof: `cargo test -p fala --lib dictionary::tests::normalize_keeps_stored_uses -- --exact`

**C10** - `record_uses` soma uma por ocorrência e ignora falado que sumiu; o formatter do desktop leva os pares ao `Dictionary` e devolve os falados aplicados (AC 9)
Proof: `cargo test -p fala --lib dictionary::tests` e `cargo test -p fala --lib llm_auto::tests::replacements_are_applied_and_reported -- --exact`

**C11** - página Dicionário: busca, termos e pares com contagem; strings só por i18next em `pt` e `en` (AC 10)
Proof: `bun run lint`; `bun run check:translations`; `bun run format:check`; `bun test src/components/shell.test.tsx` (asserta `DictionaryReplacements` na página); manual: roteiro no PR

**C12** - clippy, fmt e scripts do CI limpos
Proof: `cargo clippy --workspace --all-targets -- -D warnings`; `cargo fmt --all -- --check`; `scripts/check-no-tauri-in-crates.sh`; `scripts/check-brand.sh`
