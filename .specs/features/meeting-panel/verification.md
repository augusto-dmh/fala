# meeting-panel verification

**Verdict**: FAIL
**Profile**: light
**Diff range**: eb0482c..d4e2448 (round 2 ran at 5a4e89e. The stack was autosquashed again; 5a4e89e is still in the object store, so the round-3 fix diff was read as `git diff 5a4e89e d4e2448`.)
**Round**: 3 - scoped
**Verifier**: independent sub-agent (author != verifier)

39 of 43 checks are proven with located evidence at `d4e2448`. These four are not:

- **C14 and C22.** The only proof is manual (`tauri dev`, a click, real room audio), and it did not run.
- **C17.** The tray-decision half is automated and passes. The window and page half is manual and did not run.
- **C43 (new in this round).** Two of its three clauses hold. The third, "a capture that fails removes its empty `audio/<id>/`", holds only on the `recorder::spawn` failure branch. `SystemAudio::default_name()` failing returns the same `audio_device` error after `create_dir_all` and leaves the empty folder (F6 below).

Round 2's other gap is closed. C42 was reworded to the mechanism, and F8 is fixed: App.tsx no longer arms the flag while the page is open, so only one listener reacts to the event.

## Scope of this round

The fix diff (`5a4e89e..d4e2448`) touched:

- `apps/desktop/src/lib.rs`: the tray `meeting_pause`/`meeting_resume`/`meeting_stop` arm now runs on `std::thread::spawn`.
- `apps/desktop/src/managers/audio.rs`: `update_mode` skips the open while a meeting holds the mic.
- `apps/desktop/src/meeting/mod.rs`: `remove_dir(&dir)` on the `recorder::spawn` failure branch. Every line after 471 moved by +3.
- `src/App.tsx`: `sectionRef` guard. `src/components/meeting/MeetingsPage.tsx`: the listener no longer calls `takeConsentRequest`. Lines after 92 moved by -3.
- `src/components/meeting/meeting.test.tsx`: the C42 block only (lines 340-365).
- `.specs/features/meeting-panel/proofs/round1_fixes.py`: the C43 block. `checks.md`: C42 reworded, C43 added.

Re-judged in full (verified at d4e2448):

- the four non-PASS checks from round 2: C14, C17, C22, C42
- the new C43
- every check whose evidence or proof reads a touched file: C10, C11, C13, C18, C19, C26, C38 (`mod.rs`, `lib.rs`, `audio.rs`), C25, C27, C33, C36 (`meeting.test.tsx`, `MeetingsPage.tsx`), C34 (log lines in `lib.rs`), C39-C41 (`round1_fixes.py`, `mod.rs`, `audio.rs`), C12 (`meeting.test.tsx`)

Checks whose files show an empty `git diff 5a4e89e d4e2448` keep round 2's citations, marked `carried from 5a4e89e`:

- `crates/*`
- `apps/desktop/src/{overlay,tray,actions}.rs`, `meeting/{recorder,pipeline}.rs`, `commands/`
- `src/overlay/`

Every proof below was re-run at `d4e2448`, carried checks included.

Under `light` there is no binding-source step, no `Coverage` recompute, no `Test policy` verdict and no fault injection.

## How the proofs ran (verified at d4e2448)

Every cargo command ran as `flock -w 7200 /home/augusto/projects/fala/target/.builder.lock env CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2 cargo ...`, with `CARGO_BUILD_JOBS=1` for `-p fala`, one at a time. `free -m` showed 3.2-3.5 GB available before each, and none was killed.

The target dir is shared between worktrees, so before the first run I touched the sources under test (`crates/{storage,meeting,notes,audio,asr}` and `apps/desktop/src/{meeting/mod.rs,lib.rs,managers/audio.rs}`). Each crate then printed `Compiling ... (/home/augusto/projects/fala/.houston/worktrees/meeting-desktop/...)`, `fala` (apps/desktop) included.

One exception was `fala-audio`. Its first run reused a stale test binary: there was no `Compiling` line, the binary held 25 tests, and the filter matched **0 tests** while cargo still exited 0. After touching `crates/audio/src/{lib.rs,meeting/mod.rs,meeting/system.rs}`, it recompiled from this worktree and the named test ran. Only the second run counts.

