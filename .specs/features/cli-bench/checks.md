# cli-bench - checks

Profile: light
Plan: `.specs/features/cli-bench/plan.md`

## Intent

34 checks in 8 slices · 3 one-way doors · 1 open, of which 1 blocks (plan open question 1: cortes e
referências do Augusto; bloqueia só S8)

Model-backed proofs are `#[ignore]`d tests that read their inputs from the environment and **fail**
(never pass) when a variable is missing: `FALA_TEST_PARAKEET_DIR` (Parakeet v3 int8 dir),
`FALA_TEST_GGUF` (a GGUF/ggml file, here `ggml-large-v3-turbo.bin`), `FALA_TEST_SPEECH_WAV` (a
16 kHz mono i16 WAV of pt-BR speech, 5-20 s). They run with `-- --ignored --exact <name>`.

## Checks

Unit proofs: `cargo test -p fala-cli --bin fala-cli <path>`. Boundary proofs spawn the built binary
(`CARGO_BIN_EXE_fala-cli`) from `apps/cli/tests/bench.rs`: `cargo test -p fala-cli --test bench`.

### S1 - WER e agregado · 3 files · 20 KB · ~5k

**C1** - Normalization is NFC → lowercase → every run of non-letter/non-digit chars becomes one space → split on space: `"A\u{301}  B-c'd, 2!"` (decomposed `Á`) yields `["á","b","c","d","2"]` (plan AC 1, door 3)
Proof: `cargo test -p fala-cli --bin fala-cli bench::wer::tests::normalize_follows_door_3`

**C2** - Word-level Levenshtein reports the minimum edit split: `a b c`→`a x c` is sub 1; `a b c`→`a c` is del 1; `a b`→`a b c` is ins 1; `wer_%` = edits/ref_words × 100 formatted with 2 decimals (plan AC 1)
Proof: `cargo test -p fala-cli --bin fala-cli bench::wer::tests::levenshtein_counts_sub_del_ins`

**C3** - Reference `Olá, mundo!` vs hypothesis `olá mundo` scores `wer_%` = `0.00` (plan AC 2)
Proof: `cargo test -p fala-cli --bin fala-cli bench::wer::tests::ola_mundo_is_zero`

**C4** - Reference `o guarda-chuva ficou` vs `o guarda chuva ficou aqui` scores sub 0, del 0, ins 1, ref_words 4, `wer_%` `25.00` (plan AC 3)
Proof: `cargo test -p fala-cli --bin fala-cli bench::wer::tests::guarda_chuva_counts_one_insertion`

**C5** - The `total` row pools: `ref_words` = Σ, `wer_%` = Σ(sub+del+ins)/Σref_words × 100 (two cuts at 1/1 and 0/3 words print total `25.00`, not the mean `50.00`), `rtf` = Σwall_s/Σaudio_s (plan AC 4)
Proof: `cargo test -p fala-cli --test bench -- --exact total_line_pools_wer_and_rtf`

