# dictation-metrics (F9c: `FALA_TRACE` e métricas por ditado no desktop)

## Problem

O orçamento de latência do `ARCHITECTURE.md` (tecla → pill ≤ 50/100 ms, soltar → texto ≤ 700 ms / 1 s, com LLM ≤ 1,2 / 2 s) só é medido headless: o `fala-cli dictate` emite o trace `FALA_TRACE` (doors 3 e 4 do `pipeline-headless`), o desktop não emite nada comparável e não guarda nenhum número por ditado. O inventário de 2026-10-09 (§ 2, § 4 item 5) marca "latência no orçamento" sem número para "com LLM" nem para tecla → pill no build novo.

Quando isto entra, cada ditado do desktop marca os instantes tecla, pill, soltar, fim do ASR, fim do LLM e texto colado; com `FALA_TRACE=1` sai uma linha `event=dictation` no formato `chave=valor` do CLI, e sempre uma linha de métricas sem texto vai para `fala.sqlite`. `fala-storage` resume p50/p90 por campo e `fala-cli history stats` imprime o resumo.

## Flow

Reusa o `TranscriptionCoordinator` (exists), `TranscribeAction::start/stop` (exists), `process_transcription_output` + `llm_auto::format` (exists), `deliver_unless_cancelled` (exists), `HistoryManager::save_entry` (exists), o `Store` (exists) e o `Trace` do CLI só como formato.

1. `TranscriptionCoordinator::send` carimba `Instant::now()` na thread de quem chamou (o hook do teclado, ou o sinal/flag externo), antes do canal e do `AudioRecordingManager` (door 2): aperto → `pressed`, toda borda → `last_edge`
2. `TranscribeAction::start` lê o aperto e mede até a volta de `show_recording_overlay` (door 3); guarda os dois para o `stop`
3. `stop` lê a última borda como "soltar"; a tarefa assíncrona marca o fim do ASR, o fim do formatador (com `llm_used` e `fallback` do `Formatted`), e o fim da colagem no começo do `save_history` (que roda logo depois do `paste`)
4. `dictation_metrics::finish` monta a linha de métricas (`fala_storage::DictationMetrics`, sem texto) e, com `FALA_TRACE=1`, escreve `event=dictation ...` com target `fala_trace` em `info`
5. `HistoryManager::record_metrics` grava a linha com o id do item que o `save_entry` devolveu (ou sem id quando o item não foi salvo)
6. `fala-cli history stats [--days N]` abre o `Store` e imprime `metrics_summary(N)`; o comando Tauri `get_dictation_stats(days)` devolve o mesmo resumo para a UI

## Impact

| Front | What changes |
| --- | --- |
| stored data | tabela nova `dictation_metrics` em `fala.sqlite`, criada com `CREATE TABLE IF NOT EXISTS` em todo `open`; `user_version` não muda (door 1) |
| domain | o desktop passa a medir e guardar por ditado `e2e_ms`, `asr_ms`, `llm_ms`, `paste_ms`, `speech_ms`, `words`, `lang`, `llm_used`, `fallback`, `model`, `app`; nada de texto |
| logs | com `FALA_TRACE=1`, uma linha `info` por ditado com target `fala_trace`; sem a variável, nada novo acima de `debug` |
| platform | nenhuma; o relógio é `std::time::Instant` |

## Surface

| Surface | Change |
| --- | --- |
| `fala_storage::DictationMetrics`, `MetricsSummary`, `Percentiles`, `DayCount` | novos |
| `Store::add_metrics(&DictationMetrics)`, `Store::metrics_summary(days)` | novos |
| `fala-cli history stats [--days N]` | novo; padrão 7 dias |
| comando Tauri `get_dictation_stats(days)` | novo; devolve o `MetricsSummary` |
| `llm_auto::AutoFormatted` | ganha `fallback: Option<Fallback>` |

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. formato da tabela | tabela própria `dictation_metrics(rowid, dictation_id TEXT NULL, created_at TEXT, created_ms INTEGER, e2e_ms, asr_ms, llm_ms NULL, paste_ms, speech_ms, words INTEGER, lang TEXT, llm_used 0/1, fallback TEXT NULL, model TEXT, app TEXT NULL)` + índice por `created_ms`, criada com `IF NOT EXISTS` fora do bloco de versão; `dictation_id` sem FK: a métrica sobrevive ao item (retenção, apagar) porque não tem texto | colunas em `dictations`: a retenção do histórico (5 itens por padrão) apagaria as métricas junto e o `reindex` as perderia; subir `user_version`: colide com a versão 2 do #62 (`auto_vacuum`) |
| 2. onde o relógio "tecla" começa | `TranscriptionCoordinator::send`, na thread do hook, antes do canal e do `AudioRecordingManager`; aperto e borda valem só se tiverem até 1 s quando a ação os lê, senão vale o `now` da ação (corte do limite de sessão, aperto lembrado durante o processamento) | carimbar dentro da thread do coordinator: perde a fila do canal; levar o `Instant` no `InputEvent` e nos `Effect`: mexe em ~40 testes do coordinator |
| 3. o que é "pill visível" | a volta de `show_recording_overlay`/`show_streaming_overlay` (janela mostrada, antes da pintura do WebView) | evento da UI depois do paint: ida e volta pelo IPC, outra fatia |
| 4. formato do trace | `event=dictation dictation=<n> key_to_pill_ms=<int> release_to_asr_ms=<int> release_to_llm_ms=<int> release_to_text_ms=<int> asr_ms=<int> llm_ms=<int> paste_ms=<int> speech_ms=<int> words=<int> lang=<tag> llm_used=<0\|1> fallback=<none\|timeout\|http\|network\|invalid>`, target `fala_trace`, `info`, só com `FALA_TRACE=1` (a door 3 do `pipeline-headless`, chaves do desktop) | JSON ou as chaves do CLI: `flush_ms`/`tail_asr_ms` não existem no desktop |
| 5. o que entra em "com LLM" | ditados em que o LLM foi tentado (`llm_used = 1` ou `fallback` não nulo); `e2e_ms` = soltar → fim do `paste`; `llm_ms` nulo quando não tentado | `llm_used` só: esconderia os timeouts de 2 s, que são o pior caso |

