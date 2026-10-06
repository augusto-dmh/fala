# pipeline-headless verification

**Verdict**: PASS
**Profile**: light
**Diff range**: feat/core-contract (`008102f`)..HEAD (`932548c`) - commits `c95057c`, `7b91ce1`, `7e71e0e`, `932548c`
**Round**: 2 - scoped
**Verifier**: independent sub-agent (author != verifier)

Os 28 checks têm prova rodada em `932548c` e assert localizado (28/28 PASS). Os dois membros de
Coverage que derrubaram a rodada 1 (exit `1` e `--language` desconhecido -> `2`) agora têm
check próprio (C27, C28) com asserção sobre o valor que o check define.

**Rodada 1 (resumo, `b6ef00f`):** FAIL. Os 26 checks de então passaram com assert localizado,
mas dois membros nomeados pelo Surface do plano não tinham asserção: o exit `1` (atribuído ao C16,
cujo assert parava no `Err` do `Texts::join`, sem fixar o código) e o exit `2` por `--language`
desconhecido (sem AC, sem check e sem linha de Coverage). Também ficaram registrados gaps de
precisão/nível que não derrubavam check (C22 sem lista de dispositivos, C18 contando contra o
contador do próprio programa, C21 com proxy do log `microfone:`, C16/C20 provados na camada
unit, C15 com `paced` aproximado, C12 provando só o rótulo do idioma).

**Escopo desta rodada.** A correção reescreveu os commits (fixup/autosquash); `git diff b6ef00f
932548c` toca só `apps/cli/src/dictate.rs` (+37, teste novo `inference_error_exits_1` e dois
stubs no módulo `tests`), `apps/cli/tests/dictate.rs` (+11, teste novo
`unknown_language_exits_2`), `plan.md` (AC 24, linha de exit codes do Observable) e `checks.md`
(C27, C28, Coverage e Swept). Nenhum código de produção mudou. Verificados de novo, por inteiro:
C16, C20, C21, C22, C27, C28, a Coverage de exit codes e de entradas inválidas, e o `existing`
do Observable (que cita `dictate.rs`). Os demais checks são `carried from b6ef00f` quanto ao
julgamento, com as provas re-rodadas em `932548c` e as citações dos arquivos tocados
atualizadas (as linhas de `tests/dictate.rs` depois de `:173` andaram +11; as do módulo
`tests` de `dictate.rs`, +37).

## Binding sources

Carried from `b6ef00f`: a correção não toca as ADRs nem a interface das doors. A prova do C25
(ADR-0009 `status: proposed`, ADR-0003 intocada em `feat/core-contract..HEAD`) foi re-rodada em
`932548c` com exit 0.

| Source | Opened | Contradiction | Uncovered |
| --- | --- | --- | --- |
| `docs/decisions/0009-parakeet-via-transcribe-rs-com-silero-v4.md` | yes - lido na rodada 1 (`b6ef00f`); carried from `b6ef00f`, arquivo fora do diff da correção; linha 2 reconferida em `932548c` pelo C25 | none (carried from `b6ef00f`) | - |
| `docs/decisions/0003-asr-do-ditado-local-parakeet-via-sherpa-onnx.md` | yes - lido na rodada 1; carried from `b6ef00f`; intocado em `932548c` (C25, `git diff --quiet` exit 0) | none (carried from `b6ef00f`) | - |

## Checks

