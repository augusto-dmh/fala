//! Keyboard shortcut management module
//!
//! This module provides a unified interface for keyboard shortcuts with
//! multiple backend implementations:
//!
//! - `tauri`: Uses Tauri's built-in global-shortcut plugin
//! - `fala_keys`: Uses the handy-keys library for more control
//!
//! The active implementation is determined by the `keyboard_implementation`
//! setting and can be changed at runtime.

mod conflict;
pub mod fala_keys;
mod handler;
pub mod tauri_impl;

pub use conflict::{ShortcutOverlap, ShortcutRefusal};

use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

use crate::meeting::{MeetingManager, MeetingPhase};
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use crate::settings::APPLE_INTELLIGENCE_DEFAULT_MODEL_ID;
use crate::settings::{
    self, get_settings, AutoSubmitKey, ClipboardHandling, KeyboardImplementation, LLMPrompt,
    OverlayPosition, OverlayStyle, PasteMethod, ShortcutActivation, ShortcutBinding, SoundTheme,
    Theme, TypingTool, APPLE_INTELLIGENCE_PROVIDER_ID,
};
use crate::tray;
use crate::TranscriptionCoordinator;

// Note: Commands are accessed via shortcut::fala_keys:: in lib.rs

/// Initialize shortcuts using the configured implementation
pub fn init_shortcuts(app: &AppHandle) {
    let user_settings = settings::load_or_create_app_settings(app);

    // Check which implementation to use
    match user_settings.keyboard_implementation {
        KeyboardImplementation::Tauri => {
            tauri_impl::init_shortcuts(app);
        }
        KeyboardImplementation::FalaKeys => {
            if let Err(e) = fala_keys::init_shortcuts(app) {
                error!("Failed to initialize handy-keys shortcuts: {}", e);
                // Fall back to Tauri implementation and persist this fallback
                warn!("Falling back to Tauri global shortcut implementation and saving fallback to settings");

                // Update settings to persist the fallback so we don't retry FalaKeys on next launch
                let mut settings = settings::get_settings(app);
                settings.keyboard_implementation = KeyboardImplementation::Tauri;
                settings::write_settings(app, settings);

                tauri_impl::init_shortcuts(app);
            }
        }
    }
}

/// The cancel key's requested and actual registration. Callers write the request
/// synchronously, then a spawned pass applies it. Spawned tasks run in any order:
/// a very short dictation can see its unregister task run before its register task,
/// which used to leave Esc captured system-wide until the app quit. Each pass applies
/// the latest request instead of the action that scheduled it, so order no longer
/// matters (cjpais/Handy#2190).
struct CancelKey<B> {
    /// What the dictation lifecycle wants now; always the latest request.
    requested: AtomicBool,
    /// The binding registered with the backend, if any. The lock also serializes passes.
    registered: Mutex<Option<B>>,
}

impl<B> CancelKey<B> {
    const fn new() -> Self {
        Self {
            requested: AtomicBool::new(false),
            registered: Mutex::new(None),
        }
    }

    fn request(&self, armed: bool) {
        self.requested.store(armed, Ordering::SeqCst);
    }

    /// Bring the registration in line with the latest request. `register` returns the
    /// binding it registered; `unregister` gets that same binding back, so a cancel
    /// binding edited mid-dictation still releases the key that was captured. A failed
    /// call leaves the recorded state alone, so the next pass retries.
    fn reconcile(
        &self,
        register: impl FnOnce() -> Result<B, String>,
        unregister: impl FnOnce(&B) -> Result<(), String>,
    ) {
        let mut registered = self.registered.lock().unwrap_or_else(|e| e.into_inner());
        let requested = self.requested.load(Ordering::SeqCst);
        match (requested, registered.as_ref()) {
            (true, None) => match register() {
                Ok(binding) => *registered = Some(binding),
                Err(e) => error!("Failed to register cancel shortcut: {}", e),
            },
            (false, Some(binding)) => match unregister(binding) {
                Ok(()) => *registered = None,
                Err(e) => error!("Failed to unregister cancel shortcut: {}", e),
            },
            _ => {}
        }
    }
}

static CANCEL_KEY: CancelKey<ShortcutBinding> = CancelKey::new();

/// Register the cancel shortcut (called when a dictation starts)
pub fn register_cancel_shortcut(app: &AppHandle) {
    // Track recording lifecycle independently of the current implementation so
    // switching implementations mid-recording cannot leave stale fallback state.
    crate::secure_input::register_cancel_fallback(app);

    CANCEL_KEY.request(true);
    schedule_cancel_reconcile(app);
}

/// Unregister the cancel shortcut (called when the dictation ends)
pub fn unregister_cancel_shortcut(app: &AppHandle) {
    crate::secure_input::unregister_cancel_fallback(app);

    CANCEL_KEY.request(false);
    schedule_cancel_reconcile(app);
}

/// Apply the requested cancel state off the calling thread, which must not block on
/// registration.
fn schedule_cancel_reconcile(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        CANCEL_KEY.reconcile(
            || register_cancel_binding(&app),
            |binding| unregister_cancel_binding(&app, binding),
        );
    });
}

// The cancel shortcut is disabled on Linux due to instability with dynamic shortcut
// registration: there the binding is tracked but never handed to the backend.

fn register_cancel_binding(app: &AppHandle) -> Result<ShortcutBinding, String> {
    let binding = get_settings(app)
        .bindings
        .get("cancel")
        .cloned()
        .ok_or_else(|| "no cancel binding in settings".to_string())?;
    #[cfg(not(target_os = "linux"))]
    register_shortcut(app, binding.clone())?;
    Ok(binding)
}

fn unregister_cancel_binding(app: &AppHandle, binding: &ShortcutBinding) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let _ = (app, binding);
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        unregister_shortcut(app, binding.clone())
    }
}

/// Register a shortcut using the appropriate implementation
pub fn register_shortcut(app: &AppHandle, binding: ShortcutBinding) -> Result<(), String> {
    let settings = get_settings(app);
    match settings.keyboard_implementation {
        KeyboardImplementation::Tauri => tauri_impl::register_shortcut(app, binding),
        KeyboardImplementation::FalaKeys => fala_keys::register_shortcut(app, binding),
    }
}

/// Unregister a shortcut using the appropriate implementation
pub fn unregister_shortcut(app: &AppHandle, binding: ShortcutBinding) -> Result<(), String> {
    let settings = get_settings(app);
    match settings.keyboard_implementation {
        KeyboardImplementation::Tauri => tauri_impl::unregister_shortcut(app, binding),
        KeyboardImplementation::FalaKeys => fala_keys::unregister_shortcut(app, binding),
    }
}

// ============================================================================
// Binding Management Commands
// ============================================================================

/// Which of an action's two combinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum BindingSlot {
    Primary,
    Alternate,
}

#[derive(Serialize, Type)]
pub struct BindingResponse {
    success: bool,
    binding: Option<ShortcutBinding>,
    error: Option<String>,
    /// Why the combination was not saved, for the shortcuts dialog to explain.
    refusal: Option<ShortcutRefusal>,
    /// Saved, but holding one combination may fire another action.
    overlap: Option<ShortcutOverlap>,
}

impl BindingResponse {
    fn saved(binding: ShortcutBinding) -> Self {
        Self {
            success: true,
            binding: Some(binding),
            error: None,
            refusal: None,
            overlap: None,
        }
    }

    fn failed(error: String) -> Self {
        Self {
            success: false,
            binding: None,
            error: Some(error),
            refusal: None,
            overlap: None,
        }
    }

