# dictation-stats — página "Desempenho" sobre `get_dictation_stats`

## Problem

O desktop grava os tempos de cada ditado, sem texto, em `dictation_metrics` (#75) e expõe o resumo
por `get_dictation_stats(days)`, mas nenhuma tela chama o comando: o número só sai pelo
`fala-cli history stats`. O fechamento da fase 1 pede "latência dentro do orçamento do
`ARCHITECTURE.md`" e o p50/p90 real ficou sem prova. Uma página no app instalado responde com o uso
real, sem telemetria.

Quando isto entrar, o rail tem **Desempenho** entre Reuniões e o rodapé; a página mostra, para
7/30/90 dias, ditados, palavras, média de palavras por dia com ditado, a tabela p50/p90 por etapa
contra o orçamento, o bloco da IA, barras de palavras por dia e "Copiar resumo" em Markdown.

## Flow

Só frontend. O backend não muda.

1. `Rail.tsx` (exists) — `RailDestination` ganha `"stats"`; item `rail.stats` com `ChartColumn`, depois de `meetings`
2. `App.tsx` (exists) — `destination === "stats" && <StatsPage />`; a janela segue abrindo em `home`
3. `stats/StatsPage.tsx` (new) — lê o período lembrado, chama `commands.getDictationStats(days)` a cada troca, guarda o resultado; `StatsView` (puro, sem efeitos) desenha
4. `stats/statsModel.ts` (new) — orçamentos, `formatMs`, `budgetState`, `latencyRows`, `averageWordsPerDay`, `fallbackShare`, `dayBars`, `summaryMarkdown`, `readPeriod`/`writePeriod`
5. locales `pt`/`en` — `rail.stats` e bloco `stats.*`

Novo em `localStorage`: `fala.stats.days` (7, 30 ou 90), só conveniência. Nenhuma one-way door:
tudo é frontend e reversível.

## Criteria

1. The rail SHALL list Início, Dicionário, Reuniões, Desempenho on top and Configurações in the footer; the window SHALL still open on Início
2. The page SHALL call `getDictationStats(days)` with 7, 30 or 90, defaulting to 7 and remembering the choice in `localStorage` without failing when storage throws
3. `formatMs` SHALL render < 1000 ms as "380 ms" and ≥ 1000 ms as seconds with one decimal in the UI language ("1,2 s" pt, "1.2 s" en); null SHALL render "—"
4. For `e2e` (700/1000) and `e2e_llm` (1200/2000) a percentile SHALL be `ok` at or below the target, `warn` above it, and `over` (warn colour plus "acima do máximo") above the maximum; the other stages have no budget and no state
5. The table SHALL have six rows in order e2e, e2e_llm, asr, llm, paste, speech with p50, p90 and budget columns
6. The page SHALL show dictations, words and the average words per day with dictation; the AI block SHALL show attempts, fallbacks and their share ("—" with no attempts)
7. Words per day SHALL be CSS bars, one per local day of the period up to today (days without dictation at zero), each with its value in `title`
8. "Copiar resumo" SHALL copy Markdown with the period, the counts and the p50/p90/budget table
9. With zero dictations the page SHALL show only the empty state
10. Every visible string SHALL come from i18next (pt source, en); colours only from theme tokens (ADR-0017)

## Out of scope

Streak, ranking, gauge, heatmap, compartilhar, metas; recorte por app ou idioma; métrica nova ou
mudança no comando; palavras por minuto; métricas de reunião.

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Orçamento de asr, llm, paste | sem orçamento ("—") | o `ARCHITECTURE.md` só orça soltar → texto; inventar meta por etapa seria número sem fonte | y — decidido pelo executor |
| Texto do `warn` | "acima da meta" entre a meta e o máximo, "acima do máximo" depois | a cor não pode ser a única pista (WCAG 1.4.1) | y — decidido pelo executor |
| Barras | um dia por barra de `hoje − (days − 1)` a hoje, estendendo para trás se o resumo trouxer um dia mais antigo (a janela é de `days × 24 h`) | o `per_day` só traz dias com ditado; sem preencher, 3 barras em 30 dias enganam | y — decidido pelo executor |
| Média por dia | palavras ÷ dias com ditado, arredondada | é o que a definição pede ("por dia com ditado") | y — definição |

**Open questions:** none.
