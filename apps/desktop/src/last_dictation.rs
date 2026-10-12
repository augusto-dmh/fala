//! Paste or copy the last dictation again, from a shortcut or the tray.
//!
//! The text is the one the history shows for the newest entry with ASR text, so it matches
//! what "Copiar" copies on the home screen. Pasting goes through `utils::paste`, the same path
//! a dictation takes (per-app chord, reliable paste when on, auto-submit, clipboard handling).

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use log::{debug, error, info};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::managers::history::{HistoryEntry, HistoryManager};
use crate::managers::history_dictations::HistoryShowing;
use crate::TranscriptionCoordinator;

/// How long a paste waits for the user to let go of the shortcut's modifiers.
const MODIFIER_RELEASE_TIMEOUT: Duration = Duration::from_millis(1000);

/// The text the entry shows: its dictation's final text, or the raw one after an undo. When
/// the pipeline emptied it, the raw text, which is what "Recuperar" would bring back. Without a
/// linked dictation, the history.db text. `None` when there is nothing but whitespace.
pub fn text_of(entry: &HistoryEntry) -> Option<&str> {
    let text = match &entry.dictation {
        Some(d) => {
            let shown = match d.showing {
                HistoryShowing::Final => &d.final_text,
                HistoryShowing::Raw => &d.raw_text,
            };
            if shown.trim().is_empty() {
                &d.raw_text
            } else {
                shown
            }
        }
        None => entry
            .post_processed_text
            .as_ref()
            .unwrap_or(&entry.transcription_text),
    };
    (!text.trim().is_empty()).then_some(text.as_str())
}

/// What `paste_last` pastes: nothing while a dictation is in flight (it is not interrupted);
/// the text whose paste failed while the pill offers it ("cola de novo"), which may have no
/// history entry; otherwise the newest entry's text.
fn to_paste<'a>(
    busy: bool,
    failed: Option<&'a str>,
    latest: Option<&'a HistoryEntry>,
) -> Option<&'a str> {
    if busy {
        return None;
    }
    failed.or_else(|| latest.and_then(text_of))
}

fn latest_entry(app: &AppHandle) -> Option<HistoryEntry> {
    match app
        .state::<Arc<HistoryManager>>()
        .get_latest_completed_entry()
    {
        Ok(entry) => entry,
        Err(err) => {
            error!("Failed to fetch the last dictation: {}", err);
            None
        }
    }
}

/// Puts the last dictation on the clipboard.
pub fn copy_last(app: &AppHandle) {
    let Some(entry) = latest_entry(app) else {
        debug!("copy_last: no dictation to copy");
        return;
    };
    let Some(text) = text_of(&entry) else {
        debug!("copy_last: the last dictation is empty");
        return;
    };
    match app.clipboard().write_text(text) {
        Ok(()) => info!("Copied the last dictation to the clipboard"),
        Err(err) => error!("Failed to copy the last dictation: {}", err),
    }
}

/// The text of the paste that failed, while the pill says so: a click on it copies the text.
/// Only in memory, and dropped when that pill ends or another state replaces it.
static FAILED_PASTE: Mutex<Option<String>> = Mutex::new(None);

fn failed_paste() -> MutexGuard<'static, Option<String>> {
    FAILED_PASTE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn remember_failed_paste(text: String) {
    *failed_paste() = Some(text);
}

pub fn forget_failed_paste() {
    failed_paste().take();
}

/// A click on the pill that says the paste failed: puts that text on the clipboard, where the
/// user asked for it. `false` when there is nothing to copy or the clipboard refused it.
#[tauri::command]
#[specta::specta]
pub fn copy_failed_paste(app: AppHandle) -> bool {
    let Some(text) = failed_paste().clone() else {
        debug!("copy_failed_paste: no failed paste to copy");
        return false;
    };
    match app.clipboard().write_text(text) {
        Ok(()) => {
            info!("Copied the dictation that failed to paste");
            true
        }
        Err(err) => {
            error!("Failed to copy the dictation that failed to paste: {}", err);
            false
        }
    }
}

