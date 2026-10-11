# meeting-detection — oferecer "Gravar?" quando uma call começa e perguntar "parar?" quando o app larga o mic

## Problem

"Esqueço de clicar Gravar no começo da reunião; o Granola me lembra" (pitch F8, fase 2). O Granola
e o Wispr notam a call pela sessão de **captura** de áudio de cada processo e só avisam: o Granola
espera 2 s, mostra um cartão de 404 × 104 no canto superior direito sem roubar foco, conta 20 s
(pausando no hover), deixa no mínimo 2 min entre avisos e tem "não avisar neste app". O fim da call
("o app largou o mic") é um sinal determinístico e barato; o Granola só o usa depois de 3 min de
gravação, e o Wispr, num caso ambíguo, segue gravando.

Quando isto entrar, o Fala avisa "Parece uma chamada no Teams. Gravar reunião?" ~2 s depois de o
app de call abrir o mic, e pergunta "Parar a gravação?" quando ele o larga. Nada grava nem para
sem clique (ADR-0005).

## Flow

1. `fala_audio::platform_activity()` (new, `crates/audio/src/activity.rs`) - a cada chamada,
   `IAudioSessionManager2` em cada endpoint de captura ativo; sessões em `AudioSessionStateActive`,
   sem a de sons do sistema → pids; um snapshot do ToolHelp dá o exe e até 4 ancestrais. Linux:
   `AudioError::Unsupported` (ADR-0007: o `cfg` fica no crate de plataforma)
2. `fala_meeting::identify(chain, own_exe)` (new) - exe → `CallApp` pela tabela do Granola; o
   `msedgewebview2.exe` passa a vez ao ancestral (Teams novo, WhatsApp); o Fala nunca é call
3. `fala_meeting::CallDetector::observe(now, apps, ctx)` (new) - lógica pura: posse do mic por
   app, 2 s de espera, 2 s de tolerância para sumir, um aviso por posse, 2 min entre avisos, nada
   gravando, nada para app silenciado ou com a chave desligada; quando todos os apps de call vistos
   na gravação largam o mic com ≥ 3 min gravados, `AskStop` uma vez
4. `call_watch` (new, `apps/desktop`) - thread que lê a atividade a 1 Hz com a chave ligada e
   passa ao detector o estado da reunião (`MeetingManager::status`)
5. `call_prompt` (new, `apps/desktop`) - janela `call_prompt` de 404 × 104 lógicos, sem moldura,
   sempre no topo, sem foco, no canto superior direito da área de trabalho do monitor principal
   (`x = direita − 408`, `y = topo + 4`), criada na primeira vez e reaproveitada; some sozinha em
   2 min
6. `src/callprompt` (new, front) - cartão com contagem de 20 s que pausa no hover e fecha sem
   gravar; [Gravar] [Agora não] [Não avisar no <app>], ou [Parar] [Continuar]
7. `call_prompt_answer` (new) - [Gravar] chama `MeetingManager::start_from_tray` (exists): sem o
   aviso de terceiros aceito, abre a janela nele em vez de gravar; [Parar] chama
   `MeetingManager::stop` (exists); [Não avisar] guarda o id em `call_prompt_muted_apps`
8. Configurações → Geral → Reuniões (new) - "Avisar quando uma chamada começar" (padrão ligado) e a
   lista "Não avisar nestes apps" com remover

## Impact

| Front | What changes |
| --- | --- |
| domain | termo novo: `MicHolder` (pid, exe, ancestrais) e trait `AudioActivity` em `fala-audio`; `AudioError::Unsupported` |
| domain | termo novo: `CallApp`, `CallDetector`, `CallPrompt` (`OfferRecording`, `AskStop`) em `fala-meeting`; nenhuma variante vira `UserAction` |
| behaviour | com a chave ligada, o desktop lê as sessões de captura a 1 Hz (só leitura, nenhum stream aberto) |
| ui | janela nova `call_prompt` (entrada `src/callprompt/index.html` no Vite, capability `default`) |
| ui | grupo "Reuniões" em Configurações → Geral |
| stored data | duas chaves novas em `settings`: `call_prompt_enabled` (padrão `true`) e `call_prompt_muted_apps` (padrão `[]`), com `serde(default)`; nada a migrar |

