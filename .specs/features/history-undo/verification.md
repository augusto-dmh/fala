# history-undo verification

**Verdict**: FAIL
**Profile**: light
**Diff range**: 1b812da..2324988
**Round**: 1 - full
**Verifier**: independent sub-agent, distinct from the author (the desktop-stack executor sub-agent); read-only over the real tree, verified at 2324988

The only check that is not proven is C13, the manual Windows check (`TODO(windows)`). The other
29 checks have a green proof at HEAD and a located assertion. The FAIL stays until someone runs
C13 on a Windows build.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | delete removes the row, FTS hit and `.md`; the other item keeps `get` and `.md` | `cargo test -p fala-storage --test store -- delete_removes_row_fts_and_mirror ...` (4 names, one run) exit 0, `delete_removes_row_fts_and_mirror ... ok` | `crates/storage/tests/store.rs:410` - `matches!(store.get(&gone.id), Err(StorageError::NotFound(id)) if id == gone.id)`; `:411` `store.search("jabuticaba", 10).unwrap().is_empty()`; `:412` `!gone_md.exists()` | PASS |
| C2 | unknown v7 id gives `NotFound(id)`, the existing item is untouched | same run, `delete_unknown_id_is_not_found ... ok` | `crates/storage/tests/store.rs:428` - `matches!(err, StorageError::NotFound(ref id) if *id == unknown)`; `:433` `store.search("pitanga", 10).unwrap().len(), 1` | PASS |
| C3 | `reindex` after a delete: `indexed = 1`, `skipped` empty, the deleted item stays `NotFound` | same run, `reindex_after_delete_does_not_resurrect ... ok` | `crates/storage/tests/store.rs:446` - `assert_eq!(report.indexed, 1)`; `:447` `report.skipped.is_empty()`; `:450` `Err(StorageError::NotFound(_))` | PASS |
| C4 | a `.md` that is already gone is not an error, and the row goes | same run, `delete_tolerates_missing_mirror ... ok` | `crates/storage/tests/store.rs:463` - `store.delete(&gone.id).unwrap()`; `:465` `Err(StorageError::NotFound(_))` | PASS |
| C5 | `editor_for` table, 4 rows | `cargo test -p fala --lib -- managers::history_dictations:: managers::history::tests::migration_five_adds_nullable_dictation_id actions::tests::delivery_is_all_or_nothing_after_the_last_check` exit 0, 18 passed, `editor_follows_pasted_text_and_llm ... ok` | `apps/desktop/src/managers/history_dictations.rs:383-386` (table) + `:388` `assert_eq!(editor_for(raw, pasted, llm), want, ...)` | PASS |
| C6 | save writes 1 linked row; the item has raw, final, `Llm`, `En`, `notepad`, `Final` | same run, `save_links_new_dictation ... ok` | `apps/desktop/src/managers/history_dictations.rs:411` `rows(&conn), 1`; `:413` `saved.dictation_id == Some(id)`; `:415-420` raw `"acao de amanha"`, final `"Ação de amanhã."`, `Editor::Llm`, `Language::En`, `Some("notepad")`, `Showing::Final` | PASS |
| C7 | language from settings, 5 values | same run, `language_falls_back_to_pt_br ... ok` | `apps/desktop/src/managers/history_dictations.rs:426-430` (table) + `:432` `assert_eq!(language_from_setting(setting), want, ...)` | PASS |
| C8 | store `None` and store with `Db` failure: row saved with text, `dictation_id` null | same run, `save_without_store_keeps_row_unlinked ... ok` | `apps/desktop/src/managers/history_dictations.rs:441-442` `dictation_id == None`, `link == None` (no store); `:446` `break_store()`; `:449-451` null link and `rows == 2` | PASS |
| C9 | mirror failure: the row links the id the error reports, and `get` finds it | same run, `save_mirror_failure_links_reported_id ... ok` | `apps/desktop/src/managers/history_dictations.rs:469` `link(...).expect("linked despite the mirror failure")`; `:470` `store.get(&id).unwrap().dictation.raw.text == "reunião"` | PASS |
| C10 | empty text: row with empty text, null link, nothing in `fala.sqlite` | same run, `failed_transcription_saves_unlinked_row ... ok` | `apps/desktop/src/managers/history_dictations.rs:486` `transcription_text == ""`; `:487` `link == None`; `:488` `store.search("", 10).unwrap().is_empty()` | PASS |
| C11 | the only `save_entry` with the pasted text sits in `save_history`; `actions.rs` has no `fala_storage`; all-or-nothing delivery stays green | 3 literal proofs: awk/grep exit 0; `! grep -q "fala_storage"` exit 0; `delivery_is_all_or_nothing_after_the_last_check ... ok` in the run above | `apps/desktop/src/actions.rs:817-833` (closure, `hm.save_entry(NewEntry {` at `:819`); `apps/desktop/src/actions.rs:1087` `calls.borrow().is_empty()` when cancelled; `:1097` `*calls.borrow() == vec!["paste", "save"]` | PASS |
| C12 | `save_history` passes the pasted text and `fala_inject::foreground_app()` | 2 literal awk/grep proofs exit 0 | `apps/desktop/src/actions.rs:826` `pasted_text,` (bound at `:816` `let pasted_text = processed.final_text.clone();`); `:828` `app: fala_inject::foreground_app(),` | PASS |
| C13 | Windows build: dictation in Notepad shows the pasted text and "em notepad"; `fala-cli history search` finds it | not run - `TODO(windows)` manual check, no Windows session here | no evidence | UNPROVEN |
| C14 | `user_version` 4 -> 5, nullable `dictation_id`, the existing row null | same run, `migration_five_adds_nullable_dictation_id ... ok` | `apps/desktop/src/managers/history.rs:1013` `assert_eq!(version, 5)`; `:1021` `assert_eq!(notnull, 0)`; `:1029` `assert_eq!(link, None)` | PASS |
| C15 | backfill of 4 rows: `None`/`Llm`/`Rules` with door-4 finals, null app, given language, same instant at the local offset; empty row null | same run, `backfill_maps_rows_by_door_four ... ok` | `apps/desktop/src/managers/history_dictations.rs:501` returns 3; `:504-506` table (`Rules` for post-processed without a request); `:510-517` raw, final, editor, `app_name == None`, `Language::En`, `created_at.timestamp() == ts`, offset == local; `:519` `link(&conn, empty) == None` | PASS |
| C16 | a second backfill adds nothing and keeps the ids | same run, `backfill_is_idempotent ... ok` | `apps/desktop/src/managers/history_dictations.rs:533` second run returns 0; `:536` `search("", 100).len() == 3`; `:537` `first == second` | PASS |
| C17 | `open_store` under a regular file gives `None`; `new` uses `open_store(` without `?`/`expect` | `open_store_failure_returns_none ... ok`; literal grep exit 0 | `apps/desktop/src/managers/history_dictations.rs:545` `open_store(&file.join("fala.sqlite"), ...).is_none()`; `apps/desktop/src/managers/history.rs:118` `let store = history_dictations::open_store(` | PASS |
| C18 | deleting a linked entry removes the item, `.md`, row and WAV | same run, `delete_linked_removes_dictation_row_and_wav ... ok` | `apps/desktop/src/managers/history_dictations.rs:561` `not_found(&store, &id)`; `:562` `!md.exists()`; `:563` `rows == 0`; `:564` `!wav.exists()` | PASS |
| C19 | `Db` failure: `Err`, row and WAV kept; `NotFound`: `Ok`, row and WAV gone | same run, `delete_failure_keeps_row_and_wav ... ok` | `apps/desktop/src/managers/history_dictations.rs:576-581` `is_err()`, `rows == 1`, `wav.exists()`; `:590-592` `unwrap()`, `rows == 0`, `!wav.exists()` | PASS |
| C20 | count limit 1 deletes the 2 oldest items with their rows; with `Db` failure, count and time cleanup keep everything | same run, `retention_deletes_dictations_or_keeps_entry ... ok` | `apps/desktop/src/managers/history_dictations.rs:609-612` `rows == 1`, ids[0] and ids[1] `NotFound`, ids[2] ok; `:627-630` `rows == 3` after count and after time cleanup with the store broken | PASS |
| C21 | retry adds a new item with the new texts and app "slack", relinks, deletes the old one | same run, `retry_replaces_dictation_keeping_app ... ok` | `apps/desktop/src/managers/history_dictations.rs:668` `new != old`; `:670-673` `"novo"`, `"Novo."`, `Editor::Llm`, `Some("slack")`; `:674` `not_found(&store, &old)` | PASS |
| C22 | `Raw` then `Final` switch the returned entry, the store and the `.md` | same run, `undo_then_redo_switches_showing ... ok` | `apps/desktop/src/managers/history_dictations.rs:687-690` `HistoryShowing::Raw`, `Showing::Raw`, `md.contains("showing: \"raw\"")`; `:695-698` the same for `Final`/`"final"` | PASS |
| C23 | `Err` and no change in 4 cases | same run, `undo_errors_change_nothing ... ok` | `apps/desktop/src/managers/history_dictations.rs:708` unknown id `is_err()`; `:712-715` unlinked `is_err()`; `:726-733` editor `None` `is_err()`, `showing == Final`, `.md` unchanged; `:738-739` store `None` `is_err()`, `showing == Final` | PASS |
| C24 | `set_showing` emits `Updated`; both commands registered and calling `Raw`/`Final` | 5 literal awk/grep proofs exit 0 | `apps/desktop/src/managers/history.rs:900` `HistoryUpdatePayload::Updated` inside `set_showing` (`:894`); `apps/desktop/src/lib.rs:773-774`; `apps/desktop/src/commands/history.rs:120` `Showing::Raw`, `:133` `Showing::Final` | PASS |
| C25 | the page fills `dictation` on linked entries and leaves it null on unlinked ones | same run, `entries_carry_their_dictation ... ok` | `apps/desktop/src/managers/history_dictations.rs:754` `page.entries[0].dictation == None`; `:755-764` `Some(HistoryDictation { raw_text, final_text, Llm, Final, Some("notepad") })` | PASS |
| C26 | `bindings.ts` has both commands and `dictation_id` on `HistoryEntry` | 3 literal grep/awk proofs exit 0 | `src/bindings.ts:897` `async undoHistoryEntryEdit(id: number)`; `:908` `async redoHistoryEntryEdit(id: number)`; `:1056` `dictation_id: string \| null;` | PASS |
| C27 | `shownText` returns final, raw or `transcription_text`; the screen shows and copies it | `bun src/components/settings/history/historyModel.test.ts` exit 0, printed `C27 ok` | `src/components/settings/history/historyModel.test.ts:78` `"Ação de amanhã."`; `:80-81` `"acao de amanha"`; `:83` `"texto do history.db"`; `:85` regex `onCopyText={() => copyToClipboard(shownText(entry))}` | PASS |
| C28 | `editAction` over 5 cases; button only when not null, with the two title keys | same run, `C28 ok` | `src/components/settings/history/historyModel.test.ts:101-108` table + `assert.equal(editAction(entry(d)), want)`; `:115` `{action && (`; `:118-119` both `t(...)` keys | PASS |
| C29 | undo/redo replace the entry, copy, toast; failure keeps the entry; `appName` and `inApp` | same run, `C29 ok` | `src/components/settings/history/historyModel.test.ts:130-135` copied `["acao de amanha"]`, `toastKey: "settings.history.originalCopied"`; `:141-146` `editedCopied`; `:154-159` `copied == []`, `entry: null`, `editToggleError`; `:186-188` `appName`; `:191` `{app && ( ... t("settings.history.inApp", { app })` | PASS |
| C30 | the 6 pt and en strings; lint, tsc, format and translations green | `bun ...historyModel.test.ts` `C30 ok`; `bun run lint` exit 0; `bunx tsc --noEmit` exit 0; `bun run format:check` exit 0 (Prettier and `cargo fmt --check`); `bun run check:translations` exit 0 ("PT: All keys present") | `src/components/settings/history/historyModel.test.ts:202-210` (expected pt/en), `:213-214` `assert.equal(pt.settings.history[key], ptText)` / `en...` | PASS |

