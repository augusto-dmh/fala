# meeting-mic plan

Profile: light

## Problema

1. A reunião abre `default_input_device()` (`meeting/recorder.rs`: `open_mic`, `reopen_mic`, a
   volta do sono), não o microfone escolhido para o ditado. No Alienware gravou a LifeCam (zero
   digital, −98 dBFS) em vez do NVIDIA Broadcast.
2. O "ms of speech" conta um segundo quando o canal mais alto passa de 0,01 RMS (−40 dBFS). Medido
   nos três `sys.opus` retidos: a fala da call no canal do sistema fica em −54 dBFS de mediana e
   −44 no máximo (janelas de 250 ms). Só o som de início do Fala (0,21 RMS no 1º segundo) passava:
   daí ~1,25 s em reuniões de 16, 43 e 100 s, e o "Recorded by mistake?".
3. A UI não diz qual microfone a reunião grava.

## Portas

- D1: a reunião resolve o mic como o ditado (`clamshell_microphone`, `selected_microphone`, canal
  escolhido), pelo `AudioRecordingManager`; o escolhido sumido cai no padrão com `warn`. O
  `MIC_HUB` (chave = nome) segue com um stream só para os dois.
- D2: trocar o mic em Configurações durante a reunião troca o canal do mic na hora (mesma
  `set_mic_rate` da volta do sono); a falha mantém o antigo. A queda de stream reabre o escolhido
  e, com outra taxa, passa pelo `set_mic_rate` em vez de ficar mudo.
- D3 (recado das 00:10): um segundo tem som quando qualquer canal passa do piso de silêncio
  (`SILENCE_RMS`, −60 dBFS); "Gravou por engano?" só abaixo de 10 s gravados ou com menos de 3 s de
  som (o som de início ocupa 1 s). O agrupamento 4 Hz → 1 s sai do `on_report` para um tipo
  testável.
- D5 (recado das 01:05): o card que expira, ou sai da tela sem resposta, mantém e transcreve; só
  Descartar apaga; uma reunião já transcrita não é enviada de novo.
- D4: `MeetingStatus.mic_input` com o nome do mic aberto; o painel mostra "Microfone: <nome>".

## Fora

O aviso de mic mudo em ~10 s com ação de troca: PR seguinte.
