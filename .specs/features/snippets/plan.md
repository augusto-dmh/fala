# snippets (fase 4: frase dita → texto fixo)

## Problem

Quem dita repete as mesmas coisas várias vezes por semana: a assinatura, o link da agenda, o endereço, uma resposta-padrão. O dicionário do #108 troca uma forma falada curta pela escrita, mas o escrito vai ao LLM como termo e o LLM pode reescrevê-lo junto com o resto. Um texto longo e fixo (várias linhas, um link) não pode passar pelo LLM: ele o reescreveria, e o conteúdo sairia da máquina sem necessidade.

Quando isto entra, a página Dicionário ganha uma terceira lista, "Snippets": gatilho (duas palavras ou mais) → texto. Ditar o gatilho põe o texto, com `{data}`, `{data_extenso}` e `{hora}` resolvidos na hora. O LLM nunca vê o texto: recebe um marcador opaco (`⟦S1⟧`) no lugar, e o marcador volta a ser o texto depois. Se o ditado inteiro é o gatilho, o texto entra como está, sem LLM.

## Flow

Reusa `Dictionary` (`fala-core`), o motor `replace_phrases` e `match_key` (`fala-postproc`), `Postprocessor::process` e `LateEdit`, o armazenamento do #108 (`DictionaryReplacement`, `normalize`, `count_uses`) e o componente `DictionaryReplacements.tsx`.

1. a pessoa adiciona "minha assinatura → Augusto\nfala.dev" na página Dicionário; `update_dictionary_snippets` normaliza (gatilho com duas palavras ou mais, sem repetido) e grava em `dictionary_snippets` (door 1), mantendo `uses`
2. a pessoa dita; `crate::dictionary::dictionary` monta o `Dictionary` com termos, pares e snippets, com as variáveis do texto já resolvidas no instante do ditado (`fala_core::render_snippet`)
3. `Rules` roda como hoje; os gatilhos passam pelo motor de frases num passe próprio, antes da pontuação falada (um gatilho pode ter "ponto final"), e cada ocorrência vira `⟦Sn⟧` (n = ordem no texto); o passe dos pares e termos não relê o marcador (door 2)
4. o `Postprocessor` guarda o texto mascarado; o texto das regras é ele com cada marcador trocado pelo texto do snippet; se o mascarado é só um marcador (com pontuação do ASR), o texto final é o do snippet, exato, e o LLM não roda
5. se o LLM roda, recebe o mascarado e uma linha a mais no system prompt para manter os marcadores; a resposta só vale se cada marcador volta uma vez e nada mais com `⟦S` aparece; senão fica o texto das regras, `fallback = InvalidResponse`, e um `debug` (door 3). A resposta tardia (`LateEdit`) passa pela mesma reposição
6. out: o desktop cola o texto e soma um uso por ocorrência a cada snippet aplicado, junto com os pares

## Impact

| Front | What changes |
| --- | --- |
| domain | `fala_core::Snippet { trigger, expansion }`, `Dictionary::with_snippets`/`snippets()` (fora de `terms()` e do JSON), `render_snippet(template, Moment)` |
| domain | `fala-postproc`: grupo de snippets no motor, `Rules::mask`, reposição no `Postprocessor` e no `LateEdit`, `Formatted.snippets`, linha do prompt |
| stored data | `settings.dictionary_snippets: [{spoken, written, uses}]` (o tipo do #108), `#[serde(default)]` |
| UI | página Dicionário: lista "Snippets" com gatilho, texto em várias linhas, contagem; busca filtra as três |

## Surface

| Surface | Change |
| --- | --- |
| comando Tauri `update_dictionary_snippets(snippets) -> Vec<DictionaryReplacement>` | new; `bindings.ts` regenerado |
| `fala-cli format --dictionary` | linha `gatilho => texto` vira snippet (prova sem UI) |

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. onde os snippets moram | `settings.dictionary_snippets`, ao lado de `dictionary_replacements`, com o mesmo tipo `{spoken, written, uses}` (falado = gatilho, escrito = texto) | `snippets.toml` na pasta de dados (pitch): seria um segundo armazenamento de configuração, sem a contagem e a página do #108; a definição pede o mesmo motor e a mesma página |
| 2. ordem | gatilhos depois de fillers e repetições e antes da pontuação falada, dos pares e dos termos; mais longo primeiro; gatilho com ≥ 2 palavras; o marcador não é relido | gatilho de uma palavra: falso positivo (pitch); gatilhos no passe dos pares: "ponto final" num gatilho já teria virado sinal |
| 3. marcador | `⟦S1⟧`, `⟦S2⟧`… por ocorrência; resposta do LLM aceita só com cada marcador uma vez e nenhum outro `⟦S` | aceitar marcador faltando: o texto do snippet sumiria do que a pessoa ditou |

## Criteria

### S1: motor, LLM e loja (P1)

1. WHEN o ditado tem o gatilho (sem caixa/acento, palavra inteira) THEN o texto final SHALL ter o texto do snippet, com e sem LLM
2. WHEN o ditado inteiro é o gatilho THEN o texto final SHALL ser exatamente o texto do snippet, e nenhuma request SHALL sair
3. WHEN o LLM roda THEN o corpo da request SHALL ter o marcador e nunca o texto do snippet nem o gatilho no dicionário
4. WHEN o LLM devolve o marcador alterado, faltando ou repetido (resposta imediata ou tardia) THEN fica o texto das regras com o snippet
5. The `Snippet` SHALL exigir gatilho com duas palavras ou mais e texto não vazio; gatilho repetido (sem caixa) fica com o primeiro
6. The `{data}`, `{data_extenso}` e `{hora}` SHALL virar "02/10/2026", "2 de outubro de 2026" e "14:05"
7. WHEN a UI grava a lista THEN o desktop SHALL normalizar e manter `uses`; WHEN um ditado aplica snippets THEN soma um uso por ocorrência
8. The página Dicionário SHALL mostrar os snippets com a contagem, filtrar pela busca, e validar duas palavras, com strings em `pt` e `en`

**Independent test:** `cargo test -p fala-core -p fala-postproc`; `cargo test -p fala --lib dictionary`; `bun run lint`

## Out of scope

| Excluded | Why |
| --- | --- |
| "snippet aplicado" com desfazer no histórico | o desfazer existe só para a edição da IA e precisa de um campo novo no registro do ditado (`fala-storage`); fatia seguinte |
| `fala-cli snippet add|list|rm` | os snippets moram no store do app, que o CLI não edita; `format --dictionary` cobre a prova |
| rich text, variáveis com lógica, por app, compartilhados | no-gos do pitch |
| snippet no atalho de pós-processamento herdado (Legacy) | esse caminho não roda regra nenhuma, nem os pares do #108; a expansão também não vai a ele |
| gatilho partido entre dois blocos de um ditado longo (#111) | cada bloco é formatado sozinho |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| store vs `snippets.toml` | store (door 1) | consistência com o #108, pedida no brief | y — brief |
| gatilho | só a frase, ≥ 2 palavras | brief | y — brief |
| formato de `{data}` | dd/mm/aaaa sempre; `{data_extenso}` em pt-BR | pitch | y — pitch |

**Open questions:** none.
