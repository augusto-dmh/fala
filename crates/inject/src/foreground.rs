//! O app em primeiro plano, para o `AppContext` do ditado.
//!
//! Só o nome do executável sai daqui; o título da janela nunca é lido, porque carrega conteúdo
//! do documento e a ADR-0004 manda só o nome do app ao LLM.

/// Reduz o caminho de um executável ao nome do app que o Fala usa no `AppContext`.
///
/// O nome é o arquivo depois do último `\` ou `/`, sem o sufixo `.exe` (sem diferenciar
/// maiúsculas), em minúsculas: `C:\...\Chrome\Application\chrome.exe` vira `chrome`. Aceita os
/// dois separadores em qualquer plataforma, para a regra ser testável fora do Windows.
/// Devolve `None` quando não sobra nome.
pub fn app_name_from_exe_path(path: &str) -> Option<String> {
    let file = path.rsplit(['\\', '/']).next().unwrap_or(path);
    let name = match file.len().checked_sub(4) {
        Some(cut) if file.is_char_boundary(cut) && file[cut..].eq_ignore_ascii_case(".exe") => {
            &file[..cut]
        }
        _ => file,
    };
    (!name.is_empty()).then(|| name.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exe_path_becomes_lowercase_name() {
        for (path, expected) in [
            (
                r"C:\Program Files\Google\Chrome\Application\chrome.exe",
                "chrome",
            ),
            (
                r"C:\Users\a\AppData\Local\Programs\Microsoft VS Code\Code.exe",
                "code",
            ),
            (
                r"C:\Program Files\WindowsApps\MSTeams_x64\ms-teams.EXE",
                "ms-teams",
            ),
            (r"C:\Tools\Foo.Bar.exe", "foo.bar"),
            ("notepad", "notepad"),
            ("/usr/bin/gnome-text-editor", "gnome-text-editor"),
        ] {
            assert_eq!(
                app_name_from_exe_path(path),
                Some(expected.to_string()),
                "{path}"
            );
        }
    }

    #[test]
    fn exe_path_without_name_is_none() {
        for path in ["", r"C:\dir\", ".exe", r"C:\x\.EXE"] {
            assert_eq!(app_name_from_exe_path(path), None, "{path:?}");
        }
    }
}
