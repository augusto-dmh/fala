//! `fala-cli record` pela fronteira: roda o binário e confere exit code, stdout, stderr e o WAV.
//!
//! Os testes `#[ignore]` gravam de verdade: precisam do PipeWire desta máquina, alto-falante com
//! volume ≥ 50 %, nada mais tocando, e do `node.name` do sink em `FALA_TEST_SINK` (falham se faltar).

// `allow-unwrap-in-tests` não cobre os helpers de um crate de teste de integração.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

const SUMMARY_HEADER: &str = "| rate | wall_s | mic_frames | sys_frames | mic_ppm | sys_ppm | rel_drift_ms | dropped_mic | dropped_sys | stream_errors | click_1_s | click_2_s | mic_peak | mic_rms_dbfs | sys_peak | sys_rms_dbfs |";
const ANALYZE_HEADER: &str = "| onset_mic_start_s | onset_sys_start_s | offset_start_ms | onset_mic_end_s | onset_sys_end_s | offset_end_ms | drift_ms | drift_ppm |";

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("record")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn sink() -> String {
    std::env::var("FALA_TEST_SINK").expect("FALA_TEST_SINK is not set")
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

fn record(dir: &Path, seconds: &str, extra: &[&str]) -> (Output, PathBuf) {
    let wav = dir.join("t.wav");
    let sink = sink();
    let mut args = vec![
        "record",
        "--system",
        &sink,
        "--duration",
        seconds,
        "--out",
        wav.to_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    (fala(&args), wav)
}

// ---- S1 ----

#[test]
#[ignore = "records from this machine's PipeWire; needs FALA_TEST_SINK"]
fn records_stereo_48k_wav() {
    let dir = scratch("stereo");
    let (o, wav) = record(&dir, "10s", &[]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let spec = hound::WavReader::open(&wav).unwrap().spec();
    assert_eq!(
        (spec.sample_rate, spec.channels, spec.bits_per_sample),
        (48_000, 2, 16)
    );
    assert_eq!(spec.sample_format, hound::SampleFormat::Int);
    let out = stdout(&o);
    assert_eq!(out.lines().count(), 3, "{out}");
    assert_eq!(out.lines().next().unwrap(), SUMMARY_HEADER);
    assert_eq!(row(&out)[0], "48000");
}

#[test]
#[ignore = "records from this machine's PipeWire; needs FALA_TEST_SINK"]
fn sigkill_leaves_readable_wav() {
    let dir = scratch("sigkill");
    let wav = dir.join("t.wav");
    let sink = sink();
    let mut child = Command::new(env!("CARGO_BIN_EXE_fala-cli"))
        .args([
            "record",
            "--system",
            &sink,
            "--duration",
            "20s",
            "--flush-s",
            "10",
            "--no-click",
            "--out",
            wav.to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_secs(15));
    child.kill().unwrap(); // SIGKILL no Unix
    child.wait().unwrap();
    let reader = hound::WavReader::open(&wav).unwrap();
    assert!(
        reader.duration() >= 480_000,
        "only {} frames readable",
        reader.duration()
    );
}

#[test]
#[ignore = "records from this machine's PipeWire for 70 s; needs FALA_TEST_SINK"]
fn progress_once_per_minute() {
    let dir = scratch("progress");
    let (o, _) = record(&dir, "70s", &["--no-click"]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let err = stderr(&o);
    let progress: Vec<&str> = err
        .lines()
        .filter(|l| l.contains("mic_frames=") && l.contains("sys_frames="))
        .collect();
    assert_eq!(progress.len(), 1, "{err}");
    assert!(progress[0].starts_with("60 s:"), "{}", progress[0]);
}

#[test]
fn unknown_mic_exits_2_listing_inputs() {
    let dir = scratch("unknown_mic");
    let o = fala(&[
        "record",
        "--system",
        "x",
        "--duration",
        "5s",
        "--no-click",
        "--mic",
        "nenhum-microfone-tem-este-nome",
        "--out",
        dir.join("t.wav").to_str().unwrap(),
    ]);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(
        stderr(&o).contains("entradas disponíveis:"),
        "{}",
        stderr(&o)
    );
    assert_eq!(stdout(&o), "");
}

// ---- S2 ----

#[test]
#[ignore = "records from this machine's PipeWire; needs FALA_TEST_SINK"]
fn clicks_at_2s_and_before_end() {
    let dir = scratch("clicks");
    let (o, wav) = record(&dir, "10s", &[]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let r = row(&stdout(&o));
    let (c1, c2): (f64, f64) = (r[10].parse().unwrap(), r[11].parse().unwrap());
    assert!((2.0..=2.1).contains(&c1), "click_1_s {c1}");
    assert!((8.0..=8.1).contains(&c2), "click_2_s {c2}");
    let a = fala(&["record", "--analyze", wav.to_str().unwrap()]);
    assert_eq!(a.status.code(), Some(0), "{}", stderr(&a));
    let ar = row(&stdout(&a));
    let (sys_start, sys_end): (f64, f64) = (ar[1].parse().unwrap(), ar[4].parse().unwrap());
    assert!(
        (sys_start - c1).abs() <= 0.5,
        "sys onset {sys_start} vs click {c1}"
    );
    assert!(
        (sys_end - c2).abs() <= 0.5,
        "sys onset {sys_end} vs click {c2}"
    );
}

#[test]
fn click_output_failure_exits_2() {
    let dir = scratch("no_output");
    let wav = dir.join("t.wav");
    let o = Command::new(env!("CARGO_BIN_EXE_fala-cli"))
        .args([
            "record",
            "--system",
            "x",
            "--duration",
            "5s",
            "--out",
            wav.to_str().unwrap(),
        ])
        .env("ALSA_CONFIG_PATH", "/nonexistent/alsa.conf")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(stderr(&o).contains("--no-click"), "{}", stderr(&o));
    assert!(!wav.exists(), "recording started");
}

#[test]
#[ignore = "records from this machine's PipeWire; needs FALA_TEST_SINK"]
fn no_click_prints_dashes() {
    let dir = scratch("no_click");
    let (o, _) = record(&dir, "3s", &["--no-click"]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let r = row(&stdout(&o));
    assert_eq!((r[10].as_str(), r[11].as_str()), ("-", "-"));
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

#[test]
fn analyze_rejects_recording_flags() {
    let flags: [&[&str]; 6] = [
        &["--out", "x.wav"],
        &["--duration", "5s"],
        &["--mic", "m"],
        &["--system", "s"],
        &["--no-click"],
        &["--flush-s", "5"],
    ];
    for flag in flags {
        let mut args = vec!["record", "--analyze", "x.wav"];
        args.extend_from_slice(flag);
        let o = fala(&args);
        assert_eq!(o.status.code(), Some(2), "{flag:?} accepted");
    }
}

#[test]
fn bad_duration_exits_2() {
    for bad in ["5min", "5", "1.5m", "0s", "-5s"] {
        let o = fala(&[
            "record",
            "--system",
            "x",
            "--out",
            "x.wav",
            "--no-click",
            "--duration",
            bad,
        ]);
        assert_eq!(o.status.code(), Some(2), "{bad} accepted: {}", stderr(&o));
    }
}

#[test]
#[ignore = "records from this machine's PipeWire in silence; needs FALA_TEST_SINK"]
fn system_channel_is_the_silent_monitor() {
    let dir = scratch("silent_monitor");
    let (o, wav) = record(&dir, "3s", &["--no-click"]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let mut reader = hound::WavReader::open(&wav).unwrap();
    let samples: Vec<i16> = reader.samples::<i16>().map(Result::unwrap).collect();
    let nonzero_l = samples.iter().step_by(2).filter(|&&s| s != 0).count();
    let nonzero_r = samples
        .iter()
        .skip(1)
        .step_by(2)
        .filter(|&&s| s != 0)
        .count();
    assert_eq!(nonzero_r, 0, "system channel is not the silent monitor");
    assert!(nonzero_l > 0, "mic channel is digital silence");
}
