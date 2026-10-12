use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::NotesError;

/// Versão do schema dos templates. Um campo novo sobe a versão e ganha migração.
pub const TEMPLATE_SCHEMA: u32 = 1;

/// Os templates genéricos versionados no repo (D10). Templates da empresa nunca entram aqui:
/// chegam como templates do usuário, lidos por [`Template::from_json`].
const BUILTIN: [&str; 2] = [
    include_str!("../templates/geral.json"),
    include_str!("../templates/um-a-um.json"),
];

/// Um template de notas, na estrutura do Granola: propósito e contexto em prosa, tamanho e
/// estilo, e seções com instrução própria.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Template {
    pub schema: u32,
    pub id: String,
    pub name: String,
    pub purpose: String,
    pub style: String,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub title: String,
    pub instruction: String,
}

impl Template {
    /// Lê um template JSON e recusa campo desconhecido, outra versão de schema ou nenhuma seção.
    pub fn from_json(json: &str) -> Result<Self, NotesError> {
        let template: Template = serde_json::from_str(json)
            .map_err(|error| NotesError::InvalidTemplate(error.to_string()))?;
        if template.schema != TEMPLATE_SCHEMA {
            return Err(NotesError::InvalidTemplate(format!(
                "schema {} não suportado",
                template.schema
            )));
        }
        if template.sections.is_empty() {
            return Err(NotesError::InvalidTemplate("nenhuma seção".to_string()));
        }
        Ok(template)
    }
}

/// `geral` e `um-a-um`, nessa ordem.
pub fn builtin_templates() -> Result<Vec<Template>, NotesError> {
    BUILTIN
        .iter()
        .map(|json| Template::from_json(json))
        .collect()
}

/// O `LEIAME.txt` da pasta de templates do usuário, editada fora do app.
pub const TEMPLATES_README: &str = "Cada arquivo .json desta pasta é um template de notas do Fala: schema (1), id (único), name, purpose (para que servem as notas), style (tom e tamanho) e sections, uma lista de {title, instruction}.\r\n\
Para começar, use \"Duplicar para editar\" num template embutido em Reuniões e edite a cópia; o Fala relê esta pasta quando você volta para ele.\r\n";

/// Um arquivo da pasta de templates que não virou template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateLoadError {
    /// O nome do arquivo, sem a pasta.
    pub file: String,
    pub error: String,
}

/// Os templates do usuário: cada `*.json` de `dir`, em ordem de nome do arquivo. Pasta ausente é lista
/// vazia. Um arquivo ilegível, inválido, de `id` vazio ou de `id` já usado (por um embutido ou
/// por um arquivo anterior) vira um erro com o nome do arquivo e não derruba os outros.
pub fn load_user_templates(dir: &Path) -> (Vec<Template>, Vec<TemplateLoadError>) {
    let mut templates: Vec<Template> = Vec::new();
    let mut errors = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return (templates, errors),
        Err(error) => {
            errors.push(TemplateLoadError {
                file: dir.display().to_string(),
                error: error.to_string(),
            });
            return (templates, errors);
        }
    };
    let mut files: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        })
        .collect();
    // Pelo nome sem extensão e sem caixa ("geral-copia" antes de "geral-copia-2"); o caminho
    // inteiro desempata, para a ordem não depender do `read_dir`.
    files.sort_by_cached_key(|path| {
        let stem = path.file_stem().unwrap_or_default();
        (stem.to_string_lossy().to_lowercase(), path.clone())
    });
    let builtin = builtin_ids();
    for path in files {
        let file = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let parsed = fs::read_to_string(&path)
            .map_err(|error| error.to_string())
            .and_then(|json| {
                // O Bloco de Notas pode salvar com BOM.
                Template::from_json(json.trim_start_matches('\u{feff}'))
                    .map_err(|error| error.to_string())
            })
            .and_then(|template| {
                if template.id.trim().is_empty() {
                    Err("id vazio".to_string())
                } else if builtin.contains(&template.id) {
                    Err(format!(
                        "o id \"{}\" é de um template embutido",
                        template.id
                    ))
                } else if templates.iter().any(|t| t.id == template.id) {
                    Err(format!("o id \"{}\" já está em outro arquivo", template.id))
                } else {
                    Ok(template)
                }
            });
        match parsed {
            Ok(template) => templates.push(template),
            Err(error) => {
                // A lista é relida a cada foco: a UI mostra o erro, o log só em debug.
                log::debug!("template do usuário recusado: {file}: {error}");
                errors.push(TemplateLoadError { file, error });
            }
        }
    }
    (templates, errors)
}

