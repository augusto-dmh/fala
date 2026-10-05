//! A chave da API de notas, vinda de um provedor injetado (ADR-0008: a chave vive no keyring).
//! Este crate não depende de `fala-secrets`; a ligação implementa [`KeySource`] sobre ele.

use std::fmt;

use crate::NotesError;

/// Uma chave de API. Sem `Display` nem `Serialize`; o `Debug` não mostra o valor.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiKey(String);

impl ApiKey {
    /// Recusa chave vazia ou só com espaços.
    pub fn new(value: impl Into<String>) -> Result<Self, NotesError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(NotesError::MissingKey);
        }
        Ok(Self(value))
    }

    /// O valor, para o header da requisição. Nunca para log.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey([REDACTED])")
    }
}

/// De onde vem a chave, por id de provedor (`"anthropic"`). Consultado só no momento da
/// requisição, nunca numa sessão "só local".
pub trait KeySource: Send + Sync {
    fn api_key(&self, provider: &str) -> Result<Option<ApiKey>, NotesError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_key_debug_is_redacted() {
        let key = ApiKey::new("sk-ant-segredo-123").unwrap();
        let debug = format!("{key:?}");
        assert!(!debug.contains("segredo"), "{debug}");
        assert_eq!(debug, "ApiKey([REDACTED])");
        assert_eq!(key.expose(), "sk-ant-segredo-123");
        assert_eq!(ApiKey::new("  "), Err(NotesError::MissingKey));
    }
}