**C6** - A reference with 0 words after normalization exits 2 naming the file, before any model loads (run with `--engine parakeet-onnx --model <nonexistent>`: exit is 2, not the load failure's 1) (plan AC 5)
Proof: `cargo test -p fala-cli --test bench -- --exact empty_reference_exits_2`

### S2 - engine parakeet-onnx · 4 files · 30 KB · ~8k

**C7** - `--engine parakeet-onnx --model <dir>` loads Parakeet v3 int8 and prints legend + header + separator + one row per cut + `total`, with `audio_s`, `wall_s`, `rtf` (3 decimals) numeric and `rtf` < 1 on a speech cut (plan AC 6)
Proof: `cargo test -p fala-cli --test bench -- --ignored --exact parakeet_onnx_prints_one_row_per_cut`

**C8** - `load_s` is reported only in the legend (no table column) and is excluded from `wall_s`: on a 2 s speech cut (the first 2 s of `FALA_TEST_SPEECH_WAV`) the cut's `wall_s` is below the legend's `load_s` (plan AC 7)
Proof: `cargo test -p fala-cli --test bench -- --ignored --exact load_time_is_excluded_from_wall_s`

**C9** - `--threads` and `--device gpu` with `--engine parakeet-onnx` each exit 2 saying the engine does not expose the option (plan AC 8)
Proof: `cargo test -p fala-cli --test bench -- --exact parakeet_rejects_threads_and_device`

### S3 - engine gguf · 4 files · 30 KB · ~8k

**C10** - `--engine gguf --model <file> --threads 8` prints `threads=8 device=cpu` in the legend and one row per cut; without `--threads` the legend shows `threads=<available_parallelism>` (plan AC 9)
Proof: `cargo test -p fala-cli --test bench -- --ignored --exact gguf_legend_reports_threads_and_cpu`

**C11** - In a build with the `vulkan` feature and a GPU/iGPU device, `--device gpu` exits 0 and the legend's `device=` is the loaded `Model::device()` name, never `cpu` (plan AC 10)
Proof: `cargo test -p fala-cli --features vulkan --test bench -- --ignored --exact gguf_gpu_reports_device`

**C12** - In a build with no GPU feature, `--device gpu` exits 1 naming the missing backend (`vulkan`/`cuda`), without falling back to CPU (plan AC 11)
Proof: `cargo test -p fala-cli --test bench -- --exact gpu_without_backend_exits_1`

**C13** - When the engine errors on a cut, stderr names the error and the `stem`, the exit is 1, and the rows already printed stay on stdout (plan AC 12)
Proof: `cargo test -p fala-cli --test bench -- --ignored --exact engine_error_keeps_printed_rows`

**C34** - `apps/cli/Cargo.toml` declares exactly the door 1 features `vulkan = ["transcribe-cpp/vulkan"]` and `cuda = ["transcribe-cpp/cuda"]`, and the default build enables neither (plan door 1)
Proof: `cargo metadata --no-deps --format-version 1 | jq -e '.packages[] | select(.name=="fala-cli") | .features == {"cuda":["transcribe-cpp/cuda"],"default":[],"vulkan":["transcribe-cpp/vulkan"]} or .features == {"cuda":["transcribe-cpp/cuda"],"vulkan":["transcribe-cpp/vulkan"]}'`

### S4 - corpus e recorte · 3 files · 20 KB · ~5k

**C14** - Cuts run in ascending byte order of file name (`B.wav` < `_c.wav` < `a.wav`) and non-`.wav` files are ignored (plan AC 13)
Proof: `cargo test -p fala-cli --test bench -- --exact cuts_run_in_byte_order_ignoring_other_files`

**C15** - A `--cuts` dir with no `.wav` exits 2 (plan AC 14)
Proof: `cargo test -p fala-cli --test bench -- --exact no_wav_exits_2`

**C16** - A cut with no `<refs>/<stem>.txt` exits 2 naming the stem, before any model loads (plan AC 15)
Proof: `cargo test -p fala-cli --test bench -- --exact missing_reference_exits_2`

**C17** - A WAV that is not 16000 Hz / 1 channel / 16-bit int exits 2 naming the file and the spec found; table-driven over 48000 Hz, 2 channels, 32-bit float and 8-bit int, each message citing the offending value (plan AC 16)
Proof: `cargo test -p fala-cli --test bench -- --exact wrong_wav_spec_exits_2`

**C18** - `--chunk-s 30` over 65 s of samples runs 3 consecutive windows of 480000, 480000, 80000 samples with no overlap, joins the trimmed texts with a single space, and sums the per-window wall time (plan AC 17)
Proof: `cargo test -p fala-cli --bin fala-cli bench::engine::tests::chunks_are_consecutive_without_overlap`

### S5 - hipóteses externas · 3 files · 20 KB · ~5k

**C19** - `--out <dir>` with an engine creates `<dir>` and writes `<stem>.txt` (raw hypothesis) and `<stem>.wall_s` (the row's `wall_s`); a second run with `--hyp <dir>` prints the same `wer_%`, `sub`, `del`, `ins`, `ref_words` (plan AC 18)
Proof: `cargo test -p fala-cli --test bench -- --ignored --exact out_then_hyp_round_trips`

**C20** - `--hyp <dir>` alone scores `<dir>/<stem>.txt` per cut and the legend reads `engine=hyp model=<basename of dir>` (plan AC 19)
Proof: `cargo test -p fala-cli --test bench -- --exact hyp_scores_and_names_dir`

**C21** - A missing `<hyp>/<stem>.wall_s` prints `-` in that row's `wall_s` and `rtf`, and `-` in the `total` row's `rtf` (plan AC 20)
Proof: `cargo test -p fala-cli --test bench -- --exact missing_wall_s_prints_dash`

**C22** - `--hyp` combined with each of `--engine`, `--model`, `--threads`, `--device`, `--chunk-s` exits 2; table-driven over the 5 flags (plan AC 21)
Proof: `cargo test -p fala-cli --test bench -- --exact hyp_rejects_engine_flags`

**C23** - `--hyp <dir>` missing `<stem>.txt` for a cut exits 2 naming the stem (plan AC 22)
Proof: `cargo test -p fala-cli --test bench -- --exact hyp_missing_stem_exits_2`

### S6 - saída e diagnóstico · 2 files · 15 KB · ~4k

**C24** - stdout is exactly: line 1 `engine=hyp model=<v> threads=- device=- load_s=- tag=<v|->`, line 2 the header `| cut | audio_s | wall_s | rtf | wer_% | sub | del | ins | ref_words |`, line 3 the separator, then one row per cut and the `total` row - nothing else; `--tag x` prints `tag=x`, no `--tag` prints `tag=-` (plan AC 23)
Proof: `cargo test -p fala-cli --test bench -- --exact stdout_is_legend_then_table`

**C25** - With `RUST_LOG=info`, a word that exists only in the reference and hypothesis appears in neither stdout nor stderr (plan AC 24)
Proof: `cargo test -p fala-cli --test bench -- --exact transcript_text_never_printed_above_debug`

**C26** - A cut's row reaches the stdout pipe while the process is still running the next cut (plan AC 25)
Proof: `cargo test -p fala-cli --test bench -- --ignored --exact rows_stream_while_running`

### S7 - relatório (sem cortes) · 2 files · 15 KB · ~4k

**C27** - `docs/spikes/01-asr-pt-br.md` has `## Objetivo`, `## Como reproduzir`, `## Evidência medida`, and no heading containing "Recomenda" (plan AC 26)
Proof: `test "$(grep -cE '^## (Objetivo|Como reproduzir|Evidência medida)$' docs/spikes/01-asr-pt-br.md)" -eq 3 && ! grep -iE '^#+ .*recomenda' docs/spikes/01-asr-pt-br.md`

**C28** - `## Como reproduzir` lists the `ffmpeg` cut command, the origin of each model (Parakeet dir, turbo URL + sha256 `1fc70f77…`), a `fala-cli bench` invocation per engine, the faster-whisper script invocation, and the `fala` commit (plan AC 27)
Proof: `s=$(awk '/^## Como reproduzir/,/^## Evid/' docs/spikes/01-asr-pt-br.md); for p in 'ffmpeg -ss' 'parakeet-tdt-0.6b-v3-int8' '1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69' 'engine parakeet-onnx' 'engine gguf' 'bench_faster_whisper.py' 'commit'; do grep -qF -- "$p" <<<"$s" || exit 1; done`

**C29** - `## Evidência medida` records, for each rabbit hole (Vulkan na iGPU; Nemotron GGUF / NeMo-Speech.cpp), whether it fit in the hour or was abandoned, with the `device=` seen (plan AC 28, second half)
Proof: `awk '/^## Evidência medida/,0' docs/spikes/01-asr-pt-br.md | grep -cE '^\| (Vulkan na iGPU|Nemotron 3\.5) \|' | grep -qx 2`

**C30** - A `## Windows` section carries `TODO(windows)` and ready invocations for whisper turbo CUDA, Voxtral and Parakeet on the Core 7 (plan AC 30)
Proof: `s=$(awk '/^## Windows/,0' docs/spikes/01-asr-pt-br.md); for p in 'TODO(windows)' 'features cuda' 'voxtral' 'parakeet-onnx'; do grep -qiF -- "$p" <<<"$s" || exit 1; done`

**C31** - `bench_faster_whisper.py` in `~/projects/fala-research/benchmarks/scripts/` writes `<stem>.txt` and `<stem>.wall_s` per cut and `fala-cli bench --hyp` scores its output (door 2; plan Out of scope row "script do faster-whisper")
Proof: `S=~/.cache/fala-bench/smoke; uv run ~/projects/fala-research/benchmarks/scripts/bench_faster_whisper.py --cuts $S/cuts --out $S/fw && cargo run -q -p fala-cli -- bench --cuts $S/cuts --refs $S/refs --hyp $S/fw` exit 0 (the smoke corpus is one 15 s slice of an interview, `ffmpeg`'d to 16 kHz mono, with a placeholder reference)

### S8 - rodadas (blocked by plan open question 1 - closes in the rounds `docs` PR) · 2 files · 10 KB · ~3k

**C32** - `## Evidência medida` compares measured Parakeet v3 int8 WER and RTF on the real cuts against ADR-0003's figures (WER pt ~6 %; 10-20x real time) (plan AC 28, first half)
Proof: `awk '/^## Evidência medida/,0' docs/spikes/01-asr-pt-br.md | grep -E 'ADR-0003' | grep -E '[0-9]+\.[0-9]+ ?%' `

**C33** - `benchmarks/README.md` § Rodadas has a table per round with columns data, máquina, engine, modelo, versão, threads, device, corte, WER, RTF and ≥ 1 row per engine measured (plan AC 29)
Proof: `awk '/^## Rodadas/,0' ~/projects/fala-research/benchmarks/README.md | grep -cE '^\| 2026-' | awk '$1>=1{ok=1} END{exit !ok}'`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| `fala-cli bench` exit statuses (3) | 0 C5 · 1 C12 · 1 C13 · 2 C6 · 2 C9 · 2 C15 · 2 C16 · 2 C17 · 2 C22 · 2 C23 | - |
| engine selectors (3) | `parakeet-onnx` C7 · `gguf` C10 · `hyp` C20 | - |
| `--device` values on gguf (2) | `cpu` C10 · `gpu` C11, C12 | - |
| parakeet-rejected flags (2) | `--threads` C9 · `--device gpu` C9 | - |
| `--hyp`-rejected flags (5) | C22, table-driven over all 5 | - |
| WAV spec fields (3) | rate C17 · channels C17 · bits/format C17 | - |
| edit kinds (3) | sub C2 · del C2 · ins C2, C4 | - |
| door 3 normalization steps (4) | NFC C1 · lowercase C1, C3 · non-alnum→space C1, C4 · split C1 | - |
| door 2 files (2) | `<stem>.txt` C19, C20 · `<stem>.wall_s` C19, C21 | - |
| door 1 build features (3) | default CPU C12, C34 · `vulkan` C11, C34 · `cuda` C34 (declared; runtime run is the Windows invocation in C30) | - |
| legend fields (6) | engine C20, C24 · model C20 · threads C10, C24 · device C10, C11, C24 · load_s C8, C24 · tag C24 | - |
| rabbit holes (2) | Vulkan iGPU C29 · Nemotron C29 | - |

- `cuda` cannot build here (no NVIDIA toolchain): C34 proves the feature's literal shape; running
  it is the Windows invocation C30 leaves ready. Named, not hidden.
- Claims naming an exit status or stdout shape: C5, C6, C9, C12-C17, C19-C26 - each proof runs the
  built binary and asserts the exit code and stdout it returns.
- No other check claims more than the single case its proof exercises.

## Swept

- validation: C6, C9, C15, C16, C17, C22, C23
- failure modes: C12, C13
- idempotency: n/a - a bench run reads files and overwrites `--out` files with the same content shape; rerunning is the intended use
- authorization: n/a - local CLI over local files, no user or permission model
- concurrency: n/a - one process, cuts processed sequentially; no shared state
- data lifecycle: n/a - nothing persisted besides `--out` files the user points at
- dependency failure: C12, C13 (missing GPU backend, engine error mid-run); model load failure exits 1 via the same path
- state transitions: n/a - no stateful entity
- observability: C24, C25, C26

## Handoff

- S1-S7 touch `apps/cli` (new `src/bench/{mod,wer,corpus,engine}.rs`, `src/main.rs`, `Cargo.toml`, `tests/bench.rs`) ≈ 60 KB written + ≈ 90 KB read (plan, the two runtime APIs, desktop `transcription.rs` excerpts) ≈ 150 KB / 4 ≈ 38k; S8 ≈ 3k; total ≈ 41k, under the 150k budget - one builder
- Mechanism: one builder (fits)
- **Settled mid-build:** C8 originally used a speech cut ≤ 20 s; on 15 s, Parakeet's warm load
  (~2 s) ≈ its transcription, and the proof failed 5 of 9 verifier runs although the code times
  them apart. Augusto approved on 2026-09-27 tightening the cut to 2 s (wall ≈ 0.2 s vs load
  ≈ 1.5-2.5 s); the claim is unchanged
- S8 is deferred by Augusto's decision of 2026-09-27: this PR's Verifier verifies C1-C31 and lists C32-C33 as BLOCKED by plan open question 1; the rounds `docs` PR re-verifies C32-C33
