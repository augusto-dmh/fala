# bench-language verification

**Verdict**: PASS
**Profile**: light
**Diff range**: 930464c..3e17a2d (`main..HEAD`, branch `feat/bench-language`)
**Round**: 2 - scoped
**Verifier**: fresh sub-agent (not the author) - independent sub-agent (author != verifier)

## Round 2 (scoped) - verified at `3e17a2d`

The feature commit `1709c13` was amended to `3e17a2d`; `4767543` (checks only) is unchanged.
`git diff 1709c13 3e17a2d --stat` shows one file, `apps/cli/src/bench/engine.rs`, +5/-0, all
five lines inside `#[cfg(test)] mod tests`: a new assertion in `language_resolution_table` at
`:233-237`. `git diff 1709c13 3e17a2d --quiet -- apps/cli/src/bench/mod.rs apps/cli/tests/bench.rs
docs .specs` exits 0, so no production code, boundary test, report or check moved.
`git status --porcelain` showed only the untracked report before and after.

Scope for this round:
- **Re-judged in full:** C1, the only check whose test the amendment touched. Its citations were
  refreshed (every assertion after `:232` shifted by 5 lines). Finding F1 is re-judged below.
- **Proofs:** every proof for C1-C7 was re-run at `3e17a2d` (see the round 2 table below).
  Lint and format were re-run too.
- **Carried from round 1 (`1709c13`):** the judgments for C2-C7, the level and sampling
  section (except the C1 bullet), `Swept existing`, and findings F2, F3 and the note. The files
  they cite (`apps/cli/tests/bench.rs`, `apps/cli/src/bench/mod.rs`, `docs/spikes/01-asr-pt-br.md`
  and the non-test part of `engine.rs`, `:1-215`) are byte-identical between the two commits, so
  those line numbers still hold.

### Round 2 proofs run (at `3e17a2d`)

| Invocation | Result |
| --- | --- |
| `cargo test -p fala-cli --bin fala-cli bench::engine::tests::language_resolution_table` | `test bench::engine::tests::language_resolution_table ... ok`; 1 passed, 0 failed, 13 filtered out; exit 0 |
| model-backed batch (same three env vars as round 1) `cargo test -p fala-cli --test bench -- --ignored nemotron_runs_with_pt_br whisper_keeps_bare_pt language_override_and_unsupported` | `test nemotron_runs_with_pt_br ... ok`, `test whisper_keeps_bare_pt ... ok`, `test language_override_and_unsupported ... ok`; 3 passed, 0 failed; 123.33 s |
| `cargo test -p fala-cli --test bench -- --exact language_rejected_outside_gguf stdout_is_legend_then_table` | both `... ok`; 2 passed, 0 failed |
| C7 shell proof, as written in `checks.md` | exit 0 |
| `cargo clippy -p fala-cli --all-targets -- -D warnings` and `cargo fmt --all -- --check` | both exit 0 |

### C1 at `3e17a2d`