The named tests exist and ran. Each `cargo test` name above shows up as its own `... ok` line: the
4 storage tests in one run, and the 16 `history_dictations` tests plus
`migration_five_adds_nullable_dictation_id` and `delivery_is_all_or_nothing_after_the_last_check`
in one run (18 passed). Every new test name is in the diff `1b812da..HEAD`.
`delivery_is_all_or_nothing_after_the_last_check` is inherited from 1.F2 and unchanged. C11 cites
it as a regression guard, and the awk proof covers the new part, the `save_entry` call inside the
closure.

## Coverage

The coverage recompute does not run under `light`. The author's join was only read. One member
the checks never named is reported in the ranked gaps (store unavailable while deleting).

## Faults injected

Fault injection does not run under `light`, so none were injected and no scratch worktree was
created.

## Scrutiny requested by the orchestrator

- **Cancel-anywhere (all-or-nothing delivery).** The only `save_entry` call that can write to
  `fala.sqlite` is the one inside `save_history` (`apps/desktop/src/actions.rs:819`). That closure
  is passed only to `deliver_unless_cancelled` (`:836-840`, `:849-862`). The other `save_entry`
  (`:892`, the transcription-error path, inherited) sends empty text, so `dictation_for` returns
  `None` (`history_dictations.rs:112-113`) and nothing reaches `fala.sqlite` (C10). Apart from
  that, the store is written by the backfill at startup and by the user's retry, delete, undo
  and redo. None of these runs during a dictation. `actions::tests::delivery_is_all_or_nothing_after_the_last_check`
  passes (`actions.rs:1086-1097`).
