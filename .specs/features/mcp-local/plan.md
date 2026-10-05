# mcp-local — servidor MCP local, stdio e só leitura, sobre o histórico de ditados

## Problem

Quem usa o Claude Code (Linux do trabalho) ou o Claude Desktop (Windows) e quer "o que eu ditei
sobre o contrato semana passada" hoje copia e cola do `fala-cli history search` ou do espelho
`Ditados/` à mão. Granola e Wispr resolvem isso com um MCP remoto, o que exige backend (ADR-0008).
Um servidor de arquivos genérico apontado para `Ditados/` já funcionaria, mas leria também os
ditados marcados `sensitive: true` (door 8 do `storage-history`), e então o assistente contornaria
uma escolha explícita do usuário. Não há número de incidente: o custo é de produto (o histórico
não chega ao assistente) e de privacidade (o único caminho que chega hoje não respeita a marca).
Ninguém mediu ainda quanto um servidor MCP custa ao binário e ao `Cargo.lock`; a ADR-0010
(`proposed`) precisa desses números.

Quando isto fechar, `fala-cli mcp` serve por stdio um MCP só de leitura sobre `fala.sqlite`, com
`search_dictations`, `get_dictation` e as resources `fala://dictation/<id>`, desligado até o
usuário criar `mcp.toml` com `enabled = true` na pasta de dados, e nunca entrega ditado sensível,
áudio, chave nem caminho de disco. Um teste fala JSON-RPC com o binário de verdade, e o tamanho do
binário e as dependências novas ficam medidos.

## Flow

Reusa `fala_storage::Store::open`, `Store::search` e `Store::get` (busca sem acento, prefixo,
ordem por `created_at`) e o default de pasta de dados da CLI; o `fala-mcp` não escreve SQL nem
conhece o schema.

1. o cliente MCP lança `fala-cli mcp [--data-dir <dir>]` -> `Cli` clap em `apps/cli/src/main.rs` (exists) - resolve a pasta de dados (door 1) e chama `fala_mcp::run_stdio`
2. `fala_mcp` (new, door 1) - lê `<data-dir>/mcp.toml` (door 7); ligado e com `fala.sqlite` existente, abre `fala_storage::Store` (exists) pelo caminho da door 9
3. transporte stdio do `rmcp` (new, door 6) - uma mensagem JSON-RPC por linha do stdin, despachada para o `ServerHandler` escrito à mão (door 6)
4. `tools/call` -> `Store::search` / `Store::get` (exists) - descarta os `sensitive` (door 5) e monta a saída (door 3)
5. `resources/list`, `resources/templates/list`, `resources/read` -> os mesmos `Store::search` / `Store::get`, endereçados por `fala://dictation/<id>` (door 4)
6. out: uma resposta JSON-RPC por linha no stdout, log no stderr; EOF no stdin encerra com exit 0

## Impact

| Front | What changes |
| --- | --- |
| domain | novo termo: `McpConfig` - o consentimento lido de `<data-dir>/mcp.toml` (só `enabled`); vive em `fala-mcp` |
| domain | termo existente: `DictationRecord.sensitive` (door 8 do `storage-history`) era só gravado; `fala-mcp` é o primeiro que filtra por ele. Ninguém mais ramifica nele hoje |
| stored data | nada a migrar: o servidor só lê `fala.sqlite` e `mcp.toml`, nunca os cria nem os altera |
| build | `fala-mcp` depende de `rmcp ~3.5` (sem macros, sem rede), `tokio`, `toml`, `serde_json`, `fala-storage`; `fala-cli` passa a depender de `fala-mcp`. Números medidos no fim do build: binário e `Cargo.lock` (seção `## Measured` abaixo, mantida verdadeira) |
| docs | `ARCHITECTURE.md`: linha nova no Code Map para `crates/mcp`. A frase "as chamadas de rede opcionais são três" não muda aqui: é da ADR-0010 (`proposed`) |

## Relations

None - nenhuma mudança na forma dos dados guardados; o servidor só lê `dictations` pelo `Store`.
O formato do `mcp.toml` é contrato de arquivo e está na door 7.

