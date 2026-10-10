# desktop-pipeline-audio verification

**Verdict**: PASS
**Profile**: light
**Diff range**: eb0482c..857f3a9 (4aba4e7, 12fc982, 857f3a9); HEAD `857f3a9`
**Verifier**: independent sub-agent (author != verifier). The author is the desktop-pipeline-audio executor. This Verifier was dispatched fresh by the orchestrator on the Windows 11 machine. It built none of the code and fixed nothing. The only file it wrote is this report.

All 14 automated checks (C1-C11, C13-C15) are proven at `857f3a9` with a located assertion, a grep or command output. C12 is the manual Windows check. `checks.md` marks it manual, so it is listed under `## Manual (outside the verdict)` with the executor's facts and does not affect the verdict. One part of the gate could not run on this machine: `cargo deny check` (not installed). The diff adds no external crate (`Cargo.lock` gains only the internal `fala-audio` edge for `fala`), so `cargo deny` has nothing new to judge. The review found no blocking defect. It found four non-blocking ones (see `## Findings`).

## Checks

All proofs ran at `857f3a9` (`git rev-parse HEAD`) in `C:\dev\fala\.houston\worktrees\desktop-pipeline-audio`, with `CARGO_TARGET_DIR=C:\f\pipe`, one cargo command at a time. `git status --porcelain` was empty before this report was written. The 7 named desktop tests ran in one invocation: `cargo test -p fala --lib -- --exact dictation_capture::tests::prebuffer_leads_the_recording dictation_capture::tests::disabled_policy_keeps_everything dictation_capture::tests::utterances_reach_router_and_recording dictation_capture::tests::detector_error_counts_as_voice dictation_capture::tests::readiness_fires_on_first_block_after_start dictation_capture::tests::levels_only_while_recording settings::tests::earshot_store_still_loads`. It exited 0 with `7 passed; 0 failed; 0 ignored; 348 filtered out`, and each name printed `... ok`. In the table, "batch" refers to this run.

