# upstream-catchup PR 4 (native capture format) verification

**Verdict**: PASS - C1 to C5 proven at HEAD with located evidence; C6 (manual, Windows) not run and listed separately below. The PR body over-claims; see Findings 1 and 2
**Profile**: light
**Diff range**: eb0482c..ba0c5cb
**Round**: 1 - full
**Verifier**: independent sub-agent (author != verifier)

Checks file: `.specs/features/upstream-catchup/checks-pr4-capture-formats.md`. There is no plan.md; `## Intent` holds the delegated decisions. Every proof ran in the detached worktree `wt-pr4` at `ba0c5cb8da653d497e3614083b160f0d75e3d782`, with a clean `git status --porcelain`. Cargo compiled `fala` from that worktree's sources (log line `Compiling fala v0.1.0 (.../wt-pr4/apps/desktop)`), so no stale artifact from another worktree was reused. Context read: cjpais/Handy#2141 (body and 3 comments, via `gh api`), upstream PR cjpais/Handy#2144 (diff and comments), cpal 0.16.0 sources (`host/wasapi/device.rs`, `host/alsa/mod.rs`, `lib.rs`, `samples_formats.rs`) and dasp_sample 0.11.0 `conv.rs`.

## Binding sources

None. The checks file says `Plan: none`, and profile light does not run step 1.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | `downmix` converts each native format to f32 (f32, i16, i32, u8, f64 values listed, tolerance 1e-3) | `cargo test -p fala-audio` exit 0; `mic::tests::downmix_converts_every_native_format_to_f32 ... ok` | `crates/audio/src/mic.rs:181` - `(actual - expected).abs() < 1e-3` · `:188` - `close(downmix(&[0.5f32, -0.25]), 0.125)` · `:189-191` - i16 `MAX,MAX` -> `1.0`, `MIN` -> `-1.0`, `16_384, 0` -> `0.25` · `:192-193` - i32 `MIN,MIN` -> `-1.0`, `1_073_741_824` -> `0.5` · `:194-195` - u8 `128,128` -> `0.0`, `255` -> `127.0 / 128.0` · `:196` - f64 `0.5,0.5` -> `0.5`. All nine values match the claim | PASS |
| C2 | an integer signal is not turned into silence: i16 `[8192, 8192]` gives magnitude > 0.2 | same invocation; `mic::tests::downmix_keeps_a_signal_that_is_not_silence ... ok` | `crates/audio/src/mic.rs:202-203` - `let frame = [8_192i16, 8_192]; assert!(downmix(&frame).abs() > 0.2);` | PASS |
| C3 | `Mic::open` no longer looks for f32: it uses `default_input_config()` and picks `build::<T>` from the default `sample_format()`, with no `supported_input_configs` and no "não oferece f32" message | structural grep from the checks, exit 0 (`C3=0`) | `crates/audio/src/mic.rs:52` - `.default_input_config()` · `:54` - `let format = default.sample_format();` · `:62-72` - `match format { SampleFormat::I8 => build::<i8>(...)`, ..., `SampleFormat::F32 => build::<f32>`, `SampleFormat::F64 => build::<f64>` · `:73-75` - `other => Err(AudioError::UnsupportedConfig(...))`. `grep -n supported_input_configs crates/audio/src/mic.rs` finds nothing | PASS |
| C4 | desktop `builds_sample_format` is true for U8, I8, I16, I32, F32 and false for U16, U32, I64, F64; `get_preferred_config` returns `default_config` when that holds, before the scored search | `cargo test -p fala --lib -- audio_toolkit::audio::recorder::tests` exit 0 (`CARGO_BUILD_JOBS=1`); `device_default_format_is_kept_when_the_builder_handles_it ... ok`; structural awk from the checks, exit 0 (`C4b=0`) | `apps/desktop/src/audio_toolkit/audio/recorder/tests.rs:433` - `assert!(builds_sample_format(format), "{format:?}")` over `[U8, I8, I16, I32, F32]` · `:442` - `assert!(!builds_sample_format(format), "{format:?}")` over `[U16, U32, I64, F64]` · `apps/desktop/src/audio_toolkit/audio/recorder.rs:569-570` - `if builds_sample_format(default_config.sample_format()) { return Ok(default_config); }`, before `for config_range in supported_configs` at `:584`. The `matches!` at `:623-631` lists exactly the 5 arms that build a stream at `:250`, `:258`, `:266`, `:274`, `:282` (U8, I8, I16, I32, F32); any other format reaches `Unsupported sample format` at `:290-292` | PASS |
| C5 | the existing `fala-audio` tests and the desktop recorder tests stay green with no assertion edited | `cargo test -p fala-audio` exit 0: 25 + 1 + 1 + 1 passed, 1 ignored (`speech_gives_an_utterance`, needs `FALA_TEST_SPEECH_WAV`, ignored before this diff too); desktop invocation exit 0: 21 passed, 329 filtered out | Grepping `git diff eb0482c..ba0c5cb -- apps crates` for removed lines containing `assert` finds none. The only removed test line is the `use super::{...}` import at `apps/desktop/src/audio_toolkit/audio/recorder/tests.rs:1-5`, which now also imports `builds_sample_format`. `crates/audio/src/mic.rs` had no test module at `eb0482c` (`mod tests` occurs 0 times in `git show eb0482c:crates/audio/src/mic.rs`) | PASS |