    /// `error` stays readable for callers that only show it; the dialog reads `refusal`.
    fn refused(refusal: ShortcutRefusal) -> Self {
        let error = match &refusal {
            ShortcutRefusal::InUse { action, .. } => format!("Already used by '{action}'"),
            ShortcutRefusal::Reserved => "Windows or another app already uses it".to_string(),
            ShortcutRefusal::LoneKey => "Use it with Ctrl, Alt, Shift or Win".to_string(),
        };
        Self {
            refusal: Some(refusal),
            ..Self::failed(error)
        }
    }
}

/// The first combination of an action, without taking it from another one.
#[tauri::command]
#[specta::specta]
pub fn change_binding(
    app: AppHandle,
    id: String,
    binding: String,
) -> Result<BindingResponse, String> {
    set_binding(app, id, BindingSlot::Primary, binding, false)
}

/// Saves `binding` in `slot` of action `id`. A combination another action holds is refused,
/// unless `take_over` ("use it here") takes it from that action in the same operation.
#[tauri::command]
#[specta::specta]
pub fn set_binding(
    app: AppHandle,
    id: String,
    slot: BindingSlot,
    binding: String,
    take_over: bool,
) -> Result<BindingResponse, String> {
    // Reject empty bindings: removing one is `remove_binding`
    if binding.trim().is_empty() {
        return Err("Binding cannot be empty".to_string());
    }
    if id == "cancel" && slot == BindingSlot::Alternate {
        return Err("The cancel shortcut has one combination only".to_string());
    }

    let settings = settings::get_settings(&app);

    // Get the binding to modify, or create it from defaults if it doesn't exist
    let binding_to_modify = match settings.bindings.get(&id) {
        Some(binding) => binding.clone(),
        None => match settings::get_default_settings().bindings.remove(&id) {
            Some(default_binding) => {
                warn!(
                    "Binding '{}' not found in settings, creating from defaults",
                    id
                );
                default_binding
            }
            None => {
                let error_msg = format!("Binding with id '{}' not found in defaults", id);
                warn!("change_binding error: {}", error_msg);
                return Ok(BindingResponse::failed(error_msg));
            }
        },
    };

    // Validate the new shortcut for the current keyboard implementation
    if let Err(e) = validate_shortcut_for_implementation(&binding, settings.keyboard_implementation)
    {
        warn!("change_binding validation error: {}", e);
        return Err(e);
    }

    let placed = place_binding(
        &settings.bindings,
        binding_to_modify,
        slot,
        &binding,
        take_over,
    );
    let (next, overlap) = match placed {
        Ok(placed) => placed,
        Err(refusal) => {
            info!("Shortcut for '{}' refused: {:?}", id, refusal);
            return Ok(BindingResponse::refused(refusal));
        }
    };
    if let Err(e) = apply_bindings(&app, &settings, next.clone()) {
        error!("change_binding error: {}", e);
        return Ok(BindingResponse::failed(e));
    }
    Ok(BindingResponse {
        overlap,
        ..BindingResponse::saved(next[&id].clone())
    })
}

/// Removes one combination; a second one becomes the first. Cancel keeps its key and dictation
/// its last one.
#[tauri::command]
#[specta::specta]
pub fn remove_binding(
    app: AppHandle,
    id: String,
    slot: BindingSlot,
) -> Result<BindingResponse, String> {
    let settings = settings::get_settings(&app);
    let binding = settings::get_stored_binding(&settings, &id)?;
    if !slot_removable(&binding, slot) {
        return Err(format!("The '{}' shortcut must keep a combination", id));
    }
    let removed = without_slot(&binding, slot);
    let mut next = settings.bindings.clone();
    next.insert(id, removed.clone());
    apply_bindings(&app, &settings, next)?;
    Ok(BindingResponse::saved(removed))
}

/// One action back to its default: the default as the first combination and no second one.
#[tauri::command]
#[specta::specta]
pub fn reset_binding(app: AppHandle, id: String) -> Result<BindingResponse, String> {
    let settings = settings::get_settings(&app);
    let binding = settings::get_stored_binding(&settings, &id)?;
    let restored = restored(&binding, &settings::get_default_settings().bindings);
    let mut next = settings.bindings.clone();
    next.insert(id.clone(), restored.clone());
    // Another action may have taken the default meanwhile.
    let default = &restored.current_binding;
    if let Err(refusal @ ShortcutRefusal::InUse { .. }) =
        conflict::check(default, &id, BindingSlot::Primary, &next)
    {
        return Ok(BindingResponse::refused(refusal));
    }
    apply_bindings(&app, &settings, next)?;
    Ok(BindingResponse::saved(restored))
}

/// Why "Restore defaults" refused, serialized as `{"kind": "..."}` for the dialog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum ResetBindingsError {
    /// A dictation is in flight: finish it first.
    DictationActive,
    /// A meeting records or is paused: restoring would take away the key that stops it.
    MeetingRecording,
    Failed(String),
}

/// "Restore defaults": every action's first combination back to its default and no second one,
/// registered in one pass and undone if a registration fails.
#[tauri::command]
#[specta::specta]
pub fn reset_all_bindings(app: AppHandle) -> Result<(), ResetBindingsError> {
    let dictating = app
        .try_state::<TranscriptionCoordinator>()
        .is_some_and(|c| c.is_busy());
    let meeting = app
        .try_state::<Arc<MeetingManager>>()
        .map(|m| m.status().state)
        .unwrap_or_default();
    if let Some(refusal) = reset_refusal(dictating, meeting) {
        info!("Restoring the default shortcuts refused: {:?}", refusal);
        return Err(refusal);
    }
    let settings = settings::get_settings(&app);
    let next = defaults_restored(
        &settings.bindings,
        &settings::get_default_settings().bindings,
    );
    apply_bindings(&app, &settings, next).map_err(ResetBindingsError::Failed)?;
    info!("Default shortcuts restored");
    Ok(())
}

fn reset_refusal(dictating: bool, meeting: MeetingPhase) -> Option<ResetBindingsError> {
    if dictating {
        return Some(ResetBindingsError::DictationActive);
    }
    match meeting {
        MeetingPhase::Recording | MeetingPhase::Paused => {
            Some(ResetBindingsError::MeetingRecording)
        }
        _ => None,
    }
}

/// The binding with today's default as its first combination and no second one. An old store
/// may carry an outdated `default_binding`; the defaults of this version win.
fn restored(
    binding: &ShortcutBinding,
    defaults: &HashMap<String, ShortcutBinding>,
) -> ShortcutBinding {
    let mut restored = binding.clone();
    if let Some(default) = defaults.get(&binding.id) {
        restored.default_binding = default.default_binding.clone();
    }
    restored.current_binding = restored.default_binding.clone();
    restored.alternate_binding.clear();
    restored
}

fn defaults_restored(
    bindings: &HashMap<String, ShortcutBinding>,
    defaults: &HashMap<String, ShortcutBinding>,
) -> HashMap<String, ShortcutBinding> {
    bindings
        .iter()
        .map(|(id, binding)| (id.clone(), restored(binding, defaults)))
        .collect()
}

/// Suffix of the id a second combination is registered under.
const ALTERNATE_SUFFIX: &str = "#2";

/// The action a registered id belongs to: `<id>#2` is `<id>`'s second combination.
pub(crate) fn action_id(registered: &str) -> &str {
    registered
        .strip_suffix(ALTERNATE_SUFFIX)
        .unwrap_or(registered)
}

/// What a binding registers with a backend: the first combination under the action's id, the
/// second under `<id>#2`. Cancel has one only; an unset combination registers nothing.
pub(crate) fn registrations(binding: &ShortcutBinding) -> Vec<ShortcutBinding> {
    let mut registered = Vec::new();
    if !binding_is_unset(binding) {
        registered.push(binding.clone());
    }
    if binding.id != "cancel" && !binding.alternate_binding.trim().is_empty() {
        let mut alternate = binding.clone();
        alternate.id = format!("{}{}", binding.id, ALTERNATE_SUFFIX);
        alternate.current_binding = binding.alternate_binding.clone();
        registered.push(alternate);
    }
    registered
}

