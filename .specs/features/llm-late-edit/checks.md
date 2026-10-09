# llm-late-edit checks

Profile: light
Plan: `.specs/features/llm-late-edit/plan.md`

10 checks in 2 slices · 1 one-way door · 0 open, of which 0 block

Comandos reais do repositório: `cargo test -p fala --lib <filtro>` (o pacote de `apps/desktop` é `fala`),
`cargo test -p fala-storage --test store <filtro>`, `bun <arquivo>.test.ts` (padrão de `historyModel.test.ts`),
`bun run lint`, `bun run check:translations`. Todo `cargo` com
`CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`. O "Gemini" dos testes é o
`FakeGemini` de `apps/desktop/src/llm_auto.rs` (mod `tests`, de main), um `TcpListener` local apontado por
`Gemini::with_base_url`. Os nomes de teste abaixo são obrigações: o builder os cria com esses nomes.

## Checks

### S1 - o histórico sabe o que é sensível · 3 files · 60 KB · ~15k

**C1** - With `app_name` in `llm_disabled_apps` (the list `["keepassxc"]`, the app `"KeePassXC"`, case-insensitive) the dictation is added through `Store::add_sensitive` and `Store::get` returns `sensitive = true`; with `"notepad"`, `sensitive = false` (AC 1)
Proof: `cargo test -p fala --lib managers::history_dictations::tests::disabled_app_marks_sensitive` (two cases)

**C2** - `editAction` offers `undo` only for `editor === "llm"` with `showing === "final"`, and nothing for `rules` (final or raw), `none` (final or raw) or an unlinked entry (AC 2)
Proof: `bun src/components/settings/history/historyModel.test.ts` (prints `C2 ok`; the file is a script that exits non-zero at the first failure)

### S2 - a resposta que chega tarde vira "Aplicar edição da IA" · 8 files · 120 KB · ~30k

**C3** - `Store::apply_late_edit(id, "texto do llm")` on an existing sensitive dictation returns a record with `final_text = "texto do llm"`, `editor = Llm`, `showing = Raw`, the same `id`, `raw`, `app`, `created_at` and `sensitive` as before, and `Store::get(id)` agrees (AC 3)
Proof: `cargo test -p fala-storage --test store -- --exact apply_late_edit_sets_final_editor_and_showing`

**C4** - After `apply_late_edit` the item's `.md` frontmatter has `edited_by: "llm"` and `showing: "raw"` and its body is the new final text; `reindex` over that `.md` yields the same record (AC 3)
Proof: `cargo test -p fala-storage --test store -- --exact apply_late_edit_rewrites_mirror_and_survives_reindex`

**C5** - `apply_late_edit` with an unknown id returns `StorageError::NotFound` with that id and no row or `.md` changes; with `""` or `"   "` it returns an error and the row and `.md` are unchanged; with the `dictations` table gone it returns `StorageError::Db` (AC 4, Surface `Database`)
Proof: `cargo test -p fala-storage --test store -- --exact apply_late_edit_rejects_unknown_id_and_blank_text` (four cases)

**C6** - When the fake Gemini answers 3 s after the request, `llm_auto::format_with_late_edit` returns the rules text with `llm_produced = false` in under 2.5 s together with `Some(LateEdit)`; the entry saved with that text is then updated by `finish_late_edit` with `HistoryManager::apply_late_edit_with`: `Store::get(dictation_id)` shows `final_text` = the server's answer, `editor = Llm`, `showing = Raw`; the `history.db` row's `post_processed_text` still equals the pasted rules text; and the announce callback (the `history-update-payload` `Updated` emitter in the app) runs once with that entry (AC 5)
Proof: `cargo test -p fala --lib llm_auto::tests::late_answer_is_applied_and_announced`

**C7** - A late answer that is an error (server 500 after 3 s), one that arrives after the late deadline (server answers after 4 s, with the deadline shortened to 3 s through `Postprocessor::with_late_deadline`), and one whose history entry was deleted before it arrived each leave the store unchanged, never call the announce callback, and log only at `debug` or below (AC 6)
Proof: `cargo test -p fala --lib llm_auto::tests::late_answer_errors_change_nothing` (three cases)

**C8** - `editAction` returns `redo` for `editor === "llm"` with `showing === "raw"`, and the screen labels `redo` with `t("settings.history.applyAiEdit")`, rendered as "Aplicar edição da IA" in pt and "Apply AI edit" in en (AC 7)
Proof: `bun src/components/settings/history/historyModel.test.ts` (prints `C8 apply label for redo ok`)

**C9** - `switchText` on a `redo` action for a late-edited entry calls `redoHistoryEntryEdit`, replaces the entry and copies the applied text with the toast `settings.history.editedCopied`; undoing afterwards goes back to `showing = raw` and `editAction` is `redo` again; on the Rust side, after `apply_late_edit_with`, `set_showing_with(Final)` shows the late text and `set_showing_with(Raw)` goes back to raw with `editor = llm` (AC 7, AC 8)
Proof: `bun src/components/settings/history/historyModel.test.ts` (prints `C9 switchText redo ok` and `C9 undo after apply offers apply again ok`) and `cargo test -p fala --lib managers::history_dictations::tests::undo_after_late_edit`

