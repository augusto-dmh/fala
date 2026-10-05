# mcp-local checks

Profile: light
Plan: `.specs/features/mcp-local/plan.md`

Todas as provas rodam com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`
(abreviado como `$ENV` abaixo). Os testes de `apps/cli/tests/mcp.rs` sobem o binário `fala-cli`
de verdade e falam JSON-RPC pelo stdin/stdout.

37 checks in 6 slices · 9 one-way doors · 0 open, of which 0 block

## Checks

Grouped by the spec's slices; numbering runs across the whole feature.

### S1 - o cliente descobre o servidor · 4 files · 60 KB · ~15k

**C1** - `server/discover` com `_meta` 2026-07-28 responde `supportedVersions` com `"2026-07-28"`, capacidades `tools` e `resources`, e `_meta["io.modelcontextprotocol/serverInfo"].name = "fala"` (onde a 2026-07-28 põe o `serverInfo`) (AC 1)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact discover_reports_fala_with_tools_and_resources`

**C2** - `initialize` com `protocolVersion` `2025-11-25` responde `serverInfo.name = "fala"` e as capacidades `tools` e `resources` (AC 2)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact initialize_legacy_reports_fala`

**C3** - `tools/list` lista exatamente `search_dictations` e `get_dictation`, nessa ordem, com `readOnlyHint: true`, `destructiveHint: false`, `idempotentHint: true`, `openWorldHint: false` (AC 3)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact tools_list_has_two_read_only_tools_in_order`

**C4** - Numa sessão com discover, list, search, get e read, cada linha do stdout é um objeto JSON com `"jsonrpc": "2.0"`, e nenhuma linha do stdout é log (AC 4)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact stdout_is_only_json_rpc_lines`

**C5** - Fechar o stdin depois de um discover, e também sem nenhuma mensagem, encerra o processo com exit 0 em até 5 s (AC 5)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact eof_exits_zero_within_five_seconds`

**C6** - Em 2026-07-28, os resultados de `tools/list`, `resources/list`, `resources/templates/list` e `resources/read` trazem `ttlMs` numérico e `cacheScope: "private"` (AC 6)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact cacheable_results_carry_ttl_and_private_scope`

**C7** - Uma notificação como primeira mensagem faz `fala-cli mcp` sair com 1 e o stderr trazer o motivo (`sessão MCP: expect initialized request, but received: ...`); um `server/discover` sem `_meta` recebe erro `-32602` e o processo sai com 1 com o mesmo motivo no stderr (AC 35; elevado nas rodadas 2 e 4: o stderr "não vazio" e depois só o rótulo ficavam abaixo do AC)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact bad_opening_message_exits_one`

### S2 - buscar e ler ditados · 3 files · 50 KB · ~13k

**C8** - `search_dictations { query: "acao" }` devolve os dois ditados que têm "ação" (um só no bruto), o mais recente primeiro, e não devolve o terceiro; `amanh` acha só o de "amanhã" (prefixo) e `acao contr` só o que tem os dois termos (AC 7)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact search_finds_without_accent_newest_first`

**C9** - Cada item da busca tem exatamente as chaves `id`, `created_at`, `app`, `language`, `edited_by`, `showing`, `text`, e `text` é o bruto num item desfeito e o final num item não desfeito (AC 8)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact search_items_have_exactly_the_contract_keys`

**C10** - `search_dictations` sem `query` e com `query: "   "` devolve os ditados mais recentes, em ordem decrescente, cortados no `limit: 2` (AC 9)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact search_without_query_lists_most_recent`

**C11** - Com 25 ditados gravados, `search_dictations {}` devolve 20 itens (AC 10)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact search_limit_defaults_to_twenty`

**C12** - `limit` 0, 51, -1, `"5"` e 5.5 recebem erro JSON-RPC `-32602`; `limit` 1 e 50 são aceitos (AC 11)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact search_limit_out_of_range_is_invalid_params`

**C13** - `get_dictation` de um id existente devolve exatamente as chaves `id`, `created_at`, `app`, `language`, `edited_by`, `showing`, `final`, `raw`, `uri`, com `uri = "fala://dictation/<id>"` e os valores gravados (AC 12)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact get_returns_exactly_the_contract_keys`

**C14** - `get_dictation` de um UUID que não existe devolve `isError: true` e o texto `dictation not found: <id>` (AC 13)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact get_unknown_id_is_not_found_error`

**C15** - `get_dictation {}`, `get_dictation { id: 7 }` e a tool `delete_dictation` recebem erro JSON-RPC `-32602` (AC 14)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact get_without_id_or_unknown_tool_is_invalid_params`