| Check | Claim | Proof run | Evidence (file:line) | Result |
| --- | --- | --- | --- | --- |
| C1 | 500 ms before `Start`, 200 ms after, always-voice detector: `stop` returns exactly samples 3 200..11 200 (AC 1, door 1) | `dictation_capture::tests::prebuffer_leads_the_recording ... ok` (batch) | `apps/desktop/src/dictation_capture.rs:425-431`: feeds `ramp(0..8_000)` in 160-sample blocks, `start(Offline)`, feeds `ramp(8_000..11_200)`, and `assert_eq!(p.stop(), ramp(3_200..11_200))`. The pre-buffer is the crate's (`crates/audio/src/capture.rs:64-68`, `PREBUFFER_SAMPLES = 4_800` at `:17`), and `DictationCapture::start` pushes it through the session first (`:77-85`). | PASS |
| C2 | `Disabled` with an always-silence detector returns pre-buffer + session; `Offline` with the same detector returns nothing (AC 2) | `...disabled_policy_keeps_everything ... ok` (batch) | `dictation_capture.rs:434-447`: `assert_eq!(p.stop(), input)` (all 4 800 samples) and `assert!(p.stop().is_empty())`. Bypass: `Processor::start` stores `policy == Disabled` in `bypass` (`:142-143`) before `capture.start()` (`:148`), so the pre-buffer is also classified as voice. `DictationVad::is_voice` returns `Ok(true)` without touching the detector (`:54-56`). | PASS |
| C3 | two utterances 600 ms apart: the audio callback gets 2 blocks in order, `stop` returns their concatenation without the middle silence beyond pre-roll and hangover (AC 3, door 3) | `...utterances_reach_router_and_recording ... ok` (batch) | `dictation_capture.rs:450-474`: lengths `[25 * FRAME_SAMPLES, 25 * FRAME_SAMPLES]` (2 onset + 8 voice + 15 hangover; 15 pre-roll + 2 onset + 8 voice), `recording == routed.concat()`, and `recording.len() < codes.len() * FRAME_SAMPLES` (10 of 40 silent frames dropped). `Processor::keep` sends each utterance to `audio_cb` and appends it to `samples` in the same loop (`:160-172`). | PASS |
| C4 | a detector that errors: the whole session comes back as voice, no panic (AC 4) | `...detector_error_counts_as_voice ... ok` (batch) | `dictation_capture.rs:477-482`: `Broken` detector, `assert_eq!(p.stop().len(), 3_200)`. Code: `:57-67` maps `Err` (and a poisoned lock) to `true` and warns only while `warned` is false. `warned` is cleared in `reset` (`:70-75`), which `DictationCapture` calls at `start` and `stop` (`capture.rs:81`, `:102`), so the warn is once per session. The log line has the error only, no audio. The warn itself is not asserted by the test (read, not tested). | PASS |
| C5 | readiness fires on the first block after `Start`, not before; a `stop` before any block drops the sender (AC 5) | `...readiness_fires_on_first_block_after_start ... ok` (batch) | `dictation_capture.rs:485-498`: `try_recv().is_err()` after `start`, `try_recv().is_ok()` after one 160-sample `feed`, then `start` + `stop` and `rx.recv().is_err()`. Code: `start` arms `ready` (`:147`), `feed` sends only for a non-empty block while recording (`:121-136`), `stop` sets `ready = None` (`:154`). In the worker, the drain runs before the command (`:343-347`), so the block that resolves readiness is drained on the tick after `Start`. | PASS |
| C6 | blocks outside a session do not call the level callback; inside they do (AC 6) | `...levels_only_while_recording ... ok` (batch) | `dictation_capture.rs:501-516`: count stays `0` after 1 s idle, `> 0` after 1 s recording. Code: the visualizer is fed only under `if self.recording` (`:125-128`), and `start` resets it (`:144`). | PASS |
| C7 | the detector lives in an `Arc` built in `DictationRecorder::new`; `open` only clones it (AC 7) | `grep -c "SileroVad::load" apps/desktop/src/dictation_capture.rs apps/desktop/src/managers/audio.rs` printed `0` and `1` (sum 1) | The one load is in `create_audio_recorder` (`apps/desktop/src/managers/audio.rs:286`). `DictationRecorder::new` wraps it in `Arc<Mutex<..>>` (`dictation_capture.rs:195-205`). `open` does `Arc::clone(&self.detector)` (`:247`). The manager builds the recorder only when `recorder_opt.is_none()` (`managers/audio.rs:579-590`) and keeps it across `close`/`open`. Startup preloads it off the hotkey path (`apps/desktop/src/lib.rs:207-216`). | PASS |
| C8 | `needs_reopen()` reads the `Mic::failed()` the worker copies each tick, besides the worker having ended (AC 8) | `cargo test -p fala-audio mic::tests` exit 0, `1 passed; 0 failed; 23 filtered out` | `crates/audio/src/mic.rs:190-193` sets `failed` in the cpal error callback; `:126-128` exposes it. The worker copies it each tick (`dictation_capture.rs:348-350`). `needs_reopen` = `failed || worker.is_finished()` (`:312-314`), the same rule as the inherited recorder (`git show eb0482c:.../recorder.rs`, `needs_reopen` = `stream_error \|\| is_finished`). `start_microphone_stream` checks it on every start and rebuilds (`managers/audio.rs:592-631`). `open` clears the flag before a new worker (`dictation_capture.rs:243`). | PASS |
| C9 | `mono(&[0.1, 0.5, 0.9], Some(1)) = 0.5`; `Some(3)` and `None` give the average; `[0.2]` with `Some(0)` = 0.2 (AC 9, door 4) | `cargo test -p fala-audio mic::tests::mono_picks_channel_or_averages -- --exact` exit 0, `1 passed; 0 failed; 23 filtered out` | `crates/audio/src/mic.rs:204-210`. Code: `mono` (`:161-166`) picks `frame.get(c)` or averages. `open_device` also filters `channel` against the opened channel count (`:81`). | PASS |
| C10 | `fala-cli dictate`/`meeting` still call `Mic::open(needle)`, which delegates to `open_device`; `fala-audio` and `fala-cli` tests pass (AC 10) | `cargo test -p fala-audio` exit 0: lib 24 passed, `manifest` 1, `meeting_shape` 1, `silero` 1 passed + 1 ignored, doc 0. `cargo test -p fala-cli` exit 0: unit 36, `bench` 16 (+10 ignored), `dictate` 3 (+6), `history` 13, `import` 3 (+4), `mcp` 36, `meeting` 0 (+3), `record` 7 (+6); 114 passed, 0 failed | `crates/audio/src/mic.rs:34-43`: `find` by substring or the default input, then `Self::open_device(device, None)`. Callers unchanged: `apps/cli/src/dictate.rs:109`, `apps/cli/src/meeting.rs:58`. `None` keeps the old channel average. | PASS |
| C11 | `managers/audio.rs` imports nothing from `audio_toolkit::vad` or `AudioRecorder`; the device selector reopens through the new recorder (AC 11) | `grep -nE "AudioRecorder\|SmoothedVad\|EarshotVad\|audio_toolkit::vad" apps/desktop/src/managers/audio.rs apps/desktop/src/actions.rs apps/desktop/src/commands/audio.rs` printed nothing (exit 1) | `update_selected_device` (`managers/audio.rs:819-829`) invalidates the device cache, then `stop_microphone_stream` (`close` joins the worker, `:704-728`) and `start_microphone_stream`, which re-resolves the device and calls `rec.open(resolution.device)` with `worker == None`, so the new device is used (`:667-676`, `dictation_capture.rs:229-242`). The cache/clamshell/fallback resolver is unchanged. Live switching from the UI was not exercised (see Manual). | PASS |
| C13 | `ARCHITECTURE.md`: the status paragraph says the desktop uses `audio` for capture and VAD; the "Pré-buffer de áudio" row cites door 1 (AC 13) | `grep -n "Pré-buffer" ARCHITECTURE.md` printed line 69 | `ARCHITECTURE.md:35` now lists "`audio` (captura do mic, pré-buffer e Silero v4 do ditado, pelo `dictation_capture.rs`)" and drops capture/VAD from the inherited list. `:69`: "300 ms enquanto o mic está aberto (always-on ou janela de `lazy_stream_close`); sob demanda, o mic abre na tecla", citing door 1 of F9a. No crate was created or renamed, so the Code Map table is unchanged. | PASS |
| C14 | no `VadBackendSelector` / `change_vad_backend_setting` left; a store with `"vad_backend": "earshot"` still loads (door 2) | `grep -rn "VadBackendSelector\|changeVadBackendSetting\|change_vad_backend_setting" src apps/desktop/src` printed nothing (exit 1). `settings::tests::earshot_store_still_loads ... ok` (batch) | `apps/desktop/src/settings.rs:2113-2128` parses a schema-2 store with `"vad_backend": "earshot"` and asserts `VadBackend::Earshot`. `create_audio_recorder` no longer reads `vad_backend` (`managers/audio.rs:272-307`). `src/bindings.ts` keeps only `vad_backend?: VadBackend` (`:1034`) and the type (`:1240`), which match the kept field. `VadBackendSelector.tsx` is deleted, its updater is gone from `settingsStore.ts`, and the `vadBackend` keys are gone from pt and en. | PASS |
| C15 | fmt, clippy, `cargo test -p fala --lib`, `cargo test -p fala-audio`, no-tauri, brand, `bun run lint`, `check:translations` exit 0 | see `## Gate` | `cargo test -p fala --lib`: `355 passed; 0 failed; 0 ignored`. Clippy printed only the two build-script notes (`Generated tray translations`, `Staged 13 transcribe-cpp runtime library file(s)`). | PASS (deny not run) |

