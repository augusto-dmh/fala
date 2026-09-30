# cli-bench verification

**Verdict**: PASS
**Profile**: light
**Diff range**: 773cea7..f2f2bb3f0420b24ed0e2a233d49c316b2469479b (this round: dc1021b..f2f2bb3; rounds brief: da03312..f2f2bb3)
**Round**: 3 - scoped
**Verifier**: independent sub-agent (author != verifier)

Verified at `f2f2bb3` on branch `docs/asr-rounds-linux`. `git status --porcelain` was empty
before and after the run. `~/projects/fala-research` is at `3041c60` and its porcelain was
empty too.

**Checks**: 34 of 34 proven with located evidence (C1-C34). C32 and C33 were deferred in
round 2 and are judged for the first time here. Every proof was re-run at `f2f2bb3`.

### Scope

The brief said that `da03312..f2f2bb3` touches only `docs/spikes/01-asr-pt-br.md`. That is
true, but round 2 verified `dc1021b`, not `da03312`. `git diff --stat dc1021b HEAD -- apps/cli`
also shows #11 (`da03312`, "resolve the bench language"):

- `apps/cli/src/bench/engine.rs` changed by 94 lines, `mod.rs` by 28 and `tests/bench.rs` by 104.
- In `tests/bench.rs`, one comment line was added at `:4-5` and new tests were appended after
  `:643`.
- `wer.rs` and `corpus.rs` did not change.

verify.md scopes a round by the diff, not by the brief, so this round covers the following:

- **Proofs:** every proof was re-run at `f2f2bb3`, for C1-C34.
- **Judged in full for the first time:** C32 and C33.
- **Re-judged, because the proof reads the changed report:** C27, C28, C29 and C30. Their
  citations were refreshed.
- **Re-judged, because #11 changed the code under them:** C10, C11, C12 and C13 (the gguf load
  path now returns `Failure`), C22 (the `--hyp` conflict list grew by one flag), C25 (the
  hypothesis log line moved) and the Swept "dependency failure" row.
- **Citations refreshed:** every citation into `tests/bench.rs` (+1 line), `engine.rs` and
  `mod.rs`. Citations into `wer.rs` are carried unchanged.
- **Carried from round 2 (`dc1021b`):** every other judgment. Those rows say so.

## Binding sources

Carried from round 2 (`dc1021b`). The profile is `light`, so step 1 does not run. The plan
marks none of its `Sources` as binding, and there is no screen.

## Proofs run

All at `f2f2bb3`, one target at a time, nothing in parallel.

