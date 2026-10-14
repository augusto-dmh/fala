//! Meetings of the same series: the title without dates or a final number (`fala_storage`'s
//! `series_of`). Nothing is stored; renaming a meeting moves it to another series at once.

use std::collections::{BTreeMap, HashMap};

use fala_meeting::SessionId;
use fala_storage::{series_of, Store};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::pipeline::{list_in, summary_in, MeetingSummary};
use super::{MeetingError, MeetingManager};
use crate::settings::SeriesTemplate;
use tauri::Emitter;

/// At most this many earlier meetings under the title; "see all" lists the rest.
const EARLIER_SHOWN: usize = 5;

/// The series of an opened session: "Mesma série:" under its title.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct MeetingSeries {
    pub key: String,
    /// The title without its date or number, as typed ("Weekly produto").
    pub name: String,
    /// The meetings that started before this one, latest first, at most [`EARLIER_SHOWN`].
    pub earlier: Vec<MeetingSummary>,
    /// Every meeting of the series outside the trash, this one included.
    pub total: u32,
    /// "Usar sempre em …": the template of its meetings without their own pick.
    pub template: Option<String>,
}

/// The template "Usar sempre" set for the series `key`, if any.
fn template_for(key: &str, templates: &BTreeMap<String, SeriesTemplate>) -> Option<String> {
    templates.get(key).map(|t| t.template.clone())
}

/// The template "Usar sempre" set for the series of `title`, if any.
pub(crate) fn series_template(
    title: &str,
    templates: &BTreeMap<String, SeriesTemplate>,
) -> Option<String> {
    template_for(&series_of(title)?.key, templates)
}

/// The sessions of the series `key`, newest first.
fn series_sessions(store: &Store, key: &str) -> Result<Vec<MeetingSummary>, MeetingError> {
    let rank: HashMap<String, usize> = store
        .series_meetings(key)?
        .iter()
        .enumerate()
        .map(|(at, id)| (id.to_string(), at))
        .collect();
    let mut list = list_in(store)?;
    list.retain(|session| rank.contains_key(&session.id));
    list.sort_by_key(|session| rank.get(&session.id).copied());
    Ok(list)
}

/// The series of the session `id` titled `title`; `None` without one, alone in it, or in the
/// trash. Reads only the titles and the few earlier sessions: it runs on every open.
pub(crate) fn series_in(
    store: &Store,
    id: SessionId,
    title: &str,
    templates: &BTreeMap<String, SeriesTemplate>,
) -> Result<Option<MeetingSeries>, MeetingError> {
    let Some(series) = series_of(title) else {
        return Ok(None);
    };
    let ids = store.series_meetings(&series.key)?;
    let Some(at) = ids.iter().position(|other| *other == id) else {
        return Ok(None);
    };
    if ids.len() < 2 {
        return Ok(None);
    }
    let earlier = ids[at + 1..]
        .iter()
        .take(EARLIER_SHOWN)
        .map(|older| summary_in(store, &store.meeting(*older)?))
        .collect::<Result<_, _>>()?;
    Ok(Some(MeetingSeries {
        template: template_for(&series.key, templates),
        key: series.key,
        name: series.name,
        earlier,
        total: u32::try_from(ids.len()).unwrap_or(u32::MAX),
    }))
}

impl MeetingManager {
    /// "Usar sempre em …": `template` for every meeting of the series `key` without its own
    /// pick; `None` forgets it.
    pub fn set_series_template(
        &self,
        key: String,
        name: String,
        template: Option<String>,
    ) -> Result<(), MeetingError> {
        if let Some(template) = &template {
            self.template(template)?;
        }
        let mut settings = crate::settings::get_settings(&self.app);
        match template {
            Some(template) => {
                settings
                    .meeting_series_templates
                    .insert(key, SeriesTemplate { name, template });
            }
            None => {
                settings.meeting_series_templates.remove(&key);
            }
        }
        let value = serde_json::to_value(&settings.meeting_series_templates).ok();
        crate::settings::write_settings(&self.app, settings);
        let _ = self.app.emit(
            "settings-changed",
            serde_json::json!({ "setting": "meeting_series_templates", "value": value }),
        );
        Ok(())
    }

    pub fn series_meetings(&self, key: &str) -> Result<Vec<MeetingSummary>, MeetingError> {
        self.with_store(|store| series_sessions(store, key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, TimeZone};
    use fala_meeting::SessionMode;
    use fala_storage::NewMeeting;

    fn meeting(store: &Store, day: u32, title: &str) -> SessionId {
        let id = SessionId::from_parts(1_727_000_000_000 + u64::from(day), u128::from(day));
        let at = FixedOffset::west_opt(3 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, 10, day, 10, 0, 0)
            .unwrap();
        store
            .create_meeting(&NewMeeting {
                id,
                title: title.to_string(),
                mode: SessionMode::Meeting,
                local_only: false,
                created_at: at,
            })
            .unwrap();
        store
            .start_meeting_recording(id, SessionMode::Meeting, at)
            .unwrap();
        id
    }

    #[test]
    fn earlier_meetings_of_the_series_latest_first() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("series.sqlite"), dir.path()).unwrap();
        let ids: Vec<SessionId> = (1..=8)
            .map(|day| meeting(&store, day, &format!("Weekly produto {day:02}/10")))
            .collect();
        let alone = meeting(&store, 9, "1:1 Ana");

        let none = BTreeMap::new();
        let series = series_in(&store, ids[6], "Weekly produto 07/10", &none)
            .unwrap()
            .unwrap();
        assert_eq!(series.name, "Weekly produto");
        assert_eq!(series.total, 8);
        let earlier: Vec<String> = series.earlier.iter().map(|s| s.id.clone()).collect();
        let want: Vec<String> = ids[1..6].iter().rev().map(ToString::to_string).collect();
        assert_eq!(earlier, want);

        let first = series_in(&store, ids[0], "Weekly produto 01/10", &none)
            .unwrap()
            .unwrap();
        assert!(first.earlier.is_empty());
        assert_eq!(series_in(&store, alone, "1:1 Ana", &none).unwrap(), None);
        assert_eq!(series_in(&store, alone, "Reunião", &none).unwrap(), None);
    }

    #[test]
    fn a_series_template_follows_the_title() {
        let mut templates = BTreeMap::new();
        templates.insert(
            "weekly produto".to_string(),
            SeriesTemplate {
                name: "Weekly produto".to_string(),
                template: "1-1".to_string(),
            },
        );
        assert_eq!(
            series_template("Weekly Produto 20/10", &templates).as_deref(),
            Some("1-1")
        );
        assert_eq!(series_template("Retro #3", &templates), None);
        assert_eq!(series_template("", &templates), None);
    }
}
