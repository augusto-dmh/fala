//! `fala-cli import` pela fronteira: roda o binário e confere exit code, stdout, stderr e o WAV.
//!
//! Os testes `#[ignore]` precisam de `ffmpeg` e `ffprobe` no PATH (o job `rust` do CI não instala
//! o ffmpeg); rode com `cargo test -p fala-cli --test import -- --include-ignored`. Os demais rodam
//! com `PATH` vazio, então provam que a CLI decide antes de procurar o ffmpeg.

// `allow-unwrap-in-tests` não cobre os helpers de um crate de teste de integração.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const NEEDS_FFMPEG: &str = "precisa de ffmpeg e ffprobe no PATH";

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("import")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Roda `fala-cli` em `dir`; `path` substitui a variável PATH quando dado.
fn fala(dir: &Path, args: &[&str], path: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fala-cli"));
    command.current_dir(dir).args(args).env("RUST_LOG", "info");
    if let Some(path) = path {
        command.env("PATH", path);
    }
    command.output().unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn generate(args: &[&str], out: &Path) -> PathBuf {
    let status = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"])
        .args(args)
        .arg(out)
        .status()
        .expect(NEEDS_FFMPEG);
    assert!(status.success(), "ffmpeg não gerou {}", out.display());
    out.to_path_buf()
}

fn tone_mp3(dir: &Path) -> PathBuf {
    generate(
        &[
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:a",
            "libmp3lame",
        ],
        &dir.join("tom.mp3"),
    )
}

fn wav_spec(path: &Path) -> hound::WavSpec {
    hound::WavReader::open(path).unwrap().spec()
}

#[test]
#[ignore = "precisa de ffmpeg e ffprobe no PATH"]
fn import_prints_table_and_writes_wav() {
    let dir = scratch("table");
    tone_mp3(&dir);
    let o = fala(&dir, &["import", "tom.mp3", "--out", "t.wav"], None);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let spec = wav_spec(&dir.join("t.wav"));
    assert_eq!((spec.sample_rate, spec.channels), (48_000, 1));
    let out = stdout(&o);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3, "{out}");
    assert_eq!(lines[0], "| out | sample_rate | channels | duration_s |");
    let cells: Vec<&str> = lines[2]
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect();
    assert_eq!(cells[0], "t.wav");
    assert_eq!(cells[1], "48000");
    assert_eq!(cells[2], "1");
    let decimals = cells[3].split_once('.').map(|(_, d)| d.len());
    assert_eq!(decimals, Some(3), "{}", cells[3]);
    let seconds: f64 = cells[3].parse().unwrap();
    assert!((seconds - 1.0).abs() <= 0.06, "{seconds}");
}

#[test]
#[ignore = "precisa de ffmpeg e ffprobe no PATH"]
fn import_default_out_is_stem_fala_wav() {
    let dir = scratch("default-out");
    let media = dir.join("media");
    fs::create_dir_all(&media).unwrap();
    let input = tone_mp3(&media);
    let o = fala(&dir, &["import", input.to_str().unwrap()], None);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let spec = wav_spec(&dir.join("tom.fala.wav"));
    assert_eq!((spec.sample_rate, spec.channels), (48_000, 1));
    assert!(!media.join("tom.fala.wav").exists());
}

#[test]
#[ignore = "precisa de ffmpeg e ffprobe no PATH"]
fn import_reports_progress_on_stderr() {
    let dir = scratch("progress");
    tone_mp3(&dir);
    let o = fala(&dir, &["import", "tom.mp3", "--out", "t.wav"], None);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let err = stderr(&o);
    let progress: Vec<&str> = err.lines().filter(|l| l.contains("import: ")).collect();
    // Um evento em 0 ao iniciar o ffmpeg, mais ao menos o bloco `progress=end`.
    assert!(progress.len() >= 2, "{err}");
    assert!(
        progress[0].ends_with("import: 0.0 s / 1.0 s (0 %)"),
        "{}",
        progress[0]
    );
    assert!(progress.iter().all(|l| l.contains("/ 1.0 s (")), "{err}");
    assert!(!stdout(&o).contains("import: "));
}

#[test]
fn import_without_ffmpeg_exits_1() {
    let dir = scratch("no-ffmpeg");
    fs::write(dir.join("tom.mp3"), b"qualquer coisa").unwrap();
    let o = fala(&dir, &["import", "tom.mp3"], Some(""));
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    let err = stderr(&o);
    assert!(err.contains("ffmpeg"), "{err}");
    assert!(err.contains("PATH"), "{err}");
    assert!(stdout(&o).is_empty());
}

#[test]
fn import_missing_file_exits_2() {
    let dir = scratch("missing");
    let o = fala(&dir, &["import", "nao-existe.mp3"], Some(""));
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(stderr(&o).contains("nao-existe.mp3"), "{}", stderr(&o));
}

#[test]
#[ignore = "precisa de ffmpeg e ffprobe no PATH"]
fn import_no_audio_exits_2() {
    let dir = scratch("no-audio");
    generate(
        &[
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=1:size=64x64:rate=10",
            "-c:v",
            "mpeg4",
        ],
        &dir.join("video.mp4"),
    );
    let o = fala(&dir, &["import", "video.mp4"], None);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(!dir.join("video.fala.wav").exists());
}

#[test]
fn import_existing_out_exits_2() {
    let dir = scratch("existing-out");
    fs::write(dir.join("tom.mp3"), b"qualquer coisa").unwrap();
    fs::write(dir.join("t.wav"), b"conteudo anterior").unwrap();
    let o = fala(&dir, &["import", "tom.mp3", "--out", "t.wav"], Some(""));
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert_eq!(fs::read(dir.join("t.wav")).unwrap(), b"conteudo anterior");
}
