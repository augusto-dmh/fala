# session-limit — aviso aos 19 min, corte aos 20, sem perder o texto

## Problem

Uma gravação de ditado não tem fim enquanto ninguém a encerra. Com o gesto da 1.F1, dois toques
travam a gravação sem as mãos até o próximo toque, e o modo `toggle` herdado faz o mesmo. Quem
trava o ditado e se distrai (levanta, atende uma ligação, esquece o atalho) deixa o microfone
aberto por tempo indeterminado. Hoje não existe nenhum limite: o delta procurou por
`max_record`, `recording_limit` e `1200` em `apps/desktop/src` sem achar nada; só existe um
cronômetro visual no overlay de streaming (`front:overlay/RecordingOverlay.tsx`, item W4).

Quem paga é a própria pessoa que dita: uma sessão esquecida acumula áudio que nunca vira texto
útil, prende o pipeline e, quando finalmente é encerrada, cola de uma vez um bloco enorme no app
em foco. O pitch da fase 1 pede "limite de sessão" nas semanas 3-4, e o design doc (§3.3) fixa a
regra herdada do Wispr Flow: "aviso aos 19 min e corte aos 20". A pesquisa de referência confirma
o mesmo comportamento no Wispr ("Aviso aos 19 min, corte automático aos 20 min de sessão",
`fala-research/research/01-referencia-wispr-flow.md`). A bug bar da fase 1 proíbe "perda de
texto ditado", então o corte não pode descartar o que foi dito. A fonte não traz números de
incidência de sessões esquecidas.

Quando isto entrar, qualquer gravação contínua de ditado, em qualquer modo de ativação e
inclusive a travada por dois toques, avisa aos 19 min com dois toques do som de início e com o
ponto da pill em âmbar, e aos 20 min para sozinha. O que foi dito até ali é transcrito, colado e
guardado no histórico exatamente como num encerramento pelo atalho.

## Flow

Reusa o laço de prazos do `CoordinatorState` que a 1.F1 criou (`next_deadline`/`on_deadline`,
dormindo em `recv_timeout`), o efeito `Stop` que já leva ao `TranscribeAction::stop` com a entrega
tudo-ou-nada da 1.F2 (`deliver_unless_cancelled`), e a reprodução de som de `audio_feedback`. Não
cria uma thread de timer nem um segundo caminho de parada.

