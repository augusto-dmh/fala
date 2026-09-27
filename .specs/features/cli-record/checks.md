# cli-record - checks

Profile: light
Plan: `.specs/features/cli-record/plan.md`

## Intent

26 checks in 6 slices · 3 one-way doors · 0 open

Device-backed proofs are `#[ignore]`d tests in `apps/cli/tests/record.rs` that read the sink's
`node.name` from `FALA_TEST_SINK` and **fail** when it is missing. They need this machine's
PipeWire, speakers at ≥ 50 % volume, and nothing else playing (C22 needs silence). They run with
`cargo test -p fala-cli --test record -- --ignored --exact <name>`. Everything else runs in CI.

## Checks

Unit proofs: `cargo test -p fala-cli --bin fala-cli <path>`. Boundary proofs spawn the built binary.

### S1 - gravação dupla · 4 files · 40 KB · ~10k

**C1** - `--out t.wav --duration 10s --system <sink>` exits 0 and writes a WAV of 48000 Hz, 2 channels, 16-bit int (plan AC 1, door 2)
Proof: `cargo test -p fala-cli --test record -- --ignored --exact records_stereo_48k_wav`

**C2** - With `--flush-s 10`, a 20 s recording killed with SIGKILL at 15 s leaves a file `hound` opens with ≥ 10 s (480000) of frames (plan AC 2, door 2)
Proof: `cargo test -p fala-cli --test record -- --ignored --exact sigkill_leaves_readable_wav`

**C3** - A 70 s recording writes exactly 1 progress line to stderr, carrying the elapsed seconds, `mic_frames=` and `sys_frames=` (plan AC 3)
Proof: `cargo test -p fala-cli --test record -- --ignored --exact progress_once_per_minute`

**C4** - Fallback detection fires when the first 96000 frames of L and R are identical sample by sample, and not when a single sample differs or fewer than 96000 frames exist (plan AC 4)
Proof: `cargo test -p fala-cli --bin fala-cli record::capture::tests::fallback_needs_96000_identical_frames`

**C5** - The watchdog names `mic` or `system` once that stream's frame count has not grown for 5 s, and names nothing at 4.9 s (plan AC 5)
Proof: `cargo test -p fala-cli --bin fala-cli record::capture::tests::watchdog_names_stream_stalled_5s`

**C6** - Door 3 recording metrics: 48 048 000 frames over 1000 s at 48 kHz is `+1000.0` ppm; mic 1000 frames ahead of sys is `rel_drift_ms` `20.8`; RMS of a full-scale square wave is `0.0` dBFS (plan AC 6, door 3)
Proof: `cargo test -p fala-cli --bin fala-cli record::capture::tests::metrics_follow_door_3`

**C7** - The recording summary on stdout is a Markdown table with exactly the columns `rate wall_s mic_frames sys_frames mic_ppm sys_ppm rel_drift_ms dropped_mic dropped_sys stream_errors click_1_s click_2_s mic_peak mic_rms_dbfs sys_peak sys_rms_dbfs`, one data row, `rate` = `48000` (plan AC 6, AC 19)
Proof: `cargo test -p fala-cli --test record -- --ignored --exact records_stereo_48k_wav`

**C8** - `--mic <name no input has>` exits 2 and stderr lists the available input names under `entradas disponíveis:` (plan AC 7)
Proof: `cargo test -p fala-cli --test record -- --exact unknown_mic_exits_2_listing_inputs`

**C9** - Device matching by substring returns the first name containing the needle, and when none does, an error listing every name (the path `--system` takes on a non-ALSA host) (plan AC 8)
Proof: `cargo test -p fala-cli --bin fala-cli record::capture::tests::pick_device_matches_or_lists`

**C10** - Pushing more frames than the ring holds drops the excess, counts it, and returns without blocking: 15 frames into a 10-frame ring store 10 and count 5 dropped (plan AC 9)
Proof: `cargo test -p fala-cli --bin fala-cli record::capture::tests::full_ring_drops_and_counts`

### S2 - cliques · 2 files · 20 KB · ~5k

**C11** - The click is a 1000 Hz sine, 20 ms long, peak amplitude 0.5: at 48 kHz, 960 samples, max |x| in [0.49, 0.5], zero crossings every 24 samples (plan AC 10)
Proof: `cargo test -p fala-cli --bin fala-cli record::capture::tests::click_is_1khz_20ms_half_amplitude`

**C12** - A 10 s recording reports `click_1_s` in [2.0, 2.1] and `click_2_s` in [8.0, 8.1], and `--analyze` on the file finds the system onsets within 0.5 s of both (plan AC 10)
Proof: `cargo test -p fala-cli --test record -- --ignored --exact clicks_at_2s_and_before_end`