`git status --porcelain` was empty before the runs. Afterwards it shows only this report.

| Invocation | Result |
| --- | --- |
| `cargo test -p fala-storage --test meetings` | 10 passed, each listed: `migrates_v1_to_v2_keeping_dictations`, `create_then_read_meeting`, `finish_meeting_records_end`, `annotations_survive_reopen`, `segments_sorted_numbered_and_replaced`, `notes_saved`, `list_newest_first`, `unknown_id_is_not_found`, `draft_takes_agenda_and_records_once`, `schema_constraints`; exit 0 |
| `cargo test -p fala-storage --test store open_sets_wal_timeout_and_version` | running 1 test, 1 passed, 21 filtered out |
| `cargo test -p fala-meeting --lib mute` | 4 passed: `system_mutes_at_120_s`, `sound_clears_and_restarts`, `mic_only_counts_with_mic`, `paused_time_does_not_count` |
| `cargo test -p fala-notes --lib document` | 2 passed: `document_with_generated_notes`, `document_without_notes_or_segments` |
| `cargo test -p fala-audio --lib default_sink` (after touch) | running 1 test: `meeting::system::tests::parses_default_sink_from_pw_metadata` ok, 23 filtered out |
| `cargo test -p fala-asr --test meeting_scribe progress::` | 3 passed: `reports_at_least_every_second`, `cancel_returns_within_a_second`, `failures_keep_audio_and_classify_retry` |
| `cargo test -p fala-asr --test meeting_scribe progress::reports_at_least_every_second -- --exact` | running 1 test, 1 passed, 8 filtered out |
| `cargo test -p fala-asr --test meeting_scribe progress::cancel_returns_within_a_second -- --exact` | running 1 test, 1 passed, 8 filtered out |
| `cargo test -p fala --lib -- meeting:: overlay::tests::meeting tray::tests::meeting` (JOBS=1) | 14 passed, 350 filtered out, exit 0. Listed individually: `start_guard_order`, `consent_stamp_and_error_shape`, `tray_start_without_consent_asks`, `dictation_blocked_while_recording`, `stop_reasons_reach_storage`, `start_uses_the_draft`, `recorder::tests::{paused_time_is_not_counted, recorded_ms_follows_frames}`, `pipeline::tests::{segments_round_trip, keys_come_from_the_store, retain_before_transcribing, notes_input_from_session}`, `overlay::tests::meeting_overlay_states`, `tray::tests::meeting_items_follow_state` |
| `bun src/components/meeting/meeting.test.tsx` | `C12`, `C25-muted`, `C25-cap`, `C27`, `C33`, `C36-empty`, `C36-list`, `C36-detail`, `C36-error`, `C36-busy`, `C36-key`, `C42` all `ok`, exit 0 |
| `bun src/overlay/pill.test.tsx` | `meeting-pill ok`, `meeting-muted ok` (plus the 16 earlier checks), exit 0 |
| `bun run lint` · `bun run check:translations` · `bunx tsc --noEmit` | exit 0 · "All 1 languages have complete translations", exit 0 · exit 0 |
| `bash .specs/features/meeting-panel/proofs/*.sh` (6 scripts) | all exit 0: `cancel_token_wired` (1 ok), `dictation_refusal` (2 ok), `explicit_start` (4 ok), `no_content_in_logs` ("24 log calls"), `one_mic_stream` (2 ok), `round1_fixes` (C39, C40, C41, C43 ok) |

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | v1 with 2 dictations migrates to v2 and keeps them; a new DB opens at 2 | `migrates_v1_to_v2_keeping_dictations` ok at d4e2448 | carried from 5a4e89e: `crates/storage/tests/meetings.rs:110` `assert_eq!(user_version(&e.db), 2)`; `:114-127` both dictations' text; `:133` new DB at 2 | PASS |
| C2 | draft read-back shape | `create_then_read_meeting` ok | carried from 5a4e89e: `crates/storage/tests/meetings.rs:152` mode `InPerson`; `:155` `started_at == None`; `:156-163` `ended_at`, `recorded_ms 0`, annotations, notes, `audio_retained` | PASS |
| C3 | `finish_meeting` + retained; `cap_reached` stored | `finish_meeting_records_end` ok | carried from 5a4e89e: `crates/storage/tests/meetings.rs:179-181`; `:187` raw column `== "cap_reached"` | PASS |
| C4 | annotations survive reopen | `annotations_survive_reopen` ok | carried from 5a4e89e: `crates/storage/tests/meetings.rs:202-205` `annotations == "decidir data\nAna: contrato"` | PASS |
| C5 | segments sorted, numbered, JSON speakers, replaced | `segments_sorted_numbered_and_replaced` ok | carried from 5a4e89e: `crates/storage/tests/meetings.rs:231-238`, `:253-256`, `:240-243`, `:266-267` | PASS |
| C6 | notes saved with template | `notes_saved` ok | carried from 5a4e89e: `crates/storage/tests/meetings.rs:281-282` | PASS |
| C7 | newest first | `list_newest_first` ok | carried from 5a4e89e: `crates/storage/tests/meetings.rs:298` | PASS |
| C8 | unknown id → `NotFound(id)` | `unknown_id_is_not_found` ok | carried from 5a4e89e: `crates/storage/tests/meetings.rs:303` `NotFound(got) => assert_eq!(got, missing.to_string())` over `:314-331` | PASS |
| C37 | draft takes agenda, records once, `MeetingAlreadyStarted` | `draft_takes_agenda_and_records_once` ok, `unknown_id_is_not_found` ok | carried from 5a4e89e: `crates/storage/tests/meetings.rs:346-348`, `:351-353`, `:359-361`, `:364`, `:367-370`, `:327-331` | PASS |
| C9 | cascade; `mode='outro'` refused | `schema_constraints` ok | carried from 5a4e89e: `crates/storage/tests/meetings.rs:404` `bad.is_err()`; `:413` `left == 0` | PASS |
| C10 | start guard order | `meeting::tests::start_guard_order` ok | verified at d4e2448: `apps/desktop/src/meeting/mod.rs:788-791` `check_start(None, false, false) == Err(ConsentRequired)`; `:797-800` `Err(DictationActive)`; `:801-804` `Err(AlreadyActive)`; `:805` `Ok(())` | PASS |
| C11 | RFC 3339 stamp; `{"kind":"consent_required"}` | `consent_stamp_and_error_shape` ok | verified at d4e2448: `apps/desktop/src/meeting/mod.rs:814` `consent_stamp(at) == "2026-10-09T22:14:03-03:00"`; `:815-818` `json!({ "kind": "consent_required" })` | PASS |
| C12 | door 2 text verbatim, accept/cancel rendered | `meeting.test.tsx` `C12 ok` | verified at d4e2448 (lines unchanged, the fix only edits `:340-365`): `src/components/meeting/meeting.test.tsx:136-139` `pt.meeting.consent.body` equals the door 2 string; `:140-141` "Entendi, gravar"/"Cancelar" | PASS |
| C13 | start/resume built only in `start`/`resume`, reached only from commands and tray items | `explicit_start.sh` exit 0 (4 ok) | verified at d4e2448: `.specs/features/meeting-panel/proofs/explicit_start.py:7-16` both actions only inside `start`/`resume` in `meeting/mod.rs`; `:24-25` `.start(` callers are exactly the command and `start_from_tray`; `:33-35` the `meeting_start` arm calls `start_from_tray`; `:40-43` `resume` callers are `commands/meeting.rs` and the `"meeting_resume" => meetings.resume()` arm, which now sits inside the spawned thread (`apps/desktop/src/lib.rs:346-350`). No new caller: the thread only moves where the tray item runs | PASS |
| C14 | an accepted `start_meeting` creates the row and the WAV and emits `recording` | manual, **not run** (needs `tauri dev`, a click and real room audio) | none at runtime. Source only: `apps/desktop/src/meeting/mod.rs:467` `recorder::spawn(.., dir.join(WORK_WAV), ..)`, `:490`/`:500` row and start written after it, `:518` publish | NOT RUN |
| C15 | overlay states; pill 12:05 | `overlay::tests::meeting_overlay_states` ok, `pill.test.tsx` `meeting-pill ok` | carried from 5a4e89e: `apps/desktop/src/overlay.rs:820-828` `overlay_for_meeting(..) == Some("meeting")/Some("meeting_paused")/None`; `:843`, `:845-848`; `src/overlay/pill.test.tsx:369` `text(live) == "12:05"`; `:375` | PASS |
| C16 | tray items per state; recording icon | `tray::tests::meeting_items_follow_state` ok | carried from 5a4e89e: `apps/desktop/src/tray.rs:806` `== ["meeting_start"]`; `:807-814`; `:815-822` | PASS |
| C17 | tray start without consent shows window, emits event, page opens on notice, no `start` | `tray_start_without_consent_asks` ok; manual **not run** | verified at d4e2448: `apps/desktop/src/meeting/mod.rs:827` `tray_start(None) == TrayStart::AskConsent`; `:828-831` `Start` with consent. The window, event and page half (`mod.rs:557-558`, `src/App.tsx:151-153`, `src/components/meeting/MeetingsPage.tsx:49`, `:93`) has only C42's source-level assertions on the listeners, and its manual proof did not run | PARTIAL |
| C18 | dictation refused while recording/paused, before model load; UI maps it | `dictation_blocked_while_recording` ok, `dictation_refusal.sh` exit 0 (2 ok) | verified at d4e2448: `apps/desktop/src/meeting/mod.rs:836-843` Recording/Paused → `Some("meeting_active")`, `:844` None → `None`; `.specs/features/meeting-panel/proofs/dictation_refusal.py:13-16` refusal offset precedes `initiate_model_load`/`set_tray_state`/`try_start_recording`; `:23-24` `App.tsx` maps to `errors.meetingActive` (`src/App.tsx:174`) | PASS |
| C19 | release before the mic opens; reopen only when `AlwaysOn` | `one_mic_stream.sh` exit 0 (2 ok) | verified at d4e2448: `.specs/features/meeting-panel/proofs/one_mic_stream.py:7-10` release (`apps/desktop/src/meeting/mod.rs:453`) < spawn (`:467`); `:11-15` restore in `finish` (`:693`) and on the failure path (`:471`); `:22-26` `AlwaysOn` precedes `start_microphone_stream` (`apps/desktop/src/managers/audio.rs:796-803`). With the round-3 `update_mode` change, a toggle to always-on during a meeting now leaves the manager in `AlwaysOn` (`audio.rs:831`), so this restore reopens the stream at the end | PASS |
| C20 | paused time not counted | `recorder::tests::paused_time_is_not_counted` ok | carried from 5a4e89e: `apps/desktop/src/meeting/recorder.rs:251` `elapsed(at(18)) == 13 s`; `:248-254` | PASS |
| C21 | `recorded_ms = written/48` | `recorder::tests::recorded_ms_follows_frames` ok | carried from 5a4e89e: `apps/desktop/src/meeting/recorder.rs:259` `recorded_ms(48_000) == 1_000`; `:261`. The "status carries it" half is still not asserted (P3, kept) | PASS |
| C22 | stop via command: end, `recorded_ms`, `user`, pill hidden, tray back, `processing` then `idle` | manual, **not run** | none at runtime. Source only: `apps/desktop/src/meeting/mod.rs:668` `stop_reason(active.session.state())`, `:693-694` restore and retain in `finish` | NOT RUN |
| C23 | `MuteWatch` 119 s / 120 s / clears and restarts | `system_mutes_at_120_s` ok, `sound_clears_and_restarts` ok | carried from 5a4e89e: `crates/meeting/src/mute.rs:85` `at_119 == Muted::default()`; `:87-93`; `:101`; `:103-104` | PASS |
| C24 | `system_only` never marks mic | `mic_only_counts_with_mic` ok | carried from 5a4e89e: `crates/meeting/src/mute.rs:110` `!run(system_only, ..).mic`; `:113` | PASS |
| C25 | muted warning on page and pill; cap remaining and "Mais 1 h" | `C25-muted ok`, `C25-cap ok`, `meeting-muted ok` | verified at d4e2448: `src/components/meeting/meeting.test.tsx:162-172` `mutedMic`/`mutedSystem` only with the flag; `:175` `capWarning` with `cap_remaining_ms`; carried from 5a4e89e: `src/overlay/pill.test.tsx:390`, `:393-395` | PASS |
| C26 | `silence`/`cap_reached` via the user-stop path | `stop_reasons_reach_storage` ok | verified at d4e2448: `apps/desktop/src/meeting/mod.rs:854-855` `stop_reason(Stopped/Stopping { reason }) == reason` for all 3; `:879` stored `stop_reason == Some(reason)` | PASS |
| C27 | saver 2000 ms, flush, no equal save | `C27 ok` | verified at d4e2448: `src/components/meeting/meeting.test.tsx:206` `SAVE_AFTER_MS == 2000`; `:211-223` (unchanged lines) | PASS |
| C38 | `start_target` uses the draft; agenda reaches `notes_input` | `start_uses_the_draft` ok, `notes_input_from_session` ok | verified at d4e2448: `apps/desktop/src/meeting/mod.rs:897-900` `== Ok((draft, true))`; `:904-907` `Err(MeetingError::AlreadyStarted)`; `:908-911` new id and still 1 row; carried from 5a4e89e: `apps/desktop/src/meeting/pipeline.rs:667` `input.annotations == "pauta: contrato"` | PASS |
| C28 | stored shape and `notes_segments` mapping | `segments_round_trip` ok | carried from 5a4e89e: `apps/desktop/src/meeting/pipeline.rs:565-569` `notes[0].id == 1`, `Speaker::Me`, `Person(2)`. Precision gap P2 kept (storage numbers the segments, C5) | PASS |
| C29 | keys only from the store | `keys_come_from_the_store` ok | carried from 5a4e89e: `apps/desktop/src/meeting/pipeline.rs:578` `Err(MissingKey)`; `:583`; `:597-600` | PASS |
| C30 | `needs_retain` | `retain_before_transcribing` ok | carried from 5a4e89e: `apps/desktop/src/meeting/pipeline.rs:612`, `:614`, `:616` | PASS |
| C31 | `notes_input` fields; unknown template | `notes_input_from_session` ok | carried from 5a4e89e: `apps/desktop/src/meeting/pipeline.rs:661-669`; `:680-683` `Err(UnknownTemplate)` | PASS |
| C32 | `note_document` shapes | both `document::tests` ok | carried from 5a4e89e: `crates/notes/src/document.rs:52-55`, `:59-62`, `:71-74`, `:79`, `:84-87`, `:91-94` | PASS |
| C33 | copy writes exactly `meeting_markdown` | `C33 ok` | verified at d4e2448: `src/components/meeting/meeting.test.tsx:231-233` page source holds `commands.meetingMarkdown(id)` → `copyToClipboard(result.data)` (code at `src/components/meeting/MeetingsPage.tsx:169-170`, moved -3). Source-level (L1, kept) | PASS |
| C34 | no content or key in logs above debug | `no_content_in_logs.sh` exit 0, 24 calls | verified at d4e2448: `.specs/features/meeting-panel/proofs/no_content_in_logs.py:7` forbidden pattern; `:12-22` applied to each call. The one log line the fix moved, `apps/desktop/src/lib.rs:354` `log::warn!("Meeting tray action {} failed: {}", id, e)`, carries the tray item id and the error only | PASS |
| C35 | progress and cancel from `fala-asr`, wired to `MeetingProgress` and the same token | `progress::reports_at_least_every_second -- --exact` ran 1 test ok; `progress::cancel_returns_within_a_second -- --exact` ran 1 test ok; `cancel_token_wired.sh` exit 0 | carried from 5a4e89e: `crates/asr/tests/meeting_scribe.rs:488` `assert!(gap <= Duration::from_secs(1))`; `:513-514` `AsrError::Cancelled` and `elapsed < 1_500 ms`; `.specs/features/meeting-panel/proofs/cancel_token_wired.py:7-21` | PASS |
| C36 | page states; i18n only; components in `src/components/meeting/` | `C36-*` ok; lint, translations, tsc exit 0 | verified at d4e2448 (lines unchanged): `src/components/meeting/meeting.test.tsx:246` `meeting.list.empty`; `:267` `2026-10-02 14:02` and `37:30`; `:284` `"[01:05] Pessoa 1: Proponho 15 de novembro."`; `:299` `errorKey({ kind: "missing_key" }) == "meeting.errors.missing_key"`; `:317` 2 `disabled=""` while busy; `:322` `type="password"`, `value=""` | PASS |
| C39 | `retain` holds `retaining` before it looks at the WAV | `round1_fixes.sh` exit 0 | verified at d4e2448: `.specs/features/meeting-panel/proofs/round1_fixes.py:10-15` lock offset < `wav.exists()`, field present. Code: `apps/desktop/src/meeting/mod.rs:713` `let _retaining = self.retaining.lock()` before `:715` `if !wav.exists()`; field `:313`; callers `:694` and `apps/desktop/src/meeting/pipeline.rs:449-450`. Static proof only | PASS |
| C40 | `start_microphone_stream` refuses first while a meeting records | `round1_fixes.sh` exit 0 | verified at d4e2448: `.specs/features/meeting-panel/proofs/round1_fixes.py:18-23` guard offset < `self.is_open`. Code: `apps/desktop/src/managers/audio.rs:641` `if crate::meeting::indicator() != MeetingIndicator::None { return Err(..) }` before `:646` `self.is_open.lock()` | PASS |
| C41 | the row is written only after `recorder::spawn` succeeded | `round1_fixes.sh` exit 0 | verified at d4e2448: `.specs/features/meeting-panel/proofs/round1_fixes.py:25-30` `create_meeting` offset > `recorder::spawn` offset. Code: `apps/desktop/src/meeting/mod.rs:467` spawn, failure returns at `:469-477`; `:490` `create_meeting`, `:500` `start_meeting_recording` | PASS |
| C42 | (a) no notes field while the open session records; (b) reloads when the recording state changes; (c) the tray's request is left for the page only when the page is not open (App checks the section; the page consumes on mount, never in its listener) | `C42 ok` | verified at d4e2448. (a) rendered: `src/components/meeting/meeting.test.tsx:334` `live.includes("meeting.view.notesAbove")`, `:335-338` `!live.includes("<textarea")`, `:339`. (b) source: `:340-343` `[status.state, status.id, openId, open]` (code `MeetingsPage.tsx:115-118`). (c) source: `:348-352` `if (sectionRef.current !== "meetings") {` precedes `requestConsent();` in App's listener (code `src/App.tsx:147-153`); `:353` `sectionRef.current = currentSection`; `:357-360` the page listener holds no `takeConsentRequest` (code `MeetingsPage.tsx:93` `listen("meeting-consent-required", () => setAskConsent(true))`); `:361-364` `useState(takeConsentRequest)` (code `MeetingsPage.tsx:49`). The page renders only when `currentSection === "meetings"` (`src/App.tsx:48-50` renders the active section only), so the two listeners no longer write the same flag and their order no longer matters. Level: source grep (L1), not a rendered App | PASS |
| C43 | (a) always-on toggle during a meeting stores the mode without opening the mic; (b) tray pause/resume/stop run off the event loop; (c) a capture that fails removes its empty `audio/<id>/` | `round1_fixes.sh` exit 0 (`C43` ok) | verified at d4e2448. (a) `.specs/features/meeting-panel/proofs/round1_fixes.py:32-36` guard precedes `self.start_microphone_stream()` inside the `OnDemand → AlwaysOn` arm; code `apps/desktop/src/managers/audio.rs:824-826` skips the open, `:831` `*self.mode.lock().unwrap() = new_mode` runs either way. (b) `round1_fixes.py:37-40` `std::thread::spawn` precedes `meetings.stop()`; code `apps/desktop/src/lib.rs:346-356`, all three calls inside the closure. (c) `round1_fixes.py:41-43` `remove_dir(&dir)` before the first `return Err` after `recorder::spawn(`; code `apps/desktop/src/meeting/mod.rs:474`. But `:430` `create_dir_all(&dir)` runs before `:452` `SystemAudio::default_name().map_err(.. AudioDevice ..)?` and before `:446` `session.apply(..)?` (`insufficient_disk`); both return without removing the folder. A missing default sink (`pw-metadata` absent or no sink, `crates/audio/src/meeting/system.rs:68-74`) is a capture that fails with the same `audio_device` error and still leaves an empty `audio/<id>/` | PARTIAL |

