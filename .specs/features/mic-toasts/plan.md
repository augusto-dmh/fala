# mic-toasts — erro visível no gesto e avisos de microfone na pill

## Problem

Quem dita só olha para a pill. Hoje três falhas do gesto acabam num toast da janela principal, que
fica escondida (`start_hidden`), e a pill simplesmente some:

- o microfone falha ao abrir (`recording-error` em `actions.rs`: permissão negada, nenhum
  dispositivo, ou outro erro, que inclui o mic ocupado por outro app) — item S10 do plano delta;
- não há modelo baixado: `TranscribeAction::start` desiste com um `warn!` e o carregamento em
  segundo plano emite `loading_failed` com o texto inglês fixo `"Model not downloaded"` — S11;
- o microfone está mudo (mudo no Windows, chave do headset, ou acesso negado aos apps de desktop,
  que entrega silêncio): com o VAD ligado, a gravação sai vazia e é descartada sem nenhum sinal;
  com o VAD desligado, o silêncio é transcrito e entra no histórico como entrada vazia.

A pesquisa de UI (`fala-research/research/20-ui-wispr-granola.md` § 3.1) aponta o retorno do Wispr
como o que ele resolve melhor: "Using <mic>" ao começar, "Is your microphone muted?" com [Select
microphone] e [Troubleshoot] quando não captou áudio, e ditado vazio fora do histórico. A síntese
de 2026-10-09 (§ 5 item 1) junta isso a S10/S11 numa feature `mic-toasts` (S). O vocabulário de
erro foi decidido em D3 do plano delta e aplicado aos 19 min pela `session-limit` (#30): som
distinto + estado de cor na pill. A fonte não traz números de incidência.

Quando isto entrar, cada falha do gesto aparece onde a pessoa está olhando: um aviso curto na
janela da pill, com som de erro quando o ditado nem começou, e o ditado mudo ou vazio nunca vira
entrada no histórico.

## Flow

Reusa a janela `recording_overlay` (já criada no boot, inclusive com a pill desligada), a
reprodução de sons de `audio_feedback`, o atraso de esconder por geração (`OVERLAY_SHOW_GENERATION`)
e o comando existente que abre a privacidade do microfone no Windows.

1. key-down -> `TranscribeAction::start` (exists) - sem modelo utilizável, chama `overlay::show_notice(ModelMissing)` (new) e `audio_feedback::play_error_chime` (new) antes do `return`
2. `try_start_recording` falha (exists) - no lugar de `hide_recording_overlay`, mostra `MicDenied`, `NoMic` ou `MicFailed` pela mesma classificação de hoje e toca o som de erro; o evento `recording-error` para a janela principal continua
3. `try_start_recording` dá certo com a pill compacta (exists) - se o nome do microfone aberto (`DictationRecorder::device_name`, new) difere do último anunciado nesta execução, `show_notice(MicInUse)` empilha "Usando <mic>" sobre a pill por 1,5 s
4. `Processor` (exists, `dictation_capture.rs`) - mede, enquanto grava, o RMS de cada bloco cru do mic (`SignalMeter`, new); `stop` devolve as amostras e o veredito `MicSignal` (`Heard`, `Silent`, `Unknown`)
5. `TranscribeAction::stop` (exists) - depois da checagem de cancelamento, `Silent` mostra `MicMuted` e encerra sem WAV, sem transcrição, sem colar e sem histórico; amostras vazias seguem o descarte silencioso de hoje; transcrição vazia (só espaço) encerra sem colar e sem histórico e apaga o WAV
6. `show_notice` (new, `overlay.rs`) - aumenta a janela para o tamanho do aviso, mostra-a mesmo com a pill desligada quando o aviso é sozinho, emite `overlay-notice` e agenda o `hide` para o fim da duração se nenhuma sessão nova aparecer
7. `RecordingOverlay` (exists, front) - `Notice` (new) desenha o aviso traduzido pelo código, sozinho ou empilhado sobre a pill; [Escolher microfone] chama `open_microphone_settings` (new) e [Resolver] chama `open_microphone_troubleshooting` (new)
8. `model-state-changed` (exists) - ganha `error_code` (`model_not_downloaded`, `model_not_found`), e o toast da janela principal traduz o código em vez de mostrar o texto inglês

## Impact

| Front | What changes |
| --- | --- |
| domain | termo novo: aviso da pill (`NoticeKind`) — `mic_in_use`, `mic_muted`, `mic_denied`, `no_mic`, `mic_failed`, `model_missing`. Vive em `overlay.rs`; o front só conhece o código |
| domain | termo novo: `MicSignal` — veredito da gravação: `Silent` quando houve ao menos 0,5 s de áudio cru e nenhum bloco passou de RMS 0,001 (−60 dBFS); `Unknown` abaixo de 0,5 s; `Heard` no resto. Vive em `dictation_capture.rs` |
| domain | termo existente: `DictationRecorder::stop` e `AudioRecordingManager::stop_recording` passam a devolver o veredito junto das amostras. Quem ramifica nelas hoje: o `stop` do `TranscribeAction` (único chamador de `stop_recording`), o fechamento do recorder em `managers/audio.rs` (descarta o retorno) e os testes do `Processor` |
| domain | termo existente: `ModelStateEvent` ganha `error_code` opcional; os demais construtores mandam `None`. Lido por `App.tsx` e pelo `RecordingOverlay` (que só olha `event_type`) |
| behaviour | erro ao abrir o mic e modelo ausente tocam o som de fim do tema duas vezes (150 ms), mesmo com `audio_feedback` desligado, e mostram um aviso de 5 s (com ações: 6 s) na janela da pill, mesmo com a pill desligada |
| behaviour | gravação com o mic mudo não é transcrita: mostra "O microfone está mudo?" por 6 s, sem som extra (o som de fim normal segue `audio_feedback`) |
| behaviour | transcrição vazia não cola nada, não entra no histórico e não deixa WAV; antes entrava como entrada vazia |
| ui | janela do overlay ganha o tamanho do aviso (340×110 lógicos) enquanto um aviso aparece; volta ao tamanho do estado no próximo `show-overlay` |
| ui | oito chaves novas em `overlay.notice.*` e duas em `errors.*` (pt e en) |
| stored data | nada a migrar; nenhuma chave nova no store |

## Relations

`None - no stored-data shape change`

## Surface

`None - nothing consumed outside`. `overlay-notice`, `open_microphone_settings`,
`open_microphone_troubleshooting` e o campo `error_code` são lidos só pelo frontend deste app.

## Landing

`None` - nada aqui é caro de reverter: eventos e comandos internos, campo opcional novo num evento
interno, constantes em memória, nenhum dado gravado.

- Nothing else in this change is hard to reverse

## Criteria

### S1: o aviso de erro no gesto (P1)

**Acceptance Criteria**

1. WHEN the dictation key is pressed and no model can transcribe (`get_model_path` fails) THEN the desktop SHALL show the `model_missing` notice alone and SHALL play the error chime, and SHALL not open the microphone
2. WHEN `try_start_recording` fails THEN the desktop SHALL show `mic_denied`, `no_mic` or `mic_failed` alone, chosen by the same classification as `recording-error`, SHALL play the error chime, and SHALL still emit `recording-error`
3. The error chime SHALL be the sound theme's stop chime twice, 150 ms apart, played even with `audio_feedback` off
4. WHEN a notice is shown alone THEN the desktop SHALL show the overlay window even with `overlay_style` = none, and SHALL hide it after the notice's duration unless a newer show happened
5. The notice durations SHALL be 1.5 s for `mic_in_use`, 6 s for the notices with actions (`mic_muted`, `mic_denied`, `no_mic`, `mic_failed`) and 5 s for `model_missing`
6. WHEN the model load fails because the model is not downloaded, or because the selected model is unknown or empty, THEN `model-state-changed` SHALL carry `error_code: "model_not_downloaded"` or `"model_not_found"`, and the main window toast SHALL show the translated text for that code instead of the backend's English message

**Independent test:** `cargo test -p fala --lib overlay::tests`; `bun src/overlay/notice.test.tsx`; no Windows, store portátil com o modelo apagado: apertar o atalho toca dois toques e mostra "Nenhum modelo de voz baixado".

### S2: microfone mudo e ditado vazio (P1)

**Acceptance Criteria**

7. WHILE recording, the capture SHALL report `Silent` only when at least 0.5 s of raw audio arrived and no block's RMS reached 0.001, `Unknown` under 0.5 s, and `Heard` otherwise; the pre-buffer before the key-down SHALL not count
8. WHEN a stopped, non-cancelled recording is `Silent` THEN the desktop SHALL show the `mic_muted` notice alone and SHALL not save a WAV, transcribe, paste or write history
9. WHEN a stopped recording is not `Silent` and its transcription is empty or only whitespace THEN the desktop SHALL not paste, SHALL not write history or metrics, and SHALL delete the WAV it saved
10. WHEN a recording is not `Silent` and the VAD kept no audio THEN the desktop SHALL keep today's silent discard (no notice, no history)

**Independent test:** `cargo test -p fala --lib dictation_capture`, `cargo test -p fala --lib actions::tests`; no Windows, mutar o microfone nas configurações de som e ditar: aparece "O microfone está mudo?" e o histórico não muda.

### S3: o aviso na pill (P2)

**Acceptance Criteria**

11. WHEN the compact pill is recording and the opened microphone's name differs from the last one announced in this run THEN the desktop SHALL stack `mic_in_use` with that name over the pill; the Live streaming panel and a disabled overlay SHALL get no `mic_in_use`
12. The `mic_muted`, `mic_denied`, `no_mic` and `mic_failed` notices SHALL offer [Escolher microfone], which hides the overlay and opens Configurações > Geral (the microphone selector) in the main window, and [Resolver], which hides the overlay and on Windows opens the microphone privacy page when access is denied and the sound settings page otherwise; off Windows it opens the microphone selector
13. Every notice text SHALL come from i18next (`overlay.notice.*`, pt-BR source and en), translated from the code; the notice SHALL draw an amber dot for `mic_muted`, a red dot for the errors and no dot for `mic_in_use`, with literal colours like the pill
14. WHEN the overlay hides or shows a new `recording` or `streaming` session THEN the notice SHALL disappear

**Independent test:** `bun src/overlay/notice.test.tsx`, `bun run lint`, `bunx tsc --noEmit`, `bun run check:translations`.

## Out of scope

| Excluded | Why |
| --- | --- |
| Notificação do SO com a pill desligada (D3 b) | não há plugin de notificação no desktop; adicionar `tauri-plugin-notification` é dependência nova com toasts do WinRT que só identificam o app instalado. O aviso na janela da pill, mostrada mesmo com a pill desligada, cobre o caso (decisão listada para o Augusto) |
| "Usando <mic>" no painel Live com streaming | o painel cresce com o texto ao vivo; empilhar o aviso nele mexe na geometria do Live. Os modelos padrão não fazem streaming e usam a pill compacta |
| Anel de contagem regressiva e × no aviso | cosmético; o aviso fecha sozinho e as ações fecham a janela |
| Pill em repouso com tooltip do atalho (20 § 3.3) | a pill não existe em repouso; um modo de repouso é um estado novo da janela, não um tooltip barato |
| "Microfone ›" na bandeja | já existe (#78) |
| Ação [Baixar modelo] no aviso de modelo ausente | o texto diz o que falta; a tela de modelos está a um clique na janela principal |
| Detectar o mic ocupado à parte do erro genérico | o `cpal` não distingue; cai em `mic_failed` |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Limiar do mic mudo | RMS de bloco < 0,001 (−60 dBFS) em toda a gravação, com no mínimo 0,5 s | o mudo do Windows e o acesso negado entregam zeros; fala a qualquer distância razoável passa de −40 dBFS. Um falso positivo numa sala silenciosa com supressão de ruído só troca o descarte silencioso por um aviso | n — decidido pelo executor; listado no PR |
| Quando anunciar "Usando <mic>" | só quando o nome do mic aberto muda em relação ao último anunciado nesta execução (o primeiro ditado depois de abrir o app também anuncia) | 1,5 s em todo ditado cansa; a troca é o momento em que a informação importa | n — decidido pelo executor |
| Som do aviso de mic mudo | nenhum além do som de fim normal | a gravação existiu e terminou; o aviso aparece mesmo com a pill desligada | n — decidido pelo executor |
| Ditado com transcrição vazia | não cola, não entra no histórico, apaga o WAV | "ditado vazio nunca entra no histórico" (20 § 3.1). A falha de transcrição (erro) continua salvando a entrada vazia para re-transcrever, como hoje | n — decidido pelo executor |

**Open questions:** none - all resolved or logged above.

## Observable

| Surface | Decision | Landing |
| --- | --- | --- |
| gesto | sem modelo | AC 1 |
| gesto | mic não abre (3 classes) | AC 2 |
| som | som de erro | AC 3 |
| som | mic mudo | Assumptions (nenhum som extra) |
| janela da pill | pill desligada | AC 4 |
| janela da pill | duração | AC 5 |
| janela principal | toast do modelo ausente | AC 6 |
| captura | veredito do sinal | AC 7 |
| histórico | mic mudo | AC 8 |
| histórico | transcrição vazia | AC 9 |
| histórico | VAD sem fala | AC 10 (existing) |
| histórico | transcrição que falhou | existing - entrada vazia para re-transcrever |
| pill | "Usando <mic>" | AC 11 |
| aviso | ações | AC 12 |
| aviso | texto e cor | AC 13 |
| aviso | some | AC 14 |
| ação destrutiva | apagar o WAV da transcrição vazia | n/a - o áudio não tinha texto e não é referenciado por nenhuma entrada |

## Sources

- `fala-research/plans/fase-1-delta-e-semanas-3-4.md` - S10, S11, D3
- `fala-research/plans/status/sintese-2026-10-09.md` § 5 itens 1 e 4
- `fala-research/research/20-ui-wispr-granola.md` § 3.1, § 3.3
