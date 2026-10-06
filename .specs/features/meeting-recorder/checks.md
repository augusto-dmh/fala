# meeting-recorder checks

Profile: light
Plan: `.specs/features/meeting-recorder/plan.md`

18 checks em 4 grupos (S1–S3 e as doors) · 4 one-way doors · 0 open

Todo `cargo` roda com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`
(omitido abaixo). Os proofs `--ignored` gravam do PipeWire desta máquina e leem `FALA_TEST_SINK`
(o `node.name` do sink), como os do `record`.

## Checks

### S1 - WAV à prova de crash · crates/audio

**C1** - Pares escritos e um flush depois, um leitor aberto sem `finalize` (escritor esquecido com `std::mem::forget`) lê exatamente os pares escritos até o último flush; o intervalo de flush é 1 s (AC 1)
Proof: `cargo test -p fala-audio --lib meeting::wav::tests::killed_writer_leaves_flushed_frames`

**C2** - O WAV finalizado tem 48 000 Hz, 2 canais, 16 bits, L = mic e R = sistema (AC 2)
Proof: `cargo test -p fala-audio --lib meeting::wav::tests::finished_wav_is_48k_stereo_mic_left`

### S2 - o relógio manda · crates/audio

**C3** - Com o sistema mudo (fonte falsa sem frames) e o mic a 48 kHz, 10 s de relógio falso dão R só com zeros, L igual ao mic entregue, nenhum erro, e `relógio − 250 ms` de pares ± 1 tick (AC 3)
Proof: `cargo test -p fala-audio --lib meeting::recorder::tests::silent_system_is_filled_by_the_clock`

**C4** - Com o mic mudo, L sai com zeros do mesmo modo e R igual ao sistema entregue (AC 4)
Proof: `cargo test -p fala-audio --lib meeting::recorder::tests::silent_mic_is_filled_by_the_clock`

**C5** - Com os dois canais a 48 kHz no ritmo do relógio, L e R são exatamente as amostras entregues, em ordem, e o preenchimento é 0 nos dois (AC 5)
Proof: `cargo test -p fala-audio --lib meeting::recorder::tests::realtime_sources_are_written_verbatim`

**C6** - Um canal que entrega 2 s de uma vez com o relógio em 0,5 s descarta o excesso acima de 500 ms à frente do escrito, do mais antigo, e conta o descarte (AC 6)
Proof: `cargo test -p fala-audio --lib meeting::recorder::tests::backlog_over_500_ms_is_dropped_and_counted`

**C7** - Mic a 16 000 Hz e sistema a 44 100 Hz saem a 48 000 Hz, com a contagem de pares seguindo o relógio ± 480 frames (AC 7)
Proof: `cargo test -p fala-audio --lib meeting::recorder::tests::any_input_rate_is_resampled_to_48k`

**C8** - `finish` escreve até o relógio, sem a folga de 250 ms, e finaliza o WAV (AC 8)
Proof: `cargo test -p fala-audio --lib meeting::recorder::tests::finish_writes_up_to_the_clock`

**C9** - O `Resampler` ganha taxa de saída arbitrária e a saída de 16 000 → 48 000 Hz tem `in_len × 3` amostras ± 480 (AC 7)
Proof: `cargo test -p fala-audio --lib resample::tests::upsamples_to_48k`

### S3 - `fala-cli meeting` · apps/cli

**C10** - `meeting --out --system $FALA_TEST_SINK` com o stdin fechado 3 s depois da linha do indicador sai com 0 e deixa um WAV de 48 kHz estéreo com 3 s ± 0,5 s (AC 9)
Proof: `cargo test -p fala-cli --test meeting records_until_stdin_closes -- --ignored`

**C11** - `--system` sem correspondência, `--mic` sem correspondência e `--out` num diretório inexistente saem com 2, com o nome ou o caminho no stderr, sem criar o WAV (AC 10)
Proof: `cargo test -p fala-cli --test meeting bad_inputs_exit_2 -- --ignored`

**C12** - O stderr tem a linha do indicador `gravando reunião` antes do resumo do fim, e o progresso sai a cada 60 s (provado no `Progress` da CLI; o log em `run` não roda em teste, que exigiria 60 s de gravação) (AC 11)
Proof: `cargo test -p fala-cli --test meeting records_until_stdin_closes -- --ignored`
Proof: `cargo test -p fala-cli --bin fala-cli meeting::tests::progress_every_60_s`

**C15** - `meeting --out /dev/full` (o disco cheio do Linux) sai com 1 na primeira escrita que chega ao disco, com `WAV /dev/full` e `No space left on device` no stderr (Surface, exit 1)
Proof: `cargo test -p fala-cli --test meeting full_disk_exits_1 -- --ignored`

**C16** - O `tick` do gravador reescreve o cabeçalho a cada 1 s de relógio: aos 0,5 s o cabeçalho diz 0 pares, aos 1 s e aos 1,5 s diz 36 000, aos 2 s diz 84 000 (AC 1, door 3; adicionado na rodada 2 da verificação)
Proof: `cargo test -p fala-audio --lib meeting::recorder::tests::tick_rewrites_the_header_every_second`

**C17** - A abertura do monitor roda com `PIPEWIRE_NODE=<name>` e `PIPEWIRE_ALSA` no ambiente e a trava `ENV_OPEN` presa, e ao fim as duas variáveis somem e a trava está solta (door 1, ADR-0013; adicionado na rodada 2 da verificação)
Proof: `cargo test -p fala-audio --lib meeting::system::tests::monitor_env_is_set_only_while_opening_under_the_lock`

**C18** - O `Mic::open` segura a mesma trava `ENV_OPEN` antes de abrir o mic (door 1, ADR-0013; adicionado na rodada 2 da verificação)
Proof: `grep -n "crate::meeting::ENV_OPEN" crates/audio/src/mic.rs`

### Doors

**C13** - `crates/audio` declara a door 1 da trilha A mais `hound = "3.5.1"` em `[dependencies]`, sem `[dev-dependencies]`, sem `sherpa` nem `tauri` (Landing 4)
Proof: `cargo test -p fala-audio --test manifest dependencies_match_door_1`

**C14** - `SystemAudio::open` tem a forma literal da door 1 e o `cfg(target_os)` não aparece em `crates/audio/src/meeting` (Landing 1, ADR-0007)
Proof: `cargo test -p fala-audio --test meeting_shape system_audio_has_door_1_shape`
Proof: `! grep -rn "cfg(target_os" crates/audio/src/meeting`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| `fala-cli meeting` exit codes (3) | 0 C10 · 1 C15 · 2 C11 | - |
| canal mudo (2) | sistema C3 · mic C4 | - |
| taxas de entrada (3) | 48 000 C5 · 16 000 C7 · 44 100 C7 | - |
| entradas inválidas (3) | `--system` C11 · `--mic` C11 · `--out` C11 | - |
| doors do Landing (4) | 1 sistema C14, C17, C18 · 2 relógio C3–C8 · 3 WAV C1, C2, C16 · 4 deps C13 | - |

- Claims naming an exit code or output shape: C10, C11, C12, C15 - cada um com proof que roda o binário
- C11 é `#[ignore]` porque enumerar dispositivos abre o PipeWire desta máquina; no runner Windows sem áudio ele não roda

