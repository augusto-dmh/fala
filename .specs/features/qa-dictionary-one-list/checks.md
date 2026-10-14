# qa-dictionary-one-list — checks

Achado Q8 do QA de 2026-10-13 (e o Q42 dos termos: os chips num card solto): o Dicionário tinha
três formas de lista e três formulários sempre abertos. Agora é uma lista só, com filtro por tipo
e um "Adicionar" que abre o formulário do tipo escolhido, como o Wispr.

| ID | Check | Proof |
| --- | --- | --- |
| D1 | termos, pares e snippets numa lista, na ordem de cada lista; contagem por tipo; filtro por tipo e pela busca (sem acento, no falado e no escrito) | `bun src/components/dictionaryList.test.tsx` D1 |
| D2 | o formulário aplica as regras de cada tipo: termo normalizado e até 50, par com escrito até 200, snippet com duas palavras e até 4000, falado até 60, repetido recusado dentro do mesmo tipo | `dictionaryList.test.tsx` D2 |
| D3 | remover pelo teclado leva o foco ao vizinho, senão ao "Adicionar" (ou ao campo, com o formulário aberto); os campos ficam habilitados durante a gravação e o foco volta ao primeiro campo depois de adicionar; Enter só adiciona a partir de um campo; trocar de tipo limpa o segundo campo; o repetido diz em texto por que "Adicionar" está desligado; os grupos de opção andam com as setas | `dictionaryList.test.tsx` D3, `dictionaryFocus.test.ts` F3 |
| D4 | a página usa a lista; vazio convida a adicionar; textos novos em pt e en; os componentes antigos saem | `dictionaryList.test.tsx` D4, `shell.test.tsx` C29 |
| D5 | portões | `bun run lint`, `bunx prettier --check .`, `bunx tsc --noEmit`, `bun run check:translations`, todos os testes de `src/` |