- **Migration 5 is additive.** It adds a single `ALTER TABLE transcription_history ADD COLUMN
  dictation_id TEXT;` (`history.rs:41`), nullable, with no default, no index and no rewrite of
  rows. `init_database` runs migrations before `backfill_dictations()`, and the tauri-plugin-sql
  conversion only sets `user_version` to the old count. C14 proves a v4 database with a row ends
  at v5 with a null link.
- **Backfill mapping.** `(None, _) -> None`, `(Some, true) -> Llm`, `(Some, false) -> Rules`
  (`history_dictations.rs:201-205`). C15 covers all three plus the empty-text row.
- **Delete and retention reach `fala.sqlite` and the `.md`.** Delete goes through `delete_entry_with`
  -> `delete_dictation` -> `Store::delete` (the row, then the `.md`), proven by C18. Retention goes
  through `delete_entries_and_files_with`, proven by C20. The store-unavailable case is the
  exception: see gap 2.
- **No inherited assertion weakened.** In the inherited tests, `managers::history::tests::setup_conn`
  gained only the `dictation_id TEXT` column, `tray.rs` `build_entry` gained only
  `dictation_id: None, dictation: None`, and the `actions.rs` tests are untouched. The 337 tests
  of `cargo test -p fala --lib` pass. C12: the approved text named `&pasted_text`, and commit
  44edae1 changed the claim's parenthetical and the second proof to `pasted_text,`. The claim is
  still that the closure passes the pasted text. That holds: the struct field is bound at
  `actions.rs:816` from `processed.final_text`, the same value pasted at `:846-851`.
