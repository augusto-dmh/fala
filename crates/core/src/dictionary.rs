use serde::{Deserialize, Serialize};

/// Termos do dicionário pessoal, normalizados: sem espaço nas pontas, sem vazios e sem
/// duplicatas que só diferem em maiúsculas (fica a primeira grafia, na ordem recebida).
/// Desserializar passa pela mesma normalização de `Dictionary::new`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "DictionaryTerms")]
pub struct Dictionary {
    terms: Vec<String>,
}

impl Dictionary {
    pub fn new<I, S>(terms: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut seen = std::collections::HashSet::new();
        let terms = terms
            .into_iter()
            .map(|term| term.as_ref().trim().to_string())
            .filter(|term| !term.is_empty() && seen.insert(term.to_lowercase()))
            .collect();
        Self { terms }
    }

    pub fn terms(&self) -> &[String] {
        &self.terms
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }
}

#[derive(Deserialize)]
struct DictionaryTerms {
    terms: Vec<String>,
}

impl From<DictionaryTerms> for Dictionary {
    fn from(raw: DictionaryTerms) -> Self {
        Dictionary::new(raw.terms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_terms() {
        let dictionary = Dictionary::new([" Fala ", "fala", "", "ADR", "  "]);
        assert_eq!(dictionary.terms(), ["Fala", "ADR"]);
    }

    #[test]
    fn deserializing_normalizes_terms() {
        let json = r#"{"terms":[" Fala ","fala","","ADR","  "]}"#;
        let dictionary: Dictionary = serde_json::from_str(json).unwrap();
        assert_eq!(dictionary.terms(), ["Fala", "ADR"]);
        assert_eq!(
            serde_json::to_string(&dictionary).unwrap(),
            r#"{"terms":["Fala","ADR"]}"#
        );
    }
}
