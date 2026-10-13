# reindex-granola-imports checks

Profile: light (diff de uma frase: o `.md` de uma reunião importada guarda a linha de
`meeting_imports` e o `reindex` a restaura). Sem plano.

Prefixo de todo `cargo`: `CARGO_TARGET_DIR='C:\f\followup-fixes'`.

**C1** - importar o CSV → apagar o banco → `reindex` → importar o mesmo CSV: 0 reuniões e 0 `.md`
a mais, as duas saem `Updated`, o título que a pessoa mudou antes fica; o `reindex` não reescreve
nenhum `.md`. Sem a restauração o teste falha.
Proof: `cargo test -p fala-storage --test granola importing_again_after_losing_the_db`

**C2** - só a sessão importada tem `import` no frontmatter; ida e volta igual byte a byte; valor
inválido (campos faltando, origem vazia, data ruim) vai para o relatório com motivo
Proof: `cargo test -p fala-storage --lib the_import_link_round_trips`

**C3** - reuniões não importadas: frontmatter e round trip do `reindex` iguais aos de antes
Proof: `cargo test -p fala-storage` (testes `meeting_mirror`, `reindex` sem mudança)

**C4** - Portões: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`
Proof: exit 0

Swept: conflito (a origem já ligada a outra sessão no banco): a sessão do `.md` entra sem a
ligação, o banco vence (`ON CONFLICT DO NOTHING`) · `serde_json` com `preserve_order` ligado por
outro crate: chaves escritas em ordem alfabética, o `.md` sai igual nos dois.