- **ADR-0004.** `fala_inject::foreground_app()` returns `AppContext`, whose only field is
  `app_name: Option<String>` (`crates/core/src/dictation.rs:14-15`). It carries no window title,
  path or content. Only text and the app name are stored, never audio.
- **i18n.** The 6 new keys are present in both `src/i18n/locales/pt/translation.json` and
  `en/translation.json` (C30). `check:translations` passes.
- **`src/bindings.ts`.** The diff has two hunks: the two command wrappers, and the
  `HistoryDictation`/`HistoryEditor`/`HistoryShowing` types plus the two `HistoryEntry` fields.
  It has nothing else (the unrelated `isLaptop` hunk is absent).
- **AGENTS.md for `apps/desktop`.** The `history.rs` hunks all belong to the feature, with no
  drive-by reformatting. `format_timestamp_title` lost `&self` because the new `*_with` free
  functions call it. `cargo clippy -p fala -p fala-storage --all-targets -- -D warnings` exits 0,
  `scripts/check-brand.sh` exits 0, and `scripts/check-no-tauri-in-crates.sh` exits 0. The new
  crates are workspace crates without `tauri` (ADR-0002).
- **`Swept existing` re-read.** The concurrency row checks out: `store: Option<Mutex<Store>>`
  (`history.rs:102`), and `Store::open` sets WAL and a 5 s `busy_timeout`
  (`crates/storage/src/store.rs:61-62`).

