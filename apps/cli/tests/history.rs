//! `fala-cli history` e `fala-cli reindex` pela fronteira: roda o binário e confere exit code,
//! stdout, stderr e o que ficou em disco.

// `allow-unwrap-in-tests` não cobre os helpers de um crate de teste de integração.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const HEADER: &str = "| id | quando | app | editado_por | mostrando | texto |";
const SEPARATOR: &str = "| --- | --- | --- | --- | --- | --- |";

struct Dirs {
    data: PathBuf,
}

impl Dirs {
    fn new(name: &str) -> Self {
        let data = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("history")
            .join(name);
        let _ = fs::remove_dir_all(&data);
        Dirs { data }
    }

    fn db(&self) -> PathBuf {
        self.data.join("fala.sqlite")
    }

    fn ditados(&self) -> PathBuf {
        self.data.join("notas").join("Ditados")
    }

    fn run(&self, args: &[&str]) -> Output {
        run_with(&self.data, args, &[])
    }

    fn add(&self, args: &[&str]) -> String {
        let mut all = vec!["history", "add"];
        all.extend_from_slice(args);
        let o = self.run(&all);
        assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
        stdout(&o).trim().to_string()
    }

    fn delete_db(&self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = fs::remove_file(format!("{}{suffix}", self.db().display()));
        }
    }

    fn md_files(&self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        let Ok(days) = fs::read_dir(self.ditados()) else {
            return out;
        };
        for day in days {
            for file in fs::read_dir(day.unwrap().path()).unwrap() {
                out.push(file.unwrap().path());
            }
        }
        out.sort();
        out
    }
}

fn run_with(data: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fala-cli"));
    cmd.args(args).arg("--data-dir").arg(data);
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8(o.stderr.clone()).unwrap()
}

fn is_uuid(s: &str) -> bool {
    s.len() == 36
        && s.chars().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                c == '-'
            } else {
                c.is_ascii_hexdigit() && !c.is_ascii_uppercase()
            }
        })
}

