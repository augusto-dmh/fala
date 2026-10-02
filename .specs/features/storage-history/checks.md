# storage-history checks

Profile: light
Plan: `.specs/features/storage-history/plan.md`

44 checks in 5 slices · 7 one-way doors · 0 open, of which 0 block

Os checks de 1 a 36 seguem a numeração dos ACs do plano (35 e 36 vieram do rebase sobre o S0).
Os de 37 a 44 fecham os status da `Surface` e as portas que nenhum AC nomeia sozinho.

Comandos: `S=CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`. As provas
abaixo omitem o prefixo; todas rodam com ele.

## Checks

### S1 - gravar e buscar · 5 files · ~40 KB · ~10k

**C1** - `Store::add` devolve um `DictationRecord` com id UUID v7 (versão 7 no `uuid`), `created_at` igual ao passado, bruto, final, `edited_by`, app e `showing = final`, e `get(id)` lê o mesmo registro do banco (AC 1, door 3)
Proof: `cargo test -p fala-storage --test store -- --exact add_returns_v7_record_showing_final`

**C2** - Com a escrita do espelho falhando (`<notes>/Ditados` é um arquivo), a linha já está no banco: `get(id)` e `search` a encontram (AC 2)
Proof: `cargo test -p fala-storage --test store -- --exact mirror_failure_keeps_row`

**C3** - `search("acao")` acha um item cujo final é "Ação de amanhã." (AC 3, door 2)
Proof: `cargo test -p fala-storage --test store -- --exact search_ignores_accents`

**C4** - Uma palavra que só existe no bruto acha o item (AC 4, door 2)
Proof: `cargo test -p fala-storage --test store -- --exact search_matches_raw_only_word`

**C5** - Dois termos exigem os dois (um item com só um deles não volta) e `amanh` casa "amanhã" como prefixo (AC 5)
Proof: `cargo test -p fala-storage --test store -- --exact search_terms_are_anded_prefixes`

**C6** - Consultas com `"`, `*`, `(`, `NEAR` e `-` devolvem `Ok`, nunca erro, e acham o item que contém o texto literal quando ele existe (AC 6)
Proof: `cargo test -p fala-storage --test store -- --exact search_fts_syntax_is_literal`

**C7** - Consulta `""` e `"   "` devolvem os itens mais recentes até o limite (AC 7)
Proof: `cargo test -p fala-storage --test store -- --exact search_empty_returns_recent`

**C8** - Cinco itens com `created_at` fora de ordem voltam em `created_at` decrescente, e `limit = 2` devolve os dois mais novos (AC 8)
Proof: `cargo test -p fala-storage --test store -- --exact search_orders_newest_first_and_limits`

**C9** - Depois de `Store::open`, uma conexão nova lê `journal_mode = wal`, `user_version = 1`, e a conexão do store tem `busy_timeout = 5000` (AC 9, door 1)
Proof: `cargo test -p fala-storage --test store -- --exact open_sets_wal_timeout_and_version`

### S2 - espelho Markdown · 3 files · ~25 KB · ~6k

**C10** - O `.md` de um item criado em 2026-10-02 14:30:22 local fica em `Ditados/2026-10-02/143022-<id>.md`; o frontmatter tem, nesta ordem, `id`, `created_at`, `language`, `app`, `edited_by`, `showing`, `raw`, cada linha `chave: <JSON>`, `app: null` quando não há app, e o corpo é o final seguido de exatamente um `\n` (AC 10, door 4, door 5)
Proof: `cargo test -p fala-storage --test mirror -- --exact mirror_path_and_frontmatter_literal`

**C11** - Bruto e final com `---`, `## x`, aspas, `\n`, `\r\n`, acentos, `\n` no fim e texto vazio passam pelo `.md` e voltam do `reindex` byte a byte iguais (AC 11)
Proof: `cargo test -p fala-storage --test mirror -- --exact hostile_text_round_trips`

**C12** - Com o espelho falhando, `add` devolve `StorageError::Mirror { id }` com o id da linha gravada (AC 12)
Proof: `cargo test -p fala-storage --test store -- --exact mirror_failure_keeps_row`