## Surface

Só o que esta feature cria. O protocolo é MCP sobre stdio; nenhuma rota é HTTP.

| Route | In | Out | Status |
| --- | --- | --- | --- |
| `fala-cli mcp` | `[--data-dir <dir>]` (default `<dirs::data_dir()>/br.com.augusto.fala`) | stdout: só JSON-RPC 2.0, uma mensagem por linha; stderr: log | exit `0` no EOF do stdin · `1` `fala.sqlite` existe mas não abre, ou a primeira mensagem não abre a sessão · `2` argumentos · sem status HTTP (local; 200-599 n/a) |
| MCP `server/discover` e `initialize` (legado) | `_meta` com `protocolVersion` e `clientCapabilities` (discover) · `InitializeRequestParams` (legado) | `serverInfo.name = "fala"`, capacidades `tools` e `resources` | resultado · erro JSON-RPC `-32602` sem `_meta` obrigatório (SDK), seguido de exit `1` · sem status HTTP (local; 200-599 n/a) |
| MCP `tools/list` | - | `search_dictations`, `get_dictation` com schema de entrada e anotações | resultado · sem status HTTP (local; 200-599 n/a) |
| MCP `tools/call` `search_dictations` | `query?: string`, `limit?: 1..=50` (default 20) | `structuredContent.items[]` mais o mesmo JSON num bloco `text` | resultado · `isError` (desligado, falha de banco) · erro JSON-RPC `-32602` (argumento inválido) · sem status HTTP (local; 200-599 n/a) |
| MCP `tools/call` `get_dictation` | `id: string` | `structuredContent` com um ditado mais o mesmo JSON num bloco `text` | resultado · `isError` (inexistente ou sensível, desligado, falha de banco) · erro JSON-RPC `-32602` (argumento inválido, tool desconhecida) · sem status HTTP (local; 200-599 n/a) |
| MCP `resources/list` | - | até 50 `fala://dictation/<id>` | resultado (vazio quando desligado) · sem status HTTP (local; 200-599 n/a) |
| MCP `resources/templates/list` | - | o template `fala://dictation/{id}` | resultado · sem status HTTP (local; 200-599 n/a) |
| MCP `resources/read` | `uri` | um conteúdo `application/json` | resultado · erro JSON-RPC "resource not found" (`-32602` em 2026-07-28, `-32002` no legado) · sem status HTTP (local; 200-599 n/a) |
| MCP `resources/list` e `resources/read` com falha de banco (adicionada na rodada 2 da verificação) | os mesmos de cima | nada além do erro | erro JSON-RPC `-32603` com a mensagem exata `storage error` · sem status HTTP (local; 200-599 n/a) |
| `fala_mcp` (API Rust) | `run_stdio(data_dir: &Path)` | `Result<(), McpError>` | `Ok` no EOF · `Err(Open)` · `Err(Protocol)` · sem status HTTP (local; 200-599 n/a) |

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. nome do servidor e ponto de entrada | `serverInfo.name = "fala"`; subcomando `fala-cli mcp`; crate lib `crates/mcp` = pacote `fala-mcp`, `fala_mcp::run_stdio(&Path)`; a mesma pasta de dados do `fala-cli history` | `fala.exe --mcp` no binário do desktop: ele monta `tauri::Builder` com log no console e single-instance, e um byte no stdout quebra o protocolo. Nomes `fala-history`/`fala-server`: vão para o JSON de cada cliente e trocar quebra a config de todos |
| 2. nomes e entrada das tools | `search_dictations { query?: string, limit?: integer 1..=50, default 20 }` e `get_dictation { id: string }`, inglês `snake_case`, `additionalProperties: false`; anotações `readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false` | nomes em pt-BR (`buscar_ditados`): prompts e hábitos citam o nome, e o resto do ecossistema (Granola, Minutes) é inglês. Filtros `since`/`until`/`app`/`cursor` agora: viram contrato antes de existir consumidor (Out of scope) |
| 3. campos de saída | item de busca `{ id, created_at, app, language, edited_by, showing, text }` com `text` = o texto do lado de `showing`; `get_dictation` `{ id, created_at, app, language, edited_by, showing, final, raw, uri }`; `created_at` RFC 3339 com offset, `app` `null` se ausente, `edited_by`/`showing`/`language` com os literais do banco (doors 5 e 7 do `storage-history`). Lista fechada: nenhum outro campo sai | serializar o `DictationRecord` inteiro: carrega `sensitive` e a forma interna dos tipos do core, e todo campo novo do storage vazaria pelo MCP sem decisão |
| 4. esquema de URI das resources | `fala://dictation/{id}` (template e `resources/list`), `mimeType: "application/json"`, conteúdo = o mesmo objeto do `get_dictation` | `file://` com o caminho real: revela nome de usuário e pastas e obriga a sanitizar traversal. `fala://ditados/{id}` da pesquisa 17: o nome em pt-BR destoa das tools. `text/markdown` igual ao espelho: o `mirror::render` é privado do `fala-storage`, que esta feature só lê, e uma segunda cópia do formato divergiria |
| 5. política de exclusão | `sensitive = true` é inexistente para o MCP: some da busca e da listagem, e `get_dictation`/`resources/read` respondem igual a um id que não existe | devolver um esboço "restrito" (como o Minutes): confirma que o item existe. Expor tudo e confiar no cliente: texto que saiu para um provedor não volta |
| 6. transporte e SDK | `rmcp = { version = "~3.5", default-features = false, features = ["server", "transport-io"] }`; `ServerHandler` implementado à mão, sem `rmcp-macros`; só stdio; versões de protocolo = as que o SDK conhece (discover 2026-07-28 e `initialize` legado) | macros `#[tool]`: puxam `syn` 3 e `darling` 0.24, novas no lock, numa máquina com pouca RAM de build. HTTP local: porta aberta, `Origin`, token, DNS rebinding (pesquisa 17 §5.1) |
| 7. consentimento | arquivo `<data-dir>/mcp.toml` com `enabled = true` (booleano TOML); ausente, ilegível ou qualquer outro valor = desligado; desligado, toda `tools/call` devolve `isError: true` com `Fala MCP is disabled: set enabled = true in mcp.toml in the Fala data folder`, `resources/list` vem vazio e o banco nem é aberto | flag na linha de comando ou variável de ambiente: mora no JSON do cliente, e quem edita esse JSON passa a ser a autoridade (pesquisa 17, D8). Tabela no `fala.sqlite`: acopla o schema do storage a uma config de UI e exigiria escrever no banco |
| 8. ponto de abertura do banco | `fala-mcp` abre com `fala_storage::Store::open` só se `<data-dir>/fala.sqlite` já existe, e chama só `search` e `get`; sem banco, a busca devolve `items: []` | conexão `rusqlite` própria com `SQLITE_OPEN_READ_ONLY` e SQL no `fala-mcp`: duplica a montagem da consulta FTS5 e o conhecimento do schema fora do storage, e quebra na próxima migração dele. O `Store::open_read_only` fica para a ADR-0010 (Out of scope) |
| 9. erro de banco não vaza detalhe | qualquer `StorageError` que não seja `NotFound` vira `isError: true` com o texto `storage error`; a causa vai só para o stderr | repassar a mensagem do erro: `Io`/`Mirror` carregam caminhos de disco |

