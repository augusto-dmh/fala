//! Chaves de API no keyring do SO (ADR-0008).
//!
//! `ApiKey` nunca é formatada nem serializada; `SecretStore` lê, grava e apaga por id de
//! provedor. `KeyringStore` usa o cofre do SO (Credential Manager no Windows, Secret Service no
//! Linux), escolhido pelo crate `keyring` sem `cfg` aqui (ADR-0007). `MemoryStore` serve a testes.

use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;

/// Service de toda entrada do Fala no keyring: o `identifier` do `tauri.conf.json`.
pub const SERVICE: &str = "br.com.augusto.fala";

/// Uma chave de API. Sem `Display` nem `Serialize`; o `Debug` não mostra o valor.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiKey(String);

impl ApiKey {
    /// Recusa chave vazia ou só com espaços.
    pub fn new(value: impl Into<String>) -> Result<Self, SecretError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(SecretError::EmptyKey);
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

/// Erros de `fala-secrets`. Nenhuma variante carrega a chave nem o id recebido (um id inválido
/// pode ser uma chave colada no lugar errado).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SecretError {
    #[error("id de provedor inválido: use 1 a 32 caracteres entre a-z, 0-9 e _")]
    InvalidProvider,
    #[error("chave vazia")]
    EmptyKey,
    #[error("keyring indisponível")]
    Unavailable,
    #[error("o keyring recusou a operação")]
    Store,
}

/// Onde as chaves vivem, por id de provedor (`gemini`, `openai`, ...).
pub trait SecretStore: Send + Sync {
    fn get(&self, provider: &str) -> Result<Option<ApiKey>, SecretError>;
    fn set(&self, provider: &str, key: &ApiKey) -> Result<(), SecretError>;
    fn delete(&self, provider: &str) -> Result<(), SecretError>;
}

/// `^[a-z0-9_]{1,32}$`, o formato dos ids de provedor do desktop.
pub fn validate_provider(provider: &str) -> Result<(), SecretError> {
    let valid = (1..=32).contains(&provider.len())
        && provider
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    if valid {
        Ok(())
    } else {
        Err(SecretError::InvalidProvider)
    }
}

/// O keyring do SO: service [`SERVICE`], account = id do provedor.
#[derive(Debug, Default, Clone, Copy)]
pub struct KeyringStore;

impl KeyringStore {
    fn entry(provider: &str) -> Result<keyring::Entry, SecretError> {
        validate_provider(provider)?;
        keyring::Entry::new(SERVICE, provider).map_err(map_keyring_error)
    }
}

impl SecretStore for KeyringStore {
    fn get(&self, provider: &str) -> Result<Option<ApiKey>, SecretError> {
        get_result(Self::entry(provider)?.get_password())
    }

    fn set(&self, provider: &str, key: &ApiKey) -> Result<(), SecretError> {
        Self::entry(provider)?
            .set_password(key.expose())
            .map_err(map_keyring_error)
    }

    fn delete(&self, provider: &str) -> Result<(), SecretError> {
        delete_result(Self::entry(provider)?.delete_credential())
    }
}

fn get_result(result: keyring::Result<String>) -> Result<Option<ApiKey>, SecretError> {
    match result {
        Ok(value) => Ok(ApiKey::new(value).ok()),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(map_keyring_error(e)),
    }
}

fn delete_result(result: keyring::Result<()>) -> Result<(), SecretError> {
    match result {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(map_keyring_error(e)),
    }
}

/// Descarta o conteúdo do erro: `BadEncoding` e `BadDataFormat` carregam os bytes do segredo.
fn map_keyring_error(error: keyring::Error) -> SecretError {
    match error {
        keyring::Error::NoStorageAccess(_)
        | keyring::Error::PlatformFailure(_)
        | keyring::Error::NoDefaultStore => SecretError::Unavailable,
        _ => SecretError::Store,
    }
}

/// Store em memória, para testes de quem consome `SecretStore`.
#[derive(Debug, Default)]
pub struct MemoryStore {
    keys: Mutex<HashMap<String, ApiKey>>,
}

impl MemoryStore {
    pub fn is_empty(&self) -> bool {
        self.keys.lock().map(|keys| keys.is_empty()).unwrap_or(true)
    }
}

impl SecretStore for MemoryStore {
    fn get(&self, provider: &str) -> Result<Option<ApiKey>, SecretError> {
        validate_provider(provider)?;
        let keys = self.keys.lock().map_err(|_| SecretError::Unavailable)?;
        Ok(keys.get(provider).cloned())
    }

    fn set(&self, provider: &str, key: &ApiKey) -> Result<(), SecretError> {
        validate_provider(provider)?;
        let mut keys = self.keys.lock().map_err(|_| SecretError::Unavailable)?;
        keys.insert(provider.to_string(), key.clone());
        Ok(())
    }