**C13** - Depois de `add`, `undo` e `redo`, nenhum `*.md.tmp` existe na pasta, e um `.md.tmp` deixado por um crash não muda o `.md` final (AC 13)
Proof: `cargo test -p fala-storage --test mirror -- --exact no_tmp_left_behind`

**C35** - `language` `pt-BR` e `en` são gravados no banco e no frontmatter como a tag BCP-47 e voltam iguais do `reindex`; `language: "es"` faz o arquivo ser ignorado (AC 35, door 7)
Proof: `cargo test -p fala-storage --test reindex -- --exact language_round_trips`
Proof: `cargo test -p fala-storage --test reindex -- --exact invalid_files_are_skipped_with_reason`

### S3 - desfazer e reaplicar · 2 files · ~15 KB · ~4k

**C14** - Para `edited_by` `rules` e `llm`, `undo` põe `showing = raw` no banco e no `.md` (`showing: "raw"`), mantém o final e devolve o registro (AC 14)
Proof: `cargo test -p fala-storage --test store -- --exact undo_shows_raw_and_keeps_final`

**C15** - `redo` depois de `undo` volta a `showing = final` no banco e no `.md` (AC 15)
Proof: `cargo test -p fala-storage --test store -- --exact redo_shows_final_again`

**C16** - `undo` num item em `raw` e `redo` num item em `final` devolvem o registro igual, sem mudar o `.md` (mesmo conteúdo) (AC 16)
Proof: `cargo test -p fala-storage --test store -- --exact undo_and_redo_are_idempotent`

**C17** - `undo` e `redo` num item `edited_by = none` devolvem `StorageError::NothingToUndo` e o banco e o `.md` ficam iguais (AC 17)
Proof: `cargo test -p fala-storage --test store -- --exact unedited_has_nothing_to_undo`

**C18** - `get`, `undo` e `redo` com um id inexistente devolvem `StorageError::NotFound` (AC 18)
Proof: `cargo test -p fala-storage --test store -- --exact unknown_id_is_not_found`

### S4 - reindex · 2 files · ~20 KB · ~5k

**C19** - `reindex` num banco novo recria cada item com o mesmo id, `created_at`, idioma, bruto, final, `edited_by`, app e `showing` do `.md` (AC 19, door 3)
Proof: `cargo test -p fala-storage --test reindex -- --exact reindex_restores_every_field`

**C20** - Gravar 6 itens (dois desfeitos), rodar 4 consultas, apagar `fala.sqlite`, `-wal` e `-shm`, rodar `reindex` e as mesmas 4 consultas devolve os mesmos registros na mesma ordem (AC 20, confirmação da ADR-0006)
Proof: `cargo test -p fala-storage --test reindex -- --exact delete_db_then_reindex_gives_same_search`

**C21** - Sem frontmatter, frontmatter sem fechamento, chave obrigatória faltando, valor que não é JSON, `edited_by` fora da door 5, `showing` fora da door 5 e `language` desconhecido: cada arquivo é ignorado, aparece em `ReindexReport` com o caminho e um motivo não vazio, e os válidos entram (AC 21)
Proof: `cargo test -p fala-storage --test reindex -- --exact invalid_files_are_skipped_with_reason`

**C22** - Dois `.md` com o mesmo id: um entra, o outro aparece em `ReindexReport` com o motivo `id duplicado` e `reindex` devolve `Ok` (AC 22)
Proof: `cargo test -p fala-storage --test reindex -- --exact duplicate_id_keeps_one`

**C23** - Com um trigger injetado que aborta o insert de um id, `reindex` devolve `Err(StorageError::Db)` e o banco mantém exatamente as linhas que tinha antes; sem o trigger, as linhas antigas sem `.md` somem (AC 23)
Proof: `cargo test -p fala-storage --test reindex -- --exact reindex_is_one_transaction`

**C24** - `notas.txt`, `x.md.tmp` e `y.MD.bak` dentro de `Ditados/` não entram nem contam como ignorados (AC 24)
Proof: `cargo test -p fala-storage --test reindex -- --exact non_md_files_are_not_counted`

### S5 - `fala-cli history` e `reindex` · 4 files · ~30 KB · ~8k

