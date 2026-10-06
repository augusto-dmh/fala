# mcp-local verification

**Verdict**: PASS
**Profile**: light
**Diff range**: f058e5c..e507b74
**Round**: 4 - scoped
**Verifier**: independent sub-agent (author != verifier)

Rodada 4 além do limite de três rodadas do skill: decisão do autor, numa rodada delegada sem ninguém para responder (registrada no `## Handoff` do `checks.md`, "Rodada 4").

O único gap da rodada 3 está fechado. O C7 agora afirma, nos dois casos do AC 35, o rótulo junto
com o motivo que o SDK gera (`sessão MCP: expect initialized request, but received:`). Conferi na
fonte do `rmcp` 3.5.0 que esse texto é o `Display` de
`ServerInitializeError::ExpectedInitializeRequest`, a variante que o SDK devolve justamente nos
dois casos do AC: uma primeira mensagem que não é request, e um request sem os campos do `_meta`,
depois do `-32602`. As 36 provas de teste e a prova do C32 rodaram de novo no `HEAD` `e507b74` e
passaram. O diff da correção (`168d21d..e507b74`) mexe em dois arquivos só: a asserção do C7 em
`apps/cli/tests/mcp.rs` e texto do `checks.md`. Nenhum código de produção mudou, então o
julgamento das outras 35 asserções vem da rodada 3, com as linhas de citação atualizadas.

## Scope of this round

- **Proofs**: todas rodaram de novo, completas, no `e507b74`. *verified at e507b74*
- **C7**: rejulgado contra o AC 35 e contra a fonte do SDK. *verified at e507b74*
- **Diff da correção**: `git diff --name-status 168d21d e507b74` dá `M .specs/features/mcp-local/checks.md`
  e `M apps/cli/tests/mcp.rs`. Em `mcp.rs`, as duas mudanças trocam cada `assert!(done.stderr.contains("sessão MCP"), ...)`
  (as antigas `:414` e `:426`) por um `assert!` de seis linhas com o texto do motivo. Em
  `checks.md`, a redação do C7, os tamanhos das linhas de Coverage `get_dictation statuses` e
  `mcp.toml estados` (de 5 para 6, que era a nota 2 da rodada 3) e as linhas "Rodada 4" do
  `## Handoff`. Nenhum `crates/` nem `apps/cli/src` mudou. *verified at e507b74*
- **Citações**: as do `apps/cli/tests/mcp.rs` acima da linha 413 não se moveram. Abaixo da
  `:426` antiga, todas desceram 10 linhas (5 por asserção expandida). Conferi com `rg -n` nos 36
  `fn <nome>()` (agora entre `:274` e `:1098`) e em mais de 50 asserções citadas, todas no lugar
  novo. As citações de outros arquivos (`crates/mcp/src/*.rs`, `crates/storage/src/store.rs`,
  `Cargo.toml`) não mudaram e conferi as que a tabela usa. *verified at e507b74*
- **Julgamento das outras 35 asserções**, varredura check contra AC, nível e amostragem, Swept
  existing: o código e as asserções não mudaram. *carried from 168d21d*

## Binding sources

O perfil é `light` e o plano não marca nenhuma fonte como binding, então o passo 1 não roda.
*carried from 168d21d*

## Checks

