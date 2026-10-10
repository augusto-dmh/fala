# dictionary checks

Profile: light
Plan: `.specs/features/dictionary/plan.md`

14 checks in 2 slices · 2 one-way doors · 0 open, of which 0 block

## Checks

Prefixo de todo `cargo` abaixo, no Windows: `CARGO_TARGET_DIR=C:\f\dict`. A lógica nova fica em `apps/desktop/src/llm_auto.rs`, com os testes ao lado e o mesmo `FakeGemini` da F7a (um `TcpListener` em `127.0.0.1:0` que guarda os corpos).

### S1 - o dicionário chega ao LLM e às regras

**C1** - com `custom_words = [" Augusto ", "augusto", "ChargeBee"]`, a chave e 16 palavras, o servidor falso recebe 1 request cujo `systemInstruction.parts[0].text` é `SYSTEM_PROMPT + "\n\nDicionário pessoal:\n- Augusto\n- ChargeBee"` (AC 1, door 1)
Proof: `cargo test -p fala --lib llm_auto::tests::dictionary_reaches_the_llm_prompt -- --exact`

**C2** - com `custom_words = []` e com `["", "  "]`, o `systemInstruction` capturado é exatamente `SYSTEM_PROMPT` (AC 2)
Proof: `cargo test -p fala --lib llm_auto::tests::empty_dictionary_keeps_the_prompt -- --exact`

**C3** - com `custom_words = ["ChargeBee"]` e o servidor falso no ar, `a charge bee mandou ...` vira `A ChargeBee mandou ...` com o LLM desligado e sem chave; com o LLM configurado, no app `keepassxc` e com 15 palavras, o texto é igual ao do pipeline sem LLM (fuzzy e regras) e traz `ChargeBee mandou`; 0 requests (AC 3)
Proof: `cargo test -p fala --lib llm_auto::tests::rules_apply_the_dictionary_without_llm -- --exact`

**C4** - `post_process_transcription_text` com `custom_words = ["Augusto"]`: com a chave `gemini` e `llm_enabled`, `agusto` fica `agusto`; sem a chave, ou com `llm_enabled = false`, vira `Augusto` (AC 4, AC 6, door 2)
Proof: `cargo test -p fala --lib managers::transcription::tests::fuzzy_waits_for_the_llm_while_it_is_configured -- --exact`

**C5** - com o LLM configurado e `custom_words = ["Augusto"]`: o servidor falso responde `O agusto mandou o relatório.` a um ditado de 16 palavras com `agusto`; o corpo traz `agusto` e o texto colado é a resposta sem mudança, `llm_produced = true` (AC 4)
Proof: `cargo test -p fala --lib llm_auto::tests::llm_text_is_not_fuzzy_corrected -- --exact`

**C6** - com o LLM configurado e `custom_words = ["Augusto"]`, `agusto` vira `Augusto` em: 4 palavras (0 requests), app `keepassxc` (0 requests) e 16 palavras com HTTP 500 (1 request) (AC 5)
Proof: `cargo test -p fala --lib llm_auto::tests::deferred_fuzzy_fixes_text_the_llm_did_not_write -- --exact`

**C7** - sem a chave, `llm_auto::formatter` não adia a fuzzy (`Fuzzy::deferred` é `None`), e `agusto mandou` sai `Agusto mandou` (a fuzzy já rodou no `TranscriptionManager`, ver C4) (AC 6)
Proof: `cargo test -p fala --lib llm_auto::tests::no_second_fuzzy_without_llm -- --exact`

**C13** - com o LLM configurado e `custom_words = ["ChargeBee", "Augusto"]`, `a charge bee mandou isso`, `o augusto mandou isso` e `o agusto mandou isso` saem iguais ao pipeline sem LLM (`apply_custom_words` sobre o texto do ASR e depois as regras) e sem `CHARGEBEE` nem `AUGUSTO`; 0 requests (AC 5; achado 1 do Verifier, round 1)
Proof: `cargo test -p fala --lib llm_auto::tests::deferred_fuzzy_matches_the_pipeline_without_llm -- --exact`