/// Whether the action's shortcuts are registered outside a dictation: cancel only during one, the
/// post-processing binding only with the feature on.
pub(crate) fn registered_now(id: &str, settings: &settings::AppSettings) -> bool {
    id != "cancel" && (id != "transcribe_with_post_process" || settings.post_process_enabled)
}

/// Cancel keeps its one combination and dictation its last.
fn slot_removable(binding: &ShortcutBinding, slot: BindingSlot) -> bool {
    let other = match slot {
        BindingSlot::Primary => &binding.alternate_binding,
        BindingSlot::Alternate => &binding.current_binding,
    };
    match binding.id.as_str() {
        "cancel" => false,
        "transcribe" => !other.trim().is_empty(),
        _ => true,
    }
}

/// The binding without the combination in `slot`; a second combination becomes the first.
fn without_slot(binding: &ShortcutBinding, slot: BindingSlot) -> ShortcutBinding {
    let mut removed = binding.clone();
    match slot {
        BindingSlot::Primary => {
            removed.current_binding = std::mem::take(&mut removed.alternate_binding);
        }
        BindingSlot::Alternate => removed.alternate_binding.clear(),
    }
    removed
}

/// The bindings with `raw` in `binding`'s `slot`, or why not. With `take_over`, a combination
/// another action (or the other slot) holds is taken from it, unless that would leave it without
/// a key it must keep. A second combination without a first becomes the first.
fn place_binding(
    bindings: &HashMap<String, ShortcutBinding>,
    binding: ShortcutBinding,
    slot: BindingSlot,
    raw: &str,
    take_over: bool,
) -> Result<(HashMap<String, ShortcutBinding>, Option<ShortcutOverlap>), ShortcutRefusal> {
    let id = binding.id.clone();
    let mut next = bindings.clone();
    next.entry(id.clone()).or_insert(binding);
    let overlap = loop {
        match conflict::check(raw, &id, slot, &next) {
            Ok(overlap) => break overlap,
            Err(ShortcutRefusal::InUse {
                action, slot: held, ..
            }) => {
                let allowed = next
                    .get(&action)
                    .is_some_and(|other| slot_removable(other, held));
                if !(take_over && allowed) {
                    return Err(ShortcutRefusal::InUse {
                        action,
                        slot: held,
                        take_over: allowed,
                    });
                }
                if let Some(other) = next.get_mut(&action) {
                    *other = without_slot(other, held);
                }
            }
            Err(refusal) => return Err(refusal),
        }
    };
    if let Some(placed) = next.get_mut(&id) {
        match slot {
            BindingSlot::Primary => placed.current_binding = raw.to_string(),
            BindingSlot::Alternate => placed.alternate_binding = raw.to_string(),
        }
        if binding_is_unset(placed) {
            placed.current_binding = std::mem::take(&mut placed.alternate_binding);
        }
    }
    Ok((next, overlap))
}

/// Saves `next` and brings the registrations in line with it: every registered action that
/// changed is unregistered, then registered again. If a registration fails, the previous ones
/// come back and nothing is saved, so a failure leaves the shortcuts working as before.
fn apply_bindings(
    app: &AppHandle,
    settings: &settings::AppSettings,
    next: HashMap<String, ShortcutBinding>,
) -> Result<(), String> {
    let mut live: Vec<&String> = next
        .iter()
        .filter(|(id, binding)| {
            registered_now(id, settings)
                && settings.bindings.get(*id).is_none_or(|old| {
                    old.current_binding != binding.current_binding
                        || old.alternate_binding != binding.alternate_binding
                })
        })
        .map(|(id, _)| id)
        .collect();
    live.sort();

    for id in &live {
        let old = settings.bindings.get(*id).map(registrations);
        for registered in old.unwrap_or_default() {
            if let Err(e) = unregister_shortcut(app, registered) {
                debug!("apply_bindings: could not unregister '{}': {}", id, e);
            }
        }
    }
    let mut done: Vec<ShortcutBinding> = Vec::new();
    let mut failure = None;
    'register: for id in &live {
        for registered in registrations(&next[*id]) {
            match register_shortcut(app, registered.clone()) {
                Ok(()) => done.push(registered),
                Err(e) => {
                    failure = Some(format!("Failed to register shortcut: {}", e));
                    break 'register;
                }
            }
        }
    }
    if let Some(error) = failure {
        for registered in done {
            let _ = unregister_shortcut(app, registered);
        }
        for id in &live {
            if let Some(old) = settings.bindings.get(*id) {
                restore_registration(app, old);
            }
        }
        return Err(error);
    }

    // Read again: another setting may have changed while the backend registered.
    let mut saved = settings::get_settings(app);
    saved.bindings = next;
    settings::write_settings(app, saved);
    crate::secure_input::reconcile_fallback(app);
    Ok(())
}

/// Best-effort re-register of the previous binding after a failed change,
/// so a failure leaves the user's shortcut working exactly as before.
fn restore_registration(app: &AppHandle, binding: &ShortcutBinding) {
    for registered in registrations(binding) {
        if let Err(e) = register_shortcut(app, registered.clone()) {
            error!(
                "Failed to restore previous binding '{}' ({}): {}",
                registered.id, registered.current_binding, e
            );
        }
    }
}

/// Unregister every binding while the user is recording a new shortcut in
/// the UI, so no existing shortcut can fire — or swallow the keystrokes —
/// mid-capture. The "cancel" binding is untouched: it is managed dynamically
/// by the recording lifecycle.
pub fn suspend_all_shortcuts(app: &AppHandle) {
    for (id, binding) in settings::get_bindings(app) {
        if id == "cancel" {
            continue;
        }
        for registered in registrations(&binding) {
            if let Err(e) = unregister_shortcut(app, registered) {
                debug!(
                    "suspend_all_shortcuts: could not unregister '{}': {}",
                    id, e
                );
            }
        }
    }
}

/// A binding with no key (the legacy post-processing one by default) is never registered.
pub(crate) fn binding_is_unset(binding: &ShortcutBinding) -> bool {
    binding.current_binding.trim().is_empty()
}

/// Re-register every binding from settings after shortcut recording ends.
/// Registering an already-registered shortcut fails cleanly in both
/// implementations, so this is idempotent and safe on every exit path.
pub fn resume_all_shortcuts(app: &AppHandle) {
    let settings = get_settings(app);
    for (id, binding) in &settings.bindings {
        if !registered_now(id, &settings) {
            continue;
        }
        for registered in registrations(binding) {
            if let Err(e) = register_shortcut(app, registered) {
                debug!("resume_all_shortcuts: could not register '{}': {}", id, e);
            }
        }
    }
}

/// Temporarily unregister all bindings while the user is recording a
/// shortcut in the UI. This avoids firing actions while keys are recorded.
#[tauri::command]
#[specta::specta]
pub fn suspend_all_bindings(app: AppHandle) -> Result<(), String> {
    suspend_all_shortcuts(&app);
    Ok(())
}

/// Re-register all bindings after the user has finished recording.
#[tauri::command]
#[specta::specta]
pub fn resume_all_bindings(app: AppHandle) -> Result<(), String> {
    resume_all_shortcuts(&app);
    Ok(())
}

// ============================================================================
// Keyboard Implementation Switching
// ============================================================================

/// Result of changing keyboard implementation
#[derive(Serialize, Type)]
pub struct ImplementationChangeResult {
    pub success: bool,
    /// List of binding IDs that were reset to defaults due to incompatibility
    pub reset_bindings: Vec<String>,
}

