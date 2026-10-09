# dictionary (F8: dicionário pessoal no prompt)

## Problem

Os nomes próprios que o Parakeet erra ("Agusto", "charge bee") só se corrigem se o dicionário pessoal chegar ao LLM e às regras locais. Hoje a lista `custom_words` (settings herdado do Handy, editada em "Avançado") só vira o `initial_prompt` do Whisper e uma correção fuzzy (`apply_custom_words`) que roda no `TranscriptionManager`, antes de tudo. O desktop passa `Dictionary::default()` ao `Postprocessor` (`llm_auto.rs:51`), então o Gemini nunca vê os termos e as regras de `fala-postproc` nunca aplicam a grafia.

A fuzzy também briga com o LLM: ela troca palavras por termos do dicionário antes do Gemini ler o texto, e o prompt manda o Gemini "usar exatamente a grafia dos termos". Uma troca errada da fuzzy (o mês "agosto" virando "Augusto") chega ao LLM como se a pessoa tivesse dito, e o LLM, que teria o contexto para acertar, a mantém.

Quando isto entra, a lista vira o `Dictionary` de todo ditado do `transcribe`: as regras locais aplicam a grafia sempre, o Gemini recebe os termos no prompt, e a fuzzy só toca texto que o LLM não escreveu.

## Flow

Reusa `Dictionary::new` (`fala-core`, normalização de `ba3b6df`), `Rules` e `Gemini::request_body` (`fala-postproc`, já usam o dicionário), `apply_custom_words` (desktop, exists) e o `llm_auto` da F7a.

1. a pessoa adiciona um termo em "Palavras personalizadas"; `update_custom_words` grava a lista normalizada por `Dictionary::new` (door 1)
2. a pessoa dita; o `TranscriptionManager` transcreve. Se o LLM está configurado (`llm_enabled` e chave `gemini`), ele não roda a fuzzy; senão, roda como hoje (door 2)
3. `process_transcription_output` monta o `AutoFormatter`: o `Postprocessor` da F7a, o `Dictionary` de `custom_words` e a fuzzy adiada (só quando o passo 2 a pulou)
4. `fala-postproc` aplica as regras com o dicionário e, se as quatro condições do LLM valem, manda o texto com o bloco "Dicionário pessoal:" no `systemInstruction`
5. o LLM não escreveu o texto (desligado, sem chave, app na lista, ≤ 15 palavras, timeout, erro): a fuzzy adiada roda sobre o texto do ASR e as regras rodam de novo sobre o resultado, a mesma ordem de antes da F8
6. out: o desktop cola o `final_text` e grava o histórico como na F7a
7. o atalho herdado (`Legacy`) recebe a fuzzy adiada antes do cliente OpenAI-compat, então para quem o usa nada muda

## Impact

| Front | What changes |
| --- | --- |
| domain | `custom_words` passa a ser o dicionário pessoal: vai às regras locais e ao prompt do Gemini, além do Whisper |
| domain | a fuzzy `apply_custom_words` sai do `TranscriptionManager` quando o LLM está configurado e roda no `AutoFormatter`, antes de uma segunda passada das regras, só no texto que o LLM não escreveu |
| stored data | `settings_store.json`: `custom_words` mantém nome e tipo; a próxima escrita grava a lista normalizada. Sem migração |
| UI | "Palavras personalizadas" vira "Dicionário pessoal"; a descrição diz que os termos vão ao modelo, às regras e à IA; duplicata por caixa é recusada |

## Relations

| Entity | Relation | Field | Note |
| --- | --- | --- | --- |
| `AppSettings.custom_words` | vira | `fala_core::Dictionary` | `Dictionary::new(&custom_words)` a cada ditado; nenhum campo novo |
| condição da fuzzy no `TranscriptionManager` | espelha | `AutoFormatter.fuzzy` | as duas leem `llm_auto::llm_configured(settings)`: uma pula, a outra adia |

## Surface

