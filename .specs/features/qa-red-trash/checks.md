# qa-red-trash — checks

R7, R8, R16, R17 e R18 da segunda passada de QA (`research/27-reqa-fala.md`). O resto do Q23 (a
folha termina ~15 px antes da janela) fica de fora: a folha está em `MeetingView.tsx`, que o
painel `meeting-session-edit` está mudando, e a causa não se reproduz sem o app.

| ID | Achado | Check | Proof |
| --- | --- | --- | --- |
| T1 | R8 | "Mover para a lixeira" separado e em vermelho no menu ⋯ | `bun src/components/meeting/trashReach.test.tsx` T1 |
| T2 | R18 | a lixeira num botão fixo na linha das ações, antes de "Importar", com a contagem; o link do pé sai | `trashReach.test.tsx` T2 |
| T3 | R16 | o resultado de uma importação do Granola que terminou bem sai quando uma reunião abre (a prévia, o erro e o cartão de outro arquivo ficam); as datas da prévia no idioma da UI, com um helper só e validado; "Linha N" com maiúscula | `trashReach.test.tsx` T3, `granola.test.tsx` (prévia) |
| T4 | R7, R17 | "apagar para sempre" diz "apagadas deste computador"; sem "âncoras" | `trashReach.test.tsx` T4 |
| T5 | — | portões | `bun run lint`, `bunx prettier --check .`, `bunx tsc --noEmit`, `bun run check:translations`, os testes de Reuniões |
