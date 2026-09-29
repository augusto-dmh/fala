# bench-language - checks

Profile: light
Plan: none - checks-only change (one-sentence diff); renegotiates `cli-bench` AC 9, approved by Augusto on 2026-09-27

## Intent

`fala-cli bench --engine gguf` sends the fixed language `pt` to every model (`cli-bench` AC 9).
Models that advertise BCP-47 locales reject it: the Nemotron 3.5 ASR Streaming GGUF lists `pt-BR`
and `pt-PT` and fails every cut with `unsupported language` (`docs/spikes/01-asr-pt-br.md`,
rabbit hole Nemotron), so phase 0 cannot measure it. When this ships, the gguf engine asks for
`pt-BR` by default (`--language` overrides it) and resolves it against the languages the model
advertises: an exact match wins, otherwise the bare prefix (`pt`, what whisper lists), and a model
that advertises neither exits 2 listing what it does support. Whisper keeps receiving `pt`, so
earlier measurements stay comparable; Nemotron becomes measurable. The resolved code goes to
stderr; stdout (legend and table) does not change.

Decided here, not in a plan: resolution against `Model::capabilities().languages` plus a
`--language` override, rather than only a flag (would make every Nemotron/Voxtral run depend on
the operator remembering the locale) or only auto-resolution (no way to measure `pt-PT` or `en`
on purpose). A model with an empty language list gets the bare prefix, which is exactly the old
behaviour.

7 checks in 3 slices · 0 one-way doors · 0 open

## Checks

Unit proofs: `cargo test -p fala-cli --bin fala-cli <path>`. Boundary proofs spawn the built binary
from `apps/cli/tests/bench.rs`. Model-backed proofs are `#[ignore]`d and fail when their variable is
missing: `FALA_TEST_GGUF` (whisper large-v3-turbo), `FALA_TEST_NEMOTRON_GGUF`
(`nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf`, sha256 in `docs/spikes/01-asr-pt-br.md`),
`FALA_TEST_SPEECH_WAV`.

### S1 - resolução · 2 files · 20 KB · ~5k

**C1** - Resolving a requested code against a model's advertised list follows one table, asserted case by case: (`pt-BR`, [`pt-BR`, `pt-PT`]) → `pt-BR`; (`pt-PT`, [`pt-BR`, `pt-PT`]) → `pt-PT`; (`pt-BR`, [`en`, `pt`]) → `pt`; (`pt`, [`pt-BR`, `pt-PT`]) → `pt-BR` (first advertised `pt-*`, in the model's order); (`pt-BR`, []) → `pt`; (`pt-BR`, [`en`, `es`]) → error naming `pt-BR` and listing `en`, `es`; matching is case-insensitive on the whole code (`PT-br` → `pt-BR`, returned in the model's spelling)
Proof: `cargo test -p fala-cli --bin fala-cli bench::engine::tests::language_resolution_table`

**C2** - Without `--language`, `--engine gguf` on the Nemotron GGUF exits 0, prints one row per cut, and stderr carries `idioma: pt-BR`
Proof: `cargo test -p fala-cli --test bench -- --ignored --exact nemotron_runs_with_pt_br`

**C3** - Without `--language`, `--engine gguf` on whisper large-v3-turbo exits 0 and stderr carries `idioma: pt` (the old behaviour, so earlier rounds stay comparable)
Proof: `cargo test -p fala-cli --test bench -- --ignored --exact whisper_keeps_bare_pt`

**C4** - `--language en` on whisper exits 0 with stderr `idioma: en`; `--language xx-YY` on whisper exits 2, stderr naming `xx-YY` and listing the model's languages, and stdout is empty (no row transcribed)
Proof: `cargo test -p fala-cli --test bench -- --ignored --exact language_override_and_unsupported`

### S2 - interface · 2 files · 15 KB · ~4k

**C5** - `--language` with `--engine parakeet-onnx` exits 2 saying the engine does not expose the option, and `--language` with `--hyp` exits 2 (joins the existing conflict list)
Proof: `cargo test -p fala-cli --test bench -- --exact language_rejected_outside_gguf`

**C6** - stdout is unchanged by this change: the existing `stdout_is_legend_then_table` still passes unmodified and C2's stdout has exactly legend + header + separator + 1 row + `total`
Proof: `cargo test -p fala-cli --test bench -- --exact stdout_is_legend_then_table`
Proof: `cargo test -p fala-cli --test bench -- --ignored --exact nemotron_runs_with_pt_br`

### S3 - relatório · 1 file · 10 KB · ~3k

**C7** - `docs/spikes/01-asr-pt-br.md`: the Nemotron rabbit-hole row no longer says the measurement is blocked by the language; it states the resolution (`pt-BR` from the model's list) and the `fala-cli bench` Nemotron invocation in `## Como reproduzir` carries no `--language` flag; the Windows Voxtral note about refusing `pt` is replaced by the resolution rule
Proof: `f=docs/spikes/01-asr-pt-br.md; ! grep -qF 'devolvido ao planejamento' $f && grep -qF 'idioma: pt-BR' $f && ! grep -qF 'se recusar "pt" como o Nemotron' $f`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| resolution outcomes (6) | exact regional C1, C2 · exact other region C1 · regional → bare prefix C1, C3 · bare → first regional C1 · empty list → bare C1 · no match → error C1, C4 | - |
| `fala-cli bench` exit statuses touched (2) | 0 C2, C3, C4 · 2 C4, C5 | - |
| engines × `--language` (3) | `gguf` C2, C3, C4 · `parakeet-onnx` C5 · `hyp` C5 | - |
| models on this machine (2) | Nemotron 3.5 C2 · whisper turbo C3, C4 | - |

- Voxtral (Windows only) is not proven here; the rule is generic and the Windows session runs it.
- Claims naming an exit status or output: C2-C6, each proven by running the built binary.

## Swept

- validation: C4, C5
- failure modes: C4 (unsupported language fails before any cut, not per cut)
- idempotency: n/a - no state
- authorization: n/a - local CLI
- concurrency: n/a - single process
- data lifecycle: n/a - nothing persisted
- dependency failure: C4 (a model whose list lacks the language)
- state transitions: n/a - no stateful entity
- observability: C2, C3, C4 (the resolved code on stderr), C6 (stdout unchanged)

## Handoff

- S1-S3 touch `apps/cli/src/bench/{mod,engine}.rs`, `apps/cli/tests/bench.rs` and the report ≈ 45 KB
  read + ≈ 5 KB written ≈ 12k, under the 150k budget - one builder
- Mechanism: one builder (fits)