| Invocation | Result |
| --- | --- |
| `cargo test -p fala-cli` | **Unit tests: 14 passed, 0 failed.** The cli-bench unit tests are listed `... ok`: `bench::wer::tests::{normalize_follows_door_3, levenshtein_counts_sub_del_ins, ola_mundo_is_zero, guarda_chuva_counts_one_insertion}` and `bench::engine::tests::chunks_are_consecutive_without_overlap`. The other 9 belong to `record` and to #11's `language_resolution_table`.<br>**`tests/bench.rs`: 16 passed, 0 failed, 10 ignored.** Each cli-bench test is listed `... ok`: `total_line_pools_wer_and_rtf`, `empty_reference_exits_2`, `parakeet_rejects_threads_and_device`, `gpu_without_backend_exits_1`, `missing_model_exits_1`, `cuts_run_in_byte_order_ignoring_other_files`, `no_wav_exits_2`, `missing_reference_exits_2`, `wrong_wav_spec_exits_2`, `hyp_scores_and_names_dir`, `missing_wall_s_prints_dash`, `hyp_rejects_engine_flags`, `hyp_missing_stem_exits_2`, `stdout_is_legend_then_table` and `transcript_text_never_printed_above_debug`. The 16th is #11's `language_rejected_outside_gguf`.<br>**`tests/record.rs`: 7 passed, 6 ignored.** That test file is not part of this feature. |
| `FALA_TEST_PARAKEET_DIR=… FALA_TEST_GGUF=…/ggml-large-v3-turbo.bin FALA_TEST_SPEECH_WAV=…/smoke.wav cargo test -p fala-cli --test bench -- --ignored --exact --test-threads=1 parakeet_onnx_prints_one_row_per_cut load_time_is_excluded_from_wall_s gguf_legend_reports_threads_and_cpu engine_error_keeps_printed_rows out_then_hyp_round_trips rows_stream_while_running` | **6 passed, 0 failed, 20 filtered out, 107.73 s.** Each test is listed `... ok`. |
| `FALA_TEST_GGUF=… FALA_TEST_SPEECH_WAV=… cargo test -p fala-cli --features vulkan --test bench -- --ignored --exact gguf_gpu_reports_device` | **1 passed**, 12.71 s. `test gguf_gpu_reports_device ... ok`. |
| C27, C28, C29 and C30 shell proofs, exactly as written in `checks.md`, from the repo root | exit 0 each. |
| C32 `awk '/^## Evidência medida/,0' … \| grep -E 'ADR-0003' \| grep -E '[0-9]+\.[0-9]+ ?%'` | **exit 0.** It matched `- **WER** (ADR-0003: ~6 %): o Parakeet v3 int8 mediu 8.94 % nos cortes de ditado, 17.79 % nas`. |
| C33 `awk '/^## Rodadas/,0' ~/projects/fala-research/benchmarks/README.md \| grep -cE '^\| 2026-' \| awk '$1>=1{ok=1} END{exit !ok}'` | **exit 0.** The count is 35. |
| C34 `cargo metadata … \| jq -e …` | `true`, exit 0. |
| C31 `uv run …/bench_faster_whisper.py --cuts $S/cuts --out <scratch>/fw_r3 && cargo run -q -p fala-cli -- bench --cuts $S/cuts --refs $S/refs --hyp <scratch>/fw_r3` | **exit 0.** The script wrote `smoke.txt` and `smoke.wall_s`. The bench legend read `engine=hyp model=fw_r3`, with `wall_s 13.84`.<br>The output went to a scratch directory, deleted afterwards, rather than `$S/fw`, so nothing outside the report was written. |
| Data check for C32/C33: for each of the 7 `hyp/<config>/` of the round, `target/debug/fala-cli bench --cuts $B/audio/cuts --refs $B/reference --hyp $B/hyp/<config>`, pooled per cut type from the printed `sub`, `del`, `ins`, `ref_words`, `audio_s` and `wall_s` | **exit 0 for all 7 configs.** Every per-cut WER and RTF, and every aggregate, equals the README tables and the report table (see C32 and C33). |
| `sha256sum ~/projects/fala-research/benchmarks/audio/cuts/*.wav` | All 5 hashes equal `benchmarks/README.md:61-65`. |

