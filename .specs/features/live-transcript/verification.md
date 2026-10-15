# live-transcript verification

Windows 11 (Alienware), `CARGO_TARGET_DIR=C:\f\live-transcript`, 2026-10-15, stack #205 → #206 →
#207 on `origin/main` at `0c888a2`.

Verdict: **PASS** for C1-C5. M1-M3 are **Not tested** (they need a real call or a meeting with
sound playing; no recording was made tonight).

| Check | Proof | Result |
| --- | --- | --- |
| C1 | `cargo test -p fala-audio` (`the_tap_gets_what_the_wav_gets_with_the_dictation_zeroed`) | 61 passed |
| C2 | same run, `meeting::live` (6 tests, −50 dBFS call raised without clipping included) | ok |
| C3 | `cargo test -p fala --lib` (`meeting::preview`, 4 tests) | 573 passed, 1 ignored |
| C4 | `bun src/components/meeting/live.test.tsx` (5 checks) and the 23 meeting UI scripts | all exit 0 |
| C5 | `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace` (1182 passed, 0 failed, 55 ignored, `RUST_TEST_THREADS=2`); `bunx tsc --noEmit`; `bun run lint`; `bun run format:check`; `bun run check:translations`; `scripts/check-brand.sh` | all exit 0 |

## Settled mid-build

- The orchestrator moved the local fallback to the `retry` panel: the preview never becomes the
  transcript of record (ADR-0019 says so).
- The loopback of a real call measured −54 dBFS: the segmenter got an automatic gain (≤ +40 dB,
  peak ≤ −6 dBFS) before the VAD and the model; Fala's start sound is ignored in the first 1.5 s of
  the system channel.
- Review fixes: frames reprocessed after a VAD error, a silent stretch after a cap cut, stale VAD
  state after silence, the model not reloaded after an unload mid-meeting, the streaming worker not
  waiting for the preview, a racy mistake check, loading under the manager lock, previews of
  deleted sessions kept, a snapshot/event race and a missed auto-scroll in the page.
- Two `fala-postproc` timing tests (1.5 s fake reply vs the 2 s deadline) fail under tonight's
  machine load with the default test threads and pass alone; not touched here.

**Not tested** (M1-M3): lines during a call, dictation in the middle of a meeting, the swap to the
transcript of record. GPU/CPU use during a long call was not measured.
