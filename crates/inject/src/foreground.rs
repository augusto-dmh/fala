//! O app em primeiro plano, para o `AppContext` do ditado.
//!
//! Só o nome do executável sai daqui; o título da janela nunca é lido, porque carrega conteúdo
//! do documento e a ADR-0004 manda só o nome do app ao LLM.

use fala_core::AppContext;

use crate::InjectError;

/// O app em primeiro plano agora, ou app desconhecido (`app_name: None`) quando não dá para saber.
///
/// Nunca falha: app desconhecido é um caso normal (no GNOME Wayland é sempre ele). O motivo vai
/// ao log em `debug` e fica disponível em [`try_foreground_app`].
pub fn foreground_app() -> AppContext {
    context_or_unknown(try_foreground_app())
}

/// O app em primeiro plano agora, ou o motivo de não dar para saber.
pub fn try_foreground_app() -> Result<AppContext, InjectError> {
    platform::foreground_exe_path().map(|path| AppContext {
        app_name: app_name_from_exe_path(&path),
    })
}

fn context_or_unknown(result: Result<AppContext, InjectError>) -> AppContext {
    result.unwrap_or_else(|err| {
        log::debug!("app em primeiro plano desconhecido: {err}");
        AppContext::default()
    })
}

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

/// Fora do Windows não há detecção. No GNOME Wayland, um app comum não tem como saber o app em
/// foco sem extensão: `org.gnome.Shell.Introspect` responde `AccessDenied`, `Shell.Eval` está
/// desligado e o `_NET_ACTIVE_WINDOW` do XWayland só vê janelas X11 (medido em 2026-10-02 no
/// GNOME 48). O adaptador Linux é da fase 3 (ADR-0007).
#[cfg(not(windows))]
mod platform {
    use crate::InjectError;

    const UNSUPPORTED: &str = "esta plataforma não expõe o app em foco a apps comuns \
                               (GNOME Wayland: Introspect nega o acesso; Linux é a fase 3)";

    pub(super) fn foreground_exe_path() -> Result<String, InjectError> {
        Err(InjectError::Unsupported(UNSUPPORTED))
    }
}

/// Windows: janela em primeiro plano → processo dono → caminho do executável.
///
/// TODO(windows): verificar à mão com `cargo run -p fala-inject --example foreground_app -- chrome`
/// e o Chrome em foco; conferir também um app elevado e um app UWP (`applicationframehost`).
#[cfg(windows)]
mod platform {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    use crate::InjectError;

    /// Caminho longo máximo do Windows, em unidades UTF-16.
    const MAX_PATH_UTF16: u32 = 32_768;

    pub(super) fn foreground_exe_path() -> Result<String, InjectError> {
        // SAFETY: sem argumentos; devolve nulo quando não há janela em primeiro plano.
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.is_null() {
            return Err(InjectError::NoForegroundWindow);
        }
        let mut pid = 0u32;
        // SAFETY: `hwnd` acabou de vir do sistema e `pid` é um `u32` válido para escrita.
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
        if pid == 0 {
            return Err(last_error("GetWindowThreadProcessId"));
        }
        // SAFETY: só abre um handle de consulta, fechado por `ProcessHandle::drop`.
        let process =
            ProcessHandle(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) });
        if process.0.is_null() {
            return Err(last_error("OpenProcess"));
        }
        let mut buf = vec![0u16; MAX_PATH_UTF16 as usize];
        let mut len = MAX_PATH_UTF16;
        // SAFETY: `buf` tem `len` unidades; o sistema escreve no máximo isso e devolve em `len`
        // quantas escreveu, sem o terminador.
        let ok = unsafe {
            QueryFullProcessImageNameW(process.0, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len)
        };
        if ok == 0 {
            return Err(last_error("QueryFullProcessImageNameW"));
        }
        buf.truncate(len as usize);
        Ok(String::from_utf16_lossy(&buf))
    }

    struct ProcessHandle(HANDLE);

    impl Drop for ProcessHandle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: handle aberto por `OpenProcess`, fechado uma vez só.
                unsafe { CloseHandle(self.0) };
            }
        }
    }

    /// Lê o erro antes de qualquer outra chamada (o `CloseHandle` do drop o sobrescreveria).
    fn last_error(call: &'static str) -> InjectError {
        // SAFETY: sem argumentos; lê o último erro da thread atual.
        let code = unsafe { GetLastError() };
        InjectError::Os { call, code }
    }
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

    #[cfg(not(windows))]
    #[test]
    fn non_windows_detection_is_unsupported() {
        match try_foreground_app() {
            Err(InjectError::Unsupported(reason)) => assert!(!reason.trim().is_empty()),
            other => panic!("esperava Unsupported, veio {other:?}"),
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_context_is_unknown() {
        assert_eq!(foreground_app(), AppContext { app_name: None });
    }

    #[test]
    fn every_error_becomes_unknown_app() {
        for err in [
            InjectError::Unsupported("x"),
            InjectError::NoForegroundWindow,
            InjectError::Os {
                call: "OpenProcess",
                code: 5,
            },
        ] {
            assert_eq!(
                context_or_unknown(Err(err.clone())),
                AppContext::default(),
                "{err:?}"
            );
        }
        let known = AppContext {
            app_name: Some("code".to_string()),
        };
        assert_eq!(context_or_unknown(Ok(known.clone())), known);
    }
}
