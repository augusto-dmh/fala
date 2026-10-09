//! Atalho global com press e release, para push-to-talk.
//!
//! [`platform_hotkey`] devolve o adaptador do sistema operacional atrás do trait
//! [`GlobalHotkey`]; cada toque de um atalho registrado chega como dois [`HotkeyEvent`]
//! (`Pressed` e `Released`) no canal de quem pediu.
//! Windows: hook `WH_KEYBOARD_LL` do crate `handy-keys`, em modo bloqueante, o mesmo que o
//! spike 02 mediu com a janela do Fala em foco. Os outros sistemas, GNOME Wayland incluído,
//! devolvem [`HotkeyError::Unsupported`]; o portal GlobalShortcuts é a fase 3 (ADR-0007).
//! `#[cfg(windows)]` e afins ficam aqui dentro, nunca fora dos crates de plataforma (ADR-0007).
//! O desktop ainda usa `apps/desktop/src/shortcut`; a troca é uma feature própria.

// Fora do Windows a tabela só é usada pelos testes, até o adaptador da fase 3.
#[cfg_attr(not(windows), allow(dead_code))]
mod bindings;
#[cfg(windows)]
mod windows;

use std::sync::mpsc::Sender;

/// Se o atalho foi pressionado ou solto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
}

/// Um toque num atalho registrado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotkeyEvent {
    /// O identificador com que o atalho foi registrado (`transcribe`, `cancel`).
    pub binding_id: String,
    /// O acelerador registrado, no formato do `handy-keys` (`ctrl+shift+space`).
    pub accelerator: String,
    pub state: KeyState,
}

/// Erros de `fala-hotkey`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HotkeyError {
    /// A plataforma não tem adaptador de atalho global.
    #[error("atalho global indisponível nesta plataforma: {0}")]
    Unsupported(&'static str),
    /// O acelerador não está no formato `mod+mod+tecla` do `handy-keys`.
    #[error("acelerador inválido {accelerator:?}: {reason}")]
    InvalidAccelerator { accelerator: String, reason: String },
    /// Já existe um atalho registrado com este `binding_id`.
    #[error("o atalho {0:?} já está registrado")]
    AlreadyRegistered(String),
    /// O hook do sistema falhou ou parou.
    #[error("falha no hook de teclado: {0}")]
    Backend(String),
}

/// Um atalho global com press e release. Soltar o valor remove o hook.
pub trait GlobalHotkey: Send {
    /// Registra `accelerator` sob `binding_id`; os eventos dele passam a sair no canal.
    fn register(&mut self, binding_id: &str, accelerator: &str) -> Result<(), HotkeyError>;
    /// Remove o atalho de `binding_id`. Um `binding_id` desconhecido não é erro.
    fn unregister(&mut self, binding_id: &str) -> Result<(), HotkeyError>;
}

/// O adaptador de atalho global desta plataforma, que envia os eventos em `events`.
pub fn platform_hotkey(events: Sender<HotkeyEvent>) -> Result<Box<dyn GlobalHotkey>, HotkeyError> {
    platform::hotkey(events)
}

#[cfg(windows)]
mod platform {
    use std::sync::mpsc::Sender;

    use crate::{GlobalHotkey, HotkeyError, HotkeyEvent};

    pub(super) fn hotkey(
        events: Sender<HotkeyEvent>,
    ) -> Result<Box<dyn GlobalHotkey>, HotkeyError> {
        Ok(Box::new(crate::windows::WindowsHotkey::start(events)?))
    }
}

/// Fora do Windows não há adaptador. No GNOME Wayland, um app comum só recebe atalho global
/// com key-up pelo portal GlobalShortcuts (relatório 13 §2), que é a fase 3 (ADR-0007).
#[cfg(not(windows))]
mod platform {
    use std::sync::mpsc::Sender;

    use crate::{GlobalHotkey, HotkeyError, HotkeyEvent};

    const UNSUPPORTED: &str = "sem adaptador de atalho global fora do Windows \
                               (GNOME Wayland: portal GlobalShortcuts, fase 3)";

    pub(super) fn hotkey(
        _events: Sender<HotkeyEvent>,
    ) -> Result<Box<dyn GlobalHotkey>, HotkeyError> {
        Err(HotkeyError::Unsupported(UNSUPPORTED))
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(windows))]
    #[test]
    fn non_windows_platform_hotkey_is_unsupported() {
        let (tx, rx) = std::sync::mpsc::channel();
        match super::platform_hotkey(tx) {
            Err(super::HotkeyError::Unsupported(reason)) => assert!(!reason.trim().is_empty()),
            Err(other) => panic!("esperava Unsupported, veio {other:?}"),
            Ok(_) => panic!("esperava Unsupported, veio um adaptador"),
        }
        assert!(
            rx.try_recv().is_err(),
            "nenhum evento pode sair sem adaptador"
        );
    }
}