## Manual proofs not run

| Check | Claim | Why not run | Status |
| --- | --- | --- | --- |
| C6 | On a laptop with a Realtek mic and "Voice clarity" on, the log `Format:` equals the device mix format (not a forced `F32`) and a 5 s dictation produces text; `fala-cli dictate` logs the same format in `microfone:`. Expected 10/10 with text (5 with effects on, 5 off) | `TODO(windows)` manual proof; this run is on Linux (Ubuntu 25.04, PipeWire) with no Realtek WASAPI endpoint | not run (manual, Windows). It is the only proof of the symptom. Upstream evidence suggests it may fail; see Finding 1 |

## Coverage

Under `light`, the Coverage join is not recomputed. I spot-checked the author's table against cpal 0.16.0 anyway:

| Set (size) | Checked against | Member -> proof | Note |
| --- | --- | --- | --- |
| mic capture paths (2) | `mic.rs:52-75`, `recorder.rs:569` | Mic C1 C2 C3, desktop C4 | holds |
| formats the desktop opens as they come (5) | stream match `recorder.rs:249-293` | U8 I8 I16 I32 F32, all asserted at `tests.rs:433` | holds; exact match with the build arms |
| formats that fall back to the scored search (4 as authored) | `cpal::SampleFormat` in cpal 0.16.0 (`samples_formats.rs:25-65`, `#[non_exhaustive]`) | U16 U32 I64 F64 asserted at `tests.rs:442` | cpal 0.16 has a 10th variant, `I24`, that no test names. `builds_sample_format(I24)` is false by construction. No host Fala uses produces it (WASAPI maps only U8/I16/I32/I64/F32; ALSA only I8/U8/I16/U16/I32/U32/F32/F64; I24 comes only from ASIO, which is not enabled). See Finding 5 |
| sample families converted in `Mic` (4) | dasp_sample 0.11 `conv.rs` | f32, signed int, unsigned int, f64 all in C1 | u16, u32, u64, i8 and i64 are not asserted. dasp converts them with the same formula (signed: `s / 2^(bits-1)`; unsigned: subtract the midpoint, then the same), so the four families hold |

Level gap (light): C3 and the second proof of C4 are structural. No test opens a stream or runs `get_preferred_config`, because opening a device does not work in CI. C1 and C2 test the conversion helper only. The symptom, WASAPI handing zeros to an f32 client, sits entirely in C6.

