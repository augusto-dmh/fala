//! `fala-cli history` e `fala-cli reindex`: o histórico de ditado do `fala-storage` sem UI.

use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context};
use clap::{Args, Subcommand, ValueEnum};
use fala_core::{AppContext, Dictation, Editor, Language, Transcript};
use fala_storage::{DictationRecord, Store};

/// A pasta do identifier do `tauri.conf.json`, a mesma `app_data_dir` do desktop.
pub(crate) const APP_DIR: &str = "br.com.augusto.fala";

/// Falha com o código de saída do contrato: 1 banco ou espelho, 2 entrada inválida.
pub struct Failure {
    pub code: u8,
    pub error: anyhow::Error,
}

fn input(error: anyhow::Error) -> Failure {
    Failure { code: 2, error }
}

fn failed(error: anyhow::Error) -> Failure {
    Failure { code: 1, error }
}

#[derive(Args)]
pub struct DirArgs {
    /// Pasta do `fala.sqlite` (padrão: `<pasta de dados do SO>/br.com.augusto.fala`).
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    /// Pasta das notas, onde fica `Ditados/` (padrão: `<data-dir>/notas`).
    #[arg(long, global = true)]
    notes_dir: Option<PathBuf>,
}

#[derive(Args)]
pub struct HistoryArgs {
    #[command(flatten)]
    dirs: DirArgs,
    #[command(subcommand)]
    command: HistoryCommand,
}

#[derive(Subcommand)]
enum HistoryCommand {
    /// Grava um ditado e imprime o id.
    Add {
        /// Texto bruto do ASR.
        #[arg(long)]
        raw: String,
        /// Texto final; exige `--edited-by`. Sem ele, o final é o bruto.
        #[arg(long = "final", requires = "edited_by")]
        final_text: Option<String>,
        /// Quem produziu o final.
        #[arg(long, value_enum, requires = "final_text")]
        edited_by: Option<EditedBy>,
        /// App ativo durante o ditado.
        #[arg(long)]
        app: Option<String>,
        /// Idioma do bruto: `pt-BR` (ou `pt`) ou `en`.
        #[arg(long, default_value = "pt-BR", value_parser = parse_language)]
        language: Language,
    },
    /// Busca sem acento no bruto e no final; sem consulta, lista os mais recentes.
    Search {
        /// Termos; cada um casa como prefixo e todos precisam casar.
        query: Option<String>,
        /// Máximo de itens.
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Desfaz a edição: o item passa a mostrar o bruto.
    Undo { id: String },
    /// Reaplica a edição: o item volta a mostrar o final.
    Redo { id: String },
}

#[derive(Clone, Copy, ValueEnum)]
enum EditedBy {
    Rules,
    Llm,
}

fn parse_language(text: &str) -> Result<Language, String> {
    text.parse::<Language>().map_err(|e| e.to_string())
}

/// Caminhos resolvidos a partir das flags.
#[derive(Debug, PartialEq, Eq)]
struct Paths {
    db: PathBuf,
    notes: PathBuf,
}

fn resolve(dirs: &DirArgs, os_data_dir: Option<PathBuf>) -> Result<Paths, Failure> {
    let data = match &dirs.data_dir {
        Some(dir) => dir.clone(),
        None => os_data_dir
            .ok_or_else(|| input(anyhow!("sem pasta de dados no SO; passe --data-dir")))?
            .join(APP_DIR),
    };
    Ok(Paths {
        db: data.join("fala.sqlite"),
        notes: dirs.notes_dir.clone().unwrap_or_else(|| data.join("notas")),
    })
}

fn open(paths: &Paths) -> Result<Store, Failure> {
    fs::create_dir_all(&paths.notes)
        .with_context(|| format!("criar {}", paths.notes.display()))
        .map_err(failed)?;
    Store::open(&paths.db, &paths.notes)
        .with_context(|| format!("abrir {}", paths.db.display()))
        .map_err(failed)
}

pub fn run(args: HistoryArgs) -> Result<(), Failure> {
    let paths = resolve(&args.dirs, dirs::data_dir())?;
    let store = open(&paths)?;
    match args.command {
        HistoryCommand::Add {
            raw,
            final_text,
            edited_by,
            app,
            language,
        } => {
            let raw = Transcript {
                text: raw,
                language,
            };
            let app = AppContext { app_name: app };
            let dictation = match (final_text, edited_by) {
                (Some(final_text), Some(editor)) => Dictation {
                    raw,
                    final_text,
                    editor: match editor {
                        EditedBy::Rules => Editor::Rules,
                        EditedBy::Llm => Editor::Llm,
                    },
                    app,
                },
                _ => Dictation::unedited(raw, app),
            };
            let now = chrono::Local::now().fixed_offset();
            let record = store.add(&dictation, now).map_err(|e| failed(e.into()))?;
            println!("{}", record.id);
        }
        HistoryCommand::Search { query, limit } => {
            let hits = store
                .search(query.as_deref().unwrap_or(""), limit)
                .map_err(|e| failed(e.into()))?;
            print!("{}", table(&hits));
        }
        HistoryCommand::Undo { id } => {
            let record = store.undo(&id).map_err(|e| failed(e.into()))?;
            println!("{}", record.shown_text());
        }
        HistoryCommand::Redo { id } => {
            let record = store.redo(&id).map_err(|e| failed(e.into()))?;
            println!("{}", record.shown_text());
        }
    }
    Ok(())
}

/// `fala-cli reindex`: reconstrói `fala.sqlite` a partir de `<notes-dir>/Ditados/`.
pub fn reindex(dirs: DirArgs) -> Result<(), Failure> {
    let paths = resolve(&dirs, dirs::data_dir())?;
    let ditados = paths.notes.join("Ditados");
    if !ditados.is_dir() {
        return Err(input(anyhow!("{} não existe", ditados.display())));
    }
    let mut store = Store::open(&paths.db, &paths.notes)
        .with_context(|| format!("abrir {}", paths.db.display()))
        .map_err(failed)?;
    let report = store.reindex().map_err(|e| failed(e.into()))?;
    for skip in &report.skipped {
        log::warn!("{}: {}", skip.path.display(), skip.reason);
    }
    println!(
        "{} ditados reindexados, {} arquivos ignorados",
        report.indexed,
        report.skipped.len()
    );
    if report.skipped.is_empty() {
        Ok(())
    } else {
        Err(failed(anyhow!(
            "{} arquivos ignorados em {}",
            report.skipped.len(),
            ditados.display()
        )))
    }
}

/// Tabela Markdown: uma linha por item, com o texto que o item mostra.
fn table(hits: &[DictationRecord]) -> String {
    let mut out = String::from("| id | quando | app | editado_por | mostrando | texto |\n");
    out.push_str("| --- | --- | --- | --- | --- | --- |\n");
    for r in hits {
        let editor = match r.dictation.editor {
            Editor::None => "none",
            Editor::Rules => "rules",
            Editor::Llm => "llm",
        };
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            r.id,
            r.created_at.format("%Y-%m-%d %H:%M:%S"),
            cell(r.dictation.app.app_name.as_deref().unwrap_or("")),
            editor,
            r.showing.as_str(),
            cell(r.shown_text()),
        ));
    }
    out
}