**C13** - When the default output cannot open (ALSA pointed at a nonexistent config), the command exits 2 before recording and stderr names `--no-click` (plan AC 11)
Proof: `cargo test -p fala-cli --test record -- --exact click_output_failure_exits_2`

**C14** - `--no-click` prints `-` in `click_1_s` and `click_2_s` (plan AC 12)
Proof: `cargo test -p fala-cli --test record -- --ignored --exact no_click_prints_dashes`

### S3 - análise de drift · 2 files · 20 KB · ~5k

**C15** - `--analyze` on a 40 s synthetic WAV (L clicks at 2.000 s and 38.000 s, R at 2.010 s and 38.030 s) exits 0 and prints one row: `onset_mic_start_s` `2.000`, `onset_sys_start_s` `2.010`, `offset_start_ms` `10.0`, `onset_mic_end_s` `38.000`, `onset_sys_end_s` `38.030`, `offset_end_ms` `30.0`, `drift_ms` `20.0`, `drift_ppm` `555.6` (plan AC 13, door 3)
Proof: `cargo test -p fala-cli --test record -- --exact analyze_prints_offsets_and_drift`

**C16** - The analysis over a 3600 s timeline (clicks L 2.000 / R 2.010, L 3598.000 / R 3598.030) gives `offset_start_ms` 10.0, `offset_end_ms` 30.0, `drift_ms` 20.0 (plan AC 14)
Proof: `cargo test -p fala-cli --bin fala-cli record::analyze::tests::one_hour_click_pair_drift_is_20ms`

**C17** - A channel with no onset in a window exits 1, stderr naming the channel/window (`sys/end`) and that channel's peak there (plan AC 15)
Proof: `cargo test -p fala-cli --test record -- --exact missing_onset_exits_1_naming_window`

**C18** - A WAV that is not 2-channel 16-bit int exits 2 naming the spec found; table-driven over mono i16 and stereo f32 (plan AC 16)
Proof: `cargo test -p fala-cli --test record -- --exact analyze_rejects_wrong_spec`

### S4 - interface · 2 files · 15 KB · ~4k

**C19** - `--analyze` combined with each of `--out`, `--duration`, `--mic`, `--system`, `--no-click`, `--flush-s` exits 2; table-driven over the 6 flags (plan AC 17)
Proof: `cargo test -p fala-cli --test record -- --exact analyze_rejects_recording_flags`

**C20** - `--duration` accepts `5s`, `5m`, `5h` (= 5, 300, 18000 s) and exits 2 for `5min`, `5`, `1.5m`, `0s`, `-5s` (plan AC 18)
Proof: `cargo test -p fala-cli --bin fala-cli record::tests::duration_accepts_only_s_m_h`
Proof: `cargo test -p fala-cli --test record -- --exact bad_duration_exits_2`

**C21** - `--analyze` stdout is exactly 3 lines (header, separator, row); the analysis diagnostics go to stderr (plan AC 19)
Proof: `cargo test -p fala-cli --test record -- --exact analyze_prints_offsets_and_drift`

**C22** - Door 1: on this machine, a 3 s `--no-click` recording with nothing playing has 0 non-zero samples in R (the sink monitor) and > 0 in L (the mic): the system stream is the monitor, not the mic (plan door 1)
Proof: `cargo test -p fala-cli --test record -- --ignored --exact system_channel_is_the_silent_monitor`

### S5 - relatório · 1 file · 10 KB · ~3k

**C23** - `docs/spikes/04-captura-dupla.md` has `## Objetivo`, `## Como reproduzir`, `## Evidência medida`, `## Windows`, and no heading containing "Recomenda" (plan AC 20)
Proof: `test "$(grep -cE '^## (Objetivo|Como reproduzir|Evidência medida|Windows)$' docs/spikes/04-captura-dupla.md)" -eq 4 && ! grep -iE '^#+ .*recomenda' docs/spikes/04-captura-dupla.md`

**C24** - `## Evidência medida` holds the recording summary table and the analysis table of a 60 min recording on this machine, with its date, the sink and mic `node.name`, and the `fala` commit (plan AC 21)
Proof: `s=$(awk '/^## Evidência medida/,/^## Windows/' docs/spikes/04-captura-dupla.md); for p in '| rate | wall_s |' '| onset_mic_start_s |' '2026-' 'alsa_output.pci-0000_00_1f.3-platform-skl_hda_dsp_generic.HiFi__Speaker__sink' 'HiFi__Mic1__source' 'commit'; do grep -qF -- "$p" <<<"$s" || exit 1; done; grep -qE '^\| 48000 \| 3[56][0-9]{2}\.' <<<"$s"`