**C10** - `settings.history.applyAiEdit` exists exactly once in each locale and `redoAiEdit` appears nowhere under `src/`; both locales have the same keys and the ESLint rule for literal JSX passes (AC 9)
Proof: `grep -c '"applyAiEdit"' src/i18n/locales/pt/translation.json src/i18n/locales/en/translation.json | grep -c ':1$' | xargs test 2 -eq` and `test "$(grep -rc 'redoAiEdit' src/ | grep -v ':0$' | wc -l)" -eq 0` and `bun run check:translations` and `bun run lint`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| `sensitive` (2) | `true` C1 · `false` C1 | - |
| late answer outcomes (4) | applied C6 · error C7 · after deadline C7 · entry deleted C7 | - |
| `Store::apply_late_edit` statuses (5) | `Ok` C3, C4 · `NotFound` C5 · `EmptyEdit` C5 · `Mirror` C4 (the mirror write path; the error is propagated by `write_mirror` as in `set_showing`) · `Database` C5 | - |
| `showing` × `editor` for the history button (6) | `llm`+`final` → undo C2 · `llm`+`raw` → apply C8 · `rules`+`final` → none C2 · `rules`+`raw` → none C2 · `none`+any → none C2 · unlinked → none C2 | - |
| i18n keys changed (2) | `applyAiEdit` added C8, C10 · `redoAiEdit` removed C10 | - |

- Claims naming a status or response shape: C3, C5, C6, C7 - each has a proof that crosses the boundary (the
  real `Store` or the fake Gemini)
- No other check claims more than the single case its proof exercises

## Swept

- validation: C5 (blank text, unknown id), C1 (case-insensitive app match)
- failure modes: C7
- idempotency: C7 (a late answer for a deleted entry changes nothing); existing - `Store::redo` on an item already `final` is a no-op (`set_showing`)
- authorization: n/a - single-user local app, no new route
- concurrency: C6 (late answer landing after save and before any undo); existing - a cancel before the paste drops `ProcessedTranscription` and its `LateEdit` with it (`complete_unless_cancelled`, cancel-anywhere)
- data lifecycle: existing - deleting an entry deletes its dictation (history-undo); C7 covers the late answer after a delete
- dependency failure: C7 (Gemini down or late)
- state transitions: C3, C6, C8, C9 (`showing` final → raw by late edit, raw → final by apply, final → raw by undo)
- observability: C7 (only `debug` lines for a late answer that is not applied)

## Handoff

- S1 = 15k, S2 = 30k: ~45k in `crates/storage`, `apps/desktop` (`llm_auto`, `actions`, `managers/history*`) and the history front, under the 150k budget - one builder (Opus pane, autonomous; Augusto delegated every decision on 2026-10-09)
- Order: door 1 (`Store::apply_late_edit`, cherry-picked from the abandoned `feat/llm-auto` branch), then S1 `sensitive`, then the S2 desktop half, then the front; the Verifier runs once over `origin/main..HEAD`
- `TODO(windows)` kept outside the table, as a checklist for the Windows session: a dictation of more than 15 words in Notepad on a throttled network (Gemini slower than 2 s) shows the rules text, then the history item offers "Aplicar edição da IA" and applying it copies the Gemini text; a dictation in KeePassXC appears in `fala-cli history search` with `sensitive`. None of these changes a verdict here.
- **Boundary:** C3-C5 fechados em `feat/llm-late-edit` (`Store::apply_late_edit`, door 1, trazido do commit f02742c da `feat/llm-auto` abandonada, com o caso `Database` acrescentado ao C5)
- **Settled mid-build:** a door 1 do plano antigo escrevia `editor = 'llm'`; a coluna de `fala.sqlite` (schema 1) é `edited_by`, como o frontmatter do `.md`; a door deste plano já nasce com `edited_by`, e o C4 confere `edited_by: "llm"` e `showing: "raw"` entre aspas, como o espelho grava. O texto em branco devolve a variante nova `StorageError::EmptyEdit(id)`, conferida antes de ler o item; o `fala-mcp` casa a enum com braço `other`. Confirmed? y — delegado pelo Augusto, decidido pelo executor
- **Boundary:** C1, C6, C7 e a metade Rust do C9 (`undo_after_late_edit`) fechados em `feat/llm-late-edit`
- **Settled mid-build:** a `LateEdit` não é `Clone` nem `Eq`, e os testes de main comparam `AutoFormatted` por igualdade; ela sai por uma função irmã, `llm_auto::format_with_late_edit`, que devolve `(AutoFormatted, Option<LateEdit>)`, e `format` passou a chamá-la e descartar a segunda parte. `ProcessedTranscription` ganhou `late_edit`; só o stop path a usa, e o retry do histórico a descarta (Out of scope). Confirmed? y — delegado pelo Augusto, decidido pelo executor
- **Settled mid-build:** C6 e C7 exercitam as peças que o stop path chama (`format_with_late_edit`, `auto_processed`, `HistoryManager::save_entry_with`, `finish_late_edit` com `HistoryManager::apply_late_edit_with`); o fio do `AppHandle` (o `spawn_blocking` depois do `save_entry` e o `emit` de `announce_updated`) não passa por teste e fica na lista `TODO(windows)`
- **Settled mid-build:** o C1 calcula `sensitive` com `llm_auto::is_disabled_app` (comparação sem caixa, como o `Postprocessor`) sobre o `AppContext` lido ao soltar a tecla, uma vez em `TranscribeAction::stop`, para os dois atalhos (assunção 3); o `NewEntry` ganhou `sensitive`, e o item da transcrição que falhou passa `false`
