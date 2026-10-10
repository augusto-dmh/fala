//! Cliente do `generateContent` do Gemini (ADR-0004). Manda só o texto das regras, o nome do
//! app e o dicionário; a chave vai só no header `x-goog-api-key`. O texto do prompt mora em
//! `prompt.rs`.

use std::time::Duration;

use fala_secrets::ApiKey;
use serde_json::{json, Value};

use crate::prompt::{system_text, user_text};
use crate::{Fallback, FormatContext, Formatter, PostprocError, INSERT_DEADLINE};

/// Modelo padrão (ADR-0004).
pub const DEFAULT_MODEL: &str = "gemini-2.5-flash-lite";
/// Endpoint público da API do Gemini.
pub const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com";

/// O cliente do Gemini. Barato de clonar.
#[derive(Debug, Clone)]
pub struct Gemini {
    key: ApiKey,
    model: String,
    base_url: String,
}

impl Gemini {
    pub fn new(key: ApiKey) -> Self {
        Self {
            key,
            model: DEFAULT_MODEL.to_string(),
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Troca o endpoint; os testes apontam para um servidor falso local.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    fn url(&self) -> String {
        format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url, self.model
        )
    }

    /// Uma request, com `timeout` do início ao fim do corpo. O erro nunca carrega o corpo da
    /// resposta, que pode ecoar o ditado ou a chave.
    pub fn call(
        &self,
        text: &str,
        ctx: &FormatContext<'_>,
        timeout: Duration,
    ) -> Result<String, Fallback> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .http_status_as_error(false)
            .build()
            .into();
        let body = request_body(text, ctx).to_string();
        let mut response = agent
            .post(&self.url())
            .header("x-goog-api-key", self.key.expose())
            .header("content-type", "application/json")
            .send(body.as_str())
            .map_err(map_error)?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(Fallback::Http(status));
        }
        let raw = response.body_mut().read_to_string().map_err(map_error)?;
        response_text(&raw).ok_or(Fallback::InvalidResponse)
    }
}

impl Formatter for Gemini {
    fn format(&self, text: &str, ctx: &FormatContext<'_>) -> Result<String, PostprocError> {
        self.call(text, ctx, INSERT_DEADLINE)
            .map_err(PostprocError::Llm)
    }
}

/// O corpo inteiro: `systemInstruction`, `contents` e `generationConfig`, nada mais.
pub(crate) fn request_body(text: &str, ctx: &FormatContext<'_>) -> Value {
    let system = system_text(ctx.cleanup, ctx.dictionary);
    let user = user_text(text, ctx.app);
    json!({
        "systemInstruction": { "parts": [{ "text": system }] },
        "contents": [{ "role": "user", "parts": [{ "text": user }] }],
        "generationConfig": { "temperature": 0 },
    })
}

/// `candidates[0].content.parts[].text` concatenado e aparado; `None` se não houver texto.
fn response_text(raw: &str) -> Option<String> {
    let value: Value = serde_json::from_str(raw).ok()?;
    let parts = value
        .get("candidates")?
        .get(0)?
        .get("content")?
        .get("parts")?
        .as_array()?;
    let text: String = parts
        .iter()
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect();
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn map_error(error: ureq::Error) -> Fallback {
    match error {
        ureq::Error::Timeout(_) => Fallback::Timeout,
        ureq::Error::StatusCode(status) => Fallback::Http(status),
        ureq::Error::Io(e) if e.kind() == std::io::ErrorKind::TimedOut => Fallback::Timeout,
        _ => Fallback::Network,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_text_from_first_candidate() {
        let raw =
            r#"{"candidates":[{"content":{"parts":[{"text":" Oi, "},{"text":"tudo bem?\n"}]}}]}"#;
        assert_eq!(response_text(raw).as_deref(), Some("Oi, tudo bem?"));
        assert_eq!(response_text(r#"{"candidates":[]}"#), None);
        assert_eq!(
            response_text(r#"{"candidates":[{"content":{"parts":[{"text":"  "}]}}]}"#),
            None
        );
        assert_eq!(response_text("não é json"), None);
    }
}