**C14** - `formatter_at(settings, false, ..)` (o retry `RulesOnly`) com o LLM configurado: 0 requests e `agusto` vira `Augusto` (AC 5; achado 3 do Verifier, round 1)
Proof: `cargo test -p fala --lib llm_auto::tests::rules_only_retry_keeps_the_deferred_fuzzy -- --exact`

**C8** - `legacy_post_process` com o LLM configurado, `custom_words = ["Augusto"]` e o provedor herdado sem configurar devolve `Augusto mandou` para `agusto mandou` (AC 7)
Proof: `cargo test -p fala --lib actions::tests::legacy_keeps_the_fuzzy_while_llm_is_configured -- --exact`

### S2 - a lista gravada e a UI

**C9** - `llm_auto::normalize_words([" Fala ", "fala", "", "ChargeBee", "  "])` devolve `["Fala", "ChargeBee"]`, e `update_custom_words` grava por ela (AC 8, door 1)
Proof: `cargo test -p fala --lib llm_auto::tests::custom_words_are_normalized -- --exact` e `grep -n "normalize_words" apps/desktop/src/shortcut/mod.rs`

**C10** - `CustomWords.tsx` compara duplicata com `toLowerCase()`; `settings.advanced.customWords.title` e `.description` mudaram em `pt` e `en`; `bun run lint`, `bun run format:check` e `bun run check:translations` saem com 0 (AC 9, AC 10)
Proof: os três comandos e `git diff main -- src/` lido

**C11** - em `bun run tauri dev` no Windows, adicionar `augusto` quando `Augusto` existe mostra o toast; um ditado de mais de 15 palavras com um termo da lista sai com a grafia do dicionário (AC 9, AC 1; `TODO(windows)`: manual)
Proof: manual, registrado no PR

### Gate

**C12** - `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p fala --lib`, `cargo test -p fala-postproc` e `scripts/check-brand.sh` saem com 0
Proof: os cinco comandos, em sequência

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| estados do dicionário (3) | com termos C1 · vazio C2 · só brancos C2 | - |
| por que o LLM não escreveu (6) | desligado C3 · sem chave C3, C7 · app na lista C3, C6 · ≤ 15 palavras C3, C6 · HTTP 500 C6 · timeout: mesmo ramo `llm_produced = false` do C6 | timeout sem teste próprio (o ramo é o mesmo do HTTP 500) |
| onde a fuzzy roda (5) | `TranscriptionManager` sem LLM C4 · pulada com LLM C4 · adiada C6, C13 · `RulesOnly` C14 · `Legacy` C8 | - |
| one-way doors (2) | store C1, C9 · fuzzy C4-C8 | - |

- O manual C11 fica fora da tabela do Verifier (L-004).

## Swept

- validation: C9, C2
- concorrência de settings: o `TranscriptionManager` e o `AutoFormatter` leem o settings em dois momentos; trocar o LLM ou a chave no meio de um ditado pode fazer a fuzzy rodar duas vezes ou nenhuma naquele ditado. Aceito (achado 2 do Verifier, round 1)
- failure modes: C6 (HTTP 500)
- idempotency: C7 (a fuzzy não roda duas vezes)
- authorization: n/a - comando local da própria UI
- concurrency: n/a - o `AutoFormatter` é montado por ditado, como o `Postprocessor` da F7a
- data lifecycle: C9 (store antigo com duplicata: normalizado no uso, C1)
- dependency failure: C6
- state transitions: n/a
- observability: um log novo, `error!("Custom-word correction panicked; keeping the text")` em `llm_auto.rs`, sem texto, termo nem chave; `grep -nE "(debug|info|warn|error)!" apps/desktop/src/llm_auto.rs` mostra só ele e o `editor`
