# qa-times-one-line — checks

Achados Q2, Q3, Q5, Q6 e Q24 do QA de 2026-10-13. O diff cabe numa frase (a hora numa linha e
as linhas do Início e de Reuniões dizem o que são), então não há plano.

| ID | Achado | Check | Proof |
| --- | --- | --- | --- |
| T1 | Q2 | a hora não tem zero à esquerda no relógio de 12 h ("3:50 PM"), tem dois dígitos no de 24 h ("07:05") e a coluna tem `w-16 whitespace-nowrap` no Início e em Reuniões | `bun src/components/settings/history/historyModel.test.ts` (T1, H2) e `bun src/components/meeting/home.test.tsx` (T4) |
| T2 | Q3 | o app de destino sai pelo nome conhecido (`winword` → Word) ou capitalizado (`houston` → Houston) | `historyModel.test.ts` T2 |
| T3 | Q5, Q6 | o Início vazio ensina o atalho atual (segurar ou apertar); o cabeçalho do dia usa o estilo de Reuniões; o rótulo "HISTORY" sai | `historyModel.test.ts` T3 |
| T4 | Q24 | uma reunião com falha diz em texto visível o que falta ("Sem transcrição" ou "As notas não saíram"); só a falta de transcrição fica com o ponto vermelho | `home.test.tsx` T4 |
| T5 | — | portões | `bun run lint`, `bun run format:check`, `bunx tsc --noEmit`, `bun run check:translations`, os testes de UI tocados |
