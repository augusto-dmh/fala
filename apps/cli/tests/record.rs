//! `fala-cli record` pela fronteira: roda o binário e confere exit code, stdout, stderr e o WAV.

// `allow-unwrap-in-tests` não cobre os helpers de um crate de teste de integração.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const ANALYZE_HEADER: &str = "| onset_mic_start_s | onset_sys_start_s | offset_start_ms | onset_mic_end_s | onset_sys_end_s | offset_end_ms | drift_ms | drift_ppm |";

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("record")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn fala(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fala-cli"))
        .args(args)
        .output()
        .unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// Colunas da linha de dados de uma tabela de uma linha.
fn row(out: &str) -> Vec<String> {
    out.lines()
        .nth(2)
        .unwrap()
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_owned())
        .collect()
}

/// WAV estéreo i16 de `seconds` em silêncio, com pulsos (canal, segundo) de amplitude 0.5.
fn stereo_clicks(path: &Path, seconds: u32, pulses: &[(usize, f64)]) {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let frames = seconds as usize * 48_000;
    let mut v = vec![0i16; frames * 2];
    for &(channel, at) in pulses {
        let f = (at * 48_000.0).round() as usize;
        for k in 0..48 {
            v[(f + k) * 2 + channel] = 16_384;
        }
    }
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for s in v {
        w.write_sample(s).unwrap();
    }
    w.finalize().unwrap();
}

// ---- S3 ----

#[test]
fn analyze_prints_offsets_and_drift() {
    let dir = scratch("analyze");
    let wav = dir.join("clicks.wav");
    stereo_clicks(
        &wav,
        40,
        &[(0, 2.000), (1, 2.010), (0, 38.000), (1, 38.030)],
    );
    let o = fala(&["record", "--analyze", wav.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let out = stdout(&o);
    assert_eq!(out.lines().count(), 3, "{out}");
    assert_eq!(out.lines().next().unwrap(), ANALYZE_HEADER);
    assert_eq!(
        row(&out),
        ["2.000", "2.010", "10.0", "38.000", "38.030", "30.0", "20.0", "555.6"]
    );
}

#[test]
fn missing_onset_exits_1_naming_window() {
    let dir = scratch("missing_onset");
    let wav = dir.join("clicks.wav");
    stereo_clicks(&wav, 40, &[(0, 2.000), (1, 2.010), (0, 38.000)]);
    let o = fala(&["record", "--analyze", wav.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    let err = stderr(&o);
    assert!(
        err.contains("sys/end") && err.contains("pico 0.000"),
        "{err}"
    );
}

#[test]
fn analyze_rejects_wrong_spec() {
    let dir = scratch("wrong_spec");
    for (i, (channels, bits, format, cited)) in [
        (
            1u16,
            16u16,
            hound::SampleFormat::Int,
            "1 canal(is), 16 bits int",
        ),
        (
            2,
            32,
            hound::SampleFormat::Float,
            "2 canal(is), 32 bits float",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let wav = dir.join(format!("{i}.wav"));
        let spec = hound::WavSpec {
            channels,
            sample_rate: 48_000,
            bits_per_sample: bits,
            sample_format: format,
        };
        let mut w = hound::WavWriter::create(&wav, spec).unwrap();
        for _ in 0..4800 * channels {
            match format {
                hound::SampleFormat::Int => w.write_sample(0i16).unwrap(),
                hound::SampleFormat::Float => w.write_sample(0f32).unwrap(),
            }
        }
        w.finalize().unwrap();
        let o = fala(&["record", "--analyze", wav.to_str().unwrap()]);
        assert_eq!(o.status.code(), Some(2), "{cited}: {}", stderr(&o));
        assert!(stderr(&o).contains(cited), "{cited}: {}", stderr(&o));
    }
}

// ---- S4 ----
