//! The call prompt (phase 2, F8 `meeting-detection`): "Parece uma chamada no Teams. Gravar
//! reunião?" when a call app opens the mic, and "Parar a gravação?" when it lets go.
//!
//! The rules live in `fala_meeting::CallDetector`; [`watch`] feeds it the mic holders from
//! `fala_audio`. This module only shows the card and runs the answer. ADR-0005: the prompt never
//! records by itself. [Gravar] goes through [`MeetingManager::start_from_tray`], the same
//! explicit path as the tray item (the third-party notice comes first), and [Parar] through
//! [`MeetingManager::stop`]. Expiring, [Agora não] and [Continuar] only close the card.

pub(crate) mod watch;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use fala_meeting::{CallApp, CallPrompt, CALL_APPS};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindowBuilder};

use crate::meeting::MeetingManager;
use crate::settings;

const LABEL: &str = "call_prompt";
/// The card, in logical points (Granola's 404 × 104).
const WIDTH: f64 = 404.0;
const HEIGHT: f64 = 104.0;
/// Distance from the right edge of the work area to the card's left edge, and from its top.
const RIGHT_INSET: f64 = 408.0;
const TOP_INSET: f64 = 4.0;
/// The countdown the card shows; it pauses on hover and closes without recording.
pub const COUNTDOWN: Duration = Duration::from_secs(20);
/// The card goes away after this even if the pointer stayed on it.
const MAX_ON_SCREEN: Duration = Duration::from_secs(120);
/// "Gravou por engano?" stays a minute.
const MISTAKE_COUNTDOWN: Duration = Duration::from_secs(60);
/// Event with the [`CallPromptPayload`], sent to the prompt window.
const PROMPT_EVENT: &str = "call-prompt";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum CallPromptKind {
    /// "Gravar reunião?" with [Gravar] [Agora não] [Não avisar no <app>].
    Offer,
    /// "Parar a gravação?" with [Parar] [Continuar].
    AskStop,
    /// "Quit" from the tray while a meeting records: [Parar e sair] [Cancelar].
    Quit,
    /// "Ainda está gravando?" a minute before the stop for silence: [Continuar] [Parar].
    StillRecording,
    /// "Parou depois de 15 min de silêncio" with [Gravar de novo] [Fechar].
    SilenceStopped,
    /// "Gravou por engano?" after a short or almost silent stop: [Manter] [Descartar].
    Mistake,
    /// "Nada foi gravado": the empty session is already gone.
    NothingRecorded,
}

/// What the card shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CallPromptPayload {
    pub kind: CallPromptKind,
    pub app_id: String,
    pub app_name: String,
    /// The meeting a browser shows ("Google Meet" in "Google Meet no Edge"); `None` for apps.
    pub web_name: Option<String>,
    pub countdown_ms: u32,
    /// The session a "Gravou por engano?" is about.
    pub session_id: Option<String>,
    /// Seconds recorded, for "Gravou por engano?".
    pub seconds: Option<u32>,
}

impl CallPromptPayload {
    fn from_prompt(prompt: CallPrompt) -> Self {
        let (kind, app, web) = match prompt {
            CallPrompt::OfferRecording { app, web } => (CallPromptKind::Offer, app, web),
            CallPrompt::AskStop { app, web } => (CallPromptKind::AskStop, app, web),
        };
        Self {
            kind,
            app_id: app.id.to_string(),
            app_name: app.name.to_string(),
            web_name: web.map(|web| web.name.to_string()),
            countdown_ms: COUNTDOWN.as_millis() as u32,
            session_id: None,
            seconds: None,
        }
    }

    /// A card about the meeting, not a call app.
    fn meeting(kind: CallPromptKind, countdown: Duration) -> Self {
        Self {
            kind,
            app_id: String::new(),
            app_name: String::new(),
            web_name: None,
            countdown_ms: u32::try_from(countdown.as_millis()).unwrap_or(u32::MAX),
            session_id: None,
            seconds: None,
        }
    }

    fn quit() -> Self {
        Self::meeting(CallPromptKind::Quit, COUNTDOWN)
    }
}