/// Cria a pasta de templates e o `LEIAME.txt`, se faltarem; nunca reescreve o `LEIAME.txt`.
pub fn ensure_templates_dir(dir: &Path) -> Result<(), NotesError> {
    fs::create_dir_all(dir).map_err(folder_error)?;
    write_new(&dir.join("LEIAME.txt"), TEMPLATES_README).map_err(folder_error)?;
    Ok(())
}

/// "Duplicar para editar": copia o embutido `id` para `dir` como `<id>-copia.json` (ou
/// `-copia-2`, …), com nome "<nome> (cópia)", e devolve a cópia. Nunca sobrescreve um arquivo.
pub fn duplicate_builtin(dir: &Path, id: &str) -> Result<Template, NotesError> {
    let source = builtin_templates()?
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| NotesError::InvalidTemplate(format!("{id} não é um template embutido")))?;
    ensure_templates_dir(dir)?;
    let (user, _) = load_user_templates(dir);
    let builtin = builtin_ids();
    for n in 1..100 {
        let (copy_id, name) = match n {
            1 => (format!("{id}-copia"), format!("{} (cópia)", source.name)),
            _ => (
                format!("{id}-copia-{n}"),
                format!("{} (cópia {n})", source.name),
            ),
        };
        if builtin.contains(&copy_id) || user.iter().any(|t| t.id == copy_id) {
            continue;
        }
        let copy = Template {
            id: copy_id,
            name,
            ..source.clone()
        };
        let json = serde_json::to_string_pretty(&copy)
            .map_err(|error| NotesError::TemplateFolder(error.to_string()))?;
        let path = dir.join(format!("{}.json", copy.id));
        if write_new(&path, &format!("{json}\n")).map_err(folder_error)? {
            return Ok(copy);
        }
    }
    Err(NotesError::TemplateFolder(format!("cópias demais de {id}")))
}

fn builtin_ids() -> Vec<String> {
    builtin_templates()
        .unwrap_or_default()
        .into_iter()
        .map(|t| t.id)
        .collect()
}

/// Escreve um arquivo novo; `false` se ele já existe (nada é sobrescrito).
fn write_new(path: &Path, text: &str) -> std::io::Result<bool> {
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => file.write_all(text.as_bytes()).map(|()| true),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => Ok(false),
        Err(error) => Err(error),
    }
}