**C16** - `resources/templates/list` devolve um único template com `uriTemplate = "fala://dictation/{id}"` (AC 15)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact templates_list_has_dictation_template`

**C17** - Com 52 ditados, `resources/list` devolve exatamente as URIs `fala://dictation/<id>` dos 50 mais recentes, em ordem decrescente, cada uma com `mimeType = "application/json"` (AC 16)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact resources_list_newest_first_with_uri_and_mime`

**C18** - `resources/read` de `fala://dictation/<id>` devolve um conteúdo `application/json` cujo texto, parseado, é igual ao `structuredContent` do `get_dictation` do mesmo id (AC 17)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact read_returns_same_object_as_get`

**C19** - `resources/read` de `fala://dictation/<uuid inexistente>`, `file:///etc/passwd` e `fala://meeting/x` recebe erro JSON-RPC `-32602` (AC 18)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact read_unknown_or_foreign_uri_is_not_found`

### S3 - o que nunca sai · 2 files · 40 KB · ~10k

**C20** - Um ditado sensível com a palavra "segredo" não aparece em `search_dictations { query: "segredo" }` nem na busca vazia; com 3 normais e 1 sensível mais recente, `limit: 3` devolve os 3 normais (AC 19)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact sensitive_is_omitted_from_search_and_limit_is_filled`

**C21** - `get_dictation` do id sensível devolve exatamente o mesmo resultado de um id inexistente, trocado só o id: `isError: true`, `dictation not found: <id>` (AC 20)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact sensitive_get_matches_nonexistent`

**C22** - `resources/read` do id sensível recebe o mesmo código e mensagem de erro de um id inexistente, trocado só o id, e `resources/list` não contém a URI dele (AC 21)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact sensitive_read_matches_nonexistent_and_list_omits`

**C23** - Nenhuma linha do stdout de uma sessão completa (discover, list, search, get, read, erros) contém o caminho da pasta de dados nem a string `"sensitive"` (AC 22)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact responses_never_contain_data_dir_or_sensitive_key`

**C24** - Com a tabela `dictations` renomeada no meio da sessão, `search_dictations` e `get_dictation` devolvem `isError: true` com o texto exato `storage error`, e o stderr traz a causa (`falha ao ler o histórico: banco: no such table ...`) enquanto nenhuma resposta do stdout a contém: o resultado tem um único bloco, sem a causa (AC 23; elevado na rodada 3: "menciona a falha" ficava abaixo do AC)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact storage_failure_is_opaque_error`

**C36** - Com a tabela `dictations` renomeada no meio da sessão, `resources/list` e `resources/read` recebem erro JSON-RPC `-32603` com a mensagem exata `storage error` e `error.data` nulo, e o stderr traz a causa (`no such table`) que nenhuma linha do stdout contém (AC 36; elevado na rodada 3)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact resource_storage_failure_is_opaque_error`

**C37** - Numa sessão aberta por `initialize` 2025-11-25, `resources/read` de um id inexistente recebe erro `-32002` (AC 37)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact legacy_read_missing_is_resource_not_found`

### S4 - desligado por padrão · 2 files · 35 KB · ~9k

**C25** - Sem `mcp.toml`, `search_dictations` e `get_dictation` devolvem `isError: true` com `Fala MCP is disabled: set enabled = true in mcp.toml in the Fala data folder`, e `resources/list` vem vazio (AC 24)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact disabled_without_toml_refuses_every_call`

**C26** - `mcp.toml` com `enabled = "yes"`, `enabled = false`, sem `enabled` e com TOML inválido dá o mesmo `isError` do C25, e o stderr traz um aviso `WARN` com o motivo nos quatro casos (AC 25; corrigido na rodada 2: a versão 1 pedia aviso só em dois casos, mais estreita que o AC)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact invalid_or_false_toml_is_disabled`

**C27** - Desligado e com um `fala.sqlite` que não é banco SQLite, o servidor responde ao discover e a uma `tools/call` sem sair, prova de que não abriu o banco (AC 26)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact disabled_does_not_open_db`

**C28** - Desligado, `tools/list` lista `search_dictations` e `get_dictation` (AC 27)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact disabled_still_lists_tools`

### S5 - só leitura, sem rede · 2 files · 30 KB · ~8k

**C29** - Ligado e sem `fala.sqlite`, a busca devolve `items: []`, o get devolve `dictation not found: <id>`, e depois do EOF o arquivo continua inexistente (AC 28)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact missing_db_gives_empty_and_is_not_created`

