//! Que tipo de campo recebe o texto, pelo nome do app que o `fala-inject` dá (exe em minúsculas,
//! sem `.exe`). Só dado: a detecção do app e do terminal fica no `inject`. Navegador fica sem
//! dica, porque o nome do processo não diz se a aba é um e-mail ou um chat de IA.

use std::fmt;

/// O estilo que o prompt pede para o destino.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Destination {
    Email,
    Chat,
    /// Terminal ou app de IA: texto para um assistente ou uma linha de comando.
    Prompt,
    Editor,
}

impl Destination {
    /// O literal que vai em `<destination>`.
    pub fn as_str(self) -> &'static str {
        match self {
            Destination::Email => "email",
            Destination::Chat => "chat",
            Destination::Prompt => "prompt",
            Destination::Editor => "editor",
        }
    }
}

impl fmt::Display for Destination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

const EMAIL: &[&str] = &["outlook", "olk", "hxoutlook", "thunderbird", "mailspring"];
const CHAT: &[&str] = &[
    "slack", "teams", "ms-teams", "discord", "whatsapp", "telegram", "signal",
];
/// Terminais e apps de IA.
const PROMPT: &[&str] = &[
    "windowsterminal",
    "wt",
    "cmd",
    "conhost",
    "powershell",
    "pwsh",
    "wezterm-gui",
    "alacritty",
    "warp",
    "gnome-terminal-server",
    "kgx",
    "konsole",
    "kitty",
    "claude",
    "chatgpt",
    "cursor",
    "windsurf",
];
const EDITOR: &[&str] = &[
    "code",
    "zed",
    "notepad",
    "notepad++",
    "sublime_text",
    "obsidian",
    "notion",
    "winword",
    "idea64",
    "pycharm64",
];

const APPS: [(&[&str], Destination); 4] = [
    (EMAIL, Destination::Email),
    (CHAT, Destination::Chat),
    (PROMPT, Destination::Prompt),
    (EDITOR, Destination::Editor),
];

/// O destino de um app, comparado sem caixa; `None` para app fora da tabela.
pub fn destination_for(app_name: &str) -> Option<Destination> {
    let name = app_name.trim();
    APPS.iter()
        .find(|(apps, _)| apps.iter().any(|app| app.eq_ignore_ascii_case(name)))
        .map(|(_, destination)| *destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destination_by_app_name() {
        let cases = [
            ("outlook", Some(Destination::Email)),
            ("olk", Some(Destination::Email)),
            ("slack", Some(Destination::Chat)),
            ("Teams", Some(Destination::Chat)),
            ("windowsterminal", Some(Destination::Prompt)),
            ("claude", Some(Destination::Prompt)),
            ("cursor", Some(Destination::Prompt)),
            ("code", Some(Destination::Editor)),
            ("notepad", Some(Destination::Editor)),
            ("msedge", None),
            ("", None),
        ];
        for (app, want) in cases {
            assert_eq!(destination_for(app), want, "{app:?}");
        }
        for app in APPS.iter().flat_map(|(apps, _)| apps.iter()) {
            assert_eq!(app.to_lowercase(), *app, "nomes em minúsculas: {app}");
        }
    }
}
