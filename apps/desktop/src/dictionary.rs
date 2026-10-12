//! The personal dictionary as the app stores it: `custom_words` holds the terms (the inherited
//! field, unchanged) and `dictionary_replacements` the spoken -> written pairs, each with how many
//! times the rules applied it. `dictionary_snippets` holds the snippets in the same shape: the
//! trigger as `spoken`, the text as `written`. `dictionary` joins all three into the
//! `fala_core::Dictionary` every dictation uses; the written form of a pair reaches the LLM as a
//! term, the spoken form never, and a snippet never reaches it at all.

use chrono::{Datelike, Local, Timelike};
use fala_core::{render_snippet, Dictionary, Moment};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;

use crate::settings::{self, AppSettings};

/// A spoken -> written pair, or a trigger -> text snippet, as stored. `uses` counts every
/// occurrence the rules replaced.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Type)]
pub struct DictionaryReplacement {
    pub spoken: String,
    pub written: String,
    #[serde(default)]
    pub uses: u32,
}

/// The dictionary for one dictation: the terms, the pairs and the snippets, with `{data}`,
/// `{data_extenso}` and `{hora}` resolved now.
pub(crate) fn dictionary(settings: &AppSettings) -> Dictionary {
    let now = Local::now();
    dictionary_at(
        settings,
        Moment {
            year: now.year(),
            month: now.month(),
            day: now.day(),
            hour: now.hour(),
            minute: now.minute(),
        },
    )
}

fn dictionary_at(settings: &AppSettings, at: Moment) -> Dictionary {
    Dictionary::new(&settings.custom_words)
        .with_replacements(
            settings
                .dictionary_replacements
                .iter()
                .map(|pair| (pair.spoken.as_str(), pair.written.as_str())),
        )
        .with_snippets(settings.dictionary_snippets.iter().map(|snippet| {
            (
                snippet.spoken.as_str(),
                render_snippet(&snippet.written, at),
            )
        }))
}

/// How two spoken forms compare: without case and with runs of spaces as one, the comparison
/// `Dictionary::with_replacements` uses to drop repeats.
fn spoken_key(spoken: &str) -> String {
    spoken
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// The list as stored: normalized the way `Dictionary` normalizes it (trimmed, blanks and
/// repeated spoken forms dropped, first one kept). `uses` comes from the stored pair with the
/// same spoken form, never from the caller, so an edit from a stale page keeps the count.
pub(crate) fn normalize(
    incoming: Vec<DictionaryReplacement>,
    stored: &[DictionaryReplacement],
) -> Vec<DictionaryReplacement> {
    let dictionary = Dictionary::default().with_replacements(
        incoming
            .iter()
            .map(|pair| (pair.spoken.as_str(), pair.written.as_str())),
    );
    with_stored_uses(
        dictionary
            .replacements()
            .iter()
            .map(|pair| (pair.spoken(), pair.written())),
        stored,
    )
}

/// The snippets as stored: normalized the way `Dictionary::with_snippets` normalizes them (a
/// trigger needs two words; blanks and repeated triggers dropped). `uses` as in `normalize`.
pub(crate) fn normalize_snippets(
    incoming: Vec<DictionaryReplacement>,
    stored: &[DictionaryReplacement],
) -> Vec<DictionaryReplacement> {
    let dictionary = Dictionary::default().with_snippets(
        incoming
            .iter()
            .map(|snippet| (snippet.spoken.as_str(), snippet.written.as_str())),
    );
    with_stored_uses(
        dictionary
            .snippets()
            .iter()
            .map(|snippet| (snippet.trigger(), snippet.expansion())),
        stored,
    )
}

fn with_stored_uses<'a>(
    normalized: impl Iterator<Item = (&'a str, &'a str)>,
    stored: &[DictionaryReplacement],
) -> Vec<DictionaryReplacement> {
    normalized
        .map(|(spoken, written)| {
            let key = spoken_key(spoken);
            let uses = stored
                .iter()
                .find(|known| spoken_key(&known.spoken) == key)
                .map_or(0, |known| known.uses);
            DictionaryReplacement {
                spoken: spoken.to_string(),
                written: written.to_string(),
                uses,
            }
        })
        .collect()
}

/// Adds one use per applied occurrence. A spoken form no longer stored is ignored. Returns
/// whether anything changed.
pub(crate) fn count_uses(list: &mut [DictionaryReplacement], applied: &[String]) -> bool {
    let mut changed = false;
    for spoken in applied {
        let key = spoken_key(spoken);
        if let Some(pair) = list.iter_mut().find(|pair| spoken_key(&pair.spoken) == key) {
            pair.uses = pair.uses.saturating_add(1);
            changed = true;
        }
    }
    changed
}

/// Stores the uses of one dictation, pairs and snippets by their spoken form; nothing is written
/// when neither was applied.
pub(crate) fn record_uses(app: &AppHandle, pairs: &[String], snippets: &[String]) {
    if pairs.is_empty() && snippets.is_empty() {
        return;
    }
    let mut settings = settings::get_settings(app);
    let pairs_changed = count_uses(&mut settings.dictionary_replacements, pairs);
    let snippets_changed = count_uses(&mut settings.dictionary_snippets, snippets);
    if pairs_changed || snippets_changed {
        settings::write_settings(app, settings);
    }
}

