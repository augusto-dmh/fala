//! Quem está capturando o microfone agora: os processos com uma sessão de captura ativa.
//!
//! É o sinal que o Granola e o Wispr usam para notar que uma call começou (o app de reunião abriu
//! o mic) e que acabou (o app o largou). Só leitura: nada aqui abre um stream ou grava.
//!
//! Windows: `IAudioSessionManager2` em cada endpoint de captura ativo, lido de novo a cada
//! chamada, mais um snapshot dos processos para o nome do exe e dos ancestrais (o Teams novo
//! captura por um `msedgewebview2.exe` filho do `ms-teams.exe`). Nos outros SOs, `Unsupported`
//! até a fase 3 (ADR-0007: o `cfg` fica neste crate de plataforma).

use crate::AudioError;

/// Um processo com uma sessão de captura ativa num microfone.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MicHolder {
    pub pid: u32,
    /// Nome do executável em minúsculas, sem o caminho (`ms-teams.exe`).
    pub exe: String,
    /// Executáveis dos processos pais, do mais próximo ao mais distante, em minúsculas.
    pub ancestors: Vec<String>,
}

impl MicHolder {
    /// O exe seguido dos ancestrais, a ordem em que um app é procurado.
    pub fn chain(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.exe.as_str()).chain(self.ancestors.iter().map(String::as_str))
    }
}

/// Lê quem captura o microfone agora.
pub trait AudioActivity: Send {
    /// Os processos com sessão de captura ativa, sem repetir processo, ordenados por pid.
    fn mic_holders(&mut self) -> Result<Vec<MicHolder>, AudioError>;
}

/// A implementação do SO atual.
pub fn platform_activity() -> Box<dyn AudioActivity> {
    Box::new(imp::PlatformActivity)
}

/// Quantos pais seguir a partir do processo que captura.
#[cfg_attr(not(windows), allow(dead_code))]
const MAX_ANCESTORS: usize = 4;

/// Os ancestrais de `pid` numa tabela `pid → (pai, exe)`, parando num ciclo, num pai que não
/// existe mais ou em [`MAX_ANCESTORS`].
#[cfg_attr(not(windows), allow(dead_code))]
fn ancestors(pid: u32, table: &std::collections::HashMap<u32, (u32, String)>) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = vec![pid];
    let mut current = pid;
    while out.len() < MAX_ANCESTORS {
        let Some(&(parent, _)) = table.get(&current) else {
            break;
        };
        if parent == 0 || seen.contains(&parent) {
            break;
        }
        let Some((_, exe)) = table.get(&parent) else {
            break;
        };
        out.push(exe.clone());
        seen.push(parent);
        current = parent;
    }
    out
}

/// O nome do exe em minúsculas, sem o caminho.
#[cfg_attr(not(windows), allow(dead_code))]
fn exe_name(path: &str) -> String {
    path.rsplit(['\\', '/'])
        .next()
        .unwrap_or(path)
        .to_lowercase()
}

#[cfg(windows)]
mod imp {
    use std::collections::HashMap;

    use windows::core::Interface;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Media::Audio::{
        eCapture, AudioSessionStateActive, IAudioSessionControl2, IAudioSessionManager2,
        IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
    };
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    use super::{ancestors, exe_name, AudioActivity, MicHolder};
    use crate::AudioError;

    pub(super) struct PlatformActivity;

    impl AudioActivity for PlatformActivity {
        fn mic_holders(&mut self) -> Result<Vec<MicHolder>, AudioError> {
            let _com = Com::init()?;
            let mut pids = capture_pids()?;
            pids.sort_unstable();
            pids.dedup();
            if pids.is_empty() {
                return Ok(Vec::new());
            }
            let table = processes()?;
            Ok(pids
                .into_iter()
                .filter_map(|pid| {
                    let (_, exe) = table.get(&pid)?;
                    Some(MicHolder {
                        pid,
                        exe: exe.clone(),
                        ancestors: ancestors(pid, &table),
                    })
                })
                .collect())
        }
    }

    /// COM inicializado nesta thread enquanto o valor vive.
    struct Com {
        owned: bool,
    }

    impl Com {
        fn init() -> Result<Self, AudioError> {
            // SAFETY: sem ponteiro reservado; o par com `CoUninitialize` fica no `Drop`.
            let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            // `RPC_E_CHANGED_MODE`: a thread já tem outro apartment, e o COM serve assim mesmo.
            Ok(Self { owned: hr.is_ok() })
        }
    }

    impl Drop for Com {
        fn drop(&mut self) {
            if self.owned {
                // SAFETY: equilibra o `CoInitializeEx` que deu certo nesta mesma thread.
                unsafe { CoUninitialize() };
            }
        }
    }

    fn com_err(what: &str, e: windows::core::Error) -> AudioError {
        AudioError::Device(format!("{what}: {e}"))
    }

