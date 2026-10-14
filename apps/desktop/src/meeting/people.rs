//! Who was in a meeting (ADR-0016): the names typed in "Who was there" and the name the notes
//! LLM suggests for each "Person N", applied only by a click.

use std::collections::{HashMap, HashSet};

use fala_meeting::SessionId;
use fala_storage::{DirectoryPerson, SegmentSpeaker, Store};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::pipeline::{list_in, rfc3339, MeetingSummary};
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

/// A person of the People list: the names from "Who was there" and from "Person N", joined by
/// [`fala_storage::person_key`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct MeetingPerson {
    pub key: String,
    /// The spelling of the latest meeting.
    pub name: String,
    /// RFC 3339: when the latest meeting with them started.
    pub last_meeting_at: String,
    pub meetings: u32,
}

impl From<DirectoryPerson> for MeetingPerson {
    fn from(person: DirectoryPerson) -> Self {
        Self {
            key: person.key,
            name: person.name,
            last_meeting_at: rfc3339(&person.last_meeting_at),
            meetings: person.meetings,
        }
    }
}

/// The sessions of the list where the person of `key` is, newest first.
fn person_meetings(store: &Store, key: &str) -> Result<Vec<MeetingSummary>, MeetingError> {
    let ids: HashSet<String> = store
        .person_meeting_ids(key)?
        .iter()
        .map(ToString::to_string)
        .collect();
    Ok(list_in(store)?
        .into_iter()
        .filter(|session| ids.contains(&session.id))
        .collect())
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

/// A typed name with its spaces collapsed, cut to [`NAME_MAX_CHARS`]; `None` when blank.
fn clean_name(name: &str) -> Option<String> {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let name: String = name.chars().take(NAME_MAX_CHARS).collect();
    let name = name.trim_end().to_string();
    (!name.is_empty()).then_some(name)
}

/// The typed names, trimmed, without blanks, capped in count and length. E-mails are stored as
/// typed; `fala-notes` keeps them out of the payload.
fn clean_participants(names: Vec<String>) -> Vec<String> {
    names
        .iter()
        .filter_map(|name| clean_name(name))
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
    /// The People list: everyone in a meeting outside the trash, latest meeting first.
    pub fn directory(&self) -> Result<Vec<MeetingPerson>, MeetingError> {
        self.with_store(|store| {
            Ok(store
                .meeting_directory()?
                .into_iter()
                .map(MeetingPerson::from)
                .collect())
        })
    }

    pub fn person_meetings(&self, key: &str) -> Result<Vec<MeetingSummary>, MeetingError> {
        self.with_store(|store| person_meetings(store, key))
    }

    /// "Rename" and "Merge with…": every name of `key`, in "Who was there" and in "Person N",
    /// becomes `name` in all meetings; gives the People list back.
    pub fn rename_person(&self, key: &str, name: &str) -> Result<Vec<MeetingPerson>, MeetingError> {
        // The page never sends a blank name, nor one that is only punctuation.
        let Some(name) = clean_name(name).filter(|n| fala_storage::person_key(n).is_some()) else {
            return Err(MeetingError::NotFound);
        };
        self.with_store(|store| {
            let changed = store.rename_person(key, &name)?;
            log::debug!("person renamed in {} meetings", changed.len());
            Ok(store
                .meeting_directory()?
                .into_iter()
                .map(MeetingPerson::from)
                .collect())
        })
    }

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

    /// The name typed for "Person N" in the transcript; a blank one gives "Person N" back. It
    /// holds for the whole session: transcript, next notes and Markdown.
    pub fn set_speaker_name(
        &self,
        id: &str,
        person: u32,
        name: &str,
    ) -> Result<MeetingPeople, MeetingError> {
        let id = parse_id(id)?;
        let name = clean_name(name);
        self.with_store(|store| {
            let speaks = store
                .meeting_segments(id)?
                .iter()
                .any(|segment| segment.speaker == SegmentSpeaker::Person(person));
            if !speaks {
                return Err(MeetingError::NotFound);
            }
            store.set_meeting_speaker_name(id, person, name.as_deref())?;
            log::debug!("meeting {id}: person {person} named {name:?}");
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

    #[test]
    fn blank_name_gives_the_label_back() {
        assert_eq!(
            clean_name(
                "  Bruno 
 Lima "
            ),
            Some("Bruno Lima".to_string())
        );
        assert_eq!(clean_name(" 	 "), None);
        let long = format!("{} x", "a".repeat(NAME_MAX_CHARS - 1));
        assert_eq!(
            clean_name(&long).map(|n| n.chars().count()),
            Some(NAME_MAX_CHARS - 1)
        );
    }
}