/// Change the keyboard implementation with runtime switching.
/// This will unregister all shortcuts from the old implementation,
/// validate shortcuts for the new implementation (resetting invalid ones to defaults),
/// and register them with the new implementation.
#[tauri::command]
#[specta::specta]
pub fn change_keyboard_implementation_setting(
    app: AppHandle,
    implementation: String,
) -> Result<ImplementationChangeResult, String> {
    let current_settings = settings::get_settings(&app);
    let current_impl = current_settings.keyboard_implementation;
    let new_impl = parse_keyboard_implementation(&implementation);

    // If same implementation, nothing to do
    if current_impl == new_impl {
        return Ok(ImplementationChangeResult {
            success: true,
            reset_bindings: vec![],
        });
    }

    info!(
        "Switching keyboard implementation from {:?} to {:?}",
        current_impl, new_impl
    );

    // Unregister all shortcuts from the current implementation
    unregister_all_shortcuts(&app, current_impl);

    // Update the setting
    let mut settings = settings::get_settings(&app);
    settings.keyboard_implementation = new_impl;
    settings::write_settings(&app, settings);

    // Carbon fallback registrations use the Tauri plugin. Remove them before
    // registering the full Tauri implementation to avoid duplicate conflicts.
    if new_impl == KeyboardImplementation::Tauri {
        crate::secure_input::reconcile_fallback(&app);
    }

    // Initialize new implementation if needed (FalaKeys needs state)
    if new_impl == KeyboardImplementation::FalaKeys && initialize_fala_keys_with_rollback(&app)? {
        // Shortcuts already registered during init.
        crate::secure_input::reconcile_fallback(&app);
        return Ok(ImplementationChangeResult {
            success: true,
            reset_bindings: vec![],
        });
    }

    // Register all shortcuts with new implementation, resetting invalid ones
    let reset_bindings = register_all_shortcuts_for_implementation(&app, new_impl);
    crate::secure_input::reconcile_fallback(&app);

    // Emit event to notify frontend of the change
    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "keyboard_implementation",
            "value": implementation,
            "reset_bindings": reset_bindings
        }),
    );

    info!("Keyboard implementation switched to {:?}", new_impl);

    Ok(ImplementationChangeResult {
        success: true,
        reset_bindings,
    })
}

/// Get the current keyboard implementation
#[tauri::command]
#[specta::specta]
pub fn get_keyboard_implementation(app: AppHandle) -> String {
    let settings = settings::get_settings(&app);
    match settings.keyboard_implementation {
        KeyboardImplementation::Tauri => "tauri".to_string(),
        KeyboardImplementation::FalaKeys => "fala_keys".to_string(),
    }
}

// ============================================================================
// Validation Helpers
// ============================================================================

/// Validate a shortcut for a specific implementation
fn validate_shortcut_for_implementation(
    raw: &str,
    implementation: KeyboardImplementation,
) -> Result<(), String> {
    match implementation {
        KeyboardImplementation::Tauri => tauri_impl::validate_shortcut(raw),
        KeyboardImplementation::FalaKeys => fala_keys::validate_shortcut(raw),
    }
}

/// Parse a keyboard implementation string into the enum
fn parse_keyboard_implementation(s: &str) -> KeyboardImplementation {
    match s {
        "tauri" => KeyboardImplementation::Tauri,
        "fala_keys" => KeyboardImplementation::FalaKeys,
        other => {
            warn!(
                "Invalid keyboard implementation '{}', defaulting to tauri",
                other
            );
            KeyboardImplementation::Tauri
        }
    }
}

/// Unregister all shortcuts for the current implementation
fn unregister_all_shortcuts(app: &AppHandle, implementation: KeyboardImplementation) {
    let bindings = settings::get_bindings(app);

    for (id, binding) in bindings {
        // Skip cancel shortcut as it's dynamically registered
        if id == "cancel" {
            continue;
        }

        for registered in registrations(&binding) {
            let result = match implementation {
                KeyboardImplementation::Tauri => tauri_impl::unregister_shortcut(app, registered),
                KeyboardImplementation::FalaKeys => fala_keys::unregister_shortcut(app, registered),
            };

            if let Err(e) = result {
                warn!(
                    "Failed to unregister shortcut '{}' during switch: {}",
                    id, e
                );
            }
        }
    }
}

/// Register all shortcuts for a specific implementation, validating and resetting invalid ones
fn register_all_shortcuts_for_implementation(
    app: &AppHandle,
    implementation: KeyboardImplementation,
) -> Vec<String> {
    let mut reset_bindings = Vec::new();
    let default_bindings = settings::get_default_settings().bindings;
    let mut current_settings = settings::get_settings(app);

    for (id, default_binding) in &default_bindings {
        // Skip cancel shortcut as it's dynamically registered
        if id == "cancel" {
            continue;
        }

        // Skip post-processing shortcut when the feature is disabled
        if id == "transcribe_with_post_process" && !current_settings.post_process_enabled {
            continue;
        }

        let mut binding = current_settings
            .bindings
            .get(id)
            .cloned()
            .unwrap_or_else(|| default_binding.clone());
        if binding_is_unset(&binding) {
            continue;
        }

        // Validate the shortcut for the target implementation
        if let Err(e) =
            validate_shortcut_for_implementation(&binding.current_binding, implementation)
        {
            info!(
                "Shortcut '{}' ({}) is invalid for {:?}: {}. Resetting to default.",
                id, binding.current_binding, implementation, e
            );

            // Reset to default
            binding.current_binding = default_binding.current_binding.clone();
            current_settings
                .bindings
                .insert(id.clone(), binding.clone());
            reset_bindings.push(id.clone());
        }

        // A second combination the target cannot register is dropped
        if !binding.alternate_binding.trim().is_empty() {
            if let Err(e) =
                validate_shortcut_for_implementation(&binding.alternate_binding, implementation)
            {
                info!(
                    "Second shortcut '{}' ({}) is invalid for {:?}: {}. Removing it.",
                    id, binding.alternate_binding, implementation, e
                );
                binding.alternate_binding.clear();
                current_settings
                    .bindings
                    .insert(id.clone(), binding.clone());
                if !reset_bindings.contains(id) {
                    reset_bindings.push(id.clone());
                }
            }
        }
        // A first combination reset to no key gives way to the second; one reset to the second's
        // key keeps it once.
        if binding_is_unset(&binding) || binding.alternate_binding == binding.current_binding {
            binding.current_binding = std::mem::take(&mut binding.alternate_binding);
            current_settings
                .bindings
                .insert(id.clone(), binding.clone());
        }

        // Register with the appropriate implementation
        for registered in registrations(&binding) {
            let result = match implementation {
                KeyboardImplementation::Tauri => tauri_impl::register_shortcut(app, registered),
                KeyboardImplementation::FalaKeys => fala_keys::register_shortcut(app, registered),
            };

            if let Err(e) = result {
                error!(
                    "Failed to register shortcut '{}' for {:?}: {}",
                    id, implementation, e
                );
            }
        }
    }

    // Save settings if any bindings were reset
    if !reset_bindings.is_empty() {
        settings::write_settings(app, current_settings);
    }

    reset_bindings
}

/// Initialize FalaKeys if not already initialized, with rollback on failure
fn initialize_fala_keys_with_rollback(app: &AppHandle) -> Result<bool, String> {
    if app.try_state::<fala_keys::FalaKeysState>().is_some() {
        return Ok(false); // Already initialized, caller should continue
    }

    if let Err(e) = fala_keys::init_shortcuts(app) {
        error!("Failed to initialize FalaKeys: {}", e);
        // Rollback to Tauri
        let mut settings = settings::get_settings(app);
        settings.keyboard_implementation = KeyboardImplementation::Tauri;
        settings::write_settings(app, settings);
        crate::secure_input::reconcile_fallback(app);
        tauri_impl::init_shortcuts(app);
        return Err(format!(
            "Failed to initialize FalaKeys: {}. Reverted to Tauri.",
            e
        ));
    }

    // init_shortcuts already registered shortcuts
    Ok(true)
}