/// Pastes the last dictation into the app in focus. Ignored while a dictation is in flight.
/// `from_tray` gives focus back to the app the tray menu took it from first (Windows).
pub fn paste_last(app: &AppHandle, from_tray: bool) {
    let busy = app
        .try_state::<TranscriptionCoordinator>()
        .is_some_and(|c| c.is_busy());
    let failed = failed_paste().clone();
    let entry = if busy || failed.is_some() {
        None
    } else {
        latest_entry(app)
    };
    let Some(text) = to_paste(busy, failed.as_deref(), entry.as_ref()).map(str::to_owned) else {
        debug!("paste_last: nothing pasted (dictation in progress: {busy})");
        return;
    };

    // Off the calling thread: the wait below must not stall the hotkey hook or the event loop.
    let app = app.clone();
    std::thread::spawn(move || {
        #[cfg(windows)]
        {
            if from_tray {
                platform::restore_foreground();
            }
            if !platform::wait_for_modifiers_released(MODIFIER_RELEASE_TIMEOUT) {
                debug!("paste_last: modifiers still held after the timeout; pasting anyway");
            }
        }
        #[cfg(not(windows))]
        let _ = (from_tray, MODIFIER_RELEASE_TIMEOUT);

        let ah = app.clone();
        let scheduled =
            app.run_on_main_thread(
                move || match crate::utils::paste(text.clone(), ah.clone()) {
                    Ok(()) => info!("Pasted the last dictation"),
                    Err(e) => {
                        error!("Failed to paste the last dictation: {}", e);
                        let _ = ah.emit("paste-error", ());
                        crate::overlay::show_paste_failed(&ah, text);
                    }
                },
            );
        if let Err(e) = scheduled {
            error!("Failed to run the paste on the main thread: {:?}", e);
        }
    });
}

/// Remembers the app in the foreground when the pointer reaches the tray icon, before the
/// menu takes the focus. Windows only: the tray's pointer events are wired there.
#[cfg(windows)]
pub fn remember_foreground() {
    platform::remember_foreground();
}