Every named test exists at the `fn` line cited below (found with `rg -n 'fn <name>'
apps/cli/tests/bench.rs`), and each name appeared in the runner output as `... ok`.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | door 3 normalization on the decomposed-`Á` sample | unit: `normalize_follows_door_3 ... ok` | `apps/cli/src/bench/wer.rs:97` - `assert_eq!(normalize("A\u{301}  B-c'd, 2!"), vec!["á", "b", "c", "d", "2"])`. The file is unchanged since `dc1021b`. | PASS (carried from dc1021b; proof re-run at f2f2bb3) |
| C2 | Levenshtein sub/del/ins; `wer_%` with 2 decimals | unit: `levenshtein_counts_sub_del_ins ... ok` | `apps/cli/src/bench/wer.rs:106` - `(1, 0, 0, 3)`; `:107` - `"33.33"`; `:110` - `(0, 1, 0, 3)`; `:113` - `(0, 0, 1, 2)` | PASS (carried from dc1021b; proof re-run at f2f2bb3) |
| C3 | `Olá, mundo!` vs `olá mundo` scores `0.00` | unit: `ola_mundo_is_zero ... ok` | `apps/cli/src/bench/wer.rs:120` - `assert_eq!(s.wer_pct(), "0.00")` | PASS (carried from dc1021b; proof re-run at f2f2bb3) |
| C4 | guarda-chuva: 0/0/1/4, `25.00` | unit: `guarda_chuva_counts_one_insertion ... ok` | `apps/cli/src/bench/wer.rs:129` - `(0, 0, 1, 4)`; `:130` - `"25.00"` | PASS (carried from dc1021b; proof re-run at f2f2bb3) |
| C5 | the `total` row pools `ref_words`, `wer_%` and `rtf` | `total_line_pools_wer_and_rtf ... ok` | `apps/cli/tests/bench.rs:186` - `assert_eq!(total[8], "4")`; `:187` - `assert_eq!(total[4], "25.00")`; `:188` - `assert_eq!(total[3], "0.875")` | PASS (carried from dc1021b; citation refreshed) |
| C6 | an empty reference exits 2 naming the file, before the model loads | `empty_reference_exits_2 ... ok` | `apps/cli/tests/bench.rs:200` - `assert_eq!(o.status.code(), Some(2), …)`; `:201` - `stderr(&o).contains("a.txt")` | PASS (carried from dc1021b; citation refreshed) |
| C7 | parakeet-onnx prints legend, header, separator, row and total; `rtf` has 3 decimals and is < 1 | ignored batch: `parakeet_onnx_prints_one_row_per_cut ... ok` | `apps/cli/tests/bench.rs:235` - `assert_eq!(out.lines().count(), 5)`; `:244` - `rtf.split('.').nth(1).map(str::len) == Some(3)`; `:245` - `rtf.parse::<f64>().unwrap() < 1.0` | PASS (carried from dc1021b; citation refreshed) |
| C8 | `load_s` appears only in the legend and is excluded from `wall_s`, on a 2 s cut | ignored batch: `load_time_is_excluded_from_wall_s ... ok` | `apps/cli/tests/bench.rs:264` - `assert!(!out.lines().nth(1).unwrap().contains("load"))`; `:267` - `assert!(wall_s < load_s, …)`. The 2 s prefix comes from the helper at `:67`. | PASS (carried from dc1021b; citation refreshed; 1 of 1 this round) |
| C9 | parakeet rejects `--threads` and `--device gpu` with exit 2 | `parakeet_rejects_threads_and_device ... ok` | `apps/cli/tests/bench.rs:216` - `Some(2)`, with `:217` - `contains("--threads")`; `:219` - `Some(2)`, with `:220` - `contains("--device gpu")` | PASS (carried from dc1021b; citation refreshed) |
| C10 | the gguf legend shows `threads=8 device=cpu`; the default is `available_parallelism` | ignored batch: `gguf_legend_reports_threads_and_cpu ... ok` | `apps/cli/tests/bench.rs:286` - `.contains(" threads=8 device=cpu ")`; `:296` - `assert_eq!(legend_field(&stdout(&o), "threads"), n)`. The load path changed in #11: `load_gguf` now takes a language and returns `Failure`. The proof is green at HEAD, and the legend shape is unchanged. | PASS (re-judged at f2f2bb3) |
| C11 | in a vulkan build, `--device gpu` exits 0 and `device=` is the `Model::device()` name, never `cpu` | `--features vulkan … gguf_gpu_reports_device ... ok` | `apps/cli/tests/bench.rs:309` - `Some(0)`; `:311` - `assert_ne!(device, "cpu")`; `:315` - the stderr line `dispositivo GPU: {device} ` must exist (it is emitted at `apps/cli/src/bench/engine.rs:83`); `:318` - `!line.contains("llvmpipe")` | PASS (re-judged at f2f2bb3) |
| C12 | with no GPU feature, `--device gpu` exits 1 naming vulkan and cuda, with no fallback | `gpu_without_backend_exits_1 ... ok` | `apps/cli/tests/bench.rs:340` - `Some(1)`; `:342` - `err.contains("vulkan") && err.contains("cuda")`; `:343` - `assert_eq!(stdout(&o), "", "fell back …")`. The message is at `apps/cli/src/bench/engine.rs:68`. It now reaches exit 1 through `From<E> for Failure` (`apps/cli/src/bench/mod.rs:77-83`, `code: 1`). | PASS (re-judged at f2f2bb3) |
| C13 | on an engine error, stderr names the error and the stem, the exit is 1, and the printed rows stay | ignored batch: `engine_error_keeps_printed_rows ... ok` | `apps/cli/tests/bench.rs:355` - `Some(1)`; `:357` - `out.lines().count() == 4`; `:358` - `rows(&out)[0][0] == "a"`; `:364` - `line.contains("run")`. The CLI's own context is `` corte `{}` `` at `apps/cli/src/bench/mod.rs:170`. | PASS (re-judged at f2f2bb3; residual R3) |
| C14 | cuts run in byte order and non-wav files are ignored | `cuts_run_in_byte_order_ignoring_other_files ... ok` | `apps/cli/tests/bench.rs:400` - `assert_eq!(names, ["B", "_c", "a", "total"])` | PASS (carried from dc1021b; citation refreshed) |
| C15 | no `.wav` exits 2 | `no_wav_exits_2 ... ok` | `apps/cli/tests/bench.rs:408` - `assert_eq!(o.status.code(), Some(2), …)` | PASS (carried from dc1021b; citation refreshed) |
| C16 | a missing reference exits 2 naming the stem, before the model loads | `missing_reference_exits_2 ... ok` | `apps/cli/tests/bench.rs:421` - `Some(2)`; `:422` - ``contains("`sem_ref`")`` | PASS (carried from dc1021b; citation refreshed) |
| C17 | a wrong WAV spec exits 2 naming the file and the value, over 4 cases | `wrong_wav_spec_exits_2 ... ok` | `apps/cli/tests/bench.rs:440` - `Some(2)`; `:442-445` - `err.contains("x.wav") && err.contains(cited)` over the 4 cases at `:429-432` | PASS (carried from dc1021b; citation refreshed) |
| C18 | `--chunk-s 30` over 65 s gives windows of 480000, 480000 and 80000 samples, joined, with wall time summed | unit: `chunks_are_consecutive_without_overlap ... ok` | `apps/cli/src/bench/engine.rs:259` - `vec![(480_000, 0.0), (480_000, 480_000.0), (80_000, 960_000.0)]`; `:261` - `assert_eq!(text, "w1 w2 w3")`; `:263` - `wall_s >= 0.060` | PASS (carried from dc1021b; citation refreshed) |
| C19 | `--out` creates the directory and writes `.txt` and `.wall_s`; `--hyp` round-trips the WER columns | ignored batch: `out_then_hyp_round_trips ... ok` | `apps/cli/tests/bench.rs:457` - nested `--out` directory; `:471` - reads `a.txt`; `:472` - `!text.trim().is_empty()`; `:486` - `format!("{wall:.2}") == engine_rows[0][2]`; `:492` - `assert_eq!(e[4..], h[4..], "WER columns differ")` | PASS (carried from dc1021b; citation refreshed; residual R4) |
| C20 | `--hyp` scores the hypotheses and the legend reads `engine=hyp model=<basename>` | `hyp_scores_and_names_dir ... ok` | `apps/cli/tests/bench.rs:505` - `out.starts_with("engine=hyp model=hyp ")`; `:506` - `rows(&out)[0][4..] == ["25.00", "0", "0", "1", "4"]` | PASS (carried from dc1021b; citation refreshed) |
| C21 | a missing `.wall_s` prints `-` in that row and in the total's `rtf` | `missing_wall_s_prints_dash ... ok` | `apps/cli/tests/bench.rs:523` - `("-", "-")`; `:524` - `r[2][3] == "-"` | PASS (carried from dc1021b; citation refreshed) |
| C22 | `--hyp` combined with each of 5 engine flags exits 2 | `hyp_rejects_engine_flags ... ok` | `apps/cli/tests/bench.rs:542-547` - `assert_eq!(o.status.code(), Some(2), …)` over the 5 flags at `:533-539`. #11 added a sixth conflict, `language`, at `apps/cli/src/bench/mod.rs:36`. That one is proven outside this feature, by `language_rejected_outside_gguf` (`bench.rs:719`, `... ok`). | PASS (re-judged at f2f2bb3) |
| C23 | `--hyp` with a missing stem exits 2 naming it | `hyp_missing_stem_exits_2 ... ok` | `apps/cli/tests/bench.rs:560` - `Some(2)`; `:561` - ``contains("`orfao`")`` | PASS (carried from dc1021b; citation refreshed) |
| C24 | stdout is exactly legend, header, separator, rows and total; `tag=-` by default, `tag=x` with `--tag x` | `stdout_is_legend_then_table ... ok` | `apps/cli/tests/bench.rs:578` - `lines.len() == 6`; `:579` - `lines[0] == LEGEND_HYP` (`…tag=-`, `:15`); `:580` - `lines[1] == HEADER`; `:589` - `legend_field(…, "tag") == "x"` | PASS (carried from dc1021b; citation refreshed) |
| C25 | with `RUST_LOG=info`, transcript words appear in neither stream | `transcript_text_never_printed_above_debug ... ok`, plus `out_then_hyp_round_trips ... ok` | `apps/cli/tests/bench.rs:602` - `!stream.contains("xiloreferencia") && !stream.contains("xilohipotese")`; `:479` - `!stream.contains(longest)` on the engine path, with `RUST_LOG=info` set at `:128`. The hypothesis log is `log::debug!` at `apps/cli/src/bench/mod.rs:172`. The new `idioma:` line is `log::info!` (`engine.rs`), and it carries the language code, not text. | PASS (re-judged at f2f2bb3; residual R2) |
| C26 | a row reaches the pipe while the next cut is still running | ignored batch: `rows_stream_while_running ... ok` | `apps/cli/tests/bench.rs:638` - `row_a.starts_with(...)` on the row-`a` prefix (pipe, space, `a`, space, pipe); `:640` - `child.try_wait().unwrap().is_none()` | PASS (carried from dc1021b; citation refreshed) |
| C27 | the report has the three headings and no "Recomenda" heading | shell proof exit 0 | `docs/spikes/01-asr-pt-br.md:5` `## Objetivo`; `:22` `## Como reproduzir`; `:106` `## Evidência medida`. `! grep -iE '^#+ .*recomenda'` matched nothing. `:20` says in prose "Este relatório mede; não recomenda". | PASS (re-judged at f2f2bb3) |
| C28 | Como reproduzir lists ffmpeg, the model origins with the sha, a bench invocation per engine, the fw script and the fala commit | shell proof exit 0 | `docs/spikes/01-asr-pt-br.md:39` `ffmpeg -ss <início> …`; `:52` Parakeet directory `parakeet-tdt-0.6b-v3-int8`; `:53` sha256 `1fc70f77…bc69`; `:68` `--engine parakeet-onnx`; `:74`, `:77` and `:82` `--engine gguf`; `:87` `bench_faster_whisper.py`; `:24-26` the commit, "anotado na tabela da rodada". The actual SHA `da03312` is recorded at `:110` and in `benchmarks/README.md:97` plus every per-cut row (`:121-155`). | PASS (re-judged at f2f2bb3; R5 narrowed, R10 new) |
| C29 | each rabbit hole records whether it fit in the hour or was abandoned, with the `device=` seen | shell proof exit 0 (count 2) | `docs/spikes/01-asr-pt-br.md:163` Vulkan: `sim, ~5 min`, `` `device=Vulkan0` ``, and not `llvmpipe`. `:164` Nemotron 3.5: `sim, ~10 min`, `` `device=cpu` ``, and "NeMo-Speech.cpp não foi tentado: o GGUF já cobre o modelo". Nemotron is now also measured in the round (`:122`). | PASS (re-judged at f2f2bb3; R6 narrowed) |
| C30 | `## Windows` has `TODO(windows)` and the cuda, voxtral and parakeet invocations | shell proof exit 0 | `docs/spikes/01-asr-pt-br.md:175` `## Windows`; `:177` `TODO(windows)`; `:182` `--features cuda`; `:190-194` the Voxtral invocation; `:197` `--engine parakeet-onnx` | PASS (re-judged at f2f2bb3) |
| C31 | the faster-whisper script writes the door 2 files and `--hyp` scores them | proof command exit 0 | `~/projects/fala-research/benchmarks/scripts/bench_faster_whisper.py:62` - `.txt` write; `:63` - `.wall_s` write. Bench printed `engine=hyp model=fw_r3`, `wall_s 13.84`. | PASS (carried from dc1021b; proof re-run at f2f2bb3) |
| C32 | Evidência medida compares the measured Parakeet v3 int8 WER and RTF on the real cuts against ADR-0003's figures (WER pt ~6 %; 10-20x real time) | shell proof exit 0 | See "C32 in detail" below. | PASS (first judged at f2f2bb3; residual R9) |
| C33 | § Rodadas has one table per round with the AC 29 columns, and at least one row per engine measured | shell proof exit 0 (count 35) | See "C33 in detail" below. | PASS (first judged at f2f2bb3; residual R8) |
| C34 | the door 1 features are exactly vulkan and cuda, and default enables neither | `cargo metadata …` piped into `jq -e …` gives `true` | `apps/cli/Cargo.toml:33` `vulkan = ["transcribe-cpp/vulkan"]`; `:34` `cuda = ["transcribe-cpp/cuda"]`; no `default` key under `[features]` (`:31`). The lines moved by +4 when #11/#9 added `cpal` and `rtrb`. | PASS (carried from dc1021b; citation refreshed) |

