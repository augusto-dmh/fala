//! Who was in a meeting (ADR-0016): the names typed in "Who was there" and the name the notes
//! LLM suggests for each "Person N", applied only by a click.

use std::collections::HashMap;

use fala_meeting::SessionId;
use fala_storage::Store;
use serde::{Deserialize, Serialize};
use specta::Type;

use super::{parse_id, MeetingError, MeetingManager};

/// At most this many names in "Who was there", each at most [`NAME_MAX_CHARS`] characters.
const MAX_PARTICIPANTS: usize = 30;
const NAME_MAX_CHARS: usize = 80;

/// "Who was there" of a session and the name suggestions waiting for a click.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct MeetingPeople {
    pub participants: Vec<String>,
    pub suggestions: Vec<SpeakerSuggestion>,
}

/// "Person `person`" may be `name`, because `reason`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct SpeakerSuggestion {
    pub person: u32,
    pub name: String,
    pub reason: String,
}

/// The names applied to "Person N" in the session.
pub(crate) fn speaker_names(
    store: &Store,
    id: SessionId,
) -> Result<HashMap<u32, String>, MeetingError> {
    Ok(store
        .meeting_speakers(id)?
        .into_iter()
        .filter_map(|speaker| speaker.name.map(|name| (speaker.person, name)))
        .collect())
}

/// The typed names, trimmed, without blanks, capped in count and length. E-mails are stored as
/// typed; `fala-notes` keeps them out of the payload.
fn clean_participants(names: Vec<String>) -> Vec<String> {
    names
        .into_iter()
        .map(|name| {
            let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
            name.chars().take(NAME_MAX_CHARS).collect::<String>()
        })
        .filter(|name| !name.is_empty())
        .take(MAX_PARTICIPANTS)
        .collect()
}

fn people(store: &Store, id: SessionId) -> Result<MeetingPeople, MeetingError> {
    Ok(MeetingPeople {
        participants: store.meeting_participants(id)?,
        suggestions: store
            .meeting_speakers(id)?
            .into_iter()
            .filter_map(|speaker| {
                speaker.suggestion.map(|s| SpeakerSuggestion {
                    person: speaker.person,
                    name: s.name,
                    reason: s.reason,
                })
            })
            .collect(),
    })
}

impl MeetingManager {
    pub fn people(&self, id: &str) -> Result<MeetingPeople, MeetingError> {
        let id = parse_id(id)?;
        self.with_store(|store| people(store, id))
    }

    pub fn set_participants(
        &self,
        id: &str,
        names: Vec<String>,
    ) -> Result<MeetingPeople, MeetingError> {
        let id = parse_id(id)?;
        let names = clean_participants(names);
        self.with_store(|store| {
            store.set_meeting_participants(id, &names)?;
            people(store, id)
        })
    }

    /// The click on "apply": "Person N" takes the suggested name in the transcript and in the
    /// next notes generation.
    pub fn apply_speaker_suggestion(
        &self,
        id: &str,
        person: u32,
    ) -> Result<MeetingPeople, MeetingError> {
        let id = parse_id(id)?;
        self.with_store(|store| {
            let name = store.apply_speaker_suggestion(id, person)?;
            log::debug!("meeting {id}: person {person} named {name}");
            people(store, id)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn participants_are_cleaned() {
        let mut names: Vec<String> = vec![
            "  Ana   Souza ".to_string(),
            " ".to_string(),
            "x".repeat(200),
        ];
        names.extend((0..40).map(|n| format!("P{n}")));
        let clean = clean_participants(names);
        assert_eq!(clean[0], "Ana Souza");
        assert_eq!(clean[1].chars().count(), NAME_MAX_CHARS);
        assert_eq!(clean.len(), MAX_PARTICIPANTS);
    }
}
