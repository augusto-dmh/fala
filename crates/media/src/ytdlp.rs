//! Argumentos e saída do `yt-dlp`.
//!
//! Os argumentos vão em vetor para `Command`, nunca por shell, e a URL vem depois de `--`, então
//! nunca vira opção. O stdout traz só o que os `--print` e o `--progress-template` pedem.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::MediaError;

const TITLE: &str = "title:";
const FILEPATH: &str = "filepath:";
const PROGRESS: &str = "progress:";

/// Aceita só `http://` e `https://` com host, sem espaço nem caractere de controle.
pub fn check_url(url: &str) -> Result<(), MediaError> {
    let invalid = || MediaError::InvalidUrl(url.to_owned());
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(invalid());
    }
    let rest = ["http://", "https://"]
        .iter()
        .find_map(|scheme| {
            url.get(..scheme.len())
                .filter(|head| head.eq_ignore_ascii_case(scheme))
                .map(|_| &url[scheme.len()..])
        })
        .ok_or_else(invalid)?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if host.is_empty() {
        return Err(invalid());
    }
    Ok(())
}

/// Baixa só a melhor trilha de áudio (ou o melhor arquivo, sem trilha separada) como
/// `<dir>/source.<ext>`, só o primeiro item de uma playlist, e imprime em UTF-8 o título e o
/// caminho final.
pub(crate) fn ytdlp_args(url: &str, dir: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = [
        "--no-playlist",
        "--playlist-items",
        "1",
        "--encoding",
        "utf-8",
        "--format",
        "bestaudio/best",
        "--no-simulate",
        "--print",
        "before_dl:title:%(title)s",
        "--print",
        "after_move:filepath:%(filepath)s",
        "--progress",
        "--newline",
        "--progress-template",
        "download:progress:%(progress.downloaded_bytes)s:%(progress.total_bytes)s:%(progress.total_bytes_estimate)s",
        "--paths",
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    args.push(dir.as_os_str().to_owned());
    for arg in ["--output", "source.%(ext)s", "--", url] {
        args.push(arg.into());
    }
    args
}

/// Bytes baixados e o total, quando o servidor o informa (ou o yt-dlp o estima).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadProgress {
    pub done_bytes: u64,
    pub total_bytes: Option<u64>,
}

/// O que o stdout do yt-dlp disse até aqui.
#[derive(Debug, Default)]
pub(crate) struct YtDlpOutput {
    pub title: Option<String>,
    pub filepath: Option<PathBuf>,
}

fn bytes(value: &str) -> Option<u64> {
    value
        .parse::<u64>()
        .ok()
        .or_else(|| value.parse::<f64>().ok().map(|b| b.max(0.0) as u64))
}

impl YtDlpOutput {
    /// Guarda o título e o caminho; devolve o avanço quando a linha é de progresso.
    pub fn feed(&mut self, line: &str) -> Option<DownloadProgress> {
        let line = line.trim_end_matches(['\r', '\n']);
        if let Some(title) = line.strip_prefix(TITLE) {
            self.title = Some(title.trim().to_owned());
        } else if let Some(path) = line.strip_prefix(FILEPATH) {
            self.filepath = Some(PathBuf::from(path.trim()));
        } else if let Some(progress) = line.strip_prefix(PROGRESS) {
            let mut fields = progress.split(':').map(str::trim);
            let done_bytes = bytes(fields.next()?)?;
            let total = fields.next().and_then(bytes);
            let estimate = fields.next().and_then(bytes);
            return Some(DownloadProgress {
                done_bytes,
                total_bytes: total.or(estimate).filter(|t| *t > 0),
            });
        }
        None
    }
}
