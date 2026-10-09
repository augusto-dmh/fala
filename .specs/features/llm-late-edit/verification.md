# llm-late-edit verification

**Verdict**: PASS
**Profile**: light
**Diff range**: 616af4f..7cd6601 (`origin/main..HEAD`)
**Round**: 3 - scoped
**Verifier**: independent sub-agent (author != verifier)

Round 3 scope is the diff `6c44ba0..7cd6601`:

- 05b61c9 touches `.specs` only: the round 2 report and two lessons.
- 7cd6601 removes `llm_auto::format`, which nothing used after the stop path moved to
  `format_with_late_edit`. It also changes main's test helper `run` in the `llm_auto.rs` tests to
  call `format_with_late_edit(..).0`, and adds one Settled line.

No verdict was open after round 2. Every proof re-ran in full at `7cd6601`, and so did the whole
`llm_auto` test module, to show that main's existing tests still pass with the new helper.
`git diff --quiet 6c44ba0..7cd6601 -- src/ crates/ apps/desktop/src/managers apps/desktop/src/actions.rs`
exits 0. Citations in those files are therefore carried from 6c44ba0, and the ones in
`apps/desktop/src/llm_auto.rs` are refreshed.

## Checks

Proofs were re-run at `7cd6601`. Cargo reported the `fala_app_lib` test binary fresh: it is dated
`16:57:19`, after `llm_auto.rs` at `16:56:45`.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | app in `llm_disabled_apps` (case-insensitive) is saved via `add_sensitive`; other app not | `cargo test -p fala --lib -- <7 names>` exit 0; `test managers::history_dictations::tests::disabled_app_marks_sensitive ... ok` | `apps/desktop/src/managers/history_dictations.rs:410` - `assert_eq!(record.sensitive, sensitive, "{app}")` over `[("KeePassXC", true), ("notepad", false)]`; wiring `apps/desktop/src/actions.rs:761` calls `llm_auto::is_disabled_app` (`llm_auto.rs:70`) (carried from 6c44ba0) | PASS |
| C11 | retranscribe keeps the replaced dictation's `sensitive`, true and false | same desktop run; `test managers::history_dictations::tests::retry_keeps_sensitive ... ok` | `apps/desktop/src/managers/history_dictations.rs:448` - `assert_ne!(new, old)`; :451 `assert_eq!(record.sensitive, sensitive, "{sensitive}")`; code `apps/desktop/src/managers/history.rs:441` maps the old record to `(record.dictation.app, record.sensitive)`, :451 passes `sensitive` to `add_dictation` (carried from 6c44ba0) | PASS |
| C2 | `editAction` offers `undo` only for llm+final; null for rules/none and unlinked | `bun src/components/settings/history/historyModel.test.ts` exit 0; prints `C2 ok` | `src/components/settings/history/historyModel.test.ts:230` - `assert.equal(editAction(entry(d)), want, ...)` over the 6 rows at :223-228 (carried from 6c44ba0) | PASS |
| C3 | `apply_late_edit` sets final/editor/showing, keeps id, raw, app, created_at, sensitive; `get` agrees | `cargo test -p fala-storage --test store -- <3 names>` exit 0; `test apply_late_edit_sets_final_editor_and_showing ... ok` | `crates/storage/tests/store.rs:487` - `assert_eq!(applied.dictation.final_text, "texto do llm")`; :489 `Showing::Raw`; :494 `assert!(applied.sensitive)`; :495 `assert_eq!(store.get(&before.id).unwrap(), applied)` (carried from 6c44ba0) | PASS |
| C4 | `.md` has `edited_by: "llm"`, `showing: "raw"`, body = new final; `reindex` yields same record | same storage run; `test apply_late_edit_rewrites_mirror_and_survives_reindex ... ok` | `crates/storage/tests/store.rs:512` - `edited_by: \"llm\"`; :513 `showing: \"raw\"`; :514 `md.ends_with("---\nTexto do LLM.\n")`; :517 equality after `reindex` (carried from 6c44ba0) | PASS |
| C5 | unknown id -> `NotFound(id)`; `""`/`"   "` -> `EmptyEdit(id)`; nothing changes; table dropped -> `Db` | same storage run; `test apply_late_edit_rejects_unknown_id_and_blank_text ... ok` | `crates/storage/tests/store.rs:535` - `Err(StorageError::NotFound(id)) if id == "nao-existe"`; :543 `Err(StorageError::EmptyEdit(id)) if id == r.id`; :556 `Err(StorageError::Db(_))` (carried from 6c44ba0) | PASS |
| C6 | 3 s answer: rules text in < 2.5 s with `Some(LateEdit)`; applied (final = LLM, llm, raw); `history.db` keeps rules text; announce once | desktop run and `cargo test -p fala --lib llm_auto`; `test llm_auto::tests::late_answer_is_applied_and_announced ... ok` in both | `apps/desktop/src/llm_auto.rs:398` - `assert!(started.elapsed() < Duration::from_millis(2500))`; :461 `assert_eq!(record.dictation.final_text, LLM_TEXT)`; :466 `Some(SIXTEEN_RULES)`; :468 `assert_eq!(announced.len(), 1)` (verified at 7cd6601) | PASS |
| C7 | 500 / past deadline / deleted entry: store unchanged, no announce, log only debug or below | desktop run and `llm_auto` run; `test llm_auto::tests::late_answer_errors_change_nothing ... ok` in both | `apps/desktop/src/llm_auto.rs:519` - `assert!(announced.is_empty(), "{case}")`; :520 `assert_eq!(history.store.search("", 10).unwrap(), before, "{case}")`; :532 every captured line satisfies `*level >= log::Level::Debug` (verified at 7cd6601) | PASS |
| C8 | `redo` for llm+raw; screen labels it `settings.history.applyAiEdit`; pt/en strings | bun run above; prints `C8 apply label for redo ok` | `historyModel.test.ts:239` - `"redo"`; :242 regex for `t("settings.history.applyAiEdit")`; :247 `"Aplicar edição da IA"`; :248 `"Apply AI edit"`; `HistorySettings.tsx:424` (carried from 6c44ba0) | PASS |
| C9 | redo replaces, copies, toast `editedCopied`; undo returns to `redo`; Rust Final/Raw after late edit | bun prints `C9 switchText redo ok`, `C9 undo after apply offers apply again ok`; desktop `test managers::history_dictations::tests::undo_after_late_edit ... ok` | `historyModel.test.ts:271` `r.calls.redo, [7]`; :273 `copied, ["Texto do LLM."]`; :277 `toastKey: "settings.history.editedCopied"`; :285 `editAction(outU.entry!) == "redo"`; `apps/desktop/src/managers/history_dictations.rs:479` `shown_text() == "Bruto, editado."`; :485 `assert_eq!(undone.editor, HistoryEditor::Llm)` (carried from 6c44ba0) | PASS |
| C10 | `applyAiEdit` once per locale, `redoAiEdit` nowhere in `src/` (fails without `src/`), locales in sync, lint green | both grep pipelines exit 0; `bun run check:translations` exit 0 (464 keys, "PT: All keys present"); `bun run lint` exit 0 | `src/i18n/locales/pt/translation.json:497` - `"applyAiEdit": "Aplicar edição da IA"`; `src/i18n/locales/en/translation.json:497` - `"applyAiEdit": "Apply AI edit"` (carried from 6c44ba0) | PASS |

