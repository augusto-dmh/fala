//! O `Formatter` local e determinístico: fillers, gaguejo, dicionário, espaços e maiúscula
//! inicial. Sempre roda, sem rede, e é o texto que fica quando o LLM não responde.

use fala_core::Language;

use crate::{FormatContext, Formatter, PostprocError};

/// Fillers removidos em pt-BR.
const FILLERS_PT_BR: &[&str] = &[
    "hã", "hãã", "ãh", "ahn", "hum", "humm", "hmm", "uh", "uhm", "éé",
];
/// Fillers removidos nos outros idiomas: só os que não são palavras do idioma.
const FILLERS_OTHER: &[&str] = &["uh", "uhm", "hmm"];
/// Sinais que fecham uma sequência do dicionário e perdem o espaço antes.
const CLOSING: &[char] = &[',', '.', ';', ':', '?', '!'];
/// Maior número de palavras ditadas que viram um termo do dicionário.
const MAX_TERM_WORDS: usize = 3;
/// A partir de quantas repetições seguidas uma palavra vira uma.
const MIN_REPEATS: usize = 3;

/// Regras pt-BR locais. Sem estado: o dicionário e o idioma vêm do `FormatContext`.
#[derive(Debug, Default, Clone, Copy)]
pub struct Rules;

impl Formatter for Rules {
    fn format(&self, text: &str, ctx: &FormatContext<'_>) -> Result<String, PostprocError> {
        Ok(apply(text, *ctx.language, ctx.dictionary.terms()))
    }
}

/// Uma palavra separada em pontuação de abertura, miolo e pontuação de fechamento.
#[derive(Debug, Clone)]
struct Token {
    lead: String,
    core: String,
    trail: String,
}

impl Token {
    fn parse(word: &str) -> Self {
        let start = word.find(char::is_alphanumeric).unwrap_or(word.len());
        let end = word
            .rfind(char::is_alphanumeric)
            .map(|i| i + word[i..].chars().next().map_or(1, char::len_utf8))
            .unwrap_or(start)
            .max(start);
        Self {
            lead: word[..start].to_string(),
            core: word[start..end].to_string(),
            trail: word[end..].to_string(),
        }
    }

    fn render(&self) -> String {
        format!("{}{}{}", self.lead, self.core, self.trail)
    }
}

fn apply(text: &str, language: Language, terms: &[String]) -> String {
    let tokens = tokenize(text);
    let tokens = remove_fillers(tokens, language);
    let tokens = collapse_repeats(tokens);
    let tokens = apply_dictionary(tokens, terms);
    let joined = tokens
        .iter()
        .map(Token::render)
        .collect::<Vec<_>>()
        .join(" ");
    capitalize_first(&joined)
}

/// Divide por espaço e cola no token anterior o sinal de fechamento que veio solto.
fn tokenize(text: &str) -> Vec<Token> {
    let mut words: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        let only_closing = word.chars().all(|c| CLOSING.contains(&c));
        match words.last_mut() {
            Some(previous) if only_closing => previous.push_str(word),
            _ => words.push(word.to_string()),
        }
    }
    words.iter().map(|w| Token::parse(w)).collect()
}

fn remove_fillers(tokens: Vec<Token>, language: Language) -> Vec<Token> {
    let fillers = match language {
        Language::PtBr => FILLERS_PT_BR,
        Language::En => FILLERS_OTHER,
    };
    let mut kept: Vec<Token> = Vec::with_capacity(tokens.len());
    // Um filler maiúsculo que abria a frase passa a maiúscula para a palavra que fica no lugar:
    // "Isso funciona. Uhm, deixa eu ver" vira "Isso funciona. Deixa eu ver".
    let mut capital_owed = false;
    for mut token in tokens {
        let is_filler = token.lead.is_empty()
            && matches!(token.trail.as_str(), "" | "," | ".")
            && fillers.contains(&token.core.to_lowercase().as_str());
        if is_filler {
            let capitalized = token.core.starts_with(char::is_uppercase);
            capital_owed |= capitalized && opens_sentence(kept.last());
            continue;
        }
        if capital_owed && !token.core.is_empty() {
            token.core = capitalize_first(&token.core);
            capital_owed = false;
        }
        kept.push(token);
    }
    kept
}

