# qa-one-casing — checks

Achados Q9, Q12, Q25, Q31, Q38, Q39, Q43 e Q45 (casing do menu da pílula) do QA de 2026-10-13. O
Q37 (IA ligada sem chave) é lógica do `LlmSettings`, que entrou no PR das chaves; os nomes de
tecla ("Left Ctrl", "Escape") e o `settings.rs` (nomes padrão dos atalhos, com outro painel)
ficam fora.

| ID | Achado | Check | Proof |
| --- | --- | --- | --- |
| C1 | Q31, Q45 | títulos, rótulos, botões e opções em sentence case em en e pt, nomes próprios e siglas mantidos (também com reticências e valores `{{x}}`); um teste varre os dois arquivos | `bun src/i18n/casing.test.ts` C1 |
| C2 | Q12, Q38 | sem macOS nem a dica de Linux nos textos de Avançado; "Settings for {{model}}"; descrição pt da posição da sobreposição igual à en; "recognize" | `casing.test.ts` C2 |
| C3 | Q9, Q43 | os tokens dos snippets explicados (en diz que os nomes ficam em pt; pt cita a data por extenso); a privacidade do MCP em linguagem simples, sem `= true`, dizendo que se liga no `mcp.toml` (não há chave no app) | `casing.test.ts` C3 |
| C4 | Q25, Q39 | "Minhas notas \| Notas da IA" ("My notes \| AI notes"); "Idioma do aplicativo" ao lado de "Tema do aplicativo"; "chamada" em pt | `casing.test.ts` C4 |
| C5 | Q39 | idiomas do ASR por `Intl.DisplayNames` no idioma da UI (seletor, filtro e cartão de modelo), com busca sem acento e pelo nome em inglês ou código; o dispositivo padrão com a palavra da bandeja ("Automático"), num helper só; placeholder do dropdown traduzido | `casing.test.ts` C5 |
| C6 | — | portões | `bun run lint`, `bunx prettier --check .`, `bunx tsc --noEmit`, `bun run check:translations`, todos os testes de `src/` |