Invocações (todas em `932548c`, `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`, uma de cada vez, sempre `-p`):
A = `cargo test -p fala-audio -- --include-ignored` exit 0 (15 passed: lib 12, manifest 1, silero 2, doc 0);
B = `cargo test -p fala-asr -- --include-ignored` exit 0 (5 passed: lib 1, manifest 1, parakeet 2, trait_shape 1);
C = `cargo test -p fala-cli --bin fala-cli dictate::tests` exit 0 (3 passed, 14 filtered: `inference_error_exits_1`, `texts_join_in_utterance_order`, `stdin_lines_toggle_and_eof_ends`);
D = `cargo build -p fala-cli` + cópia para `scratchpad/verify2/fala-cli-verify` (sha256 `bba50d14…` igual ao de `target/debug/fala-cli`; `dictate --help` mostra `--model` e `--language`, e o default do `--vad` aponta para este worktree) + `FALA_CLI_BIN=<cópia> cargo test -p fala-cli --test dictate -- --include-ignored --test-threads=1` exit 0 (9 passed, 110,45 s);
E = `scripts/check-no-tauri-in-crates.sh` exit 0 ("ok: no tauri in crates/");
F = comando do C25 exit 0.
`FALA_TEST_PARAKEET_DIR=~/.local/share/com.pais.handy/models/parakeet-tdt-0.6b-v3-int8`, `FALA_TEST_SPEECH_WAV=scratchpad/speech-pause.wav`. Cada teste nomeado aparece individualmente como `ok` na saída e existe na árvore (`grep -n "fn <nome>"`; os dois novos em `apps/cli/src/dictate.rs:465` e `apps/cli/tests/dictate.rs:177`, ambos introduzidos pelo diff da correção).

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | 48 000 e 44 100 Hz → `in_len×16000/in_rate` ± 480 | A: `resample::tests::output_length_matches_ratio ... ok` | carried from `b6ef00f` (arquivo fora do diff): `crates/audio/src/resample.rs:143` `for rate in [48_000u32, 44_100]`; `:149` `assert!(diff <= 480, ...)` | PASS |
| C2 | seno 440 Hz sai com 440 ± 1 % | A: `resample::tests::sine_keeps_its_frequency ... ok` | carried from `b6ef00f`: `crates/audio/src/resample.rs:170` `assert!((freq - 440.0).abs() <= 4.4, ...)` | PASS |
| C3 | `start` abre com as últimas 4 800, em ordem | A: `capture::tests::prebuffer_opens_session_with_last_300_ms ... ok` | carried from `b6ef00f`: `crates/audio/src/capture.rs:213` `assert_eq!(concat(&out), ramp(5_200..12_000))` | PASS |
| C4 | < 4 800 antes: abre com todas | A: `capture::tests::short_prebuffer_keeps_everything ... ok` | carried from `b6ef00f`: `crates/audio/src/capture.rs:223` `assert_eq!(concat(&out), ramp(0..3_000))` | PASS |
| C5 | 2 quadros abrem com 15 de pre-roll, limitado ao início | A: `onset_includes_450_ms_pre_roll`, `pre_roll_stops_at_session_start`, `single_voiced_frame_opens_nothing` ... ok | carried from `b6ef00f`: `crates/audio/src/capture.rs:237` `assert_eq!(first[0], 15.0, ...)`, `:238`; `:251-252`; `:264` `assert!(out.is_empty())` | PASS |
| C6 | 15 quadros de silêncio fecham antes do `stop` | A: `capture::tests::silence_of_450_ms_closes_utterance ... ok` | carried from `b6ef00f`: `crates/audio/src/capture.rs:278` `assert_eq!(closed.len(), 1, ...)`; `:284` `assert!(c.is_recording())` | PASS |
| C7 | cortes de 240 000, concatenação = entrada | A: `capture::tests::continuous_speech_splits_at_15_s_without_loss ... ok` | carried from `b6ef00f`: `crates/audio/src/capture.rs:299` `assert_eq!(lens, vec![240_000, 240_000, 160_000])`; `:300` `assert_eq!(concat(&out), input)` | PASS |
| C8 | `stop` entrega tudo, incluindo o resampler | A: `capture::tests::stop_flushes_open_utterance_and_resampler ... ok` | carried from `b6ef00f`: `crates/audio/src/capture.rs:315` `assert_eq!(concat(&out).len(), 32_100)` | PASS |
| C9 | sem voz: `stop` não entrega nada | A: `capture::tests::no_voice_no_utterance ... ok` | carried from `b6ef00f`: `crates/audio/src/capture.rs:326` `assert!(c.stop().unwrap().is_empty())` | PASS |
| C10 | Silero v4: 0 utterances em 3 s de zero | A: `silence_gives_no_utterance ... ok` | carried from `b6ef00f`: `crates/audio/tests/silero.rs:28` `assert_eq!(utterances(16_000, &vec![0.0; 48_000]), 0)` | PASS |
| C11 | Silero v4: ≥ 1 utterance na fala real | A: `speech_gives_an_utterance ... ok` (ignored, rodado) | carried from `b6ef00f`: `crates/audio/tests/silero.rs:41` `assert!(utterances(rate, &samples) >= 1)` | PASS |
| C12 | texto não vazio e idioma pedido (`PtBr`, `En`) | B: `transcribes_real_speech ... ok` (ignored, rodado) | carried from `b6ef00f`: `crates/asr/tests/parakeet.rs:46` `for language in [Language::PtBr, Language::En]`; `:48` `assert!(!t.text.trim().is_empty(), ...)`; `:49` `assert_eq!(t.language, language)` | PASS |
| C13 | inexistente e vazia → `ModelLoad` com o caminho | B: `missing_model_names_the_path ... ok` | carried from `b6ef00f`: `crates/asr/tests/parakeet.rs:25` `assert!(matches!(err, AsrError::ModelLoad { .. }))`; `:27-28` `assert!(text.contains(&dir.display().to_string()), ...)` | PASS |
| C14 | 1 599: sem inferência e texto vazio; 1 600: 1 chamada | B: `tests::short_audio_skips_inference ... ok` | carried from `b6ef00f`: `crates/asr/src/lib.rs:103` `assert_eq!(calls, 0)`; `:104` `assert_eq!(t.text, "")`; `:114` `assert_eq!(calls, 1)` | PASS |
| C15 | `--wav` 3 s de zero: ≥ 2,9 s, uma linha vazia, exit 0 | D: `wav_is_paced_and_silence_prints_empty_line ... ok` | carried from `b6ef00f`, citações atualizadas: `apps/cli/tests/dictate.rs:209` `assert_eq!(out.status.code(), Some(0), ...)`; `:210` `assert_eq!(stdout(&out), "\n")`; `:217-218` `assert!(paced >= Duration::from_millis(2_900), ...)` | PASS |
| C16 | uma linha, aparados, em ordem, um espaço, mesmo fora de ordem | C: `dictate::tests::texts_join_in_utterance_order ... ok`; D: `speech_prints_one_line ... ok` | verified at `932548c`: `apps/cli/src/dictate.rs:488` `assert_eq!(texts.join().unwrap(), "Primeira segunda, terceira.")` (inseridos fora de ordem); `:490` vazio → `""`; `apps/cli/tests/dictate.rs:228` `assert_eq!(text.lines().count(), 1, ...)`, `:231` `assert_eq!(line, line.trim(), ...)`, `:232` `assert!(!line.contains("  "), ...)`. A asserção `:495` (`Err(AsrError::Inference(_))`) deixou de carregar o exit 1, que agora é do C27 | PASS |
| C17 | toda `utterance` menos a última com `before_release=1` | D: `utterances_transcribe_before_release ... ok` | carried from `b6ef00f`, citações atualizadas: `apps/cli/tests/dictate.rs:243-244` `assert!(utterances.len() >= 2, ...)`; `:249` `assert_eq!(value(line, "before_release"), 1, ...)` sobre `utterances[..len-1]` | PASS |
| C18 | chaves exatas da door 3, inteiros, sem palavra do texto | D: `trace_lines_follow_door_3 ... ok` | carried from `b6ef00f`, citações atualizadas: `apps/cli/tests/dictate.rs:260` `assert_eq!(dictations.len(), 1)`; `:262` `assert_eq!(value(dictations[0], "utterances"), utterances.len() as u64)`; `:271` `assert_eq!(keys, expected)` contra `UTTERANCE_KEYS`/`DICTATION_KEYS` (`:18-34`); `:273` `assert!(v.parse::<u64>().is_ok(), ...)`; `:295-296` `assert!(!tokens.contains(&word.as_str()), ...)` | PASS |
| C19 | sem `FALA_TRACE`, nenhuma `event=` | D: `no_trace_without_env ... ok` | carried from `b6ef00f`, citação atualizada: `apps/cli/tests/dictate.rs:307` `assert!(!stderr(&out).contains("event="), ...)` (env removido em `:63`) | PASS |
| C20 | linha começa, seguinte termina, EOF termina o aberto e encerra | C: `dictate::tests::stdin_lines_toggle_and_eof_ends ... ok` | verified at `932548c`: `apps/cli/src/dictate.rs:503-511` `assert_eq!(events, [Line, Line, Line, Eof])`; `:519-527` `assert_eq!(actions, [Start, Stop, Start, StopAndExit])`; `:528` `assert_eq!(next_action(false, StdinEvent::Eof), Action::Exit)` | PASS |
| C21 | `--model` sem Parakeet, `--vad` e `--wav` ilegíveis → 2, caminho no stderr, sem mic | D: `bad_paths_exit_2 ... ok` | verified at `932548c` (arquivo tocado, trecho inalterado): `apps/cli/tests/dictate.rs:137-142` (4 casos); `:145` `assert_eq!(out.status.code(), Some(2), ...)`; `:146` `assert!(stderr(&out).contains(path), ...)`; `:147-150` `assert!(!stderr(&out).contains("microfone:"), ...)`; `:151` `assert_eq!(stdout(&out), "")` | PASS |
| C22 | `--mic` sem correspondência → 2 e "dispositivos de entrada" | D: `unknown_mic_exits_2 ... ok` | verified at `932548c` (trecho inalterado): `apps/cli/tests/dictate.rs:168` `assert_eq!(out.status.code(), Some(2), ...)`; `:169-173` `assert!(stderr(&out).contains("dispositivos de entrada"), ...)` | PASS |
| C23 | deps exatas das doors 1 e 2, sem sherpa, sem tauri | A: `dependencies_match_door_1 ... ok`; B: `dependencies_match_door_2 ... ok`; E exit 0 | carried from `b6ef00f`: `crates/audio/tests/manifest.rs:27-38` `assert_eq!(names, ["cpal","fala-core","log","rtrb","rubato","thiserror","vad-rs"])`, `:50` sem sherpa; `crates/asr/tests/manifest.rs:27` `assert_eq!(names, ["fala-core", "thiserror", "transcribe-rs"])`, `:35` sem sherpa; `scripts/check-no-tauri-in-crates.sh` exit 0 em `932548c` | PASS |
| C24 | `Transcriber` com a forma literal da door 2, object-safe | B: `stub_implements_transcriber ... ok` | carried from `b6ef00f`: `crates/asr/src/lib.rs:20-26` (assinatura literal da door 2); `crates/asr/tests/trait_shape.rs:27` `assert_send::<Box<dyn Transcriber>>()`; `:32` `assert_eq!(t.text, "3")` | PASS |
| C25 | ADR-0009 `status: proposed`; ADR-0003 intocada | F exit 0 em `932548c` | `docs/decisions/0009-parakeet-via-transcribe-rs-com-silero-v4.md:2` `status: proposed`; `git diff --quiet feat/core-contract..HEAD -- docs/decisions/0003-...` exit 0 | PASS |
| C26 | exatamente uma `event=load` com `model_ms`/`vad_ms` inteiros, antes da primeira `utterance` | D: `load_is_traced_apart ... ok` | carried from `b6ef00f`, citações atualizadas: `apps/cli/tests/dictate.rs:318` `assert_eq!(loads.len(), 1)`; `:321` `assert_eq!(keys, LOAD_KEYS)` (`:35`); `:322-323` `value(...)` faz `parse::<u64>().unwrap()`; `:325` `assert!(loads[0] < first_utterance)` | PASS |
| C27 | erro de inferência no meio do ditado: soltar falha com código 1 e a mensagem do erro | C: `dictate::tests::inference_error_exits_1 ... ok` | verified at `932548c` (novo): `apps/cli/src/dictate.rs:476` `session.release(&mut capture).expect_err("devia falhar")`; `:477` `assert_eq!(failure.code, 1)`; `:478` `assert!(format!("{:#}", failure.error).contains("stub"))`. O stub (`:448-452`) devolve `AsrError::Inference("stub")`; o único outro caminho de código 1 no `release`, `AsrWorker::recv` (`:389-393`), tem outra mensagem ("a thread de ASR terminou…"), então `"stub"` fixa a origem. A mutação da rodada 1 (`failed` → `input` em `dictate.rs:287`) agora quebra `:477` | PASS |
| C28 | `--language xx` → 2, `--language` e o valor no stderr, nada no stdout, sem modelo | D: `unknown_language_exits_2 ... ok` | verified at `932548c` (novo): `apps/cli/tests/dictate.rs:181` `assert_eq!(out.status.code(), Some(2), ...)`; `:182` `assert!(stderr(&out).contains("--language"), ...)`; `:183` `assert!(stderr(&out).contains("xx"), ...)`; `:184` `assert_eq!(stdout(&out), "")`. O `--model` aponta para uma pasta inexistente (`:179`); se o parse viesse depois da carga, o stderr teria o erro do modelo, sem `--language`, e `:182` falharia. Código: `apps/cli/src/dictate.rs:70-73` antes de `SileroVad::load` (`:80`) e `Parakeet::load` (`:88`) | PASS |

