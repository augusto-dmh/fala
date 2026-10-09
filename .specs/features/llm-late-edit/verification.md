# llm-late-edit verification

**Verdict**: PASS
**Profile**: light
**Diff range**: 616af4f..6c44ba0 (`origin/main..HEAD`)
**Round**: 2 - scoped
**Verifier**: independent sub-agent (author != verifier)

Round 2 scope: the fix diff `f2cc1d6..6c44ba0` (e1849f1 `test(storage)`, 6c44ba0 `fix(desktop)`
plus the `checks.md` edits) and the one non-PASS verdict of round 1 (finding 1, retranscribe drops
`sensitive`). Every proof re-ran in full at `6c44ba0`. Citations in touched files
(`crates/storage/tests/store.rs`, `apps/desktop/src/managers/history.rs`,
`apps/desktop/src/managers/history_dictations.rs`) are refreshed; the rest is carried from
`f2cc1d6`, where it was re-read in round 1, and is marked that way.

## Checks

Proofs re-run at `6c44ba0` (verified at 6c44ba0). Both test binaries were newer than the sources
at HEAD (store `16:44:30`, `fala_app_lib` `16:44:47`, latest touched source `16:44:25`), so cargo
reported them fresh.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | app in `llm_disabled_apps` (case-insensitive) is saved via `add_sensitive`; other app not | `cargo test -p fala --lib -- <7 names>` exit 0; `test managers::history_dictations::tests::disabled_app_marks_sensitive ... ok` | `apps/desktop/src/managers/history_dictations.rs:410` - `assert_eq!(record.sensitive, sensitive, "{app}")` over `[("KeePassXC", true), ("notepad", false)]`; wiring `apps/desktop/src/actions.rs:761` (carried from f2cc1d6, file untouched) | PASS |
| C11 | retranscribe (`update_transcription_with`) keeps the replaced dictation's `sensitive`, true and false | same desktop run; `test managers::history_dictations::tests::retry_keeps_sensitive ... ok` | `apps/desktop/src/managers/history_dictations.rs:448` - `assert_ne!(new, old)` (the dictation really was replaced); :451 `assert_eq!(record.sensitive, sensitive, "{sensitive}")` over `[true, false]`; code `apps/desktop/src/managers/history.rs:441` maps the old record to `(record.dictation.app, record.sensitive)` and :451 `add_dictation(store, &dictation, timestamp, sensitive)` | PASS |
| C2 | `editAction` offers `undo` only for llm+final; null for rules/none and unlinked | `bun src/components/settings/history/historyModel.test.ts` exit 0; prints `C2 ok` | `src/components/settings/history/historyModel.test.ts:230` - `assert.equal(editAction(entry(d)), want, ...)` over the 6 rows at :223-228 (file untouched by the fix) | PASS |
| C3 | `apply_late_edit` sets final/editor/showing, keeps id, raw, app, created_at, sensitive; `get` agrees | `cargo test -p fala-storage --test store -- <3 names>` exit 0; `test apply_late_edit_sets_final_editor_and_showing ... ok` | `crates/storage/tests/store.rs:487` - `assert_eq!(applied.dictation.final_text, "texto do llm")`; :489 `assert_eq!(applied.showing, Showing::Raw)`; :494 `assert!(applied.sensitive)`; :495 `assert_eq!(store.get(&before.id).unwrap(), applied)` | PASS |
| C4 | `.md` has `edited_by: "llm"`, `showing: "raw"`, body = new final; `reindex` yields same record | same storage run; `test apply_late_edit_rewrites_mirror_and_survives_reindex ... ok` | `crates/storage/tests/store.rs:512` - `md.contains("\nedited_by: \"llm\"\n")`; :513 `showing: \"raw\"`; :514 `md.ends_with("---\nTexto do LLM.\n")`; :517 `assert_eq!(store.get(&before.id).unwrap(), applied)` | PASS |
| C5 | unknown id -> `NotFound(id)`; `""`/`"   "` -> `EmptyEdit(id)`; nothing changes; table dropped -> `Db` | same storage run; `test apply_late_edit_rejects_unknown_id_and_blank_text ... ok` | `crates/storage/tests/store.rs:535` - `Err(StorageError::NotFound(id)) if id == "nao-existe"`; :543 `matches!(store.apply_late_edit(&r.id, blank), Err(StorageError::EmptyEdit(id)) if id == r.id)`; :556 `Err(StorageError::Db(_))` | PASS |
| C6 | 3 s answer: rules text in < 2.5 s with `Some(LateEdit)`; applied (final = LLM, llm, raw); `history.db` keeps rules text; announce once | desktop run; `test llm_auto::tests::late_answer_is_applied_and_announced ... ok` | `apps/desktop/src/llm_auto.rs:406` - `started.elapsed() < Duration::from_millis(2500)`; :469 `assert_eq!(record.dictation.final_text, LLM_TEXT)`; :474 `Some(SIXTEEN_RULES)`; :476 `assert_eq!(announced.len(), 1)` (file untouched) | PASS |
| C7 | 500 / past deadline / deleted entry: store unchanged, no announce, log only debug or below | desktop run; `test llm_auto::tests::late_answer_errors_change_nothing ... ok` | `apps/desktop/src/llm_auto.rs:527` - `assert!(announced.is_empty(), "{case}")`; :528 `search("", 10) == before`; :540 every captured line satisfies `*level >= log::Level::Debug` | PASS |
| C8 | `redo` for llm+raw; screen labels it `settings.history.applyAiEdit`; pt/en strings | bun run above; prints `C8 apply label for redo ok` | `historyModel.test.ts:239` - `"redo"`; :242 regex for `t("settings.history.applyAiEdit")`; :247 `"Aplicar edição da IA"`; :248 `"Apply AI edit"`; `HistorySettings.tsx:424` | PASS |
| C9 | redo replaces, copies, toast `editedCopied`; undo returns to `redo`; Rust Final/Raw after late edit | bun prints `C9 switchText redo ok`, `C9 undo after apply offers apply again ok`; desktop `test managers::history_dictations::tests::undo_after_late_edit ... ok` | `historyModel.test.ts:271` `r.calls.redo, [7]`; :273 `copied, ["Texto do LLM."]`; :277 `toastKey: "settings.history.editedCopied"`; :285 `editAction(outU.entry!) == "redo"`; `apps/desktop/src/managers/history_dictations.rs:479` `shown_text() == "Bruto, editado."`; :485 `assert_eq!(undone.editor, HistoryEditor::Llm)` | PASS |
| C10 | `applyAiEdit` once per locale, `redoAiEdit` nowhere in `src/` (fails without `src/`), locales in sync, lint green | both grep pipelines exit 0 (the negation now guarded by `test -f src/i18n/locales/pt/translation.json &&`); `bun run check:translations` exit 0 (464 keys, "PT: All keys present"); `bun run lint` exit 0 | `src/i18n/locales/pt/translation.json:497` - `"applyAiEdit": "Aplicar edição da IA"`; `src/i18n/locales/en/translation.json:497` - `"applyAiEdit": "Apply AI edit"` | PASS |