Swept rows: none resolves to "existing". The validation row (C3, "a format outside the list becomes `UnsupportedConfig`") and the observability row (C3, "the `microfone:` log names the format") are not asserted by C3's grep. The code is there: `UnsupportedConfig` at `crates/audio/src/mic.rs:73-75`, and `log::info!("microfone: {name}, {rate} Hz, {channels} canal(is), {format:?}")` at `:80`. These are precision gaps in the Swept rows, not failed checks.

## Faults injected

None - profile light (step 4 runs only under standard and ui).

## Judgment questions from the brief

- **f32 range per format.** Correct. Both paths go through `f32::from_sample` (dasp_sample 0.11, re-exported by cpal 0.16). i8/i16/i32/i64 divide by `2^(bits-1)`, so the range is [-1, 1). u8/u16/u32/u64 subtract the midpoint first (`u8`: `(s - 128) / 128`, `u16`: `(s - 32768) / 32768`), so 128 or 32768 maps to 0.0. f32 is unchanged and f64 is cast. I24 would be handled correctly by dasp, but no path builds it (Finding 5).
- **Downmix unchanged otherwise.** Yes. Before: `frame.iter().sum::<f32>() / channels as f32`. After: `sum(f32::from_sample) / frame.len().max(1)`. `frame` comes from `data.chunks_exact(channels)` (`mic.rs:159`), so `frame.len() == channels`, and `f32::from_sample(f32)` is the identity. The desktop conversion (`build_stream::<T>` with `f32: FromSample<T>`, `recorder.rs:426-437`) is untouched.
- **Desktop fallback reachable.** Yes. For U16, U32, I64 and F64 defaults (ALSA can report U16/U32/F64, WASAPI can report I64), `get_preferred_config` goes on to the scored search at `recorder.rs:574-614`. One pre-existing weakness remains, not introduced here: the search scores U8, I8, U16 and anything else alike (`_ => 1`). If only unbuildable formats match the rate, it can return one and the stream build then fails with `Unsupported sample format`.
- **`builds_sample_format` vs build arms.** Exact match (C4 row). The two lists are maintained by hand, and no test ties them together (Finding 4).
- **Linux regression risk.** Low for the format, but the channel count changes on the desktop (Finding 3). On this machine, `arecord -D default --dump-hw-params` shows the PipeWire default PCM offering `FORMAT: U8 S16_LE S24_LE S32_LE FLOAT_LE ...` and `CHANNELS: [1 64]`. cpal's ALSA default (`alsa/mod.rs:458-485`, `lib.rs:705-749`) ranks stereo first, then F32, then I16, then U16, so the default is F32 stereo and the format stays F32. A raw `hw:` device that offers only S16 or S32 now opens in that format and is converted; the old scored search picked the same formats. For `Mic`, Linux behaviour only improves: before, a non-f32 default was refused, and an F32 default already opened via `default.config()`.
- **The upstream commenter's report.** The change does not address it; see Finding 1.

## Findings

1. **The PR body must not claim a fix for the Realtek "Voice clarity" silence.** The issue's own evidence weakens the fix:
   - Upstream tried the equivalent change in cjpais/Handy#2144 ("just use default input config"). The reporter's side tested that build (comment of 2026-09-25) and said: "This version also doesn't work". The PR was closed unmerged.
   - The same commenter reports that with effects / Voice clarity on, "even the test recording in Windows stops working". So the APO itself may be zeroing the stream regardless of the client format.
   - The issue says the mix format is "48000 Hz, 32-bit, 2 ch, WAVEFORMATEXTENSIBLE" without the SubFormat. cpal maps an extensible 32-bit PCM mix format to `I32` and an IEEE-float one to `F32` (`wasapi/device.rs:200-215`). If that endpoint's mix format is float, `default_input_config()` returns F32, and this PR changes nothing on that machine.

   The draft body (`scratchpad/pr/pr4.md`) says that a client in the mix format "gets the signal" and that Fala asks for f32. That framing is fine as the issue's hypothesis. The body should add three facts: the change stops overriding the device's own format; upstream's equivalent attempt was reported not to help on the affected machine; and the C6 Windows run decides the outcome. Words like "fixes" or "Closes" for the Voice clarity case are not supported by any evidence.
