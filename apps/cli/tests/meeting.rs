//! `fala-cli meeting` pela fronteira: roda o binário e confere exit code, stderr e o WAV.
//!
//! Todos os testes são `#[ignore]`: gravam (ou enumeram) o PipeWire desta máquina, inclusive o
//! microfone, e leem o `node.name` do sink em `FALA_TEST_SINK` (falham se faltar).
//! `FALA_CLI_BIN`, se definido, troca o binário testado: com um `CARGO_TARGET_DIR` compartilhado
//! entre worktrees, outro build pode sobrescrever `target/debug/fala-cli` no meio da rodada.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn sink() -> String {
    std::env::var("FALA_TEST_SINK").expect("FALA_TEST_SINK is not set")
}

fn tmp(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("meeting")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn command() -> Command {
    let bin = std::env::var_os("FALA_CLI_BIN").map_or_else(
        || PathBuf::from(env!("CARGO_BIN_EXE_fala-cli")),
        PathBuf::from,
    );
    let mut cmd = Command::new(bin);
    cmd.arg("meeting").env_remove("RUST_LOG");
    cmd
}

const INDICATOR: &str = "● gravando reunião";

/// Apaga o WAV gravado do mic mesmo quando um assert falha.
struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

struct Recorded {
    code: Option<i32>,
    stderr: String,
}

/// Grava até `seconds` depois do indicador (o instante em que o relógio da gravação começa) e
/// então fecha o stdin (EOF = parar). Se o processo morrer antes, devolve o que ele deixou.
fn record_for(args: &[&str], seconds: u64) -> Recorded {
    let mut child = command()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let err = child.stderr.take().unwrap();
    let (started, recording) = mpsc::channel();
    let reader = thread::spawn(move || {
        let mut all = String::new();
        for line in BufReader::new(err).lines() {
            let line = line.unwrap_or_default();
            if line.contains(INDICATOR) {
                let _ = started.send(());
            }
            all.push_str(&line);
            all.push('\n');
        }
        all
    });
    if recording.recv_timeout(Duration::from_secs(30)).is_ok() {
        thread::sleep(Duration::from_secs(seconds));
    }
    drop(child.stdin.take());
    let status = child.wait().unwrap();
    Recorded {
        code: status.code(),
        stderr: reader.join().unwrap(),
    }
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
#[ignore = "records this machine's mic and PipeWire monitor; needs FALA_TEST_SINK"]
fn records_until_stdin_closes() {
    let wav = tmp("records").join("reuniao.wav");
    let _cleanup = Cleanup(wav.clone());
    let sink = sink();
    let out = record_for(&["--out", wav.to_str().unwrap(), "--system", &sink], 3);
    assert_eq!(out.code, Some(0), "{}", out.stderr);
    // O indicador vem antes do resumo do fim, ou seja, enquanto grava.
    let indicator = out.stderr.find(INDICATOR).expect("sem indicador");
    let summary = out.stderr.find("reunião gravada").expect("sem resumo");
    assert!(indicator < summary, "{}", out.stderr);
    let reader = hound::WavReader::open(&wav).unwrap();
    let spec = reader.spec();
    assert_eq!(spec.sample_rate, 48_000);
    assert_eq!(spec.channels, 2);
    let seconds = f64::from(reader.duration()) / 48_000.0;
    assert!((seconds - 3.0).abs() <= 0.5, "{seconds} s");
}

#[test]
#[ignore = "enumerates this machine's PipeWire; needs FALA_TEST_SINK"]
fn bad_inputs_exit_2() {
    let dir = tmp("bad-inputs");
    let wav = dir.join("reuniao.wav");
    let wav_s = wav.to_str().unwrap().to_owned();
    let sink = sink();
    let no_dir = dir.join("nao-existe").join("reuniao.wav");
    let no_dir_s = no_dir.to_str().unwrap().to_owned();
    let cases: [(Vec<&str>, &str); 3] = [
        (
            vec!["--out", &wav_s, "--system", "sink-que-nao-existe-7f3a"],
            "sink-que-nao-existe-7f3a",
        ),
        (
            vec![
                "--out",
                &wav_s,
                "--system",
                &sink,
                "--mic",
                "mic-que-nao-existe-7f3a",
            ],
            "mic-que-nao-existe-7f3a",
        ),
        (vec!["--out", &no_dir_s, "--system", &sink], &no_dir_s),
    ];
    for (args, needle) in cases {
        let out = command().args(&args).stdin(Stdio::null()).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}: {}", stderr(&out));
        assert!(stderr(&out).contains(needle), "{args:?}: {}", stderr(&out));
        assert!(!wav.exists() && !no_dir.exists(), "{args:?} criou o WAV");
    }
}

#[test]
#[ignore = "records this machine's mic and PipeWire monitor; needs FALA_TEST_SINK"]
fn full_disk_exits_1() {
    let sink = sink();
    let out = record_for(&["--out", "/dev/full", "--system", &sink], 2);
    assert_eq!(out.code, Some(1), "{}", out.stderr);
    assert!(out.stderr.contains("WAV /dev/full"), "{}", out.stderr);
    assert!(
        out.stderr.contains("No space left on device"),
        "{}",
        out.stderr
    );
}