- Nothing else in this change is hard to reverse: layout dos módulos, textos de erro além dos
  literais acima, `ttlMs` e o nível de log mudam num commit.

## Criteria

### S1: o cliente descobre o servidor (P1)

Um cliente moderno ou legado conecta e vê as duas tools.

**Acceptance Criteria**

1. WHEN o cliente manda `server/discover` com `_meta` de 2026-07-28 THEN the system SHALL responder com `supportedVersions` contendo `"2026-07-28"`, capacidades `tools` e `resources`, e `serverInfo.name = "fala"` (no `_meta["io.modelcontextprotocol/serverInfo"]` do resultado, onde a 2026-07-28 o põe)
2. WHEN o cliente manda `initialize` com `protocolVersion` `2025-11-25` THEN the system SHALL responder com `serverInfo.name = "fala"` e as capacidades `tools` e `resources`
3. WHEN o cliente manda `tools/list` THEN the system SHALL listar exatamente `search_dictations` e `get_dictation`, nessa ordem, cada uma com `readOnlyHint: true`, `destructiveHint: false`, `idempotentHint: true` e `openWorldHint: false`
4. The system SHALL escrever no stdout só mensagens JSON-RPC 2.0, uma por linha, e todo log no stderr
5. WHEN o stdin fecha, antes ou depois de qualquer mensagem THEN the system SHALL encerrar com exit 0 em até 5 s
6. WHILE a versão negociada é 2026-07-28 the system SHALL incluir `ttlMs` e `cacheScope: "private"` nos resultados de `tools/list`, `resources/list`, `resources/templates/list` e `resources/read`
35. IF a primeira mensagem do stdin é uma notificação, ou um `server/discover` sem os campos obrigatórios do `_meta` THEN `fala-cli mcp` SHALL sair com 1 e o motivo no stderr, depois de responder `-32602` ao request sem `_meta`