**C25** - `history add --raw t` sem `--final` grava final = bruto e `edited_by = none`, imprime só o id (uma linha, UUID) no stdout e sai com 0 (AC 25)
Proof: `cargo test -p fala-cli --test history -- --exact add_without_final_prints_id`

**C26** - `--final` sem `--edited-by` e `--edited-by` sem `--final` saem com 2 e não criam `fala.sqlite` (AC 26)
Proof: `cargo test -p fala-cli --test history -- --exact add_final_and_editor_go_together`

**C27** - `history search acao` imprime o cabeçalho `| id | quando | app | editado_por | mostrando | texto |`, a linha separadora e uma linha por item, com o texto do lado de `showing`, `|` como `\|` e quebra de linha como espaço (AC 27)
Proof: `cargo test -p fala-cli --test history -- --exact search_prints_markdown_table`

**C28** - Busca sem resultado imprime só cabeçalho e separador e sai com 0 (AC 28)
Proof: `cargo test -p fala-cli --test history -- --exact search_without_hits_prints_header_only`

**C29** - `history undo <id>` imprime o bruto, `redo <id>` imprime o final, ambos saem com 0 (AC 29)
Proof: `cargo test -p fala-cli --test history -- --exact undo_and_redo_print_shown_text`

**C30** - `undo` com id inexistente e `undo` num item sem edição saem com 1 com um motivo no stderr e nada no stdout (AC 30)
Proof: `cargo test -p fala-cli --test history -- --exact undo_failures_exit_1`

**C31** - `reindex` com 2 válidos imprime `2 ditados reindexados, 0 arquivos ignorados` e sai com 0; com um inválido a mais imprime `2 ditados reindexados, 1 arquivos ignorados`, o caminho e o motivo no stderr, e sai com 1 (AC 31)
Proof: `cargo test -p fala-cli --test history -- --exact reindex_reports_counts`

**C32** - `reindex` sem `<notes-dir>/Ditados` sai com 2 e não cria `fala.sqlite` (AC 32)
Proof: `cargo test -p fala-cli --test history -- --exact reindex_without_ditados_exits_2`

**C33** - Sem `--data-dir` e `--notes-dir`, a resolução dá `<data_dir>/br.com.augusto.fala/fala.sqlite` e `<data_dir>/br.com.augusto.fala/notas`; com `--data-dir D`, `D/fala.sqlite` e `D/notas`; com os dois, os dois (AC 33, door 6)
Proof: `cargo test -p fala-cli --bin fala-cli -- --exact history::tests::paths_resolve_from_flags_and_data_dir`

**C34** - `add`, `search`, `undo` e `reindex` com `RUST_LOG=info` não escrevem o bruto nem o final no stderr (AC 34)
Proof: `cargo test -p fala-cli --test history -- --exact dictated_text_stays_out_of_logs`

**C36** - `add` sem `--language` grava `pt-BR` (lido no frontmatter); `--language EN` grava `en`; `--language es` sai com 2 (AC 36, door 7)
Proof: `cargo test -p fala-cli --test history -- --exact add_language_defaults_and_validation`

### Status e portas sem AC próprio

**C37** - `history add` com `<notes>/Ditados` sendo um arquivo sai com 1 e o id aparece no stderr (Surface `history add` status 1)
Proof: `cargo test -p fala-cli --test history -- --exact add_mirror_failure_exits_1`

**C38** - `history search` com `--data-dir` apontando para um arquivo sai com 1; `--limit abc` sai com 2 (Surface `history search` status 1 e 2)
Proof: `cargo test -p fala-cli --test history -- --exact search_failures_exit_1_and_2`

**C39** - `history undo` e `history redo` sem id saem com 2 (Surface `history undo`/`redo` status 2)
Proof: `cargo test -p fala-cli --test history -- --exact undo_without_id_exits_2`

**C40** - `Store::open` com o caminho do banco dentro de um arquivo comum devolve `Err(StorageError::Io)` (Surface API `Err(Io)`)
Proof: `cargo test -p fala-storage --test store -- --exact open_under_a_file_is_io_error`

