# cli-record verification

**Verdict**: PASS
**Profile**: light
**Diff range**: 773cea7..6401c7c
**Round**: 2 - scoped
**Verifier**: independent sub-agent (author != verifier)

All 26 checks are proven at `6401c7c` with located evidence. Of round 1's four gaps, two are closed
(C19's proof now discriminates; the SAFETY comment now names the hazard). The other two are only
partly closed. The report fix added three numbers that do not match the diagnostic files it cites,
and a stated reason for the aborted attempt that no raw file supports. None of these is part of a
check claim, so no check row changes. They are listed under `## Findings` as open report-accuracy
gaps.

Scope of this round: the fix diff `f30df5f..6401c7c` (round 1's report commit was rebased to
`6401c7c`) touches only `apps/cli/src/record/capture.rs` (SAFETY comment only, +4 lines at
`:393-397`), `apps/cli/tests/record.rs` (the C19 test only, +11 lines at `:335-350`) and
`docs/spikes/04-captura-dupla.md` (+3 lines at `:64-65`, one row rewritten at `:102`). `git diff
--stat f30df5f 6401c7c` lists no other files. Citations in those three files were refreshed.
Everything else is carried from round 1 (at `f30df5f`) and marked as carried.

## Binding sources

Carried from f30df5f. Step 1 runs under `ui` only. The profile is `light`, and the plan marks no
source as binding. Not run.

## Environment for the device-backed proofs

Carried from f30df5f. The six `#[ignore]`d device tests were not rerun. The fix touched none of
their code: in `tests/record.rs` the diff is confined to `analyze_rejects_recording_flags`, and in
`capture.rs` it is a comment. Round 1 ran them with the sink at 0.68 on the
`...HiFi__Speaker__sink` node before and after, and got `6 passed; 0 failed; 7 filtered out;
finished in 112.44s`, each test printing `... ok`. `wpctl get-volume @DEFAULT_AUDIO_SINK@` reads
`Volume: 0.68` today. Nothing was played or recorded in this round.

## Checks

CI-safe proofs verified at 6401c7c: `cargo test -p fala-cli` gave 8 passed (unit) and 7 passed
(boundary), with 0 failed and 6 ignored. Every named non-ignored test was listed as `... ok`. The C23-C26
shell proofs were run verbatim at 6401c7c, and each exited 0. Device-backed rows (C1, C2, C3, C7,
C12, C14, C22) are carried from f30df5f, with citations refreshed.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | `--duration 10s` exits 0, WAV 48000 Hz, 2 ch, i16 | carried from f30df5f: device run `records_stereo_48k_wav ... ok` | `apps/cli/tests/record.rs:102` - `assert_eq!(o.status.code(), Some(0), ...)`; `:104` - `(spec.sample_rate, spec.channels, spec.bits_per_sample) == (48_000, 2, 16)`; `:108` - `spec.sample_format == hound::SampleFormat::Int`. Writer spec `apps/cli/src/record/capture.rs:296` | PASS |
| C2 | SIGKILL at 15 s with `--flush-s 10` leaves ≥ 480000 readable frames | carried from f30df5f: device run `sigkill_leaves_readable_wav ... ok` | `apps/cli/tests/record.rs:142` - `assert!(reader.duration() >= 480_000, ...)`. Periodic flush now at `apps/cli/src/record/capture.rs:490` - `sink.wav.flush()` | PASS |
| C3 | 70 s run writes exactly 1 progress line with elapsed s, `mic_frames=`, `sys_frames=` | carried from f30df5f: device run `progress_once_per_minute ... ok` | `apps/cli/tests/record.rs:160` - `assert_eq!(progress.len(), 1, ...)`; `:161` - `progress[0].starts_with("60 s:")` | PASS |
| C4 | fallback fires on 96000 identical frames only | verified at 6401c7c: `record::capture::tests::fallback_needs_96000_identical_frames ... ok` | `apps/cli/src/record/capture.rs:559` - `assert!(fallback(&l, &l.clone()))`; `:562` - `assert!(!fallback(&l, &r))`; `:563` - `assert!(!fallback(&l[..95_999], &l[..95_999]))` | PASS |
| C5 | watchdog names the stream after 5 s stalled, nothing at 4.9 s | verified at 6401c7c: `record::capture::tests::watchdog_names_stream_stalled_5s ... ok` | `apps/cli/src/record/capture.rs:572` - `None` at 5.9 s (sys stalled since 1 s); `:573-576` - `Some("system")` at 5 s stall; `:579` - `None` at 4.9 s; `:580` - `Some("mic")` at 5 s | PASS |
| C6 | +1000.0 ppm, `rel_drift_ms` 20.8, square-wave RMS 0.0 dBFS | verified at 6401c7c: `record::capture::tests::metrics_follow_door_3 ... ok` | `apps/cli/src/record/capture.rs:585` - `ppm(48_048_000, 48_000, 1000.0)` = `"1000.0"`; `:586` - `rel_drift_ms(2000, 1000, 48_000)` = `"20.8"`; `:591` - `square.rms_dbfs().abs()` = `"0.0"` | PASS |
| C7 | summary is a 16-column Markdown table, one row, `rate` = 48000 | carried from f30df5f: device run `records_stereo_48k_wav ... ok` | `apps/cli/tests/record.rs:110` - `out.lines().count() == 3`; `:111` - first line `== SUMMARY_HEADER` (literal at `:14`); `:112` - `row(&out)[0] == "48000"` | PASS |
| C8 | unknown `--mic` exits 2, stderr lists inputs under `entradas disponíveis:` | verified at 6401c7c: `unknown_mic_exits_2_listing_inputs ... ok` | `apps/cli/tests/record.rs:179` - `Some(2)`; `:180-182` - stderr contains `entradas disponíveis:`; `:185` - stdout empty. Precision gap carried (Findings 4) | PASS |
| C9 | substring matcher returns first match or lists every name | verified at 6401c7c: `record::capture::tests::pick_device_matches_or_lists ... ok` | `apps/cli/src/record/capture.rs:601` - `pick_device(&names, "Fone", "saída").unwrap() == 1`; `:605-606` - error contains `Alto-falantes (Realtek)` and `Fone (USB)` | PASS |
| C10 | 15 frames into a 10-frame ring: 10 stored, 5 dropped, no block | verified at 6401c7c: `record::capture::tests::full_ring_drops_and_counts ... ok` | `apps/cli/src/record/capture.rs:617` - frames `15`; `:618` - dropped `5`; `:619` - `c.slots() == 10`. Non-blocking push `:59` | PASS |
| C11 | click is 1000 Hz, 960 samples, peak in [0.49, 0.5], sign change every 24 | verified at 6401c7c: `record::capture::tests::click_is_1khz_20ms_half_amplitude ... ok` | `apps/cli/src/record/capture.rs:626` - `c.len() == 960`; `:628` - `(0.49..=0.5).contains(&peak)`; `:633-634` - `a * b < 0.0` per 24-sample half-period | PASS |
| C12 | `click_1_s` in [2.0, 2.1], `click_2_s` in [8.0, 8.1], sys onsets within 0.5 s | carried from f30df5f: device run `clicks_at_2s_and_before_end ... ok` | `apps/cli/tests/record.rs:198` - `(2.0..=2.1).contains(&c1)`; `:199` - `(8.0..=8.1).contains(&c2)`; `:204-210` - `(sys_start - c1).abs() <= 0.5` and `(sys_end - c2).abs() <= 0.5` | PASS |
| C13 | output that cannot open exits 2 before recording, names `--no-click` | verified at 6401c7c: `click_output_failure_exits_2 ... ok` | `apps/cli/tests/record.rs:231` - `Some(2)`; `:232` - stderr contains `--no-click`; `:233` - `!wav.exists()` | PASS |
| C14 | `--no-click` prints `-` for both clicks | carried from f30df5f: device run `no_click_prints_dashes ... ok` | `apps/cli/tests/record.rs:243` - `(r[10], r[11]) == ("-", "-")` | PASS |
| C15 | 40 s synthetic WAV gives the exact 8-value row, exit 0 | verified at 6401c7c: `analyze_prints_offsets_and_drift ... ok` | `apps/cli/tests/record.rs:258` - `Some(0)`; `:262-265` - `row(&out) == ["2.000", "2.010", "10.0", "38.000", "38.030", "30.0", "20.0", "555.6"]` | PASS |
| C16 | 3600 s timeline gives offset 10.0 / 30.0, drift 20.0 | verified at 6401c7c: `record::analyze::tests::one_hour_click_pair_drift_is_20ms ... ok` | `apps/cli/src/record/analyze.rs:163` - `row[2] == "10.0"`; `:164` - `row[5] == "30.0"`; `:165` - `row[6] == "20.0"` | PASS |
| C17 | missing onset exits 1 naming `sys/end` and its peak | verified at 6401c7c: `missing_onset_exits_1_naming_window ... ok` | `apps/cli/tests/record.rs:274` - `Some(1)`; `:276-277` - stderr contains `sys/end` and `pico 0.000` | PASS |
| C18 | non 2-ch i16 WAV exits 2 naming the spec (mono i16, stereo f32) | verified at 6401c7c: `analyze_rejects_wrong_spec ... ok` | `apps/cli/tests/record.rs:318` - `Some(2)`; `:319` - stderr contains the cited spec (`1 canal(is), 16 bits int`, `2 canal(is), 32 bits float`) | PASS |
| C19 | `--analyze` plus each of the 6 recording flags exits 2 | verified at 6401c7c: `analyze_rejects_recording_flags ... ok` | `apps/cli/tests/record.rs:340` - baseline `--analyze <valid wav>` alone `== Some(0)`; `:345` - `assert_eq!(o.status.code(), Some(2), "{flag:?} accepted")` over the 6 flags at `:327-334`; `:346-350` - stderr contains `cannot be used with`. Now discriminating (see below) | PASS |
| C20 | `--duration` accepts `5s`/`5m`/`5h`, rejects 5 bad forms with exit 2 | verified at 6401c7c: `record::tests::duration_accepts_only_s_m_h ... ok`; `bad_duration_exits_2 ... ok` | `apps/cli/src/record/mod.rs:114-116` - 5 / 300 / 18000 s; `:118` - `parse_duration(bad).is_err()`; `apps/cli/tests/record.rs:367` - `Some(2)` per bad form | PASS |
| C21 | `--analyze` stdout is exactly 3 lines; diagnostics on stderr | verified at 6401c7c: `analyze_prints_offsets_and_drift ... ok` | `apps/cli/tests/record.rs:260` - `out.lines().count() == 3`; `:261` - header `== ANALYZE_HEADER` | PASS |
| C22 | 3 s `--no-click` in silence: R 0 non-zero samples, L > 0 | carried from f30df5f: device run `system_channel_is_the_silent_monitor ... ok` | `apps/cli/tests/record.rs:386` - `assert_eq!(nonzero_r, 0, ...)`; `:387` - `assert!(nonzero_l > 0, ...)`. Env vars set at `apps/cli/src/record/capture.rs:386-387`, removed at `:399-400` | PASS |
| C23 | report has the 4 sections, no heading with "Recomenda" | verified at 6401c7c: shell proof verbatim, exit 0 | `docs/spikes/04-captura-dupla.md:5` `## Objetivo`; `:18` `## Como reproduzir`; `:49` `## Evidência medida`; `:139` `## Windows`; "recomenda" only in body text at `:16` | PASS |
| C24 | Evidência medida has the 60 min summary and analysis tables, date, sink/mic `node.name`, commit | verified at 6401c7c: shell proof verbatim, exit 0 | `docs/spikes/04-captura-dupla.md:51` date `2026-09-27`; `:52` sink `node.name`; `:54` `...HiFi__Mic1__source`; `:55` commit `8b04aca`; `:85` run 3 row `48000`, `3600.048`; `:92` analysis row `2.133`, `2.234`, `100.8`. The 60 min numbers are unchanged since round 1 and still match the raw files. The new sentences at `:64-65` and `:102` do not (Findings 5, 6) | PASS |
| C25 | graph-clock resampling, Linux does not predict Windows, above/below 50 ms next to the §7 line | verified at 6401c7c: shell proof verbatim, exit 0 | `docs/spikes/04-captura-dupla.md:114` - "reamostradas para o clock do grafo"; `:115` - "o número do Linux não prevê o Windows"; `:135-137` - "correção por timestamp a cada N s" ... "abaixo de 50 ms" / "passou de 50 ms nas três rodadas" | PASS |
| C26 | Windows section: `TODO(windows)`, headset `--system "…"`, Voicemeeter `--mic`, what to record | verified at 6401c7c: shell proof verbatim, exit 0 | `docs/spikes/04-captura-dupla.md:141` `TODO(windows)`; `:148` `--system "<nome da saída do fone>"`; `:152` `--mic "Voicemeeter"`; `:159` "O que registrar em cada rodada" | PASS |

### C19 re-judged (verified at 6401c7c)

Reasoned from `apps/cli/src/record/mod.rs:15-43` and `:58-63`. With `conflicts_with_all` at
`mod.rs:38` removed, each of the 6 invocations would still parse. `--out`, `--duration` and
`--system` are only `required_unless_present = "analyze"`. The test's values (`5s`, `5`, `m`, `s`)
all pass their parsers. `run` would then take the `if let Some(wav) = &args.analyze` branch at
`:59`, analyze the valid 40 s WAV and return `Ok` (exit 0). That breaks `record.rs:345`
(`Some(2)`) and `:346-350` (`cannot be used with`, clap's text for an argument conflict only). Round
1 flagged the missing WAV as the reason exit 2 would come from `analyze.rs:29-31`; that reason is
gone because `:340` pins exit 0 for the same WAV alone. The per-flag stderr assertion also catches
dropping a single flag from the list. The test now discriminates. This was established by reading
the code, not by mutation (no fault injection under `light`).

### SAFETY comment re-judged (verified at 6401c7c)

`apps/cli/src/record/capture.rs:393-397` now reads, in short: threads already exist here (the cpal
system-stream thread and the pipewire-alsa loop), so a C `getenv` in them can race this
`unsetenv`; accepted spike risk; the plugin read the variables when it opened the PCM; neither
thread re-reads the environment afterwards; phase 3 replaces this with `pipewire-rs`.

- The hazard is now named correctly. `cpal-0.16.0/src/host/alsa/mod.rs:966-968` spawns
  `cpal_alsa_in` when the stream is built, and `build_input_stream_raw` (`:89-104`) runs that at
  build time. `libasound_module_pcm_pipewire.so` imports `pw_thread_loop_new` and
  `pw_thread_loop_start`, so a loop thread exists once the PCM is open. Round 1's error ("como
  acima", no thread) is gone.
- The claim "nenhuma das duas threads relê o ambiente depois disso" is still an assertion. The
  plugin and `libpipewire-0.3.so.0` both import `getenv`, and libpipewire references about 20
  `PIPEWIRE_*` variables (`PIPEWIRE_DEBUG`, `PIPEWIRE_LOG`, `PIPEWIRE_NODE`,
  `PIPEWIRE_AUTOCONNECT`, ...). Nothing readable here shows that none of those reads runs on the
  loop thread after open. The comment frames the whole block as an accepted risk, so this is
  judged accurate enough for spike code, with that one sub-claim unverified.
- Minor: when `opened` is `Err`, no stream thread exists, so "já existem threads" overstates.
  That case is harmless.
- The first SAFETY block (`:382-384`) is unchanged and carried from round 1. It is plausible, and
  it assumes the probes at `:361` and `:373` leave no thread behind.

## Report sentences changed in 6401c7c against the raw files

Verified at 6401c7c. Raw files are in `~/.cache/fala-bench/recordings/`. `peaks.py` needs numpy,
which is not installed. The WAVs were therefore re-read with the Python stdlib (`wave`, `array`),
taking the max abs of channel L in windows around each click and over the whole file.

| Report item | Raw source | Match |
| --- | --- | --- |
| `:102` "a 0.18 o pico do mic nos cliques é 0.005" | `diag/peaks.txt`: sink018 mic top 10 ms bins at 0.53-0.58 s (0.003-0.005). Clicks are at 2.19 s and 8.2 s (sys). Recomputed mic max in 1.9-2.6 s and 7.9-8.6 s: 0.002 and 0.002. Whole-file mic max 0.0055 at ~0.54 s | no - 0.005 is the file's peak before any click; the peak at the clicks is 0.002 |
| `:102` "a 0.68 é 0.092-0.136" | `peaks.txt` sink068-a: 0.095 (2.10 s), 0.136 (8.16 s); sink068-b: 0.119 (2.14 s), 0.122 (8.16 s). Recomputed per-click window max: 0.0951, 0.1357, 0.1193, 0.1224 | no - the per-click peaks span 0.095-0.136. 0.092 is a non-maximal neighbour bin (b, 8.18 s). Round 1's text had 0.095-0.136 |
| `:102` "contra ruído mediano de 0.0012-0.0019" (attached to 0.68) | `peaks.txt` medians: sink068-a 0.0017, sink068-b 0.0019, sink018 0.0012 | partly - the 0.68 files give 0.0017-0.0019. 0.0012 is the 0.18 file |
| `:102` "WAVs e picos em `~/.cache/fala-bench/recordings/diag/`" | 3 WAVs (10.07-10.15 s, 48 kHz stereo i16), `peaks.py`, `peaks.txt` present | yes - the files exist. The volumes 0.18 and 0.68 appear only in the file names, with no volume or routing log. All three WAVs have mtime 00:00:42Z on 2026-09-28, within 100 ms of each other, after run 3 and after round 1. They were copied rather than recorded then, and when they were recorded is not logged |
| `:64` "tentativa às 22:48:28Z ... interrompida em ~23 s e descartada" | `clicks68/run-start.txt` `2026-09-27T22:48:28Z`; `record.err` start log 22:48:29Z; WAV mtime 22:48:52.6Z; `summary.md` empty; readable WAV 20.096 s (last flush) | yes |
| `:64-65` "o volume estava em 0.98" | `clicks68/run-volume.txt` `Volume: 0.98` | yes |
| `:65` "um fone plugado tinha trocado o sink e o mic" | no file in `clicks68/` records a headset, a device change or routing. `record.err` shows `system=...HiFi__Speaker__sink, mic=default`. `journalctl` 19:40-19:52 local shows no plug event. In the WAV, the Speaker monitor (R) holds the click at 2.19-2.21 s (peak 0.500, 995 non-zero samples), so the click reached the Speaker sink. The mic hears it at 0.137 | no - no raw source, and at the first click the WAV shows the Speaker was still the output. A swap after 2 s is not ruled out, but nothing records one |

## Coverage

Carried from f30df5f. Under `light` the Coverage recompute does not run. The fix added no branch
and no member.

## Faults injected

Not run: under `light` fault injection does not run. C19's discrimination was re-judged by reading
the code (above).

## Swept existing

Carried from f30df5f. No `Swept` row resolves to an existing constraint. The fix touched none.

## Findings

Findings 1, 2 and 6 are from round 1, re-judged at 6401c7c. Findings 3, 4 and 7 are carried from
f30df5f with lines refreshed. Finding 5 is new in this round.

1. **SAFETY comment on `remove_var` - mostly closed.** `apps/cli/src/record/capture.rs:393-397`
   now names the thread hazard and accepts it as spike risk. The residual claim that neither
   thread re-reads the environment after open is unverified (see the SAFETY section).
2. **C19 discrimination - closed.** `apps/cli/tests/record.rs:340` and `:346-350` make the test
   fail without `conflicts_with_all` at `apps/cli/src/record/mod.rs:38`.
3. **Level gap, carried.** The exit-1 wiring for AC 4 and AC 5 (`capture.rs:465-480` break
   paths) is never run by a test. `checks.md` names this gap. Whether the fallback detector fires
   on the real fallback is unproven. C22 is what guards door 1 on this machine.
4. **Precision gap in C8, carried.** `apps/cli/tests/record.rs:180-182` asserts the label only,
   not an input name after it.
5. **New: the rewritten diagnostic sentence misstates its own sources.** At
   `docs/spikes/04-captura-dupla.md:102`, "a 0.18 o pico do mic nos cliques é 0.005" is the file's
   peak at ~0.54 s, not at the clicks, where it is 0.002. "0.092-0.136" should read 0.095-0.136
   by per-click peak. The noise range 0.0012-0.0019 mixes in the 0.18 file. The per-file volumes
   are backed only by file names.
6. **Aborted attempt - disclosed, but the reason has no raw source.**
   `docs/spikes/04-captura-dupla.md:64-65` now discloses the 22:48:28Z attempt. Its time,
   duration and volume match `clicks68/`. "um fone plugado tinha trocado o sink e o mic" is not
   recorded anywhere, and the WAV shows the first click played on the Speaker sink.
7. **Style, minor, carried.** Progress uses `eprintln!` (`capture.rs:496`) per AC 3's line shape.

Also verified at 6401c7c: `cargo clippy -p fala-cli --all-targets -- -D warnings` is clean,
`cargo fmt --all -- --check` exits 0, and `git status --porcelain` was empty before this report
was written.

## Gate

- CI-safe, verified at 6401c7c: `cargo test -p fala-cli` gave 8 passed (unit) + 7 passed
  (boundary), 0 failed, 6 ignored. Each named test was listed as `... ok`.
- Device-backed, carried from f30df5f: 6 passed, 0 failed (112.44 s).
- Report proofs C23-C26, verified at 6401c7c: 4 shell proofs, each exit 0.