Todas as provas de teste rodaram numa invocação só, no `HEAD` `e507b74`:
`CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2 cargo test -p fala-cli --test mcp`.
O processo saiu com 0 (`36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`). Extraí os
36 nomes de `--exact` do `checks.md`, todos distintos. Cada um aparece exatamente uma vez na saída
como `test <nome> ... ok`, e existe como `fn <nome>()` em `apps/cli/tests/mcp.rs`
(`rg -n "^fn [a-z_0-9]+\(\)"`). O arquivo de testes entra inteiro na faixa da feature
(`git diff --name-status f058e5c..e507b74`: `A apps/cli/tests/mcp.rs`). A asserção nova do C7 só
existe neste commit e passou. As citações abaixo são de `apps/cli/tests/mcp.rs`, a não ser onde
outro arquivo aparece, e as linhas são as do `e507b74`.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | discover 2026-07-28: `supportedVersions` com `"2026-07-28"`, `tools`+`resources`, `_meta[serverInfo].name = "fala"` | `discover_reports_fala_with_tools_and_resources ... ok` | `apps/cli/tests/mcp.rs:287-290` - `assert_eq!(result["_meta"]["io.modelcontextprotocol/serverInfo"]["name"], json!("fala"))`; `:278-282` - `supportedVersions...contains(&json!("2026-07-28"))`; `:285-286` - `capabilities.tools/resources .is_object()`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C2 | `initialize` 2025-11-25: `serverInfo.name = "fala"`, `tools`+`resources` | `initialize_legacy_reports_fala ... ok` | `apps/cli/tests/mcp.rs:311` - `assert_eq!(result["serverInfo"]["name"], json!("fala"))`; `:312-316` - as duas capacidades `.is_object()`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C3 | `tools/list` = exatamente as duas tools, nessa ordem, com as 4 anotações | `tools_list_has_two_read_only_tools_in_order ... ok` | `apps/cli/tests/mcp.rs:331` - `assert_eq!(names, ["search_dictations", "get_dictation"])`; `:334-337` - `readOnlyHint == true`, `destructiveHint == false`, `idempotentHint == true`, `openWorldHint == false`, para cada tool. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C4 | toda linha do stdout é JSON com `"jsonrpc": "2.0"`; o log fica no stderr | `stdout_is_only_json_rpc_lines ... ok` | `apps/cli/tests/mcp.rs:362-364` - `serde_json::from_str(line)` (pânico se não for JSON) e `assert_eq!(value["jsonrpc"], json!("2.0"))`; `:360` - pelo menos 5 linhas; `:366` - `assert!(done.stderr.contains("mcp:"))`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C5 | EOF depois de um discover e sem nenhuma mensagem: exit 0 em até 5 s | `eof_exits_zero_within_five_seconds ... ok` | `apps/cli/tests/mcp.rs:375-376` - `assert_eq!(done.status.code(), Some(0))` e `assert!(done.elapsed <= Duration::from_secs(5))`; `:379-380` - o mesmo para a sessão silenciosa. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C6 | `ttlMs` numérico e `cacheScope: "private"` nos 4 resultados cacheáveis em 2026-07-28 | `cacheable_results_carry_ttl_and_private_scope ... ok` | `apps/cli/tests/mcp.rs:397` - `assert!(result["ttlMs"].is_u64())`; `:398-402` - `assert_eq!(result["cacheScope"], json!("private"))`, num laço pelos 4 métodos (`:389-394`). *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C7 | notificação primeiro: exit 1 e o motivo no stderr (`sessão MCP: expect initialized request, but received: ...`); discover sem `_meta`: `-32602`, exit 1 e o mesmo motivo no stderr (AC 35) | `bad_opening_message_exits_one ... ok` | `apps/cli/tests/mcp.rs:413` e `:429` - `assert_eq!(done.status.code(), Some(1), ...)`; `:424-428` - `assert_eq!(response["error"]["code"], json!(INVALID_PARAMS), ...)`; `:414-419` e `:430-435` - `assert!(done.stderr.contains("sessão MCP: expect initialized request, but received:"), ...)`. O texto depois do rótulo é o motivo que o SDK gera, não outro rótulo: `rmcp-3.5.0/src/service/server.rs:86` - `#[error("expect initialized request, but received: {0:?}")]` em `ServerInitializeError::ExpectedInitializeRequest`, devolvido em `:594` (primeira mensagem que não é request, o caso da notificação) e em `:620` (request sem os campos do `_meta`, depois de mandar o erro `-32602` em `:610`). O `fala-mcp` repassa esse `Display` inteiro: `crates/mcp/src/lib.rs:78` - `Err(e) => Err(McpError::Protocol(e.to_string()))`, e `:30` - `#[error("sessão MCP: {0}")]`. Se o `{0}` sumisse, ou se o erro viesse de outra variante (`connection closed`, `initialize failed`), as duas asserções cairiam. O detalhe depois de `received:` (o `Debug` da mensagem) não é afirmado, e o AC 35 não o pede. Gap da rodada 3 fechado. *verified at e507b74* | PASS |
| C8 | `acao` devolve os dois com "ação" (um só no bruto), o mais novo primeiro, sem o terceiro; `amanh` só o de "amanhã"; `acao contr` só o que tem os dois termos | `search_finds_without_accent_newest_first ... ok` | `apps/cli/tests/mcp.rs:462` - `assert_eq!(ids(&items), [b.clone(), a.clone()])`; `:463` - `assert_eq!(ids(&s.search(json!({ "query": "amanh" }))), [a])`; `:464` - `assert_eq!(ids(&s.search(json!({ "query": "acao contr" }))), [b])`. As três igualdades são exatas e excluem `c` e o termo solto. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C9 | chaves exatas do item da busca; `text` = bruto no desfeito, final no não desfeito | `search_items_have_exactly_the_contract_keys ... ok` | `apps/cli/tests/mcp.rs:485` - `assert_eq!(keys(item), contract)` (o conjunto de 7, `:475-483`); `:488-491` - `b.text == "a ação do contrato"`, `b.showing == "raw"`, `a.text == "Ação de amanhã."`, `a.showing == "final"`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C10 | sem `query` e com `"   "`, `limit: 2`: os mais recentes, em ordem decrescente | `search_without_query_lists_most_recent ... ok` | `apps/cli/tests/mcp.rs:500-503` - `assert_eq!(ids(&s.search(json!({ "limit": 2 }))), [c.clone(), b.clone()])`; `:504-507` - o mesmo com `"query": "   "`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C11 | 25 gravados, `{}` devolve 20 | `search_limit_defaults_to_twenty ... ok` | `apps/cli/tests/mcp.rs:523` - `assert_eq!(s.search(json!({})).len(), 20)`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C12 | `limit` 0, 51, -1, `"5"` e `5.5` dão `-32602`; 1 e 50 são aceitos | `search_limit_out_of_range_is_invalid_params ... ok` | `apps/cli/tests/mcp.rs:532-538` - laço por `[json!(0), json!(51), json!(-1), json!("5"), json!(5.5)]` com `assert_eq!(response["error"]["code"], json!(INVALID_PARAMS))`; `:540-541` - `limit: 1` dá `len() == 1` e `limit: 50` dá `len() == 3`, os dois via `ok()`, que exige `isError == false` (`:204`). *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C13 | `get_dictation`: chaves exatas, `uri = fala://dictation/<id>`, os valores gravados | `get_returns_exactly_the_contract_keys ... ok` | `apps/cli/tests/mcp.rs:554-567` - `assert_eq!(keys(&got), set(&[...9 chaves]))`; `:569` - `assert_eq!(got["uri"], json!(format!("fala://dictation/{}", record.id)))`; `:568-581` - cada valor comparado com o gravado. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C14 | UUID inexistente: `isError: true` e `dictation not found: <id>` | `get_unknown_id_is_not_found_error ... ok` | `apps/cli/tests/mcp.rs:591` - `assert_eq!(response["result"]["isError"], json!(true))`; `:592-595` - `assert_eq!(tool_text(&response), format!("dictation not found: {MISSING}"))`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C15 | `get_dictation {}`, `{ id: 7 }` e `delete_dictation` dão `-32602` | `get_without_id_or_unknown_tool_is_invalid_params ... ok` | `apps/cli/tests/mcp.rs:604-614` - laço pelos 3 com `assert_eq!(response["error"]["code"], json!(INVALID_PARAMS))`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C16 | um único template `fala://dictation/{id}` | `templates_list_has_dictation_template ... ok` | `apps/cli/tests/mcp.rs:624` - `assert_eq!(templates.len(), 1)`; `:625` - `assert_eq!(templates[0]["uriTemplate"], json!("fala://dictation/{id}"))`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C17 | 52 gravados: exatamente as URIs dos 50 mais recentes, em ordem decrescente, cada uma com `mimeType = "application/json"` | `resources_list_newest_first_with_uri_and_mime ... ok` | `apps/cli/tests/mcp.rs:663` - `assert_eq!(uris, expected)`, com `expected` = os ids gravados em ordem inversa, `take(50)`, formatados como `fala://dictation/{id}` (`:645-650`); `:654` - `len() == 50`; `:664-666` - `mimeType == "application/json"` em cada uma. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C18 | `resources/read`: `application/json` cujo texto parseado é o `structuredContent` do get | `read_returns_same_object_as_get ... ok` | `apps/cli/tests/mcp.rs:681-685` - `contents["mimeType"] == "application/json"`; `:687` - `assert_eq!(read, got)`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C19 | UUID inexistente, `file:///etc/passwd` e `fala://meeting/x` dão `-32602` | `read_unknown_or_foreign_uri_is_not_found ... ok` | `apps/cli/tests/mcp.rs:696-707` - laço pelas 3 URIs com `assert_eq!(response["error"]["code"], json!(INVALID_PARAMS))`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C20 | o sensível some de `query: "segredo"` e da busca vazia; `limit: 3` devolve os 3 normais | `sensitive_is_omitted_from_search_and_limit_is_filled ... ok` | `apps/cli/tests/mcp.rs:731` - `assert!(s.search(json!({ "query": "segredo" })).is_empty())`; `:732` - `!ids(&s.search(json!({}))).contains(&secret)`; `:734` - `assert_eq!(ids(&s.search(json!({ "limit": 3 }))), newest_three)`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C21 | get do sensível igual ao de um inexistente, trocado só o id | `sensitive_get_matches_nonexistent ... ok` | `apps/cli/tests/mcp.rs:746-749` - `assert_eq!(sensitive.to_string().replace(&secret, "<id>"), missing.to_string().replace(MISSING, "<id>"))`; `:745` - `isError == true`; `:750-752` - contém `dictation not found: {secret}`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C22 | read do sensível: mesmo código e mensagem do inexistente; `resources/list` não traz a URI dele | `sensitive_read_matches_nonexistent_and_list_omits ... ok` | `apps/cli/tests/mcp.rs:771` - `assert_eq!(sensitive["code"], missing["code"])`; `:772-781` - mensagens iguais depois de trocar o id; `:783` - `assert!(!list.to_string().contains(&secret))`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C23 | nenhuma linha do stdout de uma sessão completa traz o caminho da pasta de dados nem `"sensitive"` | `responses_never_contain_data_dir_or_sensitive_key ... ok` | `apps/cli/tests/mcp.rs:808` - `assert!(!line.contains(dir))`; `:809-812` - `assert!(!line.contains("\"sensitive\""))`, para cada linha de uma sessão com discover, list, search, 3 gets (o sensível e um inválido entre eles), list, templates e 2 reads (`:792-804`). *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C24 | tabela renomeada no meio da sessão: search e get dão `isError` com `storage error` exato num bloco só; a causa (`no such table`) vai ao stderr e a nenhuma linha do stdout (AC 23) | `storage_failure_is_opaque_error ... ok` | `apps/cli/tests/mcp.rs:830-831` - `isError == true` e `assert_eq!(tool_text(&response), "storage error")`, para as duas tools; `:832-836` - `content.len() == 1`; `:837-840` - `!response.to_string().contains("no such table")`; `:843-848` - `assert!(done.stderr.contains("falha ao ler o histórico: banco: no such table"))`; `:849` - `done.stdout.iter().all(...)` com uma closure que nega `l.contains("no such table")`. A causa e o "só" do AC 23 agora têm prova. Gap da rodada 2 fechado. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C25 | sem `mcp.toml`: search e get dão `isError` com a mensagem de desligado; `resources/list` vazio | `disabled_without_toml_refuses_every_call ... ok` | `apps/cli/tests/mcp.rs:921-922` (`assert_disabled`) - `isError == true` e `assert_eq!(tool_text(&response), DISABLED)`, com `DISABLED` literal em `:21-22`; `:933` - `assert_eq!(list["result"]["resources"], json!([]))`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C26 | `"yes"`, `false`, sem `enabled` e TOML inválido = desligado, com aviso `WARN` e o motivo no stderr nos quatro casos | `invalid_or_false_toml_is_disabled ... ok` | `apps/cli/tests/mcp.rs:957` - `assert_disabled` nos 4 casos; `:960-961` - `assert!(done.stderr.contains(warning))` e `assert!(done.stderr.contains("WARN"))`, com um motivo específico por caso (`:943`, `:945`, `:946`, `:950`: `precisa ser true ou false`, `enabled = false`, ``sem a chave `enabled` ``, `não é TOML válido`). Código: `crates/mcp/src/config.rs:28`, `:35`, `:39`, `:43-46`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C27 | desligado e com `fala.sqlite` lixo: responde ao discover e a uma `tools/call` sem sair | `disabled_does_not_open_db ... ok` | `apps/cli/tests/mcp.rs:971` - `assert!(s.discover()["result"].is_object())`; `:972` - `assert_disabled`; `:974` - `assert_eq!(done.status.code(), Some(0))`. A prova é indireta, mas firme: o mesmo lixo com o MCP ligado derruba o processo com exit 1 (C31). *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C28 | desligado, `tools/list` traz as duas tools | `disabled_still_lists_tools ... ok` | `apps/cli/tests/mcp.rs:988` - `assert_eq!(names, ["search_dictations", "get_dictation"])`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C29 | ligado sem banco: `items: []`, `dictation not found`, e o arquivo continua inexistente | `missing_db_gives_empty_and_is_not_created ... ok` | `apps/cli/tests/mcp.rs:998` - `assert!(s.search(json!({ "query": "acao" })).is_empty())`; `:1000-1003` - `tool_text == format!("dictation not found: {MISSING}")`; `:1006` - `assert!(!data.db().exists())`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C30 | `fala.sqlite` byte a byte igual depois de busca, get, list e read | `session_leaves_db_bytes_unchanged ... ok` | `apps/cli/tests/mcp.rs:1024` - `assert!(fs::read(data.db()).unwrap() == before)`; `:1023` - exit 0. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C31 | ligado com banco lixo: exit 1, stdout vazio, o motivo no stderr (`abrir o histórico: ... not a database`) (AC 30) | `non_sqlite_db_exits_one_with_empty_stdout ... ok` | `apps/cli/tests/mcp.rs:1032` - `assert_eq!(done.status.code(), Some(1))`; `:1033` - `assert!(done.stdout.is_empty())`; `:1034-1038` - `assert!(done.stderr.contains("abrir o histórico") && done.stderr.contains("not a database"))`. Rótulo e causa afirmados. Gap da rodada 2 fechado. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C32 | `cargo tree -p fala-mcp -e normal` sem `hyper`, `reqwest`, `rustls`, `native-tls`, `oauth2`, `tauri` | o `cargo tree` negado da seção `## Gate`, exit 0 (nenhuma linha casou) | `Cargo.toml:30` - `rmcp = { version = "~3.5", default-features = false, features = ["server", "transport-io"] }`. Controle positivo: na mesma saída do `cargo tree`, o mesmo formato de padrão, com `rmcp`, `tokio` e `fala-storage` no lugar, casa `rmcp v3.5.0`, `tokio v1.49.0` e `fala-storage v0.1.0`, então o formato casa com o padrão. *verified at e507b74* (prova e controle positivo rodados de novo) | PASS |
| C33 | sem `RUST_LOG`, o stderr de search+get não traz o texto bruto nem o final | `default_log_level_never_prints_dictated_text ... ok` | `apps/cli/tests/mcp.rs:1056-1060` - `assert!(done.stderr.contains("mcp:"))` (o nível `info` está ativo e o teste não é vazio); `:1061-1067` - `assert!(!done.stderr.contains(text))` para `"zebra listrada falou"` e `"A zebra listrada falou."`; `:130` - `env_remove("RUST_LOG")`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C34 | sem `--data-dir`, com `XDG_DATA_HOME` temporário, o ditado do `history add` é achado | `default_data_dir_is_the_history_dir ... ok` | `apps/cli/tests/mcp.rs:1093` - `assert_eq!(ids(&s.search(json!({ "query": "ornitorrinco" }))), [id])`; `:1080-1085` - `history add` sai com 0; `:1087-1091` - `mcp.toml` em `<xdg>/br.com.augusto.fala`; `:1092` - `Session::start(&["mcp"], ...)` sem `--data-dir`. Se a pasta fosse outra, o MCP estaria desligado e `ok()` falharia. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C35 | `mcp --bogus` sai com 2 sem esperar o stdin | `unknown_argument_exits_two ... ok` | `apps/cli/tests/mcp.rs:1111-1114` - `assert!(started.elapsed() < Duration::from_secs(5))` com o stdin aberto; `:1117` - `assert_eq!(status.code(), Some(2))`. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C36 | tabela renomeada no meio da sessão: `resources/list` e `resources/read` dão `-32603` com `storage error` exato e `error.data` nulo; a causa (`no such table`) vai ao stderr e a nenhuma linha do stdout (AC 36) | `resource_storage_failure_is_opaque_error ... ok` | `apps/cli/tests/mcp.rs:869` - `assert_eq!(response["error"]["code"], json!(-32603))`; `:870-874` - `assert_eq!(response["error"]["message"], json!("storage error"))`; `:875` - `assert!(response["error"]["data"].is_null())`, para os dois métodos; `:878-883` - `assert!(done.stderr.contains("falha ao ler o histórico: banco: no such table"))`; `:884` - nenhuma linha do stdout contém `no such table`. Gap da rodada 2 fechado. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |
| C37 | sessão por `initialize` 2025-11-25: `resources/read` de id inexistente dá `-32002` | `legacy_read_missing_is_resource_not_found ... ok` | `apps/cli/tests/mcp.rs:909` - `assert_eq!(response["error"]["code"], json!(-32002))`; `:891-904` - sessão legada (`modern = false`), `initialize` com `"protocolVersion": "2025-11-25"` e `notifications/initialized` antes do read. *carried from 168d21d* (julgamento da asserção); prova rodada e linhas conferidas no `e507b74` | PASS |

