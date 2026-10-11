//! The personal dictionary as the app stores it: `custom_words` holds the terms (unchanged since
//! the Handy days) and `dictionary_replacements` the spoken -> written pairs, each with how many
//! times the rules applied it. `dictionary` joins both into the `fala_core::Dictionary` every
//! dictation uses; the written form of a pair reaches the LLM as a term, the spoken form never.

use fala_core::Dictionary;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;

use crate::settings::{self, AppSettings};

/// A spoken -> written pair as stored. `uses` counts every occurrence the rules replaced.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Type)]
pub struct DictionaryReplacement {
    pub spoken: String,
    pub written: String,
    #[serde(default)]
    pub uses: u32,
}

/// The dictionary for one dictation: the terms, then the pairs.
pub(crate) fn dictionary(settings: &AppSettings) -> Dictionary {
    Dictionary::new(&settings.custom_words).with_replacements(
        settings
            .dictionary_replacements
            .iter()
            .map(|pair| (pair.spoken.as_str(), pair.written.as_str())),
    )
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
    dictionary
        .replacements()
        .iter()
        .map(|pair| {
            let key = spoken_key(pair.spoken());
            let uses = stored
                .iter()
                .find(|known| spoken_key(&known.spoken) == key)
                .map_or(0, |known| known.uses);
            DictionaryReplacement {
                spoken: pair.spoken().to_string(),
                written: pair.written().to_string(),
                uses,
            }
        })
        .collect()
}

/// Adds one use per applied occurrence. A spoken form no longer stored is ignored. Returns
/// whether anything changed.
pub(crate) fn count_uses(settings: &mut AppSettings, applied: &[String]) -> bool {
    let mut changed = false;
    for spoken in applied {
        let key = spoken_key(spoken);
        if let Some(pair) = settings
            .dictionary_replacements
            .iter_mut()
            .find(|pair| spoken_key(&pair.spoken) == key)
        {
            pair.uses = pair.uses.saturating_add(1);
            changed = true;
        }
    }
    changed
}

/// Stores the uses of one dictation; nothing is written when no pair was applied.
pub(crate) fn record_uses(app: &AppHandle, applied: &[String]) {
    if applied.is_empty() {
        return;
    }
    let mut settings = settings::get_settings(app);
    if count_uses(&mut settings, applied) {
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
        assert!(count_uses(&mut settings, &applied));
        assert_eq!(
            settings.dictionary_replacements,
            [
                pair("fala sql", "Fala.sqlite", 2),
                pair("btw", "by the way", 2)
            ]
        );
        assert!(!count_uses(&mut settings, &["gone".to_string()]));
        assert!(!count_uses(&mut settings, &[]));
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
