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