## Correctness reading

- **Worker thread (`dictation_capture.rs:252-278`, `:334-368`).** The worker owns the `Mic`, so the cpal stream stays on the thread that built it. Each loop: wait up to 10 ms for a command, drain the ring into the `Processor`, copy `mic.failed()`, warn on ring drops only while recording, then apply the command. So samples delivered before `Start`/`Stop` are processed before the command (step 4 and 6 of the plan's Flow). `Stop` replies with `processor.stop()`, which flushes the resampler and the open utterance (`capture.rs:88-104`) and calls `audio_cb` for that last utterance on the worker before the reply. The `StreamRouter` therefore gets it before the manager's finalize. `Shutdown` or a dropped sender ends the loop. `close` sends `Shutdown` and joins (`:316-323`).
- **Readiness channel (`:246`, `:264`, `:280-294`).** `open` waits on a `sync_channel(1)`. An `open_device` error is returned and the worker is joined. "OK" is sent right after `Mic::open_device` (stream playing, ring filling), before `Processor::new`. See Finding 2 for the case where `Processor::new` then fails.
- **Stop/close semantics.** `stop` on a dead worker returns `Err` (send fails, or the reply sender is dropped). `stop_recording` logs it and continues with an empty buffer (`managers/audio.rs:918-924`), then returns to `Idle`, as before. `cancel_recording` discards `stop`'s result (`:982`).
- **Door 3, what `stop_recording` returns.** It was the concatenation of `SmoothedVad` speech frames (prefill, onset, hangover), 16 kHz mono (`git show eb0482c:.../recorder.rs`, `handle_frame` / `finish_recording`). It is now the concatenation of the session's utterances, including the pre-buffer when the stream was already open, 16 kHz mono (`Processor::keep`, `:160-172`). The padding of short audio (`managers/audio.rs:948-955`) and everything after `stop_recording` are unchanged. A session with no voice still returns empty, as before (C2's `Offline` half). Utterances cut at 15 s are contiguous (`capture.rs:134-136`, `replace(Vec::new())`), so the concatenation loses nothing at a cut.
- **Door 3, `StreamRouter`.** `router.feed` accepts any length (`managers/transcription.rs:164-171`), so whole utterances work. Before, a streaming model got a 30 ms frame at a time, with a 1 650 ms hangover. Now it gets each closed utterance, so a single utterance under 15 s reaches the streaming worker only at `stop`. The plan's Impact row states this, and the installed model is non-streaming. `VadPolicy::Streaming` is still chosen in `actions.rs:589-595`. `DictationVad` and `Processor` only compare against `Disabled` (`dictation_capture.rs:142-143`), so `Streaming` behaves exactly like `Offline`, as the doc comment says (`:28-29`).
- **`managers/audio.rs`.** Only the recorder type, `create_audio_recorder`, `preload_vad` and the removed `update_vad_backend` changed. Device resolution (`resolve_microphone_device`, cache, clamshell, fallback with `persist_default_microphone_after_fallback`), the retry after a failed open, mute handling, lazy close and the state machine are the same code as at `eb0482c`. `update_selected_channel` still sets the channel only while idle and closed, and `set_selected_channel` takes effect on the next `open` (`dictation_capture.rs:224-226`).
- **VAD preload (`lib.rs:207-216`).** It runs on its own thread and holds only the `recorder` mutex while loading. `start_microphone_stream` takes `is_open` and then `recorder`, so the order cannot invert. A press during the preload waits for it, which is what the first press did before. In always-on mode `AudioRecordingManager::new` already opened the stream (and loaded the VAD), so the thread is a no-op. A failed preload only warns, and the next start retries through `preload_vad()` (`managers/audio.rs:659-662`).
- **`crates/audio/src/mic.rs`.** `open_device` holds `ENV_OPEN` during the open, as `open` did. `find` now runs outside the lock. That is harmless, because `fala-cli meeting` opens the system audio and then the mic on one thread (`apps/cli/src/meeting.rs:57-58`). The `failed` flag is set on every cpal error, the same rule the inherited recorder used. `fala-cli` behaviour is unchanged (C10).
- **Earshot removal.** The selector, its updater and the binding are gone, the i18n keys are removed in both locales (`check:translations` reports 460 reference keys, PT complete), and the `vad_backend` field plus `VadBackend` enum stay for old stores (C14).
- **Repo invariants.**
  - No `tauri` in `crates/`: the crate diff adds no dependency, and the script reports ok.
  - No `cfg(target_os)` was added.
  - No `unwrap`/`expect` in new production code: `dictation_capture.rs` uses `unwrap_or`/`unwrap_or_else`, and the `unwrap`s are in its test module only.
  - Logs: `dictation_capture.rs` adds a `debug` timing (`:130-133`, `:267-270`) and `warn`/`error` lines with error text, and `mic.rs` adds a `debug` config/stream timing (`:98-101`). None logs audio or text.
