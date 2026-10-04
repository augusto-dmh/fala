# session-limit checks

Profile: light
Plan: `.specs/features/session-limit/plan.md`

18 checks in 3 slices · 0 one-way doors · 0 open, of which 0 block

Comandos de cargo com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`,
rodados na raiz do worktree. Os testes do coordenador usam o relógio sintético da 1.F1 (`Instant`
injetado em `on_input`/`on_deadline`): `t0 + 20 min` é só aritmética, nenhum teste espera.

## Checks

### S1 - o prazo da sessão no coordenador · 1 file · 85 KB · ~21k

**C1** - Nos 4 modos (push-to-talk segurado, `push_to_talk_double_tap` travado por dois toques, `hold_or_toggle` travado, `toggle`), `on_deadline(t0 + 19 min - 1 ms)` devolve `None` e `on_deadline(t0 + 19 min)` devolve `LimitWarning` com o binding da gravação, e o estágio segue `Recording` (AC 1, borda de 19 min, table-driven)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::session_limit_warns_at_nineteen_minutes_in_every_mode`

**C2** - Depois do aviso, `on_deadline` em 19 min + 1 s e em 19 min + 30 s devolve `None`, e `session_deadline()` passa a valer `t0 + 20 min` (AC 2)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::session_limit_warns_once`

**C3** - Nos mesmos 4 modos, depois do aviso, `on_deadline(t0 + 20 min - 1 ms)` devolve `None` e `on_deadline(t0 + 20 min)` devolve `Stop` com o binding e o atalho da gravação; o estágio vai a `Processing` e nenhum `Discard` aparece na sequência (AC 3, borda de 20 min, table-driven)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::session_limit_cuts_at_twenty_minutes_with_stop`

**C4** - Sem aviso anterior, a primeira chamada `on_deadline(t0 + 25 min)` devolve `Stop` (não `LimitWarning`) e o estágio vai a `Processing`; a chamada seguinte devolve `None` (AC 4)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::session_limit_overslept_cuts_without_warning`

**C5** - Com push-to-talk segurado e nada pendente, `wake_deadline()` vale `t0 + 19 min` e `next_deadline()` vale `None`; com a janela de dois toques aberta, `wake_deadline()` vale a janela (`t0 + 500 ms`); com a sessão travada, `next_deadline()` vale `None` e `wake_deadline()` vale `t0 + 19 min`; depois do aviso, `wake_deadline()` vale `t0 + 20 min`; com uma carência pendente, `wake_deadline()` vale a carência (AC 5)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::wake_deadline_is_the_earliest_of_input_and_session`

**C6** - O laço do coordenador dorme até `wake_deadline()`, não até `next_deadline()` (AC 5, o único chamador)
Proof: `grep -q "let cmd = if let Some(deadline) = state.wake_deadline()" apps/desktop/src/transcription_coordinator.rs`

**C7** - `session_deadline()` vale `None` num estado novo e depois de cada fim antes de 19 min: `Stop` por soltar a tecla, `Discard` por fim de janela, `on_cancel(true)` gravando e `on_start_result(_, false)`; e nesses 4 casos `on_deadline(t0 + 20 min)` devolve `None` (AC 6, 4 fins)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::session_deadline_clears_when_the_recording_ends`

**C8** - Uma tecla de push-to-talk pressionada em `t1` durante o processamento e iniciada pelo drain em `t1 + 2 s` tem `session_deadline()` = `t1 + 19 min` e devolve `Stop` em `on_deadline(t1 + 20 min)` depois do aviso (AC 7)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::drained_recording_measures_limit_from_key_down`

**C9** - Push-to-talk segurado, cortado aos 20 min: o key-up em `t0 + 20 min + 5 s` não devolve efeito, a carência seguinte também não, e `on_processing_finished()` devolve `None` com estágio `Idle` (AC 8)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::release_after_limit_cut_starts_nothing`

**C10** - Sessão travada por dois toques, cortada aos 20 min: um toque de 100 ms em seguida (press, release e carência) não devolve efeito, e `on_processing_finished()` devolve `None` com estágio `Idle` (AC 9)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::tap_after_limit_cut_starts_nothing`

**C11** - Os testes do coordenador da 1.F1, da 1.F2 e os herdados continuam verdes sem asserção editada (regressão do gesto, do drain e da tecla de cancelar; inclui os `next_deadline() == None` com a sessão travada)
Proof: `cargo test -p fala --lib transcription_coordinator`

### S2 - o aviso e o corte no app · 3 files · 125 KB · ~6k (só `run_effect`, `audio_feedback.rs` e o trecho de eventos de `overlay.rs`)