## Level and sampling

- **Static source proofs.** Python reads production source for C13, C18 (second half), C19, C34, C35 (third proof), C39-C41 and C43. That is the right level for "only called from", "before" and "inside" claims. C43(c) shows the limit: the script looks only at the window after `recorder::spawn(`, so the earlier `?` returns that also mean "the capture did not open" are outside what it can see.
- **Source greps of the page and App.** C33, C36-error, C36-key and C42(b)/(c) grep `MeetingsPage.tsx` or `App.tsx`. Neither is rendered in a test (L1, kept). For C42(c) the grep is now enough: the defect in round 2 lived in the order between two listeners, and the fix removes the second writer instead of ordering them.
- **Carried precision gaps** (author chose not to reword): P2 (C28 numbering is done by storage), P3 (C21 "status carries it" has no assertion), L1 (C33/C36-error/C36-key are source-level).
- **Precision note, kept from round 2.** AC 30 says cancel returns "em até 1 s"; `cancel_returns_within_a_second` asserts `< 1_500 ms` (`crates/asr/tests/meeting_scribe.rs:514`). C35's wording does not name the bound.

## Swept re-read (only rows the fix touched)

| Row | Constraint cited | In the code at d4e2448 |
| --- | --- | --- |
| concurrency | C10, C19, C39, C40, now C43 | yes. The tray's four meeting items all run off the event loop (`lib.rs:337`, `:346`). The always-on toggle no longer opens a stream while the indicator is set (`audio.rs:824`). Residual race, not new: see N3 |
| data lifecycle | C37, C4, C30 | yes. Unchanged by the fix except the `remove_dir` on one failure branch (`mod.rs:474`), which removes only an empty folder (`std::fs::remove_dir`, not `remove_dir_all`) |
| state transitions | C13 … C38 | yes. Tray pause/resume/stop call the same `act` path as the commands (`mod.rs:523-549`); only the calling thread changed |
| other rows | - | carried from 5a4e89e (files untouched) |

