//! O payload do LLM de notas (ADR-0016, proposta): o único construtor do que sai da máquina.

use fala_core::Language;
use serde::Serialize;

use crate::{NotesError, NotesInput, Speaker};

/// O que vai ao LLM de notas, e nada além: título, início, idioma, dicionário, template,
/// anotações com id por linha e transcrição com id, timestamps e rótulo de falante. Os campos
/// são privados; [`NotesPayload::build`] é a única forma de criar um.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NotesPayload {
    title: String,
    started_at: String,
    language: Language,
    dictionary: Vec<String>,
    template: TemplatePayload,
    annotations: Vec<AnnotationPayload>,
    transcript: Vec<SegmentPayload>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct TemplatePayload {
    name: String,
    purpose: String,
    style: String,
    sections: Vec<SectionPayload>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SectionPayload {
    title: String,
    instruction: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct AnnotationPayload {
    id: String,
    text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SegmentPayload {
    id: String,
    t0_ms: u64,
    t1_ms: u64,
    speaker: String,
    text: String,
}

impl NotesPayload {
    /// Valida a entrada e monta o payload. O `channel` dos segmentos e a marca "só local" não
    /// entram.
    pub fn build(input: &NotesInput) -> Result<Self, NotesError> {
        input.validate()?;
        let template = &input.template;
        Ok(Self {
            title: input.title.clone(),
            started_at: format!("{}T{}", input.date, input.start_time),
            language: input.language,
            dictionary: input.dictionary.terms().to_vec(),
            template: TemplatePayload {
                name: template.name.clone(),
                purpose: template.purpose.clone(),
                style: template.style.clone(),
                sections: template
                    .sections
                    .iter()
                    .map(|section| SectionPayload {
                        title: section.title.clone(),
                        instruction: section.instruction.clone(),
                    })
                    .collect(),
            },
            annotations: input
                .annotation_lines()
                .into_iter()
                .enumerate()
                .map(|(index, text)| AnnotationPayload {
                    id: annotation_id(index + 1),
                    text: text.to_string(),
                })
                .collect(),
            transcript: input
                .transcript
                .iter()
                .map(|segment| SegmentPayload {
                    id: segment_id(segment.id),
                    t0_ms: segment.t0_ms,
                    t1_ms: segment.t1_ms,
                    speaker: speaker_label(&segment.speaker, input.language),
                    text: segment.text.clone(),
                })
                .collect(),
        })
    }

    /// O JSON que vai como texto da mensagem do usuário.
    pub fn to_json(&self) -> Result<String, NotesError> {
        serde_json::to_string(self)
            .map_err(|_| NotesError::InvalidInput("payload não serializável".to_string()))
    }
}

pub(crate) fn segment_id(id: u32) -> String {
    format!("s{id}")
}

pub(crate) fn annotation_id(position: usize) -> String {
    format!("a{position}")
}

/// "Eu"/"Me", "Pessoa N"/"Person N" ou o nome aplicado.
pub(crate) fn speaker_label(speaker: &Speaker, language: Language) -> String {
    match (speaker, language) {
        (Speaker::Me, Language::PtBr) => "Eu".to_string(),
        (Speaker::Me, Language::En) => "Me".to_string(),
        (Speaker::Person(n), Language::PtBr) => format!("Pessoa {n}"),
        (Speaker::Person(n), Language::En) => format!("Person {n}"),
        (Speaker::Named(name), _) => name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::Value;

    use super::*;
    use crate::fixture::{input, segment};

    fn payload_json(input: &NotesInput) -> Value {
        serde_json::from_str(&NotesPayload::build(input).unwrap().to_json().unwrap()).unwrap()
    }

    fn keys(value: &Value) -> BTreeSet<&str> {
        value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect()
    }

    fn set(keys: &[&'static str]) -> BTreeSet<&'static str> {
        keys.iter().copied().collect()
    }

    /// Todas as chaves e todos os valores de texto, em qualquer profundidade.
    fn walk<'a>(value: &'a Value, keys: &mut Vec<&'a str>, strings: &mut Vec<&'a str>) {
        match value {
            Value::Object(map) => {
                for (key, inner) in map {
                    keys.push(key);
                    walk(inner, keys, strings);
                }
            }
            Value::Array(items) => items.iter().for_each(|inner| walk(inner, keys, strings)),
            Value::String(s) => strings.push(s),
            _ => {}
        }
    }

    #[test]
    fn payload_keys_are_exactly_the_enumerated_list() {
        let json = payload_json(&input());
        assert_eq!(
            keys(&json),
            set(&[
                "title",
                "started_at",
                "language",
                "dictionary",
                "template",
                "annotations",
                "transcript"
            ])
        );
        let template = &json["template"];
        assert_eq!(
            keys(template),
            set(&["name", "purpose", "style", "sections"])
        );
        let sections = template["sections"].as_array().unwrap();
        assert_eq!(sections.len(), 3);
        for section in sections {
            assert_eq!(keys(section), set(&["title", "instruction"]));
        }
        let annotations = json["annotations"].as_array().unwrap();
        assert_eq!(annotations.len(), 2);
        for annotation in annotations {
            assert_eq!(keys(annotation), set(&["id", "text"]));
        }
        let transcript = json["transcript"].as_array().unwrap();
        assert_eq!(transcript.len(), 3);
        for segment in transcript {
            assert_eq!(
                keys(segment),
                set(&["id", "t0_ms", "t1_ms", "speaker", "text"])
            );
        }
        assert_eq!(json["title"], "Planejamento do lançamento");
        assert_eq!(json["started_at"], "2026-10-02T14:02");
        assert_eq!(json["language"], "pt-BR");
        assert_eq!(json["dictionary"], serde_json::json!(["Fala", "Parakeet"]));
        assert_eq!(
            transcript[1],
            serde_json::json!({"id":"s12","t0_ms":65000,"t1_ms":69000,"speaker":"Pessoa 1","text":"Proponho 15 de novembro."})
        );
    }

    #[test]
    fn payload_never_carries_channel() {
        let json = payload_json(&input());
        let (mut all_keys, mut strings) = (Vec::new(), Vec::new());
        walk(&json, &mut all_keys, &mut strings);
        assert!(!all_keys.contains(&"channel"), "{all_keys:?}");
        for forbidden in ["mic", "system", "local_only"] {
            assert!(
                !strings.contains(&forbidden) && !all_keys.contains(&forbidden),
                "{forbidden}"
            );
        }
    }

    #[test]
    fn annotations_get_ids_skipping_blank_lines() {
        let json = payload_json(&input());
        assert_eq!(
            json["annotations"],
            serde_json::json!([{"id":"a1","text":"decidir data"},{"id":"a2","text":"Ana: contrato"}])
        );
    }

    #[test]
    fn speaker_labels_follow_language() {
        let cases = [
            (Speaker::Me, Language::PtBr, "Eu"),
            (Speaker::Person(2), Language::PtBr, "Pessoa 2"),
            (Speaker::Named("Ana".to_string()), Language::PtBr, "Ana"),
            (Speaker::Me, Language::En, "Me"),
            (Speaker::Person(2), Language::En, "Person 2"),
            (Speaker::Named("Ana".to_string()), Language::En, "Ana"),
        ];
        for (speaker, language, expected) in cases {
            let mut input = input();
            input.language = language;
            input.transcript = vec![segment(1, speaker.clone(), 0, "oi")];
            let json = payload_json(&input);
            assert_eq!(
                json["transcript"][0]["speaker"], expected,
                "{speaker:?} {language}"
            );
        }
    }
}
