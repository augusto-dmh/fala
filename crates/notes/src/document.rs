//! O "Markdown da nota" que a UI copia (door 6 do `meeting-panel`): título, data e hora, as notas
//! geradas (ou, sem elas, as anotações) e a transcrição com as âncoras que os ponteiros citam.

use fala_core::Language;

use crate::render::{annotations_block, render_transcript};
use crate::NotesInput;

/// Monta o documento de uma sessão. `notes_md` é o Markdown de uma geração anterior
/// ([`crate::Notes::markdown`]), que já traz o bloco de anotações; sem ele, o documento leva só
/// as anotações. Seções vazias ficam de fora.
pub fn note_document(input: &NotesInput, notes_md: Option<&str>) -> String {
    let (untitled, transcript) = match input.language {
        Language::PtBr => ("Reunião", "Transcrição"),
        Language::En => ("Meeting", "Transcript"),
    };
    let title = match input.title.trim() {
        "" => untitled,
        title => title,
    };
    let mut parts = vec![
        format!("# {title}\n"),
        format!("{} {}\n", input.date, input.start_time),
    ];
    let body = match notes_md.map(str::trim).filter(|md| !md.is_empty()) {
        Some(md) => format!("{md}\n"),
        None => annotations_block(input),
    };
    if !body.is_empty() {
        parts.push(body);
    }
    if !input.transcript.is_empty() {
        parts.push(format!(
            "## {transcript}\n\n{}",
            render_transcript(&input.transcript, input.language)
        ));
    }
    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::input;

    #[test]
    fn document_with_generated_notes() {
        let mut input = input();
        input.title = "Planejamento".to_string();
        let notes = "## Anotações\n\ndecidir data ^a1\n\n## Notas · Reunião geral\n\n### Resumo\n\n- Lançamento em 15/11. [[#^s12|01:05]] <!-- fala:ia -->\n";
        let doc = note_document(&input, Some(notes));
        assert!(
            doc.starts_with("# Planejamento\n\n2026-10-02 14:02\n\n## Anotações\n"),
            "{doc}"
        );
        assert!(doc.contains(notes), "{doc}");
        let transcript = doc.find("\n## Transcrição\n\n").unwrap();
        assert!(doc.find("### Resumo").unwrap() < transcript);
        assert!(
            doc.contains("- **[01:05] Pessoa 1:** Proponho 15 de novembro. ^s12\n"),
            "{doc}"
        );
        assert!(doc.ends_with("^s13\n"), "{doc}");
    }

    #[test]
    fn document_without_notes_or_segments() {
        let mut input = input();
        input.title = "  ".to_string();
        let doc = note_document(&input, None);
        assert!(
            doc.starts_with("# Reunião\n\n2026-10-02 14:02\n\n## Anotações\n\ndecidir data ^a1\n"),
            "{doc}"
        );
        assert!(doc.contains("\n## Transcrição\n"), "{doc}");

        input.transcript.clear();
        let doc = note_document(&input, None);
        assert!(!doc.contains("Transcrição"), "{doc}");
        assert!(doc.ends_with("Ana: contrato ^a2\n"), "{doc}");

        input.annotations = String::new();
        input.language = Language::En;
        assert_eq!(
            note_document(&input, Some("  ")),
            "# Meeting\n\n2026-10-02 14:02\n"
        );

        input.transcript = crate::fixture::input().transcript;
        let doc = note_document(&input, None);
        assert!(
            doc.contains("\n## Transcript\n\n- **[00:00] Me:** "),
            "{doc}"
        );
    }
}
