# meeting-transcribe-retry — checks

| # | Criterion | Proof | Kind |
| --- | --- | --- | --- |
| R1 | Falhas sem rede e `429`/`5xx` viram nova tentativa com backoff; `401`/resposta inválida não repetem | `cargo test -p fala-asr --lib meeting::retry` (5 testes); `cargo test -p fala-asr --test meeting_scribe retry` (servidor falso: conexão recusada, conexão fechada, `429`, `503`, depois `200`) | test |
| R2 | Sem rede desiste na 8ª falha; provedor desiste depois de 3 novas tentativas; acordar adianta tudo | `offline_backs_off_then_gives_up`, `busy_gives_up_after_three_retries`, `waking_brings_every_retry_forward` | test |
| R3 | Erro → próximo passo no desktop (repetir, mais tarde, largar); tomada vs. sessão inteira, e uma tomada só limpa o próprio trabalho; marcador para a próxima abertura | `cargo test -p fala --lib meeting::retry` (5 testes) | test |
| R4 | `transcribe_if_keyed` segue o ponto de entrada; `finish` sem mudança | revisão do diff (`git diff origin/main -- apps/desktop/src/meeting/mod.rs`) | review |
| R5 | (fatia 2) Estado visível: esperando (sem rede / provedor / na fila) e falha definitiva com [Tentar de novo] | `bun src/components/meeting/retry.test.tsx` (W1–W3) | test |
| R6 | As notas automáticas seguem a transcrição que chegou depois, uma vez só, e nunca numa importação | revisão: `retry`/`transcribe` → `notes_owed` → `spawn_owed_notes` → `auto_notes` (que já tem `auto_notes_due`); o `finish` continua rodando as suas | review |
| R8 | Um clique que falha sem rede mostra texto próprio | `bun src/components/meeting/meeting.test.tsx` (`errorKey` de `unreachable`) | test |
| R7 | Nada transcrito em log acima de debug | revisão: os logs novos só levam id, erro (status HTTP) e segundos | review |

| L1 | (fatia 2) A sonda só abre conexão, com prazo no DNS; sem rede desiste na 4ª falha | `cargo test -p fala-asr --lib meeting::retry` (`the_probe_only_opens_a_connection`, `offline_backs_off_then_gives_up`) | test |
| L2 | (fatia 2) Áudio a −50 dBFS chega ao modelo a −10 dBFS (+40 dB); abaixo de −60 dBFS não vai ao modelo; nada clipa | `cargo test -p fala-asr --lib meeting::local` (`quiet_windows_are_raised_and_silent_ones_skipped`) | test |
| L3 | (fatia 2) O Opus retido volta a 16 kHz com o som de início zerado (só numa gravação) | `cargo test -p fala --lib meeting::local` (2 testes) | test |
| L4 | (fatia 2) A transcrição local fica marcada até a da nuvem a substituir; sai com a sessão | `cargo test -p fala-storage --test meetings local_transcript_is_marked_until_the_cloud_replaces_it` | test |
| L5 | (fatia 2) Sem chave, nada abre conexão | `cargo test -p fala-asr --test meeting_scribe local_only_opens_no_connection` (já existia) + revisão de `route` | test + review |
| L6 | (fatia 2) Local marcada na barra; "Transcrever na nuvem" só com a chave; "Transcrever" sem exigir chave | `bun src/components/meeting/retry.test.tsx` (W4), `bun src/components/meeting/meetingKeys.test.tsx` | test |

## Manual (não bloqueia)

1. App portátil com a chave real, Wi-Fi desligado, parar uma reunião curta: a barra diz "Sem
   conexão — tentaremos de novo sozinhos…"; o log mostra `retrying in 15 s`, `60 s`, `300 s`.
2. Religar o Wi-Fi: na tentativa seguinte (ou em "Tentar agora") a transcrição chega e, com uma
   reunião de 3 min+, as notas vêm sozinhas.
3. Com a reunião esperando, fechar o app e abrir de novo: 10 s depois do início ela tenta de novo.
4. Suspender e acordar o PC com uma reunião esperando: tenta 10 s depois de acordar.
5. (fatia 2) Sem a chave da ElevenLabs, gravar 1 min de um vídeo (só o sistema): a transcrição sai
   do Parakeet, a barra diz "Transcrita no computador…"; guardar a chave e clicar "Transcrever na
   nuvem" troca a transcrição e o rótulo some.
6. (fatia 2) Com a chave e o Wi-Fi desligado: depois de ~2 min a barra diz "transcrevendo no
   computador…" e a transcrição local chega.
