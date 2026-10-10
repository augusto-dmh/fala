# upstream-catchup PR 5 (chime output stream) verification

**Verdict**: PASS
**Profile**: light
**Diff range**: 07d2aa3..79572c6
**Round**: 2 - full
**Verifier**: independent sub-agent (author != verifier)

Round 1 (below, kept as written) verified `eb0482c..0a68490` and returned FAIL. Round 2, at the end of this file, is a full pass over `07d2aa3..79572c6` and carries the current verdict.

## Round 1 - full (eb0482c..0a68490, verdict FAIL, superseded)

## Binding sources

No binding sources: `checks-pr5-chime-stream.md` says `Plan: none`, and profile light does not run step 1. The upstream issue cjpais/Handy#1712 (body and comments read with `gh api`) was the context for the judgment questions and for recomputing the failure-mode set under Coverage.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | `get_or_open` reuses on the same key, reopens when the key changes and after `close()`; 3 opens in 4 calls | `cargo test -p fala --lib -- audio_feedback::tests` exit 0; `chime_stream_is_reused_while_the_device_stays_the_same ... ok` | `apps/desktop/src/audio_feedback.rs:313-314` - `assert_eq!(output.get_or_open("default:Speakers", open), Ok(&1)); assert_eq!(opened.get(), 1);` · `:316` - `get_or_open("default:Headphones", open), Ok(&2)` · `:320-321` - `Ok(&3)` and `assert_eq!(opened.get(), 3);` | PASS |
| C2 | a failed open returns the error, caches nothing, and the next call opens again | same invocation; `chime_stream_that_fails_to_open_is_retried_next_time ... ok` | `apps/desktop/src/audio_feedback.rs:327-331` - `assert_eq!(output.get_or_open("default:Speakers", ...Err("busy"...)), Err("busy".to_string())); assert!(!output.is_open());` · `:332-335` - `Ok(&7)` | PASS |
| C3 | `wait_for_chime` returns false in under 1 s on a 50 ms cap, true with a reply; `CHIME_WAIT` = 3 s | same invocation; `blocking_chime_wait_is_bounded ... ok`; `grep -q 'const CHIME_WAIT: Duration = Duration::from_secs(3);'` exit 0 | `apps/desktop/src/audio_feedback.rs:342-345` - `assert!(!wait_for_chime(&done_rx, Duration::from_millis(50))); assert!(started.elapsed() < Duration::from_secs(1)); ... assert!(wait_for_chime(&done_rx, Duration::from_millis(50)));` · const at `:20`, used by the only blocking path at `:103-104` | PASS |
| C4 | only the worker opens an output stream; it closes after 30 s idle and after a failed `play_chime` | structural grep/awk proof from checks, exit 0 | `apps/desktop/src/audio_feedback.rs:234` - sole call of `open_output_stream(request.device.clone())`, passed as the opener closure of `output.get_or_open(&key, ...)` inside `play_chime` (definition at `:262`) · `:17` - `const CHIME_STREAM_IDLE: Duration = Duration::from_secs(30);` · `:203-209` - `recv_timeout(CHIME_STREAM_IDLE)` only when `output.is_open()`, `Timeout => output.close()` · `:219-221` - `Err(e) => ... output.close()` | PASS |

## Manual proofs not run

| Check | Claim | Why not run | Status |
| --- | --- | --- | --- |
| C5 | Windows build, feedback on, music playing: 30 dictations, 60/60 chimes, no stuck pill; switching the default output mid-run moves the next chime | `TODO(windows)` manual proof; this run is on Linux (Ubuntu 25.04) | not run (manual, Windows) |

## Coverage

Round-1 table kept verbatim inside a fence: its Unproven member was closed in round 2 (see Round 2, Round-1 findings, 1), and the fence keeps the gate reading round 2's table instead of this superseded one.

