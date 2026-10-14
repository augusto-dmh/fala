# qa-undo-deleted-dictation — verification

Máquina: Windows 11 (Alienware 16), 2026-10-13. Verificado pelo próprio executor.

**PASS** em U1–U4: `deleteUndo.test.ts` U1–U3 ok; os testes do histórico passam; lint, prettier,
tsc e traduções limpos. Em `src/components/`, `meeting/home.test.tsx`, `import.test.tsx` e
`search.test.tsx` falham igual na base (desde o #186), sem relação com este diff.

Não provado no app: apagar um ditado pelo ⋮, ver o toast e clicar "Desfazer" (a linha volta);
apagar outro e esperar 5 s (some de vez); com o ponteiro sobre o toast, "Desfazer" segue valendo; Esc no ⋮ deixa o foco no botão.
