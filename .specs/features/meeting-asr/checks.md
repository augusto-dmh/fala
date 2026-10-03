# meeting-asr checks

Profile: light
Plan: `.specs/features/meeting-asr/plan.md`

17 checks in 5 slices · 5 one-way doors · 2 open, of which 0 block (1 blocks go-live)

Escritos nesta rodada sem build: `crates/asr` é do painel `pipeline-audio` até o PR da trilha A entrar. Os nomes de teste abaixo são o contrato para quem construir. Todas as provas rodam com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`. O servidor falso é um `std::net::TcpListener` em `127.0.0.1:0` dentro do teste, que guarda cada requisição crua e responde um JSON fixo; nenhum teste fala com a rede de verdade.

## Checks

### S1 - só áudio de reunião chega à rede · ~2 files · ~10 KB · ~3k

**C1** - `ElevenLabsScribe` não implementa `Transcriber` (`assert_not_impl_any!`), e `ElevenLabsScribe::transcribe_session` recebe `&MeetingRecording` (AC 1)
Proof: `cargo test -p fala-asr --lib meeting::guard::tests::scribe_is_not_a_dictation_transcriber -- --exact`

**C2** - `from_session_dir` com os dois arquivos devolve os caminhos `mic.opus`/`sys.opus` e o modo; sem `sys.opus` devolve `MissingChannel("sys.opus")`, sem `mic.opus` devolve `MissingChannel("mic.opus")` (AC 2)
Proof: `cargo test -p fala-asr --lib meeting::guard::tests::from_session_dir_needs_both_channels -- --exact`

### S2 - a requisição literal · ~3 files · ~25 KB · ~7k

**C3** - Sessão `meeting` contra o servidor falso: 2 requisições `POST /v1/speech-to-text`; a do mic carrega os bytes de `mic.opus` e `diarize=false`, a do sistema os de `sys.opus` e `diarize=true`; as duas têm `model_id=scribe_v2`, `language_code=por`, `timestamps_granularity=word`, e o conjunto de nomes de campo é exatamente `{model_id, file, language_code, diarize, timestamps_granularity}` com `keyterms` desligado (AC 3)
Proof: `cargo test -p fala-asr --test meeting_scribe request::meeting_sends_exactly_the_enumerated_fields -- --exact`

**C4** - Com `keyterms` ligado e dicionário `["Fala", "ADR"]`, cada requisição tem exatamente 2 campos `keyterms` com `Fala` e `ADR`; desligado, 0 (AC 4)
Proof: `cargo test -p fala-asr --test meeting_scribe request::keyterms_follow_the_setting -- --exact`

**C5** - Sessão `in_person`: a requisição do mic tem `diarize=true` (AC 5)
Proof: `cargo test -p fala-asr --test meeting_scribe request::in_person_diarizes_the_mic -- --exact`

**C6** - Idioma "detectar": nenhuma requisição tem o campo `language_code` (AC 6)
Proof: `cargo test -p fala-asr --test meeting_scribe request::auto_language_omits_language_code -- --exact`

**C7** - A chave `sk-teste-123` aparece só no header `xi-api-key` da requisição crua (nem na URL nem no corpo), e o `to_string()` do erro de uma resposta `401` não contém `sk-teste-123` (AC 7)
Proof: `cargo test -p fala-asr --test meeting_scribe request::key_only_in_header_and_never_in_errors -- --exact`

### S3 - segmentos "Eu / Pessoa N" · ~2 files · ~15 KB · ~4k

**C8** - Uma resposta com palavras de `speaker_0`, `speaker_0`, `speaker_1` e um `audio_event` vira 2 segmentos: o 1º com `t0_ms`/`t1_ms` da 1ª e da 2ª palavra, o 2º com a 3ª; o `audio_event` não cria falante (AC 8)
Proof: `cargo test -p fala-asr --lib meeting::segments::tests::groups_consecutive_words_by_speaker -- --exact`

**C9** - Modo `meeting`: todo segmento do mic tem `speaker = me`; no sistema, `speaker_7` e depois `speaker_2` viram `person 1` e `person 2` (AC 9)
Proof: `cargo test -p fala-asr --lib meeting::segments::tests::mic_is_me_system_numbered_by_first_appearance -- --exact`

**C10** - Modo `in_person` com 2 falantes no mic e 1 no sistema: mic vira `person 1` e `person 2`, sistema vira `person 3` (AC 10)
Proof: `cargo test -p fala-asr --lib meeting::segments::tests::in_person_numbering_continues_across_channels -- --exact`

**C11** - Merge de mic `[0, 5000]`, `[9000]` e sistema `[2000]`, `[5000]`: ordem `t0` 0 (mic), 2000 (sys), 5000 (mic), 5000 (sys), 9000 (mic) (AC 11)
Proof: `cargo test -p fala-asr --lib meeting::segments::tests::merge_orders_by_t0_mic_first_on_tie -- --exact`

**C12** - `Segment` do mic com `me` serializa como `{"channel":"mic","speaker":"me","t0_ms":0,"t1_ms":1200,"text":"oi"}` e um do sistema com `person 2` como `{"channel":"system","speaker":{"person":2},...}`, e os dois voltam iguais (AC 12)
Proof: `cargo test -p fala-asr --lib meeting::segments::tests::segment_serialized_form -- --exact`

### S4 - falha, progresso e cancelamento · ~2 files · ~15 KB · ~4k

**C13** - Respostas `429`, `503` e conexão fechada sem resposta dão `Network { retriable: true }`; `401` e `422` dão `retriable: false` com o status; nos 5 casos, `mic.opus` e `sys.opus` têm o mesmo sha256 de antes (AC 13)
Proof: `cargo test -p fala-asr --test meeting_scribe progress::failures_keep_audio_and_classify_retry -- --exact`

**C14** - Servidor falso que responde depois de 3 s: o callback de progresso é chamado pelo menos 3 vezes e nenhum intervalo entre chamadas passa de 1 s (AC 14)
Proof: `cargo test -p fala-asr --test meeting_scribe progress::reports_at_least_every_second -- --exact`

**C15** - Cancelar 500 ms depois do início, com o servidor demorando 5 s: `Cancelled` volta em menos de 1,5 s desde o início (AC 15)
Proof: `cargo test -p fala-asr --test meeting_scribe progress::cancel_returns_within_a_second -- --exact`

### S5 - fallback local · ~2 files · ~15 KB · ~4k

**C16** - Sessão "só local": o servidor falso recebe 0 conexões, e um áudio de 130 s vira janelas que começam em 0, 60 000 e 120 000 ms, com os `t0_ms` deslocados por isso (AC 16)
Proof: `cargo test -p fala-asr --lib meeting::local::tests::windows_of_60_s_shift_timestamps -- --exact`
Proof: `cargo test -p fala-asr --test meeting_scribe local::local_only_opens_no_connection -- --exact`

**C17** - Depois de `Network { retriable: true }`, `transcribe_with_fallback` devolve os segmentos do caminho local (transcritor falso no lugar do Parakeet) (AC 17)
Proof: `cargo test -p fala-asr --lib meeting::local::tests::network_error_falls_back_to_local -- --exact`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| campos multipart (6) | `model_id` C3 · `file` C3 · `language_code` C3, C6 · `diarize` C3, C5 · `timestamps_granularity` C3 · `keyterms` C4 | - |
| statuses de `POST https://api.elevenlabs.io/v1/speech-to-text` (6) | `200` C3 · `401` C7, C13 · `422` C13 · `429` C13 · `5xx` C13 · erro de rede C13 | - |
| `SessionMode` que gravam (3) | `meeting` C3, C9 · `in_person` C5, C10 · `system_only` C9 (só o canal do sistema tem fala) | - |
| `Speaker` (2) | `me` C9 · `person` C9, C10 | - |
| `channel` (2) | `mic` C12 · `system` C12 | - |
| tipos de item da resposta (3) | `word` C8 · `spacing` C8 · `audio_event` C8 | - |
| caminhos de transcrição (3) | nuvem C3 · "só local" C16 · fallback por erro C17 | - |
| Landing doors (5) | tipo de entrada C1, C2 · módulo C1 · requisição literal C3-C7 · forma do `Segment` C12 · `ureq` C3 | - |