#[cfg(windows)]
mod platform {
    use std::sync::atomic::{AtomicIsize, Ordering};
    use std::time::{Duration, Instant};

    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, IsWindow, SetForegroundWindow,
    };

    /// The window that had focus before the tray menu opened; 0 when unknown.
    static TRAY_FOREGROUND: AtomicIsize = AtomicIsize::new(0);

    /// Time for the restored window to take the focus before the chord.
    const FOCUS_SETTLE: Duration = Duration::from_millis(150);

    pub(super) fn remember_foreground() {
        // SAFETY: no arguments; returns a null handle when no window has the focus.
        let hwnd = unsafe { GetForegroundWindow() };
        TRAY_FOREGROUND.store(hwnd.0 as isize, Ordering::Relaxed);
    }

    pub(super) fn restore_foreground() {
        let raw = TRAY_FOREGROUND.load(Ordering::Relaxed);
        if raw == 0 {
            return;
        }
        let hwnd = HWND(raw as *mut _);
        // SAFETY: both calls only read the handle; a stale one makes `IsWindow` false.
        let restored =
            unsafe { IsWindow(Some(hwnd)).as_bool() && SetForegroundWindow(hwnd).as_bool() };
        if restored {
            std::thread::sleep(FOCUS_SETTLE);
        } else {
            log::debug!("paste_last: could not give the focus back to the previous window");
        }
    }

    /// Waits until Ctrl, Shift, Alt and Win are all up, so the paste chord does not combine
    /// with the shortcut that triggered it. `false` on timeout.
    pub(super) fn wait_for_modifiers_released(timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            let held = [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN]
                .iter()
                // SAFETY: reads the key's physical state; the high bit is set while it is down.
                .any(|vk| unsafe { GetAsyncKeyState(i32::from(vk.0)) } < 0);
            if !held {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{failed_paste, forget_failed_paste, remember_failed_paste, text_of, to_paste};
    use crate::managers::history::HistoryEntry;
    use crate::managers::history_dictations::{HistoryDictation, HistoryEditor, HistoryShowing};

    fn entry(transcription: &str, post_processed: Option<&str>) -> HistoryEntry {
        HistoryEntry {
            id: 1,
            file_name: "fala-1.wav".to_string(),
            timestamp: 0,
            saved: false,
            title: "Recording".to_string(),
            transcription_text: transcription.to_string(),
            post_processed_text: post_processed.map(str::to_string),
            post_process_prompt: None,
            post_process_requested: false,
            dictation_id: None,
            dictation: None,
            paste_failed: false,
            discarded: false,
            llm_partial: false,
        }
    }

    fn with_dictation(raw: &str, final_text: &str, showing: HistoryShowing) -> HistoryEntry {
        let mut e = entry(raw, None);
        e.dictation_id = Some("d1".to_string());
        e.dictation = Some(HistoryDictation {
            raw_text: raw.to_string(),
            final_text: final_text.to_string(),
            editor: HistoryEditor::Rules,
            showing,
            app_name: Some("notepad".to_string()),
        });
        e
    }

    #[test]
    fn text_is_what_the_history_shows() {
        let delivered = with_dictation("ação de amanhã", "Ação de amanhã.", HistoryShowing::Final);
        assert_eq!(text_of(&delivered), Some("Ação de amanhã."));
        let undone = with_dictation("ação de amanhã", "Ação de amanhã.", HistoryShowing::Raw);
        assert_eq!(text_of(&undone), Some("ação de amanhã"));
        // The pipeline emptied it: the recoverable raw text.
        let emptied = with_dictation("ação de amanhã", " ", HistoryShowing::Final);
        assert_eq!(text_of(&emptied), Some("ação de amanhã"));
        // No linked dictation: the history.db text.
        assert_eq!(text_of(&entry("raw", Some("processed"))), Some("processed"));
        assert_eq!(text_of(&entry("raw", None)), Some("raw"));
        assert_eq!(text_of(&entry("  ", None)), None);
        assert_eq!(
            text_of(&with_dictation(" ", "", HistoryShowing::Final)),
            None
        );
    }

    #[test]
    fn paste_is_ignored_while_busy_and_without_history() {
        let latest = with_dictation("bruto", "Final.", HistoryShowing::Final);
        assert_eq!(to_paste(false, None, Some(&latest)), Some("Final."));
        assert_eq!(to_paste(true, None, Some(&latest)), None);
        assert_eq!(to_paste(false, None, None), None);
        assert_eq!(to_paste(true, None, None), None);
        // While the pill says a paste failed, its text, even without a history entry.
        assert_eq!(
            to_paste(false, Some("falhou"), Some(&latest)),
            Some("falhou")
        );
        assert_eq!(to_paste(false, Some("falhou"), None), Some("falhou"));
        assert_eq!(to_paste(true, Some("falhou"), Some(&latest)), None);
    }

    #[test]
    fn failed_paste_is_kept_until_the_pill_ends() {
        remember_failed_paste("primeiro".to_string());
        remember_failed_paste("ação de amanhã".to_string());
        assert_eq!(failed_paste().as_deref(), Some("ação de amanhã"));
        forget_failed_paste();
        assert_eq!(*failed_paste(), None);
        // Both paste paths hand the text to the pill when the paste fails.
        let source = include_str!("last_dictation.rs");
        let source = &source[..source.find("#[cfg(test)]\nmod tests").unwrap()];
        let failed = &source[source.find("Failed to paste the last dictation").unwrap()..];
        assert!(failed.contains("crate::overlay::show_paste_failed(&ah, text)"));
    }
}
