//! What the meeting notes know about the user (ADR-0016): the profile, the notes language and
//! the dictionary, from the settings.

use fala_core::{Dictionary, Language};

use crate::settings::{AppSettings, NotesLanguage};

/// What the notes take from the settings.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NotesSettings {
    pub language: Language,
    pub profile: String,
    pub dictionary: Dictionary,
}

pub(crate) fn notes_settings(settings: &AppSettings) -> NotesSettings {
    NotesSettings {
        language: notes_language(settings.notes_language, &settings.app_language),
        profile: settings.notes_profile.clone(),
        dictionary: Dictionary::new(settings.custom_words.clone()),
    }
}

/// pt-BR unless chosen otherwise; "same as the app" is English only for an English interface.
pub(crate) fn notes_language(setting: NotesLanguage, app_language: &str) -> Language {
    match setting {
        NotesLanguage::PtBr => Language::PtBr,
        NotesLanguage::En => Language::En,
        NotesLanguage::App if app_language.to_ascii_lowercase().starts_with("en") => Language::En,
        NotesLanguage::App => Language::PtBr,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_language_is_pt_br_unless_chosen() {
        let defaults = crate::settings::get_default_settings();
        assert_eq!(defaults.notes_language, NotesLanguage::PtBr);
        assert!(defaults.meeting_keyterms);
        assert_eq!(defaults.notes_profile, "");
        assert_eq!(notes_language(NotesLanguage::PtBr, "en-US"), Language::PtBr);
        assert_eq!(notes_language(NotesLanguage::En, "pt-BR"), Language::En);
        assert_eq!(notes_language(NotesLanguage::App, "en-US"), Language::En);
        assert_eq!(notes_language(NotesLanguage::App, "pt-BR"), Language::PtBr);
        assert_eq!(notes_language(NotesLanguage::App, "es"), Language::PtBr);

        let json = serde_json::to_value(NotesLanguage::PtBr).unwrap();
        assert_eq!(json, "pt-BR");
        let app: NotesLanguage = serde_json::from_value(serde_json::json!("app")).unwrap();
        assert_eq!(app, NotesLanguage::App);
    }

    #[test]
    fn notes_settings_carry_profile_and_dictionary() {
        let mut settings = crate::settings::get_default_settings();
        settings.notes_profile = "PM do Fala".to_string();
        settings.custom_words = vec!["Parakeet".to_string()];
        settings.notes_language = NotesLanguage::En;
        assert_eq!(
            notes_settings(&settings),
            NotesSettings {
                language: Language::En,
                profile: "PM do Fala".to_string(),
                dictionary: Dictionary::new(["Parakeet"]),
            }
        );
    }
}
