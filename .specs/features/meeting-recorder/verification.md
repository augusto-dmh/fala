# meeting-recorder verification

**Verdict**: PASS
**Profile**: light
**Diff range**: `0d31233`..HEAD (`91875a6`) - commits `7f5b5de`, `d0e1eae`, `91875a6`
**Round**: 2 - scoped
**Verifier**: independent sub-agent (author != verifier)

Os 18 checks têm prova verde em `91875a6`, e cada um tem um assert localizado. Os 6 gaps da
rodada 1 foram fechados. O C10 passou com o relógio contado a partir da linha do indicador. O
C12 agora afirma a ordem entre o indicador e o resumo. O C15 afirma o texto do erro. O flush
periódico do `tick` ganhou o C16. A door 1 ganhou o C17 (variáveis de ambiente e trava) e o C18
(o `Mic::open` segura a mesma trava). O WAV do teste de gravação passou a ser apagado por um
guard de `Drop`. Ficam observações que não bloqueiam (ver o fim).

**Resumo da rodada 1 (`f60c93d`, FAIL).** 13 dos 15 checks tinham passado. O C10 falhou: o WAV
saiu com 2,442 s quando o check pedia 3 ± 0,5 s, porque o teste contava os 3 s desde o spawn e o
relógio do gravador só começa depois do `pw-dump` e da abertura dos streams. O C12 caiu junto,
porque dividia a mesma prova. A rodada 1 ranqueou 6 gaps: (1) C10; (2) C12, que afirmava "no
começo" só com `contains` e provava o progresso só no `Progress::due`; (3) o flush periódico do
cabeçalho no `tick` não tinha prova; (4) as variáveis de ambiente e a trava `ENV_OPEN` da door 1
não tinham prova; (5) C15 não afirmava o momento da falha nem o texto do erro; (6) o teste
deixava o áudio do mic no disco quando falhava.

**Escopo desta rodada**, conforme o `verify.md`. Todas as provas rodaram de novo, por inteiro, em
`91875a6`. Revi os checks que o diff `f60c93d..91875a6` toca: C1, C10, C12, C14, C15, C16, C17 e
C18, mais a Coverage de doors e de exit codes e o guard de limpeza. Os checks C2–C9, C11 e C13
vêm `carried from f60c93d`: o diff não toca os testes nem o código deles. Mesmo assim, as provas
desses checks rodaram de novo e passaram, e as citações foram conferidas nos arquivos tocados. O
diff da correção toca `apps/cli/tests/meeting.rs`, `crates/audio/src/meeting/recorder.rs`
(+20 linhas a partir de `:338`), `crates/audio/src/meeting/system.rs`, `checks.md` e o AC 9 do
`plan.md`. Não toca `apps/cli/src/meeting.rs`, `wav.rs`, `mic.rs`, `resample.rs` nem os testes
de integração de `crates/audio`.

Todo `cargo` rodou com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`,
com `-p <crate>` e um de cada vez. O binário dos testes de fronteira é uma cópia de
`target/debug/fala-cli`, feita logo depois do `cargo build -p fala-cli`, em
`scratchpad/verify-f2/fala-cli-verify`. O `meeting --help` dela mostra `--out`, `--system` e
`--mic`. A cópia foi passada em `FALA_CLI_BIN`.

## Binding sources

Carried from `f60c93d`. O diff da correção não muda a interface nem as ADRs. Ele só move o
mecanismo da ADR-0013 para `with_monitor_env`, e o mecanismo continua o mesmo.

| Source | Opened | Contradiction | Uncovered |
| --- | --- | --- | --- |
| ADR-0013 `docs/decisions/0013-captura-linux-pelo-plugin-alsa-do-pipewire.md` (proposed, neste diff) | yes - lida na rodada 1; o "Obrigatório" foi reconferido em `91875a6` | none | - |
| ADR-0005 `docs/decisions/0005-reuniao-transcrita-na-nuvem-com-audio-retido-localmente.md` | yes - rodada 1 | none | - |
| ADR-0007 `docs/decisions/0007-windows-primeiro-linux-gnome-depois-sem-macos.md` | yes - rodada 1 | none | - |

- ADR-0013, verified at `91875a6`. O "Obrigatório" está no código e agora tem prova.
  `crates/audio/src/meeting/system.rs:18` declara `static ENV_OPEN`. O `open` chama
  `with_monitor_env` em `:65`. A trava é presa em `:121`, as variáveis entram com `set_var` em
  `:129-130` e saem com `remove_var` em `:136-137`, antes de soltar a trava. O
  `crates/audio/src/mic.rs:35` segura a mesma trava. C17 e C18 provam isso. Dos três itens da
  "Confirmação", o primeiro (nada de `pipewire`/`libspa`) é o C13, verde. O terceiro (a gravação
  roda pelo teste `#[ignore]` com `FALA_TEST_SINK`) é o C10, que agora está verde.
