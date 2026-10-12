//! "Import" on the Meetings page (F7): a local audio or video file, or the audio of a URL,
//! becomes a session in the `import` mode with the same retention, transcription and notes as a
//! recording. `ffmpeg`, `ffprobe` and `yt-dlp` come from the PATH (`fala-media`, decision D3 of
//! the phase 2 pitch); nothing records, so there is no indicator to show (ADR-0005).

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, FixedOffset};
use fala_media::{CancelToken, MediaError, ProcessHooks, Tools};
use fala_meeting::{SessionId, SessionMode, StopReason};
use fala_storage::{NewMeeting, Store};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Manager;

use super::pipeline::ProgressStage;
use super::{millis, now_local, MeetingError, MeetingManager, WORK_WAV};

/// Where the downloaded source of a URL waits for the conversion, inside the session folder.
const DOWNLOAD_DIR: &str = "download";
/// Least time between two download progress events.
const PROGRESS_EVERY: Duration = Duration::from_millis(250);

/// What "Import" received: a path from the file dialog or a typed `http(s)://` URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ImportSource {
    File(String),
    Url(String),
}

/// The session title of an imported file: its name without the extension.
pub(crate) fn file_title(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The last non-empty line of a tool's message: what the page shows (the log has it all).
fn last_line(text: &str) -> String {
    text.lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn media_error(error: MediaError) -> MeetingError {
    log::warn!("meeting import: {error}");
    match error {
        MediaError::ToolNotFound { tool } => MeetingError::ToolMissing(tool.to_string()),
        MediaError::InvalidUrl(_) => MeetingError::InvalidUrl,
        MediaError::Download { detail, .. } => MeetingError::Download(last_line(&detail)),
        MediaError::Cancelled => MeetingError::Cancelled,
        MediaError::NoAudioTrack(_) => MeetingError::ImportNoAudio,
        MediaError::TooLong { .. } => MeetingError::ImportTooLong,
        // The tools' own words and paths, never this crate's Portuguese messages.
        MediaError::Unreadable { detail, .. } | MediaError::Ffmpeg { detail, .. } => {
            MeetingError::Import(last_line(&detail))
        }
        MediaError::InputNotFound(path) | MediaError::OutputExists(path) => {
            MeetingError::Import(path.display().to_string())
        }
        MediaError::Io(e) => MeetingError::Import(e.to_string()),
    }
}

/// The imported session as a finished recording: started and ended at the import, the source
/// duration as the recorded time, and the URL, if any, in the typed notes.
pub(crate) fn save_import(
    store: &Store,
    id: SessionId,
    title: &str,
    url: Option<&str>,
    span: (DateTime<FixedOffset>, DateTime<FixedOffset>),
    recorded_ms: u64,
) -> Result<(), MeetingError> {
    let (started, ended) = span;
    store.create_meeting(&NewMeeting {
        id,
        title: title.to_string(),
        mode: SessionMode::Import,
        local_only: false,
        created_at: started,
    })?;
    store.start_meeting_recording(id, SessionMode::Import, started)?;
    store.finish_meeting(id, ended, recorded_ms, StopReason::User)?;
    if let Some(url) = url {
        store.save_meeting_annotations(id, url)?;
    }
    Ok(())
}

/// On Windows the tools run without a console window, and a cancel kills the whole process
/// tree: `yt-dlp.exe` is a bootloader that starts a second process, which outlives a plain kill.
#[cfg(windows)]
fn hooks() -> ProcessHooks {
    use std::os::windows::process::CommandExt;
    use std::process::{Child, Command};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    fn prepare(command: &mut Command) {
        command.creation_flags(CREATE_NO_WINDOW);
    }

    fn stop(child: &mut Child) {
        let system = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        let mut taskkill = Command::new(Path::new(&system).join("System32").join("taskkill.exe"));
        taskkill.args(["/T", "/F", "/PID", &child.id().to_string()]);
        prepare(&mut taskkill);
        if !taskkill.output().is_ok_and(|out| out.status.success()) {
            let _ = child.kill();
        }
    }

    ProcessHooks { prepare, stop }
}

#[cfg(not(windows))]
fn hooks() -> ProcessHooks {
    ProcessHooks::default()
}

impl MeetingManager {
    /// Imports `source` into a new session and returns its id once the audio is retained; the
    /// transcript follows on its own when the ElevenLabs key is in the keyring, as after a stop.
    /// One import at a time; progress comes as `MeetingProgress` events.
    pub fn import(&self, source: ImportSource) -> Result<String, MeetingError> {
        if let ImportSource::Url(url) = &source {
            fala_media::check_url(url).map_err(media_error)?;
        }
        let tools = Tools::locate().map_err(media_error)?.with_hooks(hooks());
        let cancel = CancelToken::new();
        {
            let mut importing = self.importing.lock().unwrap_or_else(|e| e.into_inner());
            if importing.is_some() {
                return Err(MeetingError::Busy);
            }
            *importing = Some(cancel.clone());
        }
        let started = now_local();
        let id = SessionId::generate(millis(started).0);
        let result = id
            .map_err(|e| MeetingError::Storage(e.to_string()))
            .and_then(|id| {
                let imported = self.run_import(id, &source, &tools, &cancel, started);
                if let Err(error) = &imported {
                    // The row is the last write, so a failed import leaves only the folder, which
                    // goes. A failure inside `save_import` itself can still leave a partial row.
                    let _ = std::fs::remove_dir_all(self.session_dir(id));
                    self.progress(
                        id,
                        ProgressStage::Failed {
                            error: error.clone(),
                        },
                    );
                }
                imported.map(|()| id)
            });
        *self.importing.lock().unwrap_or_else(|e| e.into_inner()) = None;
        let id = result?;
        log::info!("meeting {id}: imported");
        self.progress(id, ProgressStage::Done);
        let app = self.app.clone();
        let spawned = std::thread::Builder::new()
            .name("meeting-import-transcribe".to_string())
            .spawn(move || {
                if let Some(manager) = app.try_state::<Arc<MeetingManager>>() {
                    manager.transcribe_if_keyed(id);
                }
            });
        if let Err(e) = spawned {
            log::warn!("meeting {id}: transcription thread did not start: {e}");
        }
        Ok(id.to_string())
    }

    /// Cancels the import in flight, if any: the tool is stopped and the session discarded.
    pub fn cancel_import(&self) {
        if let Some(cancel) = &*self.importing.lock().unwrap_or_else(|e| e.into_inner()) {
            log::info!("meeting import cancelled");
            cancel.cancel();
        }
    }

    fn run_import(
        &self,
        id: SessionId,
        source: &ImportSource,
        tools: &Tools,
        cancel: &CancelToken,
        started: DateTime<FixedOffset>,
    ) -> Result<(), MeetingError> {
        let dir = self.session_dir(id);
        std::fs::create_dir_all(&dir)
            .map_err(|e| MeetingError::Storage(format!("{}: {e}", dir.display())))?;
        let stage = |phase: &str, done: u64, total: u64| ProgressStage::Importing {
            phase: phase.to_string(),
            done,
            total,
        };
        let (input, title, url) = match source {
            ImportSource::File(path) => {
                let path = Path::new(path).to_path_buf();
                let title = file_title(&path);
                (path, title, None)
            }
            ImportSource::Url(url) => {
                // yt-dlp reports every chunk; the page needs a few updates a second.
                let mut last: Option<Instant> = None;
                let downloaded =
                    fala_media::download(tools, url, &dir.join(DOWNLOAD_DIR), cancel, &mut |p| {
                        if last.is_some_and(|at| at.elapsed() < PROGRESS_EVERY) {
                            return;
                        }
                        last = Some(Instant::now());
                        let total = p.total_bytes.unwrap_or(0);
                        self.progress(id, stage("download", p.done_bytes, total));
                    })
                    .map_err(media_error)?;
                (downloaded.path, downloaded.title, Some(url.as_str()))
            }
        };
        let ms = |d: Duration| u64::try_from(d.as_millis()).unwrap_or(u64::MAX);
        let audio = fala_media::import(tools, &input, &dir.join(WORK_WAV), cancel, &mut |p| {
            let total = p.total.map(ms).unwrap_or(0);
            self.progress(id, stage("convert", ms(p.processed), total));
        })
        .map_err(media_error)?;
        if url.is_some() {
            let _ = std::fs::remove_dir_all(dir.join(DOWNLOAD_DIR));
        }
        let span = (started, now_local());
        self.with_store(|store| save_import(store, id, &title, url, span, ms(audio.duration)))?;
        self.retain(id, &dir);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_shape_and_title() {
        assert_eq!(
            serde_json::to_value(ImportSource::Url("https://a.b/v".into())).unwrap(),
            serde_json::json!({ "kind": "url", "value": "https://a.b/v" })
        );
        let path = Path::new("Aulas").join("Aula 3 – física.mp4");
        assert_eq!(file_title(&path), "Aula 3 – física");
        assert_eq!(file_title(Path::new("")), "");
    }

    #[test]
    fn media_errors_reach_the_page_typed() {
        assert_eq!(
            media_error(MediaError::ToolNotFound { tool: "yt-dlp" }),
            MeetingError::ToolMissing("yt-dlp".into())
        );
        assert_eq!(
            media_error(MediaError::InvalidUrl("ftp://x".into())),
            MeetingError::InvalidUrl
        );
        assert_eq!(media_error(MediaError::Cancelled), MeetingError::Cancelled);
        assert_eq!(
            media_error(MediaError::NoAudioTrack("v.mp4".into())),
            MeetingError::ImportNoAudio
        );
        let ffmpeg = MediaError::Ffmpeg {
            code: Some(1),
            detail: "Invalid data found when processing input".into(),
        };
        assert_eq!(
            media_error(ffmpeg),
            MeetingError::Import("Invalid data found when processing input".into())
        );
        let download = MediaError::Download {
            code: Some(1),
            detail: "WARNING: sem runtime JS\nERROR: Video unavailable\n".into(),
        };
        assert_eq!(
            media_error(download),
            MeetingError::Download("ERROR: Video unavailable".into())
        );
    }

    #[test]
    fn imported_file_is_a_finished_session_with_its_title() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("import.sqlite"), dir.path()).unwrap();
        let started = now_local();
        let ended = started + chrono::Duration::seconds(3);
        let file = SessionId::from_parts(1_800_000_000_000, 1);
        let title = file_title(&Path::new("Vídeos").join("Aula de física.mp4"));
        save_import(&store, file, &title, None, (started, ended), 61_000).unwrap();
        let record = store.meeting(file).unwrap();
        assert_eq!(record.title, "Aula de física");
        assert_eq!(record.mode, SessionMode::Import);
        assert_eq!(record.recorded_ms, 61_000);
        assert!(record.started_at.is_some() && record.ended_at.is_some());
        assert_eq!(record.annotations, "");

        let link = SessionId::from_parts(1_800_000_000_001, 2);
        let url = "https://www.youtube.com/watch?v=jNQXAC9IVRw";
        save_import(
            &store,
            link,
            "Me at the zoo",
            Some(url),
            (started, ended),
            19_000,
        )
        .unwrap();
        assert_eq!(store.meeting(link).unwrap().annotations, url);
    }
}
