use serde::{Deserialize, Serialize};

/// Termos do dicionário pessoal, normalizados: sem espaço nas pontas, sem vazios e sem
/// duplicatas que só diferem em maiúsculas (fica a primeira grafia, na ordem recebida).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_terms() {
        let dictionary = Dictionary::new([" Fala ", "fala", "", "ADR", "  "]);
        assert_eq!(dictionary.terms(), ["Fala", "ADR"]);
    }
}
