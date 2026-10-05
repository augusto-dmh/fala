# meeting-notes

## Problem

Uma transcrição de reunião é uma parede de texto: uma hora de call vira ~10 mil palavras sem estrutura, e o que a pessoa digitou durante a call (os gatilhos curtos do design doc §3.4) fica solto ao lado dela. Hoje o Fala não tem nenhum código que transforme transcrição e anotações em notas; o design doc §3.4 item 4 e o pitch da fase 2 (F6) pedem notas em Markdown com o texto humano distinguível do gerado e cada linha gerada apontando para o trecho de origem, e a ADR-0016 (proposta) fixa o que pode sair da máquina para isso. Sem esse código, a pessoa relê a transcrição inteira para achar uma decisão, e não há como conferir de onde veio uma frase que um LLM escreveu. O pitch não traz número de uso; a evidência é a do Granola (`research/15` §1.3: notas cruas em preto, geradas em cinza, lupa por linha para a fonte).

Quando isto entra, um chamador (a 2.F5 e o `fala-cli`, numa rodada depois) entrega a transcrição com ids, as anotações, título, data, idioma, dicionário e um template ao crate `fala-notes`, e recebe um Markdown com as anotações como bloco humano, as notas geradas marcadas linha a linha e um ponteiro por fonte; numa sessão "só local", recebe só as anotações e nada sai da máquina.

## Flow

Reusa os tipos `Language` e `Dictionary` de `fala-core` e o mesmo cliente HTTP (`ureq` 3, já no `Cargo.lock`) e o mesmo padrão de servidor falso que a trilha B usou para o Gemini; não depende de `fala-asr`, `fala-meeting` nem `fala-secrets`, que ainda não integraram.

1. entra `NotesInput` (segmentos com id, anotações, título, data, hora, idioma, dicionário, template, marca "só local") -> `fala-notes` (door 6, crate novo) - valida a entrada
2. `fala-notes` - se a sessão é "só local", devolve as anotações renderizadas e para aqui, sem tocar no LLM nem no provedor de chave
3. `fala-notes` - `NotesPayload::build` monta o payload enumerado (door 1), o único construtor
4. `fala-notes` - `NotesLlm::generate(&NotesPayload)` (door 3); o `Claude` lê a chave do `KeySource` injetado (door 4) e faz um `POST /v1/messages` (door 2)
5. `fala-notes` - lê a resposta estruturada, descarta ids que não existem na entrada e renderiza o Markdown (door 5)
6. out: `Notes { markdown, generated, dropped_sources, unsourced_lines }`; nada persiste aqui (a 2.F5 grava)

## Impact

| Front | What changes |
| --- | --- |
| domain | new term: `NotesInput` / `Segment` - a entrada do crate; `Segment` tem a forma da door 4 da 2.F4 (`channel`, `speaker`, `t0_ms`, `t1_ms`, `text`) mais `id`, lives in `fala-notes` |
| domain | new term: `Template` - template de notas (propósito, estilo, seções com instrução), lives in `fala-notes` |
| domain | new term: `NotesPayload` - o que sai para o LLM de notas, exatamente a lista da ADR-0016 que este crate recebe, lives in `fala-notes` |
| domain | new term: `NotesLlm` / `Claude` / `KeySource` - o trait do LLM, o cliente da Anthropic e o provedor de chave injetado, lives in `fala-notes` |
| domain | new term: `Notes` - o Markdown resultante e as contagens, lives in `fala-notes` |
| workspace | `Cargo.toml` ganha `fala-notes = { path = "crates/notes" }` em `[workspace.dependencies]` (aditivo); `ARCHITECTURE.md` ganha a linha `crates/notes` no Code Map |
| stored data | nothing to migrate - nada persiste aqui; o formato do Markdown (door 5) e o schema do template (door 7) viram dado do usuário quando a 2.F5 gravar |

## Relations

None - no stored-data shape change: o crate não persiste nada; as formas que viram dado do usuário (Markdown e template) estão no Landing.

