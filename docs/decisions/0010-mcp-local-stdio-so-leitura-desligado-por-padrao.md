---
status: proposed
date: 2026-10-02
---
# Servidor MCP local: stdio, só leitura, desligado por padrão

## Contexto e problema
O usuário quer consultar ditados e reuniões no Claude Code (Linux), no Claude Desktop (Windows) e em outros clientes MCP sem copiar e colar. Granola e Wispr Flow fazem isso por um servidor MCP remoto (o do Granola com OAuth), o que exige backend e fere a ADR-0008. Um servidor de arquivos genérico apontado para o espelho Markdown (ADR-0006) já funcionaria sem código, mas não sabe quais ditados vieram de um app da lista "LLM desligado" nem quais sessões são "só local", e contornaria essas escolhas explícitas.

Um servidor MCP local não abre socket, mas entrega o histórico a um processo de terceiro que o manda ao provedor de LLM que *ele* escolheu. É uma saída de dados nova, que não está entre as "três chamadas de rede opcionais" do `ARCHITECTURE.md`. As boas práticas de segurança do MCP recomendam stdio para limitar o acesso ao cliente que lançou o processo, e o SDK Rust oficial (`rmcp`, Apache-2.0, Tier 1) implementa servidor stdio sem nenhuma feature de rede. O transcript de reunião é fala de terceiros (ADR-0005) e, no contexto de um modelo com acesso à web, fecha a "lethal trifecta" (dado privado, conteúdo não confiável, saída externa).

Esta ADR é proposta antes do spike de ditados previsto no pitch. O tamanho do binário, o tempo de build e o teste com o Claude Code entram nela antes do aceite.

Evidência: `fala-research/research/17-mcp-local-para-o-fala.md` (§3.3, §5.1-5.6, regras S1-S12, portas D1-D8); `fala-research/research/14-granola-mcp-e-integracoes.md` §2 e §6.2 (transporte e só leitura; o caminho de disco que o 14 sugeria expor é rejeitado aqui, seguindo o 17); `fala-research/research/16-wispr-flow-notas-e-lacunas.md` §1.2 e §4.2; `fala-research/pitches/fase-4-mcp-local.md`; `fala-research/plans/roadmap-proposta-2026-10-02.md` §3 (candidata 0010) e decisão 8. A pasta `fala-research` não é versionada neste repo.

## Opções consideradas
* Nenhum MCP: o espelho `.md` lido por um servidor genérico de arquivos.
* Binário stdio sobre `fala-storage`, num crate `crates/mcp` sem `tauri` e com `rmcp` sem features de rede; `fala-cli mcp` em dev e binário enxuto `fala-mcp` (sidecar) na distribuição.
* Servidor HTTP local embutido no `apps/desktop` (`127.0.0.1:<porta>`).
* Servidor MCP remoto (atende o ChatGPT e o claude.ai web).

## Decisão
Binário stdio só de leitura, desligado por padrão: crate `crates/mcp` (`fala-mcp`), servidor de nome `fala`, resources em URIs `fala://…`, entregue como `fala-cli mcp` em dev e como sidecar `fala-mcp` na distribuição. É o único desenho que aplica as exclusões do usuário num ponto só, sem porta aberta, sem credencial e sem backend; o servidor genérico de arquivos perde porque não conhece as exclusões, o HTTP local expõe uma porta a qualquer processo local e a DNS rebinding, e o remoto exige backend (ADR-0008).

O que o servidor **nunca expõe**, em nenhuma tool, resource ou prompt:
* áudio de qualquer tipo, nem o caminho de `audio/`: nem o de ditado (ADR-0003), nem o de reunião, que pela ADR-0005 só vai ao ASR escolhido;
* caminho de disco (as resources usam ids opacos em `fala://…`, nunca `file://`);
* `Settings`, referências de keyring, chaves, dicionário pessoal, templates, logs e o `history.db` herdado do desktop; o servidor lê só o `fala.sqlite`, via `fala-storage`;
* ditados marcados como sensíveis (feitos num app da lista "LLM desligado") e sessões "só local", que para o MCP são inexistentes: o servidor não confirma nem que existem;
* o texto de seleção que um Transform ou Command Mode enviou e o item guarda (ADR-0012, proposta);
* qualquer operação de escrita, gravação, ditado ou injeção (a gravação, pela ADR-0005; o resto, por decisão desta ADR).

