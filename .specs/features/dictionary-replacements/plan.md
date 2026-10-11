# dictionary-replacements (fase 4: "falado → escrito" no dicionário)

## Problem

O dicionário pessoal é uma lista de termos (`custom_words`): as regras locais corrigem a grafia de um termo dito com outra caixa, acento ou hífen, a fuzzy herdada aproxima palavras parecidas e o Gemini recebe os termos no prompt (#61). Não há como dizer "quando eu disser X, escreva Y". Um nome que o Parakeet erra sempre do mesmo jeito ("fala sql" para `Fala.sqlite`), uma abreviação ("btw" para "by the way") ou um nome de produto com pontuação só se conserta hoje se o LLM estiver ligado e acertar, ou se a fuzzy chegar perto. O Wispr tem isso como "Correct a misspelling" (`phrase` → `replacement`, com `frequencyUsed`).

Quando isto entra, o dicionário tem dois tipos de linha: o termo de sempre e o par "falado → escrito". A regra local troca o falado pelo escrito, palavra inteira, sem acento e sem caixa, antes do LLM e também com o LLM desligado; o escrito vai ao LLM como termo; cada par conta quantas vezes foi aplicado.

## Flow

Reusa `Dictionary` (`fala-core`), `Rules` e `match_key` (`fala-postproc`), `llm_auto::AutoFormatter` e `process_transcription_output` (desktop), `CustomWords.tsx` e `DictionaryPage.tsx` (UI).

1. a pessoa adiciona "fala sql → Fala.sqlite" na página Dicionário; `update_dictionary_replacements` normaliza e grava em `dictionary_replacements` (door 1), preservando a contagem dos pares que já existiam
2. a pessoa dita; `llm_auto::formatter` monta o `Dictionary` com `custom_words` e os pares (`Dictionary::new(..).with_replacements(..)`)
3. `Rules` roda fillers, repetições e pontuação falada como hoje; depois um passe único do motor de frases (new, `replace_phrases`): em cada posição tenta primeiro os pares (falado → escrito), depois os termos (grafia), sempre a sequência mais longa; o que entrou não é relido (door 2)
4. as regras devolvem o texto e os índices dos pares aplicados (`Rules::format_with_hits`); `Formatted.replaced` os leva adiante
5. se o LLM roda, recebe o texto já trocado e o escrito no bloco "Personal dictionary" (o falado nunca vai)
6. out: o desktop cola o texto e soma um uso por ocorrência a cada par aplicado (`dictionary::record_uses`), numa escrita do store
7. a página Dicionário mostra os termos e os pares com a contagem, com uma busca que filtra os dois

## Impact

| Front | What changes |
| --- | --- |
| domain | `fala_core::Dictionary` ganha `Replacement { spoken, written }` e `with_replacements`; `terms()` passa a incluir o escrito de cada par (é o que o LLM recebe) |
| domain | `fala-postproc`: o `apply_dictionary` vira o motor genérico `replace_phrases` (pares + termos num passe); `Rules::format_with_hits`; `Formatted.replaced` |
| stored data | `settings`: campo novo `dictionary_replacements: [{spoken, written, uses}]`, `#[serde(default)]`; `custom_words` não muda de nome nem de tipo, então o store antigo carrega igual |
| UI | página Dicionário: busca, a lista de termos de sempre e a lista "falado → escrito · N usos" |

## Relations

| Entity | Relation | Field | Note |
| --- | --- | --- | --- |
| `AppSettings.dictionary_replacements` | vira | `Dictionary.replacements` | `crate::dictionary::dictionary(settings)` a cada ditado |
| `Formatted.replaced` | conta em | `DictionaryReplacement.uses` | índice → `spoken` no desktop, comparado sem caixa na hora de gravar |

## Surface

| Surface | Change |
| --- | --- |
| comando Tauri `update_dictionary_replacements(replacements) -> Vec<DictionaryReplacement>` | new; devolve o que gravou; `bindings.ts` regenerado |
| `fala-cli format --dictionary` | linha `falado -> escrito` vira par; as outras continuam termos (ferramenta da prova sem UI) |

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. onde os pares moram no store | `settings.dictionary_replacements: [{"spoken": "fala sql", "written": "Fala.sqlite", "uses": 3}]`, ao lado de `custom_words`, que fica como está; o tipo e o comando vivem em `apps/desktop/src/dictionary.rs`, e `settings.rs` só ganha o campo | trocar `custom_words` por uma lista de entradas com `spoken` opcional (o "formato do custom_words" da definição): o Whisper (`initial_prompt`), a fuzzy, a UI e o `settingsStore.ts` leem `custom_words` como `string[]`, a migração não se desfaz, e outros painéis mexem em `settings.rs` agora; o `Dictionary` do core já tem a forma de entradas com `spoken` opcional, e é ele que as regras e o LLM usam |
| 2. ordem e prioridade | depois da pontuação falada; num passe único, por posição: pares antes de termos, mais longo primeiro; o texto que entrou não é relido; janela de até N palavras para um par de N palavras falado (até 3 para termo, como hoje); chave = `match_key` das palavras juntas, então "fala-sql" e "Fala SQL" batem com "fala sql" e "falasqlx" não | pares antes da pontuação falada: o escrito ("vírgula", "ponto") viraria sinal; dois passes (pares, depois termos): um termo reescreveria o escrito do par |
| 3. contagem | por ocorrência, só dos pares, gravada no próprio par no store, depois de colar; o comando de edição ignora o `uses` que vem da UI e mantém o do store pelo mesmo falado | contar termos: não há onde guardar sem mudar `custom_words`; tabela no `fala.sqlite`: é configuração, e os snippets vão para arquivo de config também |

- O motor (`replace_phrases`) recebe frases com chave, tamanho máximo e o texto que entra. Os snippets da fase 4 entram com o marcador opaco (`⟦S1⟧`) como texto e uma camada própria antes deste passe; nada aqui precisa mudar para isso.
- Um par cujo falado contém um gatilho de pontuação ("nova linha") não casa, porque a pontuação falada roda antes. Aceito: é raro (Out of scope).

## Criteria

### S1: o motor e o dicionário (P1)

1. WHEN o texto contém o falado de um par, com outra caixa, acento ou hífen, THEN as regras SHALL trocar pela grafia escrita, preservando a pontuação em volta
2. WHEN o falado aparece dentro de outra palavra ou atravessa pontuação THEN as regras SHALL deixar o texto como está
3. WHEN um par e um termo casam na mesma posição THEN o par SHALL vencer, e o escrito não SHALL ser relido pelos termos
4. WHEN o LLM está desligado THEN o texto final SHALL ter o escrito; WHEN ligado, o servidor falso SHALL receber o texto com o escrito e o escrito no bloco do dicionário, e nunca o falado
5. The `Dictionary` SHALL ignorar par vazio e falado repetido (sem caixa), e um JSON antigo `{"terms": [...]}` SHALL desserializar sem pares
6. The `Formatted.replaced` SHALL ter um índice por ocorrência aplicada

**Independent test:** `cargo test -p fala-core -p fala-postproc`

### S2: store, contagem e página (P1)

7. WHEN um store sem `dictionary_replacements` carrega THEN `custom_words` SHALL ficar igual e os pares vazios
8. WHEN a UI grava a lista THEN o desktop SHALL normalizar (trim, sem vazio, sem falado repetido) e manter o `uses` do store pelo falado
9. WHEN um ditado aplica pares THEN o desktop SHALL somar uma ocorrência a cada um
10. The página Dicionário SHALL mostrar termos e pares com a contagem e filtrar os dois pela busca, com strings em `pt` e `en`

**Independent test:** `cargo test -p fala --lib dictionary`; `bun run lint`; `bun run check:translations`

## Out of scope

| Excluded | Why |
| --- | --- |
| aprender sozinho pelas correções, dicionário compartilhado, estrela | definição |
| snippets | outra feature, mesmo motor |
| contagem para termos | sem campo no store sem mudar `custom_words` (door 3) |
| par na reunião (ElevenLabs keyterms, notas) | o ditado é o caso; o escrito já é termo se a pessoa o adicionar |
| falado com gatilho de pontuação falada | raro; a pontuação roda antes (door 2) |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| store separado vs. mudar `custom_words` | campo novo (door 1) | diff aditivo, sem migração que não volta, outros painéis em `settings.rs` | y — delegado ("siga a recomendação"), registrado no PR |
| prova manual com IA desligada | `fala-cli format` com um arquivo de dicionário com `falado -> escrito`, mais roteiro no PR para o app | ninguém dita hoje à noite | y — delegado |

**Open questions:** none.