## Findings

Status against the code at `d4e2448`:

| # | Earlier status | Status now | Evidence |
| --- | --- | --- | --- |
| F1 | fixed in round 2 | **fixed** (carried, code unchanged) | `apps/desktop/src/meeting/mod.rs:713` before `:715` |
| F2 | fixed in round 2, side effect N1 | **fixed** | `apps/desktop/src/managers/audio.rs:641`; N1 closed below |
| F3 | fixed in round 2 | **fixed** (carried; `MeetingView.tsx` and `AnnotationsField.tsx` untouched) | C42(a)/(b) |
| F4 | fixed in round 2 | **fixed** | C35 selectors each ran 1 test |
| F5 | partly fixed: tray pause/stop on the main thread could block on `inner` during a start | **fixed** | `apps/desktop/src/lib.rs:346-356`: pause, resume and stop run on a spawned thread. The only other users of `MeetingManager` are the commands, all `async` (`apps/desktop/src/commands/meeting.rs:16-164`), and the recorder sink. The tray and overlay read the indicator from an atomic (`mod.rs:245-252`), so no main-thread path takes `inner` any more |
| F6 | fixed for the row; empty `audio/<id>/` left on a failed start | **partly fixed** | `mod.rs:474` removes the folder when `recorder::spawn` fails. Two earlier returns after `create_dir_all` (`:430`) still leave it: `SystemAudio::default_name()` at `:452` (an `audio_device` failure) and `session.apply(..)?` at `:446` (`insufficient_disk`). Cosmetic, and the reason C43 is PARTIAL. Fix: call `default_name()` before `create_dir_all`, or remove the empty folder on every error return of `start` after `:430` |
| F7 | accepted, no defect | **accepted** (carried) | - |
| F8 | not fixed (two listeners raced over the request flag) | **fixed** | `src/App.tsx:151` arms the flag only when `sectionRef.current !== "meetings"`; `src/components/meeting/MeetingsPage.tsx:93` only sets `askConsent`; `:49` consumes on mount. When the page is open, App does nothing and the flag stays unset; when it is not, the page mounts after `setCurrentSection("meetings")` and consumes it. One edge, not a regression: while onboarding is on screen with `currentSection === "meetings"`, no listener is mounted to show the notice and App skips the request, so the tray click does nothing visible (no recording starts either, so ADR-0005 holds) |
| F9 | P2, P3, L1 kept | **kept** | - |
| N1 | new in round 2: always-on toggle during a meeting desynced setting and manager | **fixed** | `apps/desktop/src/managers/audio.rs:824-826` skips the open while the indicator is set, and `:831` stores the mode, so `update_mode` returns `Ok` and the settings store keeps the toggle. At the end, `restore_microphone_after_meeting` (`:796-803`) sees `AlwaysOn` and reopens. `AlwaysOn → OnDemand` during a meeting stops an already-closed stream (`:814-818`) and stores `OnDemand`, so the restore leaves it closed |
| N2 | observed in round 2: at stop the reload and the final notes save are independent async commands | **observed, unchanged** | the fix did not touch `MeetingsPage.tsx:115-118` or `AnnotationsField.tsx`. Not reproduced; needs the app running |