/// The person's answer, or the countdown running out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum CallPromptAnswer {
    Record,
    NotNow,
    MuteApp,
    Stop,
    KeepRecording,
    StopAndQuit,
    Cancel,
    RecordAgain,
    Dismiss,
    Keep,
    Discard,
    Expired,
}

/// What an answer does once the card closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AnswerAction {
    /// The explicit "record" click, through the tray's start path (consent first).
    Record,
    Stop,
    StopAndQuit,
    KeepRecording,
    Keep(String),
    Discard(String),
    Mute(String),
    Close,
}

/// The action for `answer` given the card on screen; `None` when the answer does not belong to
/// it (a stale click after the card changed, or no card at all).
pub(crate) fn answer_action(
    current: Option<&CallPromptPayload>,
    answer: CallPromptAnswer,
) -> Option<AnswerAction> {
    use CallPromptAnswer as A;
    let current = current?;
    match (current.kind, answer) {
        (CallPromptKind::Offer, A::Record) => Some(AnswerAction::Record),
        (CallPromptKind::Offer, A::MuteApp) => Some(AnswerAction::Mute(current.app_id.clone())),
        (CallPromptKind::Offer, A::NotNow | A::Expired) => Some(AnswerAction::Close),
        (CallPromptKind::AskStop, A::Stop) => Some(AnswerAction::Stop),
        (CallPromptKind::AskStop, A::KeepRecording | A::Expired) => Some(AnswerAction::Close),
        (CallPromptKind::Quit, A::StopAndQuit) => Some(AnswerAction::StopAndQuit),
        // Nobody answering never quits: the recording goes on.
        (CallPromptKind::Quit, A::Cancel | A::Expired) => Some(AnswerAction::Close),
        (CallPromptKind::StillRecording, A::KeepRecording) => Some(AnswerAction::KeepRecording),
        (CallPromptKind::StillRecording, A::Stop) => Some(AnswerAction::Stop),
        // Running out: the session itself stops at 15 min of silence; the card only closes.
        (CallPromptKind::StillRecording, A::Expired) => Some(AnswerAction::Close),
        // Recording again is the explicit start path, consent first (ADR-0005).
        (CallPromptKind::SilenceStopped, A::RecordAgain) => Some(AnswerAction::Record),
        (
            CallPromptKind::SilenceStopped | CallPromptKind::NothingRecorded,
            A::Dismiss | A::Expired,
        ) => Some(AnswerAction::Close),
        (CallPromptKind::Mistake, A::Keep | A::Discard) => {
            let id = current.session_id.clone()?;
            Some(if answer == A::Keep {
                AnswerAction::Keep(id)
            } else {
                AnswerAction::Discard(id)
            })
        }
        // Nobody answering keeps the recording, but sends nothing to the transcription.
        (CallPromptKind::Mistake, A::Expired) => Some(AnswerAction::Close),
        _ => None,
    }
}