/// Stores the pairs and returns them as stored.
#[tauri::command]
#[specta::specta]
pub fn update_dictionary_replacements(
    app: AppHandle,
    replacements: Vec<DictionaryReplacement>,
) -> Result<Vec<DictionaryReplacement>, String> {
    let mut settings = settings::get_settings(&app);
    settings.dictionary_replacements = normalize(replacements, &settings.dictionary_replacements);
    let stored = settings.dictionary_replacements.clone();
    settings::write_settings(&app, settings);
    Ok(stored)
}

/// Stores the snippets and returns them as stored.
#[tauri::command]
#[specta::specta]
pub fn update_dictionary_snippets(
    app: AppHandle,
    snippets: Vec<DictionaryReplacement>,
) -> Result<Vec<DictionaryReplacement>, String> {
    let mut settings = settings::get_settings(&app);
    settings.dictionary_snippets = normalize_snippets(snippets, &settings.dictionary_snippets);
    let stored = settings.dictionary_snippets.clone();
    settings::write_settings(&app, settings);
    Ok(stored)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(spoken: &str, written: &str, uses: u32) -> DictionaryReplacement {
        DictionaryReplacement {
            spoken: spoken.to_string(),
            written: written.to_string(),
            uses,
        }
    }

    #[test]
    fn normalize_keeps_stored_uses() {
        let stored = [pair("fala sql", "Fala.sqlite", 7), pair("old", "gone", 2)];
        let incoming = vec![
            pair("  Fala   SQL ", " Fala.sqlite ", 0),
            pair("btw", "by the way", 99),
            pair("", "empty", 0),
            pair("blank", "  ", 0),
            pair("BTW", "repeat", 0),
        ];
        assert_eq!(
            normalize(incoming, &stored),
            [
                pair("Fala SQL", "Fala.sqlite", 7),
                pair("btw", "by the way", 0)
            ]
        );
    }

    #[test]
    fn count_uses_adds_one_per_occurrence() {
        let mut settings = settings::get_default_settings();
        settings.dictionary_replacements = vec![
            pair("fala sql", "Fala.sqlite", 1),
            pair("btw", "by the way", 0),
        ];
        let applied = ["btw", "fala sql", "BTW", "gone"].map(String::from);
        assert!(count_uses(&mut settings.dictionary_replacements, &applied));
        assert_eq!(
            settings.dictionary_replacements,
            [
                pair("fala sql", "Fala.sqlite", 2),
                pair("btw", "by the way", 2)
            ]
        );
        assert!(!count_uses(
            &mut settings.dictionary_replacements,
            &["gone".to_string()]
        ));
        assert!(!count_uses(&mut settings.dictionary_replacements, &[]));
    }

    #[test]
    fn dictionary_joins_terms_and_pairs() {
        let mut settings = settings::get_default_settings();
        settings.custom_words = vec!["Itaú".to_string()];
        settings.dictionary_replacements = vec![pair("fala sql", "Fala.sqlite", 3)];
        let dictionary = dictionary(&settings);
        assert_eq!(dictionary.terms(), ["Itaú", "Fala.sqlite"]);
        assert_eq!(dictionary.replacements().len(), 1);
        assert_eq!(dictionary.replacements()[0].spoken(), "fala sql");
    }

    #[test]
    fn snippets_need_two_words_and_keep_stored_uses() {
        let stored = [pair("minha assinatura", "Augusto", 4)];
        let incoming = vec![
            pair(" Minha  Assinatura ", "Augusto\nfala.dev\n", 0),
            pair("assinatura", "one word", 0),
            pair("link da agenda", "  ", 0),
            pair("MINHA ASSINATURA", "repeat", 0),
            pair("link da agenda", "https://cal.example", 9),
        ];
        assert_eq!(
            normalize_snippets(incoming, &stored),
            [
                pair("Minha Assinatura", "Augusto\nfala.dev", 4),
                pair("link da agenda", "https://cal.example", 0)
            ]
        );
    }

    #[test]
    fn dictionary_resolves_snippet_variables_and_keeps_them_off_the_terms() {
        let mut settings = settings::get_default_settings();
        settings.custom_words = vec!["Itaú".to_string()];
        settings.dictionary_snippets = vec![pair("hoje por extenso", "{data_extenso}, {hora}", 0)];
        let at = Moment {
            year: 2026,
            month: 10,
            day: 2,
            hour: 9,
            minute: 30,
        };
        let dictionary = dictionary_at(&settings, at);
        assert_eq!(dictionary.terms(), ["Itaú"]);
        assert_eq!(
            dictionary.snippets()[0].expansion(),
            "2 de outubro de 2026, 09:30"
        );
        let mut counted = settings.dictionary_snippets.clone();
        assert!(count_uses(&mut counted, &["Hoje por extenso".to_string()]));
        assert_eq!(counted[0].uses, 1);
    }

    #[test]
    fn stored_pair_round_trips_and_uses_defaults_to_zero() {
        let loaded: DictionaryReplacement =
            serde_json::from_str(r#"{"spoken":"btw","written":"by the way"}"#).unwrap();
        assert_eq!(loaded, pair("btw", "by the way", 0));
        assert_eq!(
            serde_json::to_value(pair("btw", "by the way", 4)).unwrap(),
            serde_json::json!({"spoken": "btw", "written": "by the way", "uses": 4})
        );
    }
}
