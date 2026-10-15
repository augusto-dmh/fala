# meeting-transcribe-retry — a transcrição pós-reunião nunca simplesmente não acontece

## Problem

`transcribe_if_keyed` (`apps/desktop/src/meeting/mod.rs`) só registra um `warn` quando a Scribe
falha. Ninguém lê `AsrError::Network { retriable }`, e `MeetingError::from` o achata em
`Transcription(String)`. Com rede caída, `429` ou `5xx`, a reunião fica sem transcrição até alguém
clicar, e nada na reunião diz por quê. A ADR-0005 diz que nada se perde se a transcrição falhar e
que o Parakeet local é o fallback offline; `transcribe_with_fallback`/`LocalMeeting` existem no
crate `asr`, mas o desktop nunca os chama. O Granola espera a rede voltar, gera as notas sozinho e,
no erro, mostra "Your original notes and transcript are safe" com [Retry].

## Flow

1. `fala-asr::meeting::retry` (novo, puro): `Failure::of(&AsrError)` → `Offline` (sem resposta
   HTTP), `Busy` (`429`/`5xx`), `Final`. `RetryQueue<K>`: `failed(key, failure, now) → RetryAt |
   GiveUp`, `later`, `forget`, `take_due`, `next_due`, `wake`. Sem rede: 10 s, 30 s,
   1 min e daí a cada 5 min, até 8 falhas (quase sempre o envio falha na conexão, antes de subir
   áudio; uma conexão que cai depois do envio parece igual, daí o limite). Provedor: 1, 5 e 15 min,
   e desiste.
2. `MeetingError::Unreachable { status, reason }` (novo) para `Network { retriable: true }`, com
   texto próprio na UI (`meeting.errors.unreachable`).
3. `meeting/retry.rs` (novo): `transcript_failed` decide pela fila (`Busy`/`StillRecording` → de
   novo em 30 s sem contar falha; cancelado, sem áudio, sem chave → sai), grava
   `audio/<id>/transcript-pending` (vazio, ou o número da tomada) e emite
   `ProgressStage::Waiting { reason }`. Uma thread `meeting-retry` dorme até a próxima tentativa.
   Gatilhos: o backoff; `SleepEvent::Resumed` (`wake_retries`, 10 s depois); a próxima abertura
   (`resume_pending_transcripts` no `spawn_upkeep`). `transcribe_with`, no sucesso, tira da fila o
   trabalho que a transcrição cobre (uma tomada só cobre a própria). Quando a transcrição chega por
   uma nova tentativa ou por um clique numa reunião que esperava (ou tinha falhado sozinha), rodam
   as notas automáticas que o parar rodaria (nunca numa importação).
4. `transcribe_if_keyed` / `transcribe_take_if_keyed` continuam os pontos de entrada; a falha vai
   para `transcript_failed` em vez do `warn`. `finish` não muda.
5. `MeetingDetail.auto_transcript: TranscriptWait` (`waiting { reason: offline | provider | queued }`
   ou `failed { error }`). A barra da reunião diz "Sem conexão — tentaremos de novo sozinhos…" /
   "A ElevenLabs não respondeu…" / "A transcrição não deu certo. O áudio e suas notas estão salvos.",
   e o botão vira "Tentar agora" / "Tentar de novo".

## Slices

O diff inteiro passa de 1000 linhas; vão dois PRs empilhados:

1. `fix/meeting-transcribe-retry`: passos 1–4 e o tipo do passo 5 (`auto_transcript`,
   `ProgressStage::Waiting`, em `bindings.ts`). A reunião volta a ser transcrita sozinha e as notas
   seguem; critérios R1–R4, R6, R7.
2. `fix/meeting-transcribe-retry-2`: a UI do passo 5 (R5), a sonda de rede (uma conexão TCP no
   host da Scribe a cada 10 s enquanto há sessão sem rede adianta a tentativa) e o fallback local pelo Parakeet (sem rede
   depois de N tentativas, erro não-retriable, desistência do provedor, e sem chave: Q28 ligada pelo
   orquestrador, ADR-0023 proposta), com a transcrição marcada como local e "Transcrever na nuvem".

## Fatia 2 — fallback local, sonda e estado na reunião

6. Sonda (`reachable`: TCP no host da Scribe, DNS com prazo, nada enviado) a cada 10 s enquanto há
   sessão sem rede; só uma que conecta depois de falhar adianta as sessões sem rede
   (`wake_offline`). Sem rede desiste na 4ª falha.
7. Desistir da nuvem (sem rede 4×, `429`/`5xx` 4×, erro final) vira trabalho local
   (`WaitReason::Local`, `Route::Local`); falha local é definitiva. `401`/`403` → `InvalidKey`:
   a reunião pede outra chave. `batch` em `TranscriptionManager` serializa `transcribe` (ditado e
   janela da reunião esperam um ao outro); o adaptador recarrega o modelo descarregado.
8. `route`: nuvem com chave; local sem chave (Q28, ADR-0023 proposta) ou "só local". `local.rs`
   decodifica o Opus em blocos de 10 min a 16 kHz e zera o 1º segundo do sistema numa gravação
   (som de início). As notas devidas a uma transcrição local esperam a rede (até 1 h).
9. `LocalMeeting`: pico de cada janela a −6 dBFS (ganho 1×–100×), janela abaixo de −60 dBFS fora
   (a call chega a −54 dBFS).
10. `meeting_local_transcripts` marca a transcrição local; `MeetingDetail.transcript_local`; a
    barra mostra espera, falha, "Transcrita no computador" e "Transcrever na nuvem" com a chave.

## Impact

| Front | What changes |
| --- | --- |
| `crates/asr` | `meeting/retry.rs` novo, exportado |
| `apps/desktop` | `meeting/retry.rs` novo; `MeetingError::Unreachable`; `ProgressStage::Waiting`; `MeetingDetail.auto_transcript`; ganchos no sono e no `spawn_upkeep` |
| UI | `transcriptWait` em `meetingModel.ts`; `SessionBar` com status e rótulo; `MeetingsPage` trata `waiting`; strings pt/en (fatia 2) |
| `crates/storage` | `meeting_local_transcripts` (fatia 2) |
| `docs/decisions` | ADR-0023 proposta: transcrição local sem chave (fatia 2) |