- **Commits stand alone (read, not built per commit).**
  - `4aba4e7` touches only `crates/audio`, and its test is in the same commit.
  - `12fc982` adds the module, wires the manager, and removes `update_vad_backend` together with the `change_vad_backend_setting` command and its `invoke_handler` entry, so it compiles on its own. Its frontend still calls the command (Finding 4).
  - `857f3a9` removes the frontend and adds the store test.
  - Each body ends with `Assisted-by: Claude Code`, with no `Co-Authored-By` or `Signed-off-by`.

## Findings

None of these is blocking. None contradicts an acceptance criterion or a one-way door.

1. **Non-blocking - the VAD toggle's tooltip still describes the removed streaming profile.**
   - `src/i18n/locales/en/translation.json:412` says "Streaming-capable models use a longer VAD tail". `src/i18n/locales/pt/translation.json:412` says "Modelos com suporte a streaming usam uma cauda de VAD mais longa".
   - Door 3 made `Streaming` equal to `Offline`: there is one 450 ms hangover (`capture.rs:23`, `dictation_capture.rs:28-29`).
   - Scenario: someone with a streaming model reads the Advanced tab tooltip and expects a longer tail that no longer exists.
   - The fix is to drop the middle sentence in both locales.
2. **Non-blocking - a `Processor::new` failure after `open` has reported success surfaces as a misleading start error.**
   - The worker sends `Ok` (`dictation_capture.rs:264`) before `Processor::new` (`:265`). If `Processor::new` fails, it only logs `error!` and sets `failed` (`:273-276`), while `open` has already returned `Ok` and the manager has logged "Microphone stream initialized".
   - Usually the worker has exited by then. `try_start_recording` then fails with "Failed to start recorder: sending on a closed channel".
   - If `start` wins the race against the worker dropping `cmd_rx`, the `Start` is queued and dropped. The state goes to `Recording`, readiness never fires (`actions.rs:633` only logs at debug), and `stop` returns `Err`, which becomes an empty recording.
   - Recovery is correct: the next press sees `needs_reopen()` (`failed`) and rebuilds.
   - It is reachable only when `DictationCapture::new` → `Resampler::new` rejects the device rate (`crates/audio/src/resample.rs:31-38`). Then every press fails the same way, and the cause is only in the log.
   - Sending `init_tx` after `Processor::new`, or sending its error through `init_tx`, would surface it from `open`. That costs the ~1-2 ms the executor measured for `processor`.
