//! `fala-cli bench`: WER e RTF de um modelo de ASR sobre um corpus de cortes com referência.
//!
//! O stdout é a legenda e a tabela Markdown, e só isso; diagnósticos vão para o stderr. Texto de
//! referência ou de hipótese só aparece nos arquivos de `--out` e em log `debug`.
//!
//! `fala-cli bench format` mede o pós-processamento (`format.rs`).

mod corpus;
mod engine;
pub mod format;
mod wer;

use std::fmt::Display;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Subcommand, ValueEnum};

use corpus::Cut;
use wer::Score;

#[derive(Args)]
pub struct BenchArgs {
    #[command(subcommand)]
    pub command: Option<BenchCommand>,
    /// Pasta com os cortes `<stem>.wav` (16 kHz, mono, PCM de 16 bits).
    #[arg(long, required = true)]
    cuts: Option<PathBuf>,
    /// Pasta com as referências `<stem>.txt`.
    #[arg(long, required = true)]
    refs: Option<PathBuf>,
    /// Engine que transcreve os cortes.
    #[arg(long, value_enum, required_unless_present = "hyp", requires = "model")]
    engine: Option<EngineKind>,
    /// Modelo: pasta do Parakeet ONNX ou arquivo GGUF/ggml.
    #[arg(long)]
    model: Option<PathBuf>,
    /// Pontua hipóteses externas (`<stem>.txt` e, opcional, `<stem>.wall_s`) em vez de uma engine.
    #[arg(long, conflicts_with_all = ["engine", "model", "threads", "device", "chunk_s", "language"])]
    hyp: Option<PathBuf>,
    /// Idioma pedido ao modelo (só gguf; default pt-BR), resolvido contra a lista que ele anuncia.
    #[arg(long)]
    language: Option<String>,
    /// Threads de CPU (só gguf; default: todas as disponíveis).
    #[arg(long)]
    threads: Option<usize>,
    /// Backend (só gguf; default cpu). `gpu` nunca cai para CPU.
    #[arg(long, value_enum)]
    device: Option<DeviceKind>,
    /// Transcreve em janelas consecutivas de N segundos (0 = arquivo inteiro).
    #[arg(long)]
    chunk_s: Option<u32>,
    /// Grava `<stem>.txt` e `<stem>.wall_s` de cada corte nesta pasta.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Rótulo livre para a legenda.
    #[arg(long)]
    tag: Option<String>,
}

#[derive(Subcommand)]
pub enum BenchCommand {
    /// Mede o pós-processamento contra um corpus JSONL de ditados já formatados.
    Format(format::FormatBenchArgs),
}

#[derive(Clone, Copy, ValueEnum)]
enum EngineKind {
    ParakeetOnnx,
    Gguf,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum DeviceKind {
    Cpu,
    Gpu,
}

/// Falha com o código de saída do contrato: 1 engine/backend, 2 entrada inválida.
pub struct Failure {
    pub code: u8,
    pub error: anyhow::Error,
}

/// `?` sem mapeamento explícito é falha de engine/backend (código 1).
impl<E: Into<anyhow::Error>> From<E> for Failure {
    fn from(error: E) -> Self {
        Failure {
            code: 1,
            error: error.into(),
        }
    }
}

fn input(error: anyhow::Error) -> Failure {
    Failure { code: 2, error }
}

fn engine_failure(error: anyhow::Error) -> Failure {
    Failure { code: 1, error }
}

pub fn run(args: BenchArgs) -> Result<(), Failure> {
    let gpu = args.device == Some(DeviceKind::Gpu);
    if matches!(args.engine, Some(EngineKind::ParakeetOnnx)) {
        if args.threads.is_some() {
            return Err(input(anyhow::anyhow!(
                "--threads: a engine parakeet-onnx não expõe essa opção (o ONNX Runtime usa as threads padrão)"
            )));
        }
        if gpu {
            return Err(input(anyhow::anyhow!(
                "--device gpu: a engine parakeet-onnx não expõe essa opção (só CPU)"
            )));
        }
        if args.language.is_some() {
            return Err(input(anyhow::anyhow!(
                "--language: a engine parakeet-onnx não expõe essa opção"
            )));
        }
    }

    // O clap exige os dois sem subcomando; `main` só chega aqui sem subcomando.
    let (Some(cuts), Some(refs)) = (&args.cuts, &args.refs) else {
        return Err(input(anyhow::anyhow!("--cuts e --refs são obrigatórios")));
    };
    let cuts = corpus::load(cuts, refs).map_err(input)?;
    let tag = args.tag.as_deref().unwrap_or("-");

    if let Some(dir) = &args.hyp {
        let hyps = corpus::load_hypotheses(dir, &cuts).map_err(input)?;
        let mut out = Table::start(format_args!(
            "engine=hyp model={} threads=- device=- load_s=- tag={tag}",
            basename(dir)
        ));
        for (cut, hyp) in cuts.iter().zip(&hyps) {
            out.row(cut, &hyp.text, hyp.wall_s);
        }
        out.total();
        return Ok(());
    }

    let (Some(kind), Some(model)) = (args.engine, args.model.as_deref()) else {
        return Err(input(anyhow::anyhow!(
            "--engine e --model são obrigatórios sem --hyp"
        )));
    };
    if let Some(dir) = &args.out {
        fs::create_dir_all(dir)
            .with_context(|| format!("não consegui criar --out {}", dir.display()))
            .map_err(input)?;
    }
    let loaded = match kind {
        EngineKind::ParakeetOnnx => engine::load_parakeet(model).map_err(engine_failure)?,
        EngineKind::Gguf => {
            let threads = match args.threads {
                Some(n) => n,
                None => std::thread::available_parallelism().map_or(1, |n| n.get()),
            };
            let language = args.language.as_deref().unwrap_or(engine::DEFAULT_LANGUAGE);
            engine::load_gguf(model, threads, gpu, language)?
        }
    };

    let engine_name = match kind {
        EngineKind::ParakeetOnnx => "parakeet-onnx",
        EngineKind::Gguf => "gguf",
    };
    let threads = loaded
        .threads
        .map_or_else(|| "-".to_owned(), |n| n.to_string());
    let mut out = Table::start(format_args!(
        "engine={engine_name} model={} threads={threads} device={} load_s={:.2} tag={tag}",
        basename(model),
        loaded.device,
        loaded.load_s
    ));
    let mut engine = loaded.engine;
    let chunk_samples = args.chunk_s.unwrap_or(0) as usize * corpus::SAMPLE_RATE as usize;
    for cut in &cuts {
        let (text, wall_s) = corpus::read_samples(&cut.wav)
            .and_then(|samples| engine.transcribe_cut(&samples, chunk_samples))
            .with_context(|| format!("corte `{}`", cut.stem))
            .map_err(engine_failure)?;
        log::debug!("{}: hipótese: {text}", cut.stem);
        if let Some(dir) = &args.out {
            write_hypothesis(dir, &cut.stem, &text, wall_s).map_err(engine_failure)?;
        }
        out.row(cut, &text, Some(wall_s));
    }
    out.total();
    Ok(())
}

fn write_hypothesis(dir: &Path, stem: &str, text: &str, wall_s: f64) -> Result<()> {
    fs::write(dir.join(format!("{stem}.txt")), text)?;
    fs::write(dir.join(format!("{stem}.wall_s")), format!("{wall_s:.3}\n"))?;
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