fn folder_error(error: std::io::Error) -> NotesError {
    NotesError::TemplateFolder(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_templates_parse() {
        let templates = builtin_templates().unwrap();
        let ids: Vec<&str> = templates.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["geral", "um-a-um"]);
        let titles: Vec<&str> = templates[0]
            .sections
            .iter()
            .map(|s| s.title.as_str())
            .collect();
        assert_eq!(titles, ["Resumo", "Decisões", "Próximos passos"]);
        assert!(templates[0].sections[2].instruction.contains("Dono"));
        assert!(templates[0].sections[2].instruction.contains("prazo"));
        assert!(!templates[1].sections.is_empty());
        for template in &templates {
            assert_eq!(template.schema, TEMPLATE_SCHEMA);
            assert!(!template.purpose.is_empty() && !template.style.is_empty());
        }
    }

    fn user_json(id: &str) -> String {
        format!(
            r#"{{"schema":1,"id":"{id}","name":"Vendas","purpose":"p","style":"s","sections":[{{"title":"Dor","instruction":"x"}}]}}"#
        )
    }

    #[test]
    fn missing_or_empty_folder_has_no_user_templates() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            load_user_templates(&dir.path().join("templates")),
            (vec![], vec![])
        );
        assert_eq!(load_user_templates(dir.path()), (vec![], vec![]));
    }

    #[test]
    fn invalid_file_becomes_an_error_and_the_valid_one_loads() {
        let dir = tempfile::tempdir().unwrap();
        // Com BOM, como o Bloco de Notas pode salvar.
        fs::write(
            dir.path().join("b-vendas.json"),
            format!("\u{feff}{}", user_json("vendas")),
        )
        .unwrap();
        fs::write(dir.path().join("a-quebrado.json"), "{\"schema\":1,").unwrap();
        fs::write(dir.path().join("LEIAME.txt"), "não é template").unwrap();
        let (templates, errors) = load_user_templates(dir.path());
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].id, "vendas");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].file, "a-quebrado.json");
        assert!(!errors[0].error.is_empty());
    }

    #[test]
    fn colliding_id_loses_to_the_builtin_and_to_the_earlier_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.json"), user_json("geral")).unwrap();
        fs::write(dir.path().join("b.json"), user_json("vendas")).unwrap();
        fs::write(dir.path().join("c.JSON"), user_json("vendas")).unwrap();
        let (templates, errors) = load_user_templates(dir.path());
        let ids: Vec<&str> = templates.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["vendas"]);
        let files: Vec<&str> = errors.iter().map(|e| e.file.as_str()).collect();
        assert_eq!(files, ["a.json", "c.JSON"]);
        assert!(errors[0].error.contains("embutido"), "{}", errors[0].error);
    }

    #[test]
    fn duplicate_gets_a_new_id_and_name_and_loads_back() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("templates");
        let first = duplicate_builtin(&dir, "geral").unwrap();
        let second = duplicate_builtin(&dir, "geral").unwrap();
        assert_eq!(first.id, "geral-copia");
        assert_eq!(first.name, "Reunião geral (cópia)");
        assert_eq!(second.id, "geral-copia-2");
        let original = &builtin_templates().unwrap()[0];
        assert_eq!(first.sections, original.sections);
        let json = fs::read_to_string(dir.join("geral-copia.json")).unwrap();
        assert_eq!(Template::from_json(&json).unwrap(), first);
        assert_eq!(
            fs::read_to_string(dir.join("LEIAME.txt")).unwrap(),
            TEMPLATES_README
        );
        let (templates, errors) = load_user_templates(&dir);
        assert_eq!(templates, [first, second]);
        assert!(errors.is_empty());
        assert!(matches!(
            duplicate_builtin(&dir, "geral-copia"),
            Err(NotesError::InvalidTemplate(_))
        ));
    }

    #[test]
    fn readme_is_never_rewritten() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("LEIAME.txt"), "meu texto").unwrap();
        ensure_templates_dir(dir.path()).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("LEIAME.txt")).unwrap(),
            "meu texto"
        );
    }

    #[test]
    fn template_rejects_unknown_field_wrong_schema_and_no_sections() {
        let section = r#"[{"title":"Resumo","instruction":"x"}]"#;
        let valid = format!(
            r#"{{"schema":1,"id":"t","name":"T","purpose":"p","style":"s","sections":{section}}}"#
        );
        assert!(Template::from_json(&valid).is_ok());
        let cases = [
            format!(
                r#"{{"schema":1,"id":"t","name":"T","purpose":"p","style":"s","sections":{section},"prompt":"x"}}"#
            ),
            r#"{"schema":1,"id":"t","name":"T","purpose":"p","style":"s","sections":[{"title":"R","instruction":"x","extra":1}]}"#
                .to_string(),
            format!(
                r#"{{"schema":2,"id":"t","name":"T","purpose":"p","style":"s","sections":{section}}}"#
            ),
            r#"{"schema":1,"id":"t","name":"T","purpose":"p","style":"s","sections":[]}"#
                .to_string(),
        ];
        for json in cases {
            assert!(
                matches!(
                    Template::from_json(&json),
                    Err(NotesError::InvalidTemplate(_))
                ),
                "{json}"
            );
        }
    }
}
