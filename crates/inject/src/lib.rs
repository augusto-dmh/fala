//! Inserção do texto no app ativo e detecção de qual app é esse.
//!
//! Detecção: `foreground_app` devolve o `AppContext` do app em primeiro plano (ADR-0004: só o
//! nome do app vai ao LLM). Windows pergunta ao sistema; as outras plataformas, GNOME Wayland
//! incluído, devolvem app desconhecido, com o motivo em `InjectError::Unsupported`.
//!
//! Inserção: [`platform_injector`] devolve o [`Injector`] do sistema operacional, que cola por
//! clipboard + acorde ([`PasteChord`], Ctrl+V por padrão) e devolve ao clipboard o que estava
//! nele, a sequência que o spike 03 mediu. O acorde depende do app: num terminal
//! ([`is_terminal`]) é Shift+Insert, porque Ctrl+V lá não cola ([`PasteChord::for_app`]).
//! O texto só é entregue quando um app o lê do clipboard, e essa leitura é a prova da cola: sem
//! ela em `read_timeout`, `insert` devolve `InjectError::PasteNotRead`. Se o acorde falha,
//! devolve `InjectError::Keystroke`. Nos dois casos o clipboard volta ao conteúdo de antes (se
//! ninguém escreveu nele nesse meio-tempo) e quem chama ainda tem o texto para oferecer "Colar".
//! O texto colado fica fora do histórico do Win+V e da área de transferência na nuvem.
//! Windows: delayed rendering numa janela message-only, `arboard` para salvar e restaurar e
//! `enigo` para o acorde. As outras plataformas devolvem `Unsupported`; o portal
//! RemoteDesktop é a fase 3 (ADR-0007). O desktop ainda usa `apps/desktop/src/clipboard.rs`; a
//! troca é uma feature própria.

mod foreground;
mod paste;
mod terminal;

pub use foreground::{app_name_from_exe_path, foreground_app, try_foreground_app};
pub use paste::{platform_injector, Injector, PasteChord, PasteConfig};
pub use terminal::is_terminal;

/// Erros de `fala-inject`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InjectError {
    /// A plataforma não deixa um app comum saber qual app está em foco, ou colar nele.
    #[error("indisponível nesta plataforma: {0}")]
    Unsupported(&'static str),
    /// Não há janela em primeiro plano (troca de foco em curso, tela de bloqueio).
    #[error("nenhuma janela em primeiro plano")]
    NoForegroundWindow,
    /// Uma chamada do sistema falhou; `code` é o código de erro dele.
    #[error("{call} falhou com o código {code}")]
    Os { call: &'static str, code: u32 },
    /// Ler, escrever ou limpar o clipboard falhou.
    #[error("falha no clipboard: {0}")]
    Clipboard(String),
    /// O acorde de colar não foi enviado.
    #[error("falha ao enviar o acorde de colar: {0}")]
    Keystroke(String),
    /// Nenhum app leu o texto colado dentro do prazo: provavelmente não havia campo em foco.
    #[error("nenhum app leu o texto colado")]
    PasteNotRead,
}
