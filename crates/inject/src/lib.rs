//! Inserção do texto no app ativo e detecção de qual app é esse.
//!
//! Detecção: `foreground_app` devolve o `AppContext` do app em primeiro plano (ADR-0004: só o
//! nome do app vai ao LLM). Windows pergunta ao sistema; as outras plataformas, GNOME Wayland
//! incluído, devolvem app desconhecido, com o motivo em `InjectError::Unsupported`.
//!
//! Inserção: trait `Injector` com um adaptador por sistema operacional.
//! Windows: clipboard + `SendInput` Ctrl+V com restore; Unicode direto para textos curtos.
//! Falha de inserção deixa o texto no clipboard e a UI oferece "Colar".
//! `apps/desktop/src/clipboard.rs` e `paste_tx` migram na fase 1.

mod foreground;

pub use foreground::{app_name_from_exe_path, foreground_app, try_foreground_app};

/// Erros de `fala-inject`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InjectError {
    /// A plataforma não deixa um app comum saber qual app está em foco.
    #[error("detecção do app em foco indisponível: {0}")]
    Unsupported(&'static str),
    /// Não há janela em primeiro plano (troca de foco em curso, tela de bloqueio).
    #[error("nenhuma janela em primeiro plano")]
    NoForegroundWindow,
    /// Uma chamada do sistema falhou; `code` é o código de erro dele.
    #[error("{call} falhou com o código {code}")]
    Os { call: &'static str, code: u32 },
}
