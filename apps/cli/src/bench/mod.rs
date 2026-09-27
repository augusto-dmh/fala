//! `fala-cli bench`: WER e RTF de um modelo de ASR sobre um corpus de cortes com referência.
//!
//! O stdout é a legenda e a tabela Markdown, e só isso; diagnósticos vão para o stderr. Texto de
//! referência ou de hipótese nunca vai para o stdout nem para o log acima de `debug`.

mod corpus;
mod wer;

use std::fmt::Display;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Args;

use corpus::Cut;
use wer::Score;

#[derive(Args)]
pub struct BenchArgs {
    /// Pasta com os cortes `<stem>.wav` (16 kHz, mono, PCM de 16 bits).
    #[arg(long)]
    cuts: PathBuf,
    /// Pasta com as referências `<stem>.txt`.
    #[arg(long)]
    refs: PathBuf,
    /// Pontua hipóteses externas (`<stem>.txt` e, opcional, `<stem>.wall_s`).
    #[arg(long)]
    hyp: PathBuf,
    /// Rótulo livre para a legenda.
    #[arg(long)]
    tag: Option<String>,
}

/// Falha com o código de saída do contrato: 2 entrada inválida.
pub struct Failure {
    pub code: u8,
    pub error: anyhow::Error,
}

fn input(error: anyhow::Error) -> Failure {
    Failure { code: 2, error }
}

pub fn run(args: BenchArgs) -> Result<(), Failure> {
    let cuts = corpus::load(&args.cuts, &args.refs).map_err(input)?;
    let tag = args.tag.as_deref().unwrap_or("-");
    let hyps = corpus::load_hypotheses(&args.hyp, &cuts).map_err(input)?;
    let mut out = Table::start(format_args!(
        "engine=hyp model={} threads=- device=- load_s=- tag={tag}",
        basename(&args.hyp)
    ));
    for (cut, hyp) in cuts.iter().zip(&hyps) {
        out.row(cut, &hyp.text, hyp.wall_s);
    }
    out.total();
    Ok(())
}

fn basename(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Legenda, cabeçalho e linhas no stdout, com flush por linha para mostrar progresso.
struct Table {
    score: Score,
    audio_s: f64,
    wall_s: Option<f64>,
}

impl Table {
    fn start(legend: impl Display) -> Self {
        emit(format_args!("{legend}"));
        emit(format_args!(
            "| cut | audio_s | wall_s | rtf | wer_% | sub | del | ins | ref_words |"
        ));
        emit(format_args!(
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"
        ));
        Table {
            score: Score::default(),
            audio_s: 0.0,
            wall_s: Some(0.0),
        }
    }

    fn row(&mut self, cut: &Cut, hypothesis: &str, wall_s: Option<f64>) {
        let score = wer::score(&cut.reference, &wer::normalize(hypothesis));
        self.score.add(&score);
        self.audio_s += cut.audio_s;
        self.wall_s = self.wall_s.zip(wall_s).map(|(a, b)| a + b);
        line(&cut.stem, cut.audio_s, wall_s, &score);
    }

    fn total(self) {
        line("total", self.audio_s, self.wall_s, &self.score);
    }
}

fn line(name: &str, audio_s: f64, wall_s: Option<f64>, s: &Score) {
    let (wall, rtf) = match wall_s {
        Some(w) => (format!("{w:.2}"), format!("{:.3}", w / audio_s)),
        None => ("-".to_owned(), "-".to_owned()),
    };
    emit(format_args!(
        "| {name} | {audio_s:.2} | {wall} | {rtf} | {} | {} | {} | {} | {} |",
        s.wer_pct(),
        s.sub,
        s.del,
        s.ins,
        s.ref_words
    ));
}

fn emit(line: std::fmt::Arguments) {
    let mut stdout = std::io::stdout().lock();
    // Um stdout fechado (`| head`) não é erro do benchmark; o resto segue no stderr.
    let _ = writeln!(stdout, "{line}").and_then(|()| stdout.flush());
}
