# granola-csv-import checks

Prefixo de todo `cargo`: `CARGO_TARGET_DIR='C:\f\granola-csv-import'`.

**C1** - BOM, cabeçalho em maiúsculas, aspas e quebra de linha na célula; obrigatórias ausentes
nomeadas; data com `CEST`/`BST`/offset; célula grande, id vazio e data ruim rejeitados com a
linha; id repetido vale o último; vazio, UTF-16 e > 5.000 linhas recusados; `.zip` com
cabeçalhos iguais lido pelo conteúdo, diferentes recusado
Proof: `cargo test -p fala-storage granola`

**C2** - reunião importada: `mode = 'import'`, sem áudio, no instante do CSV; notas com o marcador
de IA; trechos sem tempo por falante; importar de novo atualiza a mesma sessão (sem duplicar) e
apaga a marca de nota editada; apagada no Fala, volta
Proof: `cargo test -p fala-storage --test granola`; `cargo test -p fala-storage --lib meeting_imports`

**C3** - prévia não grava; importar emite progresso e conta criadas, atualizadas e ignoradas
Proof: `cargo test -p fala --lib meeting`

**C4** - diálogo: prévia com contagem, período e rejeitadas; progresso; fim; selo "Granola"
Proof: `bun src/components/meeting/granola.test.tsx`; `bun run check:translations`

**C5** - manual, Windows: o CSV real do Augusto (não está na máquina)
Proof: roteiro em Verification