- ADR-0005 e ADR-0007: carried from `f60c93d`. O `grep -rn "cfg(" crates/audio/src/meeting` em
  `91875a6` só acha `cfg(test)`: `wav.rs:67`, `system.rs:208` (módulo de teste novo) e
  `recorder.rs:167`.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | leitor sem `finalize` (escritor esquecido) lê exatamente os pares até o último flush; flush de 1 s | verified at `91875a6`: `cargo test -p fala-audio` exit 0, `meeting::wav::tests::killed_writer_leaves_flushed_frames ... ok` | `crates/audio/src/meeting/wav.rs:99` - `assert_eq!(samples.len(), 2 * 4_800)`; `:101` - `assert_eq!(samples[2 * 4_799..], [4_799, -4_799])`; `:86` - `assert!(FLUSH_EVERY <= Duration::from_secs(1))`. O gap de nível da rodada 1 (o flush do `tick` sem prova) foi fechado pelo C16 | PASS |
| C2 | WAV finalizado com 48 000 Hz, 2 canais, 16 bits, L = mic, R = sistema | carried from `f60c93d`; rodou de novo: `meeting::wav::tests::finished_wav_is_48k_stereo_mic_left ... ok` | `crates/audio/src/meeting/wav.rs:111` - `assert_eq!(spec.sample_rate, 48_000)`; `:112` 2 canais; `:113` 16 bits; `:115` - `assert_eq!(samples, [100, -1, 101, -2, 102, -3])` | PASS |
| C3 | sistema mudo, 10 s de relógio falso: R só com zeros, L = mic, `relógio − 250 ms` de pares ± 1 tick | carried from `f60c93d`; rodou de novo: `meeting::recorder::tests::silent_system_is_filled_by_the_clock ... ok` | `crates/audio/src/meeting/recorder.rs:249` - `assert_eq!(before, 480_000 - LAG_FRAMES)`; `:253` - `assert!(right.iter().all(...r == 0))` (todo R é zero); `:254` - `assert_eq!(left, as_i16(&run.mic...))` (linhas antes de `:338`, que o diff não move) | PASS |
| C4 | mic mudo: L com zeros, R = sistema | carried from `f60c93d`; rodou de novo: `meeting::recorder::tests::silent_mic_is_filled_by_the_clock ... ok` | `crates/audio/src/meeting/recorder.rs:270` - `assert!(left.iter().all(...l == 0))` (todo L é zero); `:271` - `assert_eq!(right, as_i16(&run.system...))`; `:272` - `assert_eq!(mic.filled, 144_000)` | PASS |
| C5 | dois canais a 48 kHz no ritmo: L e R exatos, em ordem, preenchimento 0 | carried from `f60c93d`; rodou de novo: `meeting::recorder::tests::realtime_sources_are_written_verbatim ... ok` | `crates/audio/src/meeting/recorder.rs:286-287` - `assert_eq!(left, ...)`, `assert_eq!(right, ...)`; `:289` - `assert_eq!((mic.filled, system.filled), (0, 0))` | PASS |
| C6 | 2 s de uma vez com o relógio em 0,5 s: descarta o excesso acima de 500 ms, do mais antigo, e conta | carried from `f60c93d`; rodou de novo: `meeting::recorder::tests::backlog_over_500_ms_is_dropped_and_counted ... ok` | `crates/audio/src/meeting/recorder.rs:306` - `assert_eq!(mic.dropped, 96_000 - MAX_BACKLOG_FRAMES as u64)`; `:312` - `assert_eq!(written_burst[0], to_i16(burst[72_000]))` | PASS |
| C7 | mic a 16 000 e sistema a 44 100 saem a 48 000; pares seguem o relógio ± 480 | carried from `f60c93d`; rodou de novo: `meeting::recorder::tests::any_input_rate_is_resampled_to_48k ... ok` | `crates/audio/src/meeting/recorder.rs:324` - `assert!((before as i64 - clock).abs() <= 480, ...)`; `:327` - `assert_eq!(left.len(), 240_000)` | PASS |
| C8 | `finish` escreve até o relógio, sem a folga de 250 ms, e finaliza | carried from `f60c93d`, com a citação atualizada (o arquivo foi tocado); rodou de novo: `meeting::recorder::tests::finish_writes_up_to_the_clock ... ok` | `crates/audio/src/meeting/recorder.rs:366` - `assert_eq!(before, 96_000 - LAG_FRAMES)`; `:369` - `assert_eq!(reader.duration(), 96_000)` (eram `:346`/`:349`) | PASS |
| C9 | `Resampler` com saída arbitrária; 16 000 → 48 000 dá `in_len × 3` ± 480 | carried from `f60c93d`; rodou de novo: `resample::tests::upsamples_to_48k ... ok` | `crates/audio/src/resample.rs:192` - `assert!(diff <= 480, ...)` | PASS |
| C10 | stdin fechado 3 s depois da linha do indicador: exit 0 e WAV de 48 kHz estéreo com 3 s ± 0,5 s | verified at `91875a6`: `FALA_CLI_BIN=<cópia> FALA_TEST_SINK=alsa_output...HDMI1__sink cargo test -p fala-cli --test meeting -- --include-ignored --test-threads=1` exit 0, `records_until_stdin_closes ... ok` | `apps/cli/tests/meeting.rs:113` - `assert!((seconds - 3.0).abs() <= 0.5, "{seconds} s")`; `:103` - `assert_eq!(out.code, Some(0), ...)`; `:110-111` - 48 000 Hz e 2 canais. O tempo é contado a partir do indicador: a thread que lê o stderr avisa ao ver `INDICATOR` (`:73-74`), e o teste espera esse aviso antes de dormir os 3 s (`:81-82`). No binário, o relógio começa em `apps/cli/src/meeting.rs:70` (`Instant::now()`), logo antes do log do indicador em `:71`. O AC 9 do plano foi precisado do mesmo jeito | PASS |
| C11 | `--system` e `--mic` sem correspondência e `--out` em diretório inexistente: exit 2, nome ou caminho no stderr, sem criar o WAV | carried from `f60c93d`, com a citação atualizada; mesma invocação: `bad_inputs_exit_2 ... ok` | `apps/cli/tests/meeting.rs:145` - `assert_eq!(out.status.code(), Some(2), ...)`; `:146` - `assert!(stderr(&out).contains(needle), ...)`; `:147` - `assert!(!wav.exists() && !no_dir.exists(), ...)`, num laço pelos 3 casos (`:125-142`). O corpo do teste não mudou, só as linhas | PASS |
| C12 | stderr com a linha do indicador antes do resumo do fim; progresso a cada 60 s, provado no `Progress` | verified at `91875a6`: mesma invocação, `records_until_stdin_closes ... ok`; `cargo test -p fala-cli --bin fala-cli meeting::tests` exit 0, `meeting::tests::progress_every_60_s ... ok` | `apps/cli/tests/meeting.rs:105-107` - `let indicator = out.stderr.find(INDICATOR)...`, `let summary = out.stderr.find("reunião gravada")...`, `assert!(indicator < summary, ...)`; `apps/cli/src/meeting.rs:139` - `assert_eq!(p.due(Duration::from_secs(60)), Some(1))`; `:142` - `Some(2)` em 120 s; `:138`, `:140-141` - `None` entre os dois. O check agora diz que o log em `run` (`:107-108`) não roda em teste | PASS |
| C15 | `--out /dev/full`: exit 1 na primeira escrita que chega ao disco, com `WAV /dev/full` e `No space left on device` no stderr | verified at `91875a6`: mesma invocação, `full_disk_exits_1 ... ok` | `apps/cli/tests/meeting.rs:156` - `assert_eq!(out.code, Some(1), ...)`; `:157` - `assert!(out.stderr.contains("WAV /dev/full"), ...)`; `:158-162` - `assert!(out.stderr.contains("No space left on device"), ...)`. O exit 1 (e não 2) mostra que a falha veio depois do `MeetingWav::create`, que em `apps/cli/src/meeting.rs:53-56` sairia com 2. O momento exato não é afirmado (ver Observações) | PASS |
| C16 | o `tick` reescreve o cabeçalho a cada 1 s: 0 pares aos 0,5 s, 36 000 aos 1 s e aos 1,5 s, 84 000 aos 2 s | verified at `91875a6`: `cargo test -p fala-audio` exit 0, `meeting::recorder::tests::tick_rewrites_the_header_every_second ... ok` | `crates/audio/src/meeting/recorder.rs:345` - `assert_eq!(header(), 0, "0,5 s: ainda sem flush")`; `:347` - `assert_eq!(header(), 48_000 - LAG_FRAMES as u32)` (36 000); `:349-353` - o mesmo valor aos 1,5 s; `:355` - `assert_eq!(header(), 96_000 - LAG_FRAMES as u32)` (84 000). Um `tick` que nunca chamasse `flush` (`:128-131`) falharia em `:347` | PASS |
| C17 | abertura do monitor com `PIPEWIRE_NODE=<name>`, `PIPEWIRE_ALSA` e a trava presa; ao fim, sem as variáveis e com a trava solta | verified at `91875a6`: `cargo test -p fala-audio` exit 0, `meeting::system::tests::monitor_env_is_set_only_while_opening_under_the_lock ... ok` | `crates/audio/src/meeting/system.rs:222` - `assert_eq!(seen.0.as_deref(), Some("sink-de-teste"))`; `:223` - `assert_eq!(seen.1.as_deref(), Some("{ stream.capture.sink = true }"))`; `:224` - `assert!(seen.2, ...)` (`try_lock().is_err()` dentro da abertura); `:225-226` - `var_os(...).is_none()`; `:227-230` - `assert!(ENV_OPEN.try_lock().is_ok(), ...)`. O `SystemAudio::open` passa por esse helper em `:65` (`with_monitor_env(name, ...)`) | PASS |
| C18 | o `Mic::open` segura a mesma trava `ENV_OPEN` antes de abrir o mic | verified at `91875a6`: `grep -n "crate::meeting::ENV_OPEN" crates/audio/src/mic.rs` exit 0 | `crates/audio/src/mic.rs:35` - `let _guard = crate::meeting::ENV_OPEN`. Pela leitura de `:33-44`, o guard é a primeira instrução do `open` e fica vivo até o fim da função, ou seja, segura a trava antes do `cpal::default_host()` e de abrir o dispositivo | PASS |
| C13 | `crates/audio` = door 1 da trilha A + `hound = "3.5.1"` em `[dependencies]`, sem `[dev-dependencies]`, sem `sherpa`/`tauri` | carried from `f60c93d`; rodou de novo: `tests/manifest.rs` `dependencies_match_door_1 ... ok` | `crates/audio/tests/manifest.rs:28-39` - `assert_eq!(names, ["cpal","fala-core","hound","log","rtrb","rubato","thiserror","vad-rs"])`; `:50` - `assert_eq!(spec("hound"), "\"3.5.1\"")`; `:51` - `assert!(!manifest.contains("[dev-dependencies]"))` | PASS |
| C14 | `SystemAudio::open` com a forma literal da door 1; nenhum `cfg(target_os)` em `crates/audio/src/meeting` | verified at `91875a6`: `tests/meeting_shape.rs` `system_audio_has_door_1_shape ... ok`; `! grep -rn "cfg(target_os" crates/audio/src/meeting` exit 0, nenhuma linha | `crates/audio/tests/meeting_shape.rs:9` - `let _open: fn(&str) -> Result<SystemAudio, AudioError> = SystemAudio::open;`; `:10-12` - `check`, `sample_rate`, `drain_into`. O gap de precisão da rodada 1 (variáveis e trava sem prova) foi fechado por C17 e C18. O novo `mod tests` de `system.rs` usa `cfg(test)` em `:208`, que não é `cfg(target_os)` | PASS |

