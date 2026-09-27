//! `fala-cli`: o pipeline do Fala sem UI, para spikes, benchmark e uso headless.
//!
//! Depende só de `crates/` (nunca de `tauri`), então compila sem WebView (ADR-0002).
//! Os subcomandos são stubs no dia 1; cada um ganha implementação na fase 0 ou 1.

use std::path::PathBuf;

use anyhow::bail;
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
    /// Grava microfone e áudio do sistema em dois canais.
    Record,
    /// Transcreve um arquivo de áudio.
    Transcribe {
        /// Caminho do arquivo (WAV, Opus ou qualquer formato que o ffmpeg leia).
        file: PathBuf,
    },
    /// Roda o benchmark de WER/RTF sobre o corpus de referência.
    Bench,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let name = match cli.command {
        Command::Dictate => "dictate",
        Command::Record => "record",
        Command::Transcribe { .. } => "transcribe",
        Command::Bench => "bench",
    };
    bail!("`{name}` ainda não foi implementado")
}