#[test]
fn add_without_final_prints_id() {
    let d = Dirs::new("add_without_final_prints_id");
    let o = d.run(&["history", "add", "--raw", "compra pão"]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let out = stdout(&o);
    assert_eq!(out.lines().count(), 1, "{out:?}");
    let id = out.trim();
    assert!(is_uuid(id), "{id:?}");
    let md = fs::read_to_string(&d.md_files()[0]).unwrap();
    assert!(md.contains("\nedited_by: \"none\"\n"), "{md}");
    assert!(md.contains("\nraw: \"compra pão\"\n"), "{md}");
    assert!(md.ends_with("---\ncompra pão\n"), "{md}");
}

#[test]
fn add_final_and_editor_go_together() {
    let d = Dirs::new("add_final_and_editor_go_together");
    let o = d.run(&["history", "add", "--raw", "a", "--final", "A."]);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    let o = d.run(&["history", "add", "--raw", "a", "--edited-by", "llm"]);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(!d.db().exists());
}

#[test]
fn search_prints_markdown_table() {
    let d = Dirs::new("search_prints_markdown_table");
    let edited = d.add(&[
        "--raw",
        "acao a | b",
        "--final",
        "Ação a | b\nsegunda linha",
        "--edited-by",
        "llm",
        "--app",
        "Slack",
    ]);
    let plain = d.add(&["--raw", "outra acao"]);
    d.add(&["--raw", "nada a ver"]);

    let o = d.run(&["history", "search", "acao"]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let out = stdout(&o);
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines.len(), 4, "{out}");
    assert_eq!(lines[0], HEADER);
    assert_eq!(lines[1], SEPARATOR);
    let row_of = |id: &str| -> Vec<String> {
        let line = lines.iter().find(|l| l.contains(id)).unwrap();
        line.trim_matches('|')
            .split(" | ")
            .map(|c| c.trim().to_string())
            .collect()
    };
    let row = row_of(&edited);
    assert_eq!(row[0], edited);
    assert_eq!(row[2], "Slack");
    assert_eq!(row[3], "llm");
    assert_eq!(row[4], "final");
    assert_eq!(row[5], "Ação a \\| b segunda linha");
    let row = row_of(&plain);
    assert_eq!(row[3], "none");
    assert_eq!(row[5], "outra acao");

    d.run(&["history", "undo", &edited]);
    let out = stdout(&d.run(&["history", "search", "acao"]));
    let line = out.lines().find(|l| l.contains(&edited)).unwrap();
    assert!(line.contains("| raw | acao a \\| b |"), "{line}");
}

#[test]
fn search_without_hits_prints_header_only() {
    let d = Dirs::new("search_without_hits_prints_header_only");
    d.add(&["--raw", "alguma coisa"]);
    let o = d.run(&["history", "search", "inexistente"]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(stdout(&o), format!("{HEADER}\n{SEPARATOR}\n"));
}

#[test]
fn undo_and_redo_print_shown_text() {
    let d = Dirs::new("undo_and_redo_print_shown_text");
    let id = d.add(&[
        "--raw",
        "acho que sim",
        "--final",
        "Acho que sim.",
        "--edited-by",
        "rules",
    ]);
    let o = d.run(&["history", "undo", &id]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(stdout(&o), "acho que sim\n");
    let o = d.run(&["history", "redo", &id]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(stdout(&o), "Acho que sim.\n");
}

#[test]
fn undo_failures_exit_1() {
    let d = Dirs::new("undo_failures_exit_1");
    let plain = d.add(&["--raw", "sem edição"]);
    for (id, reason) in [
        ("0199a3f2-5c1e-7b3a-9d4e-2f6a8c0b1e27", "não encontrado"),
        (plain.as_str(), "não tem edição"),
    ] {
        let o = d.run(&["history", "undo", id]);
        assert_eq!(o.status.code(), Some(1), "{id}: {}", stderr(&o));
        assert!(stdout(&o).is_empty(), "{id}: {}", stdout(&o));
        assert!(stderr(&o).contains(id), "{id}: {}", stderr(&o));
        assert!(stderr(&o).contains(reason), "{id}: {}", stderr(&o));
    }
}

#[test]
fn reindex_reports_counts() {
    let d = Dirs::new("reindex_reports_counts");
    d.add(&["--raw", "um"]);
    d.add(&["--raw", "dois"]);
    d.delete_db();
    let o = d.run(&["reindex"]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(stdout(&o), "2 ditados reindexados, 0 arquivos ignorados\n");

    let day = d.md_files()[0].parent().unwrap().to_path_buf();
    let bad = day.join("quebrado.md");
    fs::write(&bad, "sem frontmatter\n").unwrap();
    d.delete_db();
    let o = d.run(&["reindex"]);
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert_eq!(stdout(&o), "2 ditados reindexados, 1 arquivos ignorados\n");
    assert!(
        stderr(&o).contains(&format!("{}: ", bad.display())),
        "{}",
        stderr(&o)
    );
    let o = d.run(&["history", "search", ""]);
    assert_eq!(stdout(&o).lines().count(), 4);
}

#[test]
fn reindex_without_ditados_exits_2() {
    let d = Dirs::new("reindex_without_ditados_exits_2");
    let o = d.run(&["reindex"]);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(!d.db().exists());
}

#[test]
fn dictated_text_stays_out_of_logs() {
    let d = Dirs::new("dictated_text_stays_out_of_logs");
    let env = [("RUST_LOG", "info")];
    let secret_raw = "palavrasecretabruta";
    let secret_final = "Palavrasecretafinal.";
    let o = run_with(
        &d.data,
        &[
            "history",
            "add",
            "--raw",
            secret_raw,
            "--final",
            secret_final,
            "--edited-by",
            "llm",
        ],
        &env,
    );
    assert_eq!(o.status.code(), Some(0));
    let id = stdout(&o).trim().to_string();
    let mut errs = stderr(&o);
    errs += &stderr(&run_with(
        &d.data,
        &["history", "search", "palavrasecretabruta"],
        &env,
    ));
    errs += &stderr(&run_with(&d.data, &["history", "undo", &id], &env));
    d.delete_db();
    errs += &stderr(&run_with(&d.data, &["reindex"], &env));
    assert!(!errs.to_lowercase().contains("palavrasecreta"), "{errs}");
}

#[test]
fn add_language_defaults_and_validation() {
    let d = Dirs::new("add_language_defaults_and_validation");
    d.add(&["--raw", "olá"]);
    d.add(&["--raw", "hello", "--language", "EN"]);
    let all: String = d
        .md_files()
        .iter()
        .map(|p| fs::read_to_string(p).unwrap())
        .collect();
    assert!(all.contains("\nlanguage: \"pt-BR\"\napp: null\nedited_by: \"none\"\nshowing: \"final\"\nraw: \"olá\"\n"), "{all}");
    assert!(all.contains("\nlanguage: \"en\"\napp: null\nedited_by: \"none\"\nshowing: \"final\"\nraw: \"hello\"\n"), "{all}");
    let o = d.run(&["history", "add", "--raw", "hola", "--language", "es"]);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
}

#[test]
fn add_mirror_failure_exits_1() {
    let d = Dirs::new("add_mirror_failure_exits_1");
    fs::create_dir_all(d.data.join("notas")).unwrap();
    fs::write(d.ditados(), "não é pasta").unwrap();
    let o = d.run(&["history", "add", "--raw", "algo"]);
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    let err = stderr(&o);
    let id = err
        .split(|c: char| !(c.is_ascii_hexdigit() || c == '-'))
        .find(|w| is_uuid(w))
        .unwrap_or_else(|| panic!("nenhum id no stderr: {err}"));
    assert!(d.db().exists(), "{id}");
}

#[test]
fn search_failures_exit_1_and_2() {
    let d = Dirs::new("search_failures_exit_1_and_2");
    fs::create_dir_all(d.data.parent().unwrap()).unwrap();
    fs::write(&d.data, "arquivo, não pasta").unwrap();
    let o = d.run(&["history", "search", "x"]);
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    fs::remove_file(&d.data).unwrap();
    let o = d.run(&["history", "search", "x", "--limit", "abc"]);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
}

#[test]
fn undo_without_id_exits_2() {
    let d = Dirs::new("undo_without_id_exits_2");
    for sub in ["undo", "redo"] {
        let o = d.run(&["history", sub]);
        assert_eq!(o.status.code(), Some(2), "{sub}: {}", stderr(&o));
    }
}