### C32 in detail

- **Comparison against ADR-0003.** `docs/spikes/01-asr-pt-br.md:125` introduces it: "Contra a
  ADR-0003, que cita para o Parakeet-TDT-0.6B-v3 WER pt ~6 % e 10-20x tempo real em CPU".
- **WER.** `:127-128`: "(ADR-0003: ~6 %): o Parakeet v3 int8 mediu 8.94 % nos cortes de ditado,
  17.79 % nas reuniões e 15.03 % no total".
- **Speed.** `:129-130`: "(ADR-0003: 10-20x): RTF 0.097, 10.3× tempo real, no limite de baixo
  da faixa, sem contar a carga do modelo".
- **ADR figures match the source.** `docs/decisions/0003*:8` says "Parakeet v3 int8 roda 10-20x
  tempo real com WER pt de ~6 %".
- **The report's numbers match the round data.** The report row `:117` is 8.94 / 17.79 / 15.03 /
  0.097 / 10.3×. That equals the README summary `benchmarks/README.md:109`. It also equals my
  re-scoring of `hyp/parakeet-v3-int8` with `bench --hyp`:
  - ditado: 1599 ref_words, WER 8.94.
  - reunião: 3536 ref_words, WER 17.79.
  - total: 5135 words, 772 edits, WER 15.03.
  - RTF 163.41 / 1682.15 = 0.097, which is 10.3×.
