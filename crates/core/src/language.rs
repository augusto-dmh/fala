use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::CoreError;

/// Idioma de um ditado. Serializado como a tag BCP-47 (`"pt-BR"`, `"en"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "pt-BR")]
    PtBr,
    #[serde(rename = "en")]
    En,
}

impl Language {
    /// A tag BCP-47, igual à forma serializada.
    pub fn tag(self) -> &'static str {
        match self {
            Language::PtBr => "pt-BR",
            Language::En => "en",
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag())
    }
}

impl FromStr for Language {
    type Err = CoreError;

    /// Aceita `pt-BR`, `pt` e `en`, sem diferenciar maiúsculas.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "pt-br" | "pt" => Ok(Language::PtBr),
            "en" => Ok(Language::En),
            _ => Err(CoreError::UnknownLanguage(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_tags() {
        assert_eq!("pt-BR".parse::<Language>(), Ok(Language::PtBr));
        assert_eq!("PT-br".parse::<Language>(), Ok(Language::PtBr));
        assert_eq!("pt".parse::<Language>(), Ok(Language::PtBr));
        assert_eq!("en".parse::<Language>(), Ok(Language::En));
    }

    #[test]
    fn rejects_unknown_tags() {
        for input in ["es", "pt-PT", ""] {
            assert_eq!(
                input.parse::<Language>(),
                Err(CoreError::UnknownLanguage(input.to_string()))
            );
        }
    }

    #[test]
    fn serializes_as_bcp47_tag() {
        assert_eq!(serde_json::to_string(&Language::PtBr).unwrap(), "\"pt-BR\"");
        assert_eq!(serde_json::to_string(&Language::En).unwrap(), "\"en\"");
        assert_eq!(
            serde_json::from_str::<Language>("\"pt-BR\"").unwrap(),
            Language::PtBr
        );
        assert_eq!(
            serde_json::from_str::<Language>("\"en\"").unwrap(),
            Language::En
        );
    }

    #[test]
    fn tag_matches_serialized_form() {
        assert_eq!(Language::PtBr.tag(), "pt-BR");
        assert_eq!(Language::En.tag(), "en");
        for lang in [Language::PtBr, Language::En] {
            let json = serde_json::to_string(&lang).unwrap();
            assert_eq!(json.trim_matches('"'), lang.tag());
        }
    }
}
