# cancel-anywhere checks

Profile: light
Plan: `.specs/features/cancel-anywhere/plan.md`

9 checks in 2 slices · 1 one-way door · 0 open, of which 0 block

Comandos de cargo com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`,
rodados na raiz do worktree. Os testes do coordenador usam o relógio sintético dos testes da 1.F1.

## Checks

### S1 - a tecla de cancelar acompanha o ditado · 2 files · 79 KB · ~20k

**C1** - `CoordinatorState::is_busy()` vale `false` num estado novo, `true` depois de `Start` (estágio `Recording`), `true` depois de `Stop` (`Processing`), `false` depois de `on_processing_finished()`, `false` depois de um `Discard` por fim de janela, `false` depois de `on_cancel(true)` gravando e `false` depois de `on_start_result(_, false)` (AC 1, 7 transições)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::is_busy_follows_the_stage`

**C2** - Em `Processing`, `on_cancel(false)` mantém `is_busy()` em `true`; `on_processing_finished()` depois disso devolve `None` e deixa `is_busy()` em `false`, mesmo com um toque lembrado antes do cancel (AC 2)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::cancel_during_processing_stays_busy_until_drained`

**C3** - Com um toque lembrado durante `Processing`, `on_processing_finished()` devolve `Start` e `is_busy()` é `true` antes e depois do drain, sem nenhum instante observável em `false` (AC 3)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::drain_with_remembered_press_stays_busy`

**C4** - A decisão de armar é tomada só na mudança: `cancel_key_change(armed, busy)` devolve `None` para (false,false) e (true,true), `Some(true)` para (false,true) e `Some(false)` para (true,false); e `register_cancel_shortcut(`/`unregister_cancel_shortcut(` só são chamados, fora de `shortcut/`, em `transcription_coordinator.rs` (AC 4, door 1)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::cancel_key_changes_only_when_busy_flips`
Proof: `test -z "$(grep -rln 'register_cancel_shortcut(\|unregister_cancel_shortcut(' apps/desktop/src --include=*.rs | grep -v '^apps/desktop/src/shortcut/' | grep -v '^apps/desktop/src/transcription_coordinator.rs$')"`

**C5** - Os testes do coordenador da 1.F1 e os herdados continuam verdes sem asserção editada (regressão do gesto e do drain)
Proof: `cargo test -p fala --lib transcription_coordinator`

### S2 - Esc cancela em qualquer etapa sem rastro · 3 files · 52 KB · ~13k

**C6** - `cancel_key_fires(is_pressed, busy)` é `true` só para (true, true) e `false` para os outros 3 pares, e o handler do binding `cancel` consulta `TranscriptionCoordinator::is_busy()` em vez de `is_recording()` (AC 5)
Proof: `cargo test -p fala --lib shortcut::handler::tests::cancel_key_fires_only_on_press_while_busy`
Proof: `awk '/binding_id == "cancel"/,/return;/' apps/desktop/src/shortcut/handler.rs | grep -q "is_busy()"`

**C7** - `deliver_unless_cancelled(is_cancelled, paste, save)`: com `is_cancelled` verdadeiro devolve `false` e não chama nem `paste` nem `save`; com falso devolve `true` e chama `paste` e depois `save`, nessa ordem, uma vez cada (AC 6)
Proof: `cargo test -p fala --lib actions::tests::delivery_is_all_or_nothing_after_the_last_check`

**C8** - No `TranscribeAction::stop`, a gravação do histórico da conclusão normal só acontece através de `deliver_unless_cancelled`: a função é chamada nos dois ramos (texto vazio e colagem na thread principal) e o fechamento `save_history` nunca é invocado direto (AC 6)
Proof: `test "$(grep -c 'deliver_unless_cancelled(' apps/desktop/src/actions.rs)" -ge 3`
Proof: `! grep -q 'save_history()' apps/desktop/src/actions.rs`

**C9** - TODO(windows): no build do Windows, com o Bloco de Notas em foco, Esc apertado logo depois de soltar o atalho (transcrevendo) e, com pós-processamento ligado, durante o LLM, esconde a pill, não cola nada e não cria entrada no histórico; Esc apertado depois da colagem chega ao Bloco de Notas (AC 7, AC 5 no hook real). A conferência de geração durante o LLM é provada no Linux pelo teste herdado de `complete_unless_cancelled`
Proof: `cargo test -p fala --lib actions::tests::pending_operation_stops_after_cancellation`
Proof: `TODO(windows)` manual - 10 ditados cancelados por Esc transcrevendo, 5 cancelados no LLM, 5 Esc depois da colagem; contar colagens e entradas novas no histórico (esperado 0, 0, 5 colagens com 5 entradas) e se o Esc pós-colagem chegou ao app (5/5)

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| estágios do coordenador (3) | `Idle` C1 · `Recording` C1 · `Processing` C1, C2 | - |
| transições que mudam `is_busy` (7) | início C1 · parada C1 · drain C1, C3 · descarte C1 · cancel gravando C1 · cancel processando C2 · início que falhou C1 | - |
| decisão de armar (4 pares) | (false,false) C4 · (false,true) C4 · (true,false) C4 · (true,true) C4, table-driven | - |
| disparo do Esc (4 pares) | (press, ocupado) C6 · (press, ocioso) C6 · (release, ocupado) C6 · (release, ocioso) C6, table-driven | - |
| etapas canceláveis (3) | gravando C9 (hook real), C1 · transcrevendo C9 · LLM C9 (`complete_unless_cancelled` no Linux) | - |
| saída da última conferência (2) | cancelado: nada colado nem gravado C7 · não cancelado: colado e depois gravado C7, C8 | - |
| chamadores do registro da tecla (1 lugar) | laço do coordenador C4 | - |
| door 1 (estágio arma a tecla) (2) | decisão C4 · drain sem desarmar C3 | - |

- C9 é a única obrigação que só fecha no Windows; fica `TODO(windows)` e desmarcada
- Nenhum outro check afirma mais do que o caso que sua prova exercita

## Swept

- validation: n/a - nenhuma entrada nova de usuário; o binding `cancel` herdado segue validado por `change_binding`
- failure modes: C7 (cancelar no intervalo entre a última conferência e a colagem não deixa histórico órfão)
- idempotency: C2 (cancel repetido durante o processamento mantém o coordenador ocupado e não inicia nada no drain); C4 (sem chamada redundante de registro)
- authorization: n/a - tecla local e comando da própria janela, sem chamador externo
- concurrency: C3, C4 (registro e desregistro assíncronos não se invertem no drain com toque lembrado, door 1)
- data lifecycle: C7 (ditado cancelado não grava histórico); o WAV do cancelado fica como hoje, fora de escopo (1.F5)
- dependency failure: existing - falha ao registrar o Esc é logada em `register_cancel_shortcut` herdado; falha ao iniciar a gravação volta a `Idle` e não arma, C1 (`on_start_result`)
- state transitions: C1, C2, C3
- observability: n/a - só `debug!` sem conteúdo ditado (AGENTS.md); sem métrica nem trace nesta feature

## Handoff

- S1 = 20k (coordenador) + S2 = 13k (`actions.rs`, `handler.rs`, `utils.rs`) = ~33k, tudo em `apps/desktop/src`, abaixo do budget de 150k - one builder
