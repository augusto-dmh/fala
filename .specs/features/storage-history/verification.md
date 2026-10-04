# storage-history verification

**Verdict**: PASS
**Profile**: light
**Diff range**: ba3b6df..e6a6a37 (HEAD = `e6a6a37afcd2bf04be1d61d249aaf6c0d75c043c`; commits 751c2e2 feat(storage), fe78dba feat(cli), e6a6a37 feat(storage) sensitive). A rodada 1 cobriu ba3b6df..099d188 (commits 7779433, 099d188); a rodada 2, ba3b6df..fe78dba.
**Round**: 3 - scoped (C45 novo, mais regressão de C1-C44 pelo diff `fe78dba..e6a6a37`). As seções da rodada 1 valem como `carried from 099d188` e as da rodada 2 como `carried from fe78dba`, exceto o que `## Round 3 (scoped: C45, regressão em C1-C44)` no fim refaz (C45, C9, C10 e as citações de `crates/storage/tests/mirror.rs`).
**Verifier**: sub-agente independente (author != verifier), disparado pelo orquestrador da feature; não escreveu nem alterou código

Entradas lidas inteiras: `references/verify.md`, `plan.md`, `checks.md` (Profile: light, 44 checks),
`docs/decisions/0006-storage-sqlite-fts5-com-espelho-markdown.md` (fonte binding) e o diff
`ba3b6df..HEAD` (`crates/storage/{Cargo.toml,src/*,tests/*}`, `apps/cli/{Cargo.toml,src/main.rs,src/history.rs,tests/history.rs}`,
`Cargo.toml`, `Cargo.lock`). Todos os 44 checks foram verificados; nenhum foi herdado de rodada anterior.

## Provas rodadas no HEAD

