# meeting-auto-stop — a call que acaba sem resposta para a gravação, com [Retomar]

## Problem

Quando todos os apps de call largam o mic, com ≥ 3 min gravados, o Fala pergunta "Parar a
gravação?". Expirar fecha o card e a gravação segue: quem saiu da call grava até o silêncio de
15 min, ou até o teto de 3 h se um vídeo toca. O Granola para sozinho quando o app larga o mic
(`research/28` § 3 a).

## Flow

1. `fala_meeting::CallQuiet` (new, `detect.rs`) - armado quando o "parar?" aparece; qualquer
   relato do canal do sistema ≥ `CALL_QUIET_RMS` (≈ −70 dBFS) ou uma pausa/suspensão o desarma
   para sempre; `quiet()` diz se ainda vale parar
2. `Input::CallEnded` (new, `session.rs`) - em `Recording`, para com `StopReason::Silence`; fora
   dele, nada
3. `call_prompt::show` (changed) - o `AskStop` arma o `CallQuiet` da gravação ativa e leva
   `session_id` e `auto_stop` no payload; `MeetingManager::on_report` desarma ao ouvir som e
   reenvia o card sem `auto_stop`
4. `answer_action(AskStop, Expired)` (changed) - com `auto_stop` e sessão → `StopCallEnded(id)`;
   sem → `Close`, como hoje. `MeetingManager::stop_call_ended(id)` confere de novo (mesma sessão,
   `Recording`, ainda calado), loga `call ended` e aplica `CallEnded`
5. `CallPromptKind::CallEnded` (new) - "Parei a gravação · A call terminou." [Retomar] [Fechar],
   2 min; aparece no lugar do "parou por silêncio" quando a parada foi da call. [Retomar] →
   `resume_session(id)` (o continuar do #195), esperando a retenção acabar se ela ainda roda
6. `finish` (changed, mínimo) - depois de uma parada da call, a transcrição espera o card
   [Retomar] sair da tela (≤ 2 min, como os ~10 s do Granola); se a pessoa retomou, não transcreve
   agora: a sessão nunca transcrita vai inteira na parada do take
7. Card (`src/callprompt`) - com `auto_stop`, o corpo "Paro a gravação em {{s}} s" conta os
   segundos e [Continuar gravando] vem primeiro; o card `call_ended`; textos pt/en

## Decisions

- Piso `CALL_QUIET_RMS` = 0,000 316 (−70 dBFS), não `SILENCE_RMS` (−60): nas 3 reuniões retidas a
  call fica em −45…−55 dBFS por janela de 250 ms, 25 % das janelas entre −60 e −70; o sistema
  calado fica em ≈ −100. Errar para baixo só mantém a gravação (o comportamento de hoje).
- A janela começa quando o card aparece (largada + `RELEASE_DELAY`), não no instante da largada:
  o som de "saiu da call" do app toca junto da largada e não pode valer como "sistema com som".
- `stop_reason = 'silence'` (o CHECK do schema não muda); o log diz `call ended`.
- Som no meio da contagem: o card volta a ser "Parar a gravação?" e a contagem recomeça (20 s);
  expirar então só fecha.
- Pausada ou suspensa a gravação durante a contagem: não para (a pessoa está lá, ou não há nível).

## Impact

| Front | What changes |
| --- | --- |
| domain | `CallQuiet`, `CALL_QUIET_RMS`, `Input::CallEnded` |
| behaviour | "parar?" que expira com o sistema calado para a gravação; [Retomar] grava na mesma sessão |
| ui | corpo com contagem, card "Parei a gravação" |

## Relations / Surface / Landing

`None` - nada guardado muda (`stop_reason` reusa `silence`); reverter é tirar o código.

## Criteria

1. An `AskStop` that expires SHALL stop the recording when the system channel stayed below
   `CALL_QUIET_RMS` since the card appeared, and only close the card otherwise
2. Pausing, sleeping, a different session or a stopped one SHALL never be stopped by an expiry
3. The stop SHALL be stored as `silence` and logged as `call ended`
4. After that stop the card SHALL offer [Retomar], which records a take of the same session
5. The "parar?" limit (2 per recording, 3 min apart) SHALL stay as it is
6. Nothing SHALL start recording without a click (ADR-0005)
