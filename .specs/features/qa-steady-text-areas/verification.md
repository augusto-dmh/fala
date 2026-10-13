# qa-steady-text-areas — verification

Máquina: Windows 11 (Alienware 16), 2026-10-13. Verificado pelo próprio executor.

## Verdict

**PASS** em S1–S4. O app rodando não foi aberto (ver "Não provado").

| ID | Result | Evidence |
| --- | --- | --- |
| S1 | PASS | S1 ok: `font-normal` presente, `font-semibold` ausente |
| S2 | PASS | S2 ok: wrapper `relative w-full`, svg `aria-hidden`, input `type="search"` com `ps-8` e o rótulo, lupa `opacity-50` desabilitada; os cinco arquivos usam `SearchField` e nenhum tem `<Input type="search">` solto |
| S3 | PASS | S3 ok |
| S4 | PASS | lint sem erro nem aviso; prettier limpo; tsc sem erro; `controls.test.tsx` (C1–C24), `shell.test.tsx` e os 15 testes de Reuniões passam |

## Não provado

Roteiro no app: abrir Configurações (rola) e Dicionário vazio (não rola) e conferir que o título
não pula; ver a lupa nas buscas do Dicionário e de Reuniões; digitar uma nota longa em "Minhas
notas" e ver o peso normal.
