//! Shared shortcut event handling logic
//!
//! This module contains the common logic for handling shortcut events,
//! used by both the Tauri and handy-keys implementations.

use log::warn;
use tauri::{AppHandle, Manager};

use crate::actions::ACTION_MAP;
use crate::settings::get_settings;
use crate::transcription_coordinator::is_transcribe_binding;
use crate::TranscriptionCoordinator;

/// Handle a shortcut event from either implementation.
///
/// This function contains the shared logic for:
/// - Looking up the action in ACTION_MAP
/// - Handling the cancel binding (only fires while a dictation is in flight)
/// - Routing transcribe bindings to the coordinator, which applies the
///   configured activation mode (toggle / push-to-talk / hold-or-toggle)
///
/// # Arguments
/// * `app` - The Tauri app handle
/// * `registered_id` - The ID of the binding (e.g., "transcribe", "cancel"); an action's second
///   combination arrives as `<id>#2` and runs the same action
/// * `hotkey_string` - The string representation of the hotkey
/// * `is_pressed` - Whether this is a key press (true) or release (false)
pub fn handle_shortcut_event(
    app: &AppHandle,
    registered_id: &str,
    hotkey_string: &str,
    is_pressed: bool,
) {
    let binding_id = super::action_id(registered_id);
    let settings = get_settings(app);

    // Transcribe bindings are handled by the coordinator. It gets the registered id, so the two
    // combinations stay two keys: releasing one never ends a hold on the other.
    if is_transcribe_binding(binding_id) {
        if let Some(coordinator) = app.try_state::<TranscriptionCoordinator>() {
            coordinator.send_input(
                registered_id,
                hotkey_string,
                is_pressed,
                settings.shortcut_activation,
                std::time::Duration::from_millis(settings.hold_threshold_ms),
            );
        } else {
            warn!("TranscriptionCoordinator is not initialized");
        }
        return;
    }

    let Some(action) = ACTION_MAP.get(binding_id) else {
        warn!(
            "No action defined in ACTION_MAP for shortcut ID '{}'. Shortcut: '{}', Pressed: {}",
            binding_id, hotkey_string, is_pressed
        );
        return;
    };

    // Cancel binding: fires on press while a dictation is in flight —
    // recording, transcribing or post-processing — and never after it ended.
    if binding_id == "cancel" {
        let busy = app
            .try_state::<TranscriptionCoordinator>()
            .is_some_and(|c| c.is_busy());
        if cancel_key_fires(is_pressed, busy) {
            action.start(app, binding_id, hotkey_string);
        }
        return;
    }

    // Remaining bindings (e.g. "test") use simple start/stop on press/release.
    if is_pressed {
        action.start(app, binding_id, hotkey_string);
    } else {
        action.stop(app, binding_id, hotkey_string);
    }
}

/// Whether a cancel key event cancels: a press while the coordinator has a
/// dictation in flight. Releases and presses with nothing in flight do nothing.
fn cancel_key_fires(is_pressed: bool, busy: bool) -> bool {
    is_pressed && busy
}

#[cfg(test)]
mod tests {
    use super::cancel_key_fires;

    #[test]
    fn cancel_key_fires_only_on_press_while_busy() {
        let cases = [
            (true, true, true),
            (true, false, false),
            (false, true, false),
            (false, false, false),
        ];
        for (is_pressed, busy, expected) in cases {
            assert_eq!(
                cancel_key_fires(is_pressed, busy),
                expected,
                "is_pressed={is_pressed} busy={busy}"
            );
        }
    }

    #[test]
    fn second_combination_acts_as_the_first() {
        use crate::actions::ACTION_MAP;
        use crate::shortcut::action_id;
        use crate::transcription_coordinator::is_transcribe_binding;

        assert_eq!(action_id("transcribe#2"), "transcribe");
        assert!(is_transcribe_binding(action_id("transcribe#2")));
        assert_eq!(action_id("cancel#2"), "cancel");
        for id in ["paste_last", "copy_last", "meeting_toggle"] {
            let second = format!("{id}#2");
            assert_eq!(action_id(&second), id);
            assert!(ACTION_MAP.contains_key(action_id(&second)), "{id}");
        }
        assert_eq!(action_id("paste_last"), "paste_last");
    }
}
