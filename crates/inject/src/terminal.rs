//! Que acorde cola em cada app. Num terminal, Ctrl+V não é colar: o conhost e o PuTTY mandam
//! `^V` ao programa, o Alacritty e o WezTerm não o ligam por padrão, e um TUI rodando no
//! Windows Terminal recebe a tecla antes do terminal. Shift+Insert cola em todos eles.
//!
//! Os nomes seguem o formato de [`app_name_from_exe_path`](crate::app_name_from_exe_path):
//! minúsculas, sem `.exe`. A lista é só dos terminais; o estilo do texto para um terminal ou um
//! app de IA é pergunta do `fala-postproc`, que tem a lista dele.

use fala_core::AppContext;

use crate::PasteChord;

/// Terminais, pelo nome do executável. Os nove primeiros são os que o Wispr Flow lista no
/// Windows; depois vêm os hosts de console do próprio Windows e os terminais do Linux.
const TERMINALS: &[&str] = &[
    "windowsterminal",
    "cmd",
    "powershell",
    "pwsh",
    "warp",
    "alacritty",
    "hyper",
    "kitty",
    "wezterm",
    "wt",
    "openconsole",
    "conhost",
    "wezterm-gui",
    "tabby",
    "mintty",
    "ghostty",
    "conemu",
    "conemu64",
    "putty",
    "gnome-terminal-server",
    "kgx",
    "ptyxis",
    "konsole",
    "xterm",
    "foot",
];

/// `true` se `app_name` (no formato do `AppContext`) é um terminal.
pub fn is_terminal(app_name: &str) -> bool {
    let name = app_name.trim();
    !name.is_empty() && TERMINALS.iter().any(|t| t.eq_ignore_ascii_case(name))
}

impl PasteChord {
    /// O acorde que cola em `app`: Shift+Insert num terminal, Ctrl+V em qualquer outro app e
    /// no app desconhecido.
    pub fn for_app(app: &AppContext) -> PasteChord {
        match app.app_name.as_deref() {
            Some(name) if is_terminal(name) => PasteChord::ShiftInsert,
            _ => PasteChord::CtrlV,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminals_by_app_name() {
        for name in [
            "windowsterminal",
            "WindowsTerminal",
            "pwsh",
            "cmd",
            "conhost",
            "wezterm-gui",
            "alacritty",
            "kitty",
        ] {
            assert!(is_terminal(name), "{name:?} é terminal");
        }
        for name in ["chrome", "code", "cursor", "notepad", "claude", "", "  "] {
            assert!(!is_terminal(name), "{name:?} não é terminal");
        }
    }

    #[test]
    fn chord_for_app() {
        let app = |name: Option<&str>| AppContext {
            app_name: name.map(str::to_owned),
        };
        assert_eq!(
            PasteChord::for_app(&app(Some("windowsterminal"))),
            PasteChord::ShiftInsert
        );
        assert_eq!(
            PasteChord::for_app(&app(Some("pwsh"))),
            PasteChord::ShiftInsert
        );
        assert_eq!(PasteChord::for_app(&app(Some("chrome"))), PasteChord::CtrlV);
        assert_eq!(PasteChord::for_app(&app(None)), PasteChord::CtrlV);
        assert_eq!(
            PasteChord::for_app(&AppContext::default()),
            PasteChord::CtrlV
        );
    }

    #[test]
    fn list_is_normalized() {
        let mut seen = std::collections::HashSet::new();
        for name in TERMINALS {
            assert_eq!(*name, name.to_ascii_lowercase(), "{name:?} em minúsculas");
            assert!(!name.ends_with(".exe"), "{name:?} sem .exe");
            assert!(!name.contains(char::is_whitespace), "{name:?} sem espaço");
            assert!(seen.insert(*name), "{name:?} repetido");
        }
    }
}
