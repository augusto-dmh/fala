# history-undo checks

Profile: light
Plan: `.specs/features/history-undo/plan.md`

30 checks in 6 slices · 5 one-way doors · 0 open, of which 0 block

Comandos de cargo com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`,
rodados na raiz do worktree. Os testes do desktop usam `history.db` em memória (como os testes
herdados de `managers::history`) e um `Store` real em `tempfile::tempdir()`; nenhum precisa de
`AppHandle`. A lógica nova mora em funções livres de `managers/history_dictations.rs` que recebem a
conexão e o `Store`, e o `HistoryManager` só as chama.

## Checks

### S1 - apagar um item em `fala-storage` · 2 files · 25 KB · ~6k

**C1** - Depois de `add` de um ditado com a palavra única "jabuticaba" e `delete(id)`, `get(id)` devolve `NotFound`, `search("jabuticaba", 10)` volta vazio e o `.md` do item não existe mais; um segundo ditado gravado antes continua com `get` ok e com o seu `.md` (AC 1)
Proof: `cargo test -p fala-storage --test store delete_removes_row_fts_and_mirror`

**C2** - `delete` com um id v7 que nunca foi gravado devolve `StorageError::NotFound` com esse id, e o item existente segue com `get` ok, com o `.md` intacto e achado por `search` (AC 2)
Proof: `cargo test -p fala-storage --test store delete_unknown_id_is_not_found`

**C3** - Depois de gravar dois ditados e apagar um, `reindex` devolve `indexed = 1`, `skipped` vazio, e `get` do apagado segue `NotFound` (AC 3)
Proof: `cargo test -p fala-storage --test store reindex_after_delete_does_not_resurrect`

**C4** - `delete` de um item cujo `.md` já foi removido à mão devolve `Ok` e a linha sai (door 5, `.md` ausente não é erro)
Proof: `cargo test -p fala-storage --test store delete_tolerates_missing_mirror`

### S2 - cada ditado entregue vira um item de `fala.sqlite` · 4 files · 75 KB · ~19k

**C5** - `editor_for(raw, pasted, llm_produced)` devolve `None` para ("a", "a", false) e ("a", "a", true), `Llm` para ("a", "A.", true) e `Rules` para ("a", "A.", false) (AC 5, table-driven, 4 linhas)
Proof: `cargo test -p fala --lib managers::history_dictations::tests::editor_follows_pasted_text_and_llm`

**C6** - Salvar uma entrada com bruto "acao de amanha", colado "Ação de amanhã.", LLM pedido e produzido, idioma "en" e app `Some("notepad")` grava 1 linha em `history.db` cujo `dictation_id` é o id de um item de `fala.sqlite` com `raw.text` = "acao de amanha", `final_text` = "Ação de amanhã.", `editor` = `Llm`, `raw.language` = `En`, `app_name` = `Some("notepad")` e `showing` = `Final` (AC 4)
Proof: `cargo test -p fala --lib managers::history_dictations::tests::save_links_new_dictation`

**C7** - O idioma do item vem das settings: "pt-BR" e "pt" gravam `PtBr`, "en" grava `En`, "auto" e "es" gravam `PtBr` (AC 4, table-driven, 5 valores)
Proof: `cargo test -p fala --lib managers::history_dictations::tests::language_falls_back_to_pt_br`

**C8** - Sem store (`None`), salvar grava a linha de `history.db` com o texto e `dictation_id` nulo; com o store aberto mas a tabela `dictations` removida por outra conexão (`Store::add` falha com `Db`), também grava a linha com `dictation_id` nulo (AC 6, 2 casos)
Proof: `cargo test -p fala --lib managers::history_dictations::tests::save_without_store_keeps_row_unlinked`

**C9** - Com `notas/Ditados` ocupado por um arquivo (o espelho falha), salvar grava a linha com `dictation_id` igual ao id que `StorageError::Mirror` informa, e `Store::get` desse id devolve o item (AC 7)
Proof: `cargo test -p fala --lib managers::history_dictations::tests::save_mirror_failure_links_reported_id`

**C10** - Salvar com texto vazio (transcrição que falhou) grava a linha com `transcription_text` vazio e `dictation_id` nulo, e `Store::search("", 10)` segue vazio (AC 8)
Proof: `cargo test -p fala --lib managers::history_dictations::tests::failed_transcription_saves_unlinked_row`

**C11** - Em `actions.rs`, o único `hm.save_entry(` que recebe o texto colado está dentro do closure `save_history` entregue a `deliver_unless_cancelled`, `actions.rs` não usa `fala_storage`, e a entrega tudo-ou-nada da 1.F2 segue verde (AC 9)
Proof: `awk '/let save_history = move \|\| \{/,/^                            \};/' apps/desktop/src/actions.rs | grep -q "hm.save_entry("`
Proof: `! grep -q "fala_storage" apps/desktop/src/actions.rs`
Proof: `cargo test -p fala --lib actions::tests::delivery_is_all_or_nothing_after_the_last_check`

**C12** - O closure `save_history` passa a `save_entry` o texto colado (`&pasted_text`) e `fala_inject::foreground_app()` (AC 10)
Proof: `awk '/let save_history = move \|\| \{/,/^                            \};/' apps/desktop/src/actions.rs | grep -q "fala_inject::foreground_app()"`
Proof: `awk '/let save_history = move \|\| \{/,/^                            \};/' apps/desktop/src/actions.rs | grep -q "&pasted_text"`

**C13** - TODO(windows): no build do Windows com o LLM ligado, um ditado de mais de 15 palavras no Bloco de Notas aparece no histórico com o texto colado e "em notepad", e `fala-cli history search <palavra>` acha o mesmo item (AC 4, AC 10, AC 27 no app real)
Proof: `TODO(windows)` manual - 1 ditado; registrar o texto mostrado, o app mostrado e a saída do `fala-cli history search`

### S3 - o `history.db` existente chega a `fala.sqlite` · 2 files · 40 KB · ~10k

**C14** - Uma conexão em `user_version` 4 com uma linha, levada por `MIGRATIONS` a `to_latest`, fica em `user_version` 5 com a coluna `dictation_id` aceitando nulo, e a linha existente tem `dictation_id` nulo (AC 11)
Proof: `cargo test -p fala --lib managers::history::tests::migration_five_adds_nullable_dictation_id`

**C15** - O backfill sobre 4 linhas (só bruto; `post_processed_text` com `post_process_requested`; `post_processed_text` sem pedido; texto vazio) cria 3 itens: `None` com final = bruto, `Llm` com final = pós-processado, `Rules` com final = pós-processado; cada um com `app_name` nulo, `raw.language` = o idioma passado e `created_at` igual ao `timestamp` da linha (mesmo instante, offset local); a linha de texto vazio fica com `dictation_id` nulo (AC 12, door 4, 4 linhas)
Proof: `cargo test -p fala --lib managers::history_dictations::tests::backfill_maps_rows_by_door_four`

**C16** - Rodar o backfill duas vezes deixa `Store::search("", 100)` com o mesmo número de itens e os mesmos `dictation_id` (AC 13)
Proof: `cargo test -p fala --lib managers::history_dictations::tests::backfill_is_idempotent`

**C17** - `open_store` com o caminho do banco sob um arquivo comum devolve `None` (sem pânico), e `HistoryManager::new` obtém o store por `open_store(` sem `?` nem `expect` (AC 14)
Proof: `cargo test -p fala --lib managers::history_dictations::tests::open_store_failure_returns_none`
Proof: `grep -q "let store = history_dictations::open_store(" apps/desktop/src/managers/history.rs`

### S4 - apagar, reter e retranscrever valem para os dois bancos · 2 files · 40 KB · ~10k

**C18** - Apagar uma entrada vinculada remove o item de `fala.sqlite` (`get` = `NotFound`, `.md` ausente), a linha de `history.db` e o WAV (AC 15)
Proof: `cargo test -p fala --lib managers::history_dictations::tests::delete_linked_removes_dictation_row_and_wav`

**C19** - Com a tabela `dictations` removida por outra conexão (`Store::delete` falha com `Db`), apagar devolve `Err` e mantém a linha e o WAV; com um `dictation_id` que não existe no store (`NotFound`), apagar devolve `Ok` e remove a linha e o WAV (AC 16, 2 casos)
Proof: `cargo test -p fala --lib managers::history_dictations::tests::delete_failure_keeps_row_and_wav`

**C20** - A limpeza por quantidade com limite 1 sobre 3 entradas vinculadas apaga os 2 itens mais antigos de `fala.sqlite` com as suas linhas; com `Store::delete` falhando (`Db`), a limpeza por quantidade e a por tempo mantêm as entradas e as suas linhas (AC 17)
Proof: `cargo test -p fala --lib managers::history_dictations::tests::retention_deletes_dictations_or_keeps_entry`

**C21** - O retry de uma entrada vinculada (app "slack") com os textos novos grava um item novo com `raw.text`, `final_text` e `editor` novos e `app_name` = "slack", põe o id novo em `dictation_id` e apaga o item anterior (`get` = `NotFound`) (AC 18)
Proof: `cargo test -p fala --lib managers::history_dictations::tests::retry_replaces_dictation_keeping_app`

### S5 - desfazer e reaplicar a edição · 4 files · 40 KB · ~10k

**C22** - `set_showing_with(Raw)` numa entrada vinculada com `editor = Llm` devolve a entrada com `dictation.showing = raw`, `Store::get` do item mostra `Raw` e o `.md` tem `showing: "raw"`; `set_showing_with(Final)` em seguida devolve `showing = final` e o `.md` volta a `showing: "final"` (AC 19, AC 20)
Proof: `cargo test -p fala --lib managers::history_dictations::tests::undo_then_redo_switches_showing`

**C23** - `set_showing_with` devolve `Err` e não muda `showing` nem o `.md` em 4 casos: id inexistente, entrada sem `dictation_id`, item com `editor = None`, store `None` (AC 21, 4 casos)
Proof: `cargo test -p fala --lib managers::history_dictations::tests::undo_errors_change_nothing`

**C24** - `HistoryManager::set_showing` emite `HistoryUpdatePayload::Updated` com a entrada devolvida; os comandos `undo_history_entry_edit` e `redo_history_entry_edit` estão em `collect_commands!` de `lib.rs` e chamam `set_showing` com `Showing::Raw` e `Showing::Final` (AC 19, AC 20)
Proof: `awk '/pub fn set_showing\(/,/^    }/' apps/desktop/src/managers/history.rs | grep -q "HistoryUpdatePayload::Updated"`
Proof: `grep -q "commands::history::undo_history_entry_edit," apps/desktop/src/lib.rs`
Proof: `grep -q "commands::history::redo_history_entry_edit," apps/desktop/src/lib.rs`
Proof: `awk '/pub async fn undo_history_entry_edit/,/^}/' apps/desktop/src/commands/history.rs | grep -q "Showing::Raw"`
Proof: `awk '/pub async fn redo_history_entry_edit/,/^}/' apps/desktop/src/commands/history.rs | grep -q "Showing::Final"`

**C25** - A leitura paginada completa cada entrada vinculada com `dictation` (bruto, final, `editor`, `showing`, `app_name`) e deixa `dictation` nulo na entrada sem vínculo (AC 22, lado do backend)
Proof: `cargo test -p fala --lib managers::history_dictations::tests::entries_carry_their_dictation`

**C26** - `src/bindings.ts`, regenerado pelo export do `tauri-specta` (não editado à mão), tem `undoHistoryEntryEdit` e `redoHistoryEntryEdit` e os campos `dictation_id` e `dictation` em `HistoryEntry` (Impact `generated`)
Proof: `grep -q "async undoHistoryEntryEdit(id: number)" src/bindings.ts`
Proof: `grep -q "async redoHistoryEntryEdit(id: number)" src/bindings.ts`
Proof: `awk '/^export type HistoryEntry = /,/}/' src/bindings.ts | grep -q "dictation_id: string | null"`

### S6 - o histórico mostra o texto colado e desfaz a edição da IA · 4 files · 80 KB · ~6k (só o histórico e as chaves novas)

**C27** - `shownText` devolve o final de uma entrada vinculada que mostra `final`, o bruto da que mostra `raw` e `transcription_text` da sem vínculo; `HistorySettings` mostra `shownText(entry)` e o botão de copiar copia `shownText(entry)` (AC 22, AC 23)
Proof: `bun src/components/settings/history/historyModel.test.ts`

**C28** - `editAction` devolve `"undo"` para `editor = llm` mostrando `final`, `"redo"` para `llm` mostrando `raw`, e `null` para `rules`, `none` e sem vínculo; `HistorySettings` só desenha o botão quando `editAction` não é nulo, com o título `settings.history.undoAiEdit` ou `settings.history.redoAiEdit` (AC 24, table-driven, 5 casos)
Proof: `bun src/components/settings/history/historyModel.test.ts`

**C29** - O handler de desfazer e reaplicar troca a entrada pela devolvida e copia `shownText` dela, com `settings.history.originalCopied` depois de desfazer e `settings.history.editedCopied` depois de reaplicar; na falha mostra `settings.history.editToggleError` sem trocar a entrada; `appName` devolve o app ou nulo, e a tela usa `settings.history.inApp` com `{{app}}` só quando há app (AC 25, AC 26, AC 27)
Proof: `bun src/components/settings/history/historyModel.test.ts`

**C30** - As chaves novas valem em pt "Desfazer edição da IA", "Reaplicar edição da IA", "Texto original copiado", "Texto editado copiado", "Não foi possível trocar o texto", "em {{app}}" e em en "Undo AI edit", "Reapply AI edit", "Original text copied", "Edited text copied", "Couldn't switch the text", "in {{app}}"; lint, tipos, formatação e traduções seguem verdes (AC 28)
Proof: `bun src/components/settings/history/historyModel.test.ts`
Proof: `bun run lint`
Proof: `bunx tsc --noEmit`
Proof: `bun run format:check`
Proof: `bun run check:translations`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| `editor` do item novo (3) | `none` C5 · `llm` C5, C6 · `rules` C5 | - |
| idioma das settings (5) | "pt-BR" C7 · "pt" C7 · "en" C7, C6 · "auto" C7 · "es" C7 | - |
| resultados do `Store::add` ao salvar (4) | `Ok` C6 · store fechado C8 · `Db` C8 · `Mirror` C9 | - |
| texto do ASR ao salvar (2) | não vazio C6 · vazio (transcrição falha) C10 | - |
| linhas do backfill (4) | só bruto C15 · pós-processado pedido C15 · pós-processado sem pedido C15 · texto vazio C15 | - |
| execuções do backfill (2) | primeira C15 · repetida C16 | - |
| resultados do `Store::delete` ao apagar (4) | `Ok` C18 · `NotFound` C19, C2 · `Db` C19 · `.md` ausente C4 | - |
| caminhos que apagam (3) | apagar pela tela C18, C19 · retenção por quantidade C20 · retenção por tempo C20 | - |
| `invoke undo_history_entry_edit` statuses (5) | `ok` C22 · `err-not-found` C23 · `err-unlinked` C23 · `err-nothing-to-undo` C23 · `err-store-unavailable` C23 | - |
| `invoke redo_history_entry_edit` statuses (5) | `ok` C22 · `err-not-found` C23 · `err-unlinked` C23 · `err-nothing-to-undo` C23 · `err-store-unavailable` C23 (mesma função, `Showing::Final`, C24) | - |
| `invoke get_history_entries` statuses (2) | `ok` com `dictation` C25 · `err` herdado, sem mudança C25 | - |
| texto mostrado (3) | vinculado `final` C27 · vinculado `raw` C27 · sem vínculo C27 | - |
| botão de edição (5) | `llm`+`final` C28 · `llm`+`raw` C28 · `rules` C28 · `none` C28 · sem vínculo C28 | - |
| locales (2) | pt C30 · en C30 | - |
| door 1 (vínculo) (1) | C14, C6 | - |
| door 2 (`history.db` mantido) (1) | C14 (a migração é aditiva e roda no mesmo arquivo), C17 | - |
| door 3 (pasta do store) (1) | C17 (`open_store` recebe `<app_data_dir>/fala.sqlite` e `notas`), C13 | - |
| door 4 (backfill) (1) | C15 | - |
| door 5 (`Store::delete`) (1) | C1, C2, C4 | - |

- C13 is the only check not settled on Linux; it is `TODO(windows)` and stays unchecked
- Claims naming a set: C5, C7, C15, C23, C28 - each proof is table-driven over the whole set
- No other check claims more than the cases its proof exercises

## Swept

- validation: C7 (idioma fora de pt/en cai em pt-BR); C10 (texto vazio não vira item)
- failure modes: C8, C9 (store fechado ou falha no `add` não perde a linha herdada); C19 (falha ao apagar não deixa item meio apagado); C17 (store que não abre não derruba o app)
- idempotency: C16 (backfill repetido não duplica); C22 (reaplicar sobre `final` não muda nada pelo `Store::redo`)
- authorization: n/a - comandos locais da própria janela, sem chamador externo
- concurrency: existing - o `Store` fica atrás de um `Mutex` no `HistoryManager` e o `fala.sqlite` usa WAL com `busy_timeout` de 5 s (door 1 da storage-history), o que cobre a CLI gravando ao mesmo tempo; C11 (a gravação só acontece no passo final da entrega, depois da última checagem de cancelamento)
- data lifecycle: C18, C20 (apagar e retenção valem para `fala.sqlite` e os `.md`); C21 (retry não deixa item velho); C15 (backfill de dados existentes)
- dependency failure: C8 (falha do SQLite no `add`), C19 (no `delete`), C17 (na abertura)
- state transitions: C22, C23 (`showing` só alterna em item editado)
- observability: n/a - falhas vão a `error!` com o id e o caminho, sem conteúdo ditado (AGENTS.md); sem métrica nem trace nesta feature

## Handoff

- S1 = 6k, S2 = 19k, S3 = 10k, S4 = 10k, S5 = 10k (+12k para ler o `bindings.ts` gerado), S6 = 6k = ~73k, em `crates/storage`, `apps/desktop` e no front, abaixo do budget de 150k - one builder
- **Boundary:** C1-C4 fechados em `feat/history-undo`