**Independent test:** subir `fala-cli mcp --data-dir <tmp>`, mandar `server/discover` e `tools/list` pelo stdin, ler as respostas do stdout.

### S2: buscar e ler ditados (P1)

O assistente acha um ditado por palavra sem acento e lê o item inteiro.

**Acceptance Criteria**

7. WHEN `search_dictations` recebe `query` THEN the system SHALL devolver em `structuredContent.items` os ditados que o `Store::search` devolve para essa consulta (sem acento, cada termo como prefixo, todos os termos), mais recentes primeiro
8. The system SHALL dar a cada item da busca exatamente as chaves `id`, `created_at`, `app`, `language`, `edited_by`, `showing` e `text`, com `text` igual ao texto do lado de `showing`
9. WHEN `query` falta ou é só espaço THEN `search_dictations` SHALL devolver os ditados mais recentes até `limit`
10. WHERE `limit` não é dado THEN `search_dictations` SHALL usar 20
11. IF `limit` está fora de 1..=50 ou não é inteiro THEN the system SHALL responder erro JSON-RPC `-32602`
12. WHEN `get_dictation` recebe o `id` de um ditado existente THEN the system SHALL devolver `structuredContent` com exatamente as chaves `id`, `created_at`, `app`, `language`, `edited_by`, `showing`, `final`, `raw` e `uri`, com `uri = "fala://dictation/<id>"`
13. IF `get_dictation` recebe um `id` que não existe THEN the system SHALL devolver `isError: true` com o texto `dictation not found: <id>`
14. IF `get_dictation` vem sem `id` string, ou o nome da tool é desconhecido THEN the system SHALL responder erro JSON-RPC `-32602`
15. WHEN o cliente manda `resources/templates/list` THEN the system SHALL devolver um único template com `uriTemplate = "fala://dictation/{id}"`
16. WHEN o cliente manda `resources/list` THEN the system SHALL listar até 50 ditados, mais recentes primeiro, cada um com `uri = "fala://dictation/<id>"` e `mimeType = "application/json"`
17. WHEN `resources/read` recebe `fala://dictation/<id>` de um ditado existente THEN the system SHALL devolver um conteúdo `application/json` cujo texto é o mesmo objeto do `get_dictation` desse id
18. IF `resources/read` recebe uma URI fora de `fala://dictation/` ou um id que não existe THEN the system SHALL responder erro JSON-RPC de resource não encontrada (`-32602` em 2026-07-28)
37. WHILE a sessão foi aberta por `initialize` 2025-11-25, IF `resources/read` recebe um id que não existe THEN the system SHALL responder erro JSON-RPC `-32002` (adicionado na rodada 2 da verificação, para a linha de `Surface` que já o nomeava)

**Independent test:** gravar três ditados com `fala-cli history add`, ligar o `mcp.toml`, buscar "acao", ler um id por tool e por resource.

