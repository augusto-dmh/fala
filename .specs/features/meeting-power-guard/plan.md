# meeting-power-guard — o PC não dorme enquanto grava; uma suspensão vira um buraco marcado

## Problem

Nada impede a suspensão por inatividade enquanto uma reunião grava: numa aula ou numa call em que
só se ouve, o PC dorme no meio, a captura WASAPI para e o resto não é gravado. A máquina de estados
de `fala-meeting` já tem `OsSuspended`/`OsResumed`, mas ninguém os entrega, e o desenho atual fecha o
WAV na suspensão e espera um "continuar ou parar" que não existe (`AskContinueOrStop` é ignorado no
desktop). Esta máquina (Alienware 16) só tem Modern Standby (S0 Low Power Idle).

## Flow

1. `apps/desktop/src/power.rs` (new): trait `PowerApi { keep_awake(on) }`, `PowerGuard<A>` (pede
   uma vez, solta no `Drop`). Windows: `SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED)`
   e `SetThreadExecutionState(ES_CONTINUOUS)`, por thread; sem `ES_DISPLAY_REQUIRED`.
   `watch_sleep(f)`: `PowerRegisterSuspendResumeNotification` com callback, `PBT_APMSUSPEND` →
   `Suspending`, `PBT_APMRESUMEAUTOMATIC`/`PBT_APMRESUMESUSPEND` → `Resumed`. Fora do Windows,
   no-op.
2. `meeting/recorder.rs`: `Gate` (new) junta o `ActiveClock`, a pausa da pessoa, o sono do SO e o
   `PowerGuard`: o PC fica acordado exatamente enquanto a thread escreve (`keep_awake && !pausa &&
   !sono`). A guarda vive na thread do gravador, então sai no parar, no erro e no pânico (unwind; o
   Windows ainda limpa o estado da thread que morre). `Control::Suspend(ack)`: para de escrever,
   relógio parado, cabeçalho do WAV no disco (`MeetingRecorder::flush`, new), responde. `Control::Wake`:
   reabre sistema e mic (tentando por até 5 s), troca as taxas (`set_mic_rate`, new), volta a
   escrever se a pessoa não tinha pausado e responde `Report::Woke`; sem streams em 5 s, a thread
   termina com erro.
3. `fala-meeting`: `OsSuspended` (de `Recording` ou `Paused`) → `Suspended { paused }` com
   `[SuspendCapture, ShowIndicator(Suspended)]`; `OsResumed` → volta ao estado anterior com
   `[WakeCapture, ShowIndicator(..)]` e o buraco `Suspend` fechado (uma pausa da pessoa reabre seu
   buraco `Pause`). `ResumeRecording` deixa de valer em `Suspended`; `StopRecording` continua.
   `AskContinueOrStop` sai.
4. `meeting/mod.rs`: `on_sleep(event)` aplica a entrada; `SuspendCapture` manda `Control::Suspend` e
   espera o ack até 1,5 s fora do lock. `WakeCapture` manda `Control::Wake` e marca `waking`.
   `Report::Woke`: grava `{at_ms, slept_ms}` em `audio/<id>/sleeps.json` e, gravando, mostra o card
   "A gravação continuou depois da suspensão / O computador dormiu por N min" por 10 s.
   `Finalized` com erro durante `waking`: a sessão para como interrompida (`recover_meeting`,
   `stop_reason` nulo) e segue o pipeline (retenção → transcrição), sem "gravou por engano?".
5. `MeetingDetail.sleeps` (lido de `sleeps.json`) e `TranscriptView` mostra o marcador "O computador
   dormiu por N min" antes da primeira fala depois do ponto.
6. Configuração `meeting_keep_awake` (padrão ligado) em Configurações → Reuniões, comando
   `change_meeting_keep_awake_setting`; lida no início de cada gravação.

## Slices

O diff inteiro passa do limite de 1000 linhas do check `pr`; vão dois PRs empilhados, cada um útil
sozinho:

1. `feat/meeting-power-guard`: passos 1 (só `PowerGuard`), 2 (só pausa) e 6. O PC não dorme por
   inatividade enquanto grava; critérios 1 e 6.
2. `feat/meeting-power-guard-2`: `watch_sleep`, passos 2 (sono), 3, 4 e 5; critérios 2 a 5.

## Impact

| Front | What changes |
| --- | --- |
| stored data | chave `meeting_keep_awake` (default true); `audio/<id>/sleeps.json` só quando o PC dormiu |
| behaviour | gravando, o Windows não suspende por inatividade (a tela pode apagar); após uma suspensão a mesma sessão continua sozinha, pausada se estava pausada |
| crate API | `Effect::{SuspendCapture, WakeCapture}` no lugar de `AskContinueOrStop`; `SessionState::Suspended { indicator, paused }` |
| ui | toggle em Reuniões, card `woke` no call prompt, marcador no transcript, chaves pt/en |
| deps | feature `Win32_System_Power` do `windows` 0.61 (já no lock) |

## Decisions

- Retomar sozinho só o que gravava: uma reunião pausada pela pessoa volta pausada (ADR-0005: nada
  grava sem um clique; continuar uma sessão começada por clique não é começar).
- O aviso usa o card do call prompt (o mesmo do "Parou por silêncio"), não a pílula: `src/overlay`
  está em mudança noutro PR, e o card já tem contagem e [Fechar].
- O marcador vive em `sleeps.json` ao lado do áudio, não numa tabela: `crates/storage` está em
  mudança noutro PR. Apagar o áudio da sessão apaga o marcador.
- Interrompida = `recover_meeting` (mesma regra da recuperação #128), só quando a captura não volta
  depois de acordar; outros erros de dispositivo continuam como hoje.

## Criteria

1. A guarda é pedida no início, solta na pausa, repedida na retomada, solta na suspensão, repedida ao
   acordar e solta no parar e no erro; com a opção desligada nada é pedido.
2. Suspensão: o gravador para de escrever e põe o cabeçalho no disco; ao acordar escreve na mesma
   sessão sem contar o tempo dormindo.
3. A sessão volta ao estado anterior ao acordar e registra o buraco `Suspend` com a duração.
4. O marcador aparece no transcript na posição do áudio.
5. Sem captura 5 s depois de acordar: para interrompida (`stop_reason` nulo) e segue o pipeline.
6. Configuração ligada por padrão, visível em Reuniões, pt e en.
