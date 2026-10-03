//! O espelho Markdown de um item: `Ditados/<AAAA-MM-DD>/<HHMMSS>-<id>.md`.
//!
//! Frontmatter entre linhas `---`, uma chave por linha no formato `chave: <valor JSON>` (JSON é
//! YAML válido); o corpo é o texto final exato seguido de um `\n`. Cada valor JSON cabe numa
//! linha, então o primeiro `---` depois da abertura fecha o frontmatter, seja qual for o texto.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, SecondsFormat};
use fala_core::{AppContext, Dictation, Editor, Language, Transcript};
use serde_json::Value;

use crate::{DictationRecord, Showing};

pub(crate) const DITADOS: &str = "Ditados";

/// O caminho do `.md` de um item, na data e hora locais do `created_at`.
pub(crate) fn path_for(notes_dir: &Path, record: &DictationRecord) -> PathBuf {
    let at = record.created_at;
    notes_dir
        .join(DITADOS)
        .join(at.format("%Y-%m-%d").to_string())
        .join(format!("{}-{}.md", at.format("%H%M%S"), record.id))
}

/// O conteúdo do `.md` de um item.
pub(crate) fn render(record: &DictationRecord) -> String {
    let d = &record.dictation;
    let mut out = String::from("---\n");
    let lines: [(&str, Value); 7] = [
        ("id", Value::from(record.id.as_str())),
        ("created_at", Value::from(rfc3339(record))),
        ("language", Value::from(d.raw.language.tag())),
        (
            "app",
            d.app.app_name.as_deref().map_or(Value::Null, Value::from),
        ),
        ("edited_by", Value::from(editor_str(d.editor))),
        ("showing", Value::from(record.showing.as_str())),
        ("raw", Value::from(d.raw.text.as_str())),
    ];
    for (key, value) in lines {
        out.push_str(key);
        out.push_str(": ");
        out.push_str(&value.to_string());
        out.push('\n');
    }
    // Ausente = falso: só o item marcado ganha a chave, depois de `raw`.
    if record.sensitive {
        out.push_str("sensitive: true\n");
    }
    out.push_str("---\n");
    out.push_str(&d.final_text);
    out.push('\n');
    out
}

/// Escreve o `.md` por `<arquivo>.tmp` + rename, para nunca deixar um parcial com o nome final.
pub(crate) fn write(
    notes_dir: &Path,
    record: &DictationRecord,
) -> Result<(), (PathBuf, std::io::Error)> {
    let path = path_for(notes_dir, record);
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    let result = (|| {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut file = fs::File::create(&tmp)?;
        file.write_all(render(record).as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, &path)
    })();
    result.map_err(|e| (path, e))
}

/// Lê um `.md` de volta; o `Err` é o motivo legível para o `ReindexReport`.
pub(crate) fn parse(content: &str) -> Result<DictationRecord, String> {
    let rest = content
        .strip_prefix("---\n")
        .ok_or_else(|| "sem frontmatter".to_string())?;
    let mut keys: HashMap<&str, Value> = HashMap::new();
    let mut remaining = rest;
    let body = loop {
        let Some((line, tail)) = remaining.split_once('\n') else {
            return Err("frontmatter sem a linha `---` de fechamento".to_string());
        };
        if line == "---" {
            break tail;
        }
        let (key, raw) = line
            .split_once(':')
            .ok_or_else(|| "linha de frontmatter sem `chave:`".to_string())?;
        let value: Value = serde_json::from_str(raw.trim())
            .map_err(|e| format!("valor de `{}` não é JSON: {e}", key.trim()))?;
        keys.insert(key.trim(), value);
        remaining = tail;
    };
    let final_text = body
        .strip_suffix('\n')
        .ok_or_else(|| "corpo sem o `\\n` final".to_string())?;

    let string = |key: &str| -> Result<String, String> {
        match keys.get(key) {
            None => Err(format!("falta a chave `{key}`")),
            Some(Value::String(s)) => Ok(s.clone()),
            Some(_) => Err(format!("`{key}` não é texto")),
        }
    };
    let id = string("id")?;
    let parsed = uuid::Uuid::parse_str(&id).map_err(|_| format!("`id` não é UUID: {id:?}"))?;
    if parsed.hyphenated().to_string() != id {
        return Err(format!("`id` fora da forma minúscula com hífens: {id:?}"));
    }
    let created_at = DateTime::parse_from_rfc3339(&string("created_at")?)
        .map_err(|e| format!("`created_at` não é RFC 3339: {e}"))?;
    let language: Language = serde_json::from_value(Value::String(string("language")?))
        .map_err(|_| "`language` não é `pt-BR` nem `en`".to_string())?;
    let app_name = match keys.get("app") {
        None => return Err("falta a chave `app`".to_string()),
        Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => return Err("`app` não é texto nem null".to_string()),
    };
    let editor = parse_editor(&string("edited_by")?)
        .ok_or_else(|| "`edited_by` não é `none`, `rules` nem `llm`".to_string())?;
    let showing = Showing::parse(&string("showing")?)
        .ok_or_else(|| "`showing` não é `final` nem `raw`".to_string())?;
    let raw = string("raw")?;
    let sensitive = match keys.get("sensitive") {
        None | Some(Value::Bool(false)) => false,
        Some(Value::Bool(true)) => true,
        Some(_) => return Err("`sensitive` não é `true` nem `false`".to_string()),
    };

    Ok(DictationRecord {
        id,
        created_at,
        dictation: Dictation {
            raw: Transcript {
                text: raw,
                language,
            },
            final_text: final_text.to_string(),
            editor,
            app: AppContext { app_name },
        },
        showing,
        sensitive,
    })
}

pub(crate) fn rfc3339(record: &DictationRecord) -> String {
    record
        .created_at
        .to_rfc3339_opts(SecondsFormat::AutoSi, false)
}

pub(crate) fn editor_str(editor: Editor) -> &'static str {
    match editor {
        Editor::None => "none",
        Editor::Rules => "rules",
        Editor::Llm => "llm",
    }
}

pub(crate) fn parse_editor(s: &str) -> Option<Editor> {
    match s {
        "none" => Some(Editor::None),
        "rules" => Some(Editor::Rules),
        "llm" => Some(Editor::Llm),
        _ => None,
    }
}
