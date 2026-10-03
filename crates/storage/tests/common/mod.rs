//! Helpers compartilhados pelos testes de integração do `fala-storage`.

#![allow(clippy::unwrap_used, clippy::expect_used, dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, FixedOffset, TimeZone};
use fala_core::{AppContext, Dictation, Editor, Language, Transcript};
use fala_storage::Store;

/// Pasta limpa por teste dentro do `CARGO_TARGET_TMPDIR`.
pub fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("storage")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub struct Env {
    pub dir: PathBuf,
    pub db: PathBuf,
    pub notes: PathBuf,
}

pub fn env(name: &str) -> Env {
    let dir = scratch(name);
    Env {
        db: dir.join("fala.sqlite"),
        notes: dir.join("notas"),
        dir,
    }
}

impl Env {
    pub fn open(&self) -> Store {
        Store::open(&self.db, &self.notes).unwrap()
    }

    pub fn ditados(&self) -> PathBuf {
        self.notes.join("Ditados")
    }
}

/// 2026-10-02 às `h:m:s` em -03:00.
pub fn at(h: u32, m: u32, s: u32) -> DateTime<FixedOffset> {
    FixedOffset::west_opt(3 * 3600)
        .unwrap()
        .with_ymd_and_hms(2026, 10, 2, h, m, s)
        .unwrap()
}

pub fn dictation(raw: &str, final_text: &str, editor: Editor, app: Option<&str>) -> Dictation {
    Dictation {
        raw: Transcript {
            text: raw.to_string(),
            language: Language::PtBr,
        },
        final_text: final_text.to_string(),
        editor,
        app: AppContext {
            app_name: app.map(str::to_string),
        },
    }
}

pub fn unedited(raw: &str) -> Dictation {
    dictation(raw, raw, Editor::None, None)
}

/// Todos os arquivos sob `dir`, recursivo.
pub fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(files_under(&path));
        } else {
            out.push(path);
        }
    }
    out.sort();
    out
}

/// O único `.md` de um id.
pub fn md_of(env: &Env, id: &str) -> PathBuf {
    let hits: Vec<_> = files_under(&env.ditados())
        .into_iter()
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(&format!("-{id}.md"))
        })
        .collect();
    assert_eq!(hits.len(), 1, "esperava um .md para {id}: {hits:?}");
    hits[0].clone()
}

/// Remove o banco e os arquivos do WAL.
pub fn delete_db(env: &Env) {
    for suffix in ["", "-wal", "-shm"] {
        let path = PathBuf::from(format!("{}{suffix}", env.db.display()));
        let _ = fs::remove_file(path);
    }
}
