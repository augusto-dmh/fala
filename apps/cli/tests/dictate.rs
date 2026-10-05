//! `fala-cli dictate` pela fronteira: roda o binário e confere exit code, stdout e stderr.
//!
//! Os testes `#[ignore]` precisam do modelo e de fala real e leem do ambiente (falham se faltar):
//! `FALA_TEST_PARAKEET_DIR` e `FALA_TEST_SPEECH_WAV` (fala real em pt-BR, 16 kHz mono i16, com
//! pelo menos uma pausa de 1 s). Nenhum teste abre o microfone.
//!
//! `FALA_CLI_BIN`, se definido, troca o binário testado: com um `CARGO_TARGET_DIR` compartilhado
//! entre worktrees, outro build pode sobrescrever `target/debug/fala-cli` no meio da rodada.

// `allow-unwrap-in-tests` não cobre os helpers de um crate de teste de integração.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

const UTTERANCE_KEYS: [&str; 6] = [
    "event",
    "dictation",
    "index",
    "audio_ms",
    "asr_ms",
    "before_release",
];
const DICTATION_KEYS: [&str; 7] = [
    "event",
    "dictation",
    "audio_ms",
    "utterances",
    "flush_ms",
    "tail_asr_ms",
    "release_to_text_ms",
];
const LOAD_KEYS: [&str; 3] = ["event", "model_ms", "vad_ms"];

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set")))
}