/// Se uma palavra depois de `previous` abre frase: não há palavra antes, ou a anterior termina
/// em `.`, `!`, `?` ou `…`.
fn opens_sentence(previous: Option<&Token>) -> bool {
    previous.is_none_or(|t| {
        t.trail
            .chars()
            .next_back()
            .is_some_and(|c| matches!(c, '.' | '!' | '?' | '…'))
    })
}

/// `eu eu eu` vira `eu`; duas repetições ficam (`o que que é`). A sequência só vale sem
/// pontuação entre as palavras, e a pontuação final da última é mantida.
fn collapse_repeats(tokens: Vec<Token>) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        let first = &tokens[i];
        let mut j = i + 1;
        while j < tokens.len()
            && tokens[j - 1].trail.is_empty()
            && tokens[j].lead.is_empty()
            && !first.core.is_empty()
            && tokens[j].core.to_lowercase() == first.core.to_lowercase()
        {
            j += 1;
        }
        if j - i >= MIN_REPEATS {
            let mut kept = first.clone();
            kept.trail = tokens[j - 1].trail.clone();
            out.push(kept);
            i = j;
        } else {
            out.push(first.clone());
            i += 1;
        }
    }
    out
}

/// Troca de 1 a 3 palavras pela grafia do termo quando as chaves batem (sem caixa, acento,
/// hífen ou pontuação interna). Nunca junta palavras separadas por pontuação.
fn apply_dictionary(tokens: Vec<Token>, terms: &[String]) -> Vec<Token> {
    let keyed: Vec<(String, &String)> = terms
        .iter()
        .map(|term| (match_key(term), term))
        .filter(|(key, _)| !key.is_empty())
        .collect();
    if keyed.is_empty() {
        return tokens;
    }
    let mut out = Vec::with_capacity(tokens.len());
    let mut i = 0;
    'outer: while i < tokens.len() {
        for n in (1..=MAX_TERM_WORDS.min(tokens.len() - i)).rev() {
            let window = &tokens[i..i + n];
            let joinable = window
                .iter()
                .enumerate()
                .all(|(k, t)| (k == 0 || t.lead.is_empty()) && (k == n - 1 || t.trail.is_empty()));
            if !joinable {
                continue;
            }
            let key: String = window.iter().map(|t| match_key(&t.core)).collect();
            if key.is_empty() {
                continue;
            }
            if let Some((_, term)) = keyed.iter().find(|(k, _)| *k == key) {
                out.push(Token {
                    lead: window[0].lead.clone(),
                    core: (*term).clone(),
                    trail: window[n - 1].trail.clone(),
                });
                i += n;
                continue 'outer;
            }
        }
        out.push(tokens[i].clone());
        i += 1;
    }
    out
}

/// Minúsculas, sem acento e só letras e dígitos: `Charge-Bee` e `charge bee` viram `chargebee`.
fn match_key(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(strip_accent)
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn strip_accent(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        other => other,
    }
}

