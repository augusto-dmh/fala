# cancel-anywhere — Esc cancela gravando, transcrevendo e no LLM

## Problem

Esc só cancela enquanto o microfone grava. A tecla é registrada no início da gravação
(`actions.rs:602`) e desregistrada no primeiro instante do `stop` (`actions.rs:640`), e o handler
ainda exige `audio_manager.is_recording()` (`shortcut/handler.rs:62-67`). Quem solta o atalho e
percebe que falou errado não tem como impedir que o texto caia no app em foco: durante a
transcrição e o pós-processamento o Esc vai para o app e o ditado é colado mesmo assim. O pitch
pede "Esc cancela" nas semanas 3-4 e o design doc quer cancelar até a inserção (delta W2 e F2).

O pipeline já sabe abortar em qualquer etapa: `cancel_current_operation` incrementa a geração de
cancelamento e o `TranscribeAction::stop` confere `was_cancelled_since` depois de parar a
gravação, antes da saída, durante o LLM (`complete_unless_cancelled`) e antes de colar
(`actions.rs:690`, `:756`, `:778-788`, `:790`, `:819`). O botão X da pill e o "Cancelar" do tray
chegam lá; só o Esc não chega. Há também uma fresta herdada: o histórico é gravado
(`actions.rs:797-808`) antes da última conferência, feita na thread principal antes de colar, e
um cancelamento nesse intervalo deixa uma entrada no histórico de um texto que não foi colado.

A fonte não traz números de incidência. O custo é o de qualquer ditado errado colado num chat ou
num e-mail: apagar à mão, depois de já ter sido visto.

Quando isto entrar, Esc cancela do key-down até a colagem: a pill some, o tray volta a Idle,
nada é colado e o histórico não ganha entrada. Depois que o texto entrou, Esc volta a ser só Esc.

## Flow

Reusa o teardown de `utils::cancel_current_operation`/`abort_current_operation` e as conferências
de geração que o `TranscribeAction::stop` já faz; não cria um segundo caminho de cancelamento. O
que muda é quem arma a tecla: o estágio do `CoordinatorState` (que a 1.F1 organizou) passa a
decidir, em vez de o início e o fim da gravação.

1. laço do `TranscriptionCoordinator` (exists) - depois de cada comando ou prazo, compara `CoordinatorState::is_busy()` (estágio diferente de `Idle`) com o estado armado e chama `shortcut::register_cancel_shortcut`/`unregister_cancel_shortcut` (exists) só na mudança; espelha o valor num `AtomicBool` lido por `TranscriptionCoordinator::is_busy()`
2. Esc -> `shortcut::handler::handle_shortcut_event` (exists) - o binding `cancel` dispara `CancelAction` quando o coordenador está ocupado, em vez de só quando o microfone grava
3. `CancelAction` -> `utils::cancel_current_operation` (exists) - incrementa a geração, esconde a pill, põe o tray em Idle e avisa o coordenador; durante `Processing` o coordenador continua ocupado até o pipeline drenar (comportamento herdado de `on_cancel`)
4. `TranscribeAction::stop` (exists) - cada conferência de geração existente encerra o pipeline sem colar; a última, na thread principal, passa a decidir colagem e histórico juntos, numa função pura nova no próprio `actions.rs` (placement)
5. out: cancelado antes da última conferência, nada colado e nada no histórico; depois dela, texto colado e histórico gravado, e o Esc já não está armado quando o coordenador recebe `ProcessingFinished`

## Impact

| Front | What changes |
| --- | --- |
| domain | termo novo: `is_busy` - o coordenador tem um ditado em voo (estágio `Recording` ou `Processing`). Decide se a tecla de cancelar está armada e se o Esc age. Vive em `transcription_coordinator.rs` |
| domain | termo existente: tecla `cancel` armada "enquanto grava" passa a "enquanto o coordenador está ocupado". Quem ramifica nisso hoje: `TranscribeAction::start`/`stop` (`actions.rs:602`, `:640`) e `abort_current_operation` (`utils.rs`), que deixam de registrar e desregistrar; o fallback do Secure Input no macOS (`secure_input::register_cancel_fallback`) segue chamado pelos mesmos `register_cancel_shortcut`/`unregister_cancel_shortcut` |
| behaviour | com o modo blocking do `handy-keys` (Windows), o Esc fica capturado também durante a transcrição e o LLM (~1-2 s depois de soltar), não só gravando |
| behaviour | na conclusão normal, o histórico passa a ser gravado depois da colagem, na thread principal, e não antes dela; o texto e os campos gravados são os mesmos. A 1.F5 (`history-undo`) vai mover essa gravação para antes do LLM e precisa preservar a regra do AC 6 |
| stored data | nada a migrar; nenhuma chave nova no store |

## Relations

`None - no stored-data shape change`

## Surface

