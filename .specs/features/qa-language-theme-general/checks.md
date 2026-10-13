# qa-language-theme-general — checks

Achado Q32 do QA de 2026-10-13: idioma do app e tema ficam em Sobre, ao lado de versão e pastas.
Diff de uma frase; sem plano.

| ID | Check | Proof |
| --- | --- | --- |
| A1 | Geral ganha o grupo "Aplicativo" (en "App") com idioma e tema, logo depois dos atalhos; Sobre não tem mais os dois | `bun src/components/settings/appGroup.test.ts` |
| A2 | portões | `bun run lint`, `bunx prettier --check .`, `bunx tsc --noEmit`, `bun run check:translations`, `shell.test.tsx`, `privacy.test.tsx` e os testes de Reuniões que leem `GeneralSettings.tsx` |