```text
| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| reasons to reopen the stream (4, as authored) | `audio_feedback.rs:176-195`, `:200-227` | no stream C1 · key changed C1 · `play_chime` returned `Err` C2, C4 · idle 30 s C4 | - |
| blocking-wait outcomes (2) | `audio_feedback.rs:101-112`, `:155-157` | finished C3 · timed out C3 (`warn!` at `:106`) | - |
| callers of `audio_feedback` (4) | `rg` over `apps/desktop/src`: `actions.rs:665` (start, blocking, then `apply_mute` at `:668`), `actions.rs:743` (stop, async), `commands/audio.rs:304` (test sound, blocking), `transcription_coordinator.rs:922` (limit warning, 2x blocking on its own thread) | all route through `send_chime` (`:98`, `:103`); blocking ones through `wait_for_chime` with `CHIME_WAIT` (C3, C4) | - |
| output-stream failure modes (from cjpais/Handy#1712) | issue body, Defect A: "the stream can stall with a frozen audio clock and no error - `sleep_until_end()` then never returns" | open fails C2 · play returns `Err` C2, C4 · default device switched C1 (key `default:<name>`, `:248-260`) | stall with no error: `sink.sleep_until_end()` at `audio_feedback.rs:241` has no bound on the single worker, so the worker never returns, never closes or reopens, and every later chime is lost until restart; no check covers it |
```

## Faults injected

None - profile light (step 4 runs only under standard and ui).

## Swept re-read

Every `Swept` row cites a check or `n/a`; none resolves to "existing", so there is no cited constraint to re-read. `dependency failure: C2, C3 (dispositivo recusa ou emperra)` claims the wedged-device case; C3 bounds only the caller's wait, not the worker (see Findings 1).

## Gate

`cargo test -p fala --lib -- audio_feedback::tests` at `0a68490` (clean tree, `CARGO_BUILD_JOBS=1`) - 3 passed, 0 failed, 349 filtered out, each of the 3 named tests listed `... ok` individually; grep proofs C3 and C4 exit 0 at the same HEAD. The structural gate exits 1 on the FAIL verdict alone. On a scratch copy with the verdict flipped to PASS, its only error is the Unproven Coverage cell, so the rest of the structure is valid.

## Findings

