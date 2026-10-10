//! `fala-cli bench format`: o `Postprocessor` contra um corpus JSONL de ditados já formatados.
//!
//! Cada linha tem `raw` (o bruto do ASR), `formatted` (a referência) e, opcional, `edited` (o que
//! a pessoa deixou depois de editar), mais `app` e `lang`. O stdout tem só números: igualdade
//! exata e distância de edição normalizada contra `formatted` e contra `edited`, e p50/p90 da
//! latência. Texto do corpus nunca sai, nem no stderr.

use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use clap::Args;
use fala_core::{AppContext, Dictionary, Editor, Language, Transcript};
use fala_postproc::{
    CleanupLevel, Fallback, LlmConfig, Postprocessor, DEFAULT_BASE_URL, DEFAULT_MODEL,
};
use fala_secrets::SecretStore;
use serde_json::Value;

#[derive(Args)]
pub struct FormatBenchArgs {
    /// Corpus JSONL (`scripts/export-wispr-corpus.py`).
    #[arg(long)]
    corpus: PathBuf,
    /// Nível de limpeza pedido ao LLM: verbatim, light, medium ou heavy.
    #[arg(long, default_value = "light")]
    level: CleanupLevel,
    /// Só as N primeiras linhas.
    #[arg(long)]
    limit: Option<usize>,
    /// Usa o Gemini acima de 15 palavras (precisa de `fala-cli key set gemini`).
    #[arg(long)]
    llm: bool,
    /// Modelo do Gemini.
    #[arg(long, default_value = DEFAULT_MODEL)]
    model: String,
    /// Pausa entre linhas, para caber no limite de requests por minuto da chave.
    #[arg(long, default_value_t = 0)]
    interval_ms: u64,
    /// Endpoint do Gemini; só para testes contra um servidor falso.
    #[arg(long, hide = true, default_value = DEFAULT_BASE_URL)]
    gemini_base_url: String,
}

/// Uma linha do corpus.
struct Row {
    raw: String,
    formatted: String,
    edited: Option<String>,
    app: Option<String>,
    language: Language,
}

/// Igualdade exata e distância de edição somadas sobre as linhas que têm a referência.
#[derive(Default)]
struct Score {
    n: usize,
    exact: usize,
    distance: f64,
}

impl Score {
    fn add(&mut self, hypothesis: &str, reference: &str) {
        self.n += 1;
        self.exact += usize::from(hypothesis.trim() == reference.trim());
        self.distance += normalized_distance(hypothesis.trim(), reference.trim());
    }

    fn cells(&self) -> String {
        if self.n == 0 {
            return format!("| {} | - | - |", self.n);
        }
        let n = self.n as f64;
        format!(
            "| {} | {:.2} | {:.4} |",
            self.n,
            self.exact as f64 / n * 100.0,
            self.distance / n
        )
    }
}

