# qa-dictionary-focus — verification

Máquina: Windows 11 (Alienware 16), 2026-10-13. Verificado pelo próprio executor.

## Verdict

**PASS** em F1–F4. O app rodando não foi aberto (ver "Não provado").

| ID | Result | Evidence |
| --- | --- | --- |
| F1 | PASS | F1 ok: ocupado mantém pendente; foco na busca não é roubado; foco no `body` ou nulo vai ao alvo; alvo nulo não quebra |
| F2 | PASS | F2 ok: meio → seguinte, fim → anterior, único → campo; `entryButton` acha por `data-entry` com aspas no texto |
| F3 | PASS | F3 ok |
| F4 | PASS | lint sem aviso; prettier limpo; tsc sem erro; `appStyles`, `cleanupLevel`, `dictionaryFocus`, `controls` e `shell` passam |

## Não provado

Roteiro no app: no Dicionário, digitar um termo, Enter, digitar outro logo em seguida, Enter (os
dois entram sem o mouse e sem perder letras); Tab até o × de um termo do meio e Enter (o foco vai
ao termo seguinte); o mesmo numa substituição; clicar num × com o mouse não move o foco.
