# media-pause-and-sounds — pausar a mídia ao ditar e ao gravar; sons da reunião

## Problem

`mute_while_recording` (herdado) silencia a saída inteira durante o ditado: o vídeo continua
andando sem som. Pausar faz o vídeo esperar. Na reunião, música tocando vaza para o canal do
sistema e deixa o ASR local mais lento. A reunião também não tem som próprio de início, pausa e
retomada, que reforçam o indicador da ADR-0005 sem olhar a pílula.

## Flow

1. Ditado: o worker de prontidão do `TranscribeAction::start` (exists, `actions.rs`) chama
   `AudioRecordingManager::pause_media(generation)` (new) logo depois do som de início, ao lado de
   `apply_mute`. Com `pause_media_while_recording` ligado e a geração ainda atual, manda
   `Pause(Dictation)` para o worker de mídia e marca `media_paused`, tudo sob um mutex, para que um
   `stop` que invalidou a geração antes nunca seja ultrapassado por um pause atrasado.
2. `stop` do `TranscribeAction`, `cancel_recording` e `stop_microphone_stream` chamam
   `resume_media()` (new): se `media_paused`, manda `Resume(Dictation)`.
3. Reunião: `MeetingManager::start` (StartCapture aberto com sucesso) e `run_effects` passam cada
   efeito por `meeting_sound(&Effect)` e `media_action(&Effect)` (new, puros). `StartCapture` toca
   `MeetingStart` e pausa a mídia; `PauseCapture` toca `MeetingPause` e retoma; `ResumeCapture` toca
   `MeetingResume` e pausa de novo; `FinalizeCapture` e `HideIndicator` retomam. A pausa respeita
   `pause_media_on_meeting`; a retomada sempre roda (é no-op sem nada pausado).
4. `media_pause.rs` (new): uma thread `media` recebe `Pause(owner)`/`Resume(owner)` em ordem e guarda,
   por dono (`Dictation`, `Meeting`), as sessões que o Fala pausou. `Pause` pausa só as sessões em
   `Playing`; `Resume` toca só as que ele pausou e que continuam `Paused`. O controle é o trait
   `MediaControl`; no Windows, `Smtc` sobre `GlobalSystemMediaTransportControlsSessionManager`
   (cada operação WinRT com prazo de 2 s); fora do Windows, nenhum controle: os comandos são
   ignorados. Nunca `VK_MEDIA_PLAY_PAUSE`.
5. `audio_feedback.rs`: `SoundType` ganha `MeetingStart` (som de início do tema), `MeetingPause`
   (`<tema>_meeting_pause.wav`, novo) e `MeetingResume` (`<tema>_meeting_resume.wav`, novo); no tema
   personalizado, pausa usa `custom_stop.wav` e retomada `custom_start.wav`. Tocam por
   `play_feedback_sound`, logo respeitam `audio_feedback`, volume e saída.
6. Configurações → Som: dois toggles (`PauseMedia.tsx`), comandos
   `change_pause_media_while_recording_setting` e `change_pause_media_on_meeting_setting`.

## Impact

| Front | What changes |
| --- | --- |
| stored data | duas chaves novas no store com `serde(default)`: `pause_media_while_recording` (false), `pause_media_on_meeting` (true) |
| behaviour | com a pausa ligada, o ditado pausa o que tocava e retoma no fim; a reunião pausa ao gravar e ao retomar, retoma ao pausar e ao parar |
| behaviour | reunião toca sons de início, pausa e retomada quando `audio_feedback` está ligado |
| resources | quatro `.wav` novos (`marimba_meeting_{pause,resume}`, `pop_meeting_{pause,resume}`), derivados dos sons do tema por mudança de altura (±3 semitons) |
| ui | dois toggles e quatro chaves de i18n (pt, en) |
| deps | features `Foundation`, `Foundation_Collections`, `Media_Control` do `windows` 0.61 já no lock; o lock não muda |

## Decisions

- Padrões: ditado desligado (opt-in), reunião ligado, como a definição recomenda (o Granola liga por
  padrão na reunião).
- Na pausa da reunião a mídia volta; na retomada pausa de novo: a mídia fica pausada exatamente
  enquanto a captura grava.
- Meeting start reusa o som de início do tema; só pausa e retomada são arquivos novos.
- O código fica em `apps/desktop` (ADR-0007 permite), não num crate: o único consumidor é o desktop.

## Criteria

1. Pause só pausa sessões que tocavam; Resume só toca as que o Fala pausou e que seguem pausadas.
2. Um player já pausado nunca recebe play.
3. Ditado e reunião guardam pausas separadas; Resume de um não toca o que o outro pausou.
4. Sem controle de mídia (Linux, SMTC indisponível), nada acontece e nada trava.
5. Cada efeito da sessão de reunião dá o som e a ação de mídia certos (tabela do Flow, passo 3).
6. Cada tema resolve os três sons de reunião para um arquivo que existe em `resources/`.
7. As duas chaves novas têm os padrões acima e um store antigo sem elas carrega com esses padrões.
8. A UI mostra os dois toggles em Som, com strings em pt e en.