## Surface

| Route | In | Out | Status |
| --- | --- | --- | --- |
| `POST {base_url}/v1/messages` (saída para a Anthropic) | headers `x-api-key`, `anthropic-version: 2023-06-01`, `content-type`; corpo `model`, `max_tokens`, `system`, `messages`, `output_config` | `content[].text` com o JSON de seções, `stop_reason` | `200` + `end_turn` -> notas, `200` + `max_tokens` -> `Truncated`, `200` + `refusal` -> `Refused`, `200` com corpo ilegível -> `InvalidResponse`, `4xx`/`5xx` -> `Http(status)`, timeout -> `Timeout`, conexão recusada -> `Network` |

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. payload enviado (ADR-0016) | `{"title","started_at","language","dictionary","template":{"name","purpose","style","sections":[{"title","instruction"}]},"annotations":[{"id":"a1","text"}],"transcript":[{"id":"s12","t0_ms","t1_ms","speaker","text"}]}`, serializado como o texto da única mensagem `user`; `speaker` é o rótulo ("Eu"/"Me", "Pessoa N"/"Person N" ou o nome aplicado); `channel` da entrada nunca sai | mandar o `Segment` inteiro: leva `channel`, que não está na lista da ADR-0016; payload em texto livre: não dá para provar o conjunto de campos por teste |
| 2. corpo da requisição à Anthropic | chaves de topo exatamente `model`, `max_tokens`, `system`, `messages`, `output_config` (`{"format":{"type":"json_schema","schema":...}}`); `system` é a constante `SYSTEM_PROMPT`, sem dado da sessão; sem `tools`, sem `thinking`, sem `metadata` | prefill do assistente para forçar JSON: os modelos Claude 4.6+ devolvem 400; tool use para extrair JSON: a ADR-0016 proíbe `tools` na requisição |
| 3. trait do LLM | `pub trait NotesLlm { fn generate(&self, payload: &NotesPayload) -> Result<String, NotesError>; }` | trait recebendo `&NotesInput`: um backend novo poderia mandar `channel` ou o que mais a entrada ganhar, e o teste do payload não o cobriria |
| 4. provedor de chave | `pub trait KeySource: Send + Sync { fn api_key(&self, provider: &str) -> Result<Option<ApiKey>, NotesError>; }`, provedor `"anthropic"`; `ApiKey` sem `Display`/`Serialize`, `Debug` redigido; lida só dentro de `Claude::generate` | depender de `fala-secrets`: ainda não integrou (fronteira desta rodada); um adaptador de 5 linhas `impl KeySource for` o `SecretStore` dele entra na ligação |
| 5. Markdown das notas | anotações: `## Anotações` (en: `## Notes`), cada linha não vazia como parágrafo próprio terminado em ` ^aN`; notas geradas: `## Notas · <nome do template>` (en: `## AI notes · ...`), `### <título da seção>`, e cada linha gerada `- <texto> [[#^s12\|01:05]] [[#^a2\|a2]] <!-- fala:ia -->`; transcrição (`render_transcript`): `- **[01:05] Pessoa 1:** texto ^s12` | marcador por bloco: uma linha humana dentro do bloco gerado ficaria indistinguível; link Markdown `[01:05](#^s12)`: o suporte do Obsidian a bloco em link Markdown não está documentado, o wikilink está; âncora por `t0` (`^t65000`): dois segmentos no mesmo milissegundo nos dois canais colidem |
| 6. crate novo | `crates/notes`, pacote `fala-notes`; deps `fala-core`, `serde`, `serde_json`, `thiserror` (workspace), `ureq = "3"` e `log = "0.4"` diretos no crate, como a trilha B fez com `log` | módulo dentro de `crates/meeting` (o pitch): o crate `meeting` não existe neste ramo; um crate próprio mantém o cliente HTTP das notas fora da árvore de `fala-storage` (ADR-0011, proposta) |
| 7. schema do template | JSON `{"schema":1,"id","name","purpose","style","sections":[{"title","instruction"}]}`, `deny_unknown_fields`, ≥ 1 seção; dois arquivos versionados `crates/notes/templates/geral.json` e `crates/notes/templates/um-a-um.json`, embutidos com `include_str!` | campo único de prompt livre: perde a estrutura do Granola (propósito, estilo, seções com instrução) que a revisão do pitch fixou; sem `schema`: um campo novo depois não teria como migrar os templates exportados |

