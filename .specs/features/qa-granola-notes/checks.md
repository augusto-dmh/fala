# qa-granola-notes — checks

Achados Q13, Q14, Q15 e Q23 do QA de 2026-10-13, na parte da UI. O crate do import
(`crates/storage/src/meeting_imports.rs`) e o `SessionHeader.tsx` estão com outro painel nesta
rodada: o rodapé continua gravado em pt pelo crate (a UI o reescreve no idioma dela) e a duração
"00:00" do cabeçalho fica para depois. Diff de uma frase; sem plano.

| ID | Achado | Check | Proof |
| --- | --- | --- | --- |
| G1 | Q13 | o título de seção importado sai sem `<!-- fala:ia -->`, na leitura e no editor | `bun src/components/meeting/granolaNotes.test.tsx` G1 |
| G2 | Q14 | `**x**`, `*x*` e `_x_` viram negrito e itálico; `snake_case`, `5*3*2`, `2 ** 3 ** 4` e marcador sem par ficam como digitados; bloco de código e tabela ficam como escritos | `granolaNotes.test.tsx` G2 |
| G3 | Q14 | o rodapé "_Importado do Granola em AAAA-MM-DD._" sai pela chave `meeting.granola.importedOn`, com a data local no idioma da UI | `granolaNotes.test.tsx` G3 |
| G4 | Q15 | a transcrição importada do Granola não tem ▶ nem `mm:ss` (nem no trecho omitido) e diz "Importada do Granola: a transcrição não tem áudio nem tempos" no lugar de "o áudio não está mais no disco" | `granolaNotes.test.tsx` G4 |
| G5 | Q23 | a folha da transcrição fica sobre a barra (`z-[45]`, entre a barra `z-40` e os diálogos `z-50`) e desce até o fim da janela, sem o `pb-24`; a barra volta ao fechar (✕ ou Esc), como no Granola | `granolaNotes.test.tsx` G5 e `bar.test.tsx` C11 |
| G6 | — | portões | `bun run lint`, `bunx prettier --check .`, `bunx tsc --noEmit`, `bun run check:translations`, os 16 testes de `src/components/meeting/` e `shell.test.tsx` |