### Gaps de nível e de precisão (registrados, não derrubam check)

- **AC 24 / C28, precisão (novo nesta rodada):** a AC 24 diz "IF `--language` não é `pt-BR` nem `en` THEN ... sair com 2", mas o parse é o `FromStr` do S0 (`crates/core/src/language.rs:36-41`, fora deste diff), que aceita também `pt` e qualquer caixa. Sondado em `932548c` com a cópia do binário e `--model /nonexistent-model-dir`: `--language xx` → exit 2 com `--language: idioma desconhecido: "xx"`; `--language pt` e `--language EN` passam do parse e caem no erro do modelo. O C28 prova o que afirma (`xx`); a letra da AC é mais estreita que o conjunto aceito. Sugestão: ajustar a AC 24 para "não é um idioma que `Language` aceita (`pt-BR`, `pt`, `en`, sem caixa)".
- **AC 24, precisão:** "antes de carregar qualquer modelo" inclui o VAD; o teste só discrimina a ordem contra o Parakeet (o VAD padrão carrega sem erro, então não deixa rastro). A ordem contra o VAD está só no código (`dictate.rs:70-73` antes de `:80`).
- **C27, nível:** o código 1 é fixado no `Failure` devolvido por `Session::release`; a passagem `Failure.code` → `ExitCode` (`apps/cli/src/main.rs:49-50`) é a mesma que C21/C22/C28 exercitam no binário para o código 2. Declarado no `checks.md` (forçar falha de inferência no binário exigiria modelo corrompido). A falha de stream do mic, o outro membro do Surface para exit 1 (`audio_failure`, `dictate.rs:117-122`), não tem prova; o set de exit codes do `checks.md` é por código, não por causa, e o código 1 está provado.
- Carried from `b6ef00f`, sem mudança: C22 não confere nomes de dispositivo (`apps/cli/tests/dictate.rs:169-173`); C18 conta contra o contador do próprio programa (`:262`); C21 usa o log `microfone:` como proxy (`:147-150`); C16/C20 provados na camada unit, laço `run_mic` (`dictate.rs:148-190`) sem prova; C15 `paced` inclui partida do processo; C12 prova só o rótulo do idioma.