### S3: o que nunca sai (P1)

Ditado sensível, caminho de disco e detalhe de erro não chegam ao cliente.

**Acceptance Criteria**

19. IF um ditado tem `sensitive = true` THEN `search_dictations` SHALL omiti-lo mesmo quando ele casa a consulta, e SHALL completar o `limit` com os ditados não sensíveis seguintes
20. IF `get_dictation` recebe o id de um ditado sensível THEN the system SHALL devolver a mesma resposta de um id inexistente: `isError: true` e `dictation not found: <id>`
21. IF `resources/read` recebe a URI de um ditado sensível THEN the system SHALL responder o mesmo erro de um id inexistente, e `resources/list` SHALL omiti-lo
22. The system SHALL nunca incluir em resposta nenhuma o caminho da pasta de dados nem a chave `sensitive`
23. IF uma leitura do banco falha com erro que não é `NotFound` THEN the system SHALL devolver `isError: true` com o texto exato `storage error` e escrever a causa só no stderr
36. IF a leitura do banco falha em `resources/list` ou `resources/read` THEN the system SHALL responder erro JSON-RPC `-32603` com a mensagem exata `storage error` e escrever a causa só no stderr (adicionado na rodada 2 da verificação)

**Independent test:** gravar um ditado com `sensitive = true` (`Store::add_sensitive`) com uma palavra única e conferir busca, get, read e list.

### S4: desligado por padrão (P1)

Sem consentimento explícito, o servidor não entrega nada.

**Acceptance Criteria**

24. WHILE `<data-dir>/mcp.toml` não existe the system SHALL devolver em toda `tools/call` `isError: true` com o texto `Fala MCP is disabled: set enabled = true in mcp.toml in the Fala data folder`, e `resources/list` SHALL vir vazio
25. IF `mcp.toml` não é TOML válido, ou `enabled` falta ou não é o booleano `true` THEN the system SHALL tratar como desligado (o mesmo comportamento do AC 24) e escrever um aviso no stderr
26. WHILE desligado the system SHALL não abrir `fala.sqlite`
27. WHILE desligado `tools/list` SHALL listar as duas tools, para o modelo poder explicar como ligar

**Independent test:** a mesma sessão do S2 sem `mcp.toml`, e com `enabled = "yes"`.

### S5: só leitura, sem rede (P1)

O servidor não muda nada em disco e não tem como abrir socket.

**Acceptance Criteria**

28. IF `fala.sqlite` não existe THEN `search_dictations` SHALL devolver `items: []`, `get_dictation` SHALL devolver `dictation not found: <id>`, e o arquivo SHALL continuar inexistente depois da sessão
29. The system SHALL deixar `fala.sqlite` byte a byte igual depois de uma sessão com busca, get, list e read
30. IF `fala.sqlite` existe mas não é um banco SQLite THEN `fala-cli mcp` SHALL sair com 1 e o motivo no stderr, sem escrever nada no stdout
31. The system SHALL não ter no grafo de dependências normais de `fala-mcp` nenhum de `hyper`, `reqwest`, `rustls`, `native-tls`, `oauth2` ou `tauri`
32. The system SHALL nunca escrever texto ditado no log acima de `debug`: no nível padrão `info`, o stderr de uma sessão não contém nenhum texto bruto ou final

**Independent test:** `sha256sum fala.sqlite` antes e depois da sessão; `cargo tree -p fala-mcp -e normal`.

### S6: `fala-cli mcp` (P1)

O subcomando é o único jeito de subir o servidor nesta rodada.

**Acceptance Criteria**

33. WHERE `--data-dir` não é dado THEN `fala-cli mcp` SHALL usar `<dirs::data_dir()>/br.com.augusto.fala`, a mesma pasta do `fala-cli history`
34. IF `fala-cli mcp` recebe um argumento desconhecido THEN the system SHALL sair com 2 sem ler o stdin

**Independent test:** `fala-cli mcp --help` e `fala-cli mcp --bogus; echo $?`.

## Out of scope

Product capabilities only. Process and harness rules live in AGENTS.md or as Observable `n/a`.