- Nothing else in this change is hard to reverse

## Criteria

### S1: entrada, payload e templates (P1)

O que sai para o LLM é exatamente a lista enumerada, montada num lugar só.

**Acceptance Criteria**

1. WHEN `NotesPayload::build` recebe um `NotesInput` válido THEN `fala-notes` SHALL produzir um JSON cujas chaves de topo são exatamente `title`, `started_at`, `language`, `dictionary`, `template`, `annotations` e `transcript`, cujas chaves de `template` são exatamente `name`, `purpose`, `style`, `sections`, de cada seção exatamente `title` e `instruction`, de cada anotação exatamente `id` e `text`, e de cada segmento exatamente `id`, `t0_ms`, `t1_ms`, `speaker` e `text`
2. The `NotesPayload` SHALL nunca conter o `channel` dos segmentos: um `NotesInput` cujo único texto em `channel` é distinguível não aparece em nenhum valor do JSON
3. WHEN as anotações são `"decidir data\n\n  \nAna: contrato"` THEN o payload SHALL listar `[{"id":"a1","text":"decidir data"},{"id":"a2","text":"Ana: contrato"}]` (linhas em branco não recebem id)
4. WHEN um segmento tem `speaker` `me`, `{"person":2}` ou `{"name":"Ana"}` e o idioma é `pt-BR` THEN o payload SHALL rotular `"Eu"`, `"Pessoa 2"` e `"Ana"`; com idioma `en`, `"Me"`, `"Person 2"` e `"Ana"`
5. WHEN um `Segment` é desserializado de `{"id":3,"channel":"system","speaker":{"person":1},"t0_ms":65000,"t1_ms":70000,"text":"oi"}` THEN `fala-notes` SHALL aceitá-lo, e `{"channel":"mic","speaker":"me",...}` também (forma da door 4 da 2.F4 mais `id`)
6. IF dois segmentos têm o mesmo `id` THEN `fala-notes` SHALL devolver `NotesError::InvalidInput` antes de qualquer chamada ao LLM
7. IF um segmento tem `t1_ms < t0_ms`, ou a data não está em `AAAA-MM-DD`, ou a hora não está em `HH:MM` THEN `fala-notes` SHALL devolver `NotesError::InvalidInput`
8. IF a transcrição e as anotações estão ambas vazias THEN `fala-notes` SHALL devolver `NotesError::EmptySession` sem chamar o LLM
9. The `fala-notes` SHALL trazer dois templates embutidos, `geral` (seções `Resumo`, `Decisões`, `Próximos passos` com dono e prazo) e `um-a-um`, que desserializam pelo schema da door 7
10. IF um template JSON tem um campo desconhecido, `schema` diferente de 1 ou nenhuma seção THEN `Template::from_json` SHALL devolver `NotesError::InvalidTemplate`

**Independent test:** `cargo test -p fala-notes payload` e `cargo test -p fala-notes template`

### S2: chamada ao Claude e "só local" (P1)

A requisição sai só quando deve, com a chave do usuário, e nada sai numa sessão "só local".

**Acceptance Criteria**

