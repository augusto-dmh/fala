# qa-times-one-line — verification

Máquina: Windows 11 (Alienware 16), 2026-10-13. Verificado pelo próprio executor.

## Verdict

**PASS** nos checks automáticos (T1–T5). A olhada no app rodando ficou para o Augusto (ver
"Não provado").

## Checks

| ID | Result | Evidence |
| --- | --- | --- |
| T1 | PASS | `historyModel.test.ts` T1 (`3:50 PM` em en-US, `15:50` e `07:05` em pt-BR, coluna `w-16 whitespace-nowrap`) e H2; `home.test.tsx` T4 (`w-16 shrink-0 whitespace-nowrap`, sem `w-12`) |
| T2 | PASS | `historyModel.test.ts` T2 (`winword` → Word, `msedge` → Edge, `Code` → VS Code, `houston` → Houston) |
| T3 | PASS | `historyModel.test.ts` T3 (`emptyHold`/`emptyToggle` com `{{keys}}` em pt e en, cabeçalho `h2` no estilo de Reuniões, sem `settings.history.title` nem `uppercase`) |
| T4 | PASS | `home.test.tsx` T4 (notas que falharam: ponto `bg-warn` e "failedWhy.notes"; transcrição que falhou numa reunião já transcrita: `bg-danger` e "failedWhy.transcript"; a página passa `notesFailed`) |
| T5 | PASS | `bun run lint` sem erro; `bunx prettier --check .` ok; `bunx tsc --noEmit` sem erro; `bun run check:translations` ok; os 15 arquivos `src/components/meeting/*.test.tsx` e `historyModel.test.ts` passam |

## Não provado

O app de debug não foi aberto. Roteiro: com a UI em en, um ditado à tarde mostra "3:50 PM" numa
linha no Início; a reunião com notas que falharam por falta de chave mostra "Notes didn't finish"
com ponto âmbar.