## Coverage

Profile light: a tabela do `checks.md` foi lida, não recalculada. Revi as linhas que a correção
tocou (exit codes e doors) e conferi cada membro contra a prova desta rodada.

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| `fala-cli meeting` exit codes (3) | read from checks.md + Surface do plan; verified at `91875a6` | 0 C10 (verde agora) · 1 C15 · 2 C11 | - |
| canal mudo (2) | read from checks.md; carried from `f60c93d` | sistema C3 · mic C4 | - |
| taxas de entrada (3) | read from checks.md; carried from `f60c93d` | 48 000 C5 · 16 000 C7 · 44 100 C7 | - |
| entradas inválidas (3) | read from checks.md; carried from `f60c93d` | `--system` C11 · `--mic` C11 · `--out` C11 | - |
| doors do Landing (4) | read from checks.md + Landing do plan; verified at `91875a6` | 1 sistema C14 (forma), C17 (variáveis e trava no monitor), C18 (trava no mic) · 2 relógio C3–C8 · 3 WAV C1, C2, C16 (flush periódico do `tick`) · 4 deps C13 | - |

## Swept existing

Carried from `f60c93d`. `checks.md` não tem linha `Swept` marcada `existing`. A linha `existing`
fica no `Observable` do plano ("o `main.rs` loga o erro com `{:#}` e sai com o código da
falha"), e o código continua conferindo em `91875a6`: `apps/cli/src/main.rs:57`
(`Command::Meeting(args)`), `:61` (`log::error!("{:#}", failure.error)`) e `:62`
(`ExitCode::from(failure.code)`). O C15 exercita esse caminho: o stderr traz o erro com
contexto e o exit é 1.

## Guard de limpeza do teste (gap 6 da rodada 1)

Verified at `91875a6`. O `apps/cli/tests/meeting.rs:43-50` define `struct Cleanup(PathBuf)` com
um `Drop` que chama `fs::remove_file`. O `records_until_stdin_closes` o cria em `:100`, antes de
`sink()`, de `record_for` e de qualquer assert. O `WavReader` (`:108`) é declarado depois, então
é solto antes do `Cleanup`. Depois da única execução de `--test meeting`,
`/home/augusto/projects/fala/target/tmp/meeting/records/` ficou vazio (`ls -la`: só `.` e
`..`). Só o caminho de sucesso foi observado. O caminho de falha (o `Drop` rodando no unwind do
panic) é garantido por construção e não foi exercitado, porque o profile light não injeta falha.

## Gate

Todas as provas rodaram em `91875a6`:

- `cargo test -p fala-audio`: 26 passed, 0 failed, 1 ignored. Lib: 23 passed, os 10 da feature (8
  da rodada 1, mais `tick_rewrites_the_header_every_second` e
  `monitor_env_is_set_only_while_opening_under_the_lock`) e os 13 da `pipeline-headless`
  (regressão). `manifest`: 1. `meeting_shape`: 1. `silero`: 1 passed e 1 ignored
  (`speech_gives_an_utterance`, que precisa de `FALA_TEST_SPEECH_WAV`, da feature anterior).
- `cargo test -p fala-cli --bin fala-cli meeting::tests`: 1 passed, 0 failed, 17 filtered out.
- `FALA_CLI_BIN=<cópia> FALA_TEST_SINK=alsa_output.pci-0000_00_1f.3-platform-skl_hda_dsp_generic.HiFi__HDMI1__sink cargo test -p fala-cli --test meeting -- --include-ignored --test-threads=1`:
  3 passed, 0 failed (`bad_inputs_exit_2`, `full_disk_exits_1`, `records_until_stdin_closes`),
  5,76 s. Rodou uma única vez.
- `FALA_CLI_BIN=<cópia> cargo test -p fala-cli --test dictate -- bad_paths_exit_2 unknown_mic_exits_2 unknown_language_exits_2`:
  3 passed, 0 failed, 6 filtered out (regressão).
- `! grep -rn "cfg(target_os" crates/audio/src/meeting`: exit 0, nenhuma linha.
- `grep -n "crate::meeting::ENV_OPEN" crates/audio/src/mic.rs`: `35:        let _guard = crate::meeting::ENV_OPEN`, exit 0.
- `scripts/check-no-tauri-in-crates.sh`: `ok: no tauri in crates/`, exit 0.

Não houve injeção de falha (profile light). Nenhum arquivo do worktree foi tocado além deste
relatório. O `git status --porcelain` só mostra `?? .specs/features/meeting-recorder/verification.md`.

## Observações (não bloqueiam)

1. C15: a parte "na primeira escrita que chega ao disco" não é distinguida pelo teste. Com 2 s
   de gravação, uma falha no `tick` ou só no `finish` daria o mesmo exit 1 e o mesmo stderr. O
   que o teste prova é exit 1 (não 2, ou seja, depois do `create`), `WAV /dev/full` e
   `No space left on device`. Pela leitura do código, o erro se propaga na primeira escrita:
   `crates/audio/src/meeting/wav.rs:40` usa `?` em `write_sample`, e
   `apps/cli/src/meeting.rs:106` usa `recorder.tick(now).map_err(audio)?`. Um gravador que
   engolisse erros até o `finalize` passaria no C15.
2. C12: o log periódico em `run` (`apps/cli/src/meeting.rs:107-108`) só é provado pela regra
   `Progress::due` na camada unit. O check foi precisado para dizer isso, então não é gap, mas o
   `if let Some(minutes)` que chama o log não tem prova de fronteira.
3. C17: o teste exercita o helper `with_monitor_env`. Que o `SystemAudio::open` passa por ele vem
   da leitura de `system.rs:65`. O C10 exercita o caminho inteiro no host ALSA, mas não observa o
   ambiente.
4. C17: o teste chama `set_var` dentro do harness de testes, que roda várias threads. As funções
   de ambiente da `std` se serializam internamente, e nenhum teste da lib abre o `cpal`, que
   leria o ambiente por `getenv` da libc. O risco de flake é baixo.
