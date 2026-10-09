# llm-auto checks

Profile: light
Plan: `.specs/features/llm-auto/plan.md`

18 checks in 3 slices · 4 one-way doors · 0 open, of which 0 block

## Checks

Prefixo de todo `cargo` abaixo, no Windows: `CARGO_TARGET_DIR=C:\f`. A lógica nova do desktop fica em `apps/desktop/src/llm_auto.rs`, com os testes ao lado; o servidor falso é um `TcpListener` em `127.0.0.1:0` que conta conexões, como o de `crates/postproc/tests/fake_gemini.rs`.

### S1 - o ditado passa pelo `Postprocessor`

**C1** - com o servidor falso respondendo um texto em 0,5 s e um ditado de 16 palavras, `llm_auto::format` devolve o texto do servidor, `llm_produced = true`, e o `EntryTexts` montado dele tem `post_processed_text = Some(texto do servidor)`; `dictation_for` com esse `llm_produced` dá `Editor::Llm` (AC 1, door 4)
Proof: `cargo test -p fala llm_auto::tests::llm_text_is_pasted_and_recorded_as_llm -- --exact`

**C2** - com o servidor falso no ar, 0 conexões para: 15 palavras depois das regras; `llm_enabled = false`; sem chave `gemini`. Cada caso devolve o texto das regras e `llm_produced = false` (AC 2)
Proof: `cargo test -p fala llm_auto::tests::no_request_below_threshold_disabled_or_without_key -- --exact`

**C3** - com `llm_disabled_apps = ["Code"]` e o app `code`, e com a lista padrão e o app `keepassxc`, 0 conexões e o texto das regras (AC 3)
Proof: `cargo test -p fala llm_auto::tests::no_request_for_disabled_app -- --exact`

**C4** - com `app_name = None` e 16 palavras, 1 conexão, o texto do servidor, e o corpo capturado não contém `</app>` (o `SYSTEM_PROMPT` cita `<app>`, então só a tag fechada distingue) (AC 4, D4)
Proof: `cargo test -p fala llm_auto::tests::unknown_app_still_uses_llm -- --exact`

**C5** - o servidor falso responde em 3 s, com HTTP 500 e com `{"candidates":[]}`: cada caso devolve o texto das regras em menos de 2,5 s, `llm_produced = false`, e o `EntryTexts` tem `post_processed_text = Some(texto das regras)` porque as regras mudaram o texto (AC 5)
Proof: `cargo test -p fala llm_auto::tests::slow_failed_or_invalid_llm_keeps_rules_text -- --exact`

**C6** - `TranscribeAction::stop` chama `foreground_app()` uma vez, fora do `spawn`, e o mesmo valor vai ao `llm_auto::format` e ao `NewEntry.app` (AC 6)
Proof: `grep -c "foreground_app()" apps/desktop/src/actions.rs` imprime `1`, e a linha está antes de `tauri::async_runtime::spawn(async move` em `stop`; leitura do diff confirma que `NewEntry.app` e `format` usam a mesma variável

**C7** - nenhum `log::`/`debug!`/`info!`/`warn!`/`error!` em `llm_auto.rs` recebe a chave ou o texto: o único valor formatado é o `Editor`; `scripts/check-*` e o job `secrets` passam (AC 7)
Proof: `grep -nE "(debug|info|warn|error)!" apps/desktop/src/llm_auto.rs` mostra só linhas sem `key`, `text` ou `expose`; `! grep -n "expose()" apps/desktop/src/llm_auto.rs apps/desktop/src/actions.rs`

**C18** - `retry_mode`: item do `transcribe` sem dictation → `RulesOnly`; com dictation em `keepassxc` → `Auto` com esse app; com dictation sem app → `Auto` com app desconhecido; item do atalho herdado → `Legacy`. `RulesOnly` desliga o LLM em `process_transcription_output` (`llm_enabled &= false`) (AC 8)
Proof: `cargo test -p fala commands::history::tests::retry_without_a_recorded_app_never_asks_the_llm -- --exact` e leitura do ramo `RulesOnly` em `actions.rs`

### S2 - settings e o segundo atalho

**C8** - um store JSON sem `llm_enabled` e `llm_disabled_apps` carrega `true` e `["1password", "bitwarden", "keepass", "keepassxc"]` (AC 8, door 1)
Proof: `cargo test -p fala settings::tests::store_without_llm_fields_gets_defaults -- --exact`

**C9** - um store de schema 2 com `transcribe = ctrl_left+space` e `transcribe_with_post_process` em `ctrl+space`/`ctrl+space` sai da migração com `transcribe` em `ctrl_left+space`, `transcribe_with_post_process` em `""`/`""` e schema 3 (AC 9, door 3)
Proof: `cargo test -p fala settings::tests::untouched_post_process_binding_is_unbound -- --exact`

**C10** - um store de schema 2 com `transcribe_with_post_process` em `alt+p` mantém `current_binding = "alt+p"` e fica com `default_binding = ""` (AC 10, door 3)
Proof: `cargo test -p fala settings::tests::chosen_post_process_binding_is_kept -- --exact`

