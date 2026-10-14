# qa-undo-deleted-dictation — checks

R2 e R6 da segunda passada de QA (`research/27-reqa-fala.md`): "Delete entry" apagava na hora, sem
volta, e o menu ⋮ deixava o foco em `body` ao fechar.

| ID | Check | Proof |
| --- | --- | --- |
| U1 | "Desfazer" devolve o ditado ao lugar do horário dele, sem repetir | `bun src/components/settings/history/deleteUndo.test.ts` U1 |
| U2 | apagar tira a linha na hora e mostra "Ditado apagado [Desfazer]"; o comando vai quando o toast fecha (o relógio dele pausa sob o ponteiro), uma vez só; o que espera fica fora das páginas e buscas que chegam; "Desfazer" volta à lista em que estava (e à busca só se for a mesma); sair da página manda o que espera; o foco vai ao ⋮ da linha vizinha | `deleteUndo.test.ts` U2 |
| U3 | Esc ou um item escolhido devolvem o foco ao ⋮ | `deleteUndo.test.ts` U3 |
| U4 | portões | `bun run lint`, `bunx prettier --check .`, `bunx tsc --noEmit`, `bun run check:translations`, os testes de `src/components/` |
