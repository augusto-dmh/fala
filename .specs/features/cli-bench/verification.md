# cli-bench verification

**Verdict**: PASS
**Profile**: light
**Diff range**: 773cea7..dc1021b2cb1aa7b00fd2251f871209c3900fafc1
**Round**: 2 - scoped
**Verifier**: independent sub-agent (author != verifier)

Verified at `dc1021b` on branch `docs/asr-spike-report`. `git status --porcelain` was empty
before and after the run. The history was rewritten with autosquash. Round 1's head, `9efdd4c`,
is still in the object store, and `git diff --stat 9efdd4c dc1021b` shows the fix touches exactly
three files: `apps/cli/tests/bench.rs` (+66/-12 lines, counting insertions and deletions
together), `.specs/features/cli-bench/checks.md` (+5/-1) and `docs/spikes/01-asr-pt-br.md`
(+4/-3). `git diff --quiet 9efdd4c dc1021b -- apps/cli/src apps/cli/Cargo.toml` exits 0, so no
production code changed since round 1.

Scope for this round:
- **Proofs:** every proof for C1-C31 and C34 was re-run at `dc1021b`.
- **Re-judged in full:** C8, C29, the checks whose tests the fix touched (C5, C11, C13, C19, C25
  and C28), and the new `missing_model_exits_1` against the Swept "dependency failure" row.
- **Citations:** every `apps/cli/tests/bench.rs` citation was refreshed, because a new 14-line
  helper at `:65-77` moved every test below it. Citations in `docs/spikes/01-asr-pt-br.md` were
  also refreshed.
- **Carried from round 1:** every other judgment. Those rows say `carried from round 1`.

C32 and C33 (S8) are deferred and do not count toward the verdict. See `## Deferred`.

## Binding sources

Carried from round 1. The profile is `light`, so step 1 does not run. The plan's `Sources` are
context only, none is marked binding, and there is no screen.

## Proofs run

All of these ran at `dc1021b`.

| Invocation | Result |
| --- | --- |
| `cargo test -p fala-cli` | Unit tests: 5 passed, each listed `... ok` (`bench::wer::tests::{guarda_chuva_counts_one_insertion, levenshtein_counts_sub_del_ins, normalize_follows_door_3, ola_mundo_is_zero}`, `bench::engine::tests::chunks_are_consecutive_without_overlap`). `tests/bench.rs`: **15 passed**, 0 failed, 7 ignored. Each of the 15 is listed `... ok`, including the new `missing_model_exits_1`. |
| `FALA_TEST_PARAKEET_DIR=… FALA_TEST_GGUF=… FALA_TEST_SPEECH_WAV=… cargo test -p fala-cli --test bench -- --ignored --exact parakeet_onnx_prints_one_row_per_cut load_time_is_excluded_from_wall_s gguf_legend_reports_threads_and_cpu engine_error_keeps_printed_rows out_then_hyp_round_trips rows_stream_while_running` | 6 passed, 0 failed, 174.20 s. Each is listed `... ok`. |
| `… cargo test -q -p fala-cli --test bench -- --ignored --exact load_time_is_excluded_from_wall_s`, run alone 8 times | 8 of 8 `1 passed; 0 failed; … 21 filtered out` (3.15 to 5.34 s) |
| the same test without `-q`, alone once more | `test load_time_is_excluded_from_wall_s ... ok` |
| manual `target/debug/fala-cli bench … --engine parakeet-onnx` on a 2 s `ffmpeg -t 2` prefix of the speech WAV, 3 runs | `load_s` 3.09, 3.39, 3.08; the cut's `wall_s` 0.55, 0.44, 0.55. Transcription takes about a sixth of the load. |
| `FALA_TEST_GGUF=… FALA_TEST_SPEECH_WAV=… cargo test -p fala-cli --features vulkan --test bench -- --ignored --exact gguf_gpu_reports_device` | `test gguf_gpu_reports_device ... ok`, 11.58 s |
| manual `RUST_LOG=info target/debug/fala-cli bench … --engine {parakeet-onnx,gguf} --model /nonexistent/model.bin` | exit 1 for both engines. stderr: `ERROR fala_cli] não consegui carregar o Parakeet de /nonexistent/model.bin: …` and `ERROR fala_cli] model file not found: load /nonexistent/model.bin: file not found (status 3)` |
| C27, C28, C29 and C30 shell proofs, as written in `checks.md`, from the repo root | exit 0 each |
| C34 `cargo metadata … \| jq -e …` | `true`, exit 0 |
| C31 `uv run …/bench_faster_whisper.py --cuts $S/cuts --out $S/fw_r2 && cargo run -q -p fala-cli -- bench --cuts $S/cuts --refs $S/refs --hyp $S/fw_r2` | exit 0. The script wrote `smoke.txt` and `smoke.wall_s`. The table printed `engine=hyp model=fw_r2 …` with `wall_s 24.07`. The output directory is `fw_r2` rather than `fw`, so round 1's output was not overwritten. It was deleted afterwards. |