3. **Non-blocking - the channel count shown in settings can differ from the one opened, for inputs whose default config is not f32.**
   - `input_channel_count` reports `default_input_config().channels()` (`dictation_capture.rs:37-42`). When the default is not f32, `Mic::open_device` opens the first f32 range at the default rate (`crates/audio/src/mic.rs:57-73`), whose channel count can differ.
   - Scenario: an ALSA `hw:` input with an i16 stereo default and a mono f32 range. The UI offers channel 2, and `open_device` silently drops it to the average (`:81`).
   - WASAPI shared-mode defaults are f32, so the phase-1 target is unaffected. The plan already accepts that non-f32 inputs may fail to open.
4. **Non-blocking - `12fc982` alone leaves the Earshot dropdown calling a removed command.**
   - `12fc982` removes `change_vad_backend_setting` from `shortcut/mod.rs` and `lib.rs`. `src/bindings.ts`, `settingsStore.ts` and `VadBackendSelector.tsx` still call it until `857f3a9`.
   - At `12fc982`, picking a backend in Advanced gets an invoke error, a toast and a rollback. Both sides still build, and the stack lands together.
   - Moving the command removal into `857f3a9`, or the frontend removal into `12fc982`, would make each commit stand alone in behaviour too.

## Manual (outside the verdict)

