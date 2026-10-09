//! Meeting recording commands (F2 `meeting-panel`); the logic is in `crate::meeting`.

use std::sync::Arc;

use tauri::State;

use crate::meeting::{MeetingError, MeetingManager, MeetingMode, MeetingStatus};

type Meetings<'a> = State<'a, Arc<MeetingManager>>;

#[tauri::command]
#[specta::specta]
pub async fn meeting_status(meetings: Meetings<'_>) -> Result<MeetingStatus, MeetingError> {
    Ok(meetings.status())
}

/// The third-party notice was accepted; returns the stored date.
#[tauri::command]
#[specta::specta]
pub async fn accept_meeting_consent(meetings: Meetings<'_>) -> Result<String, MeetingError> {
    Ok(meetings.accept_consent())
}

#[tauri::command]
#[specta::specta]
pub async fn create_meeting_draft(
    meetings: Meetings<'_>,
    title: String,
) -> Result<String, MeetingError> {
    meetings.create_draft(&title)
}

#[tauri::command]
#[specta::specta]
pub async fn set_meeting_title(
    meetings: Meetings<'_>,
    id: String,
    title: String,
) -> Result<(), MeetingError> {
    meetings.set_title(&id, &title)
}

#[tauri::command]
#[specta::specta]
pub async fn save_meeting_annotations(
    meetings: Meetings<'_>,
    id: String,
    text: String,
) -> Result<(), MeetingError> {
    meetings.save_annotations(&id, &text)
}

/// "Record meeting": an explicit click, never called by the app itself (ADR-0005).
#[tauri::command]
#[specta::specta]
pub async fn start_meeting(
    meetings: Meetings<'_>,
    mode: MeetingMode,
    title: String,
    draft_id: Option<String>,
) -> Result<MeetingStatus, MeetingError> {
    meetings.start(mode, &title, draft_id.as_deref())
}

#[tauri::command]
#[specta::specta]
pub async fn pause_meeting(meetings: Meetings<'_>) -> Result<MeetingStatus, MeetingError> {
    meetings.pause()
}

#[tauri::command]
#[specta::specta]
pub async fn resume_meeting(meetings: Meetings<'_>) -> Result<MeetingStatus, MeetingError> {
    meetings.resume()
}

#[tauri::command]
#[specta::specta]
pub async fn stop_meeting(meetings: Meetings<'_>) -> Result<MeetingStatus, MeetingError> {
    meetings.stop()
}

#[tauri::command]
#[specta::specta]
pub async fn extend_meeting_cap(meetings: Meetings<'_>) -> Result<MeetingStatus, MeetingError> {
    meetings.extend_cap()
}
