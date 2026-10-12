use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use crate::MediaError;

/// Como os processos filhos são iniciados e parados. O padrão não muda o `Command` e mata só o
/// processo; quem roda no Windows injeta o resto (esconder o console, matar a árvore), porque
/// este crate não tem `cfg` de plataforma (ADR-0007).
#[derive(Debug, Clone, Copy)]
pub struct ProcessHooks {
    pub prepare: fn(&mut Command),
    pub stop: fn(&mut Child),
}

impl Default for ProcessHooks {
    fn default() -> Self {
        Self {
            prepare: |_| {},
            stop: |child| {
                let _ = child.kill();
            },
        }
    }
}

/// Caminhos do `ffmpeg`, do `ffprobe` e, se houver, do `yt-dlp` achados no PATH.
///
/// Nada é baixado nem embutido: a importação exige os dois primeiros no PATH por enquanto, e a
/// de URL também o `yt-dlp` (decisão D3 do pitch da fase 2, opção c). Toda distribuição do
/// ffmpeg traz o ffprobe junto.
#[derive(Debug, Clone)]
pub struct Tools {
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
    yt_dlp: Option<PathBuf>,
    hooks: ProcessHooks,
}

impl Tools {
    /// Procura as duas ferramentas no PATH do processo.
    pub fn locate() -> Result<Self, MediaError> {
        Self::locate_in(&std::env::var_os("PATH").unwrap_or_default())
    }

    /// Procura as duas ferramentas nos diretórios de `path`, no formato da variável PATH.
    /// O primeiro diretório que tem o arquivo vence; entradas vazias são ignoradas, nunca o
    /// diretório atual.
    pub fn locate_in(path: &OsStr) -> Result<Self, MediaError> {
        let ffmpeg = find(path, "ffmpeg").ok_or(MediaError::ToolNotFound { tool: "ffmpeg" })?;
        let ffprobe = find(path, "ffprobe").ok_or(MediaError::ToolNotFound { tool: "ffprobe" })?;
        Ok(Self {
            ffmpeg,
            ffprobe,
            yt_dlp: find(path, "yt-dlp"),
            hooks: ProcessHooks::default(),
        })
    }

    pub fn with_hooks(self, hooks: ProcessHooks) -> Self {
        Self { hooks, ..self }
    }

    pub fn ffmpeg(&self) -> &Path {
        &self.ffmpeg
    }

    pub fn ffprobe(&self) -> &Path {
        &self.ffprobe
    }

    /// O `yt-dlp`, ou o erro que a importação de URL mostra sem ele.
    pub fn yt_dlp(&self) -> Result<&Path, MediaError> {
        self.yt_dlp
            .as_deref()
            .ok_or(MediaError::ToolNotFound { tool: "yt-dlp" })
    }

    pub fn hooks(&self) -> ProcessHooks {
        self.hooks
    }
}

fn find(path: &OsStr, name: &str) -> Option<PathBuf> {
    let file = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    std::env::split_paths(path)
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(|dir| dir.join(&file))
        .find(|candidate| candidate.is_file())
}