| Excluded | Why |
| --- | --- |
| Tools e resources de reunião (`list_meetings`, `get_meeting`, `get_transcript_segment`, `search_meetings`) | dependem do schema de reuniões da fase 2; os nomes ficam reservados pela pesquisa 17 |
| Paginação por `cursor`, filtros `since`/`until`/`app`, `include_raw` na busca | o `Store::search` não tem cursor nem filtro; entram com a API do storage que os suporte |
| Teto de 30.000 caracteres por resultado | um ditado é curto e o `limit` máximo é 50; entra com as transcrições de reunião |
| Auditoria de acessos (S9 da pesquisa 17) e tela "Acessos de IA" | precisa de onde gravar e de UI; o log no stderr é o rastro do spike |
| Prompts MCP | não pedidos no spike; a superfície v1 é tools + resources |
| Binário enxuto `fala-mcp` como sidecar no instalador e MCPB | é distribuição (`TODO(windows)`); o spike mede o tamanho dele por um `example` |
| Toggle no desktop que escreve o `mcp.toml` | é UI; o usuário cria o arquivo à mão no spike |
| `Store::open_read_only` com `SQLITE_OPEN_READ_ONLY` | mexe no `fala-storage`, que esta feature só lê; vai para a ADR-0010 |
| HTTP local, servidor remoto, ChatGPT | ADR-0008 e pesquisa 17 §4; só stdio |
| Qualquer tool de escrita, gravação ou ditado | ADR-0005; só leitura |

## Assumptions

Defaults that are not already a numbered criterion. Drop a row once it is.

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| `fala-cli mcp` atrás de uma feature de cargo | sem feature: o subcomando compila sempre e o opt-in é de execução (door 7) | uma feature dobraria a matriz de teste da CLI; o custo de build e de binário fica medido em `## Measured`. Confirmed? y — delegado pelo Augusto em 2026-10-02, decidido pelo painel | y |
| `get_dictation` devolve o bruto | sim, `final` e `raw` lado a lado; a busca devolve só `text` | é uma leitura explícita de um item, e o par bruto/final é o que o "desfazer" do histórico mostra. Confirmed? y — delegado pelo Augusto em 2026-10-02, decidido pelo painel | y |
| Mensagens ao modelo | em inglês, curtas, como as tools | o modelo lê nome, descrição e erro; inglês casa com o ecossistema. Confirmed? y — delegado pelo Augusto em 2026-10-02, decidido pelo painel | y |
| Leitura concorrente com o desktop gravando | WAL + `busy_timeout` 5000 ms do `Store::open` | o SQLite em WAL não bloqueia leitor por escritor; nada novo a fazer. Confirmed? y — delegado pelo Augusto em 2026-10-02, decidido pelo painel | y |
| Schema mais novo que o binário (`user_version` > 1) | não tratado no spike; o `Store` lê o que conseguir | o storage só tem o schema 1; a regra "atualize o Fala" vai para a ADR-0010. Confirmed? y — delegado pelo Augusto em 2026-10-02, decidido pelo painel | y |
| Medida de tamanho | `fala-cli` release antes e depois, e um `example` release em `crates/mcp` que só serve stdio, como proxy do sidecar | o `fala-cli` carrega onnx e whisper.cpp e não diz quanto pesa um sidecar. Confirmed? y — delegado pelo Augusto em 2026-10-02, decidido pelo painel | y |

**Open questions:** none - all resolved or logged above.

## Observable

Worksheet, not the review. `n/a` needs its reason. One row may group the same decision
across several routes.

