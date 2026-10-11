# long-dictation-chunks checks

Profile: light
Plan: `.specs/features/long-dictation-chunks/plan.md`

S1: 12 checks · 1 one-way door (coluna `blocks`) · 0 open. S2 entra antes do PR 2.

Comandos de cargo com `CARGO_TARGET_DIR=C:\f\long-dictation-chunks`, na raiz do worktree, com a
trava de cargo entre painéis para o que compila o crate `fala`.

## Checks

### S1 - ASR por blocos · `crates/audio`, `crates/storage`, `apps/desktop`, `apps/cli`

**C1** - O bloco fecha na primeira utterance terminada por silêncio depois de 25 s (duas falas de
~11 s não fecham; a terceira pausa fecha), e a concatenação dos blocos, com o resto no `stop`, é a
das utterances (AC 1, AC 3)
Proof: `cargo test -p fala-audio --lib capture::tests::blocks_close_on_the_first_pause_after_25_s`

**C2** - Utterances cortadas no teto de 15 s não fecham bloco mesmo passando de 25 s; a primeira
pausa de 450 ms depois fecha um bloco de exatamente 1.015 quadros (AC 1)
Proof: `cargo test -p fala-audio --lib capture::tests::the_15_s_cap_is_not_a_pause`

**C3** - 100 s de fala sem pausa fecham dois blocos de exatamente 640.000 amostras, e o resto do
`stop` é `input[1.280.000..]` (AC 2, AC 3)
Proof: `cargo test -p fala-audio --lib capture::tests::speech_without_pause_cuts_blocks_at_40_s`

**C4** - Sem `with_blocks`, `take_blocks` é vazio; com ele, um `start` descarta os blocos da sessão
anterior que ninguém pegou (AC 8, memória)
Proof: `cargo test -p fala-audio --lib capture::tests::blocks_are_off_by_default_and_reset_on_start`

**C5** - Um WAV sintético de 3 min (falas de 3-14 s, pausas de 0,6-2 s) pelo `Processor` fecha ≥ 3
blocos, todos entre 25 e 40 s e terminando nos 450 ms de silêncio; os blocos são o prefixo da
gravação e o resto fica para o soltar. Sem pausa, 40 s exatos. 20 s de fala não fecham bloco
(AC 4, AC 6)
Proof: `cargo test -p fala --lib dictation_capture::tests::three_minutes_with_pauses_close_blocks_in_silence`
Proof: `cargo test -p fala --lib dictation_capture::tests::speech_without_pause_cuts_at_40_s`
Proof: `cargo test -p fala --lib dictation_capture::tests::a_20_s_dictation_closes_no_block`

**C6** - Junção: blocos e resto juntam o texto da gravação inteira sem repetir nem perder palavra
(cada amostra é uma palavra), 3 chamadas ao transcritor; texto vazio e resto vazio não deixam
espaço (AC 5)
Proof: `cargo test -p fala --lib long_dictation::tests::blocks_and_rest_join_without_repeating_or_losing_words`
Proof: `cargo test -p fala --lib long_dictation::tests::empty_text_and_an_empty_rest_leave_no_gap`

**C7** - Sem bloco fechado, `finish` devolve `None` e o transcritor nunca é chamado; desarmada, a
fila ignora blocos (AC 6, AC 8)
Proof: `cargo test -p fala --lib long_dictation::tests::without_a_closed_block_the_old_path_runs`
Proof: `cargo test -p fala --lib long_dictation::tests::an_unarmed_queue_ignores_blocks`

**C8** - Esc no meio: com 3 blocos na fila e o primeiro no motor, `cancel` descarta os outros dois
(1 chamada), `finish` devolve `None` só com o motor livre, e a gravação seguinte funciona; uma
corrida nova espera a cancelada antes de usar o motor (AC 7)
Proof: `cargo test -p fala --lib long_dictation::tests::cancel_drops_the_queue_and_frees_the_engine`
Proof: `cargo test -p fala --lib long_dictation::tests::a_new_run_waits_for_the_cancelled_one`
Proof: `grep -c "self.blocks.cancel();" apps/desktop/src/managers/audio.rs` = 2 (`cancel_recording` e o stop cancelado)

**C9** - Um bloco que falha falha o ditado com o erro do motor (AC 9)
Proof: `cargo test -p fala --lib long_dictation::tests::a_failed_block_fails_the_dictation`

**C10** - A fila só é armada com VAD `Offline` e modelo que não descarrega a cada uso (AC 8)
Proof: `cargo test -p fala --lib actions::tests::only_offline_vad_with_a_resident_model_uses_blocks`

**C11** - Métricas: `blocks` vai para a linha e para o trace (`blocks=1`, `blocks=21`); uma tabela de
antes da coluna a recebe no `open` com as linhas antigas valendo 1, e a segunda abertura não falha
(AC 10)
Proof: `cargo test -p fala --lib dictation_metrics::tests::marks_become_metrics_and_trace`
Proof: `cargo test -p fala-storage --test metrics`

**C12** - A CLI manda blocos ao ASR durante a gravação: 100 s de fala viram dois blocos de 40 s
antes do soltar e um de 20 s depois (AC 11)
Proof: `cargo test -p fala-cli --bin fala-cli dictate::tests::blocks_reach_the_asr_while_recording`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| como uma utterance fecha (3) | silêncio C1, C2 · teto de 15 s C2 · stop C1, C3 | - |
| como um bloco fecha (3) | pausa depois de 25 s C1, C5 · teto de 40 s C3, C5 · resto no stop C1, C3 | - |
| fins de um ditado com fila (4) | soltar com blocos C6 · soltar sem blocos C7 · Esc C8 · erro de ASR C9 | - |
| políticas de VAD (3) | `Offline` C10 · `Streaming` C10 · `Disabled` C10 | - |
| descarga do modelo (2) | `Immediately` desarma C10 · outras armam C10 | - |
| quem corta blocos (2) | desktop C5 · CLI C12 | - |
| quem grava `dictation_metrics` (1) | `Measured::finish` C11 | - |

## Swept

- validation: n/a - nenhuma entrada nova de usuário
- failure modes: C9 (erro num bloco), C8 (thread de fundo morta vira erro no `finish`)
- idempotency: C11 (o `ALTER TABLE` não roda de novo)
- concurrency: C8 (uma thread por fila; a nova espera a cancelada; motor único)
- data lifecycle: o WAV e o histórico seguem com a gravação inteira; nada muda no `save_entry`
- dependency failure: existing - modelo não carregado falha o bloco como falharia o ditado (C9)
- state transitions: C7, C8 (`Off` → `Armed` → `Running` → `Off`)
- observability: C11 (`blocks=` no trace; nada de texto)

## Manual (não roda nesta sessão)

- `FALA_TRACE=1 cargo run --release -p fala-cli -- dictate --model <parakeet> --wav <10 min>`,
  na tomada e com a CPU quieta: `release_to_text_ms` constante, sem crescer com a duração
- Windows com o Augusto: 3 ditados reais de 5 min em hands-free (Windows Terminal e claude.ai),
  texto inteiro colado, `blocks=` e soltar → texto anotados
