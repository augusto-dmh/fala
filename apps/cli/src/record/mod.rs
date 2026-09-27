//! `fala-cli record`: grava mic (L) e sistema (R) num WAV estéreo e mede o drift entre os dois.
//!
//! Código de spike da fase 0; a captura de verdade é da fase 2, em `crates/audio`. O stdout é só
//! a tabela de resumo; progresso, avisos e erros vão para o stderr.

mod analyze;
mod capture;

use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use clap::Args;

#[derive(Args)]
pub struct RecordArgs {
    /// WAV de saída (48 kHz, estéreo, i16; L = mic, R = sistema).
    #[arg(long, required_unless_present = "analyze")]
    out: Option<PathBuf>,
    /// Duração: `<n>s`, `<n>m` ou `<n>h`.
    #[arg(long, value_parser = parse_duration, required_unless_present = "analyze")]
    duration: Option<Duration>,
    /// Sistema: `node.name` do sink no PipeWire (Linux) ou nome do dispositivo de saída (Windows).
    #[arg(long, required_unless_present = "analyze")]
    system: Option<String>,
    /// Parte do nome do microfone (default: a entrada padrão).
    #[arg(long)]
    mic: Option<String>,
    /// Não toca os cliques de sincronização.
    #[arg(long)]
    no_click: bool,
    /// Reescreve o cabeçalho do WAV a cada N segundos (default 10).
    #[arg(long)]
    flush_s: Option<u64>,
    /// Mede offset e drift entre os cliques de um WAV já gravado.
    #[arg(
        long,
        conflicts_with_all = ["out", "duration", "mic", "system", "no_click", "flush_s"]
    )]
    analyze: Option<PathBuf>,
    /// Limiar de onset em escala cheia (só com --analyze).
    #[arg(long, requires = "analyze", default_value_t = 0.1)]
    click_threshold: f32,
}

/// Falha com o código de saída do contrato: 1 captura ou análise, 2 entrada inválida.
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
    if let Some(wav) = &args.analyze {
        let row = analyze::analyze(wav, args.click_threshold)?;
        print_table(&analyze::COLUMNS, &row);
        return Ok(());
    }
    let (Some(out), Some(duration), Some(system)) = (args.out, args.duration, args.system) else {
        return Err(input(anyhow!(
            "--out, --duration e --system são obrigatórios"
        )));
    };
    capture::record(&capture::Options {
        out,
        duration,
        system,
        mic: args.mic,
        click: !args.no_click,
        flush: Duration::from_secs(args.flush_s.unwrap_or(10).max(1)),
    })
}

fn parse_duration(text: &str) -> Result<Duration, String> {
    let bad = || format!("`{text}`: use <n>s, <n>m ou <n>h, com n inteiro > 0");
    let unit = text.chars().last().ok_or_else(bad)?;
    let scale = match unit {
        's' => 1,
        'm' => 60,
        'h' => 3600,
        _ => return Err(bad()),
    };
    let digits = &text[..text.len() - 1];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    let n: u64 = digits.parse().map_err(|_| bad())?;
    if n == 0 {
        return Err(bad());
    }
    Ok(Duration::from_secs(n * scale))
}

/// Tabela Markdown de uma linha no stdout.
pub(crate) fn print_table(columns: &[&str], row: &[String]) {
    println!("| {} |", columns.join(" | "));
    println!("|{}", " ---: |".repeat(columns.len()));
    println!("| {} |", row.join(" | "));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_accepts_only_s_m_h() {
        assert_eq!(parse_duration("5s"), Ok(Duration::from_secs(5)));
        assert_eq!(parse_duration("5m"), Ok(Duration::from_secs(300)));
        assert_eq!(parse_duration("5h"), Ok(Duration::from_secs(18_000)));
        for bad in ["5min", "5", "1.5m", "0s", "-5s", "s", ""] {
            assert!(parse_duration(bad).is_err(), "{bad} accepted");
        }
    }
}