### Varredura: check contra AC

*carried from 168d21d*, com o C7 atualizado. Na rodada 3 li os 37 AC contra a asserção de cada
check, procurando AC que pede um motivo, uma causa ou um valor e teste que afirma algo mais fraco.
Os AC com "motivo"/"causa"/"aviso" no stderr são o 23 (C24), o 25 (C26), o 30 (C31), o 35 (C7) e
o 36 (C36). Agora os cinco afirmam o texto do motivo ou da causa: o C7 passou a afirmar
`expect initialized request, but received:` *(verified at e507b74)*. Os AC de valor exato e os
de ausência ou invariante ficam como na rodada 3, porque nenhuma dessas asserções mudou.

### Nível e amostragem

*carried from 168d21d*, com as linhas do teste atualizadas.

- Nível: toda prova cruza a fronteira, com o binário `fala-cli` de verdade falando JSON-RPC pelo
  stdin/stdout. O C32 é de grafo de build, e ali o nível certo é o `cargo tree`.
- C24 e C36: a asserção de stderr é uma só por teste, depois das duas chamadas. Os dois caminhos
  passam pela mesma função de log (`crates/mcp/src/server.rs:406-413` e `:416-420`), o que
  aceito como amostragem suficiente.
- C22 e AC 21: compara `code` e `message`, não o objeto de erro inteiro. O código passa `None` em
  `data` nos dois caminhos (`server.rs:240`).