/// The muted list with `id` added once; unknown ids are dropped.
pub(crate) fn normalize_muted(apps: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for id in apps {
        if CallApp::by_id(&id).is_some() && !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// Physical bounds of the card: top right of the work area, Granola's insets, at `scale`.
pub(crate) fn placement(
    work_position: PhysicalPosition<i32>,
    work_size: PhysicalSize<u32>,
    scale: f64,
) -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
    let px = |logical: f64| (logical * scale).round();
    let x = work_position.x as f64 + work_size.width as f64 - px(RIGHT_INSET);
    let y = work_position.y as f64 + px(TOP_INSET);
    (
        PhysicalPosition::new(x as i32, y as i32),
        PhysicalSize::new(px(WIDTH) as u32, px(HEIGHT) as u32),
    )
}

static CURRENT: Mutex<Option<CallPromptPayload>> = Mutex::new(None);
static GENERATION: AtomicU64 = AtomicU64::new(0);

fn current() -> Option<CallPromptPayload> {
    CURRENT.lock().map(|c| c.clone()).unwrap_or(None)
}

fn set_current(payload: Option<CallPromptPayload>) {
    if let Ok(mut current) = CURRENT.lock() {
        *current = payload;
    }
}

fn ensure_window(app: &AppHandle) -> Option<tauri::WebviewWindow> {
    if let Some(window) = app.get_webview_window(LABEL) {
        return Some(window);
    }
    let mut builder = WebviewWindowBuilder::new(
        app,
        LABEL,
        tauri::WebviewUrl::App("src/callprompt/index.html".into()),
    )
    .title("Fala")
    .resizable(false)
    .inner_size(WIDTH, HEIGHT)
    .shadow(false)
    .maximizable(false)
    .minimizable(false)
    .closable(false)
    .accept_first_mouse(true)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .transparent(true)
    .focusable(false)
    .focused(false)
    .visible(false);
    if let Some(data_dir) = crate::portable::data_dir() {
        builder = builder.data_directory(data_dir.join("webview"));
    }
    match builder.build() {
        Ok(window) => Some(window),
        Err(e) => {
            log::error!("call prompt window not created: {e}");
            None
        }
    }
}

fn place(app: &AppHandle, window: &tauri::WebviewWindow) {
    let monitor = app
        .primary_monitor()
        .ok()
        .flatten()
        .or_else(|| window.current_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        log::debug!("call prompt: no monitor to place it on");
        return;
    };
    let area = monitor.work_area();
    let (position, size) = placement(area.position, area.size, monitor.scale_factor());
    let _ = window.set_size(size);
    let _ = window.set_position(position);
}

/// Shows the card for `prompt`, replacing any card on screen.
pub fn show(app: &AppHandle, prompt: CallPrompt) {
    show_payload(app, CallPromptPayload::from_prompt(prompt));
}

/// "Still recording?" with the time left before the stop for silence.
pub fn show_still_recording(app: &AppHandle, remaining: Duration) {
    if current().is_some_and(|c| c.kind == CallPromptKind::Quit) {
        return;
    }
    let remaining = remaining.max(Duration::from_secs(1));
    show_payload(
        app,
        CallPromptPayload::meeting(CallPromptKind::StillRecording, remaining),
    );
}

/// Takes "Still recording?" off the screen once the session stops, however it stops.
pub fn hide_still_recording(app: &AppHandle) {
    if current().is_some_and(|c| c.kind == CallPromptKind::StillRecording) {
        hide(app);
    }
}

pub fn show_silence_stopped(app: &AppHandle) {
    show_payload(
        app,
        CallPromptPayload::meeting(CallPromptKind::SilenceStopped, COUNTDOWN),
    );
}

/// "Recorded by mistake?" about `id`, for a minute (Wispr's 60 s).
pub fn show_mistake(app: &AppHandle, id: fala_meeting::SessionId, seconds: u32) {
    let mut payload = CallPromptPayload::meeting(CallPromptKind::Mistake, MISTAKE_COUNTDOWN);
    payload.session_id = Some(id.to_string());
    payload.seconds = Some(seconds);
    show_payload(app, payload);
}

pub fn show_nothing_recorded(app: &AppHandle) {
    show_payload(
        app,
        CallPromptPayload::meeting(CallPromptKind::NothingRecorded, COUNTDOWN),
    );
}

/// "Stop the recording? Quitting Fala stops it." for the tray's "Quit".
pub fn show_quit(app: &AppHandle) {
    show_payload(app, CallPromptPayload::quit());
}

fn show_payload(app: &AppHandle, payload: CallPromptPayload) {
    let quit = payload.kind == CallPromptKind::Quit;
    let still_recording = payload.kind == CallPromptKind::StillRecording;
    set_current(Some(payload.clone()));
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(window) = ensure_window(&handle) else {
            set_current(None);
            // Without the card, "Quit" would do nothing: stop the recording safely and quit.
            if quit {
                if let Some(meetings) = handle.try_state::<Arc<MeetingManager>>() {
                    let meetings = Arc::clone(&meetings);
                    std::thread::spawn(move || meetings.stop_and_quit());
                }
            }
            return;
        };
        place(&handle, &window);
        let shown = window.show().is_ok();
        // Silence may stop the recording only once this card was really on screen.
        if shown && still_recording {
            if let Some(meetings) = handle.try_state::<Arc<MeetingManager>>() {
                let meetings = Arc::clone(&meetings);
                std::thread::spawn(move || meetings.silence_warning_shown());
            }
        }
        let _ = window.set_always_on_top(true);
        // A window created just now has no listener yet; it asks `call_prompt_current`.
        let _ = handle.emit_to(LABEL, PROMPT_EVENT, payload);
    });
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(MAX_ON_SCREEN);
        if GENERATION.load(Ordering::SeqCst) == generation {
            hide(&handle);
        }
    });
}