/// Roda o subcomando e devolve o código de saída: 0 ok, 1 keyring indisponível, 2 entrada
/// inválida ou `--llm` sem chave.
pub fn run(
    args: FormatBenchArgs,
    store: &dyn SecretStore,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> u8 {
    let rows = match load(&args.corpus, args.limit) {
        Ok(rows) => rows,
        Err(message) => {
            let _ = writeln!(stderr, "erro: {message}");
            return 2;
        }
    };
    let gemini = if args.llm {
        match crate::format::gemini_from_store(store, &args.model, &args.gemini_base_url, stderr) {
            Ok(gemini) => Some(gemini),
            Err(code) => return code,
        }
    } else {
        None
    };
    let processor = Postprocessor::new(LlmConfig {
        enabled: args.llm,
        gemini,
        disabled_apps: Vec::new(),
    })
    .with_cleanup_level(args.level);

    let (mut raw_vs_formatted, mut raw_vs_edited) = (Score::default(), Score::default());
    let (mut vs_formatted, mut vs_edited) = (Score::default(), Score::default());
    let mut latencies = Vec::with_capacity(rows.len());
    let (mut by_llm, mut late, mut fallbacks) = (0, 0, Vec::<Fallback>::new());
    for (i, row) in rows.iter().enumerate() {
        if i > 0 && args.interval_ms > 0 {
            thread::sleep(Duration::from_millis(args.interval_ms));
        }
        let started = Instant::now();
        let formatted = processor.process(
            Transcript {
                text: row.raw.clone(),
                language: row.language,
            },
            AppContext {
                app_name: row.app.clone(),
            },
            &Dictionary::default(),
        );
        let mut text = formatted.dictation.final_text;
        if formatted.dictation.editor == Editor::Llm {
            by_llm += 1;
        }
        if let Some(fallback) = formatted.fallback {
            match formatted.late_edit.map(|edit| edit.wait()) {
                Some(Ok(late_text)) => {
                    late += 1;
                    text = late_text;
                }
                Some(Err(late_fallback)) => fallbacks.push(late_fallback),
                None => fallbacks.push(fallback),
            }
        }
        latencies.push(started.elapsed());
        raw_vs_formatted.add(&row.raw, &row.formatted);
        vs_formatted.add(&text, &row.formatted);
        if let Some(edited) = &row.edited {
            raw_vs_edited.add(&row.raw, edited);
            vs_edited.add(&text, edited);
        }
    }

    let mode = if args.llm { "llm" } else { "regras" };
    let model = if args.llm { args.model.as_str() } else { "-" };
    let fallback_list = summarize(&fallbacks);
    let lines = [
        format!(
            "corpus={} rows={} mode={mode} level={} model={model} llm={by_llm} late={late} fallback={}{fallback_list}",
            args.corpus
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            rows.len(),
            args.level,
            fallbacks.len(),
        ),
        "| hipótese | contra | n | exata_% | edição_média |".to_string(),
        "| --- | --- | ---: | ---: | ---: |".to_string(),
        format!("| bruto | formatted {}", raw_vs_formatted.cells()),
        format!("| bruto | edited {}", raw_vs_edited.cells()),
        format!("| fala | formatted {}", vs_formatted.cells()),
        format!("| fala | edited {}", vs_edited.cells()),
        format!(
            "latência_ms: p50={} p90={}",
            percentile_ms(&mut latencies, 50),
            percentile_ms(&mut latencies, 90)
        ),
    ];
    for line in lines {
        let _ = writeln!(stdout, "{line}");
    }
    0
}

/// As linhas do corpus, na ordem do arquivo. O erro cita o número da linha, nunca o conteúdo.
fn load(path: &PathBuf, limit: Option<usize>) -> Result<Vec<Row>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("não consegui ler o corpus {}: {e}", path.display()))?;
    let mut rows = Vec::new();
    for (i, line) in content.lines().enumerate() {
        if limit.is_some_and(|n| rows.len() >= n) {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        let bad = || format!("linha {} do corpus inválida", i + 1);
        let value: Value = serde_json::from_str(line).map_err(|_| bad())?;
        let text = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        let (Some(raw), Some(formatted)) = (text("raw"), text("formatted")) else {
            return Err(bad());
        };
        let language = match text("lang").as_deref() {
            Some(lang) if lang.to_lowercase().starts_with("pt") => Language::PtBr,
            _ => Language::En,
        };
        rows.push(Row {
            raw,
            formatted,
            edited: text("edited"),
            // O nome que o `fala-inject` daria: o exe em minúsculas.
            app: text("app").map(|app| app.to_lowercase()),
            language,
        });
    }
    Ok(rows)
}

/// Levenshtein por caractere ÷ o maior comprimento; 0 quando os dois são vazios.
fn normalized_distance(a: &str, b: &str) -> f64 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let longest = a.len().max(b.len());
    if longest == 0 {
        return 0.0;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substitute = prev[j] + usize::from(ca != cb);
            cur[j + 1] = substitute.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()] as f64 / longest as f64
}

/// Percentil pelo posto mais próximo, em ms inteiros; `-` sem amostras.
fn percentile_ms(samples: &mut [Duration], p: usize) -> String {
    if samples.is_empty() {
        return "-".to_string();
    }
    samples.sort();
    let rank = (p * samples.len()).div_ceil(100).max(1);
    samples[rank - 1].as_millis().to_string()
}

