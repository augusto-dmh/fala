# dictation-metrics verification

**Verdict**: PASS nos checks automáticos (C1-C10, C12, C13); C11 (manual, Windows) UNPROVEN
**Profile**: light
**Diff range**: ae6a51f..3b18520 (fc53c04, 3b18520); HEAD do código `3b18520`. O gate rodou em `4b5d29f`; `3b18520` é o mesmo Rust mais as 24 linhas de `src/bindings.ts` que o `tauri dev` gerou para `get_dictation_stats` (checadas pelo eslint/prettier do pre-commit)
**Verifier**: o próprio executor (author == verifier), retomando o trabalho de um painel pausado no Windows 11 (Alienware 16). Não houve Verifier independente nesta rodada; quem revisar o PR deve tratar esta tabela como afirmação do autor com as provas citadas.

Todo `cargo` rodou em `C:\dev\fala\.houston\worktrees\dictation-metrics`, com `CARGO_TARGET_DIR=C:\f\pipe`, um por vez (trava `C:\dev\fala\.houston\cargo-lock.txt`).

## Condição da máquina

- Na tomada: `(Get-CimInstance Win32_Battery).BatteryStatus` = `2`.
- **Sob carga**: outros dois painéis compilavam no mesmo Windows durante a rodada, e o Fala instalado (`C:\Users\augus\AppData\Local\Fala\fala.exe`) estava aberto.

## Checks

| Check | Proof run | Evidence | Result |
| --- | --- | --- | --- |
| C1 | `cargo test --workspace` → `tests\metrics.rs`: `metrics_table_is_additive ... ok` | `crates/storage/src/metrics.rs:14` cria a tabela com `CREATE TABLE IF NOT EXISTS`, fora do bloco de `user_version` | PASS |
| C2 | `metrics_have_no_text_and_outlive_the_item ... ok` | `INSERT` em `crates/storage/src/metrics.rs:96-98`: colunas `dictation_id, created_at, created_ms, e2e_ms, asr_ms, llm_ms, paste_ms, speech_ms, words, lang, llm_used, fallback, model, app` | PASS |
| C3 | `summary_percentiles_and_days ... ok` | 14 ditados, p50/p90 500/900 sem LLM e 1300/2400 com LLM, como o teste afirma | PASS |
| C4 | `empty_summary ... ok` | banco vazio: 0 ditados, percentis `None` | PASS |
| C5 | `dictation_metrics::tests::key_clock_uses_recent_stamps ... ok` | `apps/desktop/src/dictation_metrics.rs:207-228`: 200 ms vale, 2 s cede ao `now`, soltura = última borda (PTT e toggle), corte de sessão = `now` | PASS |
| C6 | `dictation_metrics::tests::marks_become_metrics_and_trace ... ok` | `:231-269`: `e2e_ms` 1140, `asr_ms` 190, `llm_ms` 900, `paste_ms` 40, linha de trace exata com `key_to_pill_ms=30`, `fallback=none`; sem LLM, `llm_ms=-`. Sem LLM, `release_to_llm_ms` é o fim do formatador de regras (201 no fixture, 1 ms depois do ASR), não exatamente igual a `release_to_asr_ms` como o texto de C6 diz | PASS |
| C7 | leitura (o teste `trace_has_no_text` foi removido; C7 foi corrigido em `checks.md`) | `Measured` (`:84-101`) não tem campo de texto, só `words: usize`; `trace_line` (`:154-174`) formata números, `lang`, `llm_used` e `fallback`; `finish` (`:118-125`) só chama `log::info!` com `FALA_TRACE=1` | PASS |
| C8 | `llm_auto::tests` (6 testes) `... ok` | `slow_failed_or_invalid_llm_keeps_rules_text` e `no_request_below_threshold_disabled_or_without_key` cobrem `Some(Timeout)`, `Some(Http(500))` e `None` | PASS |
| C9 | `tests\history.rs`: `stats_prints_summary ... ok` (14 passed no binário) | imprime as linhas do resumo e sai 0, também em banco vazio | PASS |
| C10 | `grep -n "get_dictation_stats" apps/desktop/src/lib.rs apps/desktop/src/commands/history.rs` | 2 linhas: `lib.rs:789` (`collect_commands!`) e `commands/history.rs:222`; o corpo chama `history_manager.metrics_summary(days)` | PASS |
| C11 | `bun run tauri dev` com `FALA_TRACE=1` | não rodou; ver abaixo | UNPROVEN |
| C12 | `cargo fmt --all -- --check` (0), `cargo clippy --workspace --all-targets -- -D warnings` (0), `cargo test --workspace` (0), `scripts/check-no-tauri-in-crates.sh` (0, `ok: no tauri in crates/`), `scripts/check-brand.sh` (0) | `cargo test --workspace`: 715 passed, 0 failed, 48 ignored; `fala` lib 357, `fala-storage` `metrics` 4, `fala-cli` `history` 14 | PASS |
| C13 | leitura de `apps/desktop/src/dictation_metrics.rs` e `crates/storage/src/metrics.rs` | o texto só entra como contagem de palavras; o `INSERT` não tem coluna de texto livre além de `lang`, `model`, `app`, `fallback` | PASS |

## C11: por que não rodou

`$env:CARGO_TARGET_DIR='C:\f\pipe'; $env:FALA_TRACE='1'; bun run tauri dev` compilou (`Finished dev profile ... in 53.93s`, `Running C:\f\pipe\debug\fala.exe`, `[portable] data dir: C:\f\pipe\debug\Data`) e o processo saiu com código 0 logo depois, sem escrever nada em `C:\f\pipe\debug\Data\logs\fala.log` (última escrita ainda a da rodada do F9a, 2026-10-09 20:50). O Fala instalado (pid 22676) continuou aberto: o `tauri_plugin_single_instance` (`apps/desktop/src/lib.rs:878`) encaminhou o lançamento para ele e encerrou a instância de dev. O brief proíbe fechar o Fala instalado, então os 10 ditados, as 10 linhas `event=dictation` e as 10 linhas em `dictation_metrics` não foram produzidos.

A tentativa anterior (2026-10-09) falhou antes, no link final do `fala` (`link.exe` erro 1), com outros painéis compilando; desta vez o link passou.

## Orçamento de latência

| Marco | Orçamento (`ARCHITECTURE.md`) | p50 | p90 |
| --- | --- | --- | --- |
| tecla → pill | ≤ 50 ms (máx. 100 ms) | não medido | não medido |
| soltar → texto, sem LLM | ≤ 700 ms (máx. 1 s) | não medido | não medido |
| soltar → texto, com LLM | ≤ 1,2 s (máx. 2 s) | não medido | não medido (sem chave Gemini no keyring) |
| ASR, colagem | - | não medido | não medido |

Para fechar C11: com o Fala instalado fechado (ou com a instância única desligada no build de dev), rodar o app de dev como acima, mandar 10 frases com o driver `dictate.ps1` do F9a, e anotar aqui `Select-String C:\f\pipe\debug\Data\logs\fala.log -Pattern 'event=dictation'` e `C:\f\pipe\debug\fala-cli.exe history stats --data-dir C:\f\pipe\debug\Data`, na tomada e sem outro build rodando. A medição de tecla → pill deve ser repetida num build de release, como a verificação do F9a pediu.

## Fora do escopo

- Página "Como estou indo" na UI: o comando `get_dictation_stats` está pronto, a página não.
- `event=load` no desktop e `numWordsCorrected`.