// ============================================================================
// General Settings Commands
// ============================================================================

#[tauri::command]
#[specta::specta]
pub fn change_shortcut_activation_setting(
    app: AppHandle,
    activation: ShortcutActivation,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.shortcut_activation = activation;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_hold_threshold_ms_setting(app: AppHandle, ms: u64) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.hold_threshold_ms = ms;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_audio_feedback_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.audio_feedback = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_audio_feedback_volume_setting(app: AppHandle, volume: f32) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.audio_feedback_volume = volume;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_sound_theme_setting(app: AppHandle, theme: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let parsed = match theme.as_str() {
        "marimba" => SoundTheme::Marimba,
        "pop" => SoundTheme::Pop,
        "custom" => SoundTheme::Custom,
        other => {
            warn!("Invalid sound theme '{}', defaulting to marimba", other);
            SoundTheme::Marimba
        }
    };
    settings.sound_theme = parsed;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_theme_setting(app: AppHandle, theme: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let parsed = match theme.as_str() {
        "system" => Theme::System,
        "light" => Theme::Light,
        "dark" => Theme::Dark,
        other => {
            warn!("Invalid theme '{}', defaulting to system", other);
            Theme::System
        }
    };
    settings.theme = parsed;
    settings::write_settings(&app, settings);
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    apply_window_theme(&app, parsed);
    // Notify other webviews (the recording overlay) so they re-apply the palette
    // live — they set `data-theme` on their own document and can't see this one.
    let _ = app.emit("theme-changed", parsed);
    Ok(())
}

/// Applies the appearance setting to the native window chrome (title bar), which
/// CSS `data-theme` cannot reach. `System` clears the override so the window
/// follows the OS. Call this on startup and whenever the setting changes to keep
/// the title bar in sync with the in-app palette.
///
/// On Windows this themes the title bar only. On macOS `set_theme` sets
/// `NSApp.appearance` app-wide, which is what we want here: it darkens the title
/// bar and keeps the overlay in step. Linux is left to `data-theme` alone, since
/// its window theming is backend-dependent and unreliable.
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn apply_window_theme(app: &AppHandle, theme: Theme) {
    let window_theme = match theme {
        Theme::System => None,
        Theme::Light => Some(tauri::Theme::Light),
        Theme::Dark => Some(tauri::Theme::Dark),
    };
    if let Some(window) = app.get_webview_window("main") {
        if let Err(e) = window.set_theme(window_theme) {
            warn!("Failed to apply window theme: {}", e);
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn change_translate_to_english_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.translate_to_english = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_selected_language_setting(app: AppHandle, language: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.selected_language = language.clone();
    settings::write_settings(&app, settings);

    // The tray and the settings window both change it; keep the other in sync.
    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "selected_language",
            "value": language
        }),
    );
    crate::tray::update_tray_menu(&app);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_overlay_position_setting(app: AppHandle, position: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let parsed = match position.as_str() {
        // "none" is retired (visibility is overlay_style now); fold legacy callers
        // onto Bottom rather than warn.
        "none" | "bottom" => OverlayPosition::Bottom,
        "top" => OverlayPosition::Top,
        other => {
            warn!("Invalid overlay position '{}', defaulting to bottom", other);
            OverlayPosition::Bottom
        }
    };
    settings.overlay_position = parsed;
    // Choosing the edge puts the pill back at its center, wherever it was dragged.
    settings.pill_center_x = None;
    settings::write_settings(&app, settings);
    crate::overlay::set_pill_center(None);

    // Whether the overlay shows at all is owned by overlay_style now; position
    // only ever toggles Top/Bottom, so the enabled cache is untouched here.
    // Update overlay position without recreating window
    crate::utils::update_overlay_position(&app);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_overlay_style_setting(app: AppHandle, style: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let parsed = match style.as_str() {
        "none" => OverlayStyle::None,
        "minimal" => OverlayStyle::Minimal,
        "live" => OverlayStyle::Live,
        other => {
            warn!("Invalid overlay style '{}', defaulting to minimal", other);
            OverlayStyle::Minimal
        }
    };
    settings.overlay_style = parsed;
    settings::write_settings(&app, settings);

    // Keep the cached overlay-enabled flag in sync so emit_levels stops (or
    // resumes) emitting on the next audio callback.
    crate::overlay::update_overlay_enabled_cache(parsed != OverlayStyle::None);

    // Reposition in case the window needs to re-center for the new style.
    crate::utils::update_overlay_position(&app);

    // The idle pill exists only while a pill style is on.
    crate::overlay::apply_idle_setting(&app, &settings::get_settings(&app));

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_pill_idle_visible_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.pill_idle_visible = enabled;
    settings::write_settings(&app, settings.clone());
    crate::overlay::end_idle_snooze();
    crate::overlay::apply_idle_setting(&app, &settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_debug_mode_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.debug_mode = enabled;
    settings::write_settings(&app, settings);

    // Keep webview log streaming in sync: the live log viewer only exists in
    // debug mode, so logs are forwarded to the frontend only while it is on.
    crate::WEBVIEW_LOG_STREAMING.store(enabled, std::sync::atomic::Ordering::Relaxed);

    // Emit event to notify frontend of debug mode change
    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "debug_mode",
            "value": enabled
        }),
    );

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_start_hidden_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.start_hidden = enabled;
    settings::write_settings(&app, settings);

    // Notify frontend
    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "start_hidden",
            "value": enabled
        }),
    );

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_autostart_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.autostart_enabled = enabled;
    settings::write_settings(&app, settings);

    // Apply the autostart setting immediately
    crate::autostart::apply_autostart(&app, enabled);

    // Notify frontend
    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "autostart_enabled",
            "value": enabled
        }),
    );

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_update_checks_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    if settings::update_checks_forced_disabled() {
        return Err(
            "Update checks are disabled by system configuration (FALA_DISABLE_UPDATER)".into(),
        );
    }

    let mut settings = settings::get_settings(&app);
    settings.update_checks_enabled = enabled;
    settings::write_settings(&app, settings);

    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "update_checks_enabled",
            "value": enabled
        }),
    );

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_show_whats_new_on_update_setting(
    app: AppHandle,
    enabled: bool,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.show_whats_new_on_update = enabled;
    settings::write_settings(&app, settings);

    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "show_whats_new_on_update",
            "value": enabled
        }),
    );

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_whats_new_last_seen_version_setting(
    app: AppHandle,
    version: String,
) -> Result<(), String> {
    let version = version.trim().to_string();
    let mut settings = settings::get_settings(&app);
    settings.whats_new_last_seen_version = version.clone();
    settings::write_settings(&app, settings);

    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "whats_new_last_seen_version",
            "value": version
        }),
    );

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn update_custom_words(app: AppHandle, words: Vec<String>) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.custom_words = crate::llm_auto::normalize_words(words);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_llm_enabled_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.llm_enabled = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_cleanup_level_setting(
    app: AppHandle,
    level: settings::CleanupLevel,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.cleanup_level = level;
    settings::write_settings(&app, settings);
    Ok(())
}

/// Stores the list in the form `fala-inject` reports app names and returns what was stored.
#[tauri::command]
#[specta::specta]
pub fn change_llm_disabled_apps_setting(
    app: AppHandle,
    apps: Vec<String>,
) -> Result<Vec<String>, String> {
    let mut settings = settings::get_settings(&app);
    settings.llm_disabled_apps = crate::llm_auto::normalize_apps(apps);
    let stored = settings.llm_disabled_apps.clone();
    settings::write_settings(&app, settings);
    Ok(stored)
}