**C25** - `## Evidência medida` states that PipeWire resamples both captures to the graph clock and that the Linux number does not predict Windows; and states whether `drift_ms` and `rel_drift_ms` are above or below 50 ms next to the design doc line "correção por timestamp a cada N s" (plan AC 22, AC 24)
Proof: `s=$(awk '/^## Evidência medida/,/^## Windows/' docs/spikes/04-captura-dupla.md); for p in 'clock do grafo' 'não prevê o Windows' 'correção por timestamp a cada N s' '50 ms'; do grep -qF -- "$p" <<<"$s" || exit 1; done`

**C26** - `## Windows` carries `TODO(windows)`, a `--system "<…>"` invocation with the ordinary headset, one with Voicemeeter VAIO as `--mic`, and what to record (plan AC 23)
Proof: `s=$(awk '/^## Windows/,0' docs/spikes/04-captura-dupla.md); for p in 'TODO(windows)' '--system "' 'Voicemeeter' '--mic' 'registrar'; do grep -qiF -- "$p" <<<"$s" || exit 1; done`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| `fala-cli record` exit statuses (3) | 0 C1, C15 · 1 C4, C5, C17 · 2 C8, C13, C18, C19, C20 | - |
| exit-1 causes during a run (3) | fallback C4 · stalled or errored stream C5 (a cpal stream error stops its callbacks, the watchdog ends the run) · onset missing C17 | - |
| exit-2 causes (6) | flags C19 · duration C20 · mic not found C8 · system not found C9 · click output C13 · WAV spec C18 | - |
| modes (2) | record C1 · analyze C15 | - |
| `--analyze`-rejected flags (6) | C19, table-driven over all 6 | - |
| `--duration` units (3) | `s` C20 · `m` C20 · `h` C20 | - |
| analysis windows × channels (4) | mic/start C15 · sys/start C15 · mic/end C15 · sys/end C15, C17 | - |
| door 3 formulas (6) | `X_ppm` C6 · `rel_drift_ms` C6 · `offset_ms` C15, C16 · `drift_ms` C15, C16 · `drift_ppm` C15 · onset threshold C15, C17 | - |
| door 2 WAV shape (4) | 48 kHz C1 · 2 ch C1 · i16 C1 · flush C2 | - |
| host branch for `--system` (2) | ALSA/PipeWire C22 · other host (WASAPI loopback) C9 matcher + `TODO(windows)` run in C26 | - |
| summary columns (16) | C7, table-driven over all 16 (the exact header) | - |

- A stream that fails to build or start (exit 1 before recording) has no deterministic trigger on
  this machine; it is one `map_err` in the code and is left as a named sampling gap.
- The non-ALSA host path is proven here only through its device matcher (C9); opening a WASAPI
  output as loopback is the Windows run C26 leaves ready. Named, not hidden.
- Claims naming an exit status or stdout shape: C1, C7, C8, C13, C15, C17-C21 - each proof runs
  the built binary. C4 and C5 prove the decision at its own layer; the exit-1 wiring they feed
  runs in the 60 min recording (a watchdog trip or fallback there would have ended it with 1).

## Swept

- validation: C8, C18, C19, C20
- failure modes: C2, C4, C5, C13, C17
- idempotency: n/a - each run writes a new file at `--out`; rerunning overwrites it by intent
- authorization: n/a - local CLI; OS microphone permission is outside the process
- concurrency: C10 (callbacks never block; the writer drains two lock-free rings)
- data lifecycle: C2 (the file stays valid up to the last flush); nothing else persists
- dependency failure: C5, C13 (a stalled device, an output that will not open)
- state transitions: n/a - no stateful entity beyond one recording
- observability: C3, C7, C21

## Handoff

- S1-S4 touch `apps/cli` (new `src/record/{mod,capture,analyze}.rs`, `src/main.rs`, `Cargo.toml`,
  `tests/record.rs`) ≈ 45 KB written + ≈ 60 KB read (plan, cpal traits/ALSA/WASAPI excerpts,
  rtrb, hound) ≈ 105 KB / 4 ≈ 26k; S5 ≈ 3k plus the 60 min recording; total ≈ 30k, under the
  150k budget - one builder
- Mechanism: one builder (fits)
- **Settled mid-build:** C12 first allowed 0.2 s between `click_N_s` and the system onset; the
  measured lag is 0.15-0.21 s (output buffer, and the WAV timeline starting at the first captured
  frame rather than at wall-clock t0), so the proof sat on the boundary. Augusto approved on
  2026-09-27 widening it to 0.5 s (the other click is ≥ 6 s away); the drift metrics never use
  `click_N_s`. The report records the lag
- **Settled mid-build:** the 60 min evidence took three runs. Run 1 (speech playing) recorded
  cleanly but speech masked the end click in `--analyze`; run 2 (clicks only) ran with the sink
  lowered to 0.06 between runs, so the mic did not hear the clicks; run 3 (clicks only, 0.68,
  volume and routing logged every minute) gave the analysis table. Augusto chose each rerun on
  2026-09-27; the report keeps all three