1. **Blocker (coverage, the issue's own failure mode).** The single `chimes` worker calls `sink.sleep_until_end()` (`apps/desktop/src/audio_feedback.rs:241`) with no bound. #1712 names exactly this as Defect A: after a device state change the render stream freezes with no error and `sleep_until_end()` never returns. Before this PR each chime had its own thread, so a stall cost one thread and the next chime tried a fresh stream. Now one stall wedges the only worker for good. Every later chime is lost until the app restarts, every start chime adds a 3 s wait before `apply_mute` (`actions.rs:665-668`), and requests pile up in the unbounded channel, then all play late if the worker ever wakes. The upstream fix bounds the play, drops a suspect stream on a throwaway thread and recreates it. The caller-side bound (C3) holds; recovery of the worker is missing and nothing checks it.
2. **Major (worker death is silent and permanent).** `panic = "unwind"` (`Cargo.toml:72`). If anything inside `play_chime` panics (decoder on a bad custom WAV, cpal/rodio), the worker thread ends, `rx` drops, `tx.send` fails, and `send_chime` returns `false` (`:149`). `play_sound_blocking` then skips the wait and the `warn!` (`:103-104`, short-circuit), so every later chime is dropped with no log line. The `OnceLock` (`:134`) never respawns the worker. The same goes for a failed `thread::Builder::spawn` (one `error!`, then `None` forever).
3. **Major, unverified on Linux (cached stream after unplug).** rodio's default error callback only prints to stderr (`rodio fed3029 src/stream.rs:98-103`), so a device-lost error never reaches `play_chime` as `Err` and never closes the cache. For an explicit device the key is `device:<name>` (`:250`), which does not change when that device is unplugged, so chimes within 30 s of each other keep going to the dead stream (silent, or a stall as in Finding 1). Before this PR the next chime would re-enumerate and fall back to the default (`:284-285`). The default device is covered by the key re-read. C5 does not exercise unplugging.
4. **Minor (behaviour change for long custom sounds).** The start chime used to block for its full length before mute. Now the wait stops at 3 s, so a custom `custom_start.wav` longer than 3 s is muted mid-play and logs a misleading "did not finish" warning. Built-in themes are short.
5. **Minor.** On a key change the old stream is dropped only after the new one opens (the scrutinee temporary in `get_or_open`, `:177-183`), so two output streams briefly coexist. Closing also drops the stream on the worker itself (`:208`, `:221`); upstream drops suspect streams on a throwaway thread because a wedged WASAPI stream's drop can block.
6. **Confirmed fine.** The `OutputStream` is created (`:234`, `:262`), held (`:201`, a local of `chime_worker`) and dropped only on the `chimes` thread. The idle close is real: `recv_timeout` runs only while `output.is_open()`, otherwise a blocking `recv` (`:203-218`). Volume and device are read per request (`:122-130`), so a settings change between chimes takes effect (the device through the key). The limit warning's two chimes 150 ms apart are kept (`:88-95`). No `unwrap`/`expect` was added outside tests (the only diff hit is `unwrap_or_default` at `:256`).

## Round 2 - full

Verified at `79572c6` (detached worktree, clean tree), diff `07d2aa3..79572c6`: `apps/desktop/src/audio_feedback.rs` (+518/-26) and `checks-pr5-chime-stream.md`. The code was largely rewritten, so every check C1-C10 was re-run and re-cited; nothing is carried from round 1. Line numbers below are `apps/desktop/src/audio_feedback.rs` at `79572c6` unless another file is named.

### Binding sources (round 2)

No binding sources: `Plan: none`, profile light does not run step 1.

### Checks (round 2)

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | `get_or_open` opens once for two calls on the same key, reopens on a key change and after `close()`; 3 opens in 4 calls | `cargo test -p fala --lib -- audio_feedback::tests` exit 0; `chime_stream_is_reused_while_the_device_stays_the_same ... ok` | `apps/desktop/src/audio_feedback.rs:469-471` - `get_or_open("default:Speakers", open), Ok(&1)` twice, then `assert_eq!(opened.get(), 1);` · `:473` - `get_or_open("default:Headphones", open), Ok(&2)` · `:476-478` - `assert!(!output.is_open());` then `Ok(&3)` and `assert_eq!(opened.get(), 3);` | PASS |
| C2 | a failed open returns the error, caches nothing, the next call opens again | same invocation; `chime_stream_that_fails_to_open_is_retried_next_time ... ok` | `apps/desktop/src/audio_feedback.rs:484-488` - `assert_eq!(output.get_or_open("default:Speakers", ...Err("busy"...)), Err("busy".to_string())); assert!(!output.is_open());` · `:489-492` - `Ok(&7)` | PASS |
| C3 | no start: `false` in under 1 s on a 50 ms start wait; sender dropped: `true` at once; a 100 ms chime that never ends is abandoned between 1 s and 2 s | same invocation; `blocking_chime_wait_is_bounded ... ok` | `apps/desktop/src/audio_feedback.rs:500-501` - `assert!(!wait_for_chime(&started_rx, Duration::from_millis(50))); assert!(begun.elapsed() < Duration::from_secs(1));` · `:503-504` - `drop(started_tx); assert!(wait_for_chime(...));` · `:509-512` - `assert!(!wait_for_chime(...)); ... waited >= 1_000 ms ... waited < 2_000 ms` · production: `CHIME_START_WAIT` 2 s at `:21`, used at `:122`; two-phase wait `:163-172` | PASS |
| C4 | only the playback thread opens a stream (`open_output_stream` called only by `play_chime`); the worker closes after `CHIME_STREAM_IDLE = 30 s` and after a failed `play_chime`; no `sleep_until_end` | structural proof from checks run verbatim at `79572c6`, exit 0 | `apps/desktop/src/audio_feedback.rs:371` - sole call `open_output_stream(request.device.clone())`, the opener closure of `output.get_or_open(&key, ...)` inside `play_chime` (definition `:407`, the 2 grep hits) · `:18` - `const CHIME_STREAM_IDLE: Duration = Duration::from_secs(30);` · `:336-338` - `Err(RecvTimeoutError::Timeout) => { ... output.close();` · `:350-352` - `if let Err(e) = play_chime(...) { ... output.close(); }` · `:383-387` - playback polled with `wait_until` on `sink.empty()` against `limit`, then `sink.stop()` and `Err` on timeout | PASS |
| C6 | a 5 s start sound that ends at 300 ms counts as finished; `play_limit` is 6 s for 5 s, `CHIME_PLAY_MAX` (10 s) for 60 s and for unknown | same invocation; `a_long_start_sound_is_waited_out_before_mute ... ok` | `apps/desktop/src/audio_feedback.rs:526` - `assert!(wait_for_chime(&started_rx, Duration::from_millis(500)));` · `:528-533` - `play_limit(Some(5 s)) == 6 s`, `play_limit(Some(60 s)) == CHIME_PLAY_MAX`, `play_limit(None) == CHIME_PLAY_MAX` · production `:155-159`, `CHIME_PLAY_MAX` 10 s at `:24` | PASS |
| C7 | `wait_until` with an always-false condition and an 80 ms limit returns `false` between 80 ms and 1 s; true on the 3rd poll returns `true` | same invocation; `frozen_playback_is_abandoned_at_its_limit ... ok` | `apps/desktop/src/audio_feedback.rs:539-545` - `assert!(!wait_until(` an always-false closure, 80 ms limit, 10 ms poll `));` ... assert!(waited >= Duration::from_millis(80) && waited < Duration::from_secs(1));` · `:547-554` - `assert!(wait_until(... polls.get() >= 3 ...))` · used by `play_chime` at `:384` | PASS |
| C8 | a dead playback thread (receiver dropped) is replaced: the chime reaches the new thread, `spawn` runs once | same invocation; `a_dead_playback_thread_is_replaced ... ok` | `apps/desktop/src/audio_feedback.rs:583-585` - `assert!(player.send(7, 1_000, ...)); assert_eq!(fresh_rx.try_recv(), Ok(7)); assert_eq!(spawned.get(), 1);` · production `:242-250` (`SendError(unsent)` gives the request back, `current = None`, second attempt spawns) | PASS |
| C9 | busy 5 s (limit 15 s) keeps the chime with no spawn; busy 20 s sends it to a new thread and the stuck one gets nothing | same invocation; `a_stuck_playback_thread_is_replaced_and_a_busy_one_kept ... ok` | `apps/desktop/src/audio_feedback.rs:598-599` - `assert!(player.send(1, 15_000, stall, no_spawn)); assert_eq!(busy_rx.try_recv(), Ok(1));` · `:609-611` - `assert!(player.send(2, 30_000, stall, spawn)); assert_eq!(fresh_rx.try_recv(), Ok(2)); assert!(busy_rx.try_recv().is_err());` · production `:218-226` | PASS |
| C10 | a failed spawn makes `send` return `false` and both callers log `was not played`; the stream error callback sets `failed` and `play_chime` closes a flagged stream before reuse | same invocation; `a_thread_that_cannot_start_reports_the_chime_lost ... ok`; structural proof run verbatim, exit 0 | `apps/desktop/src/audio_feedback.rs:619` - `assert!(!player.send(1, 1_000, Duration::from_secs(15), spawn));` · `:105-110` and `:115-120` - `if !send_chime(...) { warn!("Sound '{}' was not played: no playback thread", ...)` (both callers, 2 grep hits) · `:442-445` - `.with_error_callback(` a closure that logs and runs `flag.store(true, Ordering::SeqCst);`, wired to cpal in rodio `fed3029 src/stream.rs:277-292` · `:363-369` - `chime.failed.load(Ordering::SeqCst)` then `output.close()` before `get_or_open` | PASS |

### Manual proofs not run (round 2)

| Check | Claim | Why not run | Status |
| --- | --- | --- | --- |
| C5 | Windows build, feedback on, music playing: 30 dictations in a row, 60/60 chimes, 30/30 dictations, no stuck pill; switching the default output between headphones and speakers mid-run moves the next chime; a USB headset picked by name, unplugged mid-run, sends the next chime to the default | `TODO(windows)` manual proof; this run is on Linux (Ubuntu 25.04), and the frozen-WASAPI-stream and device-invalidated paths only exist on Windows | not run (manual, Windows) |

### Coverage (round 2)

Recomputed from the code at `79572c6` (light does not require this section; kept because round 1 had one).

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| reasons to reopen the stream (5) | `audio_feedback.rs:293-305`, `:330-357`, `:363-369` | no stream C1 · key changed C1 · stream error flag C10 · failed play (open, decode or timeout) C2, C4 · idle 30 s C4 | - |
| blocking-wait outcomes (4) | `wait_for_chime` `:163-172` | never started C3 · finished or failed (sender dropped) C3 · started, overran C3 · long sound that ends C6 | - |
| playback-thread states `send` distinguishes (5) | `ChimePlayer::send` `:211-254` | idle (`busy_since` 0) and busy within bound C9 · busy past bound C9 · receiver gone C8 · spawn fails C10 | - |
| callers that play a chime (4) | `rg` over `apps/desktop/src`: `actions.rs:665` (start, blocking, then `apply_mute` at `:668`), `actions.rs:743` (stop, async), `commands/audio.rs:304` (test sound, blocking), `transcription_coordinator.rs:922` (limit warning, 2x blocking on its own thread) | async via `play_sound_async` `:104-111` C10 · blocking via `play_sound_blocking` `:113-128` C3, C6, C10 · all through `send_chime` `:260-263` | - |
| output-stream failure modes (from cjpais/Handy#1712 and round 1) | issue Defect A, round-1 findings 1-3 | open fails C2 · decode or file error C4 (close on `Err`) · frozen stream, no error C4 (no `sleep_until_end`), C7 (bounded poll) · worker stuck past bound C9 · worker dead C8 · device invalidated C10 (flag), C5 (manual) · default switched C1 | - |

### Faults injected (round 2)

None - profile light (step 4 runs only under standard and ui).

### Swept re-read (round 2)

Every `Swept` row cites checks or `n/a`; none resolves to "existing". `concurrency: C4 (uma thread só abre e toca)` is now looser than the code: a replaced thread can still be alive with its own stream, so "one thread" holds per live `ChimePlayer.current`, not per process (see New findings, D). Not a contradiction with any check's claim.

### Judgment questions (round 2)

- **Can `ChimePlayer::send` lose or duplicate a chime?** No duplication: the request moves into `tx.send` and comes back only through `SendError(unsent)` (`:242-250`), so it is delivered at most once. Losses are bounded and visible: a failed spawn returns `false` and both callers `warn!` (`:105-110`, `:115-120`) after the `error!` at `:234`; a second dead receiver in a row returns `false` (`:253`), also logged. The one silent loss is by design: chimes sent to a stuck thread in the first 15 s of a stall sit in its queue (blocking callers time out after 2 s with a `warn!`), and play late if that thread ever unwedges (D).
- **Can stall detection misfire on a normal long chime?** Not on a healthy clock. `busy_since` is set after the request is received (`:349`) and cleared after it (`:354`), so queueing time and the caller's 2 s start wait never count. A healthy chime is busy for key lookup + open + decode + at most `CHIME_PLAY_MAX` 10 s (`:383-384`) + the close on a timeout, which leaves about 5 s of margin under `CHIME_STALL` 15 s (`:27`, strict `>` at `:220`). The clock is wall time, though (B).
- **Is the `busy_since` handoff race-free enough?** Yes. The load (`:219`) and the send (`:242`) are not atomic together, but every interleaving is benign: the worker about to store a timestamp is seen as idle and the chime queues behind it; a worker that unwedges just after being replaced drains its already-buffered requests and exits on `Disconnected` (`:341`, `:346`). A worker that panics leaves `busy_since` set and its receiver dropped, and either branch (stall or `SendError`) replaces it.
- **Does an abandoned thread hold anything that blocks the new one?** No shared lock. `CHIME_PLAYER` is locked only inside `send_chime` (`:261-262`), around `spawn` and an unbounded, non-blocking `Sender::send`; the worker never touches it. Each thread owns its `Receiver`, its own `busy_since` `Arc` (`:266-268`) and its own `CachedOutput` (`:331`). Poisoning is recovered with `unwrap_or_else(|e| e.into_inner())` (`:261`). The only shared thing is the OS audio engine itself.
- **Is the start chime -> mute sequence in `actions.rs` still correct?** Yes; `actions.rs` is not in the diff. `actions.rs:664-669`: readiness check, `play_feedback_sound_blocking`, readiness check, `apply_mute`. The blocking call now returns after at most `CHIME_START_WAIT` + `play_limit` (2 s + 10 s), normally the sound's length; a stale recording skips the mute.
- **New `unwrap`/`expect` outside tests?** None. grepping the added lines of `git diff 07d2aa3..79572c6 -- apps/desktop/src/audio_feedback.rs` for `unwrap` or `expect` as whole words matches nothing; the remaining forms are `unwrap_or_else(|e| e.into_inner())` (`:261`) and `unwrap_or_default` (`:401`).

### Round-1 findings status

| Round-1 finding | Status | Evidence |
| --- | --- | --- |
| 1 Blocker - unbounded `sleep_until_end` wedges the only worker | closed | playback polled against `play_limit` (`:383-387`), no `sleep_until_end` (C4), proven bound C7; a thread busy past `CHIME_STALL` is replaced on the next chime (`:218-226`, C9). Chimes in the first 15 s of a stall are still lost (documented trade-off in checks Decisions) |
| 2 Major - worker death silent and permanent | closed | dead receiver respawns once (`:242-250`, C8); failed spawn logs `error!` (`:234`) and both callers `warn!` (`:105-110`, `:115-120`, C10); no `OnceLock` any more, the player respawns on every later chime |
| 3 Major - cached stream after unplug never reopens | closed in code, symptom unverified | error callback sets `failed` (`:442-445`), `play_chime` closes before reuse (`:363-369`), reopen falls back to the default when the name is gone (`:426-431`), C10; that cpal WASAPI raises the callback on invalidation is covered only by manual C5 (not run). Residual: see E |
| 4 Minor - 3 s cap mutes a long custom start sound | closed up to 9 s | two-phase wait (`:163-172`, C6); sounds longer than 9 s now cut by the worker (C) |
| 5 Minor - new stream opened before the stale one drops; drops on the worker | partly closed | stale stream dropped before `open()` (`:294-300`; readable in code, no test asserts the order). Drops still run on the playback thread; inside `play_chime` they are within the stall window, the idle close is not (A) |
| 6 Confirmed fine | still holds | stream created, held and dropped on its own thread (`:331`, `:371`); settings read per chime (`:139-151`); limit warning 150 ms apart kept (`:95-101`) |

### New findings (round 2)

A. **Minor, unverified (Windows) - a drop that blocks during the idle close escapes the stall detector.** The idle close (`:336-339`) drops the stream while `busy_since` is 0. If that drop blocks on a frozen WASAPI stream (the reason round-1 finding 5 cited for upstream dropping suspect streams elsewhere), the thread stays alive and never looks busy, so `:219-220` never fires, `tx.send` keeps succeeding, every later chime is queued forever, and each blocking caller waits 2 s and logs. Same exposure as round-1 finding 1 on a narrower path. Setting `busy_since` around the idle close (`:338`) would bring it under C9. Whether cpal's WASAPI `Stream` drop can block was not verified here (its drop signals the stream thread and joins it).

B. **Minor - stall clock is wall time.** `now_ms` uses `SystemTime` (`:186-190`). A forward jump (NTP step, resume from suspend mid-chime) can make a healthy thread look stuck and spawn a second one; the effect is benign (the old thread drains its queue and exits on `Disconnected`) but two streams coexist briefly and chimes can overlap. A backward jump delays detection by the jump size (`saturating_sub`, `:220`). A monotonic offset from a process-start `Instant` would avoid both.

C. **Minor - long custom sounds are cut at 10 s and logged as failures.** `play_limit` caps playback at `CHIME_PLAY_MAX` (`:24`, `:155-159`); a custom sound over 9 s is stopped by the worker (`:385`), logged at `error!` ("Failed to play sound", `:351`) and closes the stream. The cap is stated in the checks' Intent, so this is the misleading error-level log, not the cap.

D. **Minor - stuck threads accumulate and play stale chimes late.** While the engine stays wedged, each new thread can freeze too, so one thread (with its stream) leaks per 15 s of chime activity; a stuck thread that unwedges plays its queued chimes (up to 10 s each) out of context, possibly a start chime from an earlier dictation. Documented in part ("one rare thread leak").

E. **Minor - after the named-device fallback the key stays `device:<name>`.** When an unplugged device is reopened through the default fallback (`:426-431`), the cache key is still `device:<name>` (`:395`), so replugging the device does not move chimes back until the stream closes (30 s idle, an error or a failed play).

F. **Test precision notes (not failures).** C6's `wait_for_chime` assertion (`:526`, 300 ms end under a 500 ms start wait) would also pass under the old fixed 3 s cap; only the `play_limit` assertions (`:528-533`) discriminate. No test asserts that the stale stream drops before the new one opens (round-1 finding 5); it is readable at `:294-300`.

### Gate (round 2)

`cargo test -p fala --lib -- audio_feedback::tests` at `79572c6` (clean tree, `CARGO_BUILD_JOBS=1`, shared target) - 8 passed, 0 failed, 349 filtered out, each of the 8 named tests listed `... ok` individually: `chime_stream_is_reused_while_the_device_stays_the_same`, `chime_stream_that_fails_to_open_is_retried_next_time`, `blocking_chime_wait_is_bounded`, `a_long_start_sound_is_waited_out_before_mute`, `frozen_playback_is_abandoned_at_its_limit`, `a_dead_playback_thread_is_replaced`, `a_stuck_playback_thread_is_replaced_and_a_busy_one_kept`, `a_thread_that_cannot_start_reports_the_chime_lost`. Structural proofs C4 and C10 exit 0 at the same HEAD. Structural gate: `validate_verification.py x --root <gate copy>` on a copy of the checks and this report - 0 errors, 0 warnings, exit 0.
