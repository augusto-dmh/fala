# meeting-detection-polish — navegador só conta como call com uma reunião na janela, e "Parar a gravação?" não insiste

## Problem

`CALL_APPS` trata `msedge.exe`, `chrome.exe` e os outros navegadores como app de call pelo exe.
Qualquer uso do mic pelo navegador (modo de voz do claude.ai, um teste de mic) abre "Gravar
reunião?". E depois de [Continuar] no "Parar a gravação?", um app que larga e pega o mic de novo
(reconexão do Teams) repete a pergunta sem limite. O Wispr só conta hosts de conferência na faixa
de abas e suprime o "parar?" por 3 min, no máximo 2 vezes por sessão (`research/notes/23-wispr.md`
§ 3).

## Flow

1. `fala_meeting::web_call(titles)` (new) - títulos → `WebCall` ("Google Meet", "Microsoft
   Teams", "Zoom", "Webex", "Whereby", "Jitsi", "Discord", "Slack" pelo "Huddle"); o Meet por
   prefixo "Meet - "/"Meet – " ou "Google Meet", os outros por palavra inteira com caixa exata
2. `CallApp::is_browser` (new) - os 10 navegadores de `CALL_APPS`
3. `CallDetector::observe(now, &[CallHolder], ctx)` (changed) - `CallHolder { app, web }`; a posse
   de um navegador guarda a última reunião vista e só conta como call com ela (aviso, "parar?",
   e o "todos largaram" ignoram navegador sem reunião); trocar de aba sem largar o mic não tira a
   reunião da posse
4. `may_ask_stop(asked, now)` (new) - no máximo 2 "parar?" por gravação, 3 min entre eles,
   contados do instante da pergunta; zera quando a gravação acaba
5. `call_prompt::watch` (changed) - para um navegador com o mic, lê os títulos das janelas de
   topo visíveis dos processos com o exe do navegador (`EnumWindows`, `cfg(windows)`), passa a
   `web_call` e descarta; fora do Windows, lista vazia
6. `CallPromptPayload.web_name` (new) e chaves `callPrompt.{offer,askStop}.titleWeb` - "Parece
   uma chamada do Google Meet no Edge"; [Não avisar] segue silenciando o navegador

## Decisions

- Títulos por exe, não por PID: o processo que captura num Chromium é o serviço de áudio, filho do
  processo dono das janelas; o exe pega todas as janelas do navegador sem depender da árvore.
- Supressão contada do instante da pergunta, não do clique: o detector não vê a resposta, e o
  cartão fecha em 20 s (até 2 min com o ponteiro em cima).
- "Meet" só como prefixo do título; como palavra solta pegaria "Meetup", "Meet the team".

## Impact

| Front | What changes |
| --- | --- |
| domain | `CallHolder`, `WebCall`, `web_call`, `may_ask_stop`, `ASK_STOP_QUIET`, `ASK_STOP_MAX`; `CallPrompt` ganha `web` |
| behaviour | navegador com o mic sem reunião na janela não avisa nem segura o "parar?"; "parar?" ≤ 2 por gravação, 3 min entre eles |
| ui | título do cartão com a reunião e o navegador; descrição da configuração cita a reunião no navegador |

## Relations / Surface / Landing

`None` - nada guardado muda; `web_name` só vai à janela do aviso; reverter é tirar o código.

## Criteria

1. `web_call` SHALL name Google Meet for "Meet - abc-defg-hij - Microsoft Edge" and nothing for "Claude - Microsoft Edge"
2. A browser holding the mic SHALL get no offer until a window shows a meeting, then one offer naming it
3. While recording, a browser that showed a meeting SHALL keep counting as a call until it releases the mic; one that never did SHALL not count
4. `AskStop` SHALL be returned at most 2 times per recording and never within 3 min of the previous one
5. The card SHALL read "Parece uma chamada do Google Meet no Edge" for a Meet in Edge
6. Window titles SHALL not be stored, sent, or logged (ADR-0012)