`apps/cli/src/bench/engine.rs:226` `fn language_resolution_table`, listed `... ok`. Assertions:
`:229` `assert_eq!(resolve_language("pt-BR", &regional).unwrap(), "pt-BR")`; `:230` `("pt-PT",
&regional) == "pt-PT"`; `:231` `("pt-BR", &bare) == "pt"` with `bare = ["en", "pt"]`; `:232`
`("pt", &regional) == "pt-BR"`; **new** `:234-237`
`assert_eq!(resolve_language("pt", &list(&["pt-PT", "pt-BR"])).unwrap(), "pt-PT")`; `:238`
`("pt-BR", &[]) == "pt"`; `:239` `("PT-br", &regional) == "pt-BR"` (model's spelling); `:243`
`assert!(err.contains("pt-BR"))` and `:244` `assert!(err.contains("en") && err.contains("es"))` on
`("pt-BR", ["en", "es"])`. The new row reverses the list order and expects `pt-PT`, so an
implementation that sorted the list or hard-coded `pt-BR` would now fail `:234-237` while
`:232` still holds. The "first advertised `pt-*`, in the model's order" clause is discriminated.
**F1 is closed.** The function under test, `resolve_language` at `:127-153`, is unchanged.

The Checks table below carries the round 1 C1 row with its citations updated to `3e17a2d`; every
other row is carried from round 1 with its proof re-run.

## Round 1 (full) - verified at `1709c13`

Verified at `1709c13` on 2026-09-28. `git status --porcelain` was empty before and after the run.
The range has two commits: `4767543` adds only `checks.md`; `1709c13` touches
`apps/cli/src/bench/engine.rs` (+89/-3), `apps/cli/src/bench/mod.rs` (+28/-5),
`apps/cli/tests/bench.rs` (+104/-1) and `docs/spikes/01-asr-pt-br.md` (+15/-3). No plan.md
exists: this is a checks-only change with an `## Intent` paragraph, so there are no binding
sources and step 1 does not run (profile `light`).

Under `light` the following ran: every proof at `HEAD`, each named test shown to exist and to
have run individually, one located assertion per check, the level and sampling judgment, and the
`Swept existing` re-read. No fault injection and no `Coverage` recompute were done, as the profile
prescribes.

## Proofs run

All of these ran at `1709c13`, from the repo root.

| Invocation | Result |
| --- | --- |
| `cargo test -p fala-cli --bin fala-cli bench::engine::tests::language_resolution_table` | `test bench::engine::tests::language_resolution_table ... ok`; 1 passed, 0 failed, 13 filtered out; exit 0 |
| `FALA_TEST_NEMOTRON_GGUF=~/.cache/fala-bench/models/nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf FALA_TEST_GGUF=~/.cache/fala-bench/models/ggml-large-v3-turbo.bin FALA_TEST_SPEECH_WAV=~/.cache/fala-bench/smoke/cuts/smoke.wav cargo test -p fala-cli --test bench -- --ignored nemotron_runs_with_pt_br whisper_keeps_bare_pt language_override_and_unsupported` | `test nemotron_runs_with_pt_br ... ok`, `test whisper_keeps_bare_pt ... ok`, `test language_override_and_unsupported ... ok`; 3 passed, 0 failed, 23 filtered out; 165.73 s; exit 0 |
| `cargo test -p fala-cli --test bench -- --exact language_rejected_outside_gguf stdout_is_legend_then_table` | `test language_rejected_outside_gguf ... ok`, `test stdout_is_legend_then_table ... ok`; 2 passed, 0 failed, 24 filtered out; exit 0 |
| C7: `f=docs/spikes/01-asr-pt-br.md; ! grep -qF 'devolvido ao planejamento' $f && grep -qF 'idioma: pt-BR' $f && ! grep -qF 'se recusar "pt" como o Nemotron' $f` | exit 0 |
| Regression sweep (not a check): `cargo test -p fala-cli` | unit 14 passed, 0 failed; `tests/bench.rs` 16 passed, 0 failed, 10 ignored; `tests/record.rs` 7 passed, 0 failed, 6 ignored. Every existing `cli-bench` boundary test, including `missing_model_exits_1` and `gpu_without_backend_exits_1`, listed `... ok` after the `Failure` `From` impl and the `load_gguf` signature change. |
| Lint (not a check): `cargo clippy -p fala-cli --all-targets -- -D warnings` | exit 0, no warnings |
| Format (not a check): `cargo fmt --all -- --check` | exit 0 |

Every named test exists at the line cited below (`rg -n 'fn <name>'`) and appeared individually in
the runner output as `... ok`. The three model-backed tests are new in this diff; the model files
named by the three environment variables exist on this machine (1.6 GB, 751 MB, 480 KB).

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | resolution follows the 7-row table, case-insensitive, model's spelling returned | unit run: `language_resolution_table ... ok` (`apps/cli/src/bench/engine.rs:226`) | `apps/cli/src/bench/engine.rs:229` `assert_eq!(resolve_language("pt-BR", &regional).unwrap(), "pt-BR")`; `:230` `("pt-PT", &regional) == "pt-PT"`; `:231` `("pt-BR", &bare) == "pt"` with `bare = ["en", "pt"]`; `:232` `("pt", &regional) == "pt-BR"`; `:234-237` `("pt", ["pt-PT", "pt-BR"]) == "pt-PT"` (added at `3e17a2d`: the reversed order); `:238` `("pt-BR", &[]) == "pt"`; `:239` `("PT-br", &regional) == "pt-BR"` (model's spelling, not the input's); `:243` `assert!(err.contains("pt-BR"))` and `:244` `assert!(err.contains("en") && err.contains("es"))` on `("pt-BR", ["en", "es"])`. All 7 rows of the check's table have a dedicated assertion with the concrete expected value, and the "in the model's order" clause is pinned by the reversed-list row. | PASS (re-judged at 3e17a2d; F1 closed) |
| C2 | Nemotron without `--language`: exit 0, one row per cut, stderr `idioma: pt-BR` | ignored batch: `nemotron_runs_with_pt_br ... ok` (`apps/cli/tests/bench.rs:660`) | `apps/cli/tests/bench.rs:665` `assert_eq!(o.status.code(), Some(0), …)`; `:666` `assert_eq!(idioma(&o), "idioma: pt-BR")`, where `idioma()` at `:649-656` returns the stderr line from `idioma: ` to its end, so equality pins the whole code; `:669` `assert_eq!(lines.len(), 5)`; `:676` `lines[3].starts_with(…)` on the row-`a` prefix (pipe, space, `a`, space, pipe), the one cut; `:677` `lines[4].starts_with(…)` on the `total` prefix. The stderr line is emitted at `apps/cli/src/bench/engine.rs:113` `log::info!("idioma: {language}")` and the CLI's default filter is `info` (`apps/cli/src/main.rs:41` `default_filter_or("info")`), so it appears without `RUST_LOG`. | PASS |
| C3 | whisper without `--language`: exit 0, stderr `idioma: pt` (not `pt-BR`) | ignored batch: `whisper_keeps_bare_pt ... ok` (`apps/cli/tests/bench.rs:682`) | `apps/cli/tests/bench.rs:687` `assert_eq!(o.status.code(), Some(0), …)`; `:688` `assert_eq!(idioma(&o), "idioma: pt")`. Because `idioma()` returns the rest of the line, `"idioma: pt-BR"` would fail this equality: the assertion does distinguish `pt` from `pt-BR`. | PASS |
| C4 | `--language en` exit 0 with `idioma: en`; `--language xx-YY` exit 2, stderr names `xx-YY` and lists the model's languages, stdout empty | ignored batch: `language_override_and_unsupported ... ok` (`apps/cli/tests/bench.rs:693`) | `apps/cli/tests/bench.rs:699` `Some(0)`; `:700` `assert_eq!(idioma(&o), "idioma: en")`; `:703` `assert_eq!(o.status.code(), Some(2), …)`; `:705` `assert!(err.contains("xx-YY"))`; `:711-714` `assert!(listed.contains("en") && listed.contains("pt"), "model languages not listed: …")` on the stderr line that carries `xx-YY`; `:715` `assert_eq!(stdout(&o), "", "a row was transcribed")`. Exit 2 comes from `apps/cli/src/bench/engine.rs:112` the `map_err` closure returning `Failure { code: 2, error }`. See finding F2 on the list assertion. | PASS |
| C5 | `--language` with `parakeet-onnx` exits 2 saying the engine does not expose it; `--language` with `--hyp` exits 2 | `language_rejected_outside_gguf ... ok` (`apps/cli/tests/bench.rs:719`) | `apps/cli/tests/bench.rs:735` `assert_eq!(o.status.code(), Some(2), …)`; `:736` `stderr(&o).contains("--language")`; `:737-741` `stderr(&o).contains("não expõe essa opção")`; `:744` `assert_eq!(o.status.code(), Some(2), …)` for the `--hyp` invocation at `:743`. The parakeet run passes `--model /nonexistent/model`, and the guard at `apps/cli/src/bench/mod.rs:107-110` sits before corpus and model loading, so the message assertion at `:736` is what proves the exit came from the language guard rather than from the missing model (which would be exit 1). The conflict list is `apps/cli/src/bench/mod.rs:36` `conflicts_with_all = [… "language"]`. | PASS |
| C6 | stdout unchanged: `stdout_is_legend_then_table` passes unmodified; C2's stdout is legend + header + separator + 1 row + total | `stdout_is_legend_then_table ... ok` (`apps/cli/tests/bench.rs:567`) plus `nemotron_runs_with_pt_br ... ok` | `apps/cli/tests/bench.rs:578` `assert_eq!(lines.len(), 6)`; `:579` `assert_eq!(lines[0], LEGEND_HYP)`; `:580` `assert_eq!(lines[1], HEADER)`. `git diff main..HEAD -- apps/cli/tests/bench.rs` removes exactly one line, the module doc comment at `:4`, so the test is unmodified. For C2's stdout: `:669` `lines.len() == 5`; `:670` `lines[0].starts_with("engine=gguf model=")`; `:671` `lines[1] == HEADER`; `:672-675` `lines[2]` equals the separator literal; `:676` row `a`; `:677` `total` (both `starts_with` on the pipe-delimited prefix). | PASS |
| C7 | the spike report's Nemotron row no longer says the measurement is blocked; it states the resolution; the Nemotron invocation has no `--language`; the Windows Voxtral note is replaced by the rule | shell proof exit 0 | `docs/spikes/01-asr-pt-br.md:113` Nemotron row reads `sim, ~10 min` with "Desde então o `bench` resolve o idioma contra a lista do modelo e registra `idioma: pt-BR` no stderr"; the string `devolvido ao planejamento` is gone. `:75-77` the Nemotron invocation is `$FALA bench … --engine gguf --model $M/nemotron-… --tag linux` with no `--language`; `rg -n -- '--language' docs/spikes/01-asr-pt-br.md` hits only `:87`, the prose rule. `:139-141` the Voxtral note now reads "O idioma é resolvido contra a lista do modelo (pt-BR, senão pt); registrar o `idioma:` do stderr e, se sair com 2, a lista anunciada"; `se recusar "pt" como o Nemotron` is gone. See finding F3 on what the proof itself pins. | PASS |

## Level and sampling

- **Level.** C1 is a unit proof of `resolve_language` alone. Every claim that names an exit status
  or an output stream (C2, C3, C4, C5, C6) is proven by spawning the built binary from
  `apps/cli/tests/bench.rs` through `fala()` at `:125-131`, so no exit or stream claim sits below
  its boundary. C7 is a text proof over the report. No level gap.
- **Sampling, C1** (re-judged at `3e17a2d`). The table row "(`pt`, [`pt-BR`, `pt-PT`]) → `pt-BR`
  (first advertised `pt-*`, in the model's order)" is now proven on both list orders: `:232` for
  `[pt-BR, pt-PT]` → `pt-BR` and `:234-237` for `[pt-PT, pt-BR]` → `pt-PT`. In round 1 only the
  first order was tested (F1); the amendment closed it.
- **Sampling, C2/C3/C4.** Each model-backed check runs one cut (`c.speech("a")`), which is what
  the checks ask for ("one row per cut" is asserted on 1 cut). Two models cover the two
  resolution outcomes that exist on this machine (exact regional on Nemotron, regional → bare on
  whisper). Voxtral is out of scope per `checks.md` `## Coverage`.
- **Sampling, C5.** One engine (`parakeet-onnx`) and one conflicting flag (`--hyp`); the check
  names exactly those two.
- **Stderr precondition.** The `idioma:` line is `log::info!`. The test helper sets
  `RUST_LOG=info` (`:128`), but the CLI's default filter is already `info`
  (`apps/cli/src/main.rs:41`), so the check holds for a plain invocation too.

## Coverage

Not recomputed: the profile is `light`. The author's join in `checks.md` `## Coverage` was read
only. Its rows map to proofs that did run and pass in this round.

## Swept existing

Re-read at `1709c13`. No `Swept` row in `checks.md` resolves to "existing": idempotency,
authorization, concurrency, data lifecycle and state transitions are `n/a` (policy the user
approved; nothing in the code for them to be wrong about). The rows that name checks were held
against the code:

- **validation: C4, C5.** The `--language` guard for parakeet is `apps/cli/src/bench/mod.rs:107`
  and the clap conflict is `:36`; unsupported language maps to code 2 at
  `apps/cli/src/bench/engine.rs:112`. Proven above.
- **failure modes: C4 "fails before any cut, not per cut".** Resolution happens inside
  `load_gguf` (`engine.rs:111-112`) before `Loaded` is returned, so no cut runs; `bench.rs:715`
  asserts stdout is empty.
- **dependency failure: C4.** Same path.
- **observability: C2, C3, C4, C6.** The resolved code is logged at `engine.rs:113`; stdout
  unchanged per C6.

## Findings

None of these fails a check. Every check has a green proof at `HEAD` and a located assertion that
pins the check-defined value. The verdict is PASS. Ranked:

- **F1 (C1, sampling gap in the test) - CLOSED at `3e17a2d`.** In round 1,
  `apps/cli/src/bench/engine.rs:232` proved `("pt", ["pt-BR", "pt-PT"]) → "pt-BR"` on one list
  order, so "first in the model's order" was not discriminated from "alphabetically first" or a
  hard-coded `pt-BR`. The amendment adds `:234-237`
  `assert_eq!(resolve_language("pt", &list(&["pt-PT", "pt-BR"])).unwrap(), "pt-PT")`, which a
  sorting or hard-coding implementation fails. The implementation at `engine.rs:139-146`
  iterates in list order.
- **F2 (C4, precision gap in the test) - carried from round 1, still open.** `apps/cli/tests/bench.rs:711-714` asserts
  `listed.contains("en") && listed.contains("pt")` on the stderr line that names `xx-YY`. That is a
  substring check on the whole line, not a check that the list after `anuncia:` contains those
  codes. On the actual message (`engine.rs:150-152`, `idioma \`xx-YY\` não consta na lista do
  modelo; ele anuncia: <list>`) the words before the list contain neither `en` nor `pt`, so the
  assertion does discriminate today, but it is coupled to the wording of the message.
- **F3 (C7, precision gap in the proof) - carried from round 1, still open.** The proof's three greps pin "the blocked wording is
  gone", "`idioma: pt-BR` appears somewhere" and "the Voxtral refusal wording is gone". They do
  not pin "the Nemotron invocation in `## Como reproduzir` carries no `--language`" nor that the
  Voxtral note was *replaced by the rule* rather than merely deleted. Both were confirmed by
  reading (`docs/spikes/01-asr-pt-br.md:75-77` and `:139-141`), not by the proof.
- **Note (not a gap).** `apps/cli/src/bench/mod.rs:76-83` adds a blanket
  `impl<E: Into<anyhow::Error>> From<E> for Failure` mapping every `?` to code 1. This widens the
  blast radius beyond the language feature: any future `?` on a `Result<_, Failure>` path silently
  becomes exit 1. The existing `missing_model_exits_1` and `gpu_without_backend_exits_1` still
  pass, so the current mapping is correct; no check in this feature covers the impl itself.

## Gate

Round 2 at `3e17a2d`: C1 1 passed; C2-C4 3 passed (123.33 s); C5-C6 2 passed; C7 exit 0; clippy
and fmt clean. 7 of 7 checks proven with located evidence; F1 closed, F2 and F3 remain as
recorded (neither fails a check).

Round 1 at `1709c13`:

- Checks: 7 of 7 proven with located evidence.
- Proofs: 4 named test functions plus `stdout_is_legend_then_table`, each listed `... ok`;
  1 shell proof exit 0.
- `cargo test -p fala-cli`: 37 passed (14 unit, 16 `bench`, 7 `record`), 0 failed, 16 ignored;
  the 3 ignored tests this feature adds passed in the model-backed run.
- `cargo clippy -p fala-cli --all-targets -- -D warnings`: clean. `cargo fmt --all -- --check`:
  clean.