/// Hides the card and forgets it.
pub fn hide(app: &AppHandle) {
    set_current(None);
    GENERATION.fetch_add(1, Ordering::SeqCst);
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window(LABEL) {
            let _ = window.hide();
        }
    });
}

/// An offer on screen is moot once a meeting records, however it started.
pub fn hide_offer(app: &AppHandle) {
    if current().is_some_and(|c| c.kind == CallPromptKind::Offer) {
        hide(app);
    }
}

/// The card the prompt window should show now, for a window that just loaded.
#[tauri::command]
#[specta::specta]
pub fn call_prompt_current() -> Option<CallPromptPayload> {
    current()
}

/// The person answered the card, or its countdown ran out. Async so that starting a recording
/// stays off the main thread, like `start_meeting`.
#[tauri::command]
#[specta::specta]
pub async fn answer_call_prompt(app: AppHandle, answer: CallPromptAnswer) -> Result<(), String> {
    let Some(action) = answer_action(current().as_ref(), answer) else {
        log::debug!("call prompt: {answer:?} does not match the card on screen");
        return Ok(());
    };
    hide(&app);
    log::info!("call prompt answered: {answer:?}");
    match action {
        AnswerAction::Record => {
            if let Some(meetings) = app.try_state::<Arc<MeetingManager>>() {
                meetings.start_from_tray();
            }
        }
        AnswerAction::Stop => {
            if let Some(meetings) = app.try_state::<Arc<MeetingManager>>() {
                meetings.stop().map_err(|e| e.to_string())?;
            }
        }
        AnswerAction::StopAndQuit => {
            if let Some(meetings) = app.try_state::<Arc<MeetingManager>>() {
                let meetings = Arc::clone(&meetings);
                tauri::async_runtime::spawn_blocking(move || meetings.stop_and_quit());
            }
        }
        AnswerAction::KeepRecording => {
            if let Some(meetings) = app.try_state::<Arc<MeetingManager>>() {
                meetings.keep_recording().map_err(|e| e.to_string())?;
            }
        }
        AnswerAction::Keep(id) => {
            if let Some(meetings) = app.try_state::<Arc<MeetingManager>>() {
                let meetings = Arc::clone(&meetings);
                tauri::async_runtime::spawn_blocking(move || meetings.keep_after_mistake(&id));
            }
        }
        AnswerAction::Discard(id) => {
            if let Some(meetings) = app.try_state::<Arc<MeetingManager>>() {
                let meetings = Arc::clone(&meetings);
                tauri::async_runtime::spawn_blocking(move || {
                    if let Err(e) = meetings.delete(&id) {
                        log::warn!("meeting {id}: not discarded: {e}");
                    }
                });
            }
        }
        AnswerAction::Mute(id) => {
            let mut settings = settings::get_settings(&app);
            let mut muted = settings.call_prompt_muted_apps.clone();
            muted.push(id);
            settings.call_prompt_muted_apps = normalize_muted(muted);
            settings::write_settings(&app, settings);
        }
        AnswerAction::Close => {}
    }
    Ok(())
}

/// A call app the detector knows, for the settings list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CallAppInfo {
    pub id: String,
    pub name: String,
}

#[tauri::command]
#[specta::specta]
pub fn call_prompt_apps() -> Vec<CallAppInfo> {
    CALL_APPS
        .iter()
        .map(|app| CallAppInfo {
            id: app.id.to_string(),
            name: app.name.to_string(),
        })
        .collect()
}

/// "Avisar quando uma chamada começar"; off, the mic holders are not read at all.
#[tauri::command]
#[specta::specta]
pub fn change_call_prompt_enabled_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.call_prompt_enabled = enabled;
    settings::write_settings(&app, settings);
    if !enabled {
        hide(&app);
    }
    Ok(())
}

