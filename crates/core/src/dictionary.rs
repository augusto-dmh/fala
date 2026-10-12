use serde::{Deserialize, Serialize};

use crate::Snippet;

/// O dicionário pessoal: termos (a grafia certa de um nome) e pares "falado → escrito".
///
/// Os termos ficam normalizados: sem espaço nas pontas, sem vazios e sem duplicatas que só
/// diferem em maiúsculas (fica a primeira grafia, na ordem recebida). O escrito de cada par
/// também é termo, porque é o que o LLM recebe; o falado nunca é. Desserializar passa pela mesma
/// normalização de `Dictionary::new` e `with_replacements`, e um JSON sem `replacements` carrega
/// sem pares. Os snippets ficam fora de `terms()` e do JSON: nem o gatilho nem a expansão vão
/// ao LLM.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "DictionaryRaw")]
pub struct Dictionary {
    terms: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    replacements: Vec<Replacement>,
    #[serde(skip)]
    snippets: Vec<Snippet>,
}

/// Um par "falado → escrito": quando o ditado tem `spoken`, o texto final tem `written`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replacement {
    spoken: String,
    written: String,
}

impl Replacement {
    /// `None` se o falado ou o escrito ficam vazios sem os espaços das pontas. Espaços seguidos
    /// no falado viram um.
    pub fn new(spoken: impl AsRef<str>, written: impl AsRef<str>) -> Option<Self> {
        let spoken = spoken
            .as_ref()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let written = written.as_ref().trim().to_string();
        (!spoken.is_empty() && !written.is_empty()).then_some(Self { spoken, written })
    }

    pub fn spoken(&self) -> &str {
        &self.spoken
    }

    pub fn written(&self) -> &str {
        &self.written
    }
}

impl Dictionary {
    pub fn new<I, S>(terms: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut dictionary = Self::default();
        for term in terms {
            dictionary.push_term(term.as_ref());
        }
        dictionary
    }

    /// Acrescenta pares, na ordem recebida. Par vazio some; falado repetido (sem caixa) fica com
    /// o primeiro. O escrito entra nos termos se ainda não estiver lá.
    pub fn with_replacements<I, S, W>(mut self, pairs: I) -> Self
    where
        I: IntoIterator<Item = (S, W)>,
        S: AsRef<str>,
        W: AsRef<str>,
    {
        for (spoken, written) in pairs {
            let Some(pair) = Replacement::new(spoken, written) else {
                continue;
            };
            let key = pair.spoken.to_lowercase();
            if self
                .replacements
                .iter()
                .any(|known| known.spoken.to_lowercase() == key)
            {
                continue;
            }
            self.push_term(&pair.written);
            self.replacements.push(pair);
        }
        self
    }

    /// Acrescenta snippets, na ordem recebida. O que `Snippet::new` recusa some; gatilho
    /// repetido (sem caixa) fica com o primeiro.
    pub fn with_snippets<I, T, E>(mut self, snippets: I) -> Self
    where
        I: IntoIterator<Item = (T, E)>,
        T: AsRef<str>,
        E: AsRef<str>,
    {
        for (trigger, expansion) in snippets {
            let Some(snippet) = Snippet::new(trigger, expansion) else {
                continue;
            };
            let key = snippet.trigger().to_lowercase();
            if !self
                .snippets
                .iter()
                .any(|known| known.trigger().to_lowercase() == key)
            {
                self.snippets.push(snippet);
            }
        }
        self
    }

    fn push_term(&mut self, term: &str) {
        let term = term.trim();
        let key = term.to_lowercase();
        if !term.is_empty() && !self.terms.iter().any(|t| t.to_lowercase() == key) {
            self.terms.push(term.to_string());
        }
    }

    /// Os termos e o escrito de cada par: o que o LLM recebe.
    pub fn terms(&self) -> &[String] {
        &self.terms
    }

    pub fn replacements(&self) -> &[Replacement] {
        &self.replacements
    }

    pub fn snippets(&self) -> &[Snippet] {
        &self.snippets
    }

    /// Sem termos: o prompt do LLM fica sem o bloco do dicionário.
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }
}

#[derive(Deserialize)]
struct DictionaryRaw {
    terms: Vec<String>,
    #[serde(default)]
    replacements: Vec<Replacement>,
}

impl From<DictionaryRaw> for Dictionary {
    fn from(raw: DictionaryRaw) -> Self {
        Dictionary::new(raw.terms).with_replacements(
            raw.replacements
                .into_iter()
                .map(|pair| (pair.spoken, pair.written)),
        )
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
        assert!(dictionary.replacements().is_empty());
        assert_eq!(
            serde_json::to_string(&dictionary).unwrap(),
            r#"{"terms":["Fala","ADR"]}"#
        );
    }

    #[test]
    fn replacements_are_normalized() {
        let dictionary = Dictionary::new(["Fala"]).with_replacements([
            ("  fala   sql ", " Fala.sqlite "),
            ("", "vazio"),
            ("vazio", "  "),
            ("FALA SQL", "outra coisa"),
            ("btw", "by the way"),
            ("fala", "fala"),
        ]);
        let pairs: Vec<(&str, &str)> = dictionary
            .replacements()
            .iter()
            .map(|p| (p.spoken(), p.written()))
            .collect();
        assert_eq!(
            pairs,
            [
                ("fala sql", "Fala.sqlite"),
                ("btw", "by the way"),
                ("fala", "fala")
            ]
        );
        // O escrito vira termo uma vez; o falado nunca.
        assert_eq!(dictionary.terms(), ["Fala", "Fala.sqlite", "by the way"]);
        assert!(!Dictionary::default()
            .with_replacements([("btw", "by the way")])
            .is_empty());
    }

    #[test]
    fn replacements_round_trip() {
        let dictionary = Dictionary::new(["ADR"]).with_replacements([("btw", "by the way")]);
        let json = serde_json::to_string(&dictionary).unwrap();
        assert_eq!(
            json,
            r#"{"terms":["ADR","by the way"],"replacements":[{"spoken":"btw","written":"by the way"}]}"#
        );
        // Os snippets nunca entram nos termos nem no JSON.
        let with_snippet = dictionary.clone().with_snippets([
            ("minha assinatura", "Augusto"),
            ("MINHA  Assinatura", "outro"),
        ]);
        assert_eq!(with_snippet.snippets().len(), 1);
        assert_eq!(with_snippet.terms(), dictionary.terms());
        assert_eq!(serde_json::to_string(&with_snippet).unwrap(), json);
        let back: Dictionary = serde_json::from_str(&json).unwrap();
        assert_eq!(back, dictionary);
        let raw = r#"{"terms":[],"replacements":[{"spoken":" ","written":"x"},{"spoken":"a b","written":"AB"}]}"#;
        let loaded: Dictionary = serde_json::from_str(raw).unwrap();
        assert_eq!(loaded.replacements().len(), 1);
        assert_eq!(loaded.terms(), ["AB"]);
    }
}