    fn delete(&self, provider: &str) -> Result<(), SecretError> {
        validate_provider(provider)?;
        let mut keys = self.keys.lock().map_err(|_| SecretError::Unavailable)?;
        keys.remove(provider);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(value: &str) -> ApiKey {
        ApiKey::new(value).unwrap()
    }

    #[test]
    fn memory_store_round_trips() {
        let store = MemoryStore::default();
        store.set("gemini", &key("primeira")).unwrap();
        assert_eq!(store.get("gemini").unwrap(), Some(key("primeira")));
        store.set("gemini", &key("segunda")).unwrap();
        assert_eq!(store.get("gemini").unwrap(), Some(key("segunda")));
    }

    #[test]
    fn delete_then_get_is_none() {
        let store = MemoryStore::default();
        store.set("gemini", &key("valor")).unwrap();
        assert_eq!(store.delete("gemini"), Ok(()));
        assert_eq!(store.get("gemini").unwrap(), None);
        assert_eq!(store.delete("gemini"), Ok(()));
        assert_eq!(store.delete("nunca_gravado"), Ok(()));
    }

    #[test]
    fn rejects_invalid_provider_ids() {
        let store = MemoryStore::default();
        let long = "a".repeat(33);
        for id in ["", "Gemini", "a-b", "a b", "ç", long.as_str()] {
            assert_eq!(store.get(id), Err(SecretError::InvalidProvider), "{id:?}");
            assert_eq!(
                store.set(id, &key("valor")),
                Err(SecretError::InvalidProvider),
                "{id:?}"
            );
            assert_eq!(
                store.delete(id),
                Err(SecretError::InvalidProvider),
                "{id:?}"
            );
        }
        assert!(store.is_empty());
        let max = "a".repeat(32);
        for id in ["gemini", "bedrock_mantle", max.as_str()] {
            assert_eq!(validate_provider(id), Ok(()), "{id:?}");
            assert_eq!(store.set(id, &key("valor")), Ok(()), "{id:?}");
        }
    }

    #[test]
    fn api_key_rejects_empty() {
        assert_eq!(ApiKey::new(""), Err(SecretError::EmptyKey));
        assert_eq!(ApiKey::new("  \t"), Err(SecretError::EmptyKey));
    }

    #[test]
    fn maps_keyring_errors() {
        let platform = || -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(std::io::Error::other("falha"))
        };
        assert_eq!(
            map_keyring_error(keyring::Error::NoStorageAccess(platform())),
            SecretError::Unavailable
        );
        assert_eq!(
            map_keyring_error(keyring::Error::PlatformFailure(platform())),
            SecretError::Unavailable
        );
        assert_eq!(
            map_keyring_error(keyring::Error::NoDefaultStore),
            SecretError::Unavailable
        );
        assert_eq!(get_result(Err(keyring::Error::NoEntry)), Ok(None));
        assert_eq!(delete_result(Err(keyring::Error::NoEntry)), Ok(()));
        assert_eq!(
            get_result(Err(keyring::Error::NoDefaultStore)),
            Err(SecretError::Unavailable)
        );
    }

    #[test]
    fn maps_other_keyring_errors_to_store() {
        let platform = || -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(std::io::Error::other("falha"))
        };
        let others = [
            keyring::Error::BadEncoding(b"segredo".to_vec()),
            keyring::Error::BadDataFormat(b"segredo".to_vec(), platform()),
            keyring::Error::BadStoreFormat("formato".to_string()),
            keyring::Error::TooLong("user".to_string(), 32),
            keyring::Error::Invalid("service".to_string(), "vazio".to_string()),
            keyring::Error::Ambiguous(Vec::new()),
            keyring::Error::NotSupportedByStore("busca".to_string()),
        ];
        for error in others {
            let shown = format!("{error:?}");
            assert_eq!(map_keyring_error(error), SecretError::Store, "{shown}");
        }
        assert_eq!(
            get_result(Err(keyring::Error::BadEncoding(b"segredo".to_vec()))),
            Err(SecretError::Store)
        );
        assert_eq!(
            delete_result(Err(keyring::Error::TooLong("user".to_string(), 32))),
            Err(SecretError::Store)
        );
    }

    static_assertions::assert_not_impl_any!(ApiKey: fmt::Display, serde::Serialize);

    #[test]
    fn api_key_debug_is_redacted() {
        assert_eq!(format!("{:?}", key("sk-teste")), "ApiKey([REDACTED])");
        assert!(!format!("{:?}", Some(key("sk-teste"))).contains("sk-teste"));
    }

    const TEST_PROVIDER: &str = "teste_fala_secrets";

    #[test]
    #[ignore = "usa o keyring real do SO; rode com --ignored numa sessão com Secret Service"]
    fn keyring_store_round_trips() {
        let store = KeyringStore;
        store.delete(TEST_PROVIDER).unwrap();
        assert_eq!(store.get(TEST_PROVIDER).unwrap(), None);
        store.set(TEST_PROVIDER, &key("nao-e-chave-1")).unwrap();
        assert_eq!(
            store.get(TEST_PROVIDER).unwrap(),
            Some(key("nao-e-chave-1"))
        );
        store.set(TEST_PROVIDER, &key("nao-e-chave-2")).unwrap();
        assert_eq!(
            store.get(TEST_PROVIDER).unwrap(),
            Some(key("nao-e-chave-2"))
        );
        assert_eq!(store.delete(TEST_PROVIDER), Ok(()));
        assert_eq!(store.get(TEST_PROVIDER).unwrap(), None);
        assert_eq!(store.delete(TEST_PROVIDER), Ok(()));
    }

    #[test]
    #[ignore = "usa o keyring real do SO; rode com --ignored numa sessão com Secret Service"]
    fn keyring_entry_uses_fala_service() {
        let entry = KeyringStore::entry("gemini").unwrap();
        assert_eq!(
            entry.inner.get_specifiers(),
            Some(("br.com.augusto.fala".to_string(), "gemini".to_string()))
        );
    }
}