## Coverage

Profile `light` does not recompute the whole join. The one set that round 1 enumerated is
recomputed here, because the fix touched it (verified at 6c44ba0).

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| paths that add a dictation (4) | `add_dictation` callers in `apps/desktop/src/managers/` | stop path C1 (`history.rs:338`) · retranscribe C11 (`history.rs:441`, `:451`) · failed transcription adds no dictation, existing `failed_transcription_saves_unlinked_row` ran ok (`history_dictations.rs:596` `assert_eq!(link(&conn, saved.id), None)`) · backfill has no app, existing `backfill_maps_rows_by_door_four` ran ok (`history_dictations.rs:622` `assert_eq!(record.dictation.app.app_name, None)`) | - |

## Faults injected

Not run under profile light.

## Findings

Round 1 findings, re-judged at `6c44ba0`:

1. Retranscribe dropped `sensitive` (was blocking): **closed**. `update_transcription_with` now
   carries `record.sensitive` (`apps/desktop/src/managers/history.rs:441`, `:451`), and C11 proves
   both values. If the old dictation cannot be read, `unwrap_or_default()` gives `(default app, false)`,
   which is the same fallback as the app.
2. C5 precision gap: **closed**. `crates/storage/tests/store.rs:543` pins `EmptyEdit(id)` with the id.
3. Untested `AppHandle` wiring for AC 5 (non-blocking): **unchanged**. The new Settled line now says
   these items apply on any OS and stay listed because they are manual. Carried from f2cc1d6:
   `actions.rs:916-925`, `history.rs:923-927` (shifted +1 by the fix), read and correct.
4. L-002 edge in C10: **closed**. The negation now fails when `src/` is absent.
5. Stale history-undo spec text (`.specs/features/history-undo/plan.md:155`, `checks.md:119,125`):
   observation, unchanged, intended by Assumption 1.

## Swept existing re-read

Carried from f2cc1d6 (files untouched by the fix, except that `history.rs` lines shifted):
redo no-op at `crates/storage/src/store.rs:274-276`; cancel drops the `LateEdit` through
`complete_unless_cancelled` / `deliver_unless_cancelled` (`apps/desktop/src/actions.rs:100-136`);
deleting an entry deletes its dictation (`apps/desktop/src/managers/history.rs`, `delete_entry_with` ->
`history_dictations::delete_dictation`).

## Settled mid-build review

Carried from f2cc1d6 for the first five lines; all are sound. The round 1 note on the `sensitive` line
(retranscribe path) is resolved. New line (round 1 of the Verifier): sound. It accurately describes
the fix, C11 is additive and no existing check changed meaning. C5 now names `EmptyEdit`, which
strengthens the check without changing what it claims, and the C10 guard follows L-002.

## Gate

`cargo test -p fala-storage --test store -- <3 names>` - 3 passed, 0 failed (22 filtered out) ·
`cargo test -p fala --lib -- <7 names>` - 7 passed, 0 failed (347 filtered out) ·
`bun src/components/settings/history/historyModel.test.ts` exit 0 (C27-C30, C2, C8, C9 x2 ok) ·
`bun run check:translations` exit 0 · `bun run lint` exit 0 · C10 greps exit 0 ·
checks: 11 passed, 0 failed · coverage members unproven: 0
