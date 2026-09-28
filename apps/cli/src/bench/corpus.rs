//! Leitor de corpus: `<cuts>/*.wav` em ordem de nome, cada um com `<refs>/<stem>.txt`.
//! Tudo o que faria o número mentir é rejeitado aqui, antes de carregar qualquer modelo.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use super::wer;

pub const SAMPLE_RATE: u32 = 16_000;

pub struct Cut {
    pub stem: String,
    pub wav: PathBuf,
    pub audio_s: f64,
    pub reference: Vec<String>,
}

/// Lista e valida o corpus. Todo erro aqui é entrada inválida (exit 2).
pub fn load(cuts_dir: &Path, refs_dir: &Path) -> Result<Vec<Cut>> {
    let mut wavs: Vec<PathBuf> = fs::read_dir(cuts_dir)
        .with_context(|| format!("não consegui ler --cuts {}", cuts_dir.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "wav"))
        .collect();
    wavs.sort_by(|a, b| {
        a.file_name()
            .map(|n| n.as_encoded_bytes())
            .cmp(&b.file_name().map(|n| n.as_encoded_bytes()))
    });
    if wavs.is_empty() {
        bail!("nenhum arquivo .wav em {}", cuts_dir.display());
    }

    wavs.into_iter()
        .map(|wav| {
            let stem = wav
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let audio_s = check_wav(&wav)?;
            let ref_path = refs_dir.join(format!("{stem}.txt"));
            if !ref_path.is_file() {
                bail!(
                    "o corte `{stem}` não tem referência: {} não existe",
                    ref_path.display()
                );
            }
            let text = fs::read_to_string(&ref_path)
                .with_context(|| format!("não consegui ler {}", ref_path.display()))?;
            let reference = wer::normalize(&text);
            if reference.is_empty() {
                bail!(
                    "a referência {} tem 0 palavras depois da normalização",
                    ref_path.display()
                );
            }
            Ok(Cut {
                stem,
                wav,
                audio_s,
                reference,
            })
        })
        .collect()
}

/// Exige 16 kHz, mono, PCM inteiro de 16 bits; devolve a duração em segundos.
fn check_wav(path: &Path) -> Result<f64> {
    let reader = hound::WavReader::open(path)
        .with_context(|| format!("{} não é um WAV legível", path.display()))?;
    let spec = reader.spec();
    if spec.sample_rate != SAMPLE_RATE
        || spec.channels != 1
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
    {
        let format = match spec.sample_format {
            hound::SampleFormat::Int => "int",
            hound::SampleFormat::Float => "float",
        };
        bail!(
            "{}: esperado 16000 Hz, 1 canal, PCM int de 16 bits; encontrado {} Hz, {} canal(is), {} bits {format}",
            path.display(),
            spec.sample_rate,
            spec.channels,
            spec.bits_per_sample
        );
    }
    Ok(f64::from(reader.duration()) / f64::from(SAMPLE_RATE))
}

/// Amostras f32 em [-1, 1] de um corte já validado.
pub fn read_samples(path: &Path) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path)?;
    reader
        .samples::<i16>()
        .map(|s| Ok(f32::from(s?) / 32768.0))
        .collect()
}

/// Hipótese externa (door 2): `<dir>/<stem>.txt` e, opcional, `<dir>/<stem>.wall_s`.
pub struct Hypothesis {
    pub text: String,
    pub wall_s: Option<f64>,
}

/// Exige `<dir>/<stem>.txt` para todos os cortes antes de pontuar qualquer um.
pub fn load_hypotheses(dir: &Path, cuts: &[Cut]) -> Result<Vec<Hypothesis>> {
    cuts.iter()
        .map(|cut| {
            let txt = dir.join(format!("{}.txt", cut.stem));
            if !txt.is_file() {
                bail!(
                    "--hyp não tem hipótese para o corte `{}`: {} não existe",
                    cut.stem,
                    txt.display()
                );
            }
            let text = fs::read_to_string(&txt)
                .with_context(|| format!("não consegui ler {}", txt.display()))?;
            let wall_path = dir.join(format!("{}.wall_s", cut.stem));
            let wall_s =
                if wall_path.is_file() {
                    let raw = fs::read_to_string(&wall_path)?;
                    Some(raw.trim().parse::<f64>().with_context(|| {
                        format!("{} não é um número decimal", wall_path.display())
                    })?)
                } else {
                    None
                };
            Ok(Hypothesis { text, wall_s })
        })
        .collect()
}
