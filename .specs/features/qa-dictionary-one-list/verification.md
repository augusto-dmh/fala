# qa-dictionary-one-list — verification

Máquina: Windows 11 (Alienware 16), 2026-10-13. Verificado pelo próprio executor.

**PASS** em D1–D5: `dictionaryList.test.tsx` D1–D4 ok; `dictionaryFocus.test.ts` F3 reescrito para
a lista ok; `shell.test.tsx` C29 ok; lint sem aviso, prettier limpo, tsc sem erro, traduções
completas; todos os testes de `src/` passam, menos `src/styles/tokens.test.ts`, que falha igual em
`origin/main` no Windows (caminho de URL).

Não provado no app: abrir o Dicionário com termos, pares e snippets: uma lista; filtrar por tipo;
"Adicionar" abre o formulário, Enter adiciona e deixa o foco no campo, Esc fecha; remover pelo
teclado.
