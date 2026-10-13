# qa-granola-notes — verification

Máquina: Windows 11 (Alienware 16), 2026-10-13. Verificado pelo próprio executor.

## Verdict

**PASS** em G1–G6. O app rodando não foi aberto (ver "Não provado").

| ID | Result | Evidence |
| --- | --- | --- |
| G1 | PASS | G1 ok: `{ kind: "heading", level: 3, text: "Decisões" }`; `editableText` começa com `### Decisões` |
| G2 | PASS | G2 ok (as quatro linhas literais preservadas) |
| G3 | PASS | G3 ok: HTML sem `fala:ia` nem `&lt;!--`, `<strong>`, `<em>`, código e tabela como escritos, "Oct 12, 2026" em en-US, sem a frase pt; chaves em pt e en |
| G4 | PASS | G4 ok: `timed={false}` sem `meeting.source.play` nem `00:00`, o trecho omitido sem hora; o padrão mantém os dois |
| G5 | PASS | G5 ok; `bar.test.tsx` C11 atualizado para `z-[45]` |
| G6 | PASS | lint sem aviso; prettier limpo; tsc sem erro; traduções completas; os testes de Reuniões (inclusive `bar.test.tsx` C11 e `meeting.test.tsx`) e `shell.test.tsx` passam |

## Não provado

Roteiro no app (cópia portátil): importar o CSV sintético do Granola, abrir a reunião: o título
"Decisões" sem marcador, o rodapé "Imported from Granola on Oct 13, 2026." em itálico, a
transcrição sem ▷ nem 00:00 e com a frase de import; abrir a transcrição a 900 px e ver a folha
inteira até o fim da janela, sem a barra por cima.