Duas invocações (com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`):

1. `cargo test -p fala-storage --tests` - exit 0. `tests/mirror.rs` 3 passed; `tests/reindex.rs` 7 passed;
   `tests/store.rs` 18 passed; unittests 0. Total 28 passed, 0 failed.
2. `cargo test -p fala-cli --test history --bin fala-cli` - exit 0. `tests/history.rs` 13 passed;
   unittests do binário 16 passed (inclui `history::tests::paths_resolve_from_flags_and_data_dir` e
   `history::tests::cells_escape_pipes_and_newlines`). Total 29 passed, 0 failed.

Cada teste nomeado em `checks.md` aparece individualmente como `... ok` na saída (37 nomes distintos
para 44 checks; C12 reusa a prova de C2, C44 a de C19, C35 usa duas provas). Nenhum filtro vazio: os
37 nomes foram encontrados no código com `rg -n "fn <nome>"` e todos são arquivos novos do diff
(`crates/storage/tests/*.rs`, `apps/cli/tests/history.rs`, `apps/cli/src/history.rs`).

## Binding sources

Passo 1 só é exigido em `ui`; registro a leitura da ADR-0006 porque o plano a marca como binding.

| Source | Opened | Contradiction | Uncovered |
| --- | --- | --- | --- |
| `docs/decisions/0006-storage-sqlite-fts5-com-espelho-markdown.md` | sim, lida inteira | none | - |

ADR-0006 x código: FTS5 `unicode61 remove_diacritics 2` (`crates/storage/src/store.rs:30`), WAL
(`store.rs:61`), SQLite primeiro e espelho depois (`store.rs:95` `insert` antes de `store.rs:97`
`write_mirror`), `fala-cli reindex` (`apps/cli/src/main.rs:70`), teste de round-trip apagar banco ->
reindex -> mesma busca (C20).

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | `add` devolve registro v7, `created_at`, campos, `showing = final`; `get` lê igual | `store` `add_returns_v7_record_showing_final` ok | `crates/storage/tests/store.rs:29` - `assert_eq!(uuid.get_version_num(), 7)`; `:31` `record.created_at == at(14,30,22)`; `:32` `record.dictation == d`; `:33` `record.showing == Showing::Final`; `:34` `store.get(&record.id).unwrap() == record` | PASS |
| C2 | espelho falha e a linha fica no banco e na busca | `store` `mirror_failure_keeps_row` ok | `crates/storage/tests/store.rs:51-52` - `store.get(&id).unwrap()` com `final_text == "reunião às três"`; `:54-55` `hits.len() == 1`, `hits[0].id == id` | PASS |
| C3 | `search("acao")` acha "Ação de amanhã." | `store` `search_ignores_accents` ok | `crates/storage/tests/store.rs:70-71` - `search("acao")` ids `== [&r.id]` | PASS |
| C4 | palavra só do bruto acha o item | `store` `search_matches_raw_only_word` ok | `crates/storage/tests/store.rs:89-91` - `search("tipo")`, `hits.len() == 1`, `hits[0].id == r.id` | PASS |
| C5 | dois termos com E, `amanh` como prefixo | `store` `search_terms_are_anded_prefixes` ok | `crates/storage/tests/store.rs:108-109` - `search("reuniao amanh")` ids `== [&both.id]` (os itens com só um termo ficam de fora) | PASS |
| C6 | `"`, `*`, `(`, `NEAR`, `-` são literais, nunca erro | `store` `search_fts_syntax_is_literal` ok | `crates/storage/tests/store.rs:130` - `unwrap_or_else(panic)` em erro; `:131-134` `hits.iter().any(...)` com `h.id == ids[i]` para os 5 casos; `:137` `search(query).is_ok()` para `"`, `*`, `(`, `-`, `NEAR(`, `a AND`, `OR` | PASS |
| C7 | `""` e `"   "` devolvem os mais recentes até o limite | `store` `search_empty_returns_recent` ok | `crates/storage/tests/store.rs:148-153` - para `""` e `"   "` com limit 2, ids `== [&c.id, &b.id]` | PASS |
| C8 | ordem `created_at` decrescente e `limit` | `store` `search_orders_newest_first_and_limits` ok | `crates/storage/tests/store.rs:174` - `got == want` (minutos 4..0); `:176-179` `limit 2 == want[..2]` | PASS |
| C9 | WAL, `user_version = 1`, `busy_timeout = 5000` | `store` `open_sets_wal_timeout_and_version` ok | `crates/storage/tests/store.rs:186` - `busy_timeout_ms() == 5000`; `:192` `mode == "wal"`; `:196` `version == 1` (conexão nova) | PASS |
| C10 | caminho `2026-10-02/143022-<id>.md`, frontmatter literal em ordem, `app: null`, corpo + um `\n` | `mirror` `mirror_path_and_frontmatter_literal` ok | `crates/storage/tests/mirror.rs:143-160` - `assert_eq!(read_to_string(Ditados/2026-10-02/143022-<id>.md), want)` com o arquivo inteiro literal; `:167` `md.contains("\napp: null\n")`; `:169` `ends_with("---\nOi.\n")` | PASS |
| C11 | texto hostil volta byte a byte do `reindex` | `mirror` `hostile_text_round_trips` ok | `crates/storage/tests/mirror.rs:200-203` - `report.skipped.is_empty()`, `indexed == ids.len()`, `store.get(id) == record` para 9 casos x 2 (bruto+final e só bruto) | PASS |
| C12 | `add` devolve `StorageError::Mirror { id }` com o id gravado | `store` `mirror_failure_keeps_row` ok | `crates/storage/tests/store.rs:48-50` - `let StorageError::Mirror { id, .. } = err else { panic! }`; `:51` `store.get(&id).unwrap()` | PASS |
| C13 | nenhum `.md.tmp` sobra; `.tmp` de crash não muda o `.md` | `mirror` `no_tmp_left_behind` ok | `crates/storage/tests/mirror.rs:225` - `tmps(&env) == 0`; `:231` `read_to_string(&md) == good`; `:239` `tmps(&env) == 0` após nova escrita | PASS |
| C14 | `undo` em `rules` e `llm`: `raw` no banco e no `.md`, final mantido | `store` `undo_shows_raw_and_keeps_final` ok | `crates/storage/tests/store.rs:229-232` - `showing == Raw`, `final_text == "Acho que sim."`, `get == undone`; `:234` `md.contains("\nshowing: \"raw\"\n")` (loop sobre `Rules` e `Llm`) | PASS |
| C15 | `redo` volta a `final` no banco e no `.md` | `store` `redo_shows_final_again` ok | `crates/storage/tests/store.rs:251` - `showing == Final`; `:253` `get(...).showing == Final`; `:255` `md.contains("\nshowing: \"final\"\n")` | PASS |
| C16 | `undo` em `raw` e `redo` em `final` não mudam nada | `store` `undo_and_redo_are_idempotent` ok | `crates/storage/tests/store.rs:271-272` - `redo == r`, `.md == md_final`; `:276-277` `undo == once`, `.md == md_raw` | PASS |
| C17 | `edited_by = none` dá `NothingToUndo`, banco e `.md` iguais | `store` `unedited_has_nothing_to_undo` ok | `crates/storage/tests/store.rs:287-294` - `matches!(undo, Err(NothingToUndo(id)) if id == r.id)` e `matches!(redo, Err(NothingToUndo(_)))`; `:295-296` `get == r`, `.md == before` | PASS |
| C18 | id inexistente dá `NotFound` em `get`, `undo`, `redo` | `store` `unknown_id_is_not_found` ok | `crates/storage/tests/store.rs:305-313` - `matches!(get/undo/redo(missing), Err(StorageError::NotFound(_)))` | PASS |
| C19 | `reindex` recria cada campo do `.md` | `reindex` `reindex_restores_every_field` ok | `crates/storage/tests/reindex.rs:42-46` - `indexed == 3`, `skipped.is_empty()`, `get(id) == want` para `a` (llm, app), `b` (rules, en, desfeito), `c` (none); `:47` `showing == Raw` | PASS |
| C20 | apagar banco + `-wal`/`-shm`, `reindex`, mesmas 4 consultas iguais | `reindex` `delete_db_then_reindex_gives_same_search` ok | `crates/storage/tests/reindex.rs:91` - todas as 4 consultas não vazias antes; `:101` `assert_eq!(after, before)` (registros completos, em ordem) | PASS |
| C21 | 7 tipos de `.md` inválido ignorados com caminho e motivo | `reindex` `invalid_files_are_skipped_with_reason` ok | `crates/storage/tests/reindex.rs:202-203` - `indexed == 1`, `skipped.len() == cases.len()` (7); `:205-210` cada caminho achado em `skipped` e `!skip.reason.trim().is_empty()`; `:212` o válido entra | PASS |
| C22 | id duplicado: um entra, o outro `id duplicado`, `Ok` | `reindex` `duplicate_id_keeps_one` ok | `crates/storage/tests/reindex.rs:229-232` - `reindex().unwrap()`, `indexed == 1`, `skipped.len() == 1`, `reason == "id duplicado"` | PASS |
| C23 | falha no meio: `Err(Db)` e banco intacto; sem falha, órfãos somem | `reindex` `reindex_is_one_transaction` ok | `crates/storage/tests/reindex.rs:253` - `matches!(err, StorageError::Db(_))`; `:266-267` ids `== [a, b, orphan]`, `get(orphan) == orphan`; `:273-278` depois sem trigger: ids `== [a, b]`, `get(orphan)` é `NotFound` | PASS |
| C24 | `notas.txt`, `x.md.tmp`, `y.MD.bak` não contam | `reindex` `non_md_files_are_not_counted` ok | `crates/storage/tests/reindex.rs:295-296` - `indexed == 1`, `skipped.is_empty()` | PASS |
| C25 | `add --raw` sem `--final`: final = bruto, `none`, só o id, exit 0 | `history` `add_without_final_prints_id` ok | `apps/cli/tests/history.rs:100` - `status.code() == Some(0)`; `:102-104` uma linha, `is_uuid(id)`; `:106` `edited_by: "none"`; `:108` `ends_with("---\ncompra pão\n")` | PASS |
| C26 | `--final` sem `--edited-by` e vice-versa: exit 2, sem `fala.sqlite` | `history` `add_final_and_editor_go_together` ok | `apps/cli/tests/history.rs:115` e `:117` - `status.code() == Some(2)`; `:118` `!d.db().exists()` | PASS |
| C27 | tabela Markdown com cabeçalho, separador, linha por item, barra vertical escapada e quebra como espaço | `history` `search_prints_markdown_table` ok | `apps/cli/tests/history.rs:141-143` - 4 linhas, `lines[0] == HEADER`, `lines[1] == SEPARATOR`; `:156` `row[5]` igual a `Ação a <barra escapada> b segunda linha` (barra vertical vira barra invertida + barra, quebra vira espaço); `:164` depois do undo a linha traz `raw` e o bruto `acao a <barra escapada> b` | PASS |
| C28 | sem resultado: só cabeçalho e separador, exit 0 | `history` `search_without_hits_prints_header_only` ok | `apps/cli/tests/history.rs:172-173` - `code == Some(0)`, `stdout == "{HEADER}\n{SEPARATOR}\n"` | PASS |
| C29 | `undo` imprime o bruto, `redo` o final, exit 0 | `history` `undo_and_redo_print_shown_text` ok | `apps/cli/tests/history.rs:188-192` - `Some(0)`, `stdout == "acho que sim\n"`; `Some(0)`, `stdout == "Acho que sim.\n"` | PASS |
| C30 | id inexistente e item sem edição: exit 1, motivo no stderr, stdout vazio | `history` `undo_failures_exit_1` ok | `apps/cli/tests/history.rs:201-203` - `code == Some(1)`, `stdout.is_empty()`, `stderr.contains(id)` | PASS |
| C31 | contagens exatas no stdout; exit 0 com M=0 e 1 com M>0; caminho no stderr | `history` `reindex_reports_counts` ok | `apps/cli/tests/history.rs:214-215` - `Some(0)`, `"2 ditados reindexados, 0 arquivos ignorados\n"`; `:222-228` `Some(1)`, `"2 ditados reindexados, 1 arquivos ignorados\n"`, `stderr.contains("<caminho>: ")` | PASS |
| C32 | sem `Ditados/`: exit 2 e sem `fala.sqlite` | `history` `reindex_without_ditados_exits_2` ok | `apps/cli/tests/history.rs:237-238` - `code == Some(2)`, `!d.db().exists()` | PASS |
| C33 | resolução das pastas padrão e por flag | `fala-cli` bin `history::tests::paths_resolve_from_flags_and_data_dir` ok | `apps/cli/src/history.rs:243-249` - `Paths { db: /os/data/br.com.augusto.fala/fala.sqlite, notes: /os/data/br.com.augusto.fala/notas }`; `:251-252` `/d/fala.sqlite`, `/d/notas`; `:254-255` `/d/fala.sqlite`, `/n`; `:259-265` default real a partir de `dirs::data_dir()` | PASS |
| C34 | texto ditado fora do stderr com `RUST_LOG=info` | `history` `dictated_text_stays_out_of_logs` ok | `apps/cli/tests/history.rs:272` - `!errs.to_lowercase().contains("palavrasecreta")` sobre o stderr de `add`, `search`, `undo` e `reindex` | PASS |
| C35 | `language` `pt-BR`/`en` no banco e frontmatter, volta do `reindex`; `es` é ignorado | `reindex` `language_round_trips` ok; `reindex` `invalid_files_are_skipped_with_reason` ok | `crates/storage/tests/reindex.rs:114` e `:116` - `language: "pt-BR"` / `"en"` no `.md`; `:125` `stored == "en"` no banco; `:132-139` `PtBr` e `En` depois do `reindex`; `:187-191` caso `language: "es"` contado em `:203-210` | PASS |
| C36 | `--language` default `pt-BR`, `EN` vira `en`, `es` sai com 2 | `history` `add_language_defaults_and_validation` ok | `apps/cli/tests/history.rs:285-286` - frontmatter com `language: "pt-BR"` e `language: "en"`; `:288` `code == Some(2)` para `es` | PASS |
| C37 | espelho falha na CLI: exit 1, id no stderr | `history` `add_mirror_failure_exits_1` ok | `apps/cli/tests/history.rs:297` - `code == Some(1)`; `:299-302` um UUID achado no stderr; `:303` `db().exists()` | PASS |
| C38 | `search` com `--data-dir` arquivo sai 1; `--limit abc` sai 2 | `history` `search_failures_exit_1_and_2` ok | `apps/cli/tests/history.rs:312` - `Some(1)`; `:315` - `Some(2)` | PASS |
| C39 | `undo`/`redo` sem id saem com 2 | `history` `undo_without_id_exits_2` ok | `apps/cli/tests/history.rs:323` - `code == Some(2)` para `undo` e `redo` | PASS |
| C40 | `Store::open` sob arquivo comum dá `Err(Io)` | `store` `open_under_a_file_is_io_error` ok | `crates/storage/tests/store.rs:322` - `matches!(err, StorageError::Io(_))` | PASS |
| C41 | `edited_by` no banco é `none`/`rules`/`llm` por SQL cru | `store` `editor_literals_in_db` ok | `crates/storage/tests/store.rs:346` - `got == want` para `["none", "rules", "llm"]`; `:352` `showing == "final"` | PASS |
| C42 | `history.db` com `user_version = 4` fica com os mesmos bytes | `store` `leaves_history_db_alone` ok | `crates/storage/tests/store.rs:373` - `fs::read(&history) == before`; `:374` `env.db.exists()` | PASS |
| C43 | `dictations_fts` com `unicode61 remove_diacritics 2`, indexando `raw` e `final` | `store` `fts_table_shape` ok | `crates/storage/tests/store.rs:389-394` - `sql.contains("fts5(raw, final")`, `content='dictations'`, `content_rowid='rowid'`, `tokenize='unicode61 remove_diacritics 2'` | PASS |
| C44 | `reindex` com `fala.sqlite` inexistente cria o banco e devolve o relatório | `reindex` `reindex_restores_every_field` ok | `crates/storage/tests/reindex.rs:36` - `!env.db.exists()`; `:41-42` `env.db.exists()`, `report.indexed == 3` | PASS |

## Nível e amostragem

- Todo check que nomeia exit code ou formato de saída (C25-C32, C36-C39, C34) roda o binário real
  (`env!("CARGO_BIN_EXE_fala-cli")`, `apps/cli/tests/history.rs:69`): nível certo, sem level gap.
- Os checks da API (C1-C24, C35, C40-C44) rodam pela API pública de `fala_storage` em disco real;
  os literais das portas (C9, C41, C43, C35 no banco) são lidos por uma conexão `rusqlite` à parte,
  não pela própria API.
- C33 é teste de unidade de `resolve` com o `data_dir` injetado, mais uma asserção contra o
  `dirs::data_dir()` real. Aceito: provar o padrão pelo binário escreveria na pasta de dados real do usuário.
- Amostragem: C6 5/5 sintaxes (mais 7 consultas cruas), C11 9 textos x 2 formas (cobre os 8 da lista),
  C21 7/7 motivos, C14 2/2 editores, C41 3/3 literais, C35 2 idiomas + 1 inválido, C26 2/2 combinações.
  Nenhum claim de N casos provado com menos.

## Swept (relido contra o código)

- concurrency: "dois escritores no mesmo `.md` não existem porque o nome é o id" - presente em
  `crates/storage/src/mirror.rs:26` (`format!("{}-{}.md", at.format("%H%M%S"), record.id)`); WAL e
  `busy_timeout` 5000 em `crates/storage/src/store.rs:60-61`, e `store.rs:198-214` dos testes prova que um
  escritor segurando o lock 300 ms não faz o `add` falhar.
- data lifecycle: "`reindex` substitui o banco (C23)" - `crates/storage/src/store.rs:186-191`
  (`transaction`, `DELETE FROM dictations`, inserts, `commit`).
- failure modes / idempotency / state transitions / validation / observability / dependency failure:
  apontam para checks novos desta feature, verificados na tabela acima.
- authorization e o resto de data lifecycle: `n/a` aprovado pelo usuário; nada a conferir no código.

## Faults injected

Não exigido em `light`; feito porque era barato, direto na árvore (como o orquestrador pediu) e
revertido com `git checkout -- <arquivo>`. `git status --porcelain` vazio antes; depois, só este `verification.md` (`git diff` de `crates/` vazio).

| Mutation | Location | Covering proof | Killed |
| --- | --- | --- | --- |
| busca FTS `ORDER BY created_ms DESC, id DESC` -> `ASC, ASC` | `crates/storage/src/store.rs:142` | `search_orders_newest_first_and_limits` falhou em `store.rs:174` dos testes | yes |
| guarda `editor == Editor::None` desligada (`if false && ...`) | `crates/storage/src/store.rs:203` | `unedited_has_nothing_to_undo` falhou em `store.rs:287` dos testes | yes |

## Achados (não bloqueiam; nenhum muda um Result)

1. **Lacuna de precisão em C21** (sobre o check, não sobre o código). O claim pede só "um motivo não
   vazio" e a asserção (`crates/storage/tests/reindex.rs:210`) confere só isso. Por isso o caso
   `sem-fechamento` (`reindex.rs:156-160`) passa por outro caminho: tirar o `---` de fechamento deixa a
   linha `Bom.` dentro do frontmatter e o parser recusa em `crates/storage/src/mirror.rs:93`
   ("linha de frontmatter sem `chave:`"), não no ramo "frontmatter sem a linha `---` de fechamento" de
   `mirror.rs:86`. O arquivo continua ignorado com motivo (o claim e o AC 21 valem), mas `mirror.rs:86`
   não é exercitado por nenhum teste, e um motivo errado passaria. Sugestão: asserir um trecho do
   motivo por caso.
2. **Precisão em C30**: a asserção `apps/cli/tests/history.rs:203` confere que o stderr contém o id, não
   que traz o motivo ("não encontrado" / "não tem edição"). As mensagens existem
   (`crates/storage/src/lib.rs:89,91`), mas um stderr só com o id passaria.
3. **AC 33, parte "criando as pastas"**: C33 só prova a resolução. A criação fica provada de forma
   implícita pelos testes de CLI que começam com a pasta de dados apagada (`apps/cli/tests/history.rs:23`)
   e terminam com exit 0 e o `.md` gravado (C25, `history.rs:100-108`); nenhum check a nomeia.
4. **C44**: quem cria o `fala.sqlite` é `Store::open` (`crates/storage/src/store.rs:59`), não o `reindex`; o
   claim vale no nível da API (abrir + reindexar), mas fica registrado.
5. **C9**: o `busy_timeout` é lido por um acessor que o próprio `Store` expõe
   (`crates/storage/src/store.rs:75`); compensado pelo teste comportamental do lock de 300 ms.

## Gate

`cargo test -p fala-storage --tests` - 28 passed, 0 failed · `cargo test -p fala-cli --test history --bin fala-cli` - 29 passed, 0 failed (57 no total, HEAD `099d188`)

## Round 2 (scoped: C21, C30)

**Round 2 verdict**: PASS, verified at `fe78dba` (HEAD `fe78dbaac9aaab675b1ea4f90aa7e07cb422eb9a`).
Mesmo Verifier independente (author != verifier); não alterou código.

### Escopo pelo diff

`099d188` e `fe78dba` diferem só em dois arquivos de teste (`git diff --stat 099d188 HEAD`):
`crates/storage/tests/reindex.rs` (+17/-2, só dentro de `invalid_files_are_skipped_with_reason`) e
`apps/cli/tests/history.rs` (+5/-1, só dentro de `undo_failures_exit_1`). `git diff 099d188 HEAD` sobre
`crates/storage/src`, `apps/cli/src`, `Cargo.toml`, `Cargo.lock` e os `Cargo.toml` dos dois crates é vazio:
nenhum código de produção, helper compartilhado, fixture ou config mudou. Por isso só C21 e C30 são
refeitos; os outros 42 checks, o Swept, as Binding sources e as mutações da rodada 1 são
`carried from 099d188`. Os resultados verdes desses checks são reconfirmados pelas provas completas
abaixo, rodadas no HEAD novo.

### Provas rodadas no HEAD (completas, não só C21 e C30)

Com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`:

1. `cargo test -p fala-storage --tests` - exit 0. unittests 0; `tests/mirror.rs` 3 passed;
   `tests/reindex.rs` 7 passed (inclui `invalid_files_are_skipped_with_reason ... ok`);
   `tests/store.rs` 18 passed. Total 28 passed, 0 failed.
2. `cargo test -p fala-cli --test history --bin fala-cli` - exit 0. unittests do binário 16 passed;
   `tests/history.rs` 13 passed (inclui `undo_failures_exit_1 ... ok`). Total 29 passed, 0 failed.

Mesmas contagens da rodada 1 (28 + 29 = 57); nenhum teste sumiu nem foi filtrado.

### Checks refeitos

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C21 | 7 tipos de `.md` inválido ignorados com caminho e motivo, cada um pela regra certa | `reindex` `invalid_files_are_skipped_with_reason` ok | `crates/storage/tests/reindex.rs:155-163` - tabela `reasons` com um trecho de motivo por caso; `:212-213` `indexed == 1`, `skipped.len() == cases.len()`; `:215-219` caminho achado em `skipped`; `:220-225` `assert!(skip.reason.contains(want))`; `:227-228` o válido entra. O caso `sem-fechamento` (`:166-171`) agora remove `---` e o corpo (`.replace("---\nBom.\n", "")`), então o parser lê todas as chaves e esgota o texto sem achar `---`, caindo em `crates/storage/src/mirror.rs:85-86` (`frontmatter sem a linha ... de fechamento`), não mais em `mirror.rs:93`; o trecho `fechamento` só existe nessa mensagem | PASS |
| C30 | id inexistente e item sem edição: exit 1, motivo no stderr, stdout vazio | `history` `undo_failures_exit_1` ok | `apps/cli/tests/history.rs:199-202` - pares `(id, reason)` com `não encontrado` e `não tem edição`; `:204` `code == Some(1)`; `:205` `stdout.is_empty()`; `:206` `stderr.contains(id)`; `:207` `assert!(stderr(&o).contains(reason))`. As mensagens vêm de `crates/storage/src/lib.rs:89` (`ditado não encontrado: {0}`) e `lib.rs:91` (`o ditado {0} não tem edição para desfazer`), impressas por `apps/cli/src/main.rs:80` (`log::error!("{:#}", failure.error)`) | PASS |

### Faults injected (round 2)

Feitas na árvore, uma por vez, cada uma revertida com `git checkout -- <arquivo>`. `git status --porcelain`
antes e depois: só este `verification.md` não rastreado. Depois de reverter, os dois testes rodaram de novo
e passaram (1 passed cada).

| Mutation | Location | Covering proof | Killed |
| --- | --- | --- | --- |
| motivo `frontmatter sem a linha ... de fechamento` trocado por `... final` | `crates/storage/src/mirror.rs:86` | `invalid_files_are_skipped_with_reason` falhou em `crates/storage/tests/reindex.rs:221`: `sem-fechamento: motivo "frontmatter sem a linha ... final" não contém "fechamento"` (o motivo observado é o da linha 86, o que prova que o caso cai nesse ramo) | yes |
| `ditado não encontrado: {0}` trocado por `ditado ausente: {0}` | `crates/storage/src/lib.rs:89` | `undo_failures_exit_1` falhou em `apps/cli/tests/history.rs:207` no id inexistente (stderr `ditado ausente: 0199a3f2-...`) | yes |
| `o ditado {0} não tem edição para desfazer` trocado por `o ditado {0} não pode ser desfeito` | `crates/storage/src/lib.rs:91` | `undo_failures_exit_1` falhou em `apps/cli/tests/history.rs:207` no item sem edição | yes |

### Citações atualizadas (linhas que o fix deslocou)

O fix só deslocou linhas nos dois arquivos de teste; as asserções são as mesmas da rodada 1.

- `crates/storage/tests/reindex.rs` (+15 depois da linha 151): C22 `:244-247`; C23 `:268`, `:281-282`, `:286-292`;
  C24 `:310-311`; C35 caso `language: "es"` `:196-201`, conferido em `:212-225`. C19, C20, C44 e C35 `:114-139` não mudaram.
- `apps/cli/tests/history.rs` (+4 depois da linha 196): C31 `:218-219`, `:226-232`; C32 `:241-242`; C34 `:276`;
  C36 `:289-290`, `:292`; C37 `:301`, `:303-307`; C38 `:316`, `:319`; C39 `:327`. C25-C29 não mudaram.

### Achados da rodada 2

1. Os achados 1 e 2 da rodada 1 estão resolvidos: C21 confere um trecho do motivo por caso e `mirror.rs:86`
   passa a ser exercitado; C30 confere o motivo no stderr.
2. Resíduo menor em C21 (não bloqueia): para `editor-invalido`, `showing-invalido` e `idioma-invalido` o trecho
   esperado é só o nome da chave entre crases (`reindex.rs:160-162`), que também apareceria em
   `falta a chave ...` (`mirror.rs:105`) ou `valor de ... não é JSON` (`mirror.rs:95`). Pela leitura do código os
   três casos trocam só o valor por outro texto JSON válido, então caem em `mirror.rs:126`, `:128` e `:118`;
   a asserção não distinguiria um desvio para aqueles ramos.
3. Os achados 3 a 5 da rodada 1 (AC 33 criação de pastas, C44, C9) seguem como registrados (`carried from 099d188`).

### Gate (round 2)

`cargo test -p fala-storage --tests` - 28 passed, 0 failed · `cargo test -p fala-cli --test history --bin fala-cli` - 29 passed, 0 failed (57 no total, HEAD `fe78dba`)

## Round 3 (scoped: C45, regressão em C1-C44)

**Round 3 verdict**: PASS, verified at `e6a6a37` (HEAD `e6a6a37afcd2bf04be1d61d249aaf6c0d75c043c`).
Verifier independente (author != verifier), novo nesta rodada; não alterou nem commitou código.

### Escopo pelo diff

`git diff --stat fe78dba..HEAD`: `checks.md` (+C45, door 8 na linha de doors, nova linha de Coverage),
`plan.md` (door 8, AC 37, uma linha na Surface), e código de produção em `crates/storage/src/lib.rs`
(campo `DictationRecord::sensitive`, `:60`), `crates/storage/src/mirror.rs` (render `:51-54`, parse `:134-138`)
e `crates/storage/src/store.rs` (schema `:28`, `COLUMNS` `:45`, `add`/`add_sensitive`/`add_marked` `:85-119`,
`insert` `:250-262`, `read_row` `:285`). Testes: só `crates/storage/tests/reindex.rs` (+68, o novo
`sensitive_round_trips` em `:315-381`, anexado depois da última função; nenhuma linha anterior moveu).
`apps/cli/`, `crates/storage/tests/{mirror,store}.rs`, `Cargo.toml` e `Cargo.lock` não mudaram.

Como o commit toca `render`, `parse`, o schema e `insert`/`read_row` (o caminho de todo check de API), o raio
de alcance é C1-C44 inteiro. Tratamento: as provas completas rodam de novo no HEAD (abaixo); os checks cujo
literal a mudança poderia quebrar são refeitos (C9 `user_version`, C10 frontmatter literal); os demais seguem
`carried from fe78dba`, reconfirmados como verdes pela rodada completa.

### Provas rodadas no HEAD (completas)

Com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`:

1. `cargo test -p fala-storage --tests` - exit 0. unittests 0; `tests/mirror.rs` 3 passed;
   `tests/reindex.rs` 8 passed (os 7 anteriores mais `sensitive_round_trips ... ok`); `tests/store.rs` 18 passed.
   Total 29 passed, 0 failed.
2. `cargo test -p fala-cli --test history --bin fala-cli` - exit 0. unittests do binário 16 passed;
   `tests/history.rs` 13 passed. Total 29 passed, 0 failed.

58 no total (57 da rodada 2 mais o novo); nenhum teste anterior sumiu, foi filtrado ou ficou vermelho.
`rg -n "fn sensitive_round_trips" crates/storage/tests/reindex.rs` acha `:316`; o teste é novo no diff.

### Checks refeitos

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C45 | banco 1/0 por SQL cru | `reindex` `sensitive_round_trips` ok | `crates/storage/tests/reindex.rs:329-336` - `SELECT sensitive FROM dictations WHERE id = ?1` numa `rusqlite::Connection` à parte; `:337` `assert_eq!(stored(&marked.id), 1)`; `:338` `assert_eq!(stored(&plain.id), 0)` | PASS |
| C45 | `sensitive: true` como última linha do frontmatter | idem | `crates/storage/tests/reindex.rs:342-345` - `md.contains("\nraw: \"senha do banco\"\nsensitive: true\n---\n")`: `raw` imediatamente antes e o `---` de fechamento imediatamente depois; escrito por `crates/storage/src/mirror.rs:52-54`, depois do laço das chaves e antes de `---` (`:55`) | PASS |
| C45 | item não marcado sem a chave | idem | `crates/storage/tests/reindex.rs:347` - `assert!(!md_plain.contains("sensitive"))` | PASS |
| C45 | `reindex` depois de apagar o banco: true, ausente e `false` explícito | idem | `crates/storage/tests/reindex.rs:365` `delete_db`; `:367-368` `reindex` num `Store` novo; `:369` `report.indexed == 3`; `:377` `get(marked) == marked` e `:378` `.sensitive` verdadeiro; `:379` `!get(plain).sensitive` (chave ausente); `:380` `!get(explicit_id).sensitive` (`sensitive: false` escrito em `:353-355`, conferido em `:359`) | PASS |
| C45 | `"sim"` ignorado com motivo citando `sensitive` | idem | `crates/storage/tests/reindex.rs:370` `skipped.len() == 1`; `:371` `skipped[0].path == bad_path`; `:372-376` `skipped[0].reason.contains("sensitive")`. Ramo `crates/storage/src/mirror.rs:137` (`` `sensitive` não é `true` nem `false` ``) | PASS |
| C10 | frontmatter literal em ordem, sem chave nova para item não marcado | `mirror` `mirror_path_and_frontmatter_literal` ok | `crates/storage/tests/mirror.rs:35-48` - `assert_eq!(read_to_string(path), want)` com o arquivo inteiro literal (`id`, `created_at`, `language`, `app`, `edited_by`, `showing`, `raw`, `---`, corpo); o teste não mudou no diff e segue verde, então `add` não acrescentou linha; `:55` `contains("\napp: null\n")`; `:57` `ends_with("---\nOi.\n")` | PASS |
| C9 | `user_version` continua 1 com a door 8 | `store` `open_sets_wal_timeout_and_version` ok | `crates/storage/tests/store.rs:196` - `assert_eq!(version, 1)`; `crates/storage/src/store.rs:14` `SCHEMA_VERSION: i64 = 1` intocado; a coluna entrou dentro de `SCHEMA_1` (`store.rs:28`) | PASS |

### Coverage (recomputada para a linha nova; `light` não exige)

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| `sensitive` em um `.md` (4) | `crates/storage/src/mirror.rs:134-138` (três braços: `None` ou `Bool(false)`, `Bool(true)`, resto) | `true` `reindex.rs:378` · ausente `:379` · `false` `:380` · outro `:370-376` | - |
| Landing doors (8) | `plan.md` Landing | door 8 -> C45; doors 1-7 `carried from fe78dba` | - |

### Faults injected (round 3)

Feitas na árvore real, uma por vez, cada uma revertida com `git checkout -- <arquivo>`. `git status --porcelain`
antes e depois de cada uma: só este `verification.md` não rastreado. Depois da última reversão, `mirror` (3) e
`reindex` (8) rodaram de novo e passaram.

| Mutation | Location | Covering proof | Killed |
| --- | --- | --- | --- |
| render escreve sempre `sensitive: <bool>` (inclusive `sensitive: false`) | `crates/storage/src/mirror.rs:52-54` | `mirror_path_and_frontmatter_literal` falhou em `crates/storage/tests/mirror.rs:48` (linha extra `sensitive: false`) e `sensitive_round_trips` em `crates/storage/tests/reindex.rs:347` | yes |
| parse aceita qualquer valor como falso (`Some(_) => false`) | `crates/storage/src/mirror.rs:137` | `sensitive_round_trips` falhou em `crates/storage/tests/reindex.rs:369` (`indexed` 4, não 3) | yes |
| `insert` grava sempre `false` na coluna | `crates/storage/src/store.rs:262` | `sensitive_round_trips` falhou em `crates/storage/tests/reindex.rs:337` (banco 0, não 1) | yes |
| motivo do valor inválido sem o nome da chave | `crates/storage/src/mirror.rs:137` | `sensitive_round_trips` falhou em `crates/storage/tests/reindex.rs:372` | yes |

### Achados da rodada 3 (não bloqueiam; nenhum muda um Result)

1. **Citações de `mirror.rs` erradas desde a rodada 1.** A tabela da rodada 1 cita
   `crates/storage/tests/mirror.rs:143-169` (C10), `:200-203` (C11) e `:225`, `:231`, `:239` (C13), mas o arquivo tem
   128 linhas no HEAD e também em `099d188` (`git show 099d188:crates/storage/tests/mirror.rs` com 128 linhas). As
   asserções existem, em outras linhas. Corrigidas aqui: C10 `mirror.rs:35-48`, `:55`, `:57`; C11 `mirror.rs:88`
   (`skipped.is_empty()`), `:89` (`indexed == ids.len()`), `:91` (`get(id) == record`); C13 `mirror.rs:113`
   (`tmps == 0`), `:119` (`.md == good`), `:127` (`tmps == 0` depois da nova escrita). Os três testes seguem verdes.
2. **`undo`/`redo` preservando a marca não tem check.** `set_showing` (`crates/storage/src/store.rs:221-236`) relê
   o registro com `get` (que lê `d.sensitive`, `store.rs:285`) e reescreve o espelho com `render`, então pela leitura
   do código `sensitive: true` sobrevive a um `undo`; nenhuma asserção prova isso. O AC 37 não o nomeia; fica
   registrado como lacuna de precisão para a feature que vai filtrar pela marca.
3. **`add_sensitive` com falha de espelho** (linha nova da Surface: "os mesmos de `add`") não tem prova própria;
   o caminho é o mesmo `add_marked` (`store.rs:102-119`) que C12 cobre via `add`. Aceito no nível estrutural.
4. **`CHECK (sensitive IN (0, 1))`** (`store.rs:28`) não é exercitado por SQL cru com outro valor; o claim de C45
   não o nomeia.
5. Numeração: C45 prova o AC 37, enquanto C37 prova outra coisa (status da Surface); `checks.md` explica que
   C37-C44 fecham Surface e portas. Só ruído de leitura.

### Gate (round 3)

`cargo test -p fala-storage --tests` - 29 passed, 0 failed · `cargo test -p fala-cli --test history --bin fala-cli` - 29 passed, 0 failed (58 no total, HEAD `e6a6a37`)
