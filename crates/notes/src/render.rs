//! O Markdown das notas: as anotações como bloco humano, cada linha gerada com o marcador e um
//! ponteiro por fonte, no formato de link de bloco do Obsidian (`[[#^s12|01:05]]`).

use std::collections::{HashMap, HashSet};

use fala_core::Language;
use serde::Deserialize;

use crate::payload::{annotation_id, segment_id, speaker_label};
use crate::{NotesError, NotesInput, Segment, Speaker};

/// Fim de toda linha gerada pelo LLM. Comentário HTML: o Obsidian e os renderizadores CommonMark
/// não o mostram, e a UI o usa para pintar a linha.
pub const GENERATED_MARKER: &str = "<!-- fala:ia -->";

/// O resultado de uma geração.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notes {
    pub markdown: String,
    /// `false` numa sessão "só local": o Markdown tem só as anotações.
    pub generated: bool,
    /// Ponteiros citados pelo LLM que não existem na entrada, descartados.
    pub dropped_sources: usize,
    /// Linhas geradas que ficaram sem nenhuma fonte válida.
    pub unsourced_lines: usize,
    /// O título que o LLM propôs, só quando a sessão não tem título; nunca troca um título do
    /// usuário.
    pub title: Option<String>,
    /// Um nome por "Pessoa N", para a pessoa aplicar com um clique; nada aqui muda os rótulos.
    pub speaker_suggestions: Vec<SpeakerSuggestion>,
}

/// "Pessoa `person`" talvez seja `name`, porque `reason`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeakerSuggestion {
    pub person: u32,
    /// Um dos participantes, na grafia que o usuário digitou.
    pub name: String,
    /// Uma frase, sem quebra de linha.
    pub reason: String,
}

/// Teto do título gerado, em caracteres.
const TITLE_MAX_CHARS: usize = 80;
/// Teto da justificativa de uma sugestão, em caracteres.
const REASON_MAX_CHARS: usize = 200;

/// O JSON que o prompt pede ao LLM. `title` e `speaker_suggestions` são opcionais na leitura:
/// sem eles, as notas saem como antes.
#[derive(Debug, Deserialize)]
pub(crate) struct Response {
    #[serde(default)]
    title: Option<String>,
    sections: Vec<ResponseSection>,
    #[serde(default)]
    speaker_suggestions: Vec<ResponseSuggestion>,
}

#[derive(Debug, Deserialize)]
struct ResponseSuggestion {
    /// Lido largo: um número fora de `u32` descarta só a sugestão, não as notas.
    person: i64,
    name: String,
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Deserialize)]
struct ResponseSection {
    title: String,
    lines: Vec<ResponseLine>,
}

#[derive(Debug, Deserialize)]
struct ResponseLine {
    text: String,
    sources: Vec<String>,
}

pub(crate) fn parse_response(raw: &str) -> Result<Response, NotesError> {
    let raw = raw.trim();
    let raw = raw
        .strip_prefix("```json")
        .or_else(|| raw.strip_prefix("```"))
        .and_then(|inner| inner.strip_suffix("```"))
        .unwrap_or(raw);
    serde_json::from_str(raw).map_err(|_| NotesError::InvalidResponse)
}

struct Labels {
    annotations: &'static str,
    generated: &'static str,
}

fn labels(language: Language) -> Labels {
    match language {
        Language::PtBr => Labels {
            annotations: "Anotações",
            generated: "Notas",
        },
        Language::En => Labels {
            annotations: "Notes",
            generated: "AI notes",
        },
    }
}

