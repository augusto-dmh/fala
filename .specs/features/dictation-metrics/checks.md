# dictation-metrics checks

Profile: light
Plan: `.specs/features/dictation-metrics/plan.md`

13 checks in 3 slices · 5 one-way doors · 0 open, of which 0 block

## Checks

Prefixo de todo `cargo` abaixo, no Windows: `CARGO_TARGET_DIR=C:\f\pipe`, um `cargo` por vez.

### S1 - métricas em `fala.sqlite`

**C1** - banco novo: `dictation_metrics` existe e `user_version` continua 1; um banco aberto antes desta mudança (tabela ausente, versão 1) ganha a tabela no próximo `open` sem perder itens (AC 1, door 1)
Proof: `cargo test -p fala-storage --test metrics metrics_table_is_additive -- --exact`

**C2** - `add_metrics` grava uma linha cujas colunas são exatamente `rowid, dictation_id, created_at, created_ms, e2e_ms, asr_ms, llm_ms, paste_ms, speech_ms, words, lang, llm_used, fallback, model, app`; apagar o item ligado deixa a linha (AC 2, AC 3)
Proof: `cargo test -p fala-storage --test metrics metrics_have_no_text_and_outlive_the_item -- --exact`

**C3** - 10 ditados sintéticos com `e2e_ms` 100..1000 sem LLM e 4 com LLM (1100, 1300, 2100 e um timeout de 2400 com `fallback = timeout`) em dois dias, mais 1 de 10 dias atrás: `metrics_summary(7)` dá 14 ditados, p50/p90 de `e2e` sem LLM = 500/900, com LLM = 1300/2400, `llm_attempts` 4, `fallbacks` 1, dois dias com as contagens certas; o de 10 dias fica de fora (AC 4)
Proof: `cargo test -p fala-storage --test metrics summary_percentiles_and_days -- --exact`

**C4** - banco vazio: `metrics_summary(7)` tem 0 ditados, nenhum dia e todos os percentis `None` (AC 5)
Proof: `cargo test -p fala-storage --test metrics empty_summary -- --exact`

### S2 - o desktop mede

**C5** - relógio: um carimbo de 200 ms atrás vale; um de 2 s atrás cede ao `now`; a soltura lê a última borda (aperto do toggle ou soltura do PTT) (AC 6, door 2)
Proof: `cargo test -p fala --lib dictation_metrics::tests::key_clock_uses_recent_stamps -- --exact`

**C6** - marcos sintéticos (tecla, pill +30 ms, soltar, ASR +200, LLM +900, colado +40) viram `DictationMetrics` com `e2e_ms` 1140, `asr_ms`, `llm_ms` 900, `paste_ms` 40, e a linha de trace tem exatamente as chaves da door 4 na ordem, `key_to_pill_ms=30`, `release_to_text_ms=1140`, `fallback=none`; sem LLM tentado, `llm_ms` é `None` e `release_to_llm_ms` = `release_to_asr_ms` (AC 7, AC 8)
Proof: `cargo test -p fala --lib dictation_metrics::tests::marks_become_metrics_and_trace -- --exact`

**C7** - a linha de trace nunca contém o texto: `Measured` não tem campo de texto (só `words: usize`), e `trace_line` formata só números, `lang`, `llm_used` e `fallback`; sem `FALA_TRACE=1`, `finish` não chama `log::info!` (AC 8)
Proof: leitura de `apps/desktop/src/dictation_metrics.rs` (`Measured`, `finish`, `trace_line`); o formato exato é travado por C6

**C8** - `llm_auto::format` copia o `fallback` do `Formatted`: o caso "3 s" do teste existente dá `Some(Fallback::Timeout)` e o caso "http 500" dá `Some(Fallback::Http(500))`; abaixo de 16 palavras, `None` (AC 9)
Proof: `cargo test -p fala --lib llm_auto::tests -- `

### S3 - leitura

**C9** - `fala-cli history stats --days 7` num banco com as linhas de C3 imprime as linhas `ditados: 14`, `soltar → texto (sem LLM): p50 500 ms · p90 900 ms (meta 700 ms, máx. 1000 ms)` e `com LLM: p50 1300 ms · p90 2400 ms (meta 1200 ms, máx. 2000 ms)` e sai 0; num banco vazio, `ditados: 0` e sai 0 (AC 10)
Proof: `cargo test -p fala-cli --test history stats_prints_summary -- --exact`

**C10** - o comando `get_dictation_stats` está registrado no `collect_commands!` e chama `HistoryManager::metrics_summary` (AC 11)
Proof: `grep -n "get_dictation_stats" apps/desktop/src/lib.rs apps/desktop/src/commands/history.rs` (2 linhas, uma em cada) e leitura

**C11** - no Windows, na tomada (`Win32_Battery.BatteryStatus = 2`) ou marcado "sob carga": `bun run tauri dev` com store portátil e `FALA_TRACE=1`; 10 ditados curtos no Bloco de Notas com `ctrl+shift+space` deixam 10 linhas `event=dictation` no `fala.log` e 10 linhas novas em `dictation_metrics`; com chave Gemini, 5 ditados de > 15 palavras. p50/p90 de cada marco no `verification.md` contra o orçamento (AC 12; manual, registrado como linha Unproven se não rodar)
Proof: manual, com as linhas do log e `fala-cli history stats --data-dir <pasta portátil>`

### Gate

**C12** - `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p fala --lib`, `cargo test -p fala-storage`, `cargo test -p fala-cli`, `scripts/check-no-tauri-in-crates.sh`, `scripts/check-brand.sh` saem com 0
Proof: os comandos, em sequência

**C13** - nenhum texto ditado chega à tabela nem ao trace: `dictation_metrics.rs` não lê `final_text`/`transcription` além de contar palavras, e o `INSERT` de `add_metrics` não tem coluna de texto livre além de `lang`, `model`, `app`, `fallback` (AC 2, AC 8)
Proof: leitura de `apps/desktop/src/dictation_metrics.rs` e `crates/storage/src/metrics.rs`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| marcos da door 4 (4) | tecla→pill C6 · soltar→ASR C6 · →LLM C6 · →colado C6 | valores reais: C11 |
| desfechos do LLM (3) | não tentado C6/C8 · usado C3 · fallback C3/C8 | - |
| leitores do resumo (2) | CLI C9 · Tauri C10 | página da UI: fora do escopo |
