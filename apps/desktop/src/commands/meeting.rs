//! Meeting recording commands (F2 `meeting-panel`); the logic is in `crate::meeting`.

use std::sync::Arc;

use tauri::State;

use crate::meeting::pipeline::{
    MeetingDetail, MeetingKeys, MeetingLine, MeetingSummary, MeetingTemplate,
};
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

#[tauri::command]
#[specta::specta]
pub async fn list_meetings(meetings: Meetings<'_>) -> Result<Vec<MeetingSummary>, MeetingError> {
    meetings.list()
}

#[tauri::command]
#[specta::specta]
pub async fn get_meeting(
    meetings: Meetings<'_>,
    id: String,
) -> Result<MeetingDetail, MeetingError> {
    meetings.detail(&id)
}

/// Sends the retained audio of a stopped session to ElevenLabs Scribe (ADR-0005); progress
/// comes as `MeetingProgress` events.
#[tauri::command]
#[specta::specta]
pub async fn transcribe_meeting(
    meetings: Meetings<'_>,
    id: String,
) -> Result<Vec<MeetingLine>, MeetingError> {
    let manager = meetings.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.transcribe(&id))
        .await
        .map_err(|e| MeetingError::Transcription(e.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_meeting_transcription(meetings: Meetings<'_>) -> Result<(), MeetingError> {
    meetings.cancel_transcription();
    Ok(())
}

/// Sends the enumerated payload of ADR-0016 to the notes LLM and stores the Markdown.
#[tauri::command]
#[specta::specta]
pub async fn generate_meeting_notes(
    meetings: Meetings<'_>,
    id: String,
    template_id: String,
) -> Result<String, MeetingError> {
    let manager = meetings.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.generate_notes(&id, &template_id))
        .await
        .map_err(|e| MeetingError::Notes(e.to_string()))?
}

/// The note document "Copy Markdown" puts on the clipboard.
#[tauri::command]
#[specta::specta]
pub async fn meeting_markdown(meetings: Meetings<'_>, id: String) -> Result<String, MeetingError> {
    meetings.markdown(&id)
}

#[tauri::command]
#[specta::specta]
pub async fn meeting_templates() -> Result<Vec<MeetingTemplate>, MeetingError> {
    Ok(crate::meeting::pipeline::templates())
}

#[tauri::command]
#[specta::specta]
pub async fn meeting_keys(meetings: Meetings<'_>) -> Result<MeetingKeys, MeetingError> {
    Ok(meetings.keys())
}

/// Stores the ElevenLabs key in the OS keyring (ADR-0008); it never comes back to the UI.
#[tauri::command]
#[specta::specta]
pub async fn set_meeting_transcription_key(
    meetings: Meetings<'_>,
    key: String,
) -> Result<MeetingKeys, MeetingError> {
    meetings.set_transcription_key(&key)
}