- **The real cuts were used.** The cut hashes equal `README.md:61-65`, and the durations in the
  report (`:111`: 1.682 s, 482 s and 1.200 s) equal the re-scored `audio_s` (1682.15, 482.01 and
  1200.14).
- **The other rows also match.** Rows `:118-123` equal the README summary `:110-115` and my
  re-scoring. The per-type engine order at `:131-132` holds against the numbers:
  - ditado: 6.19 < 7.88-8.32 < 8.94 < 10.63.
  - reunião: 10.58 < 12.75 < 14.23-14.28 < 17.79.
- **No recommendation.** The comparison reports numbers and does not recommend (`:20`).

### C33 in detail

- **Section.** `~/projects/fala-research/benchmarks/README.md:93` is `## Rodadas`. `:95` states
  the rule: one table per round, one row per (engine, modelo, threads, device, corte), and a
  repeated round gets a new table.
- **One table per round.** There is exactly one round heading, `:97`
  (`### 2026-09-29 · Linux … · fala da03312`). It carries a summary table at `:107-115` and the
  per-cut table at `:119-155`.
- **Columns.** The per-cut header `:119` is
  `| data | máquina | engine | modelo | versão | threads | device | corte | WER | RTF | commit |`,
  which is all 10 AC 29 columns plus `commit`.
