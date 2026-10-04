//! `fala-cli import`: converte um arquivo de áudio ou vídeo no WAV mono 48 kHz de uma sessão de
//! importação, pelo `ffmpeg` do PATH (`fala-media`).
//!
//! O stdout é só a tabela de resumo; progresso e erros vão para o stderr. Ctrl+C mata o processo
//! e pode deixar `<out>.part`, que a próxima execução sobrescreve.

use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::anyhow;
use clap::Args;
use fala_media::{CancelToken, MediaError, Progress, Tools};

use crate::record::{failed, input, print_table, Failure};

const COLUMNS: [&str; 4] = ["out", "sample_rate", "channels", "duration_s"];

#[derive(Args)]
pub struct ImportArgs {
    /// Arquivo de áudio ou vídeo (qualquer formato que o ffmpeg leia).
    file: PathBuf,
    /// WAV de saída (default: `<nome do arquivo>.fala.wav` no diretório atual). Nunca sobrescreve.
    #[arg(long)]
    out: Option<PathBuf>,
}

pub fn run(args: ImportArgs) -> Result<(), Failure> {
    let out = args.out.unwrap_or_else(|| default_out(&args.file));
    fala_media::check_paths(&args.file, &out).map_err(classify)?;
    let tools = Tools::locate().map_err(classify)?;
    let audio = fala_media::import(&tools, &args.file, &out, &CancelToken::new(), &mut |p| {
        log::info!("{}", progress_line(p))
    })
    .map_err(classify)?;
    print_table(
        &COLUMNS,
        &[
            audio.path.display().to_string(),
            audio.sample_rate.to_string(),
            audio.channels.to_string(),
            format!("{:.3}", audio.duration.as_secs_f64()),
        ],
    );
    Ok(())
}

/// `<stem>.fala.wav` no diretório atual.
fn default_out(file: &std::path::Path) -> PathBuf {
    let mut name = file
        .file_stem()
        .map(OsString::from)
        .unwrap_or_else(|| OsString::from("import"));
    name.push(".fala.wav");
    PathBuf::from(name)
}

/// Exit 2 para o que o usuário corrige no arquivo ou na saída; 1 para ferramenta e conversão.
fn classify(error: MediaError) -> Failure {
    let code_two = matches!(
        error,
        MediaError::InputNotFound(_)
            | MediaError::OutputExists(_)
            | MediaError::Unreadable { .. }
            | MediaError::NoAudioTrack(_)
            | MediaError::TooLong { .. }
    );
    let error = anyhow!(error);
    if code_two {
        input(error)
    } else {
        failed(error)
    }
}

fn progress_line(p: Progress) -> String {
    let processed = p.processed.as_secs_f64();
    match p.total.map(|t| t.as_secs_f64()).filter(|t| *t > 0.0) {
        Some(total) => format!(
            "import: {processed:.1} s / {total:.1} s ({:.0} %)",
            (processed / total * 100.0).min(100.0)
        ),
        None => format!("import: {processed:.1} s / ? s (? %)"),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn progress_line_formats_known_and_unknown_total() {
        let half = Duration::from_millis(500);
        assert_eq!(
            progress_line(Progress {
                processed: half,
                total: Some(Duration::from_secs(2)),
            }),
            "import: 0.5 s / 2.0 s (25 %)"
        );
        assert_eq!(
            progress_line(Progress {
                processed: half,
                total: None,
            }),
            "import: 0.5 s / ? s (? %)"
        );
    }
}