**C41** - O valor gravado em `edited_by` no banco é `none`, `rules` ou `llm` para cada `Editor`, lido por SQL cru (door 5)
Proof: `cargo test -p fala-storage --test store -- --exact editor_literals_in_db`

**C42** - `fala.sqlite` e `history.db` coexistem na mesma pasta: abrir o store numa pasta com um `history.db` de `user_version = 4` não toca esse arquivo (bytes iguais) (door 1, Impact)
Proof: `cargo test -p fala-storage --test store -- --exact leaves_history_db_alone`

**C43** - A tabela `dictations_fts` existe com `tokenize='unicode61 remove_diacritics 2'` e indexa `raw` e `final` (lido em `sqlite_master`) (door 2)
Proof: `cargo test -p fala-storage --test store -- --exact fts_table_shape`

**C44** - `reindex` com `fala.sqlite` inexistente cria o banco e devolve o relatório (Surface API `reindex` Ok em banco novo)
Proof: `cargo test -p fala-storage --test reindex -- --exact reindex_restores_every_field`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| `fala-cli history add` statuses (3) | 0 C25 · 1 C37 · 2 C26 | - |
| `fala-cli history search` statuses (3) | 0 C27 · 1 C38 · 2 C38 | - |
| `fala-cli history undo`/`redo` statuses (3) | 0 C29 · 1 C30 · 2 C39 | - |
| `fala-cli reindex` statuses (3) | 0 C31 · 1 C31 · 2 C32 | - |
| `fala_storage` API outcomes (6) | `Ok` C1 · `NotFound` C18 · `NothingToUndo` C17 · `Mirror` C12 · `Db` C23 · `Io` C40 | - |
| Landing doors (7) | 1 C9 · 2 C43 · 3 C1 · 4 C10 · 5 C41 · 6 C33 · 7 C35 | - |
| `edited_by` values (3) | `none` C17 · `rules` C14 · `llm` C14 | - |
| `showing` transitions (4) | add -> `final` C1 · `final` -> `raw` C14 · `raw` -> `final` C15 · no-op both ways C16 | - |
| FTS5 syntax in a query (5) | C6, table-driven over all 5 (`"`, `*`, `(`, `NEAR`, `-`) | - |
| hostile text in the mirror (8) | C11, table-driven over all 8 (`---`, `## x`, aspas, `\n`, `\r\n`, acentos, `\n` final, vazio) | - |
| reindex rejection reasons (7) | C21, table-driven over all 7; duplicate id C22 | - |
| `language` values (3) | `pt-BR` C35 · `en` C35 · unknown C35 | - |
| stored files in the data dir (2) | `fala.sqlite` C9 · `history.db` untouched C42 | - |

- Claims naming an exit code or output shape: C25–C32, C36–C39 - each runs the binary.
- No other check claims more than the single case its proof exercises.

## Swept

- validation: C21 (frontmatter), C26, C36, C38 (flags), C6 (query)
- failure modes: C2, C12 (espelho), C23 (reindex no meio), C40 (open)
- idempotency: C16 (undo/redo), C22 (id duplicado no reindex), C20 (reindex repetível)
- authorization: n/a - biblioteca e CLI locais, sem usuários nem rede; o arquivo herda as permissões da pasta de dados
- concurrency: C9 (WAL + `busy_timeout` 5000 ms); dois escritores no mesmo `.md` não existem porque o nome é o id (door 3)
- data lifecycle: n/a - apagar e retenção estão fora (plan, Out of scope); `reindex` substitui o banco (C23)
- dependency failure: C2 (disco do espelho), C40 (pasta de dados); nenhuma dependência de rede
- state transitions: C14, C15, C16, C17
- observability: C34 (texto ditado fora do log acima de `debug`); erros nomeados no stderr C30, C31

## Handoff

- S1–S5 tocam `crates/storage/{Cargo.toml,src/*,tests/*}`, `apps/cli/{Cargo.toml,src/main.rs,src/history.rs,tests/history.rs}` e `Cargo.toml`; existente ~6,4 KB + novo estimado ~130 KB escrito e relido = ~33k tokens, abaixo do budget de 150k - one builder
- Mechanism: one builder (cabe no budget, sem pergunta)