- C3, C4, C5, C6, C7 e C13 cruzam a fronteira HTTP de verdade (servidor falso em TCP), não um mock do cliente

## Swept

- validation: C2
- failure modes: C13, C15
- idempotency: C13 - o áudio fica intacto, então "tentar de novo" repete a mesma requisição
- authorization: C7 - chave BYOK só no header (ADR-0008)
- concurrency: n/a - os dois canais são enviados em sequência nesta fatia; paralelizar é otimização reversível
- data lifecycle: C13 - nada é apagado na falha; apagar é da 2.F3
- dependency failure: C13, C17
- state transitions: n/a - sem máquina de estados própria; o estado da sessão é da 2.F1
- observability: C14 - progresso ≤ 1 s (orçamento do `ARCHITECTURE.md`); nenhum log com chave ou conteúdo (C7)

## Handoff

- S1-S5 ≈ 30k tokens (módulo novo em `crates/asr` de ~5 arquivos, ~800 linhas com o servidor falso; `crates/asr` hoje tem ~4 KB), sob o orçamento de 150k - one builder
- Mechanism: one builder (cabe no orçamento; sem pergunta)
- **Onde parou (2026-10-02):** plano e checks escritos pelo painel `meeting-core`, sem build. Começa quando o PR da trilha A (`crates/asr`) e o da B (`fala-secrets`, `ureq`) estiverem em `main`. Primeiro passo do build: fechar a pergunta aberta 1 do plano na documentação oficial da ElevenLabs e, se um nome de campo mudar, corrigir a door 3 antes do código.