**C12** - `run_effect` encaminha `Effect::LimitWarning` para `audio_feedback::play_limit_warning` e `utils::emit_recording_limit_warning`; o corpo de `play_limit_warning` chama `play_test_sound` duas vezes (que ignora `audio_feedback`), dorme 150 ms entre elas e não chama `play_feedback_sound` (AC 10)
Proof: `grep -A6 "Effect::LimitWarning { binding_id } =>" apps/desktop/src/transcription_coordinator.rs | grep -q "play_limit_warning(app)"`
Proof: `grep -A6 "Effect::LimitWarning { binding_id } =>" apps/desktop/src/transcription_coordinator.rs | grep -q "emit_recording_limit_warning(app)"`
Proof: `test "$(awk '/^pub fn play_limit_warning/,/^}/' apps/desktop/src/audio_feedback.rs | grep -c 'play_test_sound(&app, SoundType::Start)')" -eq 2`
Proof: `awk '/^pub fn play_limit_warning/,/^}/' apps/desktop/src/audio_feedback.rs | grep -q "from_millis(150)"`
Proof: `! awk '/^pub fn play_limit_warning/,/^}/' apps/desktop/src/audio_feedback.rs | grep -q "play_feedback_sound"`

**C13** - `emit_recording_limit_warning` retorna sem emitir quando `OVERLAY_ENABLED` é falso e, quando verdadeiro, emite `recording-limit-warning` só para a janela `recording_overlay` (AC 11)
Proof: `awk '/^pub fn emit_recording_limit_warning/,/^}/' apps/desktop/src/overlay.rs | grep -q "if !OVERLAY_ENABLED.load"`
Proof: `awk '/^pub fn emit_recording_limit_warning/,/^}/' apps/desktop/src/overlay.rs | grep -q 'emit_to("recording_overlay", "recording-limit-warning"'`

**C14** - O corte é o mesmo `Stop` do atalho: o braço `Effect::Stop` de `run_effect` segue chamando `stop(app, ...)`, sem caminho próprio do limite, e a entrega tudo-ou-nada da 1.F2 continua verde (AC 12; o efeito do corte é `Stop` por C3)
Proof: `grep -A3 "        Effect::Stop {" apps/desktop/src/transcription_coordinator.rs | grep -q "} => stop(app, &binding_id, &hotkey_string),"`
Proof: `cargo test -p fala --lib actions::tests::delivery_is_all_or_nothing_after_the_last_check`

**C15** - TODO(windows): no build do Windows, com a pill ligada, `audio_feedback` desligado e o Bloco de Notas em foco, um ditado travado por dois toques e deixado com fala contínua perto do microfone fica com o ponto âmbar e toca dois toques do som de início aos 19:00; aos 20:00 a pill passa a "transcrevendo" e o texto cai no Bloco de Notas, com uma entrada nova no histórico. Também verifica que 20 min de áudio transcrevem sem erro no motor de ASR do build (AC 10, 12, 13 no app real)
Proof: `TODO(windows)` manual - 1 sessão de 20 min cronometrada; registrar o instante do aviso (esperado 19:00 ± 1 s), o do corte (20:00 ± 1 s), se o texto colado não está vazio e se o histórico ganhou 1 entrada

### S3 - a pill âmbar · 2 files · 26 KB · ~2k (só os trechos tocados)

**C16** - `RecordingOverlay.tsx` escuta `recording-limit-warning` e liga um estado que põe a classe `limit` no `.sdot`; esse estado volta a falso no `show-overlay` de `recording`/`streaming` e no `hide-overlay`; `.sdot.limit` usa `var(--color-warning)` com animação própria (AC 13, AC 14)
Proof: `grep -q 'listen("recording-limit-warning"' src/overlay/RecordingOverlay.tsx`
Proof: `test "$(grep -c 'setLimitWarning(false)' src/overlay/RecordingOverlay.tsx)" -ge 2`
Proof: `grep -q 'limitWarning ? " limit" : ""' src/overlay/RecordingOverlay.tsx`
Proof: `awk '/^\.sdot\.limit \{/,/^\}/' src/overlay/RecordingOverlay.css | grep -q "var(--color-warning)"`
Proof: `bun run lint`
Proof: `bunx tsc --noEmit`

**C17** - O front segue formatado e com as traduções em dia (nenhuma chave nova; o aviso não tem texto)
Proof: `bun run format:check`
Proof: `bun run check:translations`