2. **"The native format is what the OS mixes on every platform" (PR body, and the Intent decision) is true only for WASAPI.** On ALSA, cpal has no OS default. `default_input_config()` is cpal's own heuristic over the supported ranges: stereo, then F32, then I16, then U16, then the highest rate (`lib.rs:705-749`). The body should describe the Linux default as "cpal's default config", not the OS mix format.
3. **Linux desktop behaviour change not stated anywhere: mono becomes stereo on PipeWire.** The old scored search iterated cpal's ALSA ranges format-major with channels ascending and kept the first F32 range (strict `>` at `recorder.rs:599`). On a PipeWire default PCM (`CHANNELS: [1 64]`), that was F32 mono. The new early return opens cpal's default, which is F32 stereo. The sample format is unchanged. But the capture now averages 2 channels in the app instead of letting PipeWire downmix. `get_microphone_channels` (`apps/desktop/src/commands/audio.rs:337-358`, through `preferred_input_channel_count`) now reports 2 instead of 1, so the channel picker appears for a mono mic. This is derived from the cpal source and from `arecord` hw params on this machine, not observed in a running app. It is probably harmless, but the PR body should say it, and the checks have no row for it.
4. **`builds_sample_format` duplicates the build match by hand** (`recorder.rs:623-631` vs `:249-293`). Today they match exactly. A later edit that removes an arm would still pass C4's test, and devices with that default format would then fail with `Unsupported sample format`. An arm that is added but not listed only means a needless scored search. This is a maintainability note, not a defect.
5. **Precision gap in the checks: cpal 0.16's `SampleFormat::I24` is in no set.** Intent says `Mic` converts "i8 a i64, u8 a u64, f32, f64" and refuses the rest. I24 is "the rest" (`mic.rs:73`), and the desktop sends it to the scored search. Both behaviours are fine, and I24 cannot be produced by ALSA or WASAPI in cpal 0.16 (only by ASIO, which is not enabled). But "formats that fall back to the scored search (4)" has 5 members in cpal 0.16, and the check silently left one out.
6. **C2 is weak as a symptom proof.** Before this diff, `Mic` could not take i16 at all (its closure was typed `&[f32]`), and the real symptom happens in the driver. C2 proves the conversion keeps magnitude and nothing more. This is accepted as authored; C6 carries the symptom.
7. **C6 not run (manual, Windows).** It is the only proof that the change has any effect on the reported hardware. Given Finding 1, the feature should not be described as fixing #2141 until C6's 10/10 is recorded.
8. **Project rules hold in the diff.** It adds no `cfg(target_os)` (the only `cfg` is `#[cfg(test)]` at `mic.rs:175`), no `unwrap`/`expect` and no `println!`.

## Gate

- `cargo test -p fala-audio` - 28 passed, 0 failed, 1 ignored (pre-existing, needs a speech WAV)
- `cargo test -p fala --lib -- audio_toolkit::audio::recorder::tests` - 21 passed, 0 failed, 329 filtered out
- C3 structural grep - exit 0; C4 structural awk - exit 0

## Round 2 - scoped (not completed)

Written by the orchestrating author, not by a verifier. A fresh verifier was dispatched over
`07d2aa3..e1dd3e4` for the round-1 follow-ups (desktop default-format preference limited to
`#[cfg(windows)]`, honest commit message, C4 restated with a structural proof for the cfg). It did
not return within the 20-minute verifier timeout the orchestrator set (it was waiting on the shared
build lock) and was stopped. Round 2 is therefore unverified. What stands instead:

- `git diff ba0c5cb e1dd3e4 -- crates/audio` is empty, so C1-C3 carry from round 1.
- C4's structural proofs exit 0 at `e1dd3e4` (run by the author).
- The pre-push hook (`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`)
  runs over `e1dd3e4` before the branch reaches the remote; its counts are in the PR body.
