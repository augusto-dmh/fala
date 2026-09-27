//! `fala-cli record --analyze`: mede offset e drift entre os cliques dos dois canais de um WAV
//! estéreo (L = mic, R = sistema). A gravação chega na mudança seguinte.
//!
//! Código de spike da fase 0; a captura de verdade é da fase 2, em `crates/audio`. O stdout é só
//! a tabela; avisos e erros vão para o stderr.

mod analyze;

use std::path::PathBuf;

use clap::Args;

#[derive(Args)]
pub struct RecordArgs {
    /// Mede offset e drift entre os cliques de um WAV já gravado.
    #[arg(long)]
    analyze: PathBuf,
    /// Limiar de onset em escala cheia (só com --analyze).
    #[arg(long, default_value_t = 0.1)]
    click_threshold: f32,
}

/// Falha com o código de saída do contrato: 1 análise, 2 entrada inválida.
pub struct Failure {
    pub code: u8,
    pub error: anyhow::Error,
}

pub(crate) fn input(error: anyhow::Error) -> Failure {
    Failure { code: 2, error }
}

pub(crate) fn failed(error: anyhow::Error) -> Failure {
    Failure { code: 1, error }
}

pub fn run(args: RecordArgs) -> Result<(), Failure> {
    let row = analyze::analyze(&args.analyze, args.click_threshold)?;
    print_table(&analyze::COLUMNS, &row);
    Ok(())
}

/// Tabela Markdown de uma linha no stdout.
pub(crate) fn print_table(columns: &[&str], row: &[String]) {
    println!("| {} |", columns.join(" | "));
    println!("|{}", " ---: |".repeat(columns.len()));
    println!("| {} |", row.join(" | "));
}
