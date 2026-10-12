//! Meeting recording commands (F2 `meeting-panel`); the logic is in `crate::meeting`.

use std::sync::Arc;

use tauri::ipc::Response;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::meeting::granola::{GranolaImportError, GranolaImported, GranolaPreview};
use crate::meeting::import::ImportSource;
use crate::meeting::people::MeetingPeople;
use crate::meeting::pipeline::{
    templates, templates_dir, MeetingDetail, MeetingKeys, MeetingLine, MeetingSummary,
    MeetingTemplates,
};
use crate::meeting::recovery::MeetingOrphan;
use crate::meeting::{MeetingError, MeetingManager, MeetingMode, MeetingStatus};
use crate::settings::{self, NotesLanguage};

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

/// "Import": a file or the audio of a URL becomes a new session; returns its id once the audio
/// is retained. Progress comes as `MeetingProgress` events.
#[tauri::command]
#[specta::specta]
pub async fn import_meeting(
    meetings: Meetings<'_>,
    source: ImportSource,
) -> Result<String, MeetingError> {
    let manager = meetings.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.import(source))
        .await
        .map_err(|e| MeetingError::Import(e.to_string()))?
}

/// "Granola history": reads and checks the export at `path` (a CSV or a `.zip`); saves nothing.
#[tauri::command]
#[specta::specta]
pub async fn preview_granola_import(
    meetings: Meetings<'_>,
    path: String,
) -> Result<GranolaPreview, GranolaImportError> {
    let manager = meetings.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.preview_granola(&path))
        .await
        .map_err(|e| GranolaImportError::Unreadable(e.to_string()))?
}

/// Imports the export at `path`, one meeting at a time; `cancel_meeting_import` stops it.
/// Progress comes as `GranolaProgress` events.
#[tauri::command]
#[specta::specta]
pub async fn run_granola_import(
    meetings: Meetings<'_>,
    path: String,
) -> Result<GranolaImported, GranolaImportError> {
    let manager = meetings.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.import_granola(&path))
        .await
        .map_err(|e| GranolaImportError::Storage(e.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_meeting_import(meetings: Meetings<'_>) -> Result<(), MeetingError> {
    meetings.cancel_import();
    Ok(())
}

/// Sends the enumerated payload of ADR-0016 to the notes LLM and stores the Markdown. Notes
/// edited by hand, regenerated with the same template, return `notes_edited` unless
/// `replace_edited` (the confirmation click).
#[tauri::command]
#[specta::specta]
pub async fn generate_meeting_notes(
    meetings: Meetings<'_>,
    id: String,
    template_id: String,
    replace_edited: bool,
) -> Result<String, MeetingError> {
    let manager = meetings.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        manager.generate_notes(&id, &template_id, replace_edited)
    })
    .await
    .map_err(|e| MeetingError::Notes(e.to_string()))?
}

/// The retained audio around a transcript line, as WAV bytes in a binary IPC response (no
/// temporary file, no asset protocol). Registered outside tauri-specta, which has no TypeScript
/// type for `Response`: see `lib.rs` and `src/components/meeting/clipPlayer.ts`.
#[tauri::command]
pub async fn meeting_clip(
    meetings: Meetings<'_>,
    id: String,
    t0_ms: u64,
    t1_ms: u64,
) -> Result<Response, MeetingError> {
    let manager = meetings.inner().clone();
    let wav = tauri::async_runtime::spawn_blocking(move || manager.clip(&id, t0_ms, t1_ms))
        .await
        .map_err(|e| MeetingError::Storage(e.to_string()))??;
    Ok(Response::new(wav))
}

/// The note document "Copy Markdown" puts on the clipboard.
#[tauri::command]
#[specta::specta]
pub async fn meeting_markdown(meetings: Meetings<'_>, id: String) -> Result<String, MeetingError> {
    meetings.markdown(&id)
}

#[tauri::command]
#[specta::specta]
pub async fn meeting_templates(app: AppHandle) -> Result<MeetingTemplates, MeetingError> {
    Ok(templates(&templates_dir(&app)?))
}

/// "Duplicate to edit": copies a built-in template to the templates folder with a new id and
/// name, opens the folder and returns the new list.
#[tauri::command]
#[specta::specta]
pub async fn duplicate_meeting_template(
    app: AppHandle,
    id: String,
) -> Result<MeetingTemplates, MeetingError> {
    let dir = templates_dir(&app)?;
    fala_notes::duplicate_builtin(&dir, &id).map_err(|e| MeetingError::Storage(e.to_string()))?;
    // The copy is written: a folder that does not open is not a failed duplicate.
    if let Err(e) = open_folder(&app, &dir) {
        log::warn!("templates folder did not open: {e:?}");
    }
    Ok(templates(&dir))
}

/// "Open the folder": created with its `LEIAME.txt` on the first click.
#[tauri::command]
#[specta::specta]
pub async fn open_meeting_templates_folder(app: AppHandle) -> Result<(), MeetingError> {
    let dir = templates_dir(&app)?;
    fala_notes::ensure_templates_dir(&dir).map_err(|e| MeetingError::Storage(e.to_string()))?;
    open_folder(&app, &dir)
}