Facts below are the **executor's** (Windows 11 Alienware, on AC, 2026-10-09 ~23:20-23:50 UTC), copied from their evidence notes. The Verifier did not re-run them.

- **Setup.**
  - `bun run tauri dev` with `CARGO_TARGET_DIR=C:\f\pipe` (debug build) and the `portable` marker, so the store is in `C:\f\pipe\debug\Data`.
  - Store values: model `parakeet-tdt-0.6b-v3-Q8_0.gguf`, `llm_enabled=false`, `transcribe = ctrl+shift+space`, `push_to_talk_double_tap`.
  - Driver: a PowerShell script holds the key via `keybd_event` while the pt-BR TTS voice speaks, then reads Notepad through UI Automation.
  - The machine was loaded by sibling panes compiling.
- **C12, device selection through `Mic::open_device` (exact name).**
  - Default input gave `microfone: Voicemeeter Out B2 (VB-Audio Voicemeeter VAIO), 48000 Hz, 2 canal(is)`. It was a silent bus, so `sample count: 0` and the recording was skipped (correct VAD result).
  - `Microfone (Realtek(R) Audio)` gave `microfone: Microfone (Realtek(R) Audio), 48000 Hz, 2 canal(is)`, with 5/5 transcribed and pasted. The text was poor because the mic's echo cancellation removed most of the TTS.
  - `Voicemeeter Out B1` was used for the clean runs.
  - The device was changed in the store between restarts. **Live switching from the settings UI (`update_selected_device`) was not exercised**, so the C12 step "change the mic in settings and dictate again" is only partly covered.
- **C12, 5 dictations into Notepad, on-demand mic (23:44:46-23:45:24).**
  - Notepad read back: `Olá, este é um teste do ditado do Fala.O microfone abre quando a tecla desce.Hoje vamos revisar o plano da semana.A reunião de amanhã começa às 10 horas.Obrigado. Até logo!` (5/5, history `app = notepad`).
  - `dictation capture: mic open` 51.5 / 29.5 / 13.9 / 10.7 / 13.7 ms; `processor` 1.1-2.0 ms.
  - `first samples ... after Start` 56.7 / 88.2 / 31.6 / 20.7 / 31.3 ms; sample counts 63840 / 54720 / 58560 / 62400 / 43680.
  - Quieter earlier run (23:39:03-23:39:38, 5/5 correct, pasted into the Fala window): mic open 23.8 / 15.0 / 9.0 / 9.2 / 7.4 ms, first samples 35.3 / 21.3 / 31.4 / 20.8 / 20.8 ms.
- **VAD preload.** Before the preload, the first press logged `vad_ensure=212.7ms`. After it, `Initialized Silero VAD from fala-audio` appears at startup and every press shows `vad_ensure` ≤ 1.7 µs.
- **Door 1, speech that starts before the key** (phrase "Bom dia, equipe de vendas, vamos começar.").
  - On demand, key 450 ms after speech starts: "Bom dia" kept 2/3.
  - On demand, key 700 ms after: "Bom dia" lost 3/3.
  - `always_on_microphone = true`, key 700 ms after: kept 3/3. `Recording request accepted in` 30-38 µs, first samples 14.0 / 23.3 / 29.0 ms after Start, 70080 samples each.