1. key-down -> `CoordinatorState::begin_recording` (exists) - além do binding, guarda no `Hold` o instante do key-down e o atalho; `session_deadline()` (new, no door - placement) devolve key-down + 19 min enquanto o aviso não saiu, e key-down + 20 min depois dele
2. laço do `TranscriptionCoordinator` (exists) - `recv_timeout` passa a dormir até `wake_deadline()`, o menor entre `next_deadline()` (carência e janela de dois toques, com o sentido da 1.F1) e `session_deadline()`
3. `CoordinatorState::on_deadline` (exists) - aos 19 min emite o efeito novo `LimitWarning` uma vez e segue gravando; aos 20 min emite `Stop` com o binding e o atalho da gravação e vai a `Processing`
4. `run_effect` (exists) - `LimitWarning` toca o aviso por `audio_feedback` (exists) e emite `recording-limit-warning` à janela do overlay por `overlay` (exists); `Stop` segue para `TranscribeAction::stop` (exists, sem mudança)
5. `RecordingOverlay` (exists, front) - recebe o evento e liga `limitWarning` até esconder ou até a próxima gravação; no overlay Live o ponto da esquerda fica âmbar, e na pill mínima da fase 1 (`Pill`, exists desde #28) a cápsula gravando fica âmbar por `pillTone` (`pillModel.ts`, exists)
6. out: o ditado cortado é transcrito, colado e gravado no histórico por `deliver_unless_cancelled` (exists, 1.F2), com o som de fim conforme `audio_feedback`

## Impact

| Front | What changes |
| --- | --- |
| domain | termo novo: `SESSION_WARNING` (19 min) e `SESSION_LIMIT` (20 min) - marcas de uma gravação contínua, contadas do key-down que a abriu. Vivem em `transcription_coordinator.rs` |
| domain | termo novo: `LimitWarning` - efeito do coordenador que anuncia o corte próximo sem mudar o estágio. Só `run_effect` o consome; o helper de teste herdado `drive`, que faz `match` exaustivo em `Effect`, ganha um braço `panic!` como a 1.F1 fez para `Discard` |
| domain | termo existente: o laço do coordenador dormia até `next_deadline()` e passa a dormir até `wake_deadline()`. `next_deadline()` mantém o sentido da 1.F1 (carência e janela) porque testes da 1.F1 afirmam `None` com a gravação travada; quem ramifica nele hoje é só o laço e esses testes |
| behaviour | não existe mais gravação de ditado com mais de 20 min; uma sessão esquecida é encerrada e colada aos 20 min. O mesmo vale para push-to-talk segurado, `hold_or_toggle` e `toggle` |
| ui | o overlay Live ganha um estado âmbar no ponto da esquerda (`--color-warning`, que já existe no tema), e a pill mínima da fase 1 (#28) ganha a cápsula âmbar `#d97706` (o mesmo âmbar do tema claro, literal porque a pill tem as mesmas cores nos dois temas), por cima do vermelho da tecla segurada; sem texto. O vocabulário visual final é da feature E |
| stored data | nada a migrar; nenhuma chave nova no store |

## Relations

`None - no stored-data shape change`

## Surface

`None - nothing consumed outside`. O evento `recording-limit-warning` vai só para a janela
`recording_overlay` deste app, como `recording-ready` e `mic-level`.

## Landing

`None` - nada aqui é caro de reverter: as marcas de 19 e 20 min são constantes em memória, o
efeito `LimitWarning` é interno ao desktop, o nome do evento é lido só pelo overlay deste
frontend (o delta registra essa porta como interna e reversível), e o terceiro prazo do laço segue
o padrão de prazos que a 1.F1 já estabeleceu.

- Nothing else in this change is hard to reverse

## Criteria

### S1: o prazo da sessão no coordenador (P1)

A máquina pura decide, com relógio sintético e sem esperar de verdade, quando avisar e quando
cortar.

**Acceptance Criteria**

1. WHILE a dictation recording is in progress, in any activation mode (held push-to-talk, `push_to_talk_double_tap` locked by two taps, `hold_or_toggle` locked, `toggle`), WHEN 19 min have elapsed since the key-down that started it THEN the coordinator SHALL emit `LimitWarning` with the recording's binding and SHALL stay in `Recording`
2. The coordinator SHALL emit `LimitWarning` at most once per recording
3. WHEN 20 min have elapsed since that key-down THEN the coordinator SHALL emit `Stop` with the recording's binding and hotkey string and SHALL move to `Processing`, never emitting `Discard` for it
4. IF the coordinator first wakes after 20 min without having warned THEN it SHALL emit `Stop` directly, with no `LimitWarning` for that recording
5. WHILE recording, the coordinator SHALL wake at the earliest of the release grace, the double-tap window and the next session mark (19 min before the warning, 20 min after it), and `next_deadline()` SHALL keep reporting only the grace and the window
6. WHEN a recording ends before 19 min by stop, discard, cancel or a failed start THEN the coordinator SHALL have no session deadline left
7. WHEN the drain starts a press remembered during processing THEN the coordinator SHALL measure both marks of that recording from that press's key-down
8. WHILE the pipeline processes a recording cut at 20 min, WHEN the push-to-talk key that was still held comes up THEN the coordinator SHALL emit nothing, and the drain SHALL start nothing
9. WHILE the pipeline processes a recording cut at 20 min in `push_to_talk_double_tap`, WHEN a tap (press and release under 300 ms) arrives THEN the drain SHALL start nothing

**Independent test:** `cargo test -p fala --lib transcription_coordinator` com relógio sintético.

### S2: o aviso e o corte no app (P1)

**Acceptance Criteria**

10. WHEN the coordinator emits `LimitWarning` THEN the desktop SHALL play the sound theme's start chime twice, 150 ms apart, at the feedback volume and output device, even with `audio_feedback` off
11. WHEN the coordinator emits `LimitWarning` and the overlay is enabled THEN the desktop SHALL emit `recording-limit-warning` to the `recording_overlay` window, and WHEN the overlay is disabled THEN it SHALL emit nothing
12. WHEN the coordinator emits the 20 min `Stop` THEN the desktop SHALL run the same `TranscribeAction::stop` as a stop from the shortcut, so the text is pasted and the history entry saved together through `deliver_unless_cancelled`, or neither if the dictation was cancelled

**Independent test:** no Windows, travar um ditado com dois toques e deixar 20 min com um podcast tocando perto do microfone: aos 19 min a pill fica âmbar e soam dois toques; aos 20 a pill passa a "transcrevendo" e o texto cai no Bloco de Notas, com entrada nova no histórico.

### S3: a pill âmbar (P2)

**Acceptance Criteria**

13. WHEN the overlay receives `recording-limit-warning` THEN the pill's left dot SHALL turn amber (`--color-warning`) with a faster pulse
14. WHEN the overlay hides, or shows a new `recording` or `streaming` session THEN the dot SHALL return to the regular accent
15. WHILE the minimal pill (#28) is recording after `recording-limit-warning`, the system SHALL draw the capsule amber (`#d97706`, over the hold red) with a 0.9 s pulse between opacity 1 and 0.6 (still under `prefers-reduced-motion`), SHALL keep the ten bars and no text, and SHALL draw the processing pill black as before

**Independent test:** `bun run lint`, `bunx tsc --noEmit`, `bun src/overlay/pill.test.tsx`; visual na sessão Windows.

## Out of scope

| Excluded | Why |
| --- | --- |
| Limite configurável nas configurações | o pitch e o design doc fixam 19 e 20 min; ninguém pediu ajuste |
| Notificação do SO quando a pill está desligada (D3 b do delta) | não há plugin de notificação no desktop; adicionar um é dependência nova (ARCHITECTURE.md). O som do aviso cobre a pill desligada |
| Som de aviso próprio (arquivo novo) | o vocabulário de sons e cores da pill é da feature E (D3); até lá o aviso reusa o som de início do tema, tocado duas vezes |
| Cronômetro ou contagem regressiva na pill | a pill do pitch não tem texto; o estado âmbar é o aviso |
| Continuar ditando numa sessão nova depois do corte | não pedido; a pessoa abre outro ditado pelo atalho |
| Limite na gravação de reunião | outro fluxo (fase 2, ADR-0005) |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Quais gravações têm limite | toda gravação de ditado, em qualquer modo, não só a travada por dois toques | o design doc fala em "corte aos 20" sem restringir modo; um push-to-talk segurado por 20 min é raro mas também é contínuo, e uma regra só é mais simples de provar | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| De onde contam os 19 e 20 min | do key-down que abriu a gravação, inclusive o key-down lembrado que o drain inicia | o `Hold` já guarda esse instante; a diferença para o início real no drain é o tempo de processamento anterior (segundos) e corta um pouco antes, nunca depois | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| Som do aviso | o som de início do tema, duas vezes, 150 ms entre elas, tocado mesmo com `audio_feedback` desligado (padrão `false`) | D3 a do delta pede som distinto; o toque duplo se distingue do início sem arquivo novo. Com os sons desligados por padrão e a pill desligada no Linux, respeitar `audio_feedback` deixaria o aviso mudo para a maioria; ele toca uma vez por sessão de 19 min | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| Aviso com `mute_while_recording` ligado | o som não é ouvido (a saída do sistema está muda durante a gravação); fica só a pill | desmutar no meio da gravação mudaria o comportamento de quem escolheu o mudo; o corte entrega o texto de qualquer jeito | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| Toque atrasado depois do corte em `hold_or_toggle` ou `toggle`, e gatilho externo (SIGUSR2, CLI) | segue a regra herdada de toque durante o processamento: pode enfileirar uma sessão nova no drain | são modos fora do padrão; no padrão `push_to_talk_double_tap` o toque atrasado é esquecido (AC 9). Nada se perde: a sessão nova aparece na pill e é encerrada como qualquer outra | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| Push-to-talk ainda segurado no corte com o SO repetindo key-down | a repetição durante o processamento é lembrada (regra herdada de "segurando durante o processamento") e o drain abre uma gravação nova que termina ao soltar | comportamento herdado da 1.F1; nada do ditado cortado se perde. Sem repetição (o caso provado no AC 8) não abre nada | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |

**Open questions:** none - all resolved or logged above.

## Observable

| Surface | Decision | Landing |
| --- | --- | --- |
| gravação de ditado (todos os modos) | quando avisa e quando corta | AC 1-4 |
| gravação de ditado | o que acontece com o que foi dito no corte | AC 3, AC 12 (processado e colado, nunca descartado) |
| gravação de ditado | Esc durante o processamento de um ditado cortado | existing - regra da 1.F2: cancela sem colar e sem histórico (`deliver_unless_cancelled`) |
| gravação de ditado | toque no atalho depois do corte | AC 8, AC 9; modos fora do padrão em Assumptions |
| som | o que se ouve no aviso | AC 10 |
| som | o que se ouve no corte | existing - o som de fim de `TranscribeAction::stop`, conforme `audio_feedback` |
| pill (overlay) | estado de aviso | AC 13 |
| pill (overlay) | volta ao normal | AC 14 |
| pill mínima (#28) | estado de aviso | AC 15 |
| pill (overlay) | pill desligada (padrão no Linux) | AC 11 (sem evento); o som do AC 10 cobre |
| pill (overlay) | texto visível novo | n/a - o aviso não tem texto, então não há chave de i18n |
| ação destrutiva | confirmação antes do corte | n/a - o corte não destrói nada: entrega o texto como um encerramento normal |

## Sources

- `fala-research/pitches/fase-1-ditado-windows.md` - "limite de sessão" nas semanas 3-4
- `docs/design/2026-10-fala-v1.md` §3.3 - "aviso aos 19 min e corte aos 20"
- `fala-research/plans/fase-1-delta-e-semanas-3-4.md` - item W4, feature F3 e decisão D3 (som distinto + estado âmbar; o corte transcreve e insere)
