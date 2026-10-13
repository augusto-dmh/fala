# qa-dictionary-focus — checks

Achado Q7 do QA de 2026-10-13: depois de adicionar um termo com Enter o foco vai para `body`, e
ao remover um item salta para a busca. A causa: os campos ficam desabilitados enquanto o setting
grava. Diff de uma frase; sem plano.

| ID | Check | Proof |
| --- | --- | --- |
| F1 | o foco pedido espera a gravação acabar e só vai ao alvo se o foco se perdeu (`body`); se a pessoa foi a outro campo, fica lá | `bun src/components/settings/dictionaryFocus.test.ts` F1 |
| F2 | ao remover, o alvo é o item seguinte, senão o anterior, senão o campo; o botão é achado pelo `data-entry` | `dictionaryFocus.test.ts` F2 |
| F3 | termos, substituições e snippets: campos habilitados durante a gravação, adicionar espera a gravação, o campo falado recebe o foco depois de adicionar um par, a remoção pelo teclado leva ao vizinho (com o campo de reserva) e o clique não mexe no foco; `Input` repassa o `ref` | `dictionaryFocus.test.ts` F3 |
| F4 | portões | `bun run lint`, `bunx prettier --check .`, `bunx tsc --noEmit`, os testes de `src/components/settings/`, `controls.test.tsx`, `shell.test.tsx` |