11. WHEN `generate_notes` roda numa sessão que não é "só local" THEN o `Claude` SHALL fazer um único `POST /v1/messages` com `x-api-key` igual à chave do `KeySource`, `anthropic-version: 2023-06-01`, corpo com chaves de topo exatamente `model`, `max_tokens`, `system`, `messages`, `output_config`, sem `tools`, `system` igual a `SYSTEM_PROMPT`, e o texto da única mensagem `user` igual ao JSON do `NotesPayload`
12. The `Claude` SHALL usar o modelo `claude-sonnet-5` por padrão, trocável por `with_model`
13. WHILE a sessão está marcada "só local" `generate_notes` SHALL não abrir conexão com o servidor (servidor falso com 0 conexões aceitas) e não consultar o `KeySource` (0 chamadas), e SHALL devolver `Notes` com `generated = false` e o Markdown só com as anotações
14. WHEN a mesma entrada tem a marca "só local" retirada THEN a geração seguinte SHALL chegar ao servidor falso (1 requisição)
15. IF o `KeySource` devolve `None` para `"anthropic"` THEN `generate_notes` SHALL devolver `NotesError::MissingKey` sem abrir conexão
16. IF o servidor responde `4xx`/`5xx` THEN `generate_notes` SHALL devolver `NotesError::Http(status)`, e a mensagem do erro SHALL não conter o corpo da resposta
17. IF o servidor não responde dentro do timeout configurado THEN `generate_notes` SHALL devolver `NotesError::Timeout`; IF a conexão é recusada THEN `NotesError::Network`
18. IF a resposta `200` tem `stop_reason` `refusal`, `stop_reason` `max_tokens` ou um texto que não é o JSON de seções THEN `generate_notes` SHALL devolver, respectivamente, `NotesError::Refused`, `NotesError::Truncated` e `NotesError::InvalidResponse`

**Independent test:** `cargo test -p fala-notes --test fake_claude`

### S3: Markdown humano × gerado com ponteiros (P1)

O Markdown separa o que a pessoa escreveu do que o LLM escreveu, e cada linha gerada aponta para a fonte.

**Acceptance Criteria**

19. WHEN as notas são geradas THEN o Markdown SHALL ter `## Anotações` com cada anotação, texto intacto, seguida de ` ^aN`, antes de `## Notas · <nome do template>`
20. The `fala-notes` SHALL terminar cada linha gerada com ` <!-- fala:ia -->` e nenhuma linha do bloco de anotações com esse marcador
21. WHEN uma linha gerada cita `s12` (segmento existente com `t0_ms` 65000) e `a2` (anotação existente) THEN ela SHALL conter `[[#^s12|01:05]]` e `[[#^a2|a2]]`, nessa ordem; `t0_ms` ≥ 1 h SHALL formatar como `1:02:03`
22. IF uma linha gerada cita um id que não existe na entrada (`s999`, `a9`, `x1`) THEN `fala-notes` SHALL descartar esse ponteiro, manter a linha e somar 1 a `dropped_sources` por id descartado
23. IF uma linha gerada fica sem nenhuma fonte válida THEN `fala-notes` SHALL mantê-la com o marcador e sem ponteiro, e somar 1 a `unsourced_lines`
24. WHEN a resposta traz as seções fora de ordem, uma seção que o template não tem, ou uma seção sem linhas THEN o Markdown SHALL seguir a ordem do template, omitir a seção estranha e omitir o título da seção vazia
25. The `fala-notes` SHALL colapsar quebras de linha do texto gerado em espaço e remover dele o marcador `<!-- fala:ia -->`, para que uma linha gerada seja sempre uma linha do Markdown
26. WHEN `render_transcript` recebe os segmentos THEN cada um SHALL virar `- **[mm:ss] <rótulo>:** <texto> ^s<id>`, para que todo ponteiro `[[#^sN|...]]` gerado tenha a âncora correspondente

**Independent test:** `cargo test -p fala-notes render`

## Out of scope