## Swept

- validation: C11
- failure modes: C3, C4 (fonte muda não é erro), C11, C15
- idempotency: n/a - cada execução cria um WAV novo em `--out`; rodar de novo sobrescreve, como o `record`
- authorization: n/a - comando local, sem rede e sem usuário; o consentimento é do desktop (ADR-0005)
- concurrency: C5 (dois canais intercalados pelo relógio), C6 (burst)
- data lifecycle: C1 (o arquivo sobrevive ao processo morto), C8 (finalize)
- dependency failure: C3, C4 (stream que não entrega)
- state transitions: C8 (gravando → finalizado)
- observability: C12; o resumo de preenchimento e descarte por canal no fim

## Handoff

- Arquivos: `crates/audio/src/meeting/{mod,wav,recorder,system}.rs` (~25 KB novos) + `resample.rs` (6 KB) + leitura de `apps/cli/src/record/capture.rs` (25 KB) ≈ 14k; `apps/cli/src/meeting.rs` + `tests/meeting.rs` + `main.rs` (~15 KB) ≈ 4k. Total ≈ 18k tokens, sob o budget de 150k - um builder, sem pergunta.
- Mechanism: one builder.
- Decidido pelo painel (Confirmed? y — delegado pelo Augusto em 2026-10-02): `fala-cli meeting` ao lado do `record`; a regra do relógio é provada com fonte e relógio falsos, porque o loopback mudo do WASAPI não existe no Linux.
