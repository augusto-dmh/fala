//! `fala-cli bench` pela fronteira: roda o binário e confere exit code, stdout e stderr.

// `allow-unwrap-in-tests` não cobre os helpers de um crate de teste de integração.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