fn cell(text: &str) -> String {
    text.replace("\r\n", " ")
        .replace(['\n', '\r'], " ")
        .replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirs(data: Option<&str>, notes: Option<&str>) -> DirArgs {
        DirArgs {
            data_dir: data.map(PathBuf::from),
            notes_dir: notes.map(PathBuf::from),
        }
    }

    #[test]
    fn paths_resolve_from_flags_and_data_dir() {
        let os = Some(PathBuf::from("/os/data"));
        let p = resolve(&dirs(None, None), os.clone()).ok().unwrap();
        assert_eq!(
            p,
            Paths {
                db: PathBuf::from("/os/data/br.com.augusto.fala/fala.sqlite"),
                notes: PathBuf::from("/os/data/br.com.augusto.fala/notas"),
            }
        );
        let p = resolve(&dirs(Some("/d"), None), os.clone()).ok().unwrap();
        assert_eq!(p.db, PathBuf::from("/d/fala.sqlite"));
        assert_eq!(p.notes, PathBuf::from("/d/notas"));
        let p = resolve(&dirs(Some("/d"), Some("/n")), os).ok().unwrap();
        assert_eq!(p.db, PathBuf::from("/d/fala.sqlite"));
        assert_eq!(p.notes, PathBuf::from("/n"));

        // O padrão real vem de `dirs::data_dir()`.
        let real = resolve(&dirs(None, None), dirs::data_dir()).ok().unwrap();
        assert_eq!(
            real.db,
            dirs::data_dir()
                .unwrap()
                .join("br.com.augusto.fala")
                .join("fala.sqlite")
        );
    }

    #[test]
    fn cells_escape_pipes_and_newlines() {
        assert_eq!(cell("a | b\r\nc\nd\re"), "a \\| b c d e");
    }
}
