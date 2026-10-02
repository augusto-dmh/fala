use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::SessionError;

/// Alfabeto Crockford base32 do ULID: sem `I`, `L`, `O` e `U`.
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const TEXT_LEN: usize = 26;
const TIME_BITS: u32 = 48;
const ENTROPY_BITS: u32 = 80;

/// Identidade de uma sessão de reunião: um ULID (48 bits de `unix_ms` + 80 bits aleatórios).
///
/// O texto tem 26 caracteres Crockford maiúsculos e ordena por tempo. Vai para a pasta
/// `audio/<id>/`, o SQLite e o frontmatter, então a forma é porta de mão única.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct SessionId(u128);

impl SessionId {
    /// Monta o id a partir do instante e da entropia. Bits além de 48 (tempo) e 80 (entropia)
    /// são descartados.
    pub fn from_parts(unix_ms: u64, entropy: u128) -> Self {
        let time = u128::from(unix_ms) & ((1 << TIME_BITS) - 1);
        let random = entropy & ((1 << ENTROPY_BITS) - 1);
        Self((time << ENTROPY_BITS) | random)
    }

    /// Gera um id novo para o instante dado, com 80 bits de entropia do SO.
    pub fn generate(unix_ms: u64) -> Result<Self, SessionError> {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes[6..]).map_err(|e| SessionError::Entropy(e.to_string()))?;
        Ok(Self::from_parts(unix_ms, u128::from_be_bytes(bytes)))
    }

    /// Os milissegundos Unix gravados no id.
    pub fn unix_ms(&self) -> u64 {
        // Cabe em 48 bits por construção.
        (self.0 >> ENTROPY_BITS) as u64
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut text = [0u8; TEXT_LEN];
        for (i, slot) in text.iter_mut().enumerate() {
            let shift = 5 * (TEXT_LEN - 1 - i);
            *slot = CROCKFORD[((self.0 >> shift) & 31) as usize];
        }
        // Só bytes ASCII do alfabeto.
        f.write_str(std::str::from_utf8(&text).map_err(|_| fmt::Error)?)
    }
}

impl FromStr for SessionId {
    type Err = SessionError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let invalid = || SessionError::InvalidSessionId(text.to_string());
        if text.len() != TEXT_LEN {
            return Err(invalid());
        }
        let mut value: u128 = 0;
        for (i, byte) in text.bytes().enumerate() {
            let digit = CROCKFORD
                .iter()
                .position(|&c| c == byte)
                .ok_or_else(invalid)?;
            // 26 × 5 = 130 bits: o primeiro caractere só pode usar 3 bits (0-7).
            if i == 0 && digit > 7 {
                return Err(invalid());
            }
            value = (value << 5) | digit as u128;
        }
        Ok(Self(value))
    }
}

impl From<SessionId> for String {
    fn from(id: SessionId) -> Self {
        id.to_string()
    }
}

impl TryFrom<String> for SessionId {
    type Error = SessionError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

/// Como a sessão captura e como o ASR trata cada canal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    /// Call com fone: mic é "eu", sistema é "eles" (ADR-0005).
    Meeting,
    /// Todos na sala: o mic também vai com diarização.
    InPerson,
    /// Só o áudio do sistema (aula, vídeo), sem mic.
    SystemOnly,
    /// Arquivo ou URL importado: não captura.
    Import,
}

impl SessionMode {
    /// Se a sessão abre uma captura ao vivo.
    pub fn records(self) -> bool {
        !matches!(self, Self::Import)
    }

    /// Se o canal do mic existe nesta sessão.
    pub fn has_mic(self) -> bool {
        matches!(self, Self::Meeting | Self::InPerson)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: u64 = 1_727_000_000_000;

    #[test]
    fn from_parts_encodes_time_and_round_trips() {
        let id = SessionId::from_parts(T0, 0);
        let text = id.to_string();
        assert_eq!(text, format!("01J8CKHDG0{}", "0".repeat(16)));
        assert_eq!(text.len(), 26);
        assert_eq!(text.parse::<SessionId>().unwrap(), id);
        assert_eq!(id.unix_ms(), T0);

        let max = SessionId::from_parts(T0, u128::MAX).to_string();
        assert_eq!(&max[..10], "01J8CKHDG0");
        assert_eq!(&max[10..], "Z".repeat(16));
        assert_eq!(max.parse::<SessionId>().unwrap().to_string(), max);
    }

    #[test]
    fn rejects_bad_text() {
        let good = SessionId::from_parts(T0, 12345).to_string();
        let short = &good[..25];
        let long = format!("{good}0");
        for bad in [short.to_string(), long] {
            assert_eq!(
                bad.parse::<SessionId>(),
                Err(SessionError::InvalidSessionId(bad.clone()))
            );
        }
        for letter in ['I', 'L', 'O', 'U'] {
            let bad = format!("{}{letter}", &good[..25]);
            assert_eq!(
                bad.parse::<SessionId>(),
                Err(SessionError::InvalidSessionId(bad.clone()))
            );
        }
    }

    #[test]
    fn generate_unique_and_time_ordered() {
        let a = SessionId::generate(T0).unwrap();
        let b = SessionId::generate(T0).unwrap();
        let later = SessionId::generate(T0 + 1).unwrap();
        assert_ne!(a, b);
        assert!(later.to_string() > a.to_string());
        assert!(later.to_string() > b.to_string());
    }

    #[test]
    fn mode_and_id_serialized_forms() {
        for (mode, json) in [
            (SessionMode::Meeting, "\"meeting\""),
            (SessionMode::InPerson, "\"in_person\""),
            (SessionMode::SystemOnly, "\"system_only\""),
            (SessionMode::Import, "\"import\""),
        ] {
            assert_eq!(serde_json::to_string(&mode).unwrap(), json);
            assert_eq!(serde_json::from_str::<SessionMode>(json).unwrap(), mode);
        }

        let id = SessionId::from_parts(T0, 0xABCDEF);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{id}\""));
        assert_eq!(json.len(), 26 + 2);
        assert_eq!(serde_json::from_str::<SessionId>(&json).unwrap(), id);
    }
}