| Surface | Change |
| --- | --- |
| comando Tauri `update_custom_words(words)` | normaliza antes de gravar; assinatura igual, sem regenerar `bindings.ts` |
| UI: `CustomWords.tsx` | duplicata sem diferenciar caixa; textos novos de título e descrição em `pt` e `en` |

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. `custom_words` → `Dictionary` no store | o campo continua `"custom_words": ["Fala", "ChargeBee"]` em `settings`; `update_custom_words` grava `Dictionary::new(words).terms()` (trim, sem vazio, sem duplicata por caixa, fica a primeira grafia); o desktop monta o `Dictionary` com `Dictionary::new(&settings.custom_words)` em cada ditado, então um store antigo com `["Fala", "fala"]` já chega ao LLM como `["Fala"]` | renomear para `dictionary: {"terms": [...]}` com migração de schema: o Whisper, a fuzzy, a UI e o `settingsStore.ts` leem `custom_words`, o ganho é só o nome e a migração não se desfaz; normalizar no load: reescreveria o store de quem nunca abriu a tela, sem ganho, porque o `Dictionary::new` do uso já normaliza |
| 2. a fuzzy com o LLM ligado | `llm_configured = llm_enabled && chave gemini não vazia`. Configurado: o `TranscriptionManager` não roda `apply_custom_words`; o `AutoFormatter`, quando `llm_produced = false`, a roda sobre o texto do ASR e passa as regras de novo sobre o resultado, e o `Legacy` a roda antes do cliente OpenAI. Não configurado: tudo como hoje (fuzzy no `TranscriptionManager`, nada depois) | desligar a fuzzy de vez com o LLM configurado: ditados de até 15 palavras e apps da lista perderiam a correção fonética ("Agusto" → "Augusto"), que as regras não fazem (só grafia: caixa, acento, hífen, espaço); manter a fuzzy antes do LLM: a troca errada vira instrução para o LLM manter; olhar só `llm_enabled`: vem `true` por padrão, e quem não tem chave perderia a fuzzy sem LLM nenhum |

- Limite herdado, igual sem o LLM: o matcher fuzzy às vezes engole uma palavra de uma letra colada ao termo (`a charge bee` → `ChargeBee`, `o augusto` → `Augusto`). Acontece no `TranscriptionManager` desde antes da F8; a door 2 só garante que o texto que o LLM não escreveu sai igual ao do pipeline sem LLM. Mexer no matcher fica fora (Out of scope).
- A igualdade com o pipeline sem LLM vale para texto sem filler: sem o LLM, a fuzzy roda antes da remoção de fillers e da normalização do `TranscriptionManager`; no caminho adiado, depois delas. Um filler colado a um termo pode dar resultado diferente (achado 8 do Verifier, round 2; aceito).
- A door 2 é reversível (é uma condição), mas muda o texto colado; por isso está aqui.
- Whisper com o LLM configurado: hoje o Whisper recebe os termos no `initial_prompt` e pula a fuzzy. Com a door 2, a fuzzy adiada roda no texto que o LLM não escreveu também para ele. Aceito: o Whisper sai do catálogo na F9 (D8), e sobre um texto já transcrito com o prompt a fuzzy quase sempre não acha nada para trocar.

## Criteria

### S1: o dicionário chega ao LLM e às regras (P1)

**Acceptance Criteria**