- C19 e AC 18: prende o código `-32602`, que é o que o AC fixa.
- C27 e AC 26: "não abriu o banco" é inferido de "não saiu com o lixo". O código não abre
  (`crates/mcp/src/lib.rs:42`, `if enabled { open_store(data_dir)? } else { None }`).
- C7 (novo nesta rodada): o motivo é afirmado como prefixo fixo, sem o `Debug` da mensagem
  recebida. Os dois casos do AC 35 caem na mesma variante do SDK, então o mesmo texto nos dois
  não distingue um do outro. O que distingue é a asserção do `-32602` no segundo caso
  (`:424-428`), que já existia. *verified at e507b74*

## Swept existing

*carried from 168d21d*, com as linhas conferidas de novo no `e507b74`.

- concurrency - `existing`: confere. `crates/storage/src/store.rs:61` - `conn.busy_timeout(std::time::Duration::from_millis(5000))?;` e `:62` - `pragma_update_and_check(None, "journal_mode", "WAL", ...)`.
- duplicates - `existing` (Observable do plano): confere. `crates/storage/src/store.rs:19` - `id TEXT NOT NULL UNIQUE,`.
- As linhas `n/a` (idempotency, data lifecycle, state transitions) são política aprovada e não há
  o que conferir no código.

## Findings