fn open_folder(app: &AppHandle, dir: &std::path::Path) -> Result<(), MeetingError> {
    app.opener()
        .open_path(dir.to_string_lossy(), None::<String>)
        .map_err(|e| MeetingError::Storage(e.to_string()))
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

/// The profile the notes LLM receives (ADR-0016), cut to `fala_notes::PROFILE_MAX_CHARS`.
#[tauri::command]
#[specta::specta]
pub fn change_notes_profile_setting(app: AppHandle, profile: String) -> Result<String, String> {
    let profile: String = profile
        .trim()
        .chars()
        .take(fala_notes::PROFILE_MAX_CHARS)
        .collect();
    let mut settings = settings::get_settings(&app);
    settings.notes_profile = profile.clone();
    settings::write_settings(&app, settings);
    Ok(profile)
}

#[tauri::command]
#[specta::specta]
pub fn change_notes_language_setting(
    app: AppHandle,
    language: NotesLanguage,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.notes_language = language;
    settings::write_settings(&app, settings);
    Ok(())
}

/// "Generate notes when it ends": the notes follow the transcript of a stop.
#[tauri::command]
#[specta::specta]
pub fn change_meeting_auto_notes_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.meeting_auto_notes = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

/// The dictionary as Scribe `keyterms` (ADR-0005).
#[tauri::command]
#[specta::specta]
pub fn change_meeting_keyterms_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.meeting_keyterms = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

/// "Who was there" of a session and the name suggestions waiting for "apply".
#[tauri::command]
#[specta::specta]
pub async fn meeting_people(
    meetings: Meetings<'_>,
    id: String,
) -> Result<MeetingPeople, MeetingError> {
    meetings.people(&id)
}

/// Names only: `fala-notes` keeps any entry with `@` out of the notes payload (ADR-0016).
#[tauri::command]
#[specta::specta]
pub async fn set_meeting_participants(
    meetings: Meetings<'_>,
    id: String,
    names: Vec<String>,
) -> Result<MeetingPeople, MeetingError> {
    meetings.set_participants(&id, names)
}

/// The click that applies a suggested name to "Person N"; never called by the app itself.
#[tauri::command]
#[specta::specta]
pub async fn apply_speaker_suggestion(
    meetings: Meetings<'_>,
    id: String,
    person: u32,
) -> Result<MeetingPeople, MeetingError> {
    meetings.apply_speaker_suggestion(&id, person)
}

/// The name typed for "Person N"; blank gives the label back.
#[tauri::command]
#[specta::specta]
pub async fn set_meeting_speaker_name(
    meetings: Meetings<'_>,
    id: String,
    person: u32,
    name: String,
) -> Result<MeetingPeople, MeetingError> {
    meetings.set_speaker_name(&id, person, &name)
}

/// The notes as edited in the page.
#[tauri::command]
#[specta::specta]
pub async fn save_meeting_notes(
    meetings: Meetings<'_>,
    id: String,
    markdown: String,
) -> Result<(), MeetingError> {
    meetings.save_edited_notes(&id, &markdown)
}

/// The template for the next notes, picked before or during the recording.
#[tauri::command]
#[specta::specta]
pub async fn set_meeting_template(
    meetings: Meetings<'_>,
    id: String,
    template_id: String,
) -> Result<(), MeetingError> {
    meetings.set_template(&id, &template_id)
}

/// Recordings the last run left without an end: "Recover the recording from 14:02 (12 min)?".
#[tauri::command]
#[specta::specta]
pub async fn meeting_orphans(meetings: Meetings<'_>) -> Result<Vec<MeetingOrphan>, MeetingError> {
    let manager = meetings.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.orphans())
        .await
        .map_err(|e| MeetingError::Storage(e.to_string()))?
}

/// The click on "Recover"; retention and transcription follow in the background.
#[tauri::command]
#[specta::specta]
pub async fn recover_meeting(meetings: Meetings<'_>, id: String) -> Result<(), MeetingError> {
    let manager = meetings.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.recover(&id))
        .await
        .map_err(|e| MeetingError::Storage(e.to_string()))?
}

/// "Delete meeting" after its confirmation: audio, transcript and notes, for good (ADR-0005).
#[tauri::command]
#[specta::specta]
pub async fn delete_meeting(meetings: Meetings<'_>, id: String) -> Result<(), MeetingError> {
    let manager = meetings.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.delete(&id))
        .await
        .map_err(|e| MeetingError::Storage(e.to_string()))?
}

/// "Meeting audio": keep (`None`) or delete N days after the end; applied at once.
#[tauri::command]
#[specta::specta]
pub async fn change_meeting_retention_setting(
    app: AppHandle,
    meetings: Meetings<'_>,
    days: Option<u16>,
) -> Result<(), String> {
    if let Some(days) = days {
        fala_retention::RetentionDays::new(days).map_err(|e| e.to_string())?;
    }
    let mut settings = settings::get_settings(&app);
    settings.meeting_audio_retention_days = days;
    settings::write_settings(&app, settings);
    let manager = meetings.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.run_retention())
        .await
        .map_err(|e| e.to_string())
}

/// The recording cap, 1 to 8 hours; the next recording uses it.
#[tauri::command]
#[specta::specta]
pub fn change_meeting_cap_setting(app: AppHandle, hours: u8) -> Result<(), String> {
    fala_meeting::RecordingCap::from_hours(hours).map_err(|e| e.to_string())?;
    let mut settings = settings::get_settings(&app);
    settings.meeting_cap_hours = hours;
    settings::write_settings(&app, settings);
    Ok(())
}