fn tmp(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("dictate")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn dictate(args: &[&str], trace: bool) -> Output {
    let bin = std::env::var_os("FALA_CLI_BIN").map_or_else(
        || PathBuf::from(env!("CARGO_BIN_EXE_fala-cli")),
        PathBuf::from,
    );
    let mut cmd = Command::new(bin);
    cmd.arg("dictate")
        .args(args)
        .env_remove("RUST_LOG")
        .stdin(Stdio::null());
    if trace {
        cmd.env("FALA_TRACE", "1");
    } else {
        cmd.env_remove("FALA_TRACE");
    }
    cmd.output().unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Os pares `chave=valor` de cada linha de trace, a partir do `event=`.
fn trace_lines(out: &Output) -> Vec<Vec<(String, String)>> {
    stderr(out)
        .lines()
        .filter_map(|l| l.find("event=").map(|i| &l[i..]))
        .map(|l| {
            l.split(' ')
                .map(|pair| {
                    let (k, v) = pair.split_once('=').expect("token sem `=`");
                    (k.to_owned(), v.to_owned())
                })
                .collect()
        })
        .collect()
}

fn event(line: &[(String, String)]) -> &str {
    &line[0].1
}

fn value(line: &[(String, String)], key: &str) -> u64 {
    line.iter()
        .find(|(k, _)| k == key)
        .unwrap_or_else(|| panic!("sem `{key}`"))
        .1
        .parse()
        .unwrap()
}

fn speech_args() -> Vec<String> {
    vec![
        "--model".to_owned(),
        env_path("FALA_TEST_PARAKEET_DIR").display().to_string(),
        "--wav".to_owned(),
        env_path("FALA_TEST_SPEECH_WAV").display().to_string(),
    ]
}

fn run_speech(trace: bool) -> Output {
    let args = speech_args();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = dictate(&args, trace);
    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr(&out));
    out
}

#[test]
fn bad_paths_exit_2() {
    let dir = tmp("bad-paths");
    let missing_model = dir.join("sem-modelo");
    let empty_model = dir.join("modelo-vazio");
    fs::create_dir_all(&empty_model).unwrap();
    let bad_vad = dir.join("vad.onnx");
    fs::write(&bad_vad, b"isto nao e um onnx").unwrap();
    let bad_wav = dir.join("audio.wav");
    fs::write(&bad_wav, b"isto nao e um wav").unwrap();
    let missing = missing_model.display().to_string();
    let empty = empty_model.display().to_string();
    let vad = bad_vad.display().to_string();
    let wav = bad_wav.display().to_string();

    let cases: [(Vec<&str>, &str); 4] = [
        (vec!["--model", &missing], &missing),
        (vec!["--model", &empty], &empty),
        (vec!["--model", &missing, "--vad", &vad], &vad),
        (vec!["--model", &missing, "--wav", &wav], &wav),
    ];
    for (args, path) in cases {
        let out = dictate(&args, false);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {}", stderr(&out));
        assert!(stderr(&out).contains(path), "{args:?}: {}", stderr(&out));
        assert!(
            !stderr(&out).contains("microfone:"),
            "{args:?} abriu o microfone"
        );
        assert_eq!(stdout(&out), "");
    }
}

#[test]
fn unknown_mic_exits_2() {
    let dir = tmp("unknown-mic");
    let model = dir.join("sem-modelo").display().to_string();
    let out = dictate(
        &[
            "--model",
            &model,
            "--mic",
            "nenhum-dispositivo-tem-este-nome-7f3a",
        ],
        false,
    );
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("dispositivos de entrada"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn unknown_language_exits_2() {
    let dir = tmp("unknown-language");
    let model = dir.join("sem-modelo").display().to_string();
    let out = dictate(&["--model", &model, "--language", "xx"], false);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(stderr(&out).contains("--language"), "{}", stderr(&out));
    assert!(stderr(&out).contains("xx"), "{}", stderr(&out));
    assert_eq!(stdout(&out), "");
}

#[test]
#[ignore = "needs FALA_TEST_PARAKEET_DIR"]
fn wav_is_paced_and_silence_prints_empty_line() {
    let wav = tmp("silence").join("silencio.wav");
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(&wav, spec).unwrap();
    for _ in 0..48_000 {
        w.write_sample(0i16).unwrap();
    }
    w.finalize().unwrap();
    let model = env_path("FALA_TEST_PARAKEET_DIR").display().to_string();
    let wav = wav.display().to_string();
    // O relógio começa depois da carga do modelo: a linha `event=load` marca esse instante, então
    // medimos do fim da carga (pelo trace) até o fim do processo.
    let started = Instant::now();
    let out = dictate(&["--model", &model, "--wav", &wav], true);
    let wall = started.elapsed();
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(stdout(&out), "\n");
    let load = trace_lines(&out)
        .into_iter()
        .find(|l| event(l) == "load")
        .unwrap();
    let load_ms = value(&load, "model_ms") + value(&load, "vad_ms");
    let paced = wall.saturating_sub(Duration::from_millis(load_ms));
    assert!(
        paced >= Duration::from_millis(2_900),
        "3 s de WAV em {paced:?} fora a carga"
    );
}

#[test]
#[ignore = "needs FALA_TEST_PARAKEET_DIR and FALA_TEST_SPEECH_WAV"]
fn speech_prints_one_line() {
    let out = run_speech(false);
    let text = stdout(&out);
    assert_eq!(text.lines().count(), 1, "stdout: {text:?}");
    let line = text.trim_end_matches('\n');
    assert!(!line.trim().is_empty());
    assert_eq!(line, line.trim(), "linha não aparada");
    assert!(!line.contains("  "), "espaço duplo: {line:?}");
}

#[test]
#[ignore = "needs FALA_TEST_PARAKEET_DIR and FALA_TEST_SPEECH_WAV"]
fn utterances_transcribe_before_release() {
    let out = run_speech(true);
    let utterances: Vec<_> = trace_lines(&out)
        .into_iter()
        .filter(|l| event(l) == "utterance")
        .collect();
    assert!(
        utterances.len() >= 2,
        "a fala de teste deu {} utterance(s)",
        utterances.len()
    );
    for line in &utterances[..utterances.len() - 1] {
        assert_eq!(value(line, "before_release"), 1, "{line:?}");
    }
}

#[test]
#[ignore = "needs FALA_TEST_PARAKEET_DIR and FALA_TEST_SPEECH_WAV"]
fn trace_lines_follow_door_3() {
    let out = run_speech(true);
    let lines = trace_lines(&out);
    let utterances: Vec<_> = lines.iter().filter(|l| event(l) == "utterance").collect();
    let dictations: Vec<_> = lines.iter().filter(|l| event(l) == "dictation").collect();
    assert_eq!(dictations.len(), 1);
    assert!(!utterances.is_empty());
    assert_eq!(value(dictations[0], "utterances"), utterances.len() as u64);
    for line in &lines {
        let keys: Vec<&str> = line.iter().map(|(k, _)| k.as_str()).collect();
        let expected: &[&str] = match event(line) {
            "utterance" => &UTTERANCE_KEYS,
            "dictation" => &DICTATION_KEYS,
            "load" => &LOAD_KEYS,
            other => panic!("evento desconhecido `{other}`"),
        };
        assert_eq!(keys, expected);
        for (k, v) in &line[1..] {
            assert!(v.parse::<u64>().is_ok(), "`{k}={v}` não é inteiro");
        }
    }
    let words: Vec<String> = stdout(&out)
        .split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|w| w.chars().count() >= 3)
        .collect();
    assert!(!words.is_empty());
    let trace_text: Vec<String> = stderr(&out)
        .lines()
        .filter(|l| l.contains("event="))
        .map(str::to_lowercase)
        .collect();
    for word in &words {
        for line in &trace_text {
            let tokens: Vec<&str> = line
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .collect();
            assert!(
                !tokens.contains(&word.as_str()),
                "`{word}` no trace: {line}"
            );
        }
    }
}

#[test]
#[ignore = "needs FALA_TEST_PARAKEET_DIR and FALA_TEST_SPEECH_WAV"]
fn no_trace_without_env() {
    let out = run_speech(false);
    assert!(!stderr(&out).contains("event="), "{}", stderr(&out));
}

#[test]
#[ignore = "needs FALA_TEST_PARAKEET_DIR and FALA_TEST_SPEECH_WAV"]
fn load_is_traced_apart() {
    let out = run_speech(true);
    let lines = trace_lines(&out);
    let loads: Vec<usize> = (0..lines.len())
        .filter(|&i| event(&lines[i]) == "load")
        .collect();
    assert_eq!(loads.len(), 1);
    let load = &lines[loads[0]];
    let keys: Vec<&str> = load.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(keys, LOAD_KEYS);
    value(load, "model_ms");
    value(load, "vad_ms");
    let first_utterance = lines.iter().position(|l| event(l) == "utterance").unwrap();
    assert!(loads[0] < first_utterance);
}
