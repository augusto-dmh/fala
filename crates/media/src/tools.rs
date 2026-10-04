use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::MediaError;

/// Caminhos do `ffmpeg` e do `ffprobe` achados no PATH.
///
/// Nada é baixado nem embutido: a importação exige os dois no PATH por enquanto (decisão D3 do
/// pitch da fase 2, opção c). Toda distribuição do ffmpeg traz o ffprobe junto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tools {
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
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
        Ok(Self { ffmpeg, ffprobe })
    }

    pub fn ffmpeg(&self) -> &Path {
        &self.ffmpeg
    }

    pub fn ffprobe(&self) -> &Path {
        &self.ffprobe
    }
}

fn find(path: &OsStr, name: &str) -> Option<PathBuf> {
    let file = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    std::env::split_paths(path)
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(|dir| dir.join(&file))
        .find(|candidate| candidate.is_file())
}