## Helper change in `llm_auto` tests (7cd6601)

`cargo test -p fala --lib llm_auto` passed 8 and failed 0 (346 filtered out). Main's six existing
tests that go through the helper `run` (`llm_auto.rs:206-219`, now
`format_with_late_edit(..).0`) each reported `ok`:

- `llm_text_is_pasted_and_recorded_as_llm`
- `no_request_below_threshold_disabled_or_without_key`
- `no_request_for_disabled_app`
- `unknown_app_still_uses_llm`
- `slow_failed_or_invalid_llm_keeps_rules_text`
- `disabled_apps_are_normalized`

The diff changes no assertion. The removed wrapper returned exactly
`format_with_late_edit(...).0`, so behaviour is identical. No caller of `llm_auto::format(`
remains under `apps/desktop/src`: `grep` returned no hit.

## Coverage

Carried from 6c44ba0. The files in this set were untouched by 7cd6601.

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| paths that add a dictation (4) | `add_dictation` callers in `apps/desktop/src/managers/` | stop path C1 (`history.rs:338`) · retranscribe C11 (`history.rs:441`, `:451`) · failed transcription adds no dictation, existing `failed_transcription_saves_unlinked_row` ran ok at 7cd6601 (`history_dictations.rs:596` `assert_eq!(link(&conn, saved.id), None)`) · backfill has no app, existing `backfill_maps_rows_by_door_four` ran ok at 7cd6601 (`history_dictations.rs:622` `assert_eq!(record.dictation.app.app_name, None)`) | - |

## Faults injected

Not run under profile light.

## Findings

None new in round 3. Carried from 6c44ba0:

- Untested `AppHandle` wiring for AC 5: `actions.rs:916-925` and `history.rs:923-927`. This is
  non-blocking and is recorded in the Handoff as a manual item. The code was read and is correct.
- The spec text for history-undo (`.specs/features/history-undo/plan.md:155`, `checks.md:119,125`)
  still names "Reaplicar". This is an observation only, and Assumption 1 intends the change.

## Swept existing re-read

Carried from 6c44ba0, in files untouched by 7cd6601:

- redo is a no-op on an item already `final`: `crates/storage/src/store.rs:274-276`
- a cancel drops the `LateEdit`: `apps/desktop/src/actions.rs:100-136`
- deleting an entry deletes its dictation: `history.rs`, `delete_entry_with`

## Settled mid-build review

Earlier lines are carried from 6c44ba0, and all are sound. Two Settled lines describe the wrapper
and now read stale; neither is a defect:

- The round 1 line says `format` delegates to `format_with_late_edit`; the wrapper is now removed.
- The round 2 line says the stop path and the retry both use `format_with_late_edit`. The retry gets
  there through `process_transcription_output`.

The new "gates" line is sound and matches the diff. The only change is that `format` is gone and
`run` calls `.0`, and no assertion changed. The `llm_auto` run above confirms it.

## Gate

| Command | Result |
| --- | --- |
| `cargo test -p fala-storage --test store -- <3 names>` | 3 passed, 0 failed |
| `cargo test -p fala --lib -- <7 names>` | 7 passed, 0 failed |
| `cargo test -p fala --lib llm_auto` | 8 passed, 0 failed |
| `bun src/components/settings/history/historyModel.test.ts` | exit 0 |
| `bun run check:translations` | exit 0 |
| `bun run lint` | exit 0 |
| C10 grep pipelines | exit 0 |

Checks: 11 passed, 0 failed. Coverage members unproven: 0.