fn capitalize_first(text: &str) -> String {
    match text.char_indices().find(|(_, c)| c.is_alphabetic()) {
        Some((i, c)) => {
            let mut out = String::with_capacity(text.len() + 2);
            out.push_str(&text[..i]);
            out.extend(c.to_uppercase());
            out.push_str(&text[i + c.len_utf8()..]);
            out
        }
        None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(text: &str) -> String {
        apply(text, Language::PtBr, &[])
    }

    fn with_dict(text: &str) -> String {
        let terms = ["ChargeBee", "Itaú", "Fala Cloud Sync"].map(String::from);
        apply(text, Language::PtBr, &terms)
    }

    #[test]
    fn removes_pt_br_fillers() {
        assert_eq!(pt("então, hã, eu acho"), "Então, eu acho");
        for filler in [
            "hã", "hãã", "ãh", "ahn", "hum", "humm", "hmm", "uh", "uhm", "éé",
        ] {
            assert_eq!(pt(&format!("eu {filler} acho")), "Eu acho", "{filler}");
            assert_eq!(
                pt(&format!("eu {}, acho", filler.to_uppercase())),
                "Eu acho",
                "{filler} maiúsculo com vírgula"
            );
            assert_eq!(
                pt(&format!("eu {filler}. acho")),
                "Eu acho",
                "{filler} com ponto"
            );
            assert_eq!(
                pt(&format!("eu {filler}x acho")),
                format!("Eu {filler}x acho"),
                "{filler} dentro de palavra"
            );
        }
    }

    #[test]
    fn keeps_sentence_capital_after_removed_filler() {
        assert_eq!(
            pt("Isso funciona. Uhm, deixa eu ver."),
            "Isso funciona. Deixa eu ver."
        );
        assert_eq!(pt("Pronto! Hã, então vamos."), "Pronto! Então vamos.");
        assert_eq!(pt("Será? Ahn ahn, é isso."), "Será? É isso.");
        assert_eq!(pt("Certo… Hum, ótimo."), "Certo… Ótimo.");
        // No meio da frase não há maiúscula a passar adiante.
        assert_eq!(pt("ele disse, Hum, hoje não."), "Ele disse, hoje não.");
        // Filler minúsculo depois do ponto: a regra só passa a maiúscula que existia.
        assert_eq!(pt("Isso funciona. uhm, deixa."), "Isso funciona. deixa.");
    }

    #[test]
    fn en_keeps_pt_only_fillers() {
        let en = |text: &str| apply(text, Language::En, &[]);
        assert_eq!(en("ahn ok"), "Ahn ok");
        for filler in ["uh", "uhm", "hmm"] {
            assert_eq!(en(&format!("i {filler} think")), "I think", "{filler}");
        }
        for kept in ["hã", "hãã", "ãh", "ahn", "hum", "humm", "éé"] {
            assert_eq!(
                en(&format!("i {kept} think")),
                format!("I {kept} think"),
                "{kept}"
            );
        }
    }

    #[test]
    fn collapses_three_or_more_repeats() {
        assert_eq!(pt("eu eu eu acho"), "Eu acho");
        assert_eq!(pt("Eu EU eu eu acho"), "Eu acho");
        assert_eq!(pt("o que que é"), "O que que é");
    }

    #[test]
    fn dictionary_replaces_spelling_variants() {
        assert_eq!(with_dict("a charge bee e o itau."), "A ChargeBee e o Itaú.");
        assert_eq!(with_dict("charge-bee"), "ChargeBee");
        assert_eq!(with_dict("CHARGEBEE"), "ChargeBee");
        assert_eq!(with_dict("fala cloud sync"), "Fala Cloud Sync");
        assert_eq!(with_dict("falacloud sync"), "Fala Cloud Sync");
    }

    #[test]
    fn dictionary_does_not_join_across_punctuation() {
        for sign in [',', '.', ';', ':', '?', '!'] {
            assert_eq!(
                with_dict(&format!("charge{sign} bee")),
                format!("Charge{sign} bee"),
                "{sign}"
            );
        }
    }

    #[test]
    fn dictionary_ignores_partial_words() {
        assert_eq!(with_dict("chargebeex"), "Chargebeex");
        assert_eq!(with_dict("xchargebee"), "Xchargebee");
    }

    #[test]
    fn normalizes_spacing_and_capitalizes() {
        assert_eq!(pt("  eu   acho ,  que sim .  "), "Eu acho, que sim.");
        for sign in [',', '.', ';', ':', '?', '!'] {
            assert_eq!(
                pt(&format!("sim {sign} não")),
                format!("Sim{sign} não"),
                "{sign}"
            );
        }
        assert_eq!(pt("é isso"), "É isso");
    }
}