- **One row per tuple.** There are 35 rows (`:121-155`), which is 7 configurations times 5 cuts.
  Grouping them by (engine, modelo, threads, device, corte) gives 35 groups of 1, so there are no
  duplicates and no gaps.
- **At least one row per engine measured.** `parakeet-onnx` at `:121-125`; `gguf` whisper
  (t4/t8/t12/Vulkan0) at `:126-145`; `gguf` Nemotron at `:146-150`; faster-whisper at
  `:151-155`.
- **Every value matches the data.** Every per-cut WER and RTF equals what `bench --hyp` prints
  for the matching `hyp/<config>/` at HEAD.

## Coverage

Carried from round 1 via round 2 (`dc1021b`). The profile is `light`, so the join is not
recomputed. One observation from #11, which is not a Coverage verdict: the `--hyp`-rejected flag
set in the code is now 6 (`apps/cli/src/bench/mod.rs:36`), against 5 in the plan's AC 21 and in
C22. The 6th flag is proven outside this feature (see C22).

## Swept existing

Verified at `f2f2bb3`. No Swept row resolves to "existing". The prose claim under "dependency
failure", "model load failure exits 1 via the same path", still holds:

- The test `missing_model_exits_1` (`apps/cli/tests/bench.rs:368`, `... ok`) asserts `:377`
  `Some(1)` and `:378` empty stdout, for both engines.
- For Parakeet, the path is `load_parakeet(model).map_err(engine_failure)?` at
  `apps/cli/src/bench/mod.rs:141`, and `engine_failure` gives code 1 (`:90-91`).
- For gguf, the path is now `load_gguf(…)?` through the blanket `From<E> for Failure`
  (`mod.rs:77-83`, `code: 1`). The only code-2 exit on that path is the language mismatch at
  `engine.rs:111-112`.