**C11** - os defaults de fresh install: `transcribe` em `ctrl+shift+space`, `transcribe_with_post_process` em `""`/`""` (door 3; substitui o teste `default_bindings_put_dictation_on_ctrl_shift_space`)
Proof: `cargo test -p fala settings::tests::default_bindings_put_dictation_on_ctrl_shift_space -- --exact`

**C12** - `normalize_apps([" Chrome.exe ", "chrome", "", "C:\\Tools\\Slack.exe"])` devolve `["chrome", "slack"]` (AC 11)
Proof: `cargo test -p fala llm_auto::tests::disabled_apps_are_normalized -- --exact`

**C13** - os três pontos que registram atalhos (`shortcut/mod.rs` init e troca de implementação, `fala_keys.rs`, `tauri_impl.rs`) e o `secure_input.rs` pulam um binding vazio pela mesma função `binding_is_unset` (AC 12)
Proof: `cargo test -p fala shortcut::tests::empty_binding_is_unset -- --exact` e `grep -n "binding_is_unset" apps/desktop/src/shortcut/*.rs apps/desktop/src/secure_input.rs` com uma chamada em cada um dos quatro arquivos

**C14** - `default_post_process_providers()` contém `gemini` com o `base_url` da door 2, e um store sem ele ganha o provedor e a entrada vazia em `post_process_api_keys` na leitura (AC 13, door 2)
Proof: `cargo test -p fala settings::tests::gemini_provider_is_added_to_old_stores -- --exact`

### S3 - UI

**C15** - `bun run lint`, `bun run format:check` e `bun run check:translations` saem com 0; as chaves `settings.llm.*` existem em `pt` e `en` (AC 14)
Proof: `bun run lint && bun run format:check && bun run check:translations`

**C16** - em `bun run tauri dev` no Windows, o grupo "IA" aparece nos settings gerais; desligar o interruptor, digitar uma chave e editar a lista persistem depois de reabrir o app; com a chave e um ditado de mais de 15 palavras no Bloco de Notas, o texto entra formatado (AC 14, AC 1; `TODO(windows)`: manual)
Proof: manual, registrado no PR

### Gate

**C17** - `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p fala`, `cargo fmt --all -- --check`, `cargo deny check` e `scripts/check-no-tauri-in-crates.sh` saem com 0
Proof: os cinco comandos, em sequência

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| condições do LLM (4) | ligado C2 · chave C2 · app fora da lista C3 · > 15 palavras C2 | - |
| respostas do Gemini (4) | sucesso C1 · lento C5 · HTTP 500 C5 · corpo inválido C5 | - |
| app (3) | conhecido fora da lista C1 · na lista C3 · desconhecido C4 | - |
| one-way doors (4) | settings C8 · chave C14 · atalho C9, C10, C11 · quem editou C1, C5 | - |
| implementações de atalho (2) | Tauri C13 · `fala_keys` C13 | - |

- O manual C16 fica fora da tabela do Verifier: é um checklist da sessão com o Augusto, e o veredito automático não depende dele (L-004)
- Latência com LLM e W13: `TODO(windows)` do plano, sem check aqui

## Swept

- validation: C12
- failure modes: C5
- idempotency: C9 (a migração só roda com schema < 3)
- authorization: n/a - comandos locais da própria UI
- concurrency: n/a - o `Postprocessor` roda numa thread por ditado; o cancelamento herdado (`complete_unless_cancelled`) continua em volta
- data lifecycle: C8, C9, C10, C14
- dependency failure: C5 (rede e Gemini); keyring indisponível já é do `KEY_VAULT` (#35)
- state transitions: n/a
- observability: C7

## Handoff

O diff inteiro passou de 1.000 linhas (o limite do CI), então a feature vira três PRs empilhados, um por obrigação:

| PR | Branch | Checks | O que entra |
| --- | --- | --- | --- |
| 1 | `feat/llm-auto` | C1-C8, C14, C18 | `llm_auto.rs`, o caminho `Auto`/`Legacy` em `actions.rs`, `llm_produced` no histórico, os settings `llm_*` e o provedor `gemini`; a chave pode vir de `fala-cli key set gemini` |
| 2 | `feat/unbind-post-process-key` | C9-C11, C13 | D6: o default vazio, a migração do schema 3 e o atalho vazio não registrado |
| 3 | `feat/llm-settings-ui` | C12, C15, C16 | os dois comandos, `normalize_apps`, o grupo "IA", i18n e `bindings.ts`; o `verification.md` da feature inteira |

- **Settled mid-build:** C4 olha `</app>`, não `<app>`: o `SYSTEM_PROMPT` do crate cita `<app>`, então o primeiro teste falhou com o código certo. O `bindings.ts` foi gerado pelo binário de debug em modo portátil (`C:\f\debug\portable`), para não migrar o store do app instalado.
- **Settled after verification:** o Verifier achou que um item com falha de ASR, salvo sem dictation, perdia o app, e o retry chamava o LLM como app desconhecido; virou o AC 8 e o C18 (`OutputMode::RulesOnly`). O reset do atalho herdado com default vazio falhava; `reset_binding` e `change_binding` tratam o binding vazio (fatia 2).
- **Abandoned:** none