Nenhum achado derruba linha nesta rodada.

Notas que ficam abertas, todas *carried from 168d21d* e nenhuma ligada a um check:

1. Exit `2` tem um segundo caminho que o `Surface` não nomeia: `fala-cli mcp` sem `--data-dir`
   num SO sem pasta de dados (`apps/cli/src/mcp.rs:22-25`). Não tem AC nem teste.
2. Cosmético: a mensagem do banco ilegível repete a cadeia de causas, porque o `{:#}` do `anyhow`
   imprime fontes que o `Display` já inclui.
3. No segundo caso do AC 35, o motivo real para o cliente (`_meta` ausente) só aparece no
   stdout, dentro do `-32602`. O stderr diz que o servidor esperava `initialize`, o que é
   verdade, mas orienta menos. É uma observação, não um requisito do AC.

A nota 2 da rodada 3 (tamanhos errados no Coverage do `checks.md`) fechou no `e507b74`:
`get_dictation statuses (6)` e `mcp.toml estados (6)`, cada uma com seis membros. Sob `light`, o
Coverage não é recalculado. Conferi só o tamanho contra a lista.

Sob `light` não houve fault injection nem recálculo do `Coverage`, e esta rodada não injetou
falhas. O passo 7 (registrar lições com `lessons.py`) não rodou, porque este Verifier só pode
escrever este arquivo. Ele fica com o orquestrador.

