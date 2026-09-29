//! `fala-cli bench` pela fronteira: roda o binário e confere exit code, stdout e stderr.
//!
//! Os testes `#[ignore]` precisam de modelo e de fala real e leem do ambiente (falham se faltar):
//! `FALA_TEST_PARAKEET_DIR`, `FALA_TEST_GGUF`, `FALA_TEST_NEMOTRON_GGUF`, `FALA_TEST_SPEECH_WAV`
//! (16 kHz mono i16, 5-20 s).

// `allow-unwrap-in-tests` não cobre os helpers de um crate de teste de integração.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const LEGEND_HYP: &str = "engine=hyp model=hyp threads=- device=- load_s=- tag=-";
const HEADER: &str = "| cut | audio_s | wall_s | rtf | wer_% | sub | del | ins | ref_words |";

struct Corpus {
    root: PathBuf,
}

impl Corpus {
    fn new(name: &str) -> Self {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("bench")
            .join(name);
        let _ = fs::remove_dir_all(&root);
        for dir in ["cuts", "refs", "hyp"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        Corpus { root }
    }

    fn dir(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn wav(&self, stem: &str, seconds: f64) {
        write_wav(
            &self.dir("cuts").join(format!("{stem}.wav")),
            16_000,
            1,
            16,
            false,
            seconds,
        );
    }

    fn reference(&self, stem: &str, text: &str) {
        fs::write(self.dir("refs").join(format!("{stem}.txt")), text).unwrap();
    }

    fn hyp(&self, stem: &str, text: &str, wall_s: Option<&str>) {
        fs::write(self.dir("hyp").join(format!("{stem}.txt")), text).unwrap();
        if let Some(w) = wall_s {
            fs::write(self.dir("hyp").join(format!("{stem}.wall_s")), w).unwrap();
        }
    }

    fn speech(&self, stem: &str) {
        let src = env_path("FALA_TEST_SPEECH_WAV");
        fs::copy(src, self.dir("cuts").join(format!("{stem}.wav"))).unwrap();
        self.reference(stem, "referência provisória do teste");
    }

    /// Só os primeiros `seconds` da fala de teste.
    fn speech_prefix(&self, stem: &str, seconds: u32) {
        let mut reader = hound::WavReader::open(env_path("FALA_TEST_SPEECH_WAV")).unwrap();
        let spec = reader.spec();
        let n = (seconds * spec.sample_rate) as usize;
        let mut w =
            hound::WavWriter::create(self.dir("cuts").join(format!("{stem}.wav")), spec).unwrap();
        for sample in reader.samples::<i16>().take(n) {
            w.write_sample(sample.unwrap()).unwrap();
        }
        w.finalize().unwrap();
        self.reference(stem, "referência provisória do teste");
    }

    fn args(&self) -> Vec<String> {
        vec![
            "bench".into(),
            "--cuts".into(),
            self.dir("cuts").display().to_string(),
            "--refs".into(),
            self.dir("refs").display().to_string(),
        ]
    }

    fn hyp_args(&self) -> Vec<String> {
        let mut a = self.args();
        a.extend(["--hyp".into(), self.dir("hyp").display().to_string()]);
        a
    }
}

fn write_wav(path: &Path, rate: u32, channels: u16, bits: u16, float: bool, seconds: f64) {
    let spec = hound::WavSpec {
        channels,
        sample_rate: rate,
        bits_per_sample: bits,
        sample_format: if float {
            hound::SampleFormat::Float
        } else {
            hound::SampleFormat::Int
        },
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    let n = (seconds * f64::from(rate)) as usize * usize::from(channels);
    for i in 0..n {
        let x = ((i % 100) as f32 / 100.0) - 0.5;
        match (float, bits) {
            (true, _) => w.write_sample(x).unwrap(),
            (false, 8) => w.write_sample((x * 100.0) as i8).unwrap(),
            (false, _) => w.write_sample((x * 10_000.0) as i16).unwrap(),
        }
    }
    w.finalize().unwrap();
}

fn env_path(var: &str) -> PathBuf {
    PathBuf::from(std::env::var(var).unwrap_or_else(|_| panic!("{var} is not set")))
}

fn fala(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fala-cli"))
        .args(args)
        .env("RUST_LOG", "info")
        .output()
        .unwrap()
}

fn with(mut base: Vec<String>, extra: &[&str]) -> Vec<String> {
    base.extend(extra.iter().map(|s| (*s).to_owned()));
    base
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// Colunas de uma linha da tabela: cut, audio_s, wall_s, rtf, wer_%, sub, del, ins, ref_words.
fn cells(line: &str) -> Vec<String> {
    line.trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_owned())
        .collect()
}

fn rows(out: &str) -> Vec<Vec<String>> {
    out.lines().skip(3).map(cells).collect()
}

fn legend_field(out: &str, key: &str) -> String {
    let legend = out.lines().next().unwrap();
    legend
        .split(' ')
        .find_map(|kv| kv.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("no {key}= in legend {legend}"))
        .to_owned()
}

// ---- S1 ----

#[test]
fn total_line_pools_wer_and_rtf() {
    let c = Corpus::new("total_pools");
    c.wav("a", 1.0);
    c.wav("b", 3.0);
    c.reference("a", "um");
    c.reference("b", "dois três quatro");
    // rtf 0.5 e 1.0: o agregado Σwall/Σaudio = 3.5/4 = 0.875; a média dos rtf seria 0.750.
    c.hyp("a", "outro", Some("0.5"));
    c.hyp("b", "dois três quatro", Some("3.0"));
    let o = fala(&c.hyp_args());
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let r = rows(&stdout(&o));
    assert_eq!(r[0][4], "100.00");
    assert_eq!(r[1][4], "0.00");
    let total = &r[2];
    assert_eq!(total[0], "total");
    assert_eq!(total[8], "4");
    assert_eq!(total[4], "25.00");
    assert_eq!(total[3], "0.875");
}

#[test]
fn empty_reference_exits_2() {
    let c = Corpus::new("empty_ref");
    c.wav("a", 1.0);
    c.reference("a", " ... !? ");
    let o = fala(&with(
        c.args(),
        &["--engine", "parakeet-onnx", "--model", "/nonexistent/model"],
    ));
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(stderr(&o).contains("a.txt"), "{}", stderr(&o));
}

// ---- S2 ----

#[test]
fn parakeet_rejects_threads_and_device() {
    let c = Corpus::new("parakeet_rejects");
    c.wav("a", 1.0);
    c.reference("a", "um");
    let base = with(
        c.args(),
        &["--engine", "parakeet-onnx", "--model", "/nonexistent/model"],
    );
    let o = fala(&with(base.clone(), &["--threads", "4"]));
    assert_eq!(o.status.code(), Some(2));
    assert!(stderr(&o).contains("--threads"), "{}", stderr(&o));
    let o = fala(&with(base, &["--device", "gpu"]));
    assert_eq!(o.status.code(), Some(2));
    assert!(stderr(&o).contains("--device gpu"), "{}", stderr(&o));
}

#[test]
#[ignore = "needs FALA_TEST_PARAKEET_DIR and FALA_TEST_SPEECH_WAV"]
fn parakeet_onnx_prints_one_row_per_cut() {
    let c = Corpus::new("parakeet_rows");
    c.speech("a");
    let model = env_path("FALA_TEST_PARAKEET_DIR").display().to_string();
    let o = fala(&with(
        c.args(),
        &["--engine", "parakeet-onnx", "--model", &model],
    ));
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let out = stdout(&o);
    assert_eq!(out.lines().count(), 5, "{out}");
    assert!(out.starts_with(
        "engine=parakeet-onnx model=parakeet-tdt-0.6b-v3-int8 threads=- device=cpu load_s="
    ));
    assert_eq!(out.lines().nth(1).unwrap(), HEADER);
    let r = rows(&out);
    assert_eq!(r[0][0], "a");
    assert_eq!(r[1][0], "total");
    let rtf = &r[0][3];
    assert_eq!(rtf.split('.').nth(1).map(str::len), Some(3), "rtf {rtf}");
    assert!(rtf.parse::<f64>().unwrap() < 1.0, "rtf {rtf}");
    r[0][1].parse::<f64>().unwrap();
    r[0][2].parse::<f64>().unwrap();
}

#[test]
#[ignore = "needs FALA_TEST_PARAKEET_DIR and FALA_TEST_SPEECH_WAV"]
fn load_time_is_excluded_from_wall_s() {
    let c = Corpus::new("parakeet_load");
    // 2 s de fala transcrevem em ~0.2 s; a carga leva ~1.5-2.5 s. Se a carga entrasse no wall_s,
    // wall_s >= load_s sempre.
    c.speech_prefix("a", 2);
    let model = env_path("FALA_TEST_PARAKEET_DIR").display().to_string();
    let o = fala(&with(
        c.args(),
        &["--engine", "parakeet-onnx", "--model", &model],
    ));
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let out = stdout(&o);
    assert!(!out.lines().nth(1).unwrap().contains("load"));
    let load_s: f64 = legend_field(&out, "load_s").parse().unwrap();
    let wall_s: f64 = rows(&out)[0][2].parse().unwrap();
    assert!(wall_s < load_s, "wall_s {wall_s} >= load_s {load_s}");
}

// ---- S3 ----

#[test]
#[ignore = "needs FALA_TEST_GGUF and FALA_TEST_SPEECH_WAV"]
fn gguf_legend_reports_threads_and_cpu() {
    let c = Corpus::new("gguf_threads");
    c.speech("a");
    let model = env_path("FALA_TEST_GGUF").display().to_string();
    let base = with(c.args(), &["--engine", "gguf", "--model", &model]);
    let o = fala(&with(base.clone(), &["--threads", "8"]));
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let out = stdout(&o);
    assert!(
        out.lines()
            .next()
            .unwrap()
            .contains(" threads=8 device=cpu "),
        "{out}"
    );
    assert_eq!(rows(&out)[0][0], "a");
    let o = fala(&base);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let n = std::thread::available_parallelism()
        .unwrap()
        .get()
        .to_string();
    assert_eq!(legend_field(&stdout(&o), "threads"), n);
}

#[test]
#[ignore = "needs a build with --features vulkan, a GPU, FALA_TEST_GGUF and FALA_TEST_SPEECH_WAV"]
fn gguf_gpu_reports_device() {
    let c = Corpus::new("gguf_gpu");
    c.speech("a");
    let model = env_path("FALA_TEST_GGUF").display().to_string();
    let o = fala(&with(
        c.args(),
        &["--engine", "gguf", "--model", &model, "--device", "gpu"],
    ));
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let device = legend_field(&stdout(&o), "device");
    assert_ne!(device, "cpu");
    let err = stderr(&o);
    let line = err
        .lines()
        .find(|l| l.contains(&format!("dispositivo GPU: {device} ")))
        .unwrap_or_else(|| panic!("no device line in {err}"));
    assert!(
        !line.contains("llvmpipe"),
        "ran on the software rasterizer: {line}"
    );
}

#[cfg(not(any(feature = "vulkan", feature = "cuda")))]
#[test]
fn gpu_without_backend_exits_1() {
    let c = Corpus::new("gpu_missing");
    c.wav("a", 1.0);
    c.reference("a", "um");
    let o = fala(&with(
        c.args(),
        &[
            "--engine",
            "gguf",
            "--model",
            "/nonexistent/model.bin",
            "--device",
            "gpu",
        ],
    ));
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    let err = stderr(&o);
    assert!(err.contains("vulkan") && err.contains("cuda"), "{err}");
    assert_eq!(stdout(&o), "", "fell back to a run instead of failing");
}

#[test]
#[ignore = "needs FALA_TEST_GGUF and FALA_TEST_SPEECH_WAV"]
fn engine_error_keeps_printed_rows() {
    let c = Corpus::new("engine_error");
    c.speech("a");
    c.wav("b", 0.0);
    c.reference("b", "vazio");
    let model = env_path("FALA_TEST_GGUF").display().to_string();
    let o = fala(&with(c.args(), &["--engine", "gguf", "--model", &model]));
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    let out = stdout(&o);
    assert_eq!(out.lines().count(), 4, "{out}");
    assert_eq!(rows(&out)[0][0], "a");
    let err = stderr(&o);
    let line = err
        .lines()
        .find(|l| l.contains("`b`"))
        .unwrap_or_else(|| panic!("{err}"));
    assert!(line.contains("run"), "engine error text missing: {line}");
}

#[test]
fn missing_model_exits_1() {
    let c = Corpus::new("missing_model");
    c.wav("a", 1.0);
    c.reference("a", "um");
    for engine in ["parakeet-onnx", "gguf"] {
        let o = fala(&with(
            c.args(),
            &["--engine", engine, "--model", "/nonexistent/model.bin"],
        ));
        assert_eq!(o.status.code(), Some(1), "{engine}: {}", stderr(&o));
        assert_eq!(stdout(&o), "", "{engine}");
    }
}

// ---- S4 ----

#[test]
fn cuts_run_in_byte_order_ignoring_other_files() {
    let c = Corpus::new("byte_order");
    for stem in ["a", "_c", "B"] {
        c.wav(stem, 1.0);
        c.reference(stem, "um");
        c.hyp(stem, "um", None);
    }
    fs::write(c.dir("cuts").join("notes.txt"), "x").unwrap();
    fs::write(c.dir("cuts").join("x.mp3"), "x").unwrap();
    let o = fala(&c.hyp_args());
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let names: Vec<String> = rows(&stdout(&o))
        .into_iter()
        .map(|r| r[0].clone())
        .collect();
    assert_eq!(names, ["B", "_c", "a", "total"]);
}

#[test]
fn no_wav_exits_2() {
    let c = Corpus::new("no_wav");
    fs::write(c.dir("cuts").join("a.txt"), "x").unwrap();
    let o = fala(&c.hyp_args());
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
}

#[test]
fn missing_reference_exits_2() {
    let c = Corpus::new("missing_ref");
    c.wav("a", 1.0);
    c.wav("sem_ref", 1.0);
    c.reference("a", "um");
    let o = fala(&with(
        c.args(),
        &["--engine", "parakeet-onnx", "--model", "/nonexistent/model"],
    ));
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(stderr(&o).contains("`sem_ref`"), "{}", stderr(&o));
}

#[test]
fn wrong_wav_spec_exits_2() {
    // (rate, channels, bits, float, the value the message must cite)
    let cases = [
        (48_000, 1, 16, false, "48000 Hz"),
        (16_000, 2, 16, false, "2 canal"),
        (16_000, 1, 32, true, "32 bits float"),
        (16_000, 1, 8, false, "8 bits int"),
    ];
    for (i, (rate, ch, bits, float, cited)) in cases.into_iter().enumerate() {
        let c = Corpus::new(&format!("wrong_spec_{i}"));
        write_wav(&c.dir("cuts").join("x.wav"), rate, ch, bits, float, 0.1);
        c.reference("x", "um");
        c.hyp("x", "um", None);
        let o = fala(&c.hyp_args());
        assert_eq!(o.status.code(), Some(2), "case {cited}: {}", stderr(&o));
        let err = stderr(&o);
        assert!(
            err.contains("x.wav") && err.contains(cited),
            "case {cited}: {err}"
        );
    }
}

// ---- S5 ----

#[test]
#[ignore = "needs FALA_TEST_PARAKEET_DIR and FALA_TEST_SPEECH_WAV"]
fn out_then_hyp_round_trips() {
    let c = Corpus::new("round_trip");
    c.speech("a");
    let model = env_path("FALA_TEST_PARAKEET_DIR").display().to_string();
    let out_dir = c.dir("fresh").join("nested");
    let o = fala(&with(
        c.args(),
        &[
            "--engine",
            "parakeet-onnx",
            "--model",
            &model,
            "--out",
            &out_dir.display().to_string(),
        ],
    ));
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let engine_rows = rows(&stdout(&o));
    let text = fs::read_to_string(out_dir.join("a.txt")).unwrap();
    assert!(!text.trim().is_empty(), "empty hypothesis file");
    // A hipótese só vai para o arquivo: a palavra mais longa não aparece no stdout nem no stderr.
    let longest = text
        .split_whitespace()
        .max_by_key(|w| w.chars().count())
        .unwrap();
    for stream in [stdout(&o), stderr(&o)] {
        assert!(!stream.contains(longest), "`{longest}` leaked: {stream}");
    }
    let wall: f64 = fs::read_to_string(out_dir.join("a.wall_s"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(format!("{wall:.2}"), engine_rows[0][2]);

    let o = fala(&with(c.args(), &["--hyp", &out_dir.display().to_string()]));
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let hyp_rows = rows(&stdout(&o));
    for (e, h) in engine_rows.iter().zip(&hyp_rows) {
        assert_eq!(e[4..], h[4..], "WER columns differ");
    }
}

#[test]
fn hyp_scores_and_names_dir() {
    let c = Corpus::new("hyp_scores");
    c.wav("a", 1.0);
    c.reference("a", "o guarda-chuva ficou");
    c.hyp("a", "o guarda chuva ficou aqui", None);
    let o = fala(&c.hyp_args());
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let out = stdout(&o);
    assert!(out.starts_with("engine=hyp model=hyp "), "{out}");
    assert_eq!(rows(&out)[0][4..], ["25.00", "0", "0", "1", "4"]);
}

#[test]
fn missing_wall_s_prints_dash() {
    let c = Corpus::new("dash");
    c.wav("a", 2.0);
    c.wav("b", 2.0);
    for s in ["a", "b"] {
        c.reference(s, "um");
    }
    c.hyp("a", "um", Some("1.0"));
    c.hyp("b", "um", None);
    let o = fala(&c.hyp_args());
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let r = rows(&stdout(&o));
    assert_eq!((r[0][2].as_str(), r[0][3].as_str()), ("1.00", "0.500"));
    assert_eq!((r[1][2].as_str(), r[1][3].as_str()), ("-", "-"));
    assert_eq!(r[2][3], "-");
}

#[test]
fn hyp_rejects_engine_flags() {
    let c = Corpus::new("hyp_rejects");
    c.wav("a", 1.0);
    c.reference("a", "um");
    c.hyp("a", "um", None);
    let flags: [&[&str]; 5] = [
        &["--engine", "gguf"],
        &["--model", "/m"],
        &["--threads", "4"],
        &["--device", "cpu"],
        &["--chunk-s", "30"],
    ];
    for flag in flags {
        let o = fala(&with(c.hyp_args(), flag));
        assert_eq!(
            o.status.code(),
            Some(2),
            "{flag:?} accepted: {}",
            stdout(&o)
        );
    }
}

#[test]
fn hyp_missing_stem_exits_2() {
    let c = Corpus::new("hyp_missing");
    c.wav("a", 1.0);
    c.wav("orfao", 1.0);
    c.reference("a", "um");
    c.reference("orfao", "um");
    c.hyp("a", "um", None);
    let o = fala(&c.hyp_args());
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(stderr(&o).contains("`orfao`"), "{}", stderr(&o));
}

// ---- S6 ----

#[test]
fn stdout_is_legend_then_table() {
    let c = Corpus::new("stdout_shape");
    for s in ["a", "b"] {
        c.wav(s, 1.0);
        c.reference(s, "um");
        c.hyp(s, "um", Some("0.1"));
    }
    let o = fala(&c.hyp_args());
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let out = stdout(&o);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 6, "{out}");
    assert_eq!(lines[0], LEGEND_HYP);
    assert_eq!(lines[1], HEADER);
    assert_eq!(
        lines[2],
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"
    );
    assert!(lines[3].starts_with("| a |") && lines[4].starts_with("| b |"));
    assert!(lines[5].starts_with("| total |"));

    let o = fala(&with(c.hyp_args(), &["--tag", "x"]));
    assert_eq!(legend_field(&stdout(&o), "tag"), "x");
}

#[test]
fn transcript_text_never_printed_above_debug() {
    let c = Corpus::new("no_leak");
    c.wav("a", 1.0);
    c.reference("a", "xiloreferencia palavra");
    c.hyp("a", "xilohipotese palavra", None);
    let o = fala(&c.hyp_args());
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    for stream in [stdout(&o), stderr(&o)] {
        assert!(
            !stream.contains("xiloreferencia") && !stream.contains("xilohipotese"),
            "{stream}"
        );
    }
}

#[test]
#[ignore = "needs FALA_TEST_PARAKEET_DIR and FALA_TEST_SPEECH_WAV"]
fn rows_stream_while_running() {
    let c = Corpus::new("streaming");
    c.speech("a");
    // `b` = a fala repetida 6 vezes, para a engine ainda estar trabalhando quando `a` sair.
    let mut reader = hound::WavReader::open(env_path("FALA_TEST_SPEECH_WAV")).unwrap();
    let spec = reader.spec();
    let samples: Vec<i16> = reader.samples::<i16>().map(Result::unwrap).collect();
    let mut w = hound::WavWriter::create(c.dir("cuts").join("b.wav"), spec).unwrap();
    for _ in 0..6 {
        for s in &samples {
            w.write_sample(*s).unwrap();
        }
    }
    w.finalize().unwrap();
    c.reference("b", "referência provisória");

    let model = env_path("FALA_TEST_PARAKEET_DIR").display().to_string();
    let mut child = Command::new(env!("CARGO_BIN_EXE_fala-cli"))
        .args(with(
            c.args(),
            &["--engine", "parakeet-onnx", "--model", &model],
        ))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let row_a = lines.nth(3).unwrap().unwrap();
    assert!(row_a.starts_with("| a |"), "{row_a}");
    assert!(
        child.try_wait().unwrap().is_none(),
        "process ended before row a was read"
    );
    assert!(child.wait().unwrap().success());
}

// ---- bench-language ----

/// A linha do stderr que registra o idioma resolvido, sem o prefixo do logger.
fn idioma(o: &Output) -> String {
    let err = stderr(o);
    let line = err
        .lines()
        .find(|l| l.contains("idioma: "))
        .unwrap_or_else(|| panic!("no idioma: line in {err}"));
    line[line.find("idioma: ").unwrap()..].to_owned()
}

#[test]
#[ignore = "needs FALA_TEST_NEMOTRON_GGUF and FALA_TEST_SPEECH_WAV"]
fn nemotron_runs_with_pt_br() {
    let c = Corpus::new("nemotron_pt_br");
    c.speech("a");
    let model = env_path("FALA_TEST_NEMOTRON_GGUF").display().to_string();
    let o = fala(&with(c.args(), &["--engine", "gguf", "--model", &model]));
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(idioma(&o), "idioma: pt-BR");
    let out = stdout(&o);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 5, "{out}");
    assert!(lines[0].starts_with("engine=gguf model="), "{out}");
    assert_eq!(lines[1], HEADER);
    assert_eq!(
        lines[2],
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"
    );
    assert!(lines[3].starts_with("| a |"), "{out}");
    assert!(lines[4].starts_with("| total |"), "{out}");
}

#[test]
#[ignore = "needs FALA_TEST_GGUF and FALA_TEST_SPEECH_WAV"]
fn whisper_keeps_bare_pt() {
    let c = Corpus::new("whisper_bare_pt");
    c.speech("a");
    let model = env_path("FALA_TEST_GGUF").display().to_string();
    let o = fala(&with(c.args(), &["--engine", "gguf", "--model", &model]));
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(idioma(&o), "idioma: pt");
}

#[test]
#[ignore = "needs FALA_TEST_GGUF and FALA_TEST_SPEECH_WAV"]
fn language_override_and_unsupported() {
    let c = Corpus::new("language_override");
    c.speech("a");
    let model = env_path("FALA_TEST_GGUF").display().to_string();
    let base = with(c.args(), &["--engine", "gguf", "--model", &model]);
    let o = fala(&with(base.clone(), &["--language", "en"]));
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(idioma(&o), "idioma: en");

    let o = fala(&with(base, &["--language", "xx-YY"]));
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    let err = stderr(&o);
    assert!(err.contains("xx-YY"), "{err}");
    // A lista do modelo: o whisper anuncia pelo menos `en` e `pt`.
    let listed = err
        .lines()
        .find(|l| l.contains("xx-YY"))
        .unwrap_or_else(|| panic!("{err}"));
    assert!(
        listed.contains("en") && listed.contains("pt"),
        "model languages not listed: {listed}"
    );
    assert_eq!(stdout(&o), "", "a row was transcribed");
}

#[test]
fn language_rejected_outside_gguf() {
    let c = Corpus::new("language_rejected");
    c.wav("a", 1.0);
    c.reference("a", "um");
    c.hyp("a", "um", None);
    let o = fala(&with(
        c.args(),
        &[
            "--engine",
            "parakeet-onnx",
            "--model",
            "/nonexistent/model",
            "--language",
            "pt-BR",
        ],
    ));
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(stderr(&o).contains("--language"), "{}", stderr(&o));
    assert!(
        stderr(&o).contains("não expõe essa opção"),
        "{}",
        stderr(&o)
    );

    let o = fala(&with(c.hyp_args(), &["--language", "pt-BR"]));
    assert_eq!(o.status.code(), Some(2), "{}", stdout(&o));
}