/// `mm:ss`, ou `h:mm:ss` a partir de uma hora.
fn timestamp(ms: u64) -> String {
    let seconds = ms / 1_000;
    let (hours, minutes, seconds) = (seconds / 3_600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

/// Uma linha só, sem o marcador: quebras viram espaço.
fn one_line(text: &str) -> String {
    text.replace(GENERATED_MARKER, " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// [`one_line`] cortada em `max` caracteres.
fn short_line(text: &str, max: usize) -> String {
    let line: String = one_line(text).chars().take(max).collect();
    line.trim_end().to_string()
}

/// O título da resposta, só se a sessão está sem título.
fn generated_title(input: &NotesInput, response: &Response) -> Option<String> {
    if !input.title.trim().is_empty() {
        return None;
    }
    let title = short_line(response.title.as_deref()?, TITLE_MAX_CHARS);
    (!title.is_empty()).then_some(title)
}

/// As sugestões que valem: para uma "Pessoa N" que está na transcrição, com um nome dos
/// participantes que ainda não é rótulo de ninguém, uma por pessoa e uma por nome.
fn speaker_suggestions(input: &NotesInput, response: &Response) -> Vec<SpeakerSuggestion> {
    let participants = input.participant_names();
    let mut people = HashSet::new();
    let mut taken = HashSet::new();
    for segment in &input.transcript {
        match &segment.speaker {
            Speaker::Person(n) => {
                people.insert(*n);
            }
            Speaker::Named(name) => {
                taken.insert(name.to_lowercase());
            }
            Speaker::Me | Speaker::Omitted => {}
        }
    }
    let mut out: Vec<SpeakerSuggestion> = Vec::new();
    for suggestion in &response.speaker_suggestions {
        let Ok(person) = u32::try_from(suggestion.person) else {
            continue;
        };
        if !people.contains(&person) || out.iter().any(|s| s.person == person) {
            continue;
        }
        let wanted = one_line(&suggestion.name).to_lowercase();
        let Some(name) = participants.iter().find(|p| p.to_lowercase() == wanted) else {
            continue;
        };
        if taken.contains(&wanted) || out.iter().any(|s| s.name == *name) {
            continue;
        }
        out.push(SpeakerSuggestion {
            person,
            name: name.clone(),
            reason: short_line(&suggestion.reason, REASON_MAX_CHARS),
        });
    }
    out
}

pub(crate) fn annotations_block(input: &NotesInput) -> String {
    let lines = input.annotation_lines();
    if lines.is_empty() {
        return String::new();
    }
    let mut out = format!("## {}\n", labels(input.language).annotations);
    for (index, line) in lines.iter().enumerate() {
        out.push_str(&format!("\n{line} ^{}\n", annotation_id(index + 1)));
    }
    out
}

pub(crate) fn local_only(input: &NotesInput) -> Notes {
    Notes {
        markdown: annotations_block(input),
        generated: false,
        dropped_sources: 0,
        unsourced_lines: 0,
        title: None,
        speaker_suggestions: Vec::new(),
    }
}

pub(crate) fn notes(input: &NotesInput, response: &Response) -> Notes {
    let segments: HashMap<String, u64> = input
        .transcript
        .iter()
        .map(|segment| (segment_id(segment.id), segment.t0_ms))
        .collect();
    let annotation_count = input.annotation_lines().len();
    let mut dropped_sources = 0;
    let mut unsourced_lines = 0;

    let mut out = annotations_block(input);
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(&format!(
        "## {} · {}\n",
        labels(input.language).generated,
        input.template.name
    ));

    for section in &input.template.sections {
        let wanted = section.title.trim().to_lowercase();
        let Some(found) = response
            .sections
            .iter()
            .find(|s| s.title.trim().to_lowercase() == wanted)
        else {
            continue;
        };
        let mut body = String::new();
        for line in &found.lines {
            let text = one_line(&line.text);
            if text.is_empty() {
                continue;
            }
            let mut pointers: Vec<String> = Vec::new();
            for source in &line.sources {
                let source = source.trim();
                let pointer = if let Some(t0_ms) = segments.get(source) {
                    format!("[[#^{source}|{}]]", timestamp(*t0_ms))
                } else if is_annotation(source, annotation_count) {
                    format!("[[#^{source}|{source}]]")
                } else {
                    dropped_sources += 1;
                    continue;
                };
                if !pointers.contains(&pointer) {
                    pointers.push(pointer);
                }
            }
            if pointers.is_empty() {
                unsourced_lines += 1;
                body.push_str(&format!("- {text} {GENERATED_MARKER}\n"));
            } else {
                body.push_str(&format!(
                    "- {text} {} {GENERATED_MARKER}\n",
                    pointers.join(" ")
                ));
            }
        }
        if !body.is_empty() {
            out.push_str(&format!("\n### {}\n\n{body}", section.title));
        }
    }

    Notes {
        markdown: out,
        generated: true,
        dropped_sources,
        unsourced_lines,
        title: generated_title(input, response),
        speaker_suggestions: speaker_suggestions(input, response),
    }
}

fn is_annotation(source: &str, count: usize) -> bool {
    source
        .strip_prefix('a')
        .filter(|n| n.bytes().all(|b| b.is_ascii_digit()) && !n.starts_with('0'))
        .and_then(|n| n.parse::<usize>().ok())
        .is_some_and(|n| (1..=count).contains(&n))
}

/// A transcrição com uma âncora de bloco por segmento (`^s12`), alvo dos ponteiros das notas.
/// Uma linha por segmento: `- **[01:05] Pessoa 1:** texto ^s12`.
pub fn render_transcript(segments: &[Segment], language: Language) -> String {
    segments
        .iter()
        .map(|segment| {
            format!(
                "- **[{}] {}:** {} ^{}\n",
                timestamp(segment.t0_ms),
                speaker_label(&segment.speaker, language),
                one_line(&segment.text),
                segment_id(segment.id)
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::input;
    use crate::{fixture::segment, Speaker};

    fn render(json: &str) -> Notes {
        notes(&input(), &parse_response(json).unwrap())
    }

    fn line(text: &str, sources: &[&str]) -> String {
        serde_json::json!({"text": text, "sources": sources}).to_string()
    }

    fn one_section(lines: &[String]) -> String {
        format!(
            r#"{{"sections":[{{"title":"Resumo","lines":[{}]}}]}}"#,
            lines.join(",")
        )
    }

    /// As linhas entre `## Anotações` e o próximo `## `.
    fn annotation_block(markdown: &str) -> Vec<&str> {
        markdown
            .lines()
            .skip_while(|l| *l != "## Anotações")
            .skip(1)
            .take_while(|l| !l.starts_with("## "))
            .collect()
    }

    fn generated_block(markdown: &str) -> Vec<&str> {
        markdown
            .lines()
            .skip_while(|l| !l.starts_with("## Notas · "))
            .skip(1)
            .collect()
    }

    #[test]
    fn annotations_block_precedes_generated() {
        let notes = render(&one_section(&[line("Lançamento em 15/11.", &["s12"])]));
        let markdown = &notes.markdown;
        assert!(markdown.starts_with("## Anotações\n"), "{markdown}");
        let annotations = markdown.find("## Anotações").unwrap();
        let first = markdown.find("\ndecidir data ^a1\n").unwrap();
        let second = markdown.find("\nAna: contrato ^a2\n").unwrap();
        let generated = markdown.find("\n## Notas · Reunião geral\n").unwrap();
        assert!(annotations < first && first < second && second < generated);
    }

    #[test]
    fn generated_lines_carry_marker_human_lines_do_not() {
        let notes = render(&format!(
            r#"{{"sections":[{{"title":"Resumo","lines":[{},{}]}},{{"title":"Decisões","lines":[{}]}}]}}"#,
            line("Lançamento em 15/11.", &["s12"]),
            line("Ana cuida do contrato.", &["s13", "a2"]),
            line("Data fechada.", &["a1"])
        ));
        let generated: Vec<&str> = generated_block(&notes.markdown)
            .into_iter()
            .filter(|l| l.starts_with("- "))
            .collect();
        assert_eq!(generated.len(), 3);
        for line in generated {
            assert!(line.ends_with(" <!-- fala:ia -->"), "{line}");
        }
        let human = annotation_block(&notes.markdown);
        assert!(human.iter().any(|l| l.starts_with("decidir data")));
        for line in human {
            assert!(!line.contains(GENERATED_MARKER), "{line}");
        }
    }

    #[test]
    fn pointers_render_with_timestamp_and_order() {
        let notes = render(&one_section(&[line(
            "Ana cuida do contrato.",
            &["s12", "a2"],
        )]));
        let line = notes
            .markdown
            .lines()
            .find(|l| l.starts_with("- Ana cuida"))
            .unwrap();
        assert_eq!(
            line,
            "- Ana cuida do contrato. [[#^s12|01:05]] [[#^a2|a2]] <!-- fala:ia -->"
        );
        assert_eq!(timestamp(3_723_000), "1:02:03");
        assert_eq!(timestamp(65_000), "01:05");
        assert_eq!(notes.dropped_sources, 0);
    }

    #[test]
    fn unknown_ids_are_dropped_and_counted() {
        let notes = render(&one_section(&[line(
            "Lançamento em 15/11.",
            &["s999", "s12", "a9", "x1"],
        )]));
        assert_eq!(notes.dropped_sources, 3);
        assert_eq!(notes.unsourced_lines, 0);
        let line = notes
            .markdown
            .lines()
            .find(|l| l.starts_with("- Lançamento"))
            .unwrap();
        assert_eq!(
            line,
            "- Lançamento em 15/11. [[#^s12|01:05]] <!-- fala:ia -->"
        );
        for gone in ["s999", "a9", "x1"] {
            assert!(!notes.markdown.contains(gone), "{gone}");
        }
    }

    #[test]
    fn unsourced_lines_are_kept_and_counted() {
        let notes = render(&one_section(&[line("Clima bom na reunião.", &["s999"])]));
        assert_eq!(notes.unsourced_lines, 1);
        assert_eq!(notes.dropped_sources, 1);
        let line = notes
            .markdown
            .lines()
            .find(|l| l.starts_with("- Clima"))
            .unwrap();
        assert_eq!(line, "- Clima bom na reunião. <!-- fala:ia -->");
        assert!(!line.contains("[[#^"));
    }

    #[test]
    fn sections_follow_template_order() {
        let notes = render(&format!(
            r#"{{"sections":[{{"title":"Próximos passos","lines":[{}]}},{{"title":"Extra","lines":[{}]}},{{"title":"Resumo","lines":[{}]}},{{"title":"Decisões","lines":[]}}]}}"#,
            line("Ana: contrato (sexta)", &["s13"]),
            line("fora do template", &["s3"]),
            line("Lançamento em 15/11.", &["s12"])
        ));
        let markdown = &notes.markdown;
        let resumo = markdown.find("### Resumo").unwrap();
        let passos = markdown.find("### Próximos passos").unwrap();
        assert!(resumo < passos, "{markdown}");
        assert!(!markdown.contains("### Extra"));
        assert!(!markdown.contains("fora do template"));
        assert!(!markdown.contains("### Decisões"));
    }

    #[test]
    fn generated_text_is_one_line_without_marker() {
        let notes = render(&one_section(&[line(
            "linha um\nlinha dois <!-- fala:ia -->",
            &["s12"],
        )]));
        let generated: Vec<&str> = generated_block(&notes.markdown)
            .into_iter()
            .filter(|l| l.starts_with("- "))
            .collect();
        assert_eq!(
            generated,
            ["- linha um linha dois [[#^s12|01:05]] <!-- fala:ia -->"]
        );
        assert_eq!(notes.markdown.matches(GENERATED_MARKER).count(), 1);
    }

    #[test]
    fn transcript_anchors_match_pointers() {
        let input = input();
        let transcript = render_transcript(&input.transcript, input.language);
        assert!(
            transcript
                .lines()
                .any(|l| l == "- **[01:05] Pessoa 1:** Proponho 15 de novembro. ^s12"),
            "{transcript}"
        );
        assert_eq!(
            render_transcript(
                &[segment(12, Speaker::Person(1), 65_000, "oi")],
                Language::PtBr
            ),
            "- **[01:05] Pessoa 1:** oi ^s12\n"
        );
        let notes = render(&one_section(&[
            line("Lançamento em 15/11.", &["s12", "s3"]),
            line("Contrato com a Ana.", &["s13"]),
        ]));
        let mut cited = 0;
        for piece in notes.markdown.split("[[#^s").skip(1) {
            let id: String = piece.chars().take_while(char::is_ascii_digit).collect();
            assert!(
                transcript.contains(&format!(" ^s{id}\n")),
                "s{id} sem âncora"
            );
            cited += 1;
        }
        assert_eq!(cited, 3);
    }

    #[test]
    fn english_headings_follow_door_5() {
        let mut input = input();
        input.language = Language::En;
        let notes = notes(
            &input,
            &parse_response(&one_section(&[line("Launch on Nov 15.", &["s12"])])).unwrap(),
        );
        let headings: Vec<&str> = notes
            .markdown
            .lines()
            .filter(|l| l.starts_with("## "))
            .collect();
        assert_eq!(headings, ["## Notes", "## AI notes · Reunião geral"]);
        assert_eq!(local_only(&input).markdown.lines().next(), Some("## Notes"));
    }

    #[test]
    fn marker_typed_in_annotations_is_removed() {
        let mut input = input();
        input.annotations =
            format!("decidir data {GENERATED_MARKER}\n{GENERATED_MARKER}\nAna: contrato");
        let notes = notes(
            &input,
            &parse_response(&one_section(&[line(
                "Lançamento em 15/11.",
                &["s12", "a2"],
            )]))
            .unwrap(),
        );
        let human = annotation_block(&notes.markdown);
        assert_eq!(
            human.iter().filter(|l| !l.is_empty()).collect::<Vec<_>>(),
            [&"decidir data ^a1", &"Ana: contrato ^a2"]
        );
        for line in &human {
            assert!(!line.contains(GENERATED_MARKER), "{line}");
        }
        assert!(notes.markdown.contains("[[#^a2|a2]]"));
        assert_eq!(notes.dropped_sources, 0);
        let payload: serde_json::Value = serde_json::from_str(
            &crate::NotesPayload::build(&input)
                .unwrap()
                .to_json()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            payload["annotations"],
            serde_json::json!([{"id":"a1","text":"decidir data"},{"id":"a2","text":"Ana: contrato"}])
        );
    }

    #[test]
    fn local_only_renders_only_annotations() {
        let notes = local_only(&input());
        assert!(!notes.generated);
        assert_eq!(
            notes.markdown,
            "## Anotações\n\ndecidir data ^a1\n\nAna: contrato ^a2\n"
        );
    }

    fn with_extras(title: &str, suggestions: serde_json::Value) -> String {
        serde_json::json!({
            "title": title,
            "sections": [{"title": "Resumo", "lines": []}],
            "speaker_suggestions": suggestions,
        })
        .to_string()
    }

    #[test]
    fn generated_title_never_replaces_the_users() {
        let response = parse_response(&with_extras(
            "Data do lançamento
de novembro",
            serde_json::json!([]),
        ))
        .unwrap();
        let mut input = input();
        assert_eq!(notes(&input, &response).title, None);

        input.title = "  ".to_string();
        assert_eq!(
            notes(&input, &response).title.as_deref(),
            Some("Data do lançamento de novembro")
        );

        let long = parse_response(&with_extras(&"x".repeat(200), serde_json::json!([]))).unwrap();
        assert_eq!(notes(&input, &long).title.unwrap().chars().count(), 80);
        let blank = parse_response(&with_extras(
            " 
 ",
            serde_json::json!([]),
        ))
        .unwrap();
        assert_eq!(notes(&input, &blank).title, None);
        let missing = parse_response(&one_section(&[])).unwrap();
        assert_eq!(notes(&input, &missing).title, None);
        assert_eq!(local_only(&input).title, None);
    }

    #[test]
    fn invalid_speaker_suggestions_are_dropped() {
        let mut input = input();
        input.transcript.push(segment(
            20,
            Speaker::Person(2),
            90_000,
            "Fecho com o jurídico.",
        ));
        input.participants.push("carla@empresa.com".to_string());
        input.participants.push("Dani".to_string());
        let response = parse_response(&with_extras(
            "",
            serde_json::json!([
                {"person": 1, "name": "bruno lima", "reason": "Propõe a data,
            como o Bruno na pauta."},
                {"person": 1, "name": "Dani", "reason": "segunda para a mesma pessoa"},
                {"person": -1, "name": "Dani", "reason": "número negativo"},
                {"person": 4_294_967_297_i64, "name": "Dani", "reason": "fora de u32"},
                {"person": 7, "name": "Bruno Lima", "reason": "Pessoa 7 não existe"},
                {"person": 0, "name": "Bruno Lima", "reason": "Pessoa 0 não existe"},
                {"person": 2, "name": "Ana", "reason": "Ana já é rótulo"},
                {"person": 2, "name": "Carla", "reason": "fora dos participantes"},
                {"person": 2, "name": "carla@empresa.com", "reason": "e-mail"},
                {"person": 2, "name": "Bruno Lima", "reason": "nome já sugerido"},
            ]),
        ))
        .unwrap();
        assert_eq!(
            notes(&input, &response).speaker_suggestions,
            [SpeakerSuggestion {
                person: 1,
                name: "Bruno Lima".to_string(),
                reason: "Propõe a data, como o Bruno na pauta.".to_string(),
            }]
        );

        input.participants.clear();
        assert!(notes(&input, &response).speaker_suggestions.is_empty());
        assert!(local_only(&input).speaker_suggestions.is_empty());
    }

    #[test]
    fn rejects_text_that_is_not_the_sections_json() {
        assert!(parse_response("não é json").is_err());
        assert!(parse_response(r#"{"secoes":[]}"#).is_err());
        assert!(parse_response("```json\n{\"sections\":[]}\n```").is_ok());
    }
}