## Relations

`None - no stored-data shape change` além das duas chaves com padrão.

## Surface

`None - nothing consumed outside`. Comandos e eventos novos são lidos só pelo frontend deste app.

## Landing

`None` - nada caro de reverter: chaves de settings com padrão, janela e comandos internos.

## Criteria

### S1: quem captura o mic e o detector (P1)

1. `platform_activity().mic_holders()` SHALL list each process with an active capture session once, with the lowercase exe name and up to 4 ancestors, and SHALL open no stream; off Windows it SHALL return `Unsupported`
2. `identify` SHALL map the exe (or, for `msedgewebview2.exe`, the nearest known ancestor) to a `CallApp`, and SHALL return `None` for unknown apps and for any chain with `fala.exe` or the caller's own exe
3. WHEN a call app holds the mic for 2 s with no recording, the switch on and the app not muted THEN the detector SHALL return one `OfferRecording`, and no more for the same hold (a gap under 2 s is the same hold)
4. The detector SHALL return no offer within 2 min of the last one, while a meeting records or is paused, nor afterwards for a hold seen while recording
5. WHEN every call app seen while recording has released the mic for 2 s and ≥ 3 min are recorded THEN the detector SHALL return one `AskStop`; under 3 min, or with another call app still holding the mic, nothing
6. Driven by a whole simulated call with nobody clicking, a `MeetingSession` SHALL stay `Idle`; only `UserAction::StartRecording` SHALL make it record (ADR-0005)

### S2: o aviso no desktop (P1)

7. The prompt window SHALL be 404 × 104 logical, undecorated, always on top, not focusable, at `x = work area right − 408`, `y = work area top + 4` of the primary monitor
8. [Gravar] SHALL go through `MeetingManager::start_from_tray` (consent first); [Parar] through `MeetingManager::stop`; [Não avisar no <app>] SHALL add the app id to `call_prompt_muted_apps` once; [Agora não], [Continuar] and the 20 s expiry SHALL only close the prompt
9. An answer that does not match the prompt on screen SHALL do nothing
10. The countdown SHALL be 20 s, SHALL pause while hovered and SHALL close the prompt without recording at the end
11. Every visible string SHALL come from i18next (`callPrompt.*`, `settings.meetings.*`, pt and en)

## Out of scope

| Excluded | Why |
| --- | --- |
| Calendário (D4 = c) | decisão do Augusto: sem calendário nesta feature; o iCal secreto vira fatia nova se escolhido |
| Começar a gravar sozinho | ADR-0005 |
| "Disse tchau?" por LLM | fora do canal da ADR-0016 |
| Seguir o dispositivo do app de call | `loopback-device-select` |
| Saber qual aba do navegador é a call | o Windows só mostra o processo; o aviso diz "no Edge" |
| Adaptador Linux de `AudioActivity` | fase 3 (ADR-0007) |
| Arrastar para fechar e animação de mola | cosmético |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Leitura | polling de 1 s de `IAudioSessionManager2` em vez de `IAudioSessionEvents` | um enumerador novo por leitura não perde sessão e não guarda callbacks COM entre threads; 1 s + 2 s fica dentro dos ≤ 3 s da prova | n — decidido pelo executor |
| Chave desligada | não lê o SO nem pergunta "parar?" | quem desliga avisos não quer nenhum | n — decidido pelo executor |
| Navegadores | qualquer navegador com o mic conta como possível call ("no Edge") | é a regra do Granola para o Meet | n |
| [Gravar] | modo `Meeting` (fones), título vazio, como o item da bandeja | uma call é o caso do modo `Meeting` | n |
