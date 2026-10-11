//! O `Formatter` local e determinístico: fillers, gaguejo, pontuação falada, dicionário, espaços
//! e maiúscula inicial. Sempre roda, sem rede, e é o texto que fica quando o LLM não responde.

use fala_core::{Language, Replacement};

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

/// Maior número de palavras de um gatilho de pontuação falada ("ponto de interrogação").
const MAX_TRIGGER_WORDS: usize = 3;

/// Regras locais. O dicionário e o idioma vêm do `FormatContext`; aqui só o que liga e desliga.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rules {
    /// Troca a pontuação falada ("vírgula", "nova linha", "comma") pelo sinal. Ligada por padrão.
    pub spoken_punctuation: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            spoken_punctuation: true,
        }
    }
}

impl Formatter for Rules {
    fn format(&self, text: &str, ctx: &FormatContext<'_>) -> Result<String, PostprocError> {
        Ok(self.format_with_hits(text, ctx).0)
    }
}

impl Rules {
    /// O texto das regras e os pares "falado → escrito" que entraram nele: índices em
    /// `ctx.dictionary.replacements()`, um por ocorrência, na ordem do texto.
    pub fn format_with_hits(&self, text: &str, ctx: &FormatContext<'_>) -> (String, Vec<usize>) {
        self.apply_counted(
            text,
            *ctx.language,
            ctx.dictionary.terms(),
            ctx.dictionary.replacements(),
        )
    }

    #[cfg(test)]
    fn apply(&self, text: &str, language: Language, terms: &[String]) -> String {
        self.apply_counted(text, language, terms, &[]).0
    }

