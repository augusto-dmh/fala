# qa-one-casing — verification

Máquina: Windows 11 (Alienware 16), 2026-10-13. Verificado pelo próprio executor.

## Verdict

**PASS** em C1–C6. O app rodando não foi aberto (ver "Não provado").

| ID | Result | Evidence |
| --- | --- | --- |
| C1 | PASS | C1 ok: nenhum título fora do sentence case em en e pt |
| C2 | PASS | C2 ok |
| C3 | PASS | C3 ok |
| C4 | PASS | C4 ok |
| C5 | PASS | C5 ok: "Português"/"Portuguese", "Javanês" para `jw`; "portugues", "Portuguese" e "pt" acham Português; `deviceLabel` |
| C6 | PASS | lint sem aviso; prettier limpo; tsc sem erro; traduções completas; todos os testes de `src/` passam, com `output.test.tsx` atualizado para "chamada". `src/styles/tokens.test.ts` falha igual em `origin/main` no Windows (`scandir '/C:/…'`, caminho de URL), sem relação com este diff |

## Não provado

Roteiro no app: Configurações em en e pt (títulos em sentence case), o seletor de idioma do
ditado em pt ("Português"), o microfone "Automático" e o menu do clique direito da pílula ("Paste
last dictation", "Check for updates...").
