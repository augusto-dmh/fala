# dictation-stats — checks

Só frontend; não há Rust. Cada check nomeia a prova; o Verifier roda a prova e registra a saída
em `verification.md`.

| ID | AC | Check | Proof |
| --- | --- | --- | --- |
| S1 | 1 | rail com quatro destinos, Desempenho depois de Reuniões e antes do rodapé; App abre em Início e monta `<StatsPage />` | `bun src/components/shell.test.tsx` (C25, C26, C27) |
| S2 | 2 | período padrão 7, aceita só 7/30/90, `localStorage` que lança não quebra; a página chama `getDictationStats(days)` | `bun src/components/stats/stats.test.tsx` (S2) |
| S3 | 3 | `formatMs` em pt e en: 380, 999, 1000, 1234, 2350, null | `stats.test.tsx` (S3) |
| S4 | 4 | `budgetState` nos limites 700, 701, 1000, 1001 (e 1200/1201/2000/2001), null, etapa sem orçamento | `stats.test.tsx` (S4) |
| S5 | 5 | `latencyRows` e render estático com as seis linhas na ordem e as classes `text-ok`/`text-warn` | `stats.test.tsx` (S5) |
| S6 | 6 | média por dia, fatia de fallback (0 tentativas → "—") e render dos números | `stats.test.tsx` (S6) |
| S7 | 7 | `dayBars` preenche os dias sem ditado, estende para um dia mais antigo, `title` com o valor | `stats.test.tsx` (S7) |
| S8 | 8 | `summaryMarkdown` exato para um resumo fixo em pt | `stats.test.tsx` (S8) |
| S9 | 9 | zero ditados → só o estado vazio | `stats.test.tsx` (S9) |
| S10 | 10 | chaves novas em pt e en; sem cor fora dos tokens | `stats.test.tsx` (S10), `bun run check:translations`, `bun run lint` |
| S11 | — | portões | `bunx tsc --noEmit`, `bun run build`, `bunx prettier --check .` |

## Prova manual (Windows)

- M1: app de dev em modo portable sobre uma **cópia** do `fala.sqlite`; Desempenho em 7 dias;
  comparar ditados, palavras e p50/p90 de `e2e` com `fala-cli history stats --days 7` sobre a
  mesma cópia. Colar o "Copiar resumo" no PR. Se não rodar, fica Unproven.