`None - nothing consumed outside`. O comando `cancel_operation`, o item "Cancelar" do tray e o
`--cancel` da linha de comando mantêm a assinatura e o caminho.

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. padrão novo: o estágio do coordenador arma a tecla de cancelar | `if busy != armed { if busy { shortcut::register_cancel_shortcut(app) } else { shortcut::unregister_cancel_shortcut(app) }; armed = busy }` no laço do coordenador, depois de cada comando ou prazo; ninguém mais chama esses dois | mover o `unregister` do `stop` para o `FinishGuard`: os dois registram por `spawn` assíncrono, e um `FinishGuard` seguido de um toque lembrado durante o processamento gera um `unregister` e um `register` a microssegundos um do outro, em workers diferentes, que podem chegar invertidos e deixar a gravação seguinte sem Esc. Avaliado uma vez por comando, o drain com início lembrado vai de `Processing` a `Recording` sem passar por "desarmar" |

- Nothing else in this change is hard to reverse: a gravação do histórico depois da colagem é uma reordenação local que a 1.F5 refaz, e `is_busy` é interno ao desktop.

## Criteria

### S1: a tecla de cancelar acompanha o ditado (P1)

O coordenador decide, sem teclado real, quando o Esc está armado.

**Acceptance Criteria**

1. The coordinator SHALL report `is_busy()` as true while its stage is `Recording` or `Processing` and false while it is `Idle`, across start, stop, discard, cancel and drain
2. WHILE the pipeline is processing, WHEN a cancel arrives THEN the coordinator SHALL stay busy until `ProcessingFinished`, and SHALL be idle after it with no remembered press started
3. WHEN the pipeline drains with a remembered press THEN the coordinator SHALL stay busy across the drain (from `Processing` straight to `Recording`), so the cancel key is never disarmed and rearmed for it
4. The coordinator loop SHALL be the only caller of `register_cancel_shortcut` and `unregister_cancel_shortcut`, and SHALL call one of them only when `is_busy()` differs from the state it last applied

### S2: Esc cancela em qualquer etapa sem rastro (P1)

**Acceptance Criteria**

5. WHEN the cancel binding is pressed while the coordinator is busy THEN the desktop SHALL run `cancel_current_operation`, and WHEN it is pressed while the coordinator is idle THEN the desktop SHALL do nothing
6. IF the dictation was cancelled before the last cancellation check THEN the desktop SHALL neither paste the text nor save a history entry, and IF it was not THEN the desktop SHALL paste first and then save the history entry
7. WHEN Esc is pressed during transcription or during the LLM call THEN the desktop SHALL hide the pill, set the tray to idle and paste nothing (existing generation checks, reached now by Esc)

**Independent test:** no Windows, segurar o atalho, falar uma frase, soltar e apertar Esc logo depois: a pill some, nada é colado no Bloco de Notas, o histórico não ganha entrada; Esc depois da colagem chega ao app.

## Out of scope

| Excluded | Why |
| --- | --- |
| Desfazer uma colagem já feita | o texto já entrou no app; o delta recomenda não cancelar depois da colagem, e desfazer exigiria saber onde o texto está no campo (context awareness, no-go do pitch) |
| Interromper o cálculo do ASR em andamento | o motor herdado não é interrompível; o resultado é descartado na conferência seguinte e a pill some na hora |
| Apagar o WAV gravado de um ditado cancelado depois da transcrição | comportamento herdado (o WAV é salvo em paralelo); o destino dos WAVs é porta da 1.F5 |
| Esc no Linux | o registro dinâmico do cancel é desligado no Linux pelo código herdado (`fala_keys.rs:461-468`, `tauri_impl.rs:165-171`), e o Linux não é alvo da fase 1; o botão X da pill e o tray seguem cancelando lá |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Esc depois da colagem | não cancela nem desfaz; o Esc é desarmado quando o coordenador recebe `ProcessingFinished` | recomendação do delta (F2): o texto já entrou | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| Onde fica o "ponto sem volta" | a conferência de geração na thread principal, imediatamente antes de colar; histórico e colagem acontecem juntos depois dela | é a última conferência que já existe; antes dela, cancelar não deixa rastro, depois dela os dois acontecem | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| Toque no atalho durante o resto do processamento cancelado | lembrado e iniciado quando o pipeline drena, como qualquer toque durante o processamento | comportamento herdado de `on_cancel` + `classify_busy_input`; a espera é a do ASR em andamento | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| Som ao cancelar | nenhum | o cancelamento herdado não toca som e o pitch não pede | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |

**Open questions:** none - all resolved or logged above.

## Observable

| Surface | Decision | Landing |
| --- | --- | --- |
| tecla Esc (entrada física) | quando age | AC 1, AC 5 |
| tecla Esc | depois da colagem | AC 5 (coordenador ocioso, Esc ignorado e desarmado) |
| tecla Esc | engolida ou não pelo hook blocking no app em foco | n/a - verificação física do Windows, `TODO(windows)` nos checks; o comportamento do hook é herdado |
| pill (overlay) | o que a pessoa vê ao cancelar | AC 7 (some pelo teardown existente) |
| tray | estado ao cancelar | AC 7 (Idle) |
| histórico (tela) | entrada de ditado cancelado | AC 6 |
| ação destrutiva | confirmação antes de cancelar | n/a - cancelar descarta um ditado que ainda não foi colado; pedir confirmação anularia o gesto |

## Sources

- `fala-research/pitches/fase-1-ditado-windows.md` - "Esc cancela" nas semanas 3-4
- `fala-research/plans/fase-1-delta-e-semanas-3-4.md` - item W2 e feature F2 (cancelar depois da colagem: não)