| Surface | Decision | Landing |
| --- | --- | --- |
| command `fala-cli mcp` | output format | AC 4 - só JSON-RPC no stdout |
| command `fala-cli mcp` | every flag and its default | AC 33 |
| command `fala-cli mcp` | exit codes | AC 5 (0), AC 30 e AC 35 (1), AC 34 (2) |
| command `fala-cli mcp` | verbosity | AC 4, AC 32 - log no stderr, nível `info` por padrão sem conteúdo |
| command `fala-cli mcp` | what it prints when it fails halfway | AC 30 (banco ilegível na abertura); AC 23 (falha numa leitura no meio da sessão) |
| API MCP `tools/call` | response shape | AC 8, AC 12 |
| API MCP `tools/call`, `resources/read` | error shape and codes | AC 11, 13, 14, 18, 20, 23, 24 |
| API MCP | who may call it | AC 24, AC 25 (consentimento); o resto é stdio: só o processo que lançou o servidor fala com ele |
| API MCP | versioning | AC 1, AC 2 - negociação de versão do SDK; AC 6 para os campos de 2026-07-28 |
| API MCP | rate limits | n/a - um processo por cliente, consultas locais de milissegundos, sem custo por chamada |
| document resource `fala://dictation/<id>` | structure | AC 17 - o mesmo objeto do `get_dictation` |
| document resource `fala://dictation/<id>` | what the reader does next | n/a - é contexto que o modelo lê; não há ação no Fala a partir dele |
| collection resultado da busca e `resources/list` | ordering | AC 7, AC 16 - mais recentes primeiro |
| collection resultado da busca e `resources/list` | naming | AC 16 - `fala://dictation/<id>` |
| collection resultado da busca e `resources/list` | duplicates | existing - `id` único no `fala.sqlite` (door 3 do `storage-history`) |
| collection resultado da busca e `resources/list` | grouping criterion | n/a - lista plana por tempo; agrupar é do cliente |
| collection resultado da busca e `resources/list` | the exception that does not fit | AC 19, AC 21 - ditado sensível |
| screen | n/a - nenhuma tela; o toggle do desktop está fora do escopo |

## Measured

Mantida verdadeira no build. Medido em 2026-10-02 no Ubuntu 25.04 x86_64, `profile.release` do
workspace (`lto = true`, `codegen-units = 1`, `strip = true`), `CARGO_BUILD_JOBS=2`.

| O quê | Antes (`f058e5c`) | Depois | Diferença |
| --- | --- | --- | --- |
| `fala-cli` release | 28.077.536 bytes (26,8 MiB) | 29.606.048 bytes (28,2 MiB) | +1.528.512 bytes (+1,46 MiB, +5,4 %) |
| binário só do servidor (`cargo build --release -p fala-mcp --example stdio`, proxy do sidecar `fala-mcp`) | - | 4.022.464 bytes (3,84 MiB) | - |
| pacotes no `Cargo.lock` | 861 | 865 | +4: `fala-mcp` 0.1.0 (path), `rmcp` 3.5.0, `pastey` 0.2.3, `schemars_derive` 1.2.1 |
| grafo normal de `fala-mcp` (`cargo tree -e normal`, únicos, sem a marca `(*)`) | - | 68 pacotes, nenhum de rede | - |

- Os três pacotes de terceiros novos: `rmcp` (Apache-2.0, o SDK), `pastey` 0.2 (proc-macro
  pequeno exigido pela feature `server`; o lock já tinha o 0.1.1) e `schemars_derive` 1.2.1 (o
  `schemars` 1.2.1 já estava no lock, sem o derive). `syn` 3 e `darling` 0.24 não entram.
- `tokio` e `toml` já estavam no lock; o `fala-cli` passa a compilar `tokio` (rt, time, io-std).
- Build release do `fala-cli` depois da mudança: 4 min 27 s incremental (base: 1 min 26 s, quase
  tudo em cache); o `example` sozinho: 2 min 16 s.
- O `Cargo.lock` muda só por adição; as duas linhas trocadas (`schemars_derive 0.8.22`,
  `pastey 0.1.1`) são a desambiguação de versão que o cargo escreve quando passa a haver duas.

## Sources

- `docs/decisions/0002`, `0006`, `0008` - crates sem `tauri`, SQLite fonte de verdade, sem backend nem chave
- `~/projects/fala-research/research/17-mcp-local-para-o-fala.md` §5 - desenho, regras S1-S12, portas D1-D8
- `~/projects/fala-research/pitches/fase-4-mcp-local.md` - escopo do spike e no-gos