- Percentil por posto mais próximo (`ceil(q·n)`-ésimo valor ordenado), calculado em Rust sobre os valores do período; SQLite não tem percentil.
- Dia de "ditados por dia" é a data local do `created_at` (o offset de quem ditou), como no espelho.

## Criteria

### S1: métricas em `fala.sqlite` (P1)

1. The `Store::open` SHALL criar `dictation_metrics` num banco novo e num banco existente sem mudar `user_version`
2. WHEN `add_metrics` grava uma linha THEN ela SHALL ter só números, idioma, modelo, app, `fallback` e o id do item, nunca texto
3. WHEN o item ligado é apagado THEN a linha de métricas SHALL continuar
4. The `metrics_summary(days)` SHALL devolver, para os ditados dos últimos `days` dias, contagem, palavras, ditados com LLM tentado, fallbacks, contagem e palavras por dia, e p50/p90 de `e2e` sem LLM, `e2e` com LLM, `asr`, `llm`, `paste` e `speech`
5. WHEN não há ditados no período THEN o resumo SHALL ter contagens 0 e percentis vazios

### S2: o desktop mede (P1)

6. The relógio de tecla SHALL usar o carimbo do `send` quando ele tem até 1 s, senão o instante da ação
7. WHEN o ditado termina com texto colado THEN o desktop SHALL gravar uma linha de métricas, com `FALA_TRACE` ou sem
8. WHERE `FALA_TRACE=1`, o desktop SHALL escrever uma linha `event=dictation` no formato da door 4, sem nenhuma palavra do ditado; sem a variável, nenhuma
9. The `fallback` SHALL vir do `Formatted::fallback` do `fala-postproc`

### S3: leitura (P1)

10. `fala-cli history stats --days N` SHALL imprimir o resumo do item 4 e sair 0, também num banco vazio
11. The comando Tauri `get_dictation_stats(days)` SHALL devolver o mesmo `MetricsSummary`
12. WHEN o app roda no Windows (`bun run tauri dev`, store portátil, `FALA_TRACE=1`) THEN 10 ditados curtos SHALL deixar 10 linhas `event=dictation` no log e 10 linhas em `dictation_metrics`; p50/p90 de cada marco vão para o `verification.md` contra o orçamento

## Out of scope

| Excluded | Why |
| --- | --- |
| página "Como estou indo" na UI | não cabe nas 400 linhas junto com o resto; o comando Tauri está pronto e a página fica descrita no `verification.md` |
| `event=load` no desktop (início a frio) | a carga do modelo é em segundo plano no desktop; outra fatia |
| `numWordsCorrected` do Wispr | precisa do texto bruto × final; fora do pedido |
| ditados do binding legado `transcribe_with_post_process` | gravados igual, com `llm_used` = o legado produziu o texto e `fallback` nulo |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| door 1 | tabela própria, sem subir `user_version` | sobrevive à retenção; não colide com #62 | y — delegado (2026-10-09), decidido pelo executor |
| door 2 | `TranscriptionCoordinator::send`, janela de 1 s | é o primeiro ponto comum ao hook e ao gatilho externo antes do `AudioRecordingManager` | y — delegado (2026-10-09), decidido pelo executor |
| door 3 | volta do `show_*_overlay` | sem ida e volta pela UI | y — delegado (2026-10-09), decidido pelo executor |
| ditado sem item salvo (WAV falhou) | grava a métrica sem `dictation_id` | "gravadas sempre" | y — delegado (2026-10-09), decidido pelo executor |
| ditado vazio, cancelado ou com erro de ASR | sem métrica | não houve texto colado; o e2e não existe | y — delegado (2026-10-09), decidido pelo executor |

**Open questions:** none - all resolved or logged above.

## Observable

- `info` com target `fala_trace` (só com `FALA_TRACE=1`): a linha da door 4.
- `warn`: `dictation metrics not saved: <erro>` quando o `Store` falha; nunca texto.

## Sources

- `fala-research/plans/fase-1-delta-e-semanas-3-4.md` § F9 ("porta 3")
- `fala-research/plans/status/inventario-2026-10-09.md` § 2 e § 4 item 5
- `fala-research/research/18-wispr-flow-binario-1.6.1034.md` § 4 e § 7.9
- `.specs/features/pipeline-headless/plan.md` doors 3 e 4; `.specs/features/desktop-pipeline-audio/`
- `ARCHITECTURE.md` (orçamento de latência)
