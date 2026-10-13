# dictation-during-meeting — ditar durante a reunião gravada, com o mic da reunião zerado

## Problem

O ditado é recusado enquanto uma reunião grava ou está pausada (`dictation_refusal`,
`meeting_active`), e ditar no chat da própria call é o caso que a ADR-0015 diz que bloquear
atrapalha. Hoje o ditado e o gravador abrem dois `Mic` independentes, e o gravador segura 250 ms,
menos que o pré-buffer de 300 ms do ditado. Definição: `fala-research/plans/features/
dictation-during-meeting.md` (atualizada em 2026-10-12, `research/24` § 3).

## Flow

1. `fala_audio::MicHub` abre **um** stream por dispositivo (chave: nome do dispositivo) numa thread
   dona do `Mic`, que entrega quadros intercalados; cada `MicTap` reduz ao seu canal (ou à média) e
   tem a API do `Mic` (`drain_into`, `sample_rate`, `failed`, `dropped`, `name`). O último tap a
   cair fecha o stream. Um feed com falha não é reaproveitado.
2. `MeetingRecorder` segura o canal mic em memória, **antes** do resample, por `MIC_HOLD` (450 ms) de
   amostras; `begin_omit(lookback)` abre um intervalo em `mic_in − lookback` (amostras da fonte já
   empurradas), `end_omit(margin)` fecha em `mic_in + margin`. Amostra do intervalo vira zero antes
   de sair do hold, então nada do ditado chega ao WAV (nem num crash: o hold nunca foi escrito).
   Intervalos que se sobrepõem viram um. `LAG_FRAMES` sobe para 700 ms (hold + os 250 ms de
   antes) e `MAX_BACKLOG_FRAMES` para 1 s.
3. O recorder anota cada intervalo em frames do WAV (onde a primeira e a última amostra zerada
   entram na fila) e devolve os fechados em `take_omitted()`.
4. Desktop: o ditado e o gravador abrem pelo mesmo `MicHub`; o início da gravação do ditado manda
   `BeginOmit` ao gravador (lookback 400 ms = pré-buffer 300 ms + margem 100 ms, menos o tempo já
   pausado), o fim (soltar, cancelar, descartar) manda `EndOmit` (margem 50 ms). O ditado longo é um
   só intervalo. Recusa só no modo presencial.
5. Níveis: o relatório do gravador diz se houve intervalo omitido no período; `MuteWatch`, o
   contador de 15 min da sessão e o "Gravou por engano?" não contam o mic nesse período (nem
   silêncio nem fala).
6. Pílula: enquanto dita numa reunião, um cartão "Mic da reunião pausado enquanto você dita" sobre a
   pílula de ditado.
7. Marcador: cada intervalo fechado vira uma linha "trecho de ditado omitido" do canal mic, salva
   na hora (sobrevive a crash), que entra na transcrição, no SQLite, no `.md`, no MCP e no payload
   das notas (ADR-0016).

## Slices (PRs empilhados)

| PR | Branch | Conteúdo |
| --- | --- | --- |
| 1 | `feat/dictation-during-meeting` | 1–3: `MicHub`, `Mic` intercalado, hold/omit no `MeetingRecorder`, Code Map |
| 2 | `feat/dictation-during-meeting-2` | 4–6: desktop pelo hub, omit, recusa só presencial, níveis, pílula |
| 3 | `feat/dictation-during-meeting-3` | 7: marcador de ponta a ponta |

## Criteria

1. Dois consumidores do mesmo dispositivo abrem um stream só (contador de aberturas = 1); um tap
   com canal escolhido e outro com média recebem o mesmo tempo.
2. Nenhuma amostra marcada como ditado chega ao WAV, mesmo com o intervalo aberto antes do que já
   foi empurrado (pré-buffer), cruzando o fim (`finish`) e num "crash" (recorder solto sem
   `finish`).
3. Um intervalo começado antes de `lookback` alcançável zera o que ainda está no hold e conta o
   que vazou (`ChannelStats::omit_late`).
4. Ditado durante reunião no modo "reunião" não é recusado; no presencial é.
5. Um ditado de 3 min numa call em que só a pessoa fala não arma "Ainda está gravando?" nem o aviso
   de canal mudo.
6. O marcador aparece na transcrição, no SQLite, no `.md` e no payload das notas.

## Out of scope

Ditado em "só sistema" (já normal), reter o mic completo, mudar o pipeline do ditado, liberar o
modo presencial, começar uma reunião durante um ditado (continua recusado: `DictationActive`).

## Assumptions

| Assumption | Chosen default | Confirmed? |
| --- | --- | --- |
| Presencial continua bloqueado | sim (definição, "Fora de escopo") | y — definição |
| Hold antes do resample, por contagem de amostras | sim: exato e independe do relógio | n — executor |
| Lookback 400 ms, margem final 50 ms | sim: pré-buffer 300 ms + jitter | n — executor |
| Reunião segue no mic padrão | sim (como hoje); dispositivo diferente zera igual | n — executor |
| Três PRs empilhados | sim: o gate de 1000 linhas | n — executor |