## Ranked gaps

1. **C13 unproven.** It is a `TODO(windows)` manual check and nobody has run a Windows build.
   It is the only reason for the FAIL. Owed: one dictation in Notepad with the LLM on, recording
   the text shown, the "em notepad" label and the output of `fala-cli history search`.
2. **The store-unavailable member of the delete and retention sets has no check, and the code
   departs from AC 17 as written.** When `fala.sqlite` failed to open at startup but holds items
   from an earlier session, both of these remove the `history.db` row and the WAV and leave the
   dictation and its `.md` behind:
   - `delete_entry_with` logs "stays in fala.sqlite" (`apps/desktop/src/managers/history.rs:868`).
   - `delete_entries_and_files_with` (retention) skips the store when it is `None` (`history.rs:515`).

   AC 17 says "an entry whose dictation could not be deleted SHALL stay until the next cleanup".
   The plan's own assumption calls retention a privacy control. C19 and C20 cover only the `Db`
   failure. This is a precision gap in the plan and checks for the user to settle. It does not
   change C18-C20, which pass as written.
3. **Retry with an empty re-transcription keeps the old link.** When the retry returns empty text,
   `new_link` is `None` and `dictation_id` keeps pointing at the previous dictation
   (`history.rs:461`). The row then has `transcription_text = ''`, but the screen shows the old
   dictation's text through `shownText`. No AC or check covers this; AC 18 covers only a retry
   that produces a new dictation.
4. **Precision notes, not failures.**
   - C23 asserts the unchanged `.md` only in the `editor = None` case. In the store-`None` case it
     asserts only the store's `showing`.
   - C9 shows the link points at the stored item through `get`, not by comparing it with the id in
     `StorageError::Mirror`. It is equivalent here, because the store holds a single item.
   - C12's second proof matches the name `pasted_text,`. The binding to `processed.final_text`
     (`actions.rs:816`) falls outside the awk range and was read by hand.

## Gate

- `cargo test -p fala-storage` - 33 passed, 0 failed (store 22, reindex 8, mirror 3)
- `cargo test -p fala --lib` - 337 passed, 0 failed
- `bun src/components/settings/history/historyModel.test.ts` - C27, C28, C29, C30 ok
- `bun run lint`, `bunx tsc --noEmit`, `bun run format:check`, `bun run check:translations` - exit 0
- `cargo clippy -p fala -p fala-storage --all-targets -- -D warnings` - exit 0
- `scripts/check-brand.sh`, `scripts/check-no-tauri-in-crates.sh` - exit 0
- Real tree `git status --porcelain` was clean before this report. The only file added is this report.
