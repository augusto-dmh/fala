//! `fala-cli`: o pipeline do Fala sem UI, para spikes, benchmark e uso headless.
//!
//! Depende só de `crates/` (nunca de `tauri`), então compila sem WebView (ADR-0002).
//! Os subcomandos são stubs no dia 1; cada um ganha implementação na fase 0 ou 1.

mod bench;
mod dictate;
mod format;
mod history;
mod import;
mod key;
mod mcp;
mod meeting;
mod record;

use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use fala_secrets::KeyringStore;

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
    /// Dita pelo microfone (Enter começa, Enter termina) ou por um WAV e imprime o texto.
    Dictate(dictate::DictateArgs),
    /// Grava uma reunião: mic e áudio do sistema num WAV estéreo até Enter.
    Meeting(meeting::MeetingArgs),
    /// Grava microfone e áudio do sistema em dois canais e mede o drift entre eles.
    Record(record::RecordArgs),
    /// Transcreve um arquivo de áudio.
    Transcribe {
        /// Caminho do arquivo (WAV, Opus ou qualquer formato que o ffmpeg leia).
        file: PathBuf,
    },
    /// Mede WER e RTF de um modelo de ASR sobre cortes com referência.
    ///
    /// `bench format` mede o pós-processamento contra um corpus de ditados formatados.
    #[command(args_conflicts_with_subcommands = true, subcommand_negates_reqs = true)]
    Bench(bench::BenchArgs),
    /// Guarda, consulta e apaga chaves de API no keyring do SO.
    Key(key::KeyArgs),
    /// Formata o texto do stdin com as regras e, com `--llm`, o Gemini.
    Format(format::FormatArgs),
    /// Histórico de ditado: gravar, buscar, desfazer e reaplicar a edição.
    History(history::HistoryArgs),
    /// Reconstrói o banco do histórico a partir dos `.md` de `Ditados/`.
    Reindex(history::DirArgs),
    /// Converte um arquivo de áudio ou vídeo num WAV mono 48 kHz pelo ffmpeg do PATH.
    Import(import::ImportArgs),
    /// Serve o histórico a assistentes de IA por MCP (stdio, só leitura).
    ///
    /// Desligado até `<data-dir>/mcp.toml` ter `enabled = true`. Ditados sensíveis nunca saem.
    Mcp(mcp::McpArgs),
}

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();
    let name = match cli.command {
        Command::Dictate(args) => {
            return match dictate::run(args) {
                Ok(()) => ExitCode::SUCCESS,
                Err(failure) => {
                    log::error!("{:#}", failure.error);
                    ExitCode::from(failure.code)
                }
            };
        }
        Command::Meeting(args) => {
            return match meeting::run(args) {
                Ok(()) => ExitCode::SUCCESS,
                Err(failure) => {
                    log::error!("{:#}", failure.error);
                    ExitCode::from(failure.code)
                }
            };
        }
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
        Command::Key(args) => {
            return ExitCode::from(key::run(
                args,
                &KeyringStore,
                &mut io::stdin().lock(),
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            ));
        }
        Command::Format(args) => {
            return ExitCode::from(format::run(
                args,
                &KeyringStore,
                &mut io::stdin().lock(),
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            ));
        }
        Command::Bench(bench::BenchArgs {
            command: Some(bench::BenchCommand::Format(args)),
            ..
        }) => {
            return ExitCode::from(bench::format::run(
                args,
                &KeyringStore,
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            ));
        }
        Command::Bench(args) => {
            return match bench::run(args) {
                Ok(()) => ExitCode::SUCCESS,
                Err(failure) => {
                    log::error!("{:#}", failure.error);
                    ExitCode::from(failure.code)
                }
            };
        }
        Command::History(args) => return history_exit(history::run(args)),
        Command::Reindex(dirs) => return history_exit(history::reindex(dirs)),
        Command::Import(args) => {
            return match import::run(args) {
                Ok(()) => ExitCode::SUCCESS,
                Err(failure) => {
                    log::error!("{:#}", failure.error);
                    ExitCode::from(failure.code)
                }
            };
        }
        Command::Mcp(args) => return history_exit(mcp::run(args)),
    };
    log::error!("`{name}` ainda não foi implementado");
    ExitCode::FAILURE
}

fn history_exit(result: Result<(), history::Failure>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            log::error!("{:#}", failure.error);
            ExitCode::from(failure.code)
        }
    }
}