## Gate

`CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2 cargo test -p fala-cli --test mcp` - 36 passed, 0 failed (exit 0, no `HEAD` `e507b74`)

C32: `! CARGO_TARGET_DIR=/home/augusto/projects/fala/target cargo tree -p fala-mcp -e normal --prefix none --format '{p}' | grep -E '^(hyper|reqwest|rustls|native-tls|oauth2|tauri)[ -]'` - exit 0 (nenhuma linha casou), no `e507b74`. Controle positivo na mesma saída: `rmcp v3.5.0`, `tokio v1.49.0`, `fala-storage v0.1.0`.

## Previous rounds

**Rodada 1** (`f058e5c..cc8600d`, `light`, full): FAIL. 33 PASS e 2 FAIL (C26 e C7), 34 testes verdes.

| Gap da rodada 1 | Status na rodada 4 | Evidência no `e507b74` |
| --- | --- | --- |
| 1. C26 mais estreito que o AC 25 (sem aviso quando `enabled` falta ou é `false`) | closed (rodada 2) | `crates/mcp/src/config.rs:35`, `:39`; `apps/cli/tests/mcp.rs:945-946`, `:960-961` |
| 2. Asserção de stderr do C7 vazia (`!stderr.is_empty()`) | closed (rodada 4, depois de subir para "só o rótulo" na rodada 2) | `apps/cli/tests/mcp.rs:414-419`, `:430-435` |
| 3. Falha de banco em `resources/list` e `resources/read` sem teste | closed (rodada 2) | C36, `apps/cli/tests/mcp.rs:869-875` |
| 4. C17 amostrava só a primeira URI | closed (rodada 2) | `apps/cli/tests/mcp.rs:663` |
| 5. `-32002` legado sem teste | closed (rodada 2) | C37, `apps/cli/tests/mcp.rs:909` |