- **Lost pastes in earlier driver attempts** were caused by the driver itself, according to the executor: Alt key tips, Esc firing cancel, and presses sent while the ASR was busy.
- **Not done by the executor:** release-build timings (all numbers are debug on a loaded machine; the formal budget is F9c, `FALA_TRACE`), a live device switch from the UI, the Linux build and clippy (CI), and `cargo deny`.
- **Verifier observation on door 1** (not a finding). The plan's ≤ 50 ms rationale cites the installed release build. In the executor's debug evidence, mic open plus first samples is 28-59 ms in the quiet run (first press 23.8 + 35.3 ms) and up to 51.5 + 56.7 ms on the loaded run. These numbers neither confirm nor refute the budget. F9c should measure it on a release build.

Open checklist for the maintainer session:

- [ ] C12 remainder: change the microphone from the settings UI while the app runs, then dictate again (`update_selected_device`).
- [ ] Always-on mode in the real app over a longer idle period (the `checks.md` Coverage table lists it as not exercised; the executor's always-on run covered 3 dictations).
- [ ] Door 1 latency on a release build (F9c).

## Gate

| Command | Result |
| --- | --- |
| `cargo test -p fala --lib -- --exact <7 check tests>` | exit 0; 7 passed, 0 failed, 348 filtered out |
| `cargo test -p fala-audio mic::tests::mono_picks_channel_or_averages -- --exact` | exit 0; 1 passed, 23 filtered out |
| `cargo test -p fala-audio mic::tests` | exit 0; 1 passed, 23 filtered out |
| `cargo test -p fala-audio` | exit 0; lib 24, manifest 1, meeting_shape 1, silero 1 (+1 ignored); 0 failed |
| `cargo test -p fala-cli` | exit 0; 114 passed, 0 failed, 29 ignored |
| `cargo test -p fala --lib` | exit 0; 355 passed, 0 failed, 0 ignored |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0; only the two build-script notes, no lint |
| `bash scripts/check-no-tauri-in-crates.sh` | exit 0, `ok: no tauri in crates/` |
| `bash scripts/check-brand.sh` | exit 0, `ok: no Handy branding outside the allowlist` |
| `bun run lint` | exit 0 (`eslint src`, no output) |
| `bun run check:translations` | exit 0; `Reference has 460 keys`, `PT: All keys present` |
| `bun run format:check` (extra) | exit 0; Prettier clean, then `cargo fmt --check` clean |
| `grep -c "SileroVad::load" ...` (C7) | `0` + `1` = 1 |
| C11 grep / C14 grep | no output (exit 1) |
| `cargo deny check` | not run (not installed on this machine; CI) |
| C12 | not run by the Verifier (manual; executor's evidence above) |

## Executor follow-up (after the verdict, not re-verified)

Written by the executor, not the Verifier. The verdict above is for `857f3a9`. After it, the stack was rewritten into two commits (`feat(audio)` and `feat(desktop)`), and the changes below were made. Then `cargo fmt --all -- --check`, `cargo clippy -p fala --all-targets -- -D warnings`, `cargo test -p fala --lib` (355 passed) and `bun run check:translations` were re-run, and all four exited 0.

- Finding 1 (fixed): the VAD toggle's description in both locales drops the sentence about a longer streaming tail.
- Finding 2 (fixed): the worker builds the `Processor` before answering `open`, and sends its error through `init_tx`. A resampler that rejects the device rate now fails `open` with the reason, instead of failing the next start.
- Finding 3 (accepted): non-f32 inputs are already out of the phase-1 target (plan, door 4 and Impact). The channel count shown in the UI stays the one from the default config.
- Finding 4 (fixed): the Earshot selector removal is folded into the `feat(desktop)` commit, so the command and its only callers disappear together.