#[tauri::command]
#[specta::specta]
pub fn change_word_correction_threshold_setting(
    app: AppHandle,
    threshold: f64,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.word_correction_threshold = threshold;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_extra_recording_buffer_setting(app: AppHandle, ms: u64) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.extra_recording_buffer_ms = ms;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_paste_delay_ms_setting(app: AppHandle, ms: u64) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.paste_delay_ms = ms;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_paste_delay_after_ms_setting(app: AppHandle, ms: u64) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.paste_delay_after_ms = ms;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_reliable_paste_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.reliable_paste = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_paste_method_setting(app: AppHandle, method: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let parsed = match method.as_str() {
        "ctrl_v" => PasteMethod::CtrlV,
        "direct" => PasteMethod::Direct,
        "none" => PasteMethod::None,
        "shift_insert" => PasteMethod::ShiftInsert,
        "ctrl_shift_v" => PasteMethod::CtrlShiftV,
        "external_script" => PasteMethod::ExternalScript,
        other => {
            warn!("Invalid paste method '{}', defaulting to ctrl_v", other);
            PasteMethod::CtrlV
        }
    };
    settings.paste_method = parsed;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_available_typing_tools() -> Vec<String> {
    #[cfg(target_os = "linux")]
    {
        crate::clipboard::get_available_typing_tools()
    }
    #[cfg(not(target_os = "linux"))]
    {
        vec!["auto".to_string()]
    }
}

#[tauri::command]
#[specta::specta]
pub fn change_typing_tool_setting(app: AppHandle, tool: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let parsed = match tool.as_str() {
        "auto" => TypingTool::Auto,
        "wtype" => TypingTool::Wtype,
        "kwtype" => TypingTool::Kwtype,
        "dotool" => TypingTool::Dotool,
        "ydotool" => TypingTool::Ydotool,
        "xdotool" => TypingTool::Xdotool,
        other => {
            warn!("Invalid typing tool '{}', defaulting to auto", other);
            TypingTool::Auto
        }
    };
    settings.typing_tool = parsed;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_external_script_path_setting(
    app: AppHandle,
    path: Option<String>,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.external_script_path = path;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_clipboard_handling_setting(app: AppHandle, handling: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let parsed = match handling.as_str() {
        "dont_modify" => ClipboardHandling::DontModify,
        "copy_to_clipboard" => ClipboardHandling::CopyToClipboard,
        other => {
            warn!(
                "Invalid clipboard handling '{}', defaulting to dont_modify",
                other
            );
            ClipboardHandling::DontModify
        }
    };
    settings.clipboard_handling = parsed;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_auto_submit_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.auto_submit = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_auto_submit_key_setting(app: AppHandle, key: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let parsed = match key.as_str() {
        "enter" => AutoSubmitKey::Enter,
        "ctrl_enter" => AutoSubmitKey::CtrlEnter,
        "cmd_enter" => AutoSubmitKey::CmdEnter,
        other => {
            warn!("Invalid auto submit key '{}', defaulting to enter", other);
            AutoSubmitKey::Enter
        }
    };
    settings.auto_submit_key = parsed;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_post_process_enabled_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.post_process_enabled = enabled;
    settings::write_settings(&app, settings.clone());

    // Register or unregister the post-processing shortcut
    if let Some(binding) = settings
        .bindings
        .get("transcribe_with_post_process")
        .cloned()
    {
        for registered in registrations(&binding) {
            if enabled {
                let _ = register_shortcut(&app, registered);
            } else {
                let _ = unregister_shortcut(&app, registered);
            }
        }
    }

    crate::secure_input::reconcile_fallback(&app);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_experimental_enabled_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.experimental_enabled = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_post_process_base_url_setting(
    app: AppHandle,
    provider_id: String,
    base_url: String,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let label = settings
        .post_process_provider(&provider_id)
        .map(|provider| provider.label.clone())
        .ok_or_else(|| format!("Provider '{}' not found", provider_id))?;

    let provider = settings
        .post_process_provider_mut(&provider_id)
        .expect("Provider looked up above must exist");

    if provider.id != "custom" {
        return Err(format!(
            "Provider '{}' does not allow editing the base URL",
            label
        ));
    }

    provider.base_url = base_url;
    settings::write_settings(&app, settings);
    Ok(())
}

/// Generic helper to validate provider exists
fn validate_provider_exists(
    settings: &settings::AppSettings,
    provider_id: &str,
) -> Result<(), String> {
    if !settings
        .post_process_providers
        .iter()
        .any(|provider| provider.id == provider_id)
    {
        return Err(format!("Provider '{}' not found", provider_id));
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_post_process_api_key_setting(
    app: AppHandle,
    provider_id: String,
    api_key: String,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    validate_provider_exists(&settings, &provider_id)?;
    settings.post_process_api_keys.insert(provider_id, api_key);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_post_process_model_setting(
    app: AppHandle,
    provider_id: String,
    model: String,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    validate_provider_exists(&settings, &provider_id)?;
    settings.post_process_models.insert(provider_id, model);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn set_post_process_provider(app: AppHandle, provider_id: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    validate_provider_exists(&settings, &provider_id)?;
    settings.post_process_provider_id = provider_id;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn add_post_process_prompt(
    app: AppHandle,
    name: String,
    prompt: String,
) -> Result<LLMPrompt, String> {
    let mut settings = settings::get_settings(&app);

    // Generate unique ID using timestamp and random component
    let id = format!("prompt_{}", chrono::Utc::now().timestamp_millis());

    let new_prompt = LLMPrompt {
        id: id.clone(),
        name,
        prompt,
    };

    settings.post_process_prompts.push(new_prompt.clone());
    settings::write_settings(&app, settings);

    Ok(new_prompt)
}

#[tauri::command]
#[specta::specta]
pub fn update_post_process_prompt(
    app: AppHandle,
    id: String,
    name: String,
    prompt: String,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);

    if let Some(existing_prompt) = settings
        .post_process_prompts
        .iter_mut()
        .find(|p| p.id == id)
    {
        existing_prompt.name = name;
        existing_prompt.prompt = prompt;
        settings::write_settings(&app, settings);
        Ok(())
    } else {
        Err(format!("Prompt with id '{}' not found", id))
    }
}

#[tauri::command]
#[specta::specta]
pub fn delete_post_process_prompt(app: AppHandle, id: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);

    // Don't allow deleting the last prompt
    if settings.post_process_prompts.len() <= 1 {
        return Err("Cannot delete the last prompt".to_string());
    }

    // Find and remove the prompt
    let original_len = settings.post_process_prompts.len();
    settings.post_process_prompts.retain(|p| p.id != id);

    if settings.post_process_prompts.len() == original_len {
        return Err(format!("Prompt with id '{}' not found", id));
    }

    // If the deleted prompt was selected, select the first one or None
    if settings.post_process_selected_prompt_id.as_ref() == Some(&id) {
        settings.post_process_selected_prompt_id =
            settings.post_process_prompts.first().map(|p| p.id.clone());
    }

    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn fetch_post_process_models(
    app: AppHandle,
    provider_id: String,
) -> Result<Vec<String>, String> {
    let settings = settings::get_settings(&app);

    // Find the provider
    let provider = settings
        .post_process_providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("Provider '{}' not found", provider_id))?;

    if provider.id == APPLE_INTELLIGENCE_PROVIDER_ID {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            return Ok(vec![APPLE_INTELLIGENCE_DEFAULT_MODEL_ID.to_string()]);
        }

        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            return Err("Apple Intelligence is only available on Apple silicon Macs running macOS 15 or later.".to_string());
        }
    }

    // Get API key
    let api_key = settings
        .post_process_api_keys
        .get(&provider_id)
        .cloned()
        .unwrap_or_default();

    // Skip fetching if no API key for providers that typically need one
    if api_key.trim().is_empty() && provider.id != "custom" {
        return Err(format!(
            "API key is required for {}. Please add an API key to list available models.",
            provider.label
        ));
    }

    crate::llm_client::fetch_models(provider, api_key).await
}

#[tauri::command]
#[specta::specta]
pub fn set_post_process_selected_prompt(app: AppHandle, id: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);

    // Verify the prompt exists
    if !settings.post_process_prompts.iter().any(|p| p.id == id) {
        return Err(format!("Prompt with id '{}' not found", id));
    }

    settings.post_process_selected_prompt_id = Some(id);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_mute_while_recording_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.mute_while_recording = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_pause_media_while_recording_setting(
    app: AppHandle,
    enabled: bool,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.pause_media_while_recording = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_pause_media_on_meeting_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.pause_media_on_meeting = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_append_trailing_space_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.append_trailing_space = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_lazy_stream_close_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.lazy_stream_close = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_vad_enabled_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.vad_enabled = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_filler_word_removal_enabled_setting(
    app: AppHandle,
    enabled: bool,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.filler_word_removal_enabled = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_app_language_setting(app: AppHandle, language: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.app_language = language.clone();
    settings::write_settings(&app, settings);

    // Refresh the tray menu with the new language
    tray::update_tray_menu(&app);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_show_tray_icon_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.show_tray_icon = enabled;
    settings::write_settings(&app, settings);

    // Apply change immediately
    tray::set_tray_visibility(&app, enabled);

    Ok(())
}

/// Save accelerator settings and make the next model use reload with them.
/// The currently running transcription, if any, keeps its existing engine.
fn save_accelerator_and_reload_next_use(app: &AppHandle, s: settings::AppSettings) {
    settings::write_settings(app, s);

    let tm = app.state::<std::sync::Arc<crate::managers::transcription::TranscriptionManager>>();
    tm.reload_model_on_next_use();
}

#[tauri::command]
#[specta::specta]
pub fn change_transcribe_accelerator_setting(
    app: AppHandle,
    accelerator: settings::TranscribeAcceleratorSetting,
) -> Result<(), String> {
    let mut s = settings::get_settings(&app);
    s.transcribe_accelerator = accelerator;
    save_accelerator_and_reload_next_use(&app, s);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_ort_accelerator_setting(
    app: AppHandle,
    accelerator: settings::OrtAcceleratorSetting,
) -> Result<(), String> {
    let mut s = settings::get_settings(&app);
    s.ort_accelerator = accelerator;
    save_accelerator_and_reload_next_use(&app, s);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_transcribe_gpu_device(app: AppHandle, device: Option<String>) -> Result<(), String> {
    let mut s = settings::get_settings(&app);
    s.transcribe_gpu_device = device;
    save_accelerator_and_reload_next_use(&app, s);
    Ok(())
}

/// Return which accelerators and GPU devices are available for this build.
///
/// First-call cost is dominated by enumerating GPU devices through the
/// transcribe.cpp Metal/Vulkan backend, which loads dynamic libraries and
/// probes hardware. Run it on the blocking pool so the webview thread
/// stays responsive — see also the startup pre-warm in `lib.rs`.
#[tauri::command]
#[specta::specta]
pub async fn get_available_accelerators() -> crate::managers::transcription::AvailableAccelerators {
    tauri::async_runtime::spawn_blocking(crate::managers::transcription::get_available_accelerators)
        .await
        .expect("get_available_accelerators panicked")
}

#[cfg(test)]
mod tests {
    use super::{
        defaults_restored, place_binding, registrations, reset_refusal, slot_removable,
        without_slot, BindingSlot, CancelKey, ResetBindingsError, ShortcutOverlap, ShortcutRefusal,
    };
    use crate::meeting::MeetingPhase;
    use crate::settings::ShortcutBinding;
    use handy_keys::Hotkey;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use tauri_plugin_global_shortcut::Shortcut;

    /// A fake backend that records each call (`true` register, `false` unregister),
    /// the bindings it was asked to unregister, and whether a key is registered.
    #[derive(Default)]
    struct FakeBackend {
        calls: RefCell<Vec<bool>>,
        unregistered: RefCell<Vec<String>>,
        registered: RefCell<bool>,
    }

    impl FakeBackend {
        fn register(&self, binding: &str) -> Result<String, String> {
            self.calls.borrow_mut().push(true);
            *self.registered.borrow_mut() = true;
            Ok(binding.to_string())
        }

        fn unregister(&self, binding: &str) -> Result<(), String> {
            self.calls.borrow_mut().push(false);
            self.unregistered.borrow_mut().push(binding.to_string());
            *self.registered.borrow_mut() = false;
            Ok(())
        }

        fn reconcile(&self, key: &CancelKey<String>, binding: &str) {
            key.reconcile(|| self.register(binding), |b| self.unregister(b));
        }
    }

    #[test]
    fn cancel_key_ends_unregistered_when_passes_run_out_of_order() {
        let key = CancelKey::new();
        let backend = FakeBackend::default();
        // A short dictation: start and stop request synchronously, in order...
        key.request(true);
        key.request(false);
        // ...but the stop's pass runs first and the start's pass second.
        backend.reconcile(&key, "escape");
        backend.reconcile(&key, "escape");
        assert!(!*backend.registered.borrow());
        assert!(backend.calls.borrow().is_empty());
    }

    #[test]
    fn cancel_key_follows_the_latest_request() {
        let key = CancelKey::new();
        let backend = FakeBackend::default();
        key.request(true);
        backend.reconcile(&key, "escape");
        assert!(*backend.registered.borrow());
        // A second pass for the same request does not register twice.
        backend.reconcile(&key, "escape");
        key.request(false);
        backend.reconcile(&key, "escape");
        assert!(!*backend.registered.borrow());
        assert_eq!(*backend.calls.borrow(), vec![true, false]);
    }

    #[test]
    fn cancel_key_retries_after_a_failed_registration() {
        let key = CancelKey::new();
        let backend = FakeBackend::default();
        key.request(true);
        key.reconcile(|| Err("busy".to_string()), |_| Ok(()));
        backend.reconcile(&key, "escape");
        assert!(*backend.registered.borrow());
        assert_eq!(*backend.calls.borrow(), vec![true]);
    }

    #[test]
    fn cancel_key_unregisters_the_binding_it_registered() {
        let key = CancelKey::new();
        let backend = FakeBackend::default();
        key.request(true);
        backend.reconcile(&key, "escape");
        // The cancel binding is edited mid-dictation: the stop still releases Esc.
        key.request(false);
        backend.reconcile(&key, "ctrl+q");
        assert!(!*backend.registered.borrow());
        assert_eq!(*backend.unregistered.borrow(), vec!["escape".to_string()]);
    }

    #[test]
    fn cancel_key_retries_after_a_failed_unregistration() {
        let key = CancelKey::new();
        let backend = FakeBackend::default();
        key.request(true);
        backend.reconcile(&key, "escape");
        key.request(false);
        key.reconcile(|| Ok("escape".to_string()), |_| Err("busy".to_string()));
        assert!(*backend.registered.borrow());
        backend.reconcile(&key, "escape");
        assert!(!*backend.registered.borrow());
        assert_eq!(*backend.calls.borrow(), vec![true, false]);
    }

    #[test]
    fn empty_binding_is_unset() {
        let binding = |current: &str| crate::settings::ShortcutBinding {
            id: "transcribe_with_post_process".to_string(),
            name: String::new(),
            description: String::new(),
            default_binding: String::new(),
            current_binding: current.to_string(),
            alternate_binding: String::new(),
        };
        assert!(super::binding_is_unset(&binding("")));
        assert!(super::binding_is_unset(&binding("  ")));
        assert!(!super::binding_is_unset(&binding("ctrl+space")));
    }

    #[test]
    fn compound_shortcut_keys_parse_on_both_backends() {
        for key in [
            "scrolllock",
            "capslock",
            "numlock",
            "pageup",
            "pagedown",
            "printscreen",
        ] {
            assert!(key.parse::<Shortcut>().is_ok(), "Tauri rejected {key}");
            assert!(key.parse::<Hotkey>().is_ok(), "FalaKeys rejected {key}");
        }
    }

    fn bound(id: &str, default: &str, primary: &str, alternate: &str) -> ShortcutBinding {
        ShortcutBinding {
            id: id.to_string(),
            name: String::new(),
            description: String::new(),
            default_binding: default.to_string(),
            current_binding: primary.to_string(),
            alternate_binding: alternate.to_string(),
        }
    }

    fn keys(bindings: &HashMap<String, ShortcutBinding>, id: &str) -> (String, String) {
        let b = &bindings[id];
        (b.current_binding.clone(), b.alternate_binding.clone())
    }

    fn pair(primary: &str, alternate: &str) -> (String, String) {
        (primary.to_string(), alternate.to_string())
    }

    fn store() -> HashMap<String, ShortcutBinding> {
        [
            bound("transcribe", "ctrl+shift+space", "ctrl+shift+space", ""),
            bound("cancel", "escape", "escape", ""),
            bound("paste_last", "", "alt+shift+z", ""),
            bound("copy_last", "", "alt+shift+x", "f8"),
            bound("meeting_toggle", "", "", ""),
        ]
        .into_iter()
        .map(|b| (b.id.clone(), b))
        .collect()
    }

    fn place(
        bindings: &HashMap<String, ShortcutBinding>,
        id: &str,
        slot: BindingSlot,
        raw: &str,
    ) -> HashMap<String, ShortcutBinding> {
        take(bindings, id, slot, raw, false).expect("no conflict").0
    }

    type Placed = (HashMap<String, ShortcutBinding>, Option<ShortcutOverlap>);

    fn take(
        bindings: &HashMap<String, ShortcutBinding>,
        id: &str,
        slot: BindingSlot,
        raw: &str,
        take_over: bool,
    ) -> Result<Placed, ShortcutRefusal> {
        place_binding(bindings, bindings[id].clone(), slot, raw, take_over)
    }

    #[test]
    fn a_combination_in_use_is_refused_or_taken_over() {
        use BindingSlot::{Alternate as Alt, Primary as Pri};
        let all = store();
        let in_use = |action: &str, take_over| ShortcutRefusal::InUse {
            action: action.to_string(),
            slot: Pri,
            take_over,
        };
        // Dictation's only key: refused, and "use it here" cannot take it.
        let refused = take(&all, "paste_last", Alt, "ctrl+shift+space", false);
        assert_eq!(refused.err(), Some(in_use("transcribe", false)));
        assert!(take(&all, "paste_last", Alt, "ctrl+shift+space", true).is_err());
        // Copy's first key can be taken: copy keeps its second, now first.
        let refused = take(&all, "paste_last", Alt, "alt+shift+x", false);
        assert_eq!(refused.err(), Some(in_use("copy_last", true)));
        let (next, _) = take(&all, "paste_last", Alt, "alt+shift+x", true).unwrap();
        assert_eq!(
            keys(&next, "paste_last"),
            pair("alt+shift+z", "alt+shift+x")
        );
        assert_eq!(keys(&next, "copy_last"), pair("f8", ""));
        // With a second key, dictation can give its first one away.
        let mut two = store();
        two.get_mut("transcribe").unwrap().alternate_binding = "f9".to_string();
        let (next, _) = take(&two, "meeting_toggle", Pri, "ctrl+shift+space", true).unwrap();
        assert_eq!(keys(&next, "transcribe"), pair("f9", ""));
        assert_eq!(keys(&next, "meeting_toggle"), pair("ctrl+shift+space", ""));
    }

    #[test]
    fn second_combination_registers_under_its_own_id() {
        let ids = |b: &ShortcutBinding| {
            registrations(b)
                .into_iter()
                .map(|r| (r.id, r.current_binding))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            ids(&bound("copy_last", "", "alt+shift+x", "f8")),
            vec![
                ("copy_last".to_string(), "alt+shift+x".to_string()),
                ("copy_last#2".to_string(), "f8".to_string()),
            ]
        );
        assert!(ids(&bound("meeting_toggle", "", "", "")).is_empty());
        // Cancel has one combination only.
        assert_eq!(
            ids(&bound("cancel", "escape", "escape", "f8")),
            vec![("cancel".to_string(), "escape".to_string())]
        );
    }

    #[test]
    fn a_combination_goes_into_its_slot() {
        let next = place(&store(), "paste_last", BindingSlot::Alternate, "f7");
        assert_eq!(keys(&next, "paste_last"), pair("alt+shift+z", "f7"));
        let next = place(&next, "paste_last", BindingSlot::Primary, "f6");
        assert_eq!(keys(&next, "paste_last"), pair("f6", "f7"));
        // A second combination without a first becomes the first.
        let next = place(
            &store(),
            "meeting_toggle",
            BindingSlot::Alternate,
            "alt+shift+m",
        );
        assert_eq!(keys(&next, "meeting_toggle"), pair("alt+shift+m", ""));
    }

    #[test]
    fn cancel_and_dictation_keep_a_key() {
        let all = store();
        assert!(!slot_removable(&all["cancel"], BindingSlot::Primary));
        assert!(!slot_removable(&all["transcribe"], BindingSlot::Primary));
        assert!(slot_removable(&all["paste_last"], BindingSlot::Primary));
        let mut two = all["transcribe"].clone();
        two.alternate_binding = "f9".to_string();
        assert!(slot_removable(&two, BindingSlot::Primary));
        assert!(slot_removable(&two, BindingSlot::Alternate));
        // Removing the first promotes the second.
        let removed = without_slot(&all["copy_last"], BindingSlot::Primary);
        assert_eq!(
            (removed.current_binding, removed.alternate_binding),
            pair("f8", "")
        );
    }

    #[test]
    fn restoring_defaults_is_refused_while_dictating_or_recording() {
        assert_eq!(
            reset_refusal(true, MeetingPhase::Idle),
            Some(ResetBindingsError::DictationActive)
        );
        for phase in [MeetingPhase::Recording, MeetingPhase::Paused] {
            assert_eq!(
                reset_refusal(false, phase),
                Some(ResetBindingsError::MeetingRecording)
            );
        }
        for phase in [
            MeetingPhase::Idle,
            MeetingPhase::Stopping,
            MeetingPhase::Processing,
        ] {
            assert_eq!(reset_refusal(false, phase), None);
        }

        let mut all = store();
        all.get_mut("transcribe").unwrap().current_binding = "ctrl_left+space".to_string();
        all.get_mut("transcribe").unwrap().alternate_binding = "f9".to_string();
        // An old store's outdated default loses to this version's.
        all.get_mut("transcribe").unwrap().default_binding = "ctrl+space".to_string();
        let restored = defaults_restored(&all, &store());
        assert_eq!(keys(&restored, "transcribe"), pair("ctrl+shift+space", ""));
        assert_eq!(keys(&restored, "copy_last"), pair("", ""));
        assert_eq!(keys(&restored, "cancel"), pair("escape", ""));
        assert_eq!(keys(&restored, "meeting_toggle"), pair("", ""));
    }
}