## Gaps

None of these fail the verdict. Every check has a green proof at `f2f2bb3` and a located
assertion. Ranked:

1. **R10 (C28/C33, reproducibility, new).** The round-table command in Como reproduzir is
   `docs/spikes/01-asr-pt-br.md:91`: `score_rounds.py --fala … --date … --machine … --commit …`,
   with no configuration names. Without names, the script scores every directory under `hyp/`
   (`~/projects/fala-research/benchmarks/scripts/score_rounds.py:54`), and that includes the two
   `hyp/control-parakeet-{pre,post}/`. Their legends carry `engine=parakeet-onnx` and
   `threads=- device=cpu`. So the documented command would print 45 rows, with the Parakeet
   (engine, modelo, threads, device, corte) tuples duplicated, instead of the 35 rows at
   `README.md:121-155`. The published table meets AC 29; the command in the report does not
   reproduce it exactly.
2. **R8 (C33, precision gap in the proof).** The proof counts lines starting `| 2026-` (≥ 1). It
   does not assert the 10 columns, one table per round, or one row per tuple. Those were
   confirmed by reading `README.md:97`, `:119` and `:121-155`, and by the uniqueness count above.
3. **R9 (C32, precision gap in the proof).** The proof needs one line that mentions ADR-0003 and
   carries a decimal percentage. It matched only the WER line, `:127`. It does not assert the
   speed comparison (`:129`), and it does not check that the numbers equal the round data. Both
   were confirmed by reading and by re-scoring `hyp/`.
4. **R11 (plan drift from #11, outside this feature's checks).** Plan AC 9 says
   `language = Some("pt")`. The code now asks for `pt-BR` and resolves it against the model's
   list (`apps/cli/src/bench/engine.rs:111-112`, `:166`). whisper still gets `pt`, as
   `whisper_keeps_bare_pt` (`bench.rs:682`) shows, but that test was not run this round. Nemotron
   gets `pt-BR`. No cli-bench check ever covered AC 9's language (carried gap), and `plan.md` was
   not updated. Also, the rounds brief gave the diff since round 2 as `da03312..f2f2bb3`, but it
   is `dc1021b..f2f2bb3`, which includes #11. That is why C10-C13, C22 and C25 were re-judged.
5. **R5 (C28, narrowed).** The SHA `da03312` is now recorded, at `:110` and on every round row.
   `## Como reproduzir` itself (`:24-26`) still gives the procedure and a pointer, not the SHA,
   and the proof's `grep -qF commit` matches any mention of the word.
6. **R6 (C29, narrowed).** The proof still counts 2 rows only. Fit/abandon and `device=` were
   confirmed by reading `:163-164`. The Nemotron measurement that round 2 flagged as missing now
   exists (`:122`).
7. **R2 (C25, carried from dc1021b).** On the engine path, only the longest word of the
   hypothesis is checked (`bench.rs:474-479`). Nothing from the reference is checked on the
   engine path.
8. **R3 (C13, carried from dc1021b).** `bench.rs:364` is coupled to the engine's error text
   (`run`).
9. **R4 (C19, carried from dc1021b).** That the `.txt` holds the *raw*, not normalized,
   hypothesis is not asserted (`bench.rs:472`, `:492`).
10. **R7 (Swept, carried from dc1021b).** `bench.rs:377-378` does not assert that stderr names
    the load failure.
11. **Carried from round 1, no check:**
    - AC 6's 250 ms leading silence.
    - Door 1's dependency list omits `icu_normalizer`.

## Gate

- `cargo test -p fala-cli`:
  - unit: 14 passed, 0 failed. 5 of them belong to cli-bench.
  - `tests/bench.rs`: 16 passed, 0 failed, 10 ignored. 15 of them belong to cli-bench.
  - `tests/record.rs`: 7 passed, 6 ignored. Not part of this feature.
- Model-backed ignored tests: 6 of 6 passed in the batch (`--test-threads=1`, 107.73 s).
  `gguf_gpu_reports_device` passed on the Vulkan build: 1 of 1.
- Shell proofs: 8 of 8 exited 0 (C27, C28, C29, C30, C31, C32, C33 and C34).
- Data re-scoring for C32/C33: 7 of 7 configurations matched the README and the report.
- Checks: 34 of 34 proven with located evidence.