/// ` (timeout 2, http 429 1)`, na ordem em que apareceram; vazio sem fallback.
fn summarize(fallbacks: &[Fallback]) -> String {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for fallback in fallbacks {
        let name = fallback.to_string();
        match counts.iter_mut().find(|(n, _)| *n == name) {
            Some((_, count)) => *count += 1,
            None => counts.push((name, 1)),
        }
    }
    if counts.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = counts.iter().map(|(n, c)| format!("{n} {c}")).collect();
    format!(" ({})", parts.join(", "))
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use fala_secrets::{ApiKey, MemoryStore};

    use super::*;
    use crate::format::tests::answering_server;

    /// Três linhas: uma igual depois das regras, uma diferente, e a terceira acima de 15
    /// palavras, a única que o LLM formata.
    const CORPUS: &str = concat!(
        r#"{"id":"1","raw":"ahn ok","formatted":"Ok","pasted":"Ok","edited":"Ok!","app":"Slack","lang":"pt","words":2}"#,
        "\n",
        r#"{"id":"2","raw":"bom dia","formatted":"Bom dia.","pasted":null,"edited":null,"app":null,"lang":"pt","words":2}"#,
        "\n",
        r#"{"id":"3","raw":"eu acho que a gente pode mandar o relatório amanhã cedo para o time inteiro hoje","formatted":"Texto do LLM.","pasted":null,"edited":null,"app":"WindowsTerminal","lang":"pt","words":16}"#,
        "\n",
    );

    struct Outcome {
        code: u8,
        stdout: String,
        stderr: String,
    }

    fn bench(store: &dyn SecretStore, corpus: &str, argv: &[&str]) -> Outcome {
        #[derive(Parser)]
        struct Wrapper {
            #[command(flatten)]
            args: FormatBenchArgs,
        }
        let dir = std::env::temp_dir().join(format!(
            "fala-cli-bench-format-{}-{}",
            std::process::id(),
            argv.join("_").replace([':', '/', '.'], "")
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("corpus.jsonl");
        std::fs::write(&path, corpus).unwrap();
        let mut full = vec!["format", "--corpus", path.to_str().unwrap()];
        full.extend_from_slice(argv);
        let args = Wrapper::try_parse_from(full).unwrap().args;
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(args, store, &mut out, &mut err);
        std::fs::remove_dir_all(&dir).unwrap();
        Outcome {
            code,
            stdout: String::from_utf8(out).unwrap(),
            stderr: String::from_utf8(err).unwrap(),
        }
    }

    fn row<'a>(stdout: &'a str, prefix: &str) -> &'a str {
        stdout
            .lines()
            .find(|line| line.starts_with(prefix))
            .unwrap_or_else(|| panic!("sem `{prefix}` em {stdout}"))
    }

    fn corpus_words() -> Vec<String> {
        CORPUS
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.chars().count() > 3)
            .map(str::to_lowercase)
            .filter(|w| {
                ![
                    "formatted",
                    "pasted",
                    "edited",
                    "lang",
                    "words",
                    "null",
                    "slack",
                ]
                .contains(&w.as_str())
            })
            .collect()
    }

    #[test]
    fn rules_only_scores_by_hand() {
        let out = bench(&MemoryStore::default(), CORPUS, &[]);
        assert_eq!(out.code, 0, "{}", out.stderr);
        assert!(
            out.stdout
                .starts_with("corpus=corpus.jsonl rows=3 mode=regras level=light model=- llm=0 late=0 fallback=0\n"),
            "{}",
            out.stdout
        );
        // Regras: "Ok" (igual), "Bom dia" contra "Bom dia." (1/8) e a linha 3 contra
        // "Texto do LLM." (73/80): média 0,3458.
        assert_eq!(
            row(&out.stdout, "| fala | formatted "),
            "| fala | formatted | 3 | 33.33 | 0.3458 |"
        );
        // "Ok" contra "Ok!": 1/3.
        assert_eq!(
            row(&out.stdout, "| fala | edited "),
            "| fala | edited | 1 | 0.00 | 0.3333 |"
        );
        // Bruto: "ahn ok" contra "Ok" (5/6), "bom dia" contra "Bom dia." (2/8), linha 3 (73/80).
        assert_eq!(
            row(&out.stdout, "| bruto | formatted "),
            "| bruto | formatted | 3 | 0.00 | 0.6653 |"
        );
        assert!(row(&out.stdout, "latência_ms: p50=").contains(" p90="));
        for word in corpus_words() {
            let shown = format!("{}{}", out.stdout, out.stderr).to_lowercase();
            assert!(!shown.contains(&word), "`{word}` vazou: {shown}");
        }
    }

    #[test]
    fn limit_counts_rows() {
        let out = bench(&MemoryStore::default(), CORPUS, &["--limit", "1"]);
        assert_eq!(out.code, 0);
        assert!(out.stdout.contains(" rows=1 "), "{}", out.stdout);
        assert_eq!(
            row(&out.stdout, "| fala | formatted "),
            "| fala | formatted | 1 | 100.00 | 0.0000 |"
        );
    }

    #[test]
    fn bad_corpus_exits_2_without_content() {
        let out = bench(
            &MemoryStore::default(),
            "{\"raw\":\"segredo dito\"}\n",
            &["--limit", "5"],
        );
        assert_eq!(out.code, 2);
        assert!(out.stderr.contains("linha 1"), "{}", out.stderr);
        assert!(!out.stderr.contains("segredo"), "{}", out.stderr);
        assert_eq!(out.stdout, "");
    }

    #[test]
    fn llm_without_key_exits_2() {
        let out = bench(&MemoryStore::default(), CORPUS, &["--llm"]);
        assert_eq!(out.code, 2);
        assert!(
            out.stderr.contains("fala-cli key set gemini"),
            "{}",
            out.stderr
        );
        assert_eq!(out.stdout, "");
    }

    #[test]
    fn llm_formats_long_rows_at_the_level() {
        let store = MemoryStore::default();
        store
            .set("gemini", &ApiKey::new("chave-de-teste").unwrap())
            .unwrap();
        let (url, seen) = answering_server("Texto do LLM.");
        let out = bench(
            &store,
            CORPUS,
            &["--llm", "--gemini-base-url", &url, "--level", "medium"],
        );
        assert_eq!(out.code, 0, "{}", out.stderr);
        assert!(
            out.stdout.contains(
                " mode=llm level=medium model=gemini-2.5-flash-lite llm=1 late=0 fallback=0\n"
            ),
            "{}",
            out.stdout
        );
        assert_eq!(
            row(&out.stdout, "| fala | formatted "),
            "| fala | formatted | 3 | 66.67 | 0.0417 |"
        );
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        let body: Value = serde_json::from_str(&seen[0].1).unwrap();
        let system = body["systemInstruction"]["parts"][0]["text"]
            .as_str()
            .unwrap();
        assert!(
            system.contains(CleanupLevel::Medium.instruction()),
            "{system}"
        );
        assert!(
            !system.contains(CleanupLevel::Light.instruction()),
            "{system}"
        );
        let user = body["contents"][0]["parts"][0]["text"].as_str().unwrap();
        assert!(user.contains("<destination>prompt</destination>"), "{user}");
        assert!(!out.stdout.contains("chave-de-teste"));
    }

    #[test]
    fn distance_and_percentiles() {
        assert_eq!(normalized_distance("", ""), 0.0);
        assert_eq!(normalized_distance("abc", "abc"), 0.0);
        assert_eq!(normalized_distance("abc", ""), 1.0);
        assert_eq!(normalized_distance("kitten", "sitting"), 3.0 / 7.0);
        let mut samples: Vec<Duration> = (1..=10).map(Duration::from_millis).collect();
        assert_eq!(percentile_ms(&mut samples, 50), "5");
        assert_eq!(percentile_ms(&mut samples, 90), "9");
        assert_eq!(percentile_ms(&mut [], 50), "-");
        assert_eq!(
            summarize(&[Fallback::Timeout, Fallback::Http(429), Fallback::Timeout]),
            " (timeout 2, http 429 1)"
        );
    }
}
