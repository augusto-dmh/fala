//! Search on the Meetings page: titles, typed notes, generated notes and transcripts, without
//! accents (`fala-storage`'s `meetings_fts`).

use serde::{Deserialize, Serialize};
use specta::Type;

use super::{MeetingError, MeetingManager};

/// How many sessions a search returns.
const SEARCH_LIMIT: usize = 50;

/// A session the search found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct MeetingSearchHit {
    pub id: String,
    /// The matching excerpt; each found term sits between `\u{2}` and `\u{3}`.
    pub snippet: String,
}

impl MeetingManager {
    /// Newest first; an empty query finds nothing.
    pub fn search(&self, query: &str) -> Result<Vec<MeetingSearchHit>, MeetingError> {
        let hits = self.with_store(|store| Ok(store.search_meetings(query, SEARCH_LIMIT)?))?;
        Ok(hits
            .into_iter()
            .map(|hit| MeetingSearchHit {
                id: hit.id.to_string(),
                snippet: hit.snippet,
            })
            .collect())
    }
}