    /// Os pids das sessões de captura ativas em todos os microfones ativos.
    fn capture_pids() -> Result<Vec<u32>, AudioError> {
        // SAFETY: chamadas COM numa thread inicializada; as interfaces se soltam no `Drop`.
        unsafe {
            let devices: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                    .map_err(|e| com_err("MMDeviceEnumerator", e))?;
            let collection = devices
                .EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)
                .map_err(|e| com_err("EnumAudioEndpoints", e))?;
            let count = collection.GetCount().map_err(|e| com_err("GetCount", e))?;
            let mut pids = Vec::new();
            for i in 0..count {
                let Ok(device) = collection.Item(i) else {
                    continue;
                };
                let Ok(manager) = device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) else {
                    continue;
                };
                let Ok(sessions) = manager.GetSessionEnumerator() else {
                    continue;
                };
                let Ok(n) = sessions.GetCount() else {
                    continue;
                };
                for j in 0..n {
                    let Ok(control) = sessions.GetSession(j) else {
                        continue;
                    };
                    if control.GetState().ok() != Some(AudioSessionStateActive) {
                        continue;
                    }
                    let Ok(control2) = control.cast::<IAudioSessionControl2>() else {
                        continue;
                    };
                    // S_OK (0) marca a sessão de sons do sistema, que não é de um app.
                    if control2.IsSystemSoundsSession().0 == 0 {
                        continue;
                    }
                    if let Ok(pid) = control2.GetProcessId()
                        && pid != 0
                    {
                        pids.push(pid);
                    }
                }
            }
            Ok(pids)
        }
    }

    /// `pid → (pai, exe)` de todos os processos, por um snapshot do ToolHelp.
    fn processes() -> Result<HashMap<u32, (u32, String)>, AudioError> {
        // SAFETY: o handle do snapshot fecha antes de sair; a entrada leva o `dwSize` exigido.
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
                .map_err(|e| com_err("CreateToolhelp32Snapshot", e))?;
            let mut table = HashMap::new();
            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            let mut more = Process32FirstW(snapshot, &mut entry).is_ok();
            while more {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let exe = String::from_utf16_lossy(&entry.szExeFile[..len]);
                table.insert(
                    entry.th32ProcessID,
                    (entry.th32ParentProcessID, exe_name(&exe)),
                );
                more = Process32NextW(snapshot, &mut entry).is_ok();
            }
            let _ = CloseHandle(snapshot);
            Ok(table)
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::{AudioActivity, MicHolder};
    use crate::AudioError;

    pub(super) struct PlatformActivity;

    impl AudioActivity for PlatformActivity {
        fn mic_holders(&mut self) -> Result<Vec<MicHolder>, AudioError> {
            Err(AudioError::Unsupported("quem captura o microfone"))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn table(rows: &[(u32, u32, &str)]) -> HashMap<u32, (u32, String)> {
        rows.iter()
            .map(|&(pid, parent, exe)| (pid, (parent, exe.to_owned())))
            .collect()
    }

    #[test]
    fn ancestors_follow_parents_until_a_gap() {
        // explorer → ms-teams → msedgewebview2 (browser) → msedgewebview2 (áudio)
        let t = table(&[
            (4, 0, "explorer.exe"),
            (10, 4, "ms-teams.exe"),
            (20, 10, "msedgewebview2.exe"),
            (30, 20, "msedgewebview2.exe"),
            (40, 99, "orphan.exe"),
        ]);
        assert_eq!(
            ancestors(30, &t),
            ["msedgewebview2.exe", "ms-teams.exe", "explorer.exe"]
        );
        // O pai 99 já saiu: nada a seguir.
        assert!(ancestors(40, &t).is_empty());
        assert!(ancestors(4, &t).is_empty());
    }

    #[test]
    fn ancestors_stop_on_cycles_and_depth() {
        // O Windows reaproveita pids: um "pai" pode apontar de volta para o filho.
        let cycle = table(&[(1, 2, "a.exe"), (2, 1, "b.exe")]);
        assert_eq!(ancestors(1, &cycle), ["b.exe"]);

        let deep = table(&[
            (1, 2, "1.exe"),
            (2, 3, "2.exe"),
            (3, 4, "3.exe"),
            (4, 5, "4.exe"),
            (5, 6, "5.exe"),
            (6, 7, "6.exe"),
        ]);
        assert_eq!(ancestors(1, &deep).len(), MAX_ANCESTORS);
    }

    #[test]
    fn exe_names_lose_the_path_and_case() {
        assert_eq!(exe_name(r"C:\Program Files\Zoom\Zoom.exe"), "zoom.exe");
        assert_eq!(exe_name("MS-Teams.exe"), "ms-teams.exe");
        assert_eq!(exe_name("/usr/bin/firefox"), "firefox");
    }

    #[test]
    fn chain_starts_with_the_exe() {
        let holder = MicHolder {
            pid: 30,
            exe: "msedgewebview2.exe".into(),
            ancestors: vec!["ms-teams.exe".into()],
        };
        assert_eq!(
            holder.chain().collect::<Vec<_>>(),
            ["msedgewebview2.exe", "ms-teams.exe"]
        );
    }
}