**C18** - Na pill mínima da fase 1 (#28), `pillTone` devolve `limit` gravando com o aviso (com ou sem a tecla segurada), `hold` gravando segurado sem aviso, `null` gravando sem os dois e `null` processando com o aviso; a `Pill` com `limit` põe a classe `limit` (sem `hold`), mantém as 10 barras e nenhum texto; processando com `limit` segue sem a classe e com a forma fixa; `.fpill.limit` tem `background: #d97706` e `fpill-limit-pulse 0.9s infinite` entre opacidade 1 e 0,6, sem animação em `prefers-reduced-motion`; `RecordingOverlay` passa `limit={limitWarning}` à pill (AC 15, AC 13 e AC 14 na pill nova)
Proof: `bun src/overlay/pill.test.tsx`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| modos de ativação com limite (4) | push-to-talk segurado C1, C3 · `push_to_talk_double_tap` travado C1, C3 · `hold_or_toggle` travado C1, C3 · `toggle` C1, C3, table-driven | - |
| marcas da sessão (2) | aviso 19 min C1, C2 · corte 20 min C3, C4 | - |
| borda do aviso (2) | 19 min - 1 ms C1 · 19 min C1 | - |
| borda do corte (2) | 20 min - 1 ms C3 · 20 min C3 | - |
| efeitos do limite (2) | `LimitWarning` C1, C12 · `Stop` C3, C14 | - |
| prazos do laço (3) | carência C5 · janela de dois toques C5 · marca da sessão C5, C6 | - |
| fins antes de 19 min (4) | stop C7 · discard C7 · cancel C7 · início que falhou C7 | - |
| origem do key-down (2) | key-down direto C1 · press lembrado no drain C8 | - |
| entradas depois do corte (2) | key-up do push-to-talk segurado C9 · toque no modo padrão C10 | - |
| saídas do aviso (2) | som C12 · evento da pill C13, C16 | - |
| estado do overlay (2) | pill ligada: evento C13 · pill desligada: nada emitido C13 | - |
| volta do ponto ao normal (3) | `show-overlay` recording C16 · `show-overlay` streaming C16 · `hide-overlay` C16 | - |
| cor da pill mínima gravando (3) | âmbar com aviso C18 · vermelho segurado sem aviso C18 · preto C18 | - |
| pill mínima com aviso por modo (2) | `recording` âmbar C18 · `processing` preta C18 | - |

- Claims naming a value at a boundary: C1, C3 - each asserts both sides of its edge
- C15 is the only check not settled on Linux; it is `TODO(windows)` and stays unchecked
- No other check claims more than the cases its proof exercises

## Swept

- validation: n/a - nenhuma entrada nova de usuário; 19 e 20 min são constantes, não configuração
- failure modes: C3 (o corte é `Stop`, nunca `Discard`: o texto não se perde); C4 (thread acordada tarde corta sem exigir o aviso)
- idempotency: C2 (o aviso sai uma vez por gravação)
- authorization: n/a - prazo interno do coordenador, sem chamador externo
- concurrency: C5, C6 (um único laço acorda no prazo mais próximo, sem thread de timer concorrendo com as entradas); C9, C10 (entradas atrasadas depois do corte não reabrem gravação)
- data lifecycle: C14 (o ditado cortado ganha entrada no histórico pela entrega da 1.F2, junto com a colagem)
- dependency failure: existing - falha ao abrir a saída de áudio ou achar o som é logada por `play_sound_blocking` herdado e não afeta o corte; com a pill desligada o evento não sai e o som segue, C13
- state transitions: C1-C4, C7-C10
- observability: n/a - o corte loga em `info!` e o aviso em `debug!`, sem conteúdo ditado (AGENTS.md); sem métrica nem trace nesta feature

## Handoff

- S1 = 21k (coordenador, 85 KB / 4) + S2 = 6k (trechos de `audio_feedback.rs` e `overlay.rs`) + S3 = 2k (trechos do overlay React e CSS) = ~29k, tudo em `apps/desktop/src` e `src/overlay`, abaixo do budget de 150k - one builder
- **Settled mid-build:** o helper herdado `drive` (testes de #1539) faz `match` exaustivo sobre `Option<Effect>`; com a variante `LimitWarning` ele não compilava. Ganhou um braço `panic!("these sequences never reach the session limit")`, como a 1.F1 fez para `Discard`. Nenhuma asserção herdada mudou; C11 deve ser lido como "nenhuma asserção herdada editada"
- **Settled mid-build:** o corte também zera uma carência de soltura ainda pendente (`pending_release`), porque essa soltura pertence à sessão cortada; sem isso, uma repetição de key-down durante o processamento seria engolida como cancelamento da soltura
- **Abandoned:** somar a marca da sessão a `next_deadline()`; testes da 1.F1 afirmam `next_deadline() == None` com a sessão travada e durante o auto-repeat, então o laço passou a usar `wake_deadline()` e `next_deadline()` ficou como estava
- **Boundary:** C1-C14, C16 e C17 fechados em `feat/session-limit`; C15 fica `TODO(windows)`
- **Settled mid-build:** depois do merge do `main` com a pill nova (#28), o ramo mínimo do overlay deixou de ter o ponto da esquerda, e o âmbar só aparecia no overlay Live. A cápsula da pill mínima passou a ficar âmbar gravando com o aviso (AC 15, C18), com cor literal porque `Pill.css` não usa variáveis de tema (C12 da pill). Confirmed? y — delegado pelo Augusto, decidido pelo executor
- **Boundary:** C18 fechado em `feat/session-limit`
