//! Inserção do texto no app ativo e detecção de qual app é esse.
//!
//! Detecção: `app_name_from_exe_path` dá o nome do app que vai ao `AppContext` (ADR-0004: só o
//! nome do app vai ao LLM).
//!
//! Inserção: trait `Injector` com um adaptador por sistema operacional.
//! Windows: clipboard + `SendInput` Ctrl+V com restore; Unicode direto para textos curtos.
//! Falha de inserção deixa o texto no clipboard e a UI oferece "Colar".
//! `apps/desktop/src/clipboard.rs` e `paste_tx` migram na fase 1.

mod foreground;

pub use foreground::app_name_from_exe_path;
