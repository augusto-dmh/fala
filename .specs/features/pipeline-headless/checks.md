# pipeline-headless checks

Profile: light
Plan: `.specs/features/pipeline-headless/plan.md`

28 checks em 4 grupos (S1–S3 e as doors) · 4 one-way doors · 0 open

Todo `cargo` roda com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`
(omitido abaixo). Os proofs `--ignored` leem `FALA_TEST_PARAKEET_DIR` (pasta
`parakeet-tdt-0.6b-v3-int8`) e `FALA_TEST_SPEECH_WAV` (fala real em pt-BR com pelo menos uma
pausa de 1 s, ≥ 10 s), como os testes do `bench`.

## Checks

### S1 - utterances a 16 kHz · crates/audio

**C1** - Reamostrar 48 000 e 44 100 Hz entrega, depois de esvaziado, `in_len × 16000 / in_rate` amostras ± 480 (AC 1)
Proof: `cargo test -p fala-audio --lib resample::tests::output_length_matches_ratio`

**C2** - Um seno de 440 Hz a 48 kHz sai a 16 kHz com 440 Hz ± 1 % por cruzamentos de zero (AC 2)
Proof: `cargo test -p fala-audio --lib resample::tests::sine_keeps_its_frequency`

**C3** - `start` abre a sessão com as últimas 4 800 amostras anteriores, em ordem, seguidas das posteriores sem lacuna nem repetição (AC 3)
Proof: `cargo test -p fala-audio --lib capture::tests::prebuffer_opens_session_with_last_300_ms`

**C4** - Com menos de 4 800 amostras antes do `start`, a sessão abre com todas elas (AC 4)
Proof: `cargo test -p fala-audio --lib capture::tests::short_prebuffer_keeps_everything`

**C5** - 2 quadros de voz abrem uma utterance que começa 15 quadros (450 ms) antes do primeiro quadro de voz, limitada ao início da sessão (AC 5)
Proof: `cargo test -p fala-audio --lib capture::tests::onset_includes_450_ms_pre_roll`
Proof: `cargo test -p fala-audio --lib capture::tests::pre_roll_stops_at_session_start`
Proof: `cargo test -p fala-audio --lib capture::tests::single_voiced_frame_opens_nothing`

**C6** - 15 quadros de silêncio depois de voz fecham a utterance e a entregam antes do `stop` (AC 6)
Proof: `cargo test -p fala-audio --lib capture::tests::silence_of_450_ms_closes_utterance`

**C7** - Voz contínua é cortada em utterances de exatamente 240 000 amostras, e a concatenação é igual à entrada (AC 7)
Proof: `cargo test -p fala-audio --lib capture::tests::continuous_speech_splits_at_15_s_without_loss`

**C8** - `stop` com utterance aberta a entrega com todas as amostras até o `stop`, incluindo as retidas no resampler (AC 8)
Proof: `cargo test -p fala-audio --lib capture::tests::stop_flushes_open_utterance_and_resampler`

**C9** - Sessão sem voz: `stop` não entrega nenhuma utterance (AC 9)
Proof: `cargo test -p fala-audio --lib capture::tests::no_voice_no_utterance`

**C10** - O Silero v4 versionado dá 0 utterances em 3 s de silêncio digital (AC 10)
Proof: `cargo test -p fala-audio --test silero silence_gives_no_utterance`

**C11** - O Silero v4 versionado dá pelo menos 1 utterance na fala real de `FALA_TEST_SPEECH_WAV` (AC 10)
Proof: `cargo test -p fala-audio --test silero speech_gives_an_utterance -- --ignored`

### S2 - Parakeet · crates/asr

**C12** - `Parakeet::transcribe` devolve texto não vazio e o idioma pedido (`PtBr` e `En`) para fala real (AC 11)
Proof: `cargo test -p fala-asr --test parakeet transcribes_real_speech -- --ignored`

**C13** - `Parakeet::load` numa pasta inexistente e numa pasta vazia falha com `AsrError::ModelLoad` cujo texto contém o caminho (AC 12)
Proof: `cargo test -p fala-asr --test parakeet missing_model_names_the_path`

**C14** - Com 1 599 amostras a inferência não é chamada e o texto é vazio; com 1 600 ela é chamada uma vez (AC 13)
Proof: `cargo test -p fala-asr --lib tests::short_audio_skips_inference`

### S3 - `fala-cli dictate` · apps/cli

**C15** - `dictate --wav` de 3 s de silêncio leva ≥ 2,9 s de parede, imprime exatamente uma linha vazia no stdout e sai com 0 (AC 14, AC 16)
Proof: `cargo test -p fala-cli --test dictate wav_is_paced_and_silence_prints_empty_line -- --ignored`

**C16** - Os textos das utterances saem numa linha, aparados, na ordem das utterances, separados por um espaço, mesmo quando a transcrição da utterance 1 termina depois da 2 (AC 15)
Proof: `cargo test -p fala-cli --bin fala-cli dictate::tests::texts_join_in_utterance_order`
Proof: `cargo test -p fala-cli --test dictate speech_prints_one_line -- --ignored`

**C17** - Num ditado de fala real com ≥ 2 utterances, toda linha `event=utterance` menos a última tem `before_release=1` (AC 17)
Proof: `cargo test -p fala-cli --test dictate utterances_transcribe_before_release -- --ignored`

**C18** - Com `FALA_TRACE=1`, o stderr tem uma linha `event=utterance` por utterance e uma `event=dictation` por ditado, com exatamente as chaves da door 3 e valores inteiros, e nenhuma palavra do texto impresso no stdout (AC 18)
Proof: `cargo test -p fala-cli --test dictate trace_lines_follow_door_3 -- --ignored`

**C19** - Sem `FALA_TRACE`, nenhuma linha do stderr contém `event=` (AC 19)
Proof: `cargo test -p fala-cli --test dictate no_trace_without_env -- --ignored`

**C20** - No modo mic, uma linha do stdin começa o ditado, a seguinte termina; EOF termina o ditado aberto e encerra (AC 20)
Proof: `cargo test -p fala-cli --bin fala-cli dictate::tests::stdin_lines_toggle_and_eof_ends`

**C21** - `--model` sem o Parakeet, `--vad` ilegível e `--wav` ilegível saem com 2 e o caminho no stderr, sem abrir o mic (AC 21)
Proof: `cargo test -p fala-cli --test dictate bad_paths_exit_2`

**C22** - `--mic` sem correspondência sai com 2 e a mensagem "dispositivos de entrada" (AC 22)
Proof: `cargo test -p fala-cli --test dictate unknown_mic_exits_2`

**C26** - Com `FALA_TRACE=1`, o stderr tem exatamente uma linha `event=load` com as chaves `model_ms` e `vad_ms` inteiras, antes da primeira `event=utterance` (AC 23, Landing 4; adicionado pelo pedido do Lux de 2026-10-02)
Proof: `cargo test -p fala-cli --test dictate load_is_traced_apart -- --ignored`

**C27** - Um erro de inferência no meio de um ditado faz o soltar falhar com código de saída 1 e a mensagem do erro (Surface, exit 1; adicionado na rodada 2 da verificação)
Proof: `cargo test -p fala-cli --bin fala-cli dictate::tests::inference_error_exits_1`

**C28** - `--language xx` sai com 2, com `--language` e o valor no stderr e nada no stdout, sem precisar do modelo (AC 24; adicionado na rodada 2 da verificação)
Proof: `cargo test -p fala-cli --test dictate unknown_language_exits_2`

### Doors

**C23** - `crates/audio` e `crates/asr` declaram exatamente as dependências da door 1 e da door 2, nenhuma depende de `sherpa`, e nenhum crate depende de `tauri` (Landing 1, 2)
Proof: `cargo test -p fala-audio --test manifest dependencies_match_door_1`
Proof: `cargo test -p fala-asr --test manifest dependencies_match_door_2`
Proof: `scripts/check-no-tauri-in-crates.sh`

**C24** - O trait `Transcriber` tem a forma literal da door 2 e é object-safe (`Box<dyn Transcriber>`) (Landing 2)
Proof: `cargo test -p fala-asr --test trait_shape stub_implements_transcriber`

**C25** - A ADR-0009 existe em `docs/decisions/` com `status: proposed` e a ADR-0003 não muda nesta branch (Impact, ADR)
Proof: `test "$(sed -n 2p docs/decisions/0009-*.md)" = "status: proposed" && git diff --quiet feat/core-contract..HEAD -- docs/decisions/0003-asr-do-ditado-local-parakeet-via-sherpa-onnx.md`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| `fala-cli dictate` exit codes (3) | 0 C15 · 1 C27 · 2 C21, C22, C28 | - |
| taxas de entrada do resampler (2) | 48 000 C1 · 44 100 C1 | - |
| fronteiras do pré-buffer (2) | ≥ 4 800 antes C3 · < 4 800 antes C4 | - |
| transições da utterance (4) | abre por onset C5 · fecha por silêncio C6 · corta aos 15 s C7 · fecha no `stop` C8 | - |
| fronteira de 100 ms (2) | 1 599 C14 · 1 600 C14 | - |
| entradas inválidas de `dictate` (5) | `--model` C21 · `--vad` C21 · `--wav` C21 · `--mic` C22 · `--language` C28 | - |
| idiomas (2) | `PtBr` C12 · `En` C12 | - |
| eventos de trace (3) | `utterance` C18 · `dictation` C18 · `load` C26 | - |
| doors do Landing (4) | 1 deps C23 · 2 trait C23, C24 · 3 trace C18 · 4 carga C26 | - |
| entradas do stdin no modo mic (3) | linha começa C20 · linha termina C20 · EOF C20 | - |

- Claims naming an exit code or output shape: C15, C16, C18, C19, C21, C22, C28 - cada um com proof que roda o binário; C27 fixa o código 1 na camada do `Session` (forçar falha de inferência no binário exigiria um modelo corrompido)
- C16 e C20 têm a decisão provada na própria camada (unit) porque o ponta a ponta com mic exige gravar a voz do Augusto; a rodada no mic fica para quando ele liberar

## Swept

- validation: C14, C21, C22, C28
- failure modes: C13, C21, C27 (erro de inferência sai com 1)
- idempotency: n/a - nenhum estado persiste; repetir o comando repete a saída
- authorization: n/a - comando local, sem rede e sem usuário
- concurrency: C16 (ordem com transcrição fora de ordem), C17 (ASR em paralelo com a gravação)
- data lifecycle: n/a - o áudio vive só em memória até a transcrição; nada é gravado em disco
- dependency failure: C13 (modelo ausente), C22 (dispositivo ausente)
- state transitions: C3, C8, C9, C20
- observability: C18, C19, C26; a meta de latência (≤ 700 ms, máx. 1,0 s) é medida pelo trace e relatada, não é check

## Handoff

- S1 ~ 6 arquivos novos em `crates/audio` (~35 KB) + leitura de `audio_toolkit/vad` e `resampler.rs` (~25 KB) ≈ 15k; S2 ~ 4 arquivos em `crates/asr` (~12 KB) ≈ 3k; S3 `apps/cli/src/dictate.rs` + `tests/dictate.rs` + `main.rs` (~40 KB) ≈ 10k; doors + ADR ≈ 3k. Total ≈ 31k tokens de arquivos, sob o budget de 150k - um builder, sem pergunta.
- Mechanism: one builder.
- Decidido pelo painel (Confirmed? y — delegado pelo Augusto em 2026-10-02): o modo mic não roda nos proofs; C16/C20 provam a decisão na própria camada e a latência é medida com `--wav` sobre um corte do corpus.
- Decidido pelo painel na retomada (Confirmed? y — delegado pelo Augusto em 2026-10-02, decidido pelo painel a pedido do Lux): a ADR-0009 reescrita para substituir a 0003 por inteiro, com a tabela afirmado × medido da auditoria da fase 0; o trace ganha `event=load` (door 4, AC 23, C26), aditivo. O trade study 05 não é citado porque ainda não foi escrito.
- Fixtures de fala (scratchpad, não versionados): `speech-pause.wav` = `ditado-2025-04-29.wav` 0,54–8,13 s + 1,2 s de zero digital + 8,67–14,5 s (14,6 s; os cortes do corpus não têm pausa ≥ 1 s); `phrase-8s.wav` = 0,40–8,30 s do mesmo corte, para a latência.
- Latência medida pelo painel (não é check), `FALA_TRACE=1` com o `fala-cli` em release, em 2026-10-02 ~21:28, com a máquina sob carga (load average 19 em 12 threads, swap 7/7 GB, outros painéis compilando): `phrase-8s.wav` (7,9 s, 2 utterances de 1,35 s e 6,55 s) deu `release_to_text_ms` 2180 e 1998, quase todo na última utterance (`asr_ms` 2179/1998, RTF ~0,31 contra 0,097 do benchmark com a máquina quieta); `speech-pause.wav` (14,6 s, 8,64 s + 5,98 s) deu 1119. A carga do modelo foi de 4,7-6,0 s (2,3 s no benchmark). O `before_release=1` da primeira utterance confirma a transcrição durante a gravação; a meta de 700 ms precisa ser medida de novo com a máquina quieta e no Windows.