/// "Não avisar nestes apps"; returns what was stored (known ids, once each).
#[tauri::command]
#[specta::specta]
pub fn change_call_prompt_muted_apps_setting(
    app: AppHandle,
    apps: Vec<String>,
) -> Result<Vec<String>, String> {
    let mut settings = settings::get_settings(&app);
    settings.call_prompt_muted_apps = normalize_muted(apps);
    let stored = settings.call_prompt_muted_apps.clone();
    settings::write_settings(&app, settings);
    Ok(stored)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(kind: CallPromptKind) -> CallPromptPayload {
        CallPromptPayload {
            kind,
            app_id: "teams".into(),
            app_name: "Teams".into(),
            web_name: None,
            countdown_ms: 20_000,
            session_id: None,
            seconds: None,
        }
    }

    #[test]
    fn placement_top_right() {
        let origin = PhysicalPosition::new(0, 0);
        // 1920 × 1032 work area (taskbar at the bottom), 100 %.
        let (pos, size) = placement(origin, PhysicalSize::new(1920, 1032), 1.0);
        assert_eq!((pos.x, pos.y), (1920 - 408, 4));
        assert_eq!((size.width, size.height), (404, 104));
        // 150 % on a second monitor to the left, with the taskbar at the top.
        let (pos, size) = placement(
            PhysicalPosition::new(-2560, 48),
            PhysicalSize::new(2560, 1392),
            1.5,
        );
        assert_eq!((pos.x, pos.y), (-2560 + 2560 - 612, 48 + 6));
        assert_eq!((size.width, size.height), (606, 156));
    }

    #[test]
    fn answer_maps_each_offer_button() {
        let offer = card(CallPromptKind::Offer);
        use CallPromptAnswer as A;
        assert_eq!(
            answer_action(Some(&offer), A::Record),
            Some(AnswerAction::Record)
        );
        assert_eq!(
            answer_action(Some(&offer), A::MuteApp),
            Some(AnswerAction::Mute("teams".into()))
        );
        for close in [A::NotNow, A::Expired] {
            assert_eq!(
                answer_action(Some(&offer), close),
                Some(AnswerAction::Close)
            );
        }
        // An offer has no [Parar].
        for stale in [A::Stop, A::KeepRecording] {
            assert_eq!(answer_action(Some(&offer), stale), None);
        }
    }

    #[test]
    fn answer_maps_each_stop_button() {
        let ask = card(CallPromptKind::AskStop);
        use CallPromptAnswer as A;
        assert_eq!(answer_action(Some(&ask), A::Stop), Some(AnswerAction::Stop));
        // No answer keeps recording (ADR-0005; Wispr's rule for an ambiguous end).
        for keep in [A::KeepRecording, A::Expired] {
            assert_eq!(answer_action(Some(&ask), keep), Some(AnswerAction::Close));
        }
        for stale in [A::Record, A::MuteApp, A::NotNow] {
            assert_eq!(answer_action(Some(&ask), stale), None);
        }
    }

    #[test]
    fn answer_maps_each_quit_button() {
        let quit = CallPromptPayload::quit();
        use CallPromptAnswer as A;
        assert_eq!(
            answer_action(Some(&quit), A::StopAndQuit),
            Some(AnswerAction::StopAndQuit)
        );
        // Cancelling or letting it run out keeps recording and the app open.
        for keep in [A::Cancel, A::Expired] {
            assert_eq!(answer_action(Some(&quit), keep), Some(AnswerAction::Close));
        }
        for stale in [A::Record, A::Stop, A::MuteApp] {
            assert_eq!(answer_action(Some(&quit), stale), None);
        }
        // The call cards do not quit.
        let ask = card(CallPromptKind::AskStop);
        assert_eq!(answer_action(Some(&ask), A::StopAndQuit), None);
    }

    #[test]
    fn answer_maps_each_meeting_card() {
        use CallPromptAnswer as A;
        use CallPromptKind as K;
        let still = CallPromptPayload::meeting(K::StillRecording, Duration::from_secs(60));
        assert_eq!(still.countdown_ms, 60_000);
        assert_eq!(
            answer_action(Some(&still), A::KeepRecording),
            Some(AnswerAction::KeepRecording)
        );
        assert_eq!(
            answer_action(Some(&still), A::Stop),
            Some(AnswerAction::Stop)
        );
        assert_eq!(
            answer_action(Some(&still), A::Expired),
            Some(AnswerAction::Close)
        );

        let stopped = CallPromptPayload::meeting(K::SilenceStopped, COUNTDOWN);
        assert_eq!(
            answer_action(Some(&stopped), A::RecordAgain),
            Some(AnswerAction::Record)
        );
        for close in [A::Dismiss, A::Expired] {
            assert_eq!(
                answer_action(Some(&stopped), close),
                Some(AnswerAction::Close)
            );
        }

        let mut mistake = CallPromptPayload::meeting(K::Mistake, MISTAKE_COUNTDOWN);
        mistake.session_id = Some("01K".into());
        assert_eq!(
            answer_action(Some(&mistake), A::Discard),
            Some(AnswerAction::Discard("01K".into()))
        );
        assert_eq!(
            answer_action(Some(&mistake), A::Keep),
            Some(AnswerAction::Keep("01K".into()))
        );
        // Running out keeps the session: nothing is deleted without the click.
        assert_eq!(
            answer_action(Some(&mistake), A::Expired),
            Some(AnswerAction::Close)
        );
        mistake.session_id = None;
        assert_eq!(answer_action(Some(&mistake), A::Discard), None);

        let nothing = CallPromptPayload::meeting(K::NothingRecorded, COUNTDOWN);
        assert_eq!(
            answer_action(Some(&nothing), A::Dismiss),
            Some(AnswerAction::Close)
        );
        for stale in [A::Record, A::Stop, A::Discard] {
            assert_eq!(answer_action(Some(&nothing), stale), None);
        }
    }

    #[test]
    fn answer_without_a_card_does_nothing() {
        use CallPromptAnswer as A;
        for answer in [
            A::Record,
            A::NotNow,
            A::MuteApp,
            A::Stop,
            A::KeepRecording,
            A::StopAndQuit,
            A::Cancel,
            A::RecordAgain,
            A::Dismiss,
            A::Keep,
            A::Discard,
            A::Expired,
        ] {
            assert_eq!(answer_action(None, answer), None);
        }
    }

    #[test]
    fn answer_muted_list_keeps_known_ids_once() {
        let muted = normalize_muted(
            ["whatsapp", "teams", "whatsapp", "nao-existe", "fala"]
                .into_iter()
                .map(String::from),
        );
        assert_eq!(muted, ["whatsapp", "teams"]);
    }

    #[test]
    fn answer_payload_names_the_app() {
        let teams = CallApp::by_id("teams").unwrap();
        let payload = CallPromptPayload::from_prompt(CallPrompt::OfferRecording {
            app: teams,
            web: None,
        });
        assert_eq!(payload, card(CallPromptKind::Offer));
        let payload = CallPromptPayload::from_prompt(CallPrompt::AskStop {
            app: teams,
            web: None,
        });
        assert_eq!(payload.kind, CallPromptKind::AskStop);
    }

    #[test]
    fn answer_payload_names_the_meeting_in_the_browser() {
        let edge = CallApp::by_id("edge").unwrap();
        let meet = fala_meeting::web_call(["Meet - abc-defg-hij - Microsoft Edge"]);
        for prompt in [
            CallPrompt::OfferRecording {
                app: edge,
                web: meet,
            },
            CallPrompt::AskStop {
                app: edge,
                web: meet,
            },
        ] {
            let payload = CallPromptPayload::from_prompt(prompt);
            assert_eq!(payload.app_id, "edge");
            assert_eq!(payload.app_name, "Edge");
            assert_eq!(payload.web_name.as_deref(), Some("Google Meet"));
        }
        // [Não avisar] still mutes the browser, not the meeting site.
        let offer = CallPromptPayload::from_prompt(CallPrompt::OfferRecording {
            app: edge,
            web: meet,
        });
        assert_eq!(
            answer_action(Some(&offer), CallPromptAnswer::MuteApp),
            Some(AnswerAction::Mute("edge".into()))
        );
    }
}