### `existing` relido (verified at `932548c`)

- `Swept` não tem linha `existing`.
- `Observable` "o que imprime quando falha no meio" = existing: confirmado. `apps/cli/src/main.rs:49-50` `log::error!("{:#}", failure.error); ExitCode::from(failure.code)`; em `dictate.rs:287` `texts.join()` falha antes do `writeln!` (`:289`), então nada sai no stdout. O C27 agora prova o código 1 e a mensagem nesse caminho.

## Coverage

Profile `light`: a tabela do `checks.md` foi lida; as duas linhas que a correção tocou (exit codes, entradas inválidas) foram conferidas membro a membro contra o Surface do plano e o código. As demais: carried from `b6ef00f`.

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| `fala-cli dictate` exit codes (3) | Surface do plano (`0`, `1`, `2`) e `Failure`/`input`/`failed` em `apps/cli/src/dictate.rs:48-64`; verified at `932548c` | 0 C15 (`tests/dictate.rs:209`) · 1 C27 (`src/dictate.rs:477`) · 2 C21 (`:145`), C22 (`:168`), C28 (`:181`) | - |
| entradas inválidas de `dictate` (5) | Surface do plano ("modelo ou VAD ausente/ilegível, WAV ilegível, `--mic` sem correspondência, idioma desconhecido") e os `input(...)` de `run` (`dictate.rs:73`, `:75`, `:80`, `:85`, `:88`, `:119`); verified at `932548c` | `--model` C21 (`:137-138`) · `--vad` C21 (`:139`) · `--wav` C21 (`:140`) · `--mic` C22 (`:168`) · `--language` C28 (`:181-183`) | - |
| taxas de entrada do resampler (2) | carried from `b6ef00f` | 48 000 C1 · 44 100 C1 | - |
| fronteiras do pré-buffer (2) | carried from `b6ef00f` | ≥ 4 800 C3 · < 4 800 C4 | - |
| transições da utterance (4) | carried from `b6ef00f` | onset C5 · silêncio C6 · 15 s C7 · `stop` C8 | - |
| fronteira de 100 ms (2) | carried from `b6ef00f` | 1 599 C14 · 1 600 C14 | - |
| idiomas (2) | carried from `b6ef00f` | `PtBr` C12 · `En` C12 | - |
| eventos de trace (3) | carried from `b6ef00f` | `utterance` C18 · `dictation` C18 · `load` C26 | - |
| doors do Landing (4) | carried from `b6ef00f` | 1 C23 · 2 C23, C24 · 3 C18 · 4 C26 | - |
| entradas do stdin no modo mic (3) | verified at `932548c` (C20 re-citado) | linha começa C20 · linha termina C20 · EOF C20 | - |