Every named test exists. Each `fn` sits at the line cited below, and each name appeared in the
runner output.

**C8 pass count at `dc1021b`:** 10 of 10 runs. That is 1 in the batch (run in parallel with the
gguf tests under full CPU load), 8 alone with `-q`, and 1 alone showing the test name. In round 1,
on the 15 s cut, it failed 5 of 9 runs.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | door 3 normalization on the decomposed-`Á` sample | unit run: `normalize_follows_door_3 ... ok` | `apps/cli/src/bench/wer.rs:97` - `assert_eq!(normalize("A\u{301}  B-c'd, 2!"), vec!["á", "b", "c", "d", "2"])` (file unchanged) | PASS (judgment carried from round 1; proof re-run at dc1021b) |
| C2 | Levenshtein sub/del/ins; `wer_%` with 2 decimals | unit run: `levenshtein_counts_sub_del_ins ... ok` | `apps/cli/src/bench/wer.rs:106` - `(1, 0, 0, 3)`; `:107` `wer_pct() == "33.33"`; `:110` `(0, 1, 0, 3)`; `:113` `(0, 0, 1, 2)` | PASS (judgment carried from round 1; proof re-run at dc1021b) |
| C3 | `Olá, mundo!` vs `olá mundo` gives `0.00` | unit run: `ola_mundo_is_zero ... ok` | `apps/cli/src/bench/wer.rs:120` - `assert_eq!(s.wer_pct(), "0.00")` | PASS (judgment carried from round 1; proof re-run at dc1021b) |
| C4 | guarda-chuva: 0/0/1/4, `25.00` | unit run: `guarda_chuva_counts_one_insertion ... ok` | `apps/cli/src/bench/wer.rs:129` - `(0, 0, 1, 4)`; `:130` `"25.00"` | PASS (judgment carried from round 1; proof re-run at dc1021b) |
| C5 | the `total` row pools `ref_words`, `wer_%` and `rtf` | `total_line_pools_wer_and_rtf ... ok` | `apps/cli/tests/bench.rs:185` - `assert_eq!(total[8], "4")`; `:186` `assert_eq!(total[4], "25.00")`, where the mean would be 50.00; `:187` `assert_eq!(total[3], "0.875")`. The data at `:175-177` gives per-cut rtf of 0.5 and 1.0, so pooled Σwall/Σaudio = 3.5/4 = 0.875 while the mean of rtf = 0.750. The rtf half now discriminates. | PASS (P2 closed) |
| C6 | an empty reference exits 2 naming the file, before the model loads | `empty_reference_exits_2 ... ok` | `apps/cli/tests/bench.rs:199` - `assert_eq!(o.status.code(), Some(2), …)`; `:200` `stderr(&o).contains("a.txt")` | PASS (judgment carried from round 1; citation refreshed) |
| C7 | parakeet-onnx prints legend, header, separator, row and total; rtf has 3 decimals and is < 1 | ignored batch: `parakeet_onnx_prints_one_row_per_cut ... ok` | `apps/cli/tests/bench.rs:234` - `assert_eq!(out.lines().count(), 5)`; `:243` `rtf.split('.').nth(1).map(str::len) == Some(3)`; `:244` `rtf.parse::<f64>().unwrap() < 1.0` | PASS (judgment carried from round 1; citation refreshed) |
| C8 | `load_s` appears only in the legend and is excluded from `wall_s`: on a 2 s speech cut, `wall_s` < `load_s` | 10 of 10 runs `ok` (see Proofs run) | `apps/cli/tests/bench.rs:255` - `c.speech_prefix("a", 2)`, where the helper at `:66-77` writes the first `2 * sample_rate` samples of `FALA_TEST_SPEECH_WAV`, matching the check's revised text. `:263` - `assert!(!out.lines().nth(1).unwrap().contains("load"))` (no table column). `:266` - `assert!(wall_s < load_s, …)`. If load time were inside `wall_s`, then `wall_s >= load_s` would always hold, so this assertion does discriminate. Measured margin: `wall_s` 0.44-0.55 against `load_s` 3.08-3.39. | PASS |
| C9 | parakeet rejects `--threads` and `--device gpu` with exit 2 | `parakeet_rejects_threads_and_device ... ok` | `apps/cli/tests/bench.rs:215` - `Some(2)` with `:216` `contains("--threads")`; `:218` `Some(2)` with `:219` `contains("--device gpu")` | PASS (judgment carried from round 1; citation refreshed) |
| C10 | the gguf legend shows `threads=8 device=cpu`; the default is `available_parallelism` | ignored batch: `gguf_legend_reports_threads_and_cpu ... ok` | `apps/cli/tests/bench.rs:281-287` - `.contains(" threads=8 device=cpu ")`; `:295` `assert_eq!(legend_field(&stdout(&o), "threads"), n)` | PASS (judgment carried from round 1; citation refreshed) |
| C11 | in a vulkan build, `--device gpu` exits 0 and `device=` is the `Model::device()` name, never `cpu` | `--features vulkan … gguf_gpu_reports_device ... ok` | `apps/cli/tests/bench.rs:308` - `assert_eq!(o.status.code(), Some(0))`; `:310` `assert_ne!(device, "cpu")`; `:312-315` the stderr line `dispositivo GPU: {device} ` must exist (emitted at `apps/cli/src/bench/engine.rs:65-66`); `:316-319` `assert!(!line.contains("llvmpipe"), "ran on the software rasterizer: …")`. The software rasterizer now fails the test. | PASS (L1 closed) |
| C12 | with no GPU feature, `--device gpu` exits 1 naming vulkan and cuda, with no fallback | `gpu_without_backend_exits_1 ... ok` | `apps/cli/tests/bench.rs:339` - `Some(1)`; `:341` `err.contains("vulkan") && err.contains("cuda")`; `:342` `assert_eq!(stdout(&o), "")` | PASS (judgment carried from round 1; citation refreshed) |
| C13 | on an engine error, stderr names the error and the stem, the exit is 1, and printed rows stay | ignored batch: `engine_error_keeps_printed_rows ... ok` | `apps/cli/tests/bench.rs:354` - `Some(1)`; `:356` `out.lines().count() == 4`; `:357` `rows(&out)[0][0] == "a"`; `:358-362` finds the stderr line containing `` `b` ``; `:363` `assert!(line.contains("run"), "engine error text missing")`. The CLI's own context is `` corte `b` `` (`apps/cli/src/bench/mod.rs:152`), so `run` can only come from the engine's error (`invalid argument: run: …`, seen in round 1's manual run). | PASS (P3 closed; see residual R3) |
| C14 | cuts run in byte order (`B` < `_c` < `a`) and non-wav files are ignored | `cuts_run_in_byte_order_ignoring_other_files ... ok` | `apps/cli/tests/bench.rs:399` - `assert_eq!(names, ["B", "_c", "a", "total"])` | PASS (judgment carried from round 1; citation refreshed) |
| C15 | no `.wav` exits 2 | `no_wav_exits_2 ... ok` | `apps/cli/tests/bench.rs:407` - `assert_eq!(o.status.code(), Some(2), …)` | PASS (judgment carried from round 1; citation refreshed) |
| C16 | a missing reference exits 2 naming the stem, before the model loads | `missing_reference_exits_2 ... ok` | `apps/cli/tests/bench.rs:420` - `Some(2)`; `:421` ``contains("`sem_ref`")`` | PASS (judgment carried from round 1; citation refreshed) |
| C17 | a wrong WAV spec exits 2 naming the file and the value, over 4 cases | `wrong_wav_spec_exits_2 ... ok` | `apps/cli/tests/bench.rs:439` - `Some(2)`; `:441-444` `err.contains("x.wav") && err.contains(cited)`, over `48000 Hz`, `2 canal`, `32 bits float` and `8 bits int` (`:428-431`) | PASS (judgment carried from round 1; citation refreshed) |
| C18 | `--chunk-s 30` over 65 s gives windows of 480000, 480000 and 80000 samples, joined, with wall time summed | unit run: `chunks_are_consecutive_without_overlap ... ok` | `apps/cli/src/bench/engine.rs:177` - `seen == [(480_000, 0.0), (480_000, 480_000.0), (80_000, 960_000.0)]`; `:181` `text == "w1 w2 w3"`; `:183` `wall_s >= 0.060` (file unchanged) | PASS (judgment carried from round 1; proof re-run at dc1021b) |
| C19 | `--out` creates the directory and writes the `.txt` (raw hypothesis) and `.wall_s`; `--hyp` round-trips the WER columns | ignored batch: `out_then_hyp_round_trips ... ok` | `apps/cli/tests/bench.rs:470` - `fs::read_to_string(out_dir.join("a.txt")).unwrap()` into the nested `--out` directory created at `:456`; `:471` `assert!(!text.trim().is_empty())`; `:485` `format!("{wall:.2}") == engine_rows[0][2]`; `:491` `assert_eq!(e[4..], h[4..], "WER columns differ")` | PASS (P4 narrowed; see residual R4) |
| C20 | `--hyp` scores the hypotheses and the legend reads `engine=hyp model=<basename>` | `hyp_scores_and_names_dir ... ok` | `apps/cli/tests/bench.rs:504` - `out.starts_with("engine=hyp model=hyp ")`; `:505` `rows(&out)[0][4..] == ["25.00", "0", "0", "1", "4"]` | PASS (judgment carried from round 1; citation refreshed) |
| C21 | a missing `.wall_s` prints `-` in that row and in the total's rtf | `missing_wall_s_prints_dash ... ok` | `apps/cli/tests/bench.rs:522` - `(r[1][2], r[1][3]) == ("-", "-")`; `:523` `r[2][3] == "-"` | PASS (judgment carried from round 1; citation refreshed) |
| C22 | `--hyp` combined with each of 5 engine flags exits 2 | `hyp_rejects_engine_flags ... ok` | `apps/cli/tests/bench.rs:541-546` - `assert_eq!(o.status.code(), Some(2), …)` over the 5 flags at `:532-538` | PASS (judgment carried from round 1; citation refreshed) |
| C23 | `--hyp` with a missing stem exits 2 naming it | `hyp_missing_stem_exits_2 ... ok` | `apps/cli/tests/bench.rs:559` - `Some(2)`; `:560` ``contains("`orfao`")`` | PASS (judgment carried from round 1; citation refreshed) |
| C24 | stdout is exactly legend, header, separator, rows and total; `tag=-` by default and `tag=x` with `--tag x` | `stdout_is_legend_then_table ... ok` | `apps/cli/tests/bench.rs:577` - `lines.len() == 6`; `:578` `lines[0] == LEGEND_HYP` (`…tag=-`, `:14`); `:579` `lines[1] == HEADER`; `:588` `legend_field(…, "tag") == "x"` | PASS (judgment carried from round 1; citation refreshed) |
| C25 | with `RUST_LOG=info`, transcript words appear in neither stream | `transcript_text_never_printed_above_debug ... ok`, plus `out_then_hyp_round_trips ... ok` | `apps/cli/tests/bench.rs:600-603` - `!stream.contains("xiloreferencia") && !stream.contains("xilohipotese")` on the `--hyp` path. For the engine path: `:473-479` takes the longest word of the Parakeet hypothesis and asserts `!stream.contains(longest)` for both stdout and stderr, and the `fala()` helper sets `RUST_LOG=info` (`:127`). The engine path's hypothesis log (`apps/cli/src/bench/mod.rs:154`, `log::debug!`) is now held by a test. | PASS (L2 closed; see residual R2) |
| C26 | a row reaches the pipe while the next cut is still running | ignored batch: `rows_stream_while_running ... ok` | `apps/cli/tests/bench.rs:637` - `row_a.starts_with(...)` on the row-`a` prefix (pipe, space, `a`, space, pipe); `:638-641` `child.try_wait().unwrap().is_none()` | PASS (judgment carried from round 1; citation refreshed) |
| C27 | the report has the three headings and no "Recomenda" heading | shell proof exit 0 | `docs/spikes/01-asr-pt-br.md:5` `## Objetivo`, `:22` `## Como reproduzir`, `:88` `## Evidência medida`; `! grep -iE '^#+ .*recomenda'` matched nothing | PASS (judgment carried from round 1; lines unchanged) |
| C28 | Como reproduzir lists ffmpeg, the model origins with the sha, a bench invocation per engine, the fw script and the fala commit | shell proof exit 0 | `docs/spikes/01-asr-pt-br.md:37` `ffmpeg -ss …`; `:47` the Parakeet directory; `:48` sha256 `1fc70f77…bc69`; `:63` `--engine parakeet-onnx`; `:69`/`:72`/`:76` `--engine gguf`; `:80` `bench_faster_whisper.py`; `:24-25` "o commit do `fala` em `main` que contém `fala-cli bench` (`git rev-parse --short HEAD`), anotado na tabela da rodada". This section did not change in the fix. The new sentence at `:98-99` names branch `feat/cli-bench-engines` for the smoke runs, not a SHA. | PASS (P5 still open; see residual R5) |
| C29 | each rabbit hole records whether it fit in the hour or was abandoned, with the `device=` seen | shell proof exit 0 (counts 2 rows) | `docs/spikes/01-asr-pt-br.md:106` Vulkan row: `sim, ~5 min`, `` `device=Vulkan0` `` (Intel RPL-U, not llvmpipe). `:107` Nemotron row: `sim, ~10 min (medição bloqueada pelo idioma, não pelo tempo)`, `` `device=cpu` ``. `rg -n 'device=' docs/spikes/01-asr-pt-br.md` in this section hits `:106` and `:107`. Both members now carry a verdict and a device. | PASS (see residual R6) |
| C30 | `## Windows` has `TODO(windows)` and the cuda, voxtral and parakeet invocations | shell proof exit 0 | `docs/spikes/01-asr-pt-br.md:120` `TODO(windows)`; `:125` `--features cuda`; `:133-136` the Voxtral invocation; `:139` `--engine parakeet-onnx` | PASS (judgment carried from round 1; citation refreshed) |
| C31 | the faster-whisper script writes the door 2 files and `--hyp` scores them | proof command exit 0 | `~/projects/fala-research/benchmarks/scripts/bench_faster_whisper.py:62` - `(args.out / f"{wav.stem}.txt").write_text(text, …)`; `:63` `(args.out / f"{wav.stem}.wall_s").write_text(f"{wall_s:.3f}\n", …)`; bench printed `engine=hyp model=fw_r2`, `wall_s 24.07` | PASS (judgment carried from round 1; outside this repo's diff) |
| C34 | the door 1 features are exactly vulkan and cuda, and default enables neither | `cargo metadata …` piped into `jq -e …` gives `true` | `apps/cli/Cargo.toml:29` `vulkan = ["transcribe-cpp/vulkan"]`; `:30` `cuda = ["transcribe-cpp/cuda"]`; no `default` key | PASS (judgment carried from round 1; proof re-run at dc1021b) |

## Deferred

These checks are blocked by plan open question 1 (the user's decision of 2026-09-27). They are
not in the Checks table and do not count toward the verdict. Both proofs were re-run at `dc1021b`
and are still not satisfied, as expected.

| Deferred item | Claim | Proof run | Status |
| --- | --- | --- | --- |
| C32 | Evidência medida compares the measured Parakeet WER and RTF with ADR-0003 | `awk … \| grep ADR-0003 \| grep -E '[0-9]+\.[0-9]+ ?%'` gives exit 1, no match. `docs/spikes/01-asr-pt-br.md:93` says the comparison waits for the rounds. | BLOCKED - not satisfied |
| C33 | benchmarks/README § Rodadas has a round table | `awk … \| grep -cE '^\| 2026-' \| awk …` gives exit 1 | BLOCKED - not satisfied |

## Coverage

Carried from round 1. The profile is `light`, so the join is not recomputed. The fix added a
member to the Swept "dependency failure" row, which is judged below.

## Swept existing

Verified at `dc1021b`.

No Swept row resolves to "existing". The prose claim under "dependency failure", "model load
failure exits 1 via the same path", is now proven by a test:

- The test is `apps/cli/tests/bench.rs:367` `missing_model_exits_1`, which listed `... ok` in the
  non-ignored run. For both `parakeet-onnx` and `gguf` with `--model /nonexistent/model.bin`, it
  asserts `:376` `assert_eq!(o.status.code(), Some(1), …)` and `:377`
  `assert_eq!(stdout(&o), "")`.
- The code path is the load call's `.map_err(engine_failure)?` at `apps/cli/src/bench/mod.rs:132`,
  which maps to code 1 at `:77-78`.
- Manual runs confirm stderr names the load failure for both engines (see Proofs run). The test
  does not assert that; see residual R7.

## Gaps

None of these fail the verdict. Every counted check has a located assertion and a green proof.
The round-1 gaps P2, L1 and P3 are closed, and L2 and P4 are closed or narrowed. These residuals
remain, ranked:

- **R5 (C28, P5 carried).** `docs/spikes/01-asr-pt-br.md:24-25` records how to get the commit,
  not a `fala` SHA. The proof's `grep -qF commit` matches any mention. The new sentence at
  `:98-99` names a local branch "antes do squash", which cannot be reproduced once squashed.
- **R6 (C29).** The proof, `grep -c … | grep -qx 2` in `checks.md`, still counts rows only. It
  does not assert the fit/abandon verdict or `device=`, which were confirmed by reading `:106-107`.
  Also, the Nemotron verdict "sim" means the investigation fit the hour. The measurement itself
  was not made; the row says so.
- **R2 (C25).** The check's named proof (`bench.rs:592`) still exercises only `--hyp`. The
  engine-path no-leak assertion lives in C19's proof (`bench.rs:473-479`) and checks one word
  (the longest) of the hypothesis. It checks nothing from the reference on the engine path.
- **R3 (C13).** `bench.rs:363` checks the 3-letter substring `run` from `transcribe_cpp`'s
  message. It does discriminate from the CLI's own wording, but it is coupled to the engine's
  error text.
- **R4 (C19).** `bench.rs:471` asserts the `.txt` is non-empty. That the content is *raw*, not
  normalized, is not asserted: a lowercased and depunctuated file would still pass `:491`.
- **R7 (Swept).** `bench.rs:376-377` asserts exit 1 and empty stdout, but not that stderr names
  the load failure.
- **Carried from round 1:** these plan criteria have no check.
  - AC 9 `language = Some("pt")`, at `apps/cli/src/bench/engine.rs:112`. The report,
    `01-asr-pt-br.md:107`, shows this is what blocks Nemotron.
  - AC 6's 250 ms leading silence, at `engine.rs:105-107`.
  - Door 1's literal dependency list omits `icu_normalizer`, which `apps/cli/Cargo.toml:23` adds.

## Gate

- `cargo test -p fala-cli`: 20 passed (5 unit, 15 integration), 0 failed, 7 ignored.
- Ignored model-backed tests: 6 of 6 passed in the batch. `gguf_gpu_reports_device` passed on
  the Vulkan build.
- C8, `load_time_is_excluded_from_wall_s`: 10 of 10 runs passed.
- Shell proofs: 6 of 6 exited 0 (C27-C31, C34).
- Deferred C32 and C33: 2 of 2 still exit 1.
- Checks: 32 of 32 counted checks proven with located evidence (C1-C31, C34).
