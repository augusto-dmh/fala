use serde::{Deserialize, Serialize};

use crate::Language;

/// Texto bruto que o ASR devolve, com o idioma em que foi transcrito.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transcript {
    pub text: String,
    pub language: Language,
}

/// O app ativo quando o ditado aconteceu, se conhecido. Só o nome vai ao LLM (ADR-0004).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AppContext {
    pub app_name: Option<String>,
}

/// Quem produziu o texto final de um ditado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Editor {
    /// Nada foi aplicado: o final é o bruto.
    None,
    /// Regras locais de pós-processamento.
    Rules,
    /// O LLM editou o texto; é o caso que "desfazer edição da IA" reverte.
    Llm,
}

/// Um ditado completo: o bruto do ASR, o texto final, quem o produziu e o app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dictation {
    pub raw: Transcript,
    pub final_text: String,
    pub editor: Editor,
    pub app: AppContext,
}

impl Dictation {
    /// Um ditado sem pós-processamento: o final é o bruto.
    pub fn unedited(raw: Transcript, app: AppContext) -> Self {
        Self {
            final_text: raw.text.clone(),
            raw,
            editor: Editor::None,
            app,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_serialized_values() {
        for (editor, json) in [
            (Editor::None, "\"none\""),
            (Editor::Rules, "\"rules\""),
            (Editor::Llm, "\"llm\""),
        ] {
            assert_eq!(serde_json::to_string(&editor).unwrap(), json);
            assert_eq!(serde_json::from_str::<Editor>(json).unwrap(), editor);
        }
    }

    #[test]
    fn dictation_round_trips_json() {
        let dictation = Dictation {
            raw: Transcript {
                text: "oi tudo bem".to_string(),
                language: Language::PtBr,
            },
            final_text: "Oi, tudo bem?".to_string(),
            editor: Editor::Llm,
            app: AppContext {
                app_name: Some("Slack".to_string()),
            },
        };
        let json = serde_json::to_string(&dictation).unwrap();
        assert_eq!(serde_json::from_str::<Dictation>(&json).unwrap(), dictation);
    }

    #[test]
    fn unedited_keeps_raw_text() {
        let transcript = Transcript {
            text: "oi tudo bem".to_string(),
            language: Language::PtBr,
        };
        let dictation = Dictation::unedited(transcript.clone(), AppContext::default());
        assert_eq!(dictation.final_text, transcript.text);
        assert_eq!(dictation.editor, Editor::None);
    }
}