**C30** - O conteúdo de `fala.sqlite` é byte a byte o mesmo antes e depois de uma sessão com busca, get, `resources/list` e `resources/read` (AC 29)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact session_leaves_db_bytes_unchanged`

**C31** - Ligado e com um `fala.sqlite` que não é banco SQLite, `fala-cli mcp` sai com 1, stdout vazio e o motivo no stderr (`abrir o histórico: ... not a database`) (AC 30; elevado na rodada 3: "stderr não vazio" ficava abaixo do AC)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact non_sqlite_db_exits_one_with_empty_stdout`

**C32** - `cargo tree -p fala-mcp -e normal` não lista `hyper`, `reqwest`, `rustls`, `native-tls`, `oauth2` nem `tauri` (AC 31)
Proof: `! $ENV cargo tree -p fala-mcp -e normal --prefix none --format '{p}' | grep -E '^(hyper|reqwest|rustls|native-tls|oauth2|tauri)[ -]'`

**C33** - Sem `RUST_LOG` (nível `info`), o stderr de uma sessão com busca e get não contém nenhum dos textos brutos ou finais gravados (AC 32)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact default_log_level_never_prints_dictated_text`

### S6 - `fala-cli mcp` · 2 files · 20 KB · ~5k

**C34** - Sem `--data-dir`, com `XDG_DATA_HOME` apontando para um diretório temporário, um ditado gravado por `fala-cli history add` (também sem `--data-dir`) é achado por `search_dictations` (AC 33)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact default_data_dir_is_the_history_dir`

**C35** - `fala-cli mcp --bogus` sai com 2 sem esperar o stdin (AC 34)
Proof: `$ENV cargo test -p fala-cli --test mcp -- --exact unknown_argument_exits_two`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| `fala-cli mcp` exit codes (3) | `0` C5 · `1` C7, C31 · `2` C35 | - |
| `server/discover`/`initialize` statuses (2) | resultado C1, C2 · `-32602` sem `_meta` C7 | - |
| `tools/list` statuses (1) | resultado C3 | - |
| `search_dictations` statuses (4) | resultado C8 · `isError` desligado C25 · `isError` banco C24 · `-32602` C12 | - |
| `get_dictation` statuses (6) | resultado C13 · `isError` inexistente C14 · `isError` sensível C21 · `isError` desligado C25 · `isError` banco C24 · `-32602` C15 | - |
| `resources/list` statuses (3) | resultado C17 · vazio desligado C25 · `-32603` banco C36 | - |
| `resources/templates/list` statuses (1) | resultado C16 | - |
| `resources/read` statuses (4) | resultado C18 · not found `-32602` C19, C22 · not found legado `-32002` C37 · `-32603` banco C36 | - |
| `fala_mcp::run_stdio` (3) | `Ok` C5 · `Err(Open)` C31 · `Err(Protocol)` C7 | - |
| chaves de saída da busca (7) | `id` C9 · `created_at` C9 · `app` C9 · `language` C9 · `edited_by` C9 · `showing` C9 · `text` C9 (igualdade exata do conjunto) | - |
| chaves de saída do get (9) | `id` C13 · `created_at` C13 · `app` C13 · `language` C13 · `edited_by` C13 · `showing` C13 · `final` C13 · `raw` C13 · `uri` C13 (igualdade exata do conjunto) | - |
| anotações das tools (4) | `readOnlyHint` C3 · `destructiveHint` C3 · `idempotentHint` C3 · `openWorldHint` C3 (as duas tools) | - |
| `limit` (7 bordas) | 0 C12 · 1 C12 · 50 C12 · 51 C12 · ausente C11 · texto C12 · fracionário C12 | - |
| `mcp.toml` estados (6) | ausente C25 · `enabled = true` C8 · `enabled = false` C26 · `enabled = "yes"` C26 · sem `enabled` C26 · TOML inválido C26 | - |
| URIs de `resources/read` (4) | `fala://dictation/<id>` existente C18 · inexistente C19 · sensível C22 · esquema estranho C19 | - |
| caminhos de exclusão do sensível (4) | busca C20 · get C21 · read C22 · list C22 | - |
| resultados cacheáveis em 2026-07-28 (4) | `tools/list` C6 · `resources/list` C6 · `resources/templates/list` C6 · `resources/read` C6 | - |
| crates de rede proibidos (6) | `hyper` C32 · `reqwest` C32 · `rustls` C32 · `native-tls` C32 · `oauth2` C32 · `tauri` C32 | - |
| doors (9) | 1 C1, C2, C34 · 2 C3, C12, C15 · 3 C9, C13 · 4 C16, C17, C18 · 5 C20, C21, C22 · 6 C32, C1, C2 · 7 C25, C26 · 8 C29, C30 · 9 C24 | - |

- Claims naming a status, route or response shape: C1-C3, C6-C19, C21-C22, C24-C26 - cada
  prova cruza a fronteira (binário real, JSON-RPC pelo stdin/stdout)