## Gaps ranqueados

Nenhum gap que derrube a feature. Registrados, por ordem:
1. (precisão) AC 24 mais estreita que o `FromStr` do S0: `pt` e caixa diferente são aceitos - `crates/core/src/language.rs:36-41`; ajustar a letra da AC.
2. (nível) C27 prova o código 1 na camada do `Session`, não no binário; falha de stream do mic sem prova - `apps/cli/src/dictate.rs:477`, `:117-122`.
3. (precisão, carried) C22 não confere a lista de dispositivos que a AC 22 pede - `apps/cli/tests/dictate.rs:169-173`.

## Gate

- A `cargo test -p fala-audio -- --include-ignored` - 15 passed, 0 failed
- B `cargo test -p fala-asr -- --include-ignored` - 5 passed, 0 failed
- C `cargo test -p fala-cli --bin fala-cli dictate::tests` - 3 passed, 0 failed
- D `FALA_CLI_BIN=<cópia> cargo test -p fala-cli --test dictate -- --include-ignored --test-threads=1` - 9 passed, 0 failed
- E `scripts/check-no-tauri-in-crates.sh` - exit 0
- F comando do C25 - exit 0
- Total: 32 passed, 0 failed; 28/28 checks com assert localizado; 0 membros de Coverage sem prova -> PASS
