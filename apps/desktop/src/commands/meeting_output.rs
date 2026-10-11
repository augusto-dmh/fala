//! "Meeting sound": which output a meeting records, and the switch while recording.

use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::meeting::output::{self, MeetingOutputs};
use crate::meeting::{MeetingError, MeetingManager, MeetingStatus};
use crate::settings::{get_settings, write_settings};

/// The outputs and the remembered choice. Enumerating can stall, so it runs off the main loop.
#[tauri::command]
#[specta::specta]
pub async fn meeting_outputs(app: AppHandle) -> Result<MeetingOutputs, MeetingError> {
    let selected = get_settings(&app).meeting_output_device;
    tauri::async_runtime::spawn_blocking(move || output::outputs(selected))
        .await
        .map_err(|e| MeetingError::AudioDevice(e.to_string()))?
}

/// Remembers the output the next meetings record; `None` follows the default output.
#[tauri::command]
#[specta::specta]
pub fn set_meeting_output(app: AppHandle, name: Option<String>) -> Result<(), MeetingError> {
    let mut settings = get_settings(&app);
    settings.meeting_output_device = name;
    write_settings(&app, settings);
    Ok(())
}

/// "Record from this device" while a meeting records.
#[tauri::command]
#[specta::specta]
pub fn switch_meeting_output(
    meetings: State<'_, Arc<MeetingManager>>,
    name: String,
) -> Result<MeetingStatus, MeetingError> {
    meetings.switch_output(&name)
}