- Nenhum outro check afirma mais que o caso que a prova exercita

## Swept

- validation: C12, C15, C19 - `limit`, `id`, nome de tool e URI validados
- failure modes: C24 (falha de banco no meio), C31 (banco ilegível na abertura), C7 (abertura de sessão inválida)
- idempotency: n/a - só leitura; repetir uma chamada não muda estado (C30 prova que nada é escrito)
- authorization: C25, C26 - consentimento por `mcp.toml`; o resto é stdio, só o processo que lançou fala com o servidor
- concurrency: existing - WAL + `busy_timeout` 5000 ms em `fala_storage::Store::open` (`crates/storage/src/store.rs`)
- data lifecycle: n/a - o servidor não cria, muda nem apaga dado (C29, C30)
- dependency failure: C24, C36 - o SQLite falhando vira `storage error` sem derrubar o servidor
- state transitions: n/a - não há estado no servidor; ligado/desligado é lido na abertura (C25, C8)
- observability: C33 (sem conteúdo ditado no nível `info`), C4 (log só no stderr), C24 (falha logada no stderr)

## Handoff

- S1-S6 = ~60k (crate novo `crates/mcp` ~400 linhas, `apps/cli` subcomando e teste ~700 linhas,
  leitura de `crates/storage` 25 KB, plano e checks 35 KB; soma ~240 KB / 4), todos no mesmo
  caminho CLI -> `fala-mcp` -> storage, sob o budget de 150k - one builder
- **Boundary:** C1-C35 closed in the `feat(mcp)` commit; `cargo test -p fala-cli --test mcp` 34 passed, C32 by `cargo tree`
- **Settled mid-build:** nenhuma pergunta ao Augusto (rodada delegada). O C1 e o AC 1 passaram a dizer onde a 2026-07-28 põe o `serverInfo` do discover (`_meta["io.modelcontextprotocol/serverInfo"]`), visto no fio antes do teste; o C30 compara os bytes inteiros em vez do SHA-256 (mais forte, sem dependência nova). Os avisos "no test selector" do `validate_checks.py` são o heurístico não reconhecendo `-- --exact <nome>`, o mesmo do `storage-history`
- **Abandoned:** derrubar o runtime do tokio com `drop`: a leitura bloqueante do stdin prenderia o processo quando a sessão termina por erro com o stdin aberto; trocado por `shutdown_background`

Rodada 2 (depois do FAIL da verificação 1):

- **Boundary:** C1-C37 closed in the `feat(mcp)` commit, depois do fixup da rodada 2 (amend, antes de qualquer PR); `cargo test -p fala-cli --test mcp` 36 passed
- **Settled mid-build:** o C26 da rodada 1 pedia aviso em dois casos e o AC 25 em quatro; o código passou a avisar nos quatro (`enabled = false` e chave ausente também, em `WARN`) e o C26 foi alinhado ao AC, nunca o contrário. O C7 passou a exigir o motivo (`sessão MCP`) no stderr. O C17 passou a comparar as 50 URIs em ordem. C36 e C37 cobrem as lacunas de amostragem 3 e 5 da verificação 1 (AC 36 e 37 e uma linha de `Surface`, aditivos)
- **Abandoned:** nada

Rodada 3 (depois do FAIL da verificação 2):

- **Boundary:** C1-C37 closed in the `feat(mcp)` commit, amended again before any PR; `cargo test -p fala-cli --test mcp` 36 passed
- **Settled mid-build:** C24, C31 e C36 elevados aos AC 23, 30 e 36: o teste confere a causa no stderr e a ausência dela no stdout, não só um rótulo ou um stderr não vazio. C8 ganhou prefixo e dois termos, C12 um `limit` fracionário (lacunas de amostragem anotadas na rodada 2). Varredura dos outros checks pelo mesmo padrão ("motivo"/"causa" no AC, asserção mais fraca no teste): C7 e C26 já estavam elevados; nenhum outro
- **Abandoned:** nada

Rodada 4 (depois do FAIL da verificação 3, só no C7):

- **Boundary:** C1-C37 closed in the `feat(mcp)` commit, amended again before any PR; `cargo test -p fala-cli --test mcp` 36 passed
- **Settled mid-build:** C7 elevado ao AC 35: o teste confere o motivo do SDK (`expect initialized request, but received:`), não só o rótulo `sessão MCP`. As linhas de Coverage `get_dictation statuses` e `mcp.toml estados` diziam 5 e listavam 6; corrigido o tamanho. Rodada 4 além do limite de três do skill: decisão do painel (rodada delegada, sem quem responder), escopo reduzido ao diff desta correção, registrada no status
- **Abandoned:** nada
