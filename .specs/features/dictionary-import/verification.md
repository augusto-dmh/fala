# dictionary-import — verification

Máquina: Windows 11 (Alienware 16), 2026-10-13. Verificado pelo próprio executor.

**PASS** em I1–I5: `bun src/components/dictionaryImport.test.tsx` I1–I4 ok;
`dictionaryList.test.tsx` D1–D4 ok; `bun run lint` sem aviso; `bun run format:check` limpo;
`bunx tsc --noEmit` sem erro; `bun run check:translations` completo. Dos 47 testes de `src/`,
43 passam; falham `meeting/home`, `meeting/import`, `meeting/search` (asserções de texto sobre o
código de `MeetingsPage`, que esta branch não toca) e `styles/tokens` (caminho de URL no
Windows), os mesmos de antes desta feature.

Revisão `code-review high` do diff: corrigidos o arquivo fora de UTF-8 (agora o decodificador
recusa e mostra o erro), o toast de sucesso depois de uma gravação que falhou (agora conta o que
o store guardou; `custom_words` passa a lançar o erro do comando), a falta de `try/finally`, o
arquivo grande lido inteiro (recusado acima de 4 MB pelo `stat`), o texto de "repetidas" e a
cópia de `normalizeSpoken`. Mantidos como a definição pede: o separador do arquivo vale para
toda linha, `#` é comentário, cabeçalho óbvio é pulado.

Não provado no app: escolher um `.csv` de 10 linhas misturadas no seletor, ver a prévia,
Cancelar e Importar, conferir a lista e o toast.