| Excluded | Why |
| --- | --- |
| `fala-cli meet notes <sessão>` | `apps/cli` está fora da fronteira desta rodada; entra com a 2.F5, de onde a sessão é lida |
| gravar as notas e o espelho `.md` da sessão | é da 2.F5 (storage, D7); este crate só produz o Markdown |
| perfil do usuário, nomes dos participantes, evento do calendário no payload | estão na lista da ADR-0016, mas não há `Settings.profile`, participantes nem calendário neste ramo; entram por adição ao `NotesPayload` |
| nomes de falantes sugeridos (D13) | depende dos participantes; terceiro item da ordem de corte do pitch |
| confirmar antes de regenerar uma nota editada, seletor de template | são de UI (F2) |
| importar e exportar templates do usuário, templates da intranet | D10: prompts da empresa nunca no repo; a importação usa o mesmo `Template::from_json` depois |
| aviso de dados de terceiros nomeando o provedor no primeiro envio | é de UI; a ADR-0016 o exige na ligação |
| retry automático e fila offline | regenerar é ação do usuário; offline as notas esperam (ADR-0016) |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| modelo padrão | `claude-sonnet-5`, configurável por `with_model` | design doc §3.4 e pitch F6 dizem "Sonnet 5"; a ADR-0016 trata provedor e modelo como configuração. Sonnet 5.5 (`claude-sonnet-5-5`) tem o mesmo preço e fica a uma linha de config | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| `max_tokens` e timeout | 16 000 tokens, 300 s, sem streaming | o skill da API recomenda ~16 000 em chamada sem streaming; uma nota de 1 h cabe com folga e o timeout cobre a geração | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| ADR-0016 ainda `proposed` | implementar contra ela | é a única regra escrita para este canal e o pedido da rodada a nomeia; se mudar, só o `NotesPayload` muda | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| idioma dos rótulos do Markdown | `pt-BR` e `en` pelo idioma das notas; títulos das seções vêm do template como escritos | o Markdown é dado do usuário, não UI do i18next; o template é editável | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| linha gerada sem fonte válida | mantida, marcada, sem ponteiro, contada | descartar perde conteúdo sem aviso (bug bar); a contagem deixa a UI mostrar "sem fonte" | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| `Speaker::Named` | variante `{"name":"Ana"}` além de `me` e `{"person":N}` | a ADR-0016 manda o nome aplicado pelo usuário; a 2.F4 só define `me`/`person` | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| ids no payload | `s<id>` para segmento (id do chamador, `u32`) e `a<n>` para anotação (posição entre as linhas não vazias, a partir de 1) | ids curtos e distintos por prefixo deixam o LLM citar os dois alvos (pitch: segmento ou anotação) | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |

**Open questions:** none - all resolved or logged above.

## Observable

| Surface | Decision | Landing |
| --- | --- | --- |
| API `POST /v1/messages` (cliente) | response shape | AC 11, AC 18 |
| API `POST /v1/messages` (cliente) | error shape and codes | AC 16, AC 17, AC 18 |
| API `POST /v1/messages` (cliente) | who may call it | AC 11, AC 15 (chave do usuário, ADR-0008) |
| API `POST /v1/messages` (cliente) | versioning | AC 11 (`anthropic-version: 2023-06-01`) |
| API `POST /v1/messages` (cliente) | rate limit | AC 16 (`429` vira `Http(429)`; sem retry, ver Out of scope) |
| document: Markdown das notas | structure | AC 19, AC 24, AC 26 |
| document: Markdown das notas | tone, depth | n/a - vêm do template e do LLM, editáveis pelo usuário |
| document: Markdown das notas | what the reader does next | AC 21 (seguir o ponteiro até o trecho) |
| collection: seções da nota | grouping, ordering, duplicates, exception | AC 24 |

## Sources

- `docs/decisions/0016-llm-de-notas-de-reuniao-payload-enumerado.md` (ramo `docs/propose-adrs-0010-0016`, proposta) - o payload enumerado e "só local" sem LLM
- `docs/design/2026-10-fala-v1.md` §3.4 item 4 e `fala-research/pitches/fase-2-reuniao-videos.md` F6, D7, D10 - Claude Sonnet 5, humano × gerado, ponteiro por linha, dois templates genéricos