1. WHEN `custom_words` tem termos e o LLM é pedido THEN `apps/desktop` SHALL mandar ao Gemini o `systemInstruction` igual a `SYSTEM_PROMPT` seguido de `\n\nDicionário pessoal:` e de uma linha `\n- <termo>` por termo normalizado, na ordem da lista
2. WHEN `custom_words` está vazio, ou só tem itens em branco THEN o `systemInstruction` SHALL ser exatamente `SYSTEM_PROMPT`
3. WHEN o LLM não é pedido (desligado, sem chave, app na lista ou ≤ 15 palavras depois das regras) THEN o texto colado SHALL trazer a grafia do dicionário nas variantes que as regras reconhecem (`a charge bee` → `A ChargeBee` pelas regras; com o LLM configurado, o texto do AC 5), sem nenhuma request HTTP
4. WHILE o LLM está configurado THEN o `TranscriptionManager` SHALL devolver o texto sem a fuzzy, e WHEN o LLM escreve o texto THEN o Gemini SHALL receber o texto sem a fuzzy e o texto colado SHALL ser o do Gemini
5. WHILE o LLM está configurado e o LLM não escreve o texto (≤ 15 palavras, app na lista, timeout ou erro) THEN `apps/desktop` SHALL aplicar a fuzzy sobre o texto do ASR e depois as regras (`Agusto` → `Augusto`), dando o mesmo texto que o pipeline sem o LLM configurado; nenhum termo SHALL sair em maiúsculas (achado do Verifier: a fuzzy depois das regras transformava `A ChargeBee` em `CHARGEBEE`)
6. WHILE o LLM não está configurado THEN a fuzzy SHALL rodar no `TranscriptionManager` como antes e não SHALL rodar de novo no `AutoFormatter`
7. WHILE o LLM está configurado THEN o atalho herdado (`Legacy`) SHALL receber o texto com a fuzzy, como antes da F8

**Independent test:** `cargo test -p fala --lib llm_auto` e `cargo test -p fala --lib managers::transcription`

### S2: a lista gravada e a UI (P2)

8. WHEN `update_custom_words` recebe `[" Fala ", "fala", "", "ChargeBee", "  "]` THEN `apps/desktop` SHALL gravar `["Fala", "ChargeBee"]`
9. WHEN a pessoa adiciona um termo que só difere em caixa de um existente THEN a UI SHALL recusá-lo com o toast `duplicate`
10. The tela SHALL ter título e descrição novos em `pt` (fonte) e `en`, sem literal fora do i18next

**Independent test:** `cargo test -p fala --lib llm_auto::tests::custom_words_are_normalized`, `bun run lint`, `bun run check:translations`

## Out of scope

| Excluded | Why |
| --- | --- |
| renomear a chave i18n `settings.advanced.customWords` ou mover a tela de "Avançado" | só cosmético; a chave muda de novo quando a UI de settings for refeita |
| dicionário no `fala-cli format` e nas notas de reunião | o pedido é o ditado do desktop; o CLI já aceita o `Dictionary` do crate |
| ajustar o `word_correction_threshold` ou o algoritmo da fuzzy | código herdado, fora do pedido |
| verificação visual da tela | `TODO(windows)`: manual em `bun run tauri dev` |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| door 1: manter `custom_words` | sem migração, normaliza na escrita e no uso | ver Landing | y — delegado (2026-10-09), decidido pelo executor |
| door 2: fuzzy adiada com o LLM configurado | ver Landing | a recomendação do plano ("a fuzzy herdada não deve brigar com o LLM") sem perder a correção fonética dos ditados curtos | y — delegado (2026-10-09), decidido pelo executor |
| a ordem da fuzzy adiada | fuzzy sobre o texto do ASR e uma segunda passada das regras, depois de saber que o LLM não escreveu | só depois das regras se sabe se o LLM foi pedido (o limiar conta as palavras depois delas); rodar a fuzzy sobre o texto das regras estragava a grafia que elas acertaram (`CHARGEBEE`, achado do Verifier) | y — delegado (2026-10-09), decidido pelo executor |

**Open questions:** none - all resolved or logged above.

## Observable

- Nenhum log novo com texto ou termo. O `debug` `post-processing: editor=<...>` da F7a continua.

## Sources

- `fala-research/plans/fase-1-delta-e-semanas-3-4.md` § F8 e "Fila de execução"
- `docs/decisions/0004-*` (o LLM recebe texto, app e dicionário)
- `.specs/features/postproc/plan.md` (o `Dictionary` do S0 e a duplicação aceita com a fuzzy do desktop)
