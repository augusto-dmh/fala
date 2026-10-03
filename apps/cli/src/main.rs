//! `fala-cli`: o pipeline do Fala sem UI, para spikes, benchmark e uso headless.
//!
//! Depende só de `crates/` (nunca de `tauri`), então compila sem WebView (ADR-0002).
//! Os subcomandos são stubs no dia 1; cada um ganha implementação na fase 0 ou 1.

mod bench;
mod import;
mod record;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "fala-cli",
    version,
    about = "Fala sem UI: spikes, benchmark e uso headless"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Grava do microfone até Enter, transcreve e imprime o texto.
    Dictate,
    /// Grava microfone e áudio do sistema em dois canais e mede o drift entre eles.
    Record(record::RecordArgs),
    /// Transcreve um arquivo de áudio.
    Transcribe {
        /// Caminho do arquivo (WAV, Opus ou qualquer formato que o ffmpeg leia).
        file: PathBuf,
    },
    /// Mede WER e RTF de um modelo de ASR sobre cortes com referência.
    Bench(bench::BenchArgs),
    /// Converte um arquivo de áudio ou vídeo num WAV mono 48 kHz pelo ffmpeg do PATH.
    Import(import::ImportArgs),
}

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();
    let name = match cli.command {
        Command::Dictate => "dictate",
        Command::Record(args) => {
            return match record::run(args) {
                Ok(()) => ExitCode::SUCCESS,
                Err(failure) => {
                    log::error!("{:#}", failure.error);
                    ExitCode::from(failure.code)
                }
            };
        }
        Command::Transcribe { .. } => "transcribe",
        Command::Bench(args) => {
            return match bench::run(args) {
                Ok(()) => ExitCode::SUCCESS,
                Err(failure) => {
                    log::error!("{:#}", failure.error);
                    ExitCode::from(failure.code)
                }
            };
        }
        Command::Import(args) => {
            return match import::run(args) {
                Ok(()) => ExitCode::SUCCESS,
                Err(failure) => {
                    log::error!("{:#}", failure.error);
                    ExitCode::from(failure.code)
                }
            };
        }
    };
    log::error!("`{name}` ainda não foi implementado");
    ExitCode::FAILURE
}
