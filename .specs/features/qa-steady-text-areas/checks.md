# qa-steady-text-areas — checks

Achados Q10, Q11 e a parte do gutter do Q54 do QA de 2026-10-13 (a largura de Reuniões fica no
PR de Reuniões). Diff de uma frase; sem plano.

| ID | Achado | Check | Proof |
| --- | --- | --- | --- |
| S1 | Q10 | o `Textarea` usa `font-normal` (notas, snippets, estilos, perfil) | `bun src/components/ui/steadyLayout.test.tsx` S1 |
| S2 | Q11 | Dicionário, Reuniões, a busca na transcrição, a de templates e a de Modelos usam `SearchField`, com a lupa, `type="search"` e a lupa apagada quando desabilitado (o Início fica com o PR do Início) | `steadyLayout.test.tsx` S2 |
| S3 | Q54 | o `main` tem `scrollbar-gutter: stable both-edges` (centrado como o onboarding) | `steadyLayout.test.tsx` S3 |
| S4 | — | portões | `bun run lint`, `bunx prettier --check .`, `bunx tsc --noEmit`, `controls.test.tsx`, `shell.test.tsx` e os testes de `src/components/meeting/` |
