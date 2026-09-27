//! Inserção do texto no app ativo.
//!
//! Trait `Injector` com um adaptador por sistema operacional.
//! Windows: clipboard + `SendInput` Ctrl+V com restore; Unicode direto para textos curtos.
//! Falha de inserção deixa o texto no clipboard e a UI oferece "Colar".
//! Nasce vazio no dia 1; `apps/desktop/src/clipboard.rs` e `paste_tx` migram na fase 1.