### Consequências
* Bom: o histórico chega a Claude Code, Claude Desktop, Cursor, VS Code e Codex com um único binário; nada novo no keyring; quem raciocina é o cliente, então o servidor não chama LLM e não muda o custo.
* Ruim: o texto vai ao provedor que o cliente usa, fora do controle do Fala; a exclusão vale só para o `fala-mcp`: um servidor genérico apontado para o espelho continua lendo os ditados sensíveis que estiverem nele; o ChatGPT fica de fora (só aceita URL pública); o `rmcp` teve três majors em cinco meses, o que pede versão fixada e handler fino; um segundo `.exe` no instalador Windows (`TODO(windows)`: caminho instalado e SmartScreen).
* Obrigatório:
  * só stdio (`rmcp` sem `transport-streamable-http-*`, `reqwest` nem `auth`); a garantia "sem crate de rede" vale para o sidecar `fala-mcp`, não para o `fala-cli mcp` de dev, que linka o resto da CLI;
  * banco aberto com `SQLITE_OPEN_READ_ONLY` e leitura só pelo SQLite, nunca pelos `.md`; o binário só escreve o log de auditoria, fora do `fala.sqlite`;
  * a marca de sensível é gravada pelo `fala-storage` no momento do ditado; tirar o app da lista depois não libera ditados antigos;
  * o consentimento é lido de um arquivo da pasta de dados escrito pelas Configurações do Fala; variável de ambiente e argumento do cliente não o ligam;
  * reuniões têm interruptor próprio, desligado mesmo com o MCP ligado;
  * auditoria local de cada chamada só com metadados (instante, cliente, tool, ids, contagem);
  * log só em `stderr`; entradas e saídas com teto e paginação por cursor;
  * conteúdo de reunião marcado como fala de terceiros (`content_origin: "third_party_speech"`);
  * qualquer tool de escrita futura exige ADR nova.
* Quando esta ADR for aceita, o `ARCHITECTURE.md` muda no mesmo commit que criar `crates/mcp`:
  * a frase "As chamadas de rede opcionais são três" vira uma lista de saídas de dados opcionais, com os três itens atuais mais "leitura do histórico por cliente MCP local, desligada por padrão" (se outra ADR do lote já tiver criado a lista, só se acrescenta o item);
  * o Code Map ganha `crates/mcp` (`fala-mcp`), e os invariantes ganham "o `fala-mcp` não depende de crate de rede";
  * "Dados do usuário" ganha o arquivo de consentimento e o log de auditoria, e "Logs" ganha a auditoria de acessos;
  * o script novo de CI entra nos Comandos do `AGENTS.md`.

## Confirmação
* Script de CI irmão de `scripts/check-no-tauri-in-crates.sh` falha se `cargo tree -p fala-mcp -e normal` contiver `hyper`, `reqwest`, `rustls`, `native-tls` ou `oauth2`.
* Teste de integração lança o binário, faz o handshake legado (`initialize`) e o novo (`server/discover`) e confere que cada linha do stdout é JSON-RPC válido.
* Teste grava um ditado sensível e confere que nenhuma tool, `resources/list` nem `resources/read` o devolve; `get_dictation` com o id dele responde igual a um id inexistente, e contagens e cursores não mudam com a presença dele.
* Teste confere que nenhuma resposta tem bloco `audio`, MIME `audio/*`, o caminho da pasta de dados ou as strings `audio/`, `.opus` e `.wav`.
* Teste: com o consentimento desligado, toda tool, `resources/list`, `resources/read` e `prompts/get` devolve `isError` ou lista vazia; uma variável de ambiente ou argumento do cliente não muda isso.
* Teste: o `fala.sqlite` não muda (`PRAGMA data_version` e hash) depois de uma sessão de chamadas, e cada chamada gera uma linha de auditoria sem texto do ditado.
* Junto com as tools de reunião (fase 2): com reuniões desligadas, as tools e resources de reunião devolvem `isError` ou lista vazia, e uma sessão "só local" não aparece em nenhuma delas.
