//! Notas de reunião geradas por LLM (fase 2, F6).
//!
//! Recebe a transcrição com id por segmento, as anotações do usuário, título, data, idioma,
//! dicionário e um template, e devolve Markdown em que as anotações são o bloco humano e cada
//! linha gerada leva um marcador e um ponteiro por fonte (segmento ou linha de anotação).
//! O que sai para o LLM é só o `NotesPayload`, a lista enumerada da ADR-0016 (proposta), e
//! nada sai numa sessão "só local".

mod input;
mod payload;
mod render;
mod template;

pub use input::{Channel, NotesInput, Segment, Speaker};
pub use payload::NotesPayload;
pub use render::{render_transcript, Notes, GENERATED_MARKER};
pub use template::{builtin_templates, Section, Template, TEMPLATE_SCHEMA};

/// Erros de `fala-notes`. Nenhuma variante carrega a transcrição, as anotações, o corpo de uma
/// resposta do provedor nem a chave.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NotesError {
    #[error("entrada inválida: {0}")]
    InvalidInput(String),
    #[error("sessão vazia: sem transcrição e sem anotações")]
    EmptySession,
    #[error("template inválido: {0}")]
    InvalidTemplate(String),
    #[error("sem chave da API de notas no keyring")]
    MissingKey,
    #[error("o cofre de chaves falhou")]
    KeyStore,
    #[error("o provedor de notas respondeu HTTP {0}")]
    Http(u16),
    #[error("o provedor de notas não respondeu a tempo")]
    Timeout,
    #[error("sem conexão com o provedor de notas")]
    Network,
    #[error("o provedor recusou gerar as notas")]
    Refused,
    #[error("a resposta do provedor foi cortada no limite de tokens")]
    Truncated,
    #[error("resposta do provedor ilegível")]
    InvalidResponse,
}

/// O LLM de notas. Recebe só o `NotesPayload` e devolve o texto da resposta, que deve ser o JSON
/// de seções pedido no prompt.
pub trait NotesLlm {
    fn generate(&self, payload: &NotesPayload) -> Result<String, NotesError>;
}

/// Gera as notas de uma sessão. Valida a entrada; numa sessão "só local" devolve só as
/// anotações sem tocar no LLM; senão monta o payload, chama o LLM e renderiza a resposta.
pub fn generate_notes(input: &NotesInput, llm: &dyn NotesLlm) -> Result<Notes, NotesError> {
    input.validate()?;
    if input.local_only {
        log::info!("sessão só local: notas sem LLM");
        return Ok(render::local_only(input));
    }
    let payload = NotesPayload::build(input)?;
    let raw = llm.generate(&payload)?;
    let response = render::parse_response(&raw)?;
    let notes = render::notes(input, &response);
    log::info!(
        "notas geradas: {} fontes descartadas, {} linhas sem fonte",
        notes.dropped_sources,
        notes.unsourced_lines
    );
    Ok(notes)
}

#[cfg(test)]
pub(crate) mod fixture {
    use fala_core::{Dictionary, Language};

    use crate::{builtin_templates, Channel, NotesInput, Segment, Speaker};

    pub fn segment(id: u32, speaker: Speaker, t0_ms: u64, text: &str) -> Segment {
        Segment {
            id,
            channel: if speaker == Speaker::Me {
                Channel::Mic
            } else {
                Channel::System
            },
            speaker,
            t0_ms,
            t1_ms: t0_ms + 4_000,
            text: text.to_string(),
        }
    }

    pub fn input() -> NotesInput {
        let template = builtin_templates()
            .unwrap()
            .into_iter()
            .find(|t| t.id == "geral")
            .unwrap();
        NotesInput {
            title: "Planejamento do lançamento".to_string(),
            date: "2026-10-02".to_string(),
            start_time: "14:02".to_string(),
            language: Language::PtBr,
            dictionary: Dictionary::new(["Fala", "Parakeet"]),
            template,
            annotations: "decidir data\n\n  \nAna: contrato".to_string(),
            transcript: vec![
                segment(3, Speaker::Me, 0, "Vamos falar do lançamento."),
                segment(12, Speaker::Person(1), 65_000, "Proponho 15 de novembro."),
                segment(
                    13,
                    Speaker::Named("Ana".to_string()),
                    70_000,
                    "Eu cuido do contrato.",
                ),
            ],
            local_only: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::fixture::{input, segment};

    /// Um LLM que entra em pânico se for chamado.
    struct NeverCalled;

    impl NotesLlm for NeverCalled {
        fn generate(&self, _: &NotesPayload) -> Result<String, NotesError> {
            panic!("o LLM não devia ser chamado");
        }
    }

    /// Grava o JSON recebido e devolve uma resposta fixa.
    struct Recording(RefCell<Vec<String>>);

    impl NotesLlm for Recording {
        fn generate(&self, payload: &NotesPayload) -> Result<String, NotesError> {
            self.0.borrow_mut().push(payload.to_json()?);
            Ok(r#"{"sections":[{"title":"Resumo","lines":[{"text":"Lançamento em 15/11.","sources":["s12"]}]}]}"#.to_string())
        }
    }

    #[test]
    fn duplicate_segment_ids_are_rejected() {
        let mut input = input();
        input
            .transcript
            .push(segment(12, Speaker::Me, 80_000, "repetido"));
        assert!(matches!(
            generate_notes(&input, &NeverCalled),
            Err(NotesError::InvalidInput(_))
        ));
    }

    #[test]
    fn empty_session_never_calls_llm() {
        let mut input = input();
        input.transcript.clear();
        input.annotations = "  \n\n \t".to_string();
        assert_eq!(
            generate_notes(&input, &NeverCalled),
            Err(NotesError::EmptySession)
        );
    }

    #[test]
    fn llm_receives_the_built_payload() {
        let input = input();
        let llm = Recording(RefCell::new(Vec::new()));
        let notes = generate_notes(&input, &llm).unwrap();
        assert!(notes.generated);
        let expected = NotesPayload::build(&input).unwrap().to_json().unwrap();
        assert_eq!(*llm.0.borrow(), vec![expected]);
    }
}