    fn apply_counted(
        &self,
        text: &str,
        language: Language,
        terms: &[String],
        replacements: &[Replacement],
    ) -> (String, Vec<usize>) {
        let tokens = tokenize(text);
        let tokens = remove_fillers(tokens, language);
        let tokens = collapse_repeats(tokens);
        let tokens = if self.spoken_punctuation {
            spoken_punctuation(tokens, language)
        } else {
            tokens
        };
        let (tokens, hits) = apply_dictionary(tokens, terms, replacements);
        (capitalize_first(&join(&tokens)), hits)
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

/// Junta os tokens com um espaço, menos depois de uma quebra de linha.
fn join(tokens: &[Token]) -> String {
    let mut out = String::new();
    for token in tokens {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push(' ');
        }
        out.push_str(&token.render());
    }
    out
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

/// Os pares "falado → escrito" e depois os termos, num passe só do motor de frases. Um termo
/// casa com 1 a 3 palavras ditas (`charge bee` → `ChargeBee`); um par, com até tantas palavras
/// quantas tem o falado. Devolve os índices dos pares aplicados.
fn apply_dictionary(
    tokens: Vec<Token>,
    terms: &[String],
    replacements: &[Replacement],
) -> (Vec<Token>, Vec<usize>) {
    let pairs: Vec<Phrase<'_>> = replacements
        .iter()
        .enumerate()
        .map(|(index, pair)| Phrase {
            key: match_key(pair.spoken()),
            max_words: pair.spoken().split_whitespace().count(),
            text: pair.written(),
            id: Some(index),
        })
        .collect();
    let spellings: Vec<Phrase<'_>> = terms
        .iter()
        .map(|term| Phrase {
            key: match_key(term),
            max_words: MAX_TERM_WORDS,
            text: term,
            id: None,
        })
        .collect();
    replace_phrases(tokens, &[&pairs, &spellings])
}

/// Uma frase que o motor procura e o que entra no lugar dela.
#[derive(Debug, Clone)]
struct Phrase<'a> {
    /// A `match_key` da frase inteira, com as palavras juntas.
    key: String,
    /// Quantos tokens do texto a frase pode ocupar.
    max_words: usize,
    /// O texto que substitui a sequência; o lugar onde um snippet põe o seu marcador opaco.
    text: &'a str,
    /// Devolvido em `hits` a cada aplicação; `None` para o que não se conta.
    id: Option<usize>,
}

/// O motor de regras do dicionário: troca uma sequência de palavras inteiras cuja chave (sem
/// caixa, acento, hífen ou pontuação interna) bate com a de uma frase. Nunca junta palavras
/// separadas por pontuação e nunca casa com parte de uma palavra. Em cada posição, os grupos
/// são tentados em ordem e, dentro do grupo, a sequência mais longa primeiro; o texto que entrou
/// não é relido. Devolve os tokens e os `id` das frases aplicadas, um por ocorrência.
fn replace_phrases(tokens: Vec<Token>, groups: &[&[Phrase<'_>]]) -> (Vec<Token>, Vec<usize>) {
    let max = groups
        .iter()
        .flat_map(|group| group.iter())
        .filter(|phrase| !phrase.key.is_empty())
        .map(|phrase| phrase.max_words)
        .max()
        .unwrap_or(0);
    if max == 0 {
        return (tokens, Vec::new());
    }
    let mut out = Vec::with_capacity(tokens.len());
    let mut hits = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        // `keys[n - 1]`: a chave das `n` palavras que começam em `i`, se elas podem se juntar.
        let keys: Vec<Option<String>> = (1..=max.min(tokens.len() - i))
            .map(|n| window_key(&tokens[i..i + n]))
            .collect();
        let found = groups.iter().find_map(|group| {
            (1..=keys.len()).rev().find_map(|n| {
                let key = keys[n - 1].as_ref()?;
                group
                    .iter()
                    .find(|phrase| n <= phrase.max_words && phrase.key == *key)
                    .map(|phrase| (n, phrase))
            })
        });
        match found {
            Some((n, phrase)) => {
                out.push(Token {
                    lead: tokens[i].lead.clone(),
                    core: phrase.text.to_string(),
                    trail: tokens[i + n - 1].trail.clone(),
                });
                hits.extend(phrase.id);
                i += n;
            }
            None => {
                out.push(tokens[i].clone());
                i += 1;
            }
        }
    }
    (out, hits)
}

/// A chave das palavras juntas, ou `None` se há pontuação entre elas, um sinal sem palavra
/// (travessão, parêntese solto) ou nada que sobre na chave.
fn window_key(window: &[Token]) -> Option<String> {
    let n = window.len();
    let joinable = window
        .iter()
        .enumerate()
        .all(|(k, t)| (k == 0 || t.lead.is_empty()) && (k == n - 1 || t.trail.is_empty()));
    if !joinable || window.iter().any(|t| t.core.is_empty()) {
        return None;
    }
    let key: String = window.iter().map(|t| match_key(&t.core)).collect();
    (!key.is_empty()).then_some(key)
}

/// O sinal que um gatilho falado vira.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    /// `, . ? ! : ;`: cola na palavra anterior.
    Closing(char),
    /// `(` e a aspa de abertura: colam na próxima palavra.
    Open(char),
    /// `)` e a aspa de fechamento: colam na palavra anterior.
    Close(char),
    /// Travessão, entre espaços.
    Dash,
    /// Nova linha (`\n`) ou novo parágrafo (`\n\n`), sem espaço em volta.
    Break(&'static str),
}

impl Mark {
    /// Depois destes sinais a próxima palavra começa com maiúscula.
    fn ends_sentence(self) -> bool {
        matches!(self, Mark::Closing('.' | '?' | '!') | Mark::Break(_))
    }
}

/// Onde um gatilho vale.
#[derive(Debug, Clone, Copy)]
enum When {
    Anywhere,
    /// Forma solta e ambígua ("ponto", "period"): só numa pausa.
    AtPause,
    /// Menos antes destas palavras, que o fazem substantivo ("nova linha de produto").
    NotBefore(&'static str),
}

/// Uma forma falada, o sinal que ela vira e onde vale.
type Trigger = (&'static str, Mark, When);

const PT_OF: When = When::NotBefore("de do da dos das");
const EN_OF: When = When::NotBefore("of");

const TRIGGERS_PT_BR: &[Trigger] = &[
    ("vírgula", Mark::Closing(','), When::Anywhere),
    ("ponto", Mark::Closing('.'), When::AtPause),
    ("ponto final", Mark::Closing('.'), When::Anywhere),
    ("ponto de interrogação", Mark::Closing('?'), When::Anywhere),
    ("ponto de exclamação", Mark::Closing('!'), When::Anywhere),
    // "dois pontos percentuais", "dois pontos acima": placar ou índice, não sinal.
    (
        "dois pontos",
        Mark::Closing(':'),
        When::NotBefore("percentuais percentual de do da acima abaixo"),
    ),
    ("ponto e vírgula", Mark::Closing(';'), When::Anywhere),
    ("nova linha", Mark::Break("\n"), PT_OF),
    ("quebra de linha", Mark::Break("\n"), PT_OF),
    ("novo parágrafo", Mark::Break("\n\n"), PT_OF),
    ("abre parênteses", Mark::Open('('), When::Anywhere),
    ("abre parêntese", Mark::Open('('), When::Anywhere),
    ("fecha parênteses", Mark::Close(')'), When::Anywhere),
    ("fecha parêntese", Mark::Close(')'), When::Anywhere),
    ("abre aspas", Mark::Open('"'), When::Anywhere),
    ("fecha aspas", Mark::Close('"'), When::Anywhere),
    ("travessão", Mark::Dash, When::Anywhere),
];

const TRIGGERS_EN: &[Trigger] = &[
    ("comma", Mark::Closing(','), When::Anywhere),
    ("period", Mark::Closing('.'), When::AtPause),
    ("full stop", Mark::Closing('.'), When::Anywhere),
    ("question mark", Mark::Closing('?'), When::Anywhere),
    ("exclamation mark", Mark::Closing('!'), When::Anywhere),
    ("exclamation point", Mark::Closing('!'), When::Anywhere),
    ("colon", Mark::Closing(':'), When::Anywhere),
    ("semicolon", Mark::Closing(';'), When::Anywhere),
    ("new line", Mark::Break("\n"), EN_OF),
    ("new paragraph", Mark::Break("\n\n"), EN_OF),
    ("open parenthesis", Mark::Open('('), When::Anywhere),
    ("open parentheses", Mark::Open('('), When::Anywhere),
    ("open paren", Mark::Open('('), When::Anywhere),
    ("close parenthesis", Mark::Close(')'), When::Anywhere),
    ("close parentheses", Mark::Close(')'), When::Anywhere),
    ("close paren", Mark::Close(')'), When::Anywhere),
    ("open quote", Mark::Open('"'), When::Anywhere),
    ("open quotes", Mark::Open('"'), When::Anywhere),
    ("close quote", Mark::Close('"'), When::Anywhere),
    ("close quotes", Mark::Close('"'), When::Anywhere),
    ("end quote", Mark::Close('"'), When::Anywhere),
    ("dash", Mark::Dash, When::Anywhere),
];

/// Palavras que, logo antes, fazem do gatilho um substantivo ("a vírgula", "o ponto", "com
/// vírgula"). Comparadas pela `match_key`, então sem acento.
const PT_NOUN_MARKERS: &str = "o a os as um uma uns umas esse essa esses essas este esta estes \
    estas aquele aquela aqueles aquelas no na nos nas num numa ao aos do da dos das pelo pela \
    pelos pelas de com sem cada qual quais meu minha meus minhas seu sua seus suas teu tua nosso \
    nossa outro outra mesmo mesma mais menos";
const EN_NOUN_MARKERS: &str = "a an the this that these those each every any no my your his \
    her its our their one another same to of with without";
/// Adjetivos que só barram as formas soltas ("um bom ponto", "trial period").
const PT_BARE_MARKERS: &str =
    "bom boa otimo grande unico certo ultimo primeiro segundo terceiro proximo principal";
const EN_BARE_MARKERS: &str =
    "long short trial grace waiting time whole entire last first next free notice transition";

/// As tabelas de um idioma: os gatilhos, as palavras que barram qualquer gatilho logo depois
/// delas e as que barram só as formas soltas.
fn tables(language: Language) -> (&'static [Trigger], &'static str, &'static str) {
    match language {
        Language::PtBr => (TRIGGERS_PT_BR, PT_NOUN_MARKERS, PT_BARE_MARKERS),
        Language::En => (TRIGGERS_EN, EN_NOUN_MARKERS, EN_BARE_MARKERS),
    }
}

/// A palavra está na lista separada por espaços.
fn listed(list: &str, key: &str) -> bool {
    list.split_whitespace().any(|word| word == key)
}

/// Troca a pontuação falada pelo sinal. Um gatilho são palavras inteiras, sem pontuação entre
/// elas; não vale logo depois de artigo e afins, nem antes das palavras do seu `NotBefore`; e as
/// formas soltas só valem numa pausa (fim do texto, sinal do ASR logo depois, ou outro gatilho).
fn spoken_punctuation(tokens: Vec<Token>, language: Language) -> Vec<Token> {
    let (triggers, markers, bare_markers) = tables(language);
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    // Abertura à espera da próxima palavra.
    let mut pending = String::new();
    let mut capitalize = false;
    let mut i = 0;
    while i < tokens.len() {
        if let Some((n, mark)) = trigger_at(&tokens, i, triggers, markers, bare_markers) {
            place(mark, &mut out, &mut pending);
            if mark.ends_sentence() {
                capitalize = true;
            } else if matches!(mark, Mark::Closing(_)) {
                capitalize = false;
            }
            i += n;
            continue;
        }
        let mut token = tokens[i].clone();
        token.lead.insert_str(0, &std::mem::take(&mut pending));
        if capitalize && !token.core.is_empty() {
            token.core = capitalize_first(&token.core);
            capitalize = false;
        }
        out.push(token);
        i += 1;
    }
    if !pending.is_empty() {
        out.push(bare(pending, String::new()));
    }
    out
}

/// O gatilho que começa em `i`, se ele vale aqui: quantas palavras ocupa e o sinal.
fn trigger_at(
    tokens: &[Token],
    i: usize,
    triggers: &[Trigger],
    markers: &str,
    bare_markers: &str,
) -> Option<(usize, Mark)> {
    let (n, (_, mark, when)) = lexical_trigger(tokens, i, triggers)?;
    if let Some(previous) = i.checked_sub(1).and_then(|p| tokens.get(p)) {
        let key = match_key(&previous.core);
        let bare_form = matches!(when, When::AtPause);
        let blocks = listed(markers, &key) || (bare_form && listed(bare_markers, &key));
        if previous.trail.is_empty() && blocks {
            return None;
        }
    }
    let last = &tokens[i + n - 1];
    let next = tokens.get(i + n);
    match (when, next) {
        (When::NotBefore(words), Some(next)) if last.trail.is_empty() && next.lead.is_empty() => {
            if listed(words, &match_key(&next.core)) {
                return None;
            }
        }
        (When::AtPause, Some(_)) => {
            let at_pause =
                !last.trail.is_empty() || lexical_trigger(tokens, i + n, triggers).is_some();
            if !at_pause {
                return None;
            }
        }
        _ => {}
    }
    Some((n, mark))
}

/// O gatilho mais longo que começa em `i`, só pela grafia. Sem pontuação antes ou entre as
/// palavras; depois da última, só o sinal que o ASR pôs.
fn lexical_trigger(tokens: &[Token], i: usize, triggers: &[Trigger]) -> Option<(usize, Trigger)> {
    let max = MAX_TRIGGER_WORDS.min(tokens.len().saturating_sub(i));
    for n in (1..=max).rev() {
        let window = &tokens[i..i + n];
        let whole_words = window.iter().enumerate().all(|(k, t)| {
            let trail_ok = if k == n - 1 {
                t.trail.chars().all(|c| CLOSING.contains(&c))
            } else {
                t.trail.is_empty()
            };
            t.lead.is_empty() && !t.core.is_empty() && trail_ok
        });
        if !whole_words {
            continue;
        }
        let key: String = window.iter().map(|t| match_key(&t.core)).collect();
        if let Some(trigger) = triggers
            .iter()
            .find(|(phrase, ..)| match_key(phrase) == key)
        {
            return Some((n, *trigger));
        }
    }
    None
}

/// Põe o sinal no lugar. O sinal que o ASR pôs depois do gatilho some; um sinal de fechamento
/// substitui o que já fecha a palavra anterior, então nunca se repete.
fn place(mark: Mark, out: &mut Vec<Token>, pending: &mut String) {
    let attached = match mark {
        Mark::Open(c) => {
            pending.push(c);
            return;
        }
        Mark::Closing(c) | Mark::Close(c) => c.to_string(),
        Mark::Break(text) => text.to_string(),
        Mark::Dash => String::new(),
    };
    if !pending.is_empty() {
        out.push(bare(std::mem::take(pending), String::new()));
    }
    if mark == Mark::Dash {
        out.push(bare("—".to_string(), String::new()));
        return;
    }
    match out.last_mut() {
        Some(previous) => {
            if matches!(mark, Mark::Closing(_))
                && previous.trail.ends_with(|c: char| CLOSING.contains(&c))
            {
                previous.trail.pop();
            }
            previous.trail.push_str(&attached);
        }
        None => out.push(bare(String::new(), attached)),
    }
}

/// Um token só de sinal, sem palavra.
fn bare(lead: String, trail: String) -> Token {
    Token {
        lead,
        core: String::new(),
        trail,
    }
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
        Rules::default().apply(text, Language::PtBr, &[])
    }

    fn with_dict(text: &str) -> String {
        let terms = ["ChargeBee", "Itaú", "Fala Cloud Sync"].map(String::from);
        Rules::default().apply(text, Language::PtBr, &terms)
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
        let en = |text: &str| Rules::default().apply(text, Language::En, &[]);
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

    /// Os termos de `with_dict` mais três pares; devolve o texto e os índices aplicados.
    fn with_pairs(text: &str) -> (String, Vec<usize>) {
        let terms = ["ChargeBee", "Itaú", "Fala Cloud Sync"].map(String::from);
        let pairs = [
            ("fala sql", "Fala.sqlite"),
            ("btw", "by the way"),
            ("itau bba", "Itaú BBA"),
        ]
        .map(|(spoken, written)| Replacement::new(spoken, written).unwrap());
        Rules::default().apply_counted(text, Language::PtBr, &terms, &pairs)
    }

    #[test]
    fn replacement_matches_spelling_variants() {
        for (input, want) in [
            ("abre o fala sql agora", "Abre o Fala.sqlite agora"),
            ("abre o Fala SQL.", "Abre o Fala.sqlite."),
            ("abre o FALA-SQL, depois", "Abre o Fala.sqlite, depois"),
            ("(fala sql)", "(Fala.sqlite)"),
            ("ok btw", "Ok by the way"),
            ("BTW, é isso", "By the way, é isso"),
            ("conta no ítaú bbá", "Conta no Itaú BBA"),
        ] {
            assert_eq!(with_pairs(input).0, want, "{input}");
        }
    }

    #[test]
    fn replacement_only_whole_words() {
        for (input, want) in [
            ("o falasqlx abriu", "O falasqlx abriu"),
            ("xbtw e btwx", "Xbtw e btwx"),
            ("fala, sql", "Fala, sql"),
            ("fala. sql", "Fala. sql"),
            // Um falado de uma palavra nunca junta duas.
            ("b tw", "B tw"),
        ] {
            assert_eq!(with_pairs(input), (want.to_string(), vec![]), "{input}");
        }
    }

    #[test]
    fn replacement_wins_and_is_not_reread() {
        // "itau bba" é par; "itau" sozinho é termo: o par vence e ninguém relê o escrito.
        assert_eq!(
            with_pairs("itau bba e itau"),
            ("Itaú BBA e Itaú".to_string(), vec![2])
        );
        let terms = ["Way".to_string()];
        let pairs = [Replacement::new("btw", "by the way").unwrap()];
        assert_eq!(
            Rules::default().apply_counted("ok btw", Language::En, &terms, &pairs),
            ("Ok by the way".to_string(), vec![0])
        );
    }

    #[test]
    fn replacement_hits_once_per_occurrence() {
        assert_eq!(
            with_pairs("btw fala sql vírgula btw"),
            (
                "By the way Fala.sqlite, by the way".to_string(),
                vec![1, 0, 1]
            )
        );
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

    fn en(text: &str) -> String {
        Rules::default().apply(text, Language::En, &[])
    }

    const PT_BR_TRIGGERS: &[(&str, &str)] = &[
        ("sim vírgula não", "Sim, não"),
        ("fim ponto", "Fim."),
        ("fim ponto final", "Fim."),
        ("tudo bem ponto de interrogação", "Tudo bem?"),
        ("que ótimo ponto de exclamação", "Que ótimo!"),
        ("lista dois pontos um", "Lista: um"),
        ("primeiro ponto e vírgula segundo", "Primeiro; segundo"),
        ("oi nova linha tchau", "Oi\nTchau"),
        ("oi quebra de linha tchau", "Oi\nTchau"),
        ("oi novo parágrafo tchau", "Oi\n\nTchau"),
        (
            "veja abre parênteses nota fecha parênteses aqui",
            "Veja (nota) aqui",
        ),
        (
            "veja abre parêntese nota fecha parêntese aqui",
            "Veja (nota) aqui",
        ),
        ("ele disse abre aspas oi fecha aspas", "Ele disse \"oi\""),
        ("isso travessão aquilo", "Isso — aquilo"),
    ];

    const EN_TRIGGERS: &[(&str, &str)] = &[
        ("yes comma no", "Yes, no"),
        ("done period", "Done."),
        ("done full stop", "Done."),
        ("really question mark", "Really?"),
        ("great exclamation mark", "Great!"),
        ("great exclamation point", "Great!"),
        ("list colon one", "List: one"),
        ("first semicolon second", "First; second"),
        ("hi new line bye", "Hi\nBye"),
        ("hi newline bye", "Hi\nBye"),
        ("hi new paragraph bye", "Hi\n\nBye"),
        (
            "see open parenthesis note close parenthesis here",
            "See (note) here",
        ),
        ("see open paren note close paren here", "See (note) here"),
        ("he said open quote hi close quote", "He said \"hi\""),
        ("he said open quotes hi close quotes", "He said \"hi\""),
        ("wait dash that", "Wait — that"),
    ];

    #[test]
    fn spoken_pt_br_each_trigger() {
        for (input, want) in PT_BR_TRIGGERS {
            assert_eq!(pt(input), *want, "{input}");
        }
    }

    #[test]
    fn spoken_en_each_trigger() {
        for (input, want) in EN_TRIGGERS {
            assert_eq!(en(input), *want, "{input}");
        }
    }

    #[test]
    fn spoken_spacing() {
        assert_eq!(
            pt(
                "x vírgula b ponto final c ponto de interrogação d ponto de exclamação e dois \
                pontos f ponto e vírgula g"
            ),
            "X, b. C? D! E: f; g"
        );
        assert_eq!(pt("x nova linha y"), "X\nY");
        assert_eq!(pt("x abre parênteses y fecha parênteses z"), "X (y) z");
        assert_eq!(pt("x travessão y"), "X — y");
    }

    #[test]
    fn spoken_capitalizes_after_sentence_end() {
        assert_eq!(pt("sim ponto final não"), "Sim. Não");
        assert_eq!(pt("sério ponto de interrogação sim"), "Sério? Sim");
        assert_eq!(pt("uau ponto de exclamação sim"), "Uau! Sim");
        assert_eq!(pt("oi nova linha tchau"), "Oi\nTchau");
        assert_eq!(pt("oi novo parágrafo tchau"), "Oi\n\nTchau");
        assert_eq!(pt("sim vírgula não"), "Sim, não");
        assert_eq!(pt("lista dois pontos um"), "Lista: um");
        assert_eq!(pt("x ponto e vírgula b"), "X; b");
    }

    #[test]
    fn spoken_does_not_double_asr_punctuation() {
        assert_eq!(pt("eu acho, vírgula que sim"), "Eu acho, que sim");
        assert_eq!(pt("Tudo bem. Ponto final."), "Tudo bem.");
        assert_eq!(pt("sim vírgula, não"), "Sim, não");
        assert_eq!(pt("sim, ponto final não"), "Sim. Não");
        assert_eq!(pt("sim vírgula vírgula não"), "Sim, não");
        assert_eq!(en("Done. Period."), "Done.");
    }

    #[test]
    fn spoken_only_whole_words() {
        for (input, want) in [
            ("ele apontou isso", "Ele apontou isso"),
            ("isso é ponto-chave", "Isso é ponto-chave"),
            ("foi virgulado", "Foi virgulado"),
        ] {
            assert_eq!(pt(input), want, "{input}");
        }
        for (input, want) in [
            ("commas are fine", "Commas are fine"),
            ("periodically", "Periodically"),
            ("colonial era", "Colonial era"),
            ("dashboard ready", "Dashboard ready"),
        ] {
            assert_eq!(en(input), want, "{input}");
        }
    }

    #[test]
    fn spoken_pt_br_exceptions() {
        for (input, want) in [
            (
                "do meu ponto de vista está certo",
                "Do meu ponto de vista está certo",
            ),
            ("o ponto de encontro é aqui", "O ponto de encontro é aqui"),
            ("esse é o ponto de partida", "Esse é o ponto de partida"),
            (
                "subiu dois pontos percentuais",
                "Subiu dois pontos percentuais",
            ),
            ("temos nova linha de produto", "Temos nova linha de produto"),
            ("a vírgula está errada", "A vírgula está errada"),
            ("esse é o ponto.", "Esse é o ponto."),
            ("foi um bom ponto.", "Foi um bom ponto."),
            ("bateu no travessão", "Bateu no travessão"),
            ("ganhamos os dois pontos", "Ganhamos os dois pontos"),
        ] {
            assert_eq!(pt(input), want, "{input}");
        }
    }

    #[test]
    fn spoken_en_exceptions() {
        for (input, want) in [
            ("it was a long period.", "It was a long period."),
            ("the trial period.", "The trial period."),
            ("add a comma here", "Add a comma here"),
            ("new line of products", "New line of products"),
            ("a dash of salt", "A dash of salt"),
            ("I have to dash.", "I have to dash."),
            ("the colon is an organ", "The colon is an organ"),
        ] {
            assert_eq!(en(input), want, "{input}");
        }
    }

    #[test]
    fn spoken_bare_ponto_needs_pause() {
        assert_eq!(pt("fim ponto"), "Fim.");
        assert_eq!(pt("Fim ponto. Depois"), "Fim. Depois");
        assert_eq!(pt("fim ponto nova linha depois"), "Fim.\nDepois");
        assert_eq!(pt("vamos ponto depois"), "Vamos ponto depois");
        assert_eq!(en("we left period. Then"), "We left. Then");
        assert_eq!(en("we left period new line then"), "We left.\nThen");
        assert_eq!(en("we left period then"), "We left period then");
    }

    #[test]
    fn spoken_follows_language() {
        assert_eq!(en("sim vírgula não"), "Sim vírgula não");
        assert_eq!(pt("yes comma no"), "Yes comma no");
    }

    #[test]
    fn spoken_flag_off_and_default_on() {
        assert!(Rules::default().spoken_punctuation);
        let off = Rules {
            spoken_punctuation: false,
        };
        let cases = PT_BR_TRIGGERS
            .iter()
            .map(|(input, _)| (*input, Language::PtBr))
            .chain(EN_TRIGGERS.iter().map(|(input, _)| (*input, Language::En)));
        for (input, language) in cases {
            let mut want = input.to_string();
            want.replace_range(..1, &input[..1].to_uppercase());
            assert_eq!(off.apply(input, language, &[]), want, "{input}");
        }
    }

    #[test]
    fn spoken_order_with_other_rules() {
        assert_eq!(pt("então hã vírgula eu acho"), "Então, eu acho");
        assert_eq!(pt("fim ponto ponto ponto"), "Fim.");
        assert_eq!(
            with_dict("charge bee vírgula itau ponto final"),
            "ChargeBee, Itaú."
        );
        assert_eq!(with_dict("itau travessão itau"), "Itaú — Itaú");
        assert_eq!(with_dict("abre parênteses itau fecha parênteses"), "(Itaú)");
        assert_eq!(pt("nova linha oi"), "\nOi");
    }
}