New in this round:

- **N3 - observation, low, not introduced by the fix.** `update_mode` reads the indicator at `audio.rs:824` and opens at `:825` without holding anything the meeting start also holds. The meeting start releases the dictation mic at `mod.rs:453` and sets the indicator only when the `ShowIndicator` effect runs in the loop that follows (`crates/meeting/src/session.rs:342-343`, `mod.rs:459`). A toggle to always-on landing between `:453` and the indicator store would open a dictation stream right before `Mic::open`. The same window existed for `start_microphone_stream`'s own guard in round 2. The window is the time to build one effect list. Fix if wanted: set the indicator before `release_dictation_mic`.
- No defect introduced by the round-3 fix was found. The thread spawns move work off the event loop without changing what runs. `remove_dir` cannot delete content. The `sectionRef` write during render is the usual latest-value ref, and `bun run lint` passes.

## Gate

Automated, verified at d4e2448:

- **Rust:** 37 tests passed, 0 failed. storage 10 + 1, meeting 4, notes 2, audio 1, asr 3 (plus the 2 `--exact` re-runs of C35's selectors, 1 test each), desktop 14.
- **Frontend:** 2 test files exit 0 (12 meeting-panel checks ok incl. C42, 18 pill checks ok). `bun run lint`, `bun run check:translations` and `bunx tsc --noEmit` exit 0.
- **Proof scripts:** 6, all exit 0.
- **Selectors:** no named selector matched 0 tests in a counted run. The stale `fala-audio` run that matched 0 is discarded above.

Not run: the three manual proofs (C14, C17's second proof, C22). This round has no person at the app to run them.

Checks proven: 39 of 43. Not PASS:

- C14: not run
- C22: not run
- C17: partial (manual half)
- C43: partial (the `default_name()` failure leaves the empty folder)

Ranked gaps:

1. C22: stop via command. Manual, not run. Code path at `apps/desktop/src/meeting/mod.rs:668`, `:693-694`.
2. C14: an accepted start writes the row and the WAV. Manual, not run. Code at `apps/desktop/src/meeting/mod.rs:467`, `:490`, `:500`.
3. C17: the manual half (window, event, page with the notice) did not run. Code at `apps/desktop/src/meeting/mod.rs:557-558`, `src/App.tsx:151-153`, `src/components/meeting/MeetingsPage.tsx:49`.
4. C43(c) / F6 residual: an empty `audio/<id>/` stays after a `default_name()` or `insufficient_disk` failure. `apps/desktop/src/meeting/mod.rs:430` creates the folder before `:446`/`:452` return. Cosmetic.

Observations that do not fail a check:

- N2: the stop reload races the final notes save (unchanged).
- N3: a toggle to always-on in the window between `mod.rs:453` and the indicator store.
- F8 edge: a tray click is silent while onboarding shows with `currentSection === "meetings"`.

Gate command: `python3 /home/augusto/.claude/skills/tlc-spec-lean/scripts/validate_verification.py meeting-panel`. It exits 1 because the verdict is FAIL.