**Rodada 2** (`f058e5c..879b83d`, `light`, full): FAIL. 34 PASS e 3 FAIL (C24, C31, C36), 36 testes verdes.

| Gap da rodada 2 | Status na rodada 4 | Evidência no `e507b74` |
| --- | --- | --- |
| C24 / AC 23: só o rótulo, sem a causa; "só" parcial | closed (rodada 3) | `apps/cli/tests/mcp.rs:843-849`, `:832-840` |
| C36 / AC 36: o mesmo, e `error.data` não afirmado nulo | closed (rodada 3) | `apps/cli/tests/mcp.rs:875`, `:878-884` |
| C31 / AC 30: "stderr não vazio" em vez do motivo | closed (rodada 3) | `apps/cli/tests/mcp.rs:1034-1038` |
| Notas: `## Measured` com 80 pacotes; C8 sem prefixo; C12 sem fracionário | closed (rodada 3) | plano diz 68; `apps/cli/tests/mcp.rs:463-464`, `:532` |
| Nota cosmética: cadeia de causas repetida | open (nota 2 acima, sem check) | sonda manual da rodada 3 |

**Rodada 3** (`f058e5c..168d21d`, `light`, full): FAIL. 36 PASS e 1 FAIL (C7), 36 testes verdes.

| Gap da rodada 3 | Status na rodada 4 | Evidência no `e507b74` |
| --- | --- | --- |
| C7 / AC 35: só o rótulo `sessão MCP`, sem o motivo | closed | `apps/cli/tests/mcp.rs:414-419`, `:430-435` - `contains("sessão MCP: expect initialized request, but received:")`; motivo do SDK em `rmcp-3.5.0/src/service/server.rs:86` |
| Nota 2: tamanhos 5 em duas linhas de Coverage que listam 6 | closed | `.specs/features/mcp-local/checks.md:147`, `:156` |
| Nota 3: segundo caminho do exit 2 sem AC | open (nota 1 acima, sem check) | `apps/cli/src/mcp.rs:22-25` |
| Nota 4: cadeia de causas repetida | open (nota 2 acima, sem check) | - |

Nenhum check foi enfraquecido entre as rodadas. Do C7 ao C31, ao C24 e ao C36, cada correção
subiu a asserção até o AC.
