# llm-auto-timing checks

Profile: light (diff de uma frase: o bloco lento do teste espera uma resposta que só existe
quando o teste a solta, em vez de um teto de relógio). Sem plano.

Prefixo de todo `cargo`: `CARGO_TARGET_DIR='C:\f\followup-fixes'`.

**C1** - `a_slow_block_keeps_its_rules_until_the_late_answer` não mede relógio: o servidor lento
segura a resposta até `answer.send`; os blocos voltam com as regras no bloco lento
(`Fallback::Timeout`, `partial`), e só então a resposta tardia chega e vira a edição tardia.
Continua provando que o bloco lento desiste aos 2 s e não aos 10 s: se esperasse o prazo tardio,
não sobraria resposta tardia (`late` vazio ou `wait` com erro)
Proof: `cargo test -p fala --lib llm_auto::tests::a_slow_block_keeps_its_rules_until_the_late_answer`, repetido

**C2** - os outros testes de `llm_auto` passam com o `FakeGemini::start_with` comum
Proof: `cargo test -p fala --lib llm_auto`

**C3** - Portões: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`
Proof: exit 0
