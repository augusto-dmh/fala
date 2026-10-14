# dictionary-import — checks

Fase 4 (polimento do dicionário): importar termos e pares falado → escrito de um `.txt`/`.csv`,
com prévia, como o Wispr. Só frontend: o seletor do `plugin-dialog` põe o arquivo escolhido no
escopo do `plugin-fs` (`fs:read-files` já está na capability), e o dicionário já grava a lista
inteira de cada tipo (`update_custom_words`, `update_dictionary_replacements`), então não há
comando novo.

| ID | Check | Proof |
| --- | --- | --- |
| I1 | o parser lê uma entrada por linha: termo; par com `,`, `;` ou tab (o separador do arquivo é o que mais linhas usam); aspas CSV simples (`"a, b",c` e `""`); BOM; linhas vazias e `#` de fora; cabeçalho óbvio de fora; linha com mais de dois campos, aspas sem fechar, campo vazio ou acima do limite do tipo é inválida, com o número da linha | `bun src/components/dictionaryImport.test.tsx` I1 |
| I2 | duplicados contam sem diferenciar maiúsculas, contra o dicionário e dentro do arquivo; mais de 5.000 linhas de entrada ou um campo com mais de 200 caracteres dão erro e nada a importar; gravar acrescenta só os novos ao fim de cada lista e não muda o que existe | `dictionaryImport.test.tsx` I2 |
| I3 | a prévia mostra as contagens (novos termos, novos pares, duplicados, inválidas), as 5 primeiras inválidas com linha e motivo, as primeiras entradas, e [Importar N] desligado sem nada novo ou com erro; Cancelar só fecha; Importar grava e mostra o toast com a contagem | `dictionaryImport.test.tsx` I3 |
| I4 | a lista do Dicionário tem "Importar…" ao lado de "Adicionar"; o seletor filtra `.txt`/`.csv`/`.tsv`; textos em pt e en | `dictionaryImport.test.tsx` I4, `bun run check:translations` |
| I5 | portões | `bun run lint`, `bun run format:check`, `bunx tsc --noEmit`, `bun run check:translations`, testes de `src/components` |
