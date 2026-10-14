# qa-red-trash — verification

Máquina: Windows 11 (Alienware 16), 2026-10-13. Verificado pelo próprio executor.

**PASS** em T1–T5: `trashReach.test.tsx` T1–T4 ok; a prévia do `granola.test.tsx` passa com as
datas locais; lint, prettier, tsc e traduções limpos. Na base `92da49d`, `granola.test.tsx` (a
linha do `transcribeDisabled`), `home.test.tsx`, `import.test.tsx` e `search.test.tsx` já falham,
sem relação com este diff.

Não provado no app: menu ⋯ de uma reunião com "Mover para a lixeira" em vermelho; o botão da
lixeira ao lado de "Importar" com a contagem; importar o CSV do Granola, abrir uma reunião e ver o
cartão sumir; a confirmação de apagar para sempre com o texto novo.
