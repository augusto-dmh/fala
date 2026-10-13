use crate::input;
use crate::meeting::MeetingIndicator;
use crate::settings;
use crate::settings::{OverlayPosition, OverlayStyle};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};

#[cfg(not(target_os = "macos"))]
use log::debug;

#[cfg(not(target_os = "macos"))]
use tauri::WebviewWindowBuilder;

#[cfg(target_os = "macos")]
use tauri::WebviewUrl;

#[cfg(target_os = "macos")]
use tauri_nspanel::{tauri_panel, CollectionBehavior, PanelBuilder, PanelLevel, StyleMask};

#[cfg(target_os = "linux")]
use crate::utils;

#[cfg(target_os = "linux")]
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

#[cfg(target_os = "macos")]
tauri_panel! {
    panel!(RecordingOverlayPanel {
        config: {
            can_become_key_window: false,
            is_floating_panel: true
        }
    })
}

// Native overlay window sizes (logical points). One window is reused for every
// state and resized in `show_overlay_state`; each size need only be at least as
// large as the card it hosts (the `--ov-*` vars in RecordingOverlay.css). The
// card is CSS-anchored flush to the screen edge, so window height doesn't move
// where the card sits — only OVERLAY_TOP_OFFSET / OVERLAY_BOTTOM_OFFSET do. Keep
// these in sync with the CSS card geometry.
//
// On Windows these sizes are additionally multiplied by the accessibility text
// scale (see windows_text_scale_factor), which WebView2 applies as a zoom.
//
// Compact overlay (Minimal / transcribing / processing): the 40h pill animates
// width from 172 (--ov-rest-w) to 216 (--ov-work-w) and expands from center, so
// the window must fit the widest state plus a little slack.
const OVERLAY_WIDTH: f64 = 256.0;
const OVERLAY_HEIGHT: f64 = 50.0;

// Actual is 394x118, just a little extra
const OVERLAY_STREAM_WIDTH: f64 = 400.0;
const OVERLAY_STREAM_HEIGHT: f64 = 120.0;

// A notice card (two lines with actions), alone or stacked over the 30h pill.
const OVERLAY_NOTICE_WIDTH: f64 = 340.0;
const OVERLAY_NOTICE_HEIGHT: f64 = 110.0;

/// The "the call plays on another output" card sits over the meeting pill: the window takes a
/// notice's size while it shows.
static MEETING_CARD: AtomicBool = AtomicBool::new(false);

/// A dictation runs during a meeting: the card "the meeting's mic is paused while you dictate"
/// sits over the recording pill (ADR-0015), so the window takes a notice's size.
static MEETING_DICTATION: AtomicBool = AtomicBool::new(false);

/// The last state asked of the overlay was the recording pill (not the live streaming card).
static RECORDING_PILL: AtomicBool = AtomicBool::new(false);

// The idle pill: a 40x8 capsule (`.fpill.idle` in Pill.css) in a window just large enough to
// hover, so the transparent area that takes clicks stays small. On hover the window grows for the
// shortcut card (`.fpill.idle.hover`).
const OVERLAY_IDLE_WIDTH: f64 = 64.0;
const OVERLAY_IDLE_HEIGHT: f64 = 20.0;
const OVERLAY_IDLE_HOVER_WIDTH: f64 = 300.0;
const OVERLAY_IDLE_HOVER_HEIGHT: f64 = 64.0;

/// Overlay window size (logical) for a given UI state.
fn overlay_dimensions(state: &str) -> (f64, f64) {
    sized(
        state,
        MEETING_CARD.load(Ordering::Relaxed),
        MEETING_DICTATION.load(Ordering::Relaxed),
    )
}

/// The size of `state`, with a card over the meeting pill or over a dictation's pill.
fn sized(state: &str, meeting_card: bool, meeting_dictation: bool) -> (f64, f64) {
    match state {
        "streaming" => (OVERLAY_STREAM_WIDTH, OVERLAY_STREAM_HEIGHT),
        "notice" | "paste_failed" => (OVERLAY_NOTICE_WIDTH, OVERLAY_NOTICE_HEIGHT),
        "meeting" | "meeting_paused" if meeting_card => {
            (OVERLAY_NOTICE_WIDTH, OVERLAY_NOTICE_HEIGHT)
        }
        "recording" if meeting_dictation => (OVERLAY_NOTICE_WIDTH, OVERLAY_NOTICE_HEIGHT),
        "idle" => (OVERLAY_IDLE_WIDTH, OVERLAY_IDLE_HEIGHT),
        "idle_hover" => (OVERLAY_IDLE_HOVER_WIDTH, OVERLAY_IDLE_HOVER_HEIGHT),
        _ => (OVERLAY_WIDTH, OVERLAY_HEIGHT),
    }
}

/// What a notice in the overlay says. The frontend translates the code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NoticeKind {
    /// The microphone the recording opened, the first time it differs from the last one.
    MicInUse,
    /// The recording heard nothing at all.
    MicMuted,
    /// The OS denied microphone access.
    MicDenied,
    /// No input device was found.
    NoMic,
    /// The microphone failed to open for another reason (busy, unplugged).
    MicFailed,
    /// No model can transcribe the dictation.
    ModelMissing,
}

/// How long a notice stays: the microphone name briefly, the notices with actions long
/// enough to reach them.
pub fn notice_duration(kind: NoticeKind) -> std::time::Duration {
    let ms = match kind {
        NoticeKind::MicInUse => 1_500,
        NoticeKind::MicMuted
        | NoticeKind::MicDenied
        | NoticeKind::NoMic
        | NoticeKind::MicFailed => 6_000,
        NoticeKind::ModelMissing => 5_000,
    };
    std::time::Duration::from_millis(ms)
}

#[derive(Clone, serde::Serialize)]
struct NoticePayload {
    kind: NoticeKind,
    device: Option<String>,
    alone: bool,
    duration_ms: u64,
}

static LAST_MIC_LEVEL_EMIT: AtomicU64 = AtomicU64::new(0);
const EMIT_THROTTLE_MS: u64 = 33; // ~30 FPS

#[cfg(target_os = "macos")]
const OVERLAY_TOP_OFFSET: f64 = 46.0;
#[cfg(any(target_os = "windows", target_os = "linux"))]
const OVERLAY_TOP_OFFSET: f64 = 4.0;

#[cfg(target_os = "macos")]
const OVERLAY_BOTTOM_OFFSET: f64 = 15.0;

#[cfg(any(target_os = "windows", target_os = "linux"))]
const OVERLAY_BOTTOM_OFFSET: f64 = 40.0;

/// Configures the edge and offset of a GTK layer surface. gtk-layer-shell
/// commits anchor and margin changes itself, including while the surface is
/// mapped, so changing position does not require a manual hide/show cycle.
#[cfg(target_os = "linux")]
fn configure_layer_shell_position(gtk_window: &gtk::ApplicationWindow, position: OverlayPosition) {
    let (edge, opposite_edge, margin) = match position {
        OverlayPosition::Top => (Edge::Top, Edge::Bottom, OVERLAY_TOP_OFFSET),
        OverlayPosition::Bottom => (Edge::Bottom, Edge::Top, OVERLAY_BOTTOM_OFFSET),
    };

    gtk_window.set_anchor(edge, true);
    gtk_window.set_anchor(opposite_edge, false);
    gtk_window.set_layer_shell_margin(edge, margin.round() as i32);
    gtk_window.set_layer_shell_margin(opposite_edge, 0);
}

/// Configures a GTK layer surface before it is shown.
///
/// Tauri's normal `set_size` path calls `gtk_window_resize`, but layer surfaces
/// derive their dimensions from GTK's size request. gtk-layer-shell documents
/// the `set_size_request` + `resize(1, 1)` sequence for forcing a new size.
#[cfg(target_os = "linux")]
fn configure_layer_shell_surface(
    gtk_window: &gtk::ApplicationWindow,
    position: OverlayPosition,
    width: f64,
    height: f64,
) {
    use gtk::prelude::{GtkWindowExt, WidgetExt};

    configure_layer_shell_position(gtk_window, position);

    gtk_window.set_size_request(
        width.round().max(1.0) as i32,
        height.round().max(1.0) as i32,
    );
    gtk_window.resize(1, 1);
}

/// Initializes GTK layer shell for Linux overlay window
/// Returns true if layer shell was successfully initialized, false otherwise
#[cfg(target_os = "linux")]
fn init_gtk_layer_shell(overlay_window: &tauri::webview::WebviewWindow) -> bool {
    if utils::env_flag_enabled("FALA_NO_GTK_LAYER_SHELL") {
        debug!("Skipping GTK layer shell init (FALA_NO_GTK_LAYER_SHELL is enabled)");
        return false;
    }

    if !gtk_layer_shell::is_supported() {
        return false;
    }

    // Try to get the GTK window from the Tauri webview
    if let Ok(gtk_window) = overlay_window.gtk_window() {
        gtk_window.init_layer_shell();
        gtk_window.set_layer(Layer::Overlay);
        gtk_window.set_keyboard_mode(KeyboardMode::None);
        gtk_window.set_exclusive_zone(0);

        let overlay_position = settings::get_settings(overlay_window.app_handle()).overlay_position;
        configure_layer_shell_surface(&gtk_window, overlay_position, OVERLAY_WIDTH, OVERLAY_HEIGHT);

        let initialized = gtk_window.is_layer_window();
        LAYER_SHELL_ACTIVE.store(initialized, Ordering::SeqCst);
        return initialized;
    }
    false
}

/// Forces a window to be topmost using Win32 API (Windows only)
/// This is more reliable than Tauri's set_always_on_top which can be overridden
#[cfg(target_os = "windows")]
fn force_overlay_topmost(overlay_window: &tauri::webview::WebviewWindow) {
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    };

    // Clone because run_on_main_thread takes 'static
    let overlay_clone = overlay_window.clone();

    // Make sure the Win32 call happens on the UI thread
    let _ = overlay_clone.clone().run_on_main_thread(move || {
        if let Ok(hwnd) = overlay_clone.hwnd() {
            unsafe {
                // Force Z-order: make this window topmost without changing size/pos or stealing focus
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            }
        }
    });
}

fn get_monitor_with_cursor(app_handle: &AppHandle) -> Option<tauri::Monitor> {
    if let Some(mouse_location) = input::get_cursor_position(app_handle) {
        if let Ok(monitors) = app_handle.available_monitors() {
            for monitor in monitors {
                // On Windows both the cursor (enigo -> GetCursorPos) and the
                // monitor bounds are physical pixels, so compare them directly.
                #[cfg(target_os = "windows")]
                if is_mouse_within_monitor(mouse_location, monitor.position(), monitor.size()) {
                    return Some(monitor);
                }

                // macOS/Linux: enigo returns logical coords, so scale the bounds down.
                #[cfg(not(target_os = "windows"))]
                {
                    let scale = monitor.scale_factor();
                    let pos = PhysicalPosition::new(
                        (monitor.position().x as f64 / scale) as i32,
                        (monitor.position().y as f64 / scale) as i32,
                    );
                    let size = PhysicalSize::new(
                        (monitor.size().width as f64 / scale) as u32,
                        (monitor.size().height as f64 / scale) as u32,
                    );
                    if is_mouse_within_monitor(mouse_location, &pos, &size) {
                        return Some(monitor);
                    }
                }
            }
        }
    }

    app_handle.primary_monitor().ok().flatten()
}

fn is_mouse_within_monitor(
    mouse_pos: (i32, i32),
    monitor_pos: &PhysicalPosition<i32>,
    monitor_size: &PhysicalSize<u32>,
) -> bool {
    let (mouse_x, mouse_y) = mouse_pos;
    let PhysicalPosition {
        x: monitor_x,
        y: monitor_y,
    } = *monitor_pos;
    let PhysicalSize {
        width: monitor_width,
        height: monitor_height,
    } = *monitor_size;

    mouse_x >= monitor_x
        && mouse_x < (monitor_x + monitor_width as i32)
        && mouse_y >= monitor_y
        && mouse_y < (monitor_y + monitor_height as i32)
}

/// Returns overlay position in logical coordinates (points on macOS).
///
/// The Bottom anchor uses the macOS work area (visibleFrame) so the overlay
/// tracks the Dock — above it when shown, at the screen edge when hidden.
/// This relies on tauri 2.11's work_area.position.y fix (#14655), the same
/// bug that led PR #969 to abandon work_area for full monitor bounds. Top and
/// the other platforms keep full monitor bounds plus the fixed offsets
/// (work_area is unreliable on Wayland; Windows' offset clears the taskbar).
///
/// We must use LogicalPosition (not PhysicalPosition) because Tauri/tao
/// converts PhysicalPosition using the scale factor of the monitor the window
/// is *currently* on, which is wrong when moving cross-monitor. Windows uses
/// `place_windows_overlay` instead (no single logical space across mixed DPI).
fn calculate_overlay_position(
    app_handle: &AppHandle,
    width: f64,
    height: f64,
) -> Option<(f64, f64)> {
    let monitor = get_monitor_with_cursor(app_handle)?;
    let scale = monitor.scale_factor();
    let monitor_x = monitor.position().x as f64 / scale;
    let monitor_y = monitor.position().y as f64 / scale;
    let monitor_width = monitor.size().width as f64 / scale;

    let settings = settings::get_settings(app_handle);

    let x = monitor_x + (monitor_width - width) / 2.0;
    let y = match settings.overlay_position {
        OverlayPosition::Top => monitor_y + OVERLAY_TOP_OFFSET,
        OverlayPosition::Bottom => {
            // work_area.position shares monitor.position's global coordinate
            // space, so no monitor offset is added.
            #[cfg(target_os = "macos")]
            let bottom = {
                let wa = monitor.work_area();
                (wa.position.y as f64 + wa.size.height as f64) / scale
            };
            #[cfg(not(target_os = "macos"))]
            let bottom = monitor_y + monitor.size().height as f64 / scale;

            bottom - height - OVERLAY_BOTTOM_OFFSET
        }
    };

    Some((x, y))
}

/// Current overlay window size in logical units (points), for repositioning
/// without assuming a fixed size (compact vs. streaming).
#[cfg(not(target_os = "windows"))]
fn current_overlay_logical_size(window: &tauri::webview::WebviewWindow) -> Option<(f64, f64)> {
    let size = window.inner_size().ok()?;
    let scale = window.scale_factor().ok()?;
    Some((size.width as f64 / scale, size.height as f64 / scale))
}

#[cfg(target_os = "windows")]
static WINDOWS_OVERLAY_IS_STREAMING: AtomicBool = AtomicBool::new(false);

/// Windows accessibility text size (Settings > Accessibility > Text size), a
/// separate axis from display scaling that WebView2 applies as a document zoom.
#[cfg(target_os = "windows")]
fn windows_text_scale_factor() -> f64 {
    // Absent until the user moves the slider off 100%; stored as a percentage.
    winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Accessibility")
        .and_then(|key| key.get_value::<u32, _>("TextScaleFactor"))
        .map(|percent| (percent as f64 / 100.0).clamp(1.0, 2.25))
        .unwrap_or(1.0)
}

/// Overlay rectangle in the destination monitor's physical pixels, so nothing
/// is converted through the window's previous-monitor DPI.
#[cfg(target_os = "windows")]
#[allow(clippy::too_many_arguments)] // plain geometry inputs, each one named at the call site
fn windows_overlay_bounds(
    monitor_position: PhysicalPosition<i32>,
    monitor_size: PhysicalSize<u32>,
    scale: f64,
    text_scale: f64,
    logical_width: f64,
    logical_height: f64,
    overlay_position: OverlayPosition,
    center: f64,
) -> (i32, i32, i32, i32) {
    // Grow the window with the text scale; offsets stay DPI-only since the
    // card sits flush against the window's screen-edge side.
    let content_scale = scale * text_scale;
    let width = (logical_width * content_scale).round().max(1.0) as i32;
    let height = (logical_height * content_scale).round().max(1.0) as i32;
    let x = pill_left(monitor_position.x, monitor_size.width, width, center);
    let y = match overlay_position {
        OverlayPosition::Top => {
            (monitor_position.y as f64 + OVERLAY_TOP_OFFSET * scale).round() as i32
        }
        OverlayPosition::Bottom => (monitor_position.y as f64 + monitor_size.height as f64
            - height as f64
            - OVERLAY_BOTTOM_OFFSET * scale)
            .round() as i32,
    };

    (x, y, width, height)
}

/// The pill centered on its edge, as a fraction of the monitor's width.
const PILL_CENTERED: f64 = 0.5;

/// Where along its edge the pill's center sits, as a fraction of the monitor's width, in `f64`
/// bits: `pill_center_x` from the settings, or the live place while the pill is dragged.
static PILL_CENTER: AtomicU64 = AtomicU64::new(PILL_CENTERED.to_bits());

#[cfg(target_os = "windows")]
fn pill_center() -> f64 {
    f64::from_bits(PILL_CENTER.load(Ordering::SeqCst))
}

/// Applies `pill_center_x`: at startup, and when choosing the edge again recenters the pill.
pub fn set_pill_center(center: Option<f64>) {
    let center = center
        .filter(|c| c.is_finite())
        .unwrap_or(PILL_CENTERED)
        .clamp(0.0, 1.0);
    PILL_CENTER.store(center.to_bits(), Ordering::SeqCst);
}

/// Left edge of a window `width` wide whose center sits at `center` of the monitor's width,
/// kept inside the monitor.
#[cfg(target_os = "windows")]
fn pill_left(monitor_x: i32, monitor_width: u32, width: i32, center: f64) -> i32 {
    let left = (monitor_x as f64 + center * monitor_width as f64 - width as f64 / 2.0).round();
    let right_most = (monitor_x + monitor_width as i32 - width).max(monitor_x);
    (left as i32).clamp(monitor_x, right_most)
}

/// Where the center of a window dragged `dx` from `start_center` (physical x) sits, as a
/// fraction of the monitor's width.
#[cfg(target_os = "windows")]
fn dragged_center(monitor_x: i32, monitor_width: u32, start_center: i32, dx: i32) -> f64 {
    let center = (start_center + dx - monitor_x) as f64;
    (center / monitor_width.max(1) as f64).clamp(0.0, 1.0)
}

/// Moves and sizes the overlay in one native SetWindowPos, bypassing tao's
/// current-DPI logical conversion that mislands cross-monitor moves.
#[cfg(target_os = "windows")]
fn place_windows_overlay(
    app_handle: &AppHandle,
    overlay_window: &tauri::webview::WebviewWindow,
    logical_width: f64,
    logical_height: f64,
) -> Result<(), String> {
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER};

    let monitor = get_monitor_with_cursor(app_handle)
        .ok_or_else(|| "failed to determine the monitor containing the cursor".to_string())?;
    let text_scale = windows_text_scale_factor();
    let (x, y, width, height) = windows_overlay_bounds(
        *monitor.position(),
        *monitor.size(),
        monitor.scale_factor(),
        text_scale,
        logical_width,
        logical_height,
        settings::get_settings(app_handle).overlay_position,
        pill_center(),
    );
    let hwnd = overlay_window
        .hwnd()
        .map_err(|error| format!("failed to get overlay window handle: {error}"))?;

    unsafe {
        SetWindowPos(
            hwnd,
            None,
            x,
            y,
            width,
            height,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .map_err(|error| format!("failed to set overlay bounds: {error}"))?;
    }

    log::debug!(
        "windows overlay bounds: x={} y={} width={} height={} scale={} text_scale={}",
        x,
        y,
        width,
        height,
        monitor.scale_factor(),
        text_scale
    );
    Ok(())
}

/// Creates the recording overlay window and keeps it hidden by default
#[cfg(not(target_os = "macos"))]
pub fn create_recording_overlay(app_handle: &AppHandle) {
    // On Linux (Wayland), monitor detection often fails, but we don't need exact coordinates
    // for Layer Shell as we use anchors. On other platforms, we require a monitor.
    #[cfg(not(target_os = "linux"))]
    {
        let position = calculate_overlay_position(app_handle, OVERLAY_WIDTH, OVERLAY_HEIGHT);
        if position.is_none() {
            debug!("Failed to determine overlay position, not creating overlay window");
            return;
        }
    }

    // Position starts unset — update_overlay_position() sets the correct
    // LogicalPosition before the overlay is shown.
    let mut builder = WebviewWindowBuilder::new(
        app_handle,
        "recording_overlay",
        tauri::WebviewUrl::App("src/overlay/index.html".into()),
    )
    .title("Recording")
    .resizable(false)
    .inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
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

    #[allow(unused_variables)]
    match builder.build() {
        Ok(window) => {
            #[cfg(target_os = "linux")]
            {
                // Try to initialize GTK layer shell, ignore errors if compositor doesn't support it
                if init_gtk_layer_shell(&window) {
                    debug!("GTK layer shell initialized for overlay window");
                } else {
                    debug!("GTK layer shell not available, falling back to regular window");
                }
            }

            debug!("Recording overlay window created successfully (hidden)");
        }
        Err(e) => {
            debug!("Failed to create recording overlay window: {}", e);
        }
    }
}

/// Creates the recording overlay panel and keeps it hidden by default (macOS)
#[cfg(target_os = "macos")]
pub fn create_recording_overlay(app_handle: &AppHandle) {
    if let Some((x, y)) = calculate_overlay_position(app_handle, OVERLAY_WIDTH, OVERLAY_HEIGHT) {
        // PanelBuilder creates a Tauri window then converts it to NSPanel.
        // The window remains registered, so get_webview_window() still works.
        match PanelBuilder::<_, RecordingOverlayPanel>::new(app_handle, "recording_overlay")
            .url(WebviewUrl::App("src/overlay/index.html".into()))
            .title("Recording")
            .position(tauri::Position::Logical(tauri::LogicalPosition { x, y }))
            .level(PanelLevel::Status)
            .size(tauri::Size::Logical(tauri::LogicalSize {
                width: OVERLAY_WIDTH,
                height: OVERLAY_HEIGHT,
            }))
            .has_shadow(false)
            .transparent(true)
            .no_activate(true)
            .corner_radius(0.0)
            .style_mask(StyleMask::empty().borderless().nonactivating_panel())
            .with_window(|w| w.decorations(false).transparent(true).focusable(false))
            .collection_behavior(
                CollectionBehavior::new()
                    .can_join_all_spaces()
                    .full_screen_auxiliary(),
            )
            .build()
        {
            Ok(panel) => {
                panel.hide();
            }
            Err(e) => {
                log::error!("Failed to create recording overlay panel: {}", e);
            }
        }
    }
}

fn show_overlay_state(app_handle: &AppHandle, state: &str) {
    // Whether the overlay shows at all is governed by overlay_style; position
    // only chooses Top vs Bottom placement. Checked here (off the main thread)
    // so the common overlay-disabled case never pays for a main-thread hop.
    RECORDING_PILL.store(state == "recording", Ordering::SeqCst);
    let settings = settings::get_settings(app_handle);
    if settings.overlay_style == OverlayStyle::None {
        return;
    }

    // The rest queries monitors and the cursor and mutates window geometry. On
    // Linux the monitor/cursor lookups hit GDK/Xlib on the process's shared X11
    // connection, which is only safe from the GTK main thread — running them on
    // a background thread corrupts the connection and hard-crashes the app
    // (issue #227). Hop to the main thread on every platform to keep the
    // geometry path uniform (a no-op cost on Windows, and it also keeps macOS's
    // NSScreen access main-thread-correct). run_on_main_thread runs the closure
    // inline when already on the main thread, so this never deadlocks.
    let handle = app_handle.clone();
    let state = state.to_string();
    let _ = app_handle.run_on_main_thread(move || show_overlay_state_on_main(&handle, &state));
}

fn show_overlay_state_on_main(app_handle: &AppHandle, state: &str) {
    // Size the overlay for this state (compact vs. streaming), then position it.
    let (width, height) = overlay_dimensions(state);
    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        // Invalidate any delayed hide still in flight from a previous session
        // (see `hide_recording_overlay`).
        OVERLAY_SHOW_GENERATION.fetch_add(1, Ordering::SeqCst);
        // The text of a failed paste lives only while its pill shows.
        if state != "paste_failed" {
            crate::last_dictation::forget_failed_paste();
        }

        #[cfg(target_os = "linux")]
        let shown_with_layer_shell = if LAYER_SHELL_ACTIVE.load(Ordering::SeqCst) {
            let position = settings::get_settings(app_handle).overlay_position;
            match overlay_window.gtk_window() {
                Ok(gtk_window) => {
                    configure_layer_shell_surface(&gtk_window, position, width, height)
                }
                Err(error) => log::error!("Failed to access GTK overlay window: {error}"),
            }
            let _ = overlay_window.show();
            true
        } else {
            false
        };
        #[cfg(not(target_os = "linux"))]
        let shown_with_layer_shell = false;

        if !shown_with_layer_shell {
            let size_started = std::time::Instant::now();
            #[cfg(not(target_os = "windows"))]
            let _ =
                overlay_window.set_size(tauri::Size::Logical(tauri::LogicalSize { width, height }));
            #[cfg(target_os = "windows")]
            WINDOWS_OVERLAY_IS_STREAMING.store(state == "streaming", Ordering::Relaxed);
            let size_elapsed = size_started.elapsed();

            let pos_started = std::time::Instant::now();
            #[cfg(not(target_os = "windows"))]
            let set_pos_elapsed =
                if let Some((x, y)) = calculate_overlay_position(app_handle, width, height) {
                    let set_pos_started = std::time::Instant::now();
                    let _ = overlay_window
                        .set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
                    set_pos_started.elapsed()
                } else {
                    std::time::Duration::ZERO
                };
            #[cfg(target_os = "windows")]
            let set_pos_elapsed = {
                let set_pos_started = std::time::Instant::now();
                if let Err(error) =
                    place_windows_overlay(app_handle, &overlay_window, width, height)
                {
                    log::error!("Failed to place recording overlay: {error}");
                }
                set_pos_started.elapsed()
            };
            let pos_calc_elapsed = pos_started.elapsed() - set_pos_elapsed;

            let show_started = std::time::Instant::now();
            let _ = overlay_window.show();
            let show_elapsed = show_started.elapsed();

            // On Windows, aggressively re-assert "topmost" in the native Z-order after showing
            #[cfg(target_os = "windows")]
            force_overlay_topmost(&overlay_window);

            // Re-assert bounds after show(): the pre-show move crosses the DPI
            // boundary, and tao's WM_DPICHANGED reflow clobbers the first placement.
            #[cfg(target_os = "windows")]
            if let Err(error) = place_windows_overlay(app_handle, &overlay_window, width, height) {
                log::error!("Failed to re-assert recording overlay position: {error}");
            }

            log::debug!(
                "overlay '{}': set_size={:?} pos_calc={:?} set_pos={:?} show={:?}",
                state,
                size_elapsed,
                pos_calc_elapsed,
                set_pos_elapsed,
                show_elapsed
            );
        }

        SHOWN.store(Shown::of(state) as u8, Ordering::SeqCst);
        let _ = overlay_window.emit("show-overlay", state);
    }
}

/// Notify the visible recording overlay that the input stream has delivered its
/// first sample chunk. Audio feedback uses the same backend readiness signal,
/// but this targeted event is skipped when overlays are disabled.
pub fn emit_recording_ready(app_handle: &AppHandle) {
    if !OVERLAY_ENABLED.load(Ordering::Relaxed) {
        return;
    }

    // Showing the overlay is also queued onto the main thread. Queue readiness
    // there as well so a very fast always-on stream cannot overtake show-overlay
    // and then get reset back to the arming state by the frontend.
    let handle = app_handle.clone();
    let _ = app_handle.run_on_main_thread(move || {
        let _ = handle.emit_to("recording_overlay", "recording-ready", ());
    });
}

/// Tell the visible overlay that the recording is close to the session
/// limit, so the pill turns amber until it hides. Skipped when overlays are
/// disabled; queued on the main thread behind any pending show-overlay.
pub fn emit_recording_limit_warning(app_handle: &AppHandle) {
    if !OVERLAY_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let handle = app_handle.clone();
    let _ = app_handle.run_on_main_thread(move || {
        let _ = handle.emit_to("recording_overlay", "recording-limit-warning", ());
    });
}

/// Shows a notice in the overlay window. `alone` replaces whatever the overlay shows, shows
/// the window even with the overlay turned off (the gesture failed and nothing else says so),
/// and hides it after the notice's duration unless a newer session showed the overlay. Without
/// `alone` the notice stacks over the pill of the current recording.
pub fn show_notice(app_handle: &AppHandle, kind: NoticeKind, device: Option<String>, alone: bool) {
    let duration = notice_duration(kind);
    let payload = NoticePayload {
        kind,
        device,
        alone,
        duration_ms: duration.as_millis() as u64,
    };
    log::debug!("overlay notice {kind:?} (alone: {alone})");
    let handle = app_handle.clone();
    let _ = app_handle.run_on_main_thread(move || {
        show_overlay_state_on_main(&handle, "notice");
        let _ = handle.emit_to("recording_overlay", "overlay-notice", payload);
        if !alone {
            return;
        }
        let shown = OVERLAY_SHOW_GENERATION.load(Ordering::SeqCst);
        std::thread::spawn(move || {
            std::thread::sleep(duration);
            if OVERLAY_SHOW_GENERATION.load(Ordering::SeqCst) == shown {
                hide_recording_overlay(&handle);
            }
        });
    });
}

/// The longest the pill says a paste failed. The page ends it 5 s after the pointer leaves
/// (`end_paste_failed`); this bounds a page that never asks.
const PASTE_FAILED_MAX: std::time::Duration = std::time::Duration::from_secs(60);

/// Payload of `paste-failed`: the show the pill belongs to, which the page hands back to end it.
#[derive(Clone, serde::Serialize)]
struct PasteFailedPayload {
    generation: u32,
}

/// The show generation as the page carries it.
fn generation_token(generation: u64) -> u32 {
    generation as u32
}

/// Whether a failed paste may take the overlay: not over a recording that started meanwhile,
/// and never over the meeting indicator, which stays up for the whole meeting (ADR-0005).
fn paste_failed_shows(recording: bool, meeting: MeetingIndicator) -> bool {
    !recording && overlay_for_meeting(meeting).is_none()
}

/// The paste of a dictation failed: instead of hiding, the pill says so, and a click copies
/// `text` (`copy_failed_paste`). Shown even with the overlay off, like a notice alone: nothing
/// else in front of the user says the text did not go in. Returns `false` when the overlay
/// belongs to a recording or a meeting; the main window's toast still says it then.
pub fn show_paste_failed(app_handle: &AppHandle, text: String) -> bool {
    let recording = app_handle
        .try_state::<std::sync::Arc<crate::managers::audio::AudioRecordingManager>>()
        .is_some_and(|audio| audio.is_recording());
    if !paste_failed_shows(recording, crate::meeting::indicator()) {
        log::debug!("paste failed while the overlay is in use; the pill keeps its state");
        return false;
    }
    let handle = app_handle.clone();
    let _ = app_handle.run_on_main_thread(move || {
        show_overlay_state_on_main(&handle, "paste_failed");
        crate::last_dictation::remember_failed_paste(text);
        let shown = OVERLAY_SHOW_GENERATION.load(Ordering::SeqCst);
        let payload = PasteFailedPayload {
            generation: generation_token(shown),
        };
        let _ = handle.emit_to("recording_overlay", "paste-failed", payload);
        std::thread::spawn(move || {
            std::thread::sleep(PASTE_FAILED_MAX);
            end_paste_failed_shown(&handle, generation_token(shown));
        });
    });
    true
}

/// Ends the pill of show `generation`, only while no newer state replaced it.
fn end_paste_failed_shown(app_handle: &AppHandle, generation: u32) {
    let handle = app_handle.clone();
    let _ = app_handle.run_on_main_thread(move || {
        // On the main thread, where every show runs, so no show lands between check and hide.
        if generation_token(OVERLAY_SHOW_GENERATION.load(Ordering::SeqCst)) != generation {
            return;
        }
        crate::last_dictation::forget_failed_paste();
        hide_recording_overlay(&handle);
    });
}

/// The pill that says the paste failed ran its time without the pointer over it.
#[tauri::command]
#[specta::specta]
pub fn end_paste_failed(app: AppHandle, generation: u32) {
    end_paste_failed_shown(&app, generation);
}

/// The overlay state of the meeting indicator: `meeting` (red dot and time) or
/// `meeting_paused`; `None` when no meeting records.
pub(crate) fn overlay_for_meeting(indicator: MeetingIndicator) -> Option<&'static str> {
    match indicator {
        MeetingIndicator::Recording => Some("meeting"),
        MeetingIndicator::Paused => Some("meeting_paused"),
        MeetingIndicator::None => None,
    }
}

/// Shows the meeting indicator in the pill. Unlike dictation, it ignores `overlay_style`: the
/// indicator is mandatory for as long as a meeting records (ADR-0005).
pub fn show_meeting_overlay(app_handle: &AppHandle) {
    let Some(state) = overlay_for_meeting(crate::meeting::indicator()) else {
        return;
    };
    let handle = app_handle.clone();
    let _ = app_handle.run_on_main_thread(move || show_overlay_state_on_main(&handle, state));
}

/// Makes room for the meeting card over the pill, or gives it back; the card's text comes with
/// `MeetingStatus.call_output`.
pub fn set_meeting_card(app_handle: &AppHandle, on: bool) {
    if MEETING_CARD.swap(on, Ordering::SeqCst) != on {
        show_meeting_overlay(app_handle);
    }
}

/// A dictation started or ended during a meeting. The recording pill may already show, so it is
/// shown again at its new size; the live streaming card is left as it is.
pub fn set_meeting_dictation(app_handle: &AppHandle, on: bool) {
    if MEETING_DICTATION.swap(on, Ordering::SeqCst) != on && RECORDING_PILL.load(Ordering::SeqCst) {
        show_recording_overlay(app_handle);
    }
}

/// Hides the pill once the meeting indicator is gone.
pub fn hide_meeting_overlay(app_handle: &AppHandle) {
    MEETING_CARD.store(false, Ordering::SeqCst);
    hide_recording_overlay(app_handle);
}

/// Shows the recording overlay window with fade-in animation
pub fn show_recording_overlay(app_handle: &AppHandle) {
    show_overlay_state(app_handle, "recording");
}

/// Shows the larger streaming overlay that displays live transcription text
pub fn show_streaming_overlay(app_handle: &AppHandle) {
    show_overlay_state(app_handle, "streaming");
}

/// Shows the transcribing overlay window
pub fn show_transcribing_overlay(app_handle: &AppHandle) {
    show_overlay_state(app_handle, "transcribing");
}

/// Shows the processing overlay window
pub fn show_processing_overlay(app_handle: &AppHandle) {
    show_overlay_state(app_handle, "processing");
}

/// Updates the overlay window position based on current settings
pub fn update_overlay_position(app_handle: &AppHandle) {
    // Positioning queries monitors/cursor (GDK/Xlib on Linux) and moves the
    // window, so it must run on the main thread — see show_overlay_state.
    let handle = app_handle.clone();
    let _ = app_handle.run_on_main_thread(move || update_overlay_position_on_main(&handle));
}

fn update_overlay_position_on_main(app_handle: &AppHandle) {
    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        #[cfg(target_os = "linux")]
        if LAYER_SHELL_ACTIVE.load(Ordering::SeqCst) {
            let position = settings::get_settings(app_handle).overlay_position;
            match overlay_window.gtk_window() {
                Ok(gtk_window) => configure_layer_shell_position(&gtk_window, position),
                Err(error) => log::error!("Failed to access GTK overlay window: {error}"),
            }
            return;
        }

        #[cfg(target_os = "windows")]
        {
            let state = match Shown::load() {
                Shown::Idle => "idle",
                Shown::IdleHover => "idle_hover",
                _ if WINDOWS_OVERLAY_IS_STREAMING.load(Ordering::Relaxed) => "streaming",
                _ => "recording",
            };
            let (width, height) = overlay_dimensions(state);
            if let Err(error) = place_windows_overlay(app_handle, &overlay_window, width, height) {
                log::error!("Failed to update recording overlay position: {error}");
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            // Use the window's current size so centering stays correct whether the
            // overlay is in compact or streaming layout.
            let (width, height) = current_overlay_logical_size(&overlay_window)
                .unwrap_or((OVERLAY_WIDTH, OVERLAY_HEIGHT));
            if let Some((x, y)) = calculate_overlay_position(app_handle, width, height) {
                let _ = overlay_window
                    .set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
            }
        }
    }
}

/// Generation counter bumped every time the overlay is shown. The delayed
/// `hide()` below only unmaps the window if no show happened after it was
/// scheduled, so a hide left over from a finished transcription can never
/// take down the overlay of a session that started in the meantime — e.g. a
/// press the coordinator remembered while the pipeline was busy and started
/// the instant it drained, well inside the 300 ms hide delay.
static OVERLAY_SHOW_GENERATION: AtomicU64 = AtomicU64::new(0);

/// Hides the recording overlay window with fade-out animation
pub fn hide_recording_overlay(app_handle: &AppHandle) {
    // A meeting indicator stays up for the whole recording (ADR-0005): a hide from another path
    // puts the meeting state back instead.
    if overlay_for_meeting(crate::meeting::indicator()).is_some() {
        show_meeting_overlay(app_handle);
        return;
    }
    // Already resting in the idle pill, which stays: no fade out and back in.
    if matches!(Shown::load(), Shown::Idle | Shown::IdleHover) && current_rest() == Rest::Idle {
        return;
    }
    // Always hide the overlay regardless of settings - if setting was changed while recording,
    // we still want to hide it properly
    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        // Snapshot before doing anything observable, so any show that lands
        // after this point invalidates the delayed hide below.
        let scheduled_at = OVERLAY_SHOW_GENERATION.load(Ordering::SeqCst);
        // Emit event to trigger fade-out animation
        let _ = overlay_window.emit("hide-overlay", ());
        // Hide the window (or rest in the idle pill) after a short delay to allow the animation
        // to complete, unless a newer session has shown the overlay again by then.
        let window_clone = overlay_window.clone();
        let handle = app_handle.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            if OVERLAY_SHOW_GENERATION.load(Ordering::SeqCst) != scheduled_at {
                log::debug!("Skipping stale overlay hide: a newer session is showing the overlay");
                return;
            }
            match current_rest() {
                Rest::Idle => {
                    let main = handle.clone();
                    let _ = handle.run_on_main_thread(move || {
                        // Checked again on the main thread, where every show runs.
                        if OVERLAY_SHOW_GENERATION.load(Ordering::SeqCst) == scheduled_at {
                            show_overlay_state_on_main(&main, "idle");
                        }
                    });
                }
                Rest::Meeting(_) => show_meeting_overlay(&handle),
                Rest::Hidden => {
                    let _ = window_clone.hide();
                    SHOWN.store(Shown::Hidden as u8, Ordering::SeqCst);
                }
            }
        });
    }
}

/// What the overlay window shows, so the idle pill never replaces a live state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shown {
    Hidden = 0,
    Idle = 1,
    IdleHover = 2,
    /// Recording, processing, a meeting or a notice.
    Active = 3,
}

impl Shown {
    fn of(state: &str) -> Self {
        match state {
            "idle" => Shown::Idle,
            "idle_hover" => Shown::IdleHover,
            _ => Shown::Active,
        }
    }

    fn load() -> Self {
        match SHOWN.load(Ordering::SeqCst) {
            0 => Shown::Hidden,
            1 => Shown::Idle,
            2 => Shown::IdleHover,
            _ => Shown::Active,
        }
    }
}

static SHOWN: AtomicU8 = AtomicU8::new(Shown::Hidden as u8);

/// The setting, `overlay_style` and the platform allow the idle pill (`idle_wanted`). Cached like
/// `OVERLAY_ENABLED`, and read by the full-screen poll every 2 s.
static IDLE_VISIBLE: AtomicBool = AtomicBool::new(false);

/// A full-screen app is in front (video, game, presentation): the idle pill hides.
static FULLSCREEN: AtomicBool = AtomicBool::new(false);

/// The idle pill ships on Windows only for now (Linux in phase 3, macOS is not a target).
const IDLE_SUPPORTED: bool = cfg!(target_os = "windows");

/// Whether the idle pill may show: the user turned it on, the pill itself is on, and the
/// platform supports it.
fn idle_wanted(setting: bool, style: OverlayStyle, supported: bool) -> bool {
    setting && style != OverlayStyle::None && supported
}

/// Where the overlay window rests once the current state ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rest {
    /// A meeting records: its indicator stays (ADR-0005).
    Meeting(&'static str),
    Idle,
    Hidden,
}

fn rest_after_hide(idle_wanted: bool, meeting: MeetingIndicator, fullscreen: bool) -> Rest {
    if let Some(state) = overlay_for_meeting(meeting) {
        return Rest::Meeting(state);
    }
    if idle_wanted && !fullscreen {
        Rest::Idle
    } else {
        Rest::Hidden
    }
}

fn current_rest() -> Rest {
    rest_after_hide(
        IDLE_VISIBLE.load(Ordering::SeqCst) && !snoozed(IDLE_SNOOZED_UNTIL.load(Ordering::SeqCst)),
        crate::meeting::indicator(),
        FULLSCREEN.load(Ordering::SeqCst),
    )
}

/// "Ocultar por 1 hora" in the pill menu hides the idle pill this long. Only the pill at rest:
/// dictation, notices and the meeting indicator still show (ADR-0005).
const IDLE_SNOOZE: std::time::Duration = std::time::Duration::from_secs(60 * 60);

/// Until when (Unix ms) the idle pill stays hidden; 0 when it is not. Kept in memory: a restart
/// shows the pill again.
static IDLE_SNOOZED_UNTIL: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn snoozed(until_ms: u64) -> bool {
    is_snoozed(until_ms, now_ms())
}

fn is_snoozed(until_ms: u64, now_ms: u64) -> bool {
    now_ms < until_ms
}

/// The snooze timer wakes at least this often, so a clock change or a suspend never leaves the
/// pill hidden much past the end.
const SNOOZE_CHECK: std::time::Duration = std::time::Duration::from_secs(60);

/// How long the snooze timer sleeps next: until the end, at most [`SNOOZE_CHECK`]; `None` once
/// the snooze is over.
fn snooze_wait(until_ms: u64, now_ms: u64) -> Option<std::time::Duration> {
    is_snoozed(until_ms, now_ms)
        .then(|| std::time::Duration::from_millis(until_ms - now_ms).min(SNOOZE_CHECK))
}

static SNOOZE_TIMER_RUNNING: AtomicBool = AtomicBool::new(false);

/// Hides the idle pill for [`IDLE_SNOOZE`], then brings it back. A second snooze pushes the end
/// out and the one timer follows it.
pub fn snooze_idle(app_handle: &AppHandle) {
    IDLE_SNOOZED_UNTIL.store(now_ms() + IDLE_SNOOZE.as_millis() as u64, Ordering::SeqCst);
    log::debug!("idle pill hidden for {} min", IDLE_SNOOZE.as_secs() / 60);
    refresh_idle(app_handle);
    if SNOOZE_TIMER_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    let handle = app_handle.clone();
    std::thread::spawn(move || loop {
        while let Some(wait) = snooze_wait(IDLE_SNOOZED_UNTIL.load(Ordering::SeqCst), now_ms()) {
            std::thread::sleep(wait);
        }
        SNOOZE_TIMER_RUNNING.store(false, Ordering::SeqCst);
        refresh_idle(&handle);
        // Snoozed again between the check and the store: keep this thread.
        if !snoozed(IDLE_SNOOZED_UNTIL.load(Ordering::SeqCst))
            || SNOOZE_TIMER_RUNNING.swap(true, Ordering::SeqCst)
        {
            return;
        }
    });
}

/// Ends a snooze early: turning the idle pill on in Configurações shows it right away.
pub fn end_idle_snooze() {
    IDLE_SNOOZED_UNTIL.store(0, Ordering::SeqCst);
}

/// What a change of the setting or of the full-screen state does to the window: only a hidden or
/// idle window changes; a live state decides when it ends.
#[derive(Debug, PartialEq, Eq)]
enum IdleRefresh {
    ShowIdle,
    Hide,
    Keep,
}

fn idle_refresh(shown: Shown, rest: Rest) -> IdleRefresh {
    match (shown, rest) {
        (Shown::Hidden, Rest::Idle) => IdleRefresh::ShowIdle,
        (Shown::Idle | Shown::IdleHover, Rest::Hidden) => IdleRefresh::Hide,
        _ => IdleRefresh::Keep,
    }
}

/// Shows or hides the idle pill after the setting or the full-screen state changed.
fn refresh_idle(app_handle: &AppHandle) {
    let handle = app_handle.clone();
    let _ = app_handle.run_on_main_thread(move || refresh_idle_on_main(&handle));
}

fn refresh_idle_on_main(app_handle: &AppHandle) {
    match idle_refresh(Shown::load(), current_rest()) {
        IdleRefresh::ShowIdle => show_overlay_state_on_main(app_handle, "idle"),
        IdleRefresh::Hide => {
            if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
                let _ = overlay_window.emit("hide-overlay", ());
                let _ = overlay_window.hide();
            }
            SHOWN.store(Shown::Hidden as u8, Ordering::SeqCst);
        }
        IdleRefresh::Keep => {}
    }
}

/// Applies `pill_idle_visible` and `overlay_style`: at startup and whenever either changes.
pub fn apply_idle_setting(app_handle: &AppHandle, settings: &settings::AppSettings) {
    let wanted = idle_wanted(
        settings.pill_idle_visible,
        settings.overlay_style,
        IDLE_SUPPORTED,
    );
    IDLE_VISIBLE.store(wanted, Ordering::SeqCst);
    #[cfg(target_os = "windows")]
    if wanted {
        FULLSCREEN.store(foreground_is_fullscreen(), Ordering::SeqCst);
        start_fullscreen_poll(app_handle);
    }
    refresh_idle(app_handle);
}

/// The overlay page mounted (at startup, or after a reload): from here on it hears
/// `show-overlay`, so an idle pill shown before it listened is shown again.
#[tauri::command]
#[specta::specta]
pub fn pill_overlay_ready(app: AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if matches!(Shown::load(), Shown::Idle | Shown::IdleHover) {
            SHOWN.store(Shown::Hidden as u8, Ordering::SeqCst);
        }
        refresh_idle_on_main(&handle);
    });
}

/// The cursor entered or left the idle pill: the window grows for the shortcut card or shrinks
/// back, without taking focus.
#[tauri::command]
#[specta::specta]
pub fn set_idle_pill_hover(app: AppHandle, hover: bool) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if !matches!(Shown::load(), Shown::Idle | Shown::IdleHover) {
            return;
        }
        let state = if hover { "idle_hover" } else { "idle" };
        SHOWN.store(Shown::of(state) as u8, Ordering::SeqCst);
        // The pointer may be on its way to a right click: list the microphones for the menu,
        // as the tray does when the pointer reaches its icon.
        if hover {
            crate::tray::refresh_microphones(&handle);
        }
        #[cfg(target_os = "windows")]
        if let Some(overlay_window) = handle.get_webview_window("recording_overlay") {
            let (width, height) = overlay_dimensions(state);
            if let Err(error) = place_windows_overlay(&handle, &overlay_window, width, height) {
                log::error!("Failed to resize the idle pill: {error}");
            }
        }
    });
}

/// A right click on the idle pill opens its menu at the cursor (`tray::build_pill_menu`); the
/// tray's menu handler in `lib.rs` runs the item. Async so the menu inputs are read off the main
/// thread, like the tray's.
#[tauri::command]
#[specta::specta]
pub async fn open_pill_menu(app: AppHandle) {
    if !matches!(Shown::load(), Shown::Idle | Shown::IdleHover) {
        return;
    }
    // Still the user's app: the menu has not taken the focus yet. "Colar o último ditado" gives
    // it back before pasting, like from the tray.
    #[cfg(target_os = "windows")]
    crate::last_dictation::remember_foreground();
    let menu = match crate::tray::build_pill_menu(&app) {
        Ok(menu) => menu,
        Err(error) => {
            log::error!("Failed to build the pill menu: {error}");
            return;
        }
    };
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(overlay_window) = handle.get_webview_window("recording_overlay") else {
            return;
        };
        // Blocks until the menu closes (TrackPopupMenu on Windows).
        if let Err(error) = overlay_window.popup_menu(&menu) {
            log::error!("Failed to open the pill menu: {error}");
        }
        // The menu made the pill the foreground window; a pill that never takes focus hands it
        // back to the app the user was in.
        #[cfg(target_os = "windows")]
        if let Ok(hwnd) = overlay_window.hwnd() {
            crate::last_dictation::return_focus_from(hwnd);
        }
        // The menu held the pointer, so the page never heard it leave: close the shortcut card
        // when the pointer ended up off the pill.
        if Shown::load() == Shown::IdleHover && !pointer_over(&handle, &overlay_window) {
            show_overlay_state_on_main(&handle, "idle");
        }
    });
}

/// Whether the cursor is over the overlay window. Physical on Windows, where the idle pill lives.
fn pointer_over(app_handle: &AppHandle, window: &tauri::webview::WebviewWindow) -> bool {
    match (
        input::get_cursor_position(app_handle),
        window.outer_position(),
        window.outer_size(),
    ) {
        (Some(cursor), Ok(position), Ok(size)) => is_mouse_within_monitor(cursor, &position, &size),
        _ => false,
    }
}

/// A click on the idle pill opens the main window on Início.
#[tauri::command]
#[specta::specta]
pub fn open_home_from_pill(app: AppHandle) {
    crate::show_main_window(&app);
    let _ = app.emit_to("main", "open-home", ());
}

/// A click on the meeting pill opens the main window on Reuniões, on the session it records.
#[tauri::command]
#[specta::specta]
pub fn open_meeting_from_pill(app: AppHandle) {
    let id = app
        .try_state::<std::sync::Arc<crate::meeting::MeetingManager>>()
        .and_then(|meetings| meetings.status().id);
    crate::show_main_window(&app);
    let _ = app.emit_to("main", "open-meeting", id);
}

/// The pointer moved past the drag threshold on the idle or the meeting pill with the button
/// down: the pill follows the cursor along its edge until the button comes up, then the place is
/// saved in `pill_center_x`. Windows only, like the idle pill.
#[tauri::command]
#[specta::specta]
pub fn start_pill_drag(app: AppHandle) {
    #[cfg(target_os = "windows")]
    start_windows_drag(&app);
    #[cfg(not(target_os = "windows"))]
    let _ = app;
}

#[cfg(target_os = "windows")]
static PILL_DRAGGING: AtomicBool = AtomicBool::new(false);

/// How often the dragged pill follows the cursor (~60 Hz).
#[cfg(target_os = "windows")]
const DRAG_STEP: std::time::Duration = std::time::Duration::from_millis(16);

#[cfg(target_os = "windows")]
fn start_windows_drag(app_handle: &AppHandle) {
    if Shown::load() == Shown::Hidden || PILL_DRAGGING.swap(true, Ordering::SeqCst) {
        return;
    }
    let window = app_handle.get_webview_window("recording_overlay");
    let monitor = window
        .as_ref()
        .and_then(|w| w.current_monitor().ok().flatten());
    let (Some(hwnd), Some(monitor)) = (window.and_then(|w| w.hwnd().ok()), monitor) else {
        PILL_DRAGGING.store(false, Ordering::SeqCst);
        return;
    };
    // `HWND` is not `Send`; the drag thread takes the raw handle.
    let raw = hwnd.0 as isize;
    let (monitor_x, monitor_width) = (monitor.position().x, monitor.size().width);
    let handle = app_handle.clone();
    std::thread::spawn(move || {
        let center = drag_until_release(raw, monitor_x, monitor_width);
        PILL_DRAGGING.store(false, Ordering::SeqCst);
        if let Some(center) = center {
            log::debug!("pill dragged to {center:.3} of the monitor width");
            let mut settings = settings::get_settings(&handle);
            settings.pill_center_x = Some(center);
            settings::write_settings(&handle, settings);
        }
    });
}

/// Moves the window along its edge with the cursor while the primary button is down. Returns
/// where its center ended, or `None` when it never moved (a flick released before the first
/// step) or the cursor or the window could not be read.
#[cfg(target_os = "windows")]
fn drag_until_release(raw: isize, monitor_x: i32, monitor_width: u32) -> Option<f64> {
    use windows::Win32::Foundation::{HWND, POINT, RECT};
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetSystemMetrics, GetWindowRect, SetWindowPos, SM_SWAPBUTTON,
        SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
    };

    let hwnd = HWND(raw as *mut _);
    let cursor_x = || {
        let mut point = POINT::default();
        // SAFETY: writes the cursor position into a local.
        unsafe { GetCursorPos(&mut point) }.ok().map(|()| point.x)
    };
    let window_rect = || {
        let mut rect = RECT::default();
        // SAFETY: reads the rectangle of our own window into a local.
        unsafe { GetWindowRect(hwnd, &mut rect) }
            .ok()
            .map(|()| rect)
    };
    // GetAsyncKeyState reads the physical buttons: with them swapped, the primary is the right.
    // SAFETY: no arguments.
    let primary = if unsafe { GetSystemMetrics(SM_SWAPBUTTON) } != 0 {
        VK_RBUTTON
    } else {
        VK_LBUTTON
    };

    let start_cursor = cursor_x()?;
    let start = window_rect()?;
    let start_center = start.left + (start.right - start.left) / 2;
    let mut moved_to = None;
    // SAFETY: reads the button's state; the high bit is set while it is down.
    while unsafe { GetAsyncKeyState(i32::from(primary.0)) } < 0 {
        if let (Some(x), Some(rect)) = (cursor_x(), window_rect()) {
            let center = dragged_center(monitor_x, monitor_width, start_center, x - start_cursor);
            let left = pill_left(monitor_x, monitor_width, rect.right - rect.left, center);
            if left != rect.left {
                // A hover that resizes the window meanwhile places it here too.
                PILL_CENTER.store(center.to_bits(), Ordering::SeqCst);
                moved_to = Some(center);
                // SAFETY: moves our own window; async, so this thread never waits on the UI one.
                let _ = unsafe {
                    SetWindowPos(
                        hwnd,
                        None,
                        left,
                        rect.top,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS,
                    )
                };
            }
        }
        std::thread::sleep(DRAG_STEP);
    }
    moved_to
}

/// Polls the notification state every 2 s while the idle pill is on, so it hides while a
/// full-screen app is in front.
#[cfg(target_os = "windows")]
const FULLSCREEN_POLL: std::time::Duration = std::time::Duration::from_secs(2);

#[cfg(target_os = "windows")]
static FULLSCREEN_POLL_RUNNING: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
fn start_fullscreen_poll(app_handle: &AppHandle) {
    if FULLSCREEN_POLL_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    let handle = app_handle.clone();
    std::thread::spawn(move || loop {
        if !IDLE_VISIBLE.load(Ordering::SeqCst) {
            FULLSCREEN.store(false, Ordering::SeqCst);
            FULLSCREEN_POLL_RUNNING.store(false, Ordering::SeqCst);
            // Turned back on between the check and the store: keep this thread.
            if !IDLE_VISIBLE.load(Ordering::SeqCst)
                || FULLSCREEN_POLL_RUNNING.swap(true, Ordering::SeqCst)
            {
                return;
            }
        }
        let fullscreen = foreground_is_fullscreen();
        if FULLSCREEN.swap(fullscreen, Ordering::SeqCst) != fullscreen {
            log::debug!("full-screen app in front: {fullscreen}");
            refresh_idle(&handle);
        }
        std::thread::sleep(FULLSCREEN_POLL);
    });
}

/// Whether a full-screen app (video, game, presentation) is in front, by the shell's own
/// notification state. A failed query counts as no.
#[cfg(target_os = "windows")]
fn foreground_is_fullscreen() -> bool {
    match unsafe { windows::Win32::UI::Shell::SHQueryUserNotificationState() } {
        Ok(state) => is_fullscreen_state(state),
        Err(error) => {
            log::debug!("SHQueryUserNotificationState failed: {error}");
            false
        }
    }
}

/// A full-screen app, a Direct3D full-screen game or presentation mode.
#[cfg(target_os = "windows")]
fn is_fullscreen_state(state: windows::Win32::UI::Shell::QUERY_USER_NOTIFICATION_STATE) -> bool {
    use windows::Win32::UI::Shell::{
        QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
    };
    state == QUNS_BUSY || state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE
}

// Cached "overlay is enabled" flag, kept in sync with overlay_style. Avoids
// reading the Tauri store on every audio callback (~24 Hz during recording).
// Defaults to false so the audio path doesn't emit until lib.rs::setup
// populates the cache from initial settings.
static OVERLAY_ENABLED: AtomicBool = AtomicBool::new(false);

/// Tracks whether gtk-layer-shell was successfully initialized (Linux only).
/// Used to skip layer-shell calls when the window is a regular fallback.
#[cfg(target_os = "linux")]
static LAYER_SHELL_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Update the cached overlay-enabled flag. Called from `lib.rs` at
/// startup after settings load, and from `change_overlay_style_setting`
/// whenever the user changes whether the overlay is shown.
pub fn update_overlay_enabled_cache(enabled: bool) {
    OVERLAY_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn emit_levels(app_handle: &AppHandle, levels: &[f32]) {
    // Skip emission when the overlay is disabled. The recording_overlay
    // window is created at boot regardless of overlay_style, so without this
    // guard a hidden overlay's WebKit subprocess still
    // processes every event. Each event drives some kind of WebKit
    // C++ allocation that accumulates without bound (mechanism not
    // directly characterized; see issue #1279 for the investigation).
    // For users with `overlay_style: none` (the Linux default) this skip
    // eliminates the upstream driver of that accumulation.
    if !OVERLAY_ENABLED.load(Ordering::Relaxed) {
        return;
    }

    // Throttle to ~30 FPS. Even with the overlay enabled, the raw audio
    // callback fires far faster than the UI needs; capping emission rate
    // cuts the per-frame `eval_script`/IPC volume that drives the wry
    // memory growth in issue #1279 (upstream tauri-apps/wry#1489).
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let last = LAST_MIC_LEVEL_EMIT.load(Ordering::Relaxed);
    if now.saturating_sub(last) < EMIT_THROTTLE_MS {
        return;
    }
    LAST_MIC_LEVEL_EMIT.store(now, Ordering::Relaxed);

    // Target only the overlay window. In Tauri 2 both `AppHandle::emit`
    // and `WebviewWindow::emit` broadcast to all webviews; Tauri's
    // listener filter then skips webviews with no registered listener
    // for the event, so the settings webview never received `mic-level`.
    // But the previous dual-call pattern still produced two `eval_script`
    // calls to the overlay per audio callback (one from each .emit()).
    // `emit_to` with the overlay's window label produces a single
    // eval_script call per callback, cutting the per-callback WebKit
    // dispatch work in half.
    let _ = app_handle.emit_to("recording_overlay", "mic-level", levels);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notice_durations_and_codes() {
        let expected = [
            (NoticeKind::MicInUse, "mic_in_use", 1_500),
            (NoticeKind::MicMuted, "mic_muted", 6_000),
            (NoticeKind::MicDenied, "mic_denied", 6_000),
            (NoticeKind::NoMic, "no_mic", 6_000),
            (NoticeKind::MicFailed, "mic_failed", 6_000),
            (NoticeKind::ModelMissing, "model_missing", 5_000),
        ];
        for (kind, code, ms) in expected {
            assert_eq!(
                serde_json::to_value(kind).unwrap(),
                serde_json::json!(code),
                "{kind:?}"
            );
            assert_eq!(notice_duration(kind).as_millis(), ms, "{kind:?}");
        }
        assert_eq!(overlay_dimensions("notice"), (340.0, 110.0));
        assert_eq!(overlay_dimensions("recording"), (256.0, 50.0));
    }

    #[test]
    fn meeting_overlay_states() {
        assert_eq!(
            overlay_for_meeting(MeetingIndicator::Recording),
            Some("meeting")
        );
        assert_eq!(
            overlay_for_meeting(MeetingIndicator::Paused),
            Some("meeting_paused")
        );
        assert_eq!(overlay_for_meeting(MeetingIndicator::None), None);
        // A dictation during a meeting stacks its card over the recording pill only.
        let card = (OVERLAY_NOTICE_WIDTH, OVERLAY_NOTICE_HEIGHT);
        assert_eq!(sized("recording", false, true), card);
        assert_eq!(
            sized("recording", false, false),
            (OVERLAY_WIDTH, OVERLAY_HEIGHT)
        );
        assert_eq!(
            sized("transcribing", false, true),
            (OVERLAY_WIDTH, OVERLAY_HEIGHT)
        );
        assert_eq!(sized("meeting", true, false), card);
        // The meeting states use the compact pill, never the streaming card.
        assert_eq!(
            overlay_dimensions("meeting"),
            (OVERLAY_WIDTH, OVERLAY_HEIGHT)
        );
        assert_eq!(
            overlay_dimensions("meeting_paused"),
            (OVERLAY_WIDTH, OVERLAY_HEIGHT)
        );
        // ADR-0005: the meeting pill does not consult `overlay_style`, and a hide from another
        // path re-shows it while a meeting records.
        let source = include_str!("overlay.rs");
        let show = &source[source.find("pub fn show_meeting_overlay").unwrap()..];
        let show = &show[..show.find("\n}\n").unwrap()];
        assert!(!show.contains("overlay_style"), "{show}");
        let hide = &source[source.find("pub fn hide_recording_overlay").unwrap()..];
        assert!(
            hide.find("overlay_for_meeting").unwrap() < hide.find("hide-overlay").unwrap(),
            "the meeting check must come before the hide"
        );
    }

    /// The body of a production function, so a structural assertion never matches the tests.
    fn body_of(signature: &str) -> &'static str {
        let source = include_str!("overlay.rs");
        let source = &source[..source.find("#[cfg(test)]\nmod tests").unwrap()];
        let body = &source[source.find(signature).expect(signature)..];
        &body[..body.find("\n}\n").unwrap()]
    }

    #[test]
    fn rest_after_hide_table() {
        use MeetingIndicator::{None as NoMeeting, Paused, Recording};
        for (wanted, fullscreen) in [(false, false), (false, true), (true, false), (true, true)] {
            // A meeting always keeps its indicator (ADR-0005).
            let meeting = rest_after_hide(wanted, Recording, fullscreen);
            assert_eq!(meeting, Rest::Meeting("meeting"));
            let paused = rest_after_hide(wanted, Paused, fullscreen);
            assert_eq!(paused, Rest::Meeting("meeting_paused"));
            let expected = if wanted && !fullscreen {
                Rest::Idle
            } else {
                Rest::Hidden
            };
            assert_eq!(rest_after_hide(wanted, NoMeeting, fullscreen), expected);
        }
        for setting in [false, true] {
            for supported in [false, true] {
                for style in [
                    OverlayStyle::None,
                    OverlayStyle::Minimal,
                    OverlayStyle::Live,
                ] {
                    let expected = setting && supported && style != OverlayStyle::None;
                    assert_eq!(idle_wanted(setting, style, supported), expected);
                }
            }
        }
        assert_eq!(IDLE_SUPPORTED, cfg!(target_os = "windows"));
    }

    #[test]
    fn idle_dimensions() {
        assert_eq!(overlay_dimensions("idle"), (64.0, 20.0));
        assert_eq!(overlay_dimensions("idle_hover"), (300.0, 64.0));
        assert_eq!(overlay_dimensions("recording"), (256.0, 50.0));
        assert_eq!(overlay_dimensions("streaming"), (400.0, 120.0));
        assert_eq!(Shown::of("idle"), Shown::Idle);
        assert_eq!(Shown::of("idle_hover"), Shown::IdleHover);
        for state in ["recording", "streaming", "processing", "meeting", "notice"] {
            assert_eq!(Shown::of(state), Shown::Active, "{state}");
        }
    }

    #[test]
    fn idle_never_replaces_an_active_state() {
        use IdleRefresh::{Hide, Keep, ShowIdle};
        let meeting = Rest::Meeting("meeting");
        for (shown, idle, hidden, in_meeting) in [
            (Shown::Active, Keep, Keep, Keep),
            (Shown::Hidden, ShowIdle, Keep, Keep),
            (Shown::Idle, Keep, Hide, Keep),
            (Shown::IdleHover, Keep, Hide, Keep),
        ] {
            assert_eq!(idle_refresh(shown, Rest::Idle), idle, "{shown:?}");
            assert_eq!(idle_refresh(shown, Rest::Hidden), hidden, "{shown:?}");
            assert_eq!(idle_refresh(shown, meeting), in_meeting, "{shown:?}");
        }
        // The end of a state decides by `current_rest` only after the generation check.
        let hide = body_of("pub fn hide_recording_overlay");
        let stale = hide.find("Skipping stale overlay hide").unwrap();
        assert!(
            stale < hide.rfind("match current_rest()").unwrap(),
            "{hide}"
        );
        assert!(hide.contains("show_overlay_state_on_main(&main, \"idle\")"));
        let refresh = body_of("fn refresh_idle_on_main");
        assert!(refresh.contains("idle_refresh(Shown::load(), current_rest())"));
        let show = body_of("fn show_overlay_state_on_main");
        assert!(show.contains("SHOWN.store(Shown::of(state)"));
    }

    #[test]
    fn paste_failed_pill() {
        assert_eq!(
            overlay_dimensions("paste_failed"),
            (OVERLAY_NOTICE_WIDTH, OVERLAY_NOTICE_HEIGHT)
        );
        assert_eq!(Shown::of("paste_failed"), Shown::Active);
        assert_eq!(PASTE_FAILED_MAX, std::time::Duration::from_secs(60));
        assert_eq!(generation_token(7), 7);
        assert_eq!(generation_token(u64::from(u32::MAX) + 2), 1);
        // Never over a live recording, nor over the meeting indicator (ADR-0005).
        use MeetingIndicator::{None as NoMeeting, Paused, Recording};
        assert!(paste_failed_shows(false, NoMeeting));
        for (recording, meeting) in [(true, NoMeeting), (false, Recording), (false, Paused)] {
            assert!(
                !paste_failed_shows(recording, meeting),
                "{recording} {meeting:?}"
            );
        }
        // Shown like a notice alone, whatever `overlay_style` says; the page learns its show.
        let show = body_of("pub fn show_paste_failed");
        assert!(!show.contains("overlay_style"), "{show}");
        let gate = show.find("if !paste_failed_shows(").unwrap();
        let on = show
            .find("show_overlay_state_on_main(&handle, \"paste_failed\")")
            .unwrap();
        let kept = show.find("remember_failed_paste(text)").unwrap();
        let told = show.find("\"paste-failed\", payload").unwrap();
        assert!(gate < on && on < kept && kept < told, "{show}");
        // The end acts only on its own show, on the main thread, forgetting the text first.
        let end = body_of("fn end_paste_failed_shown");
        assert!(end.contains("run_on_main_thread"), "{end}");
        let check = end.find("!= generation").unwrap();
        let forget = end.find("forget_failed_paste()").unwrap();
        let hide = end.find("hide_recording_overlay(&handle)").unwrap();
        assert!(check < forget && forget < hide, "{end}");
        // Any other state drops the text.
        let any = body_of("fn show_overlay_state_on_main");
        assert!(
            any.contains("if state != \"paste_failed\" {\n            crate::last_dictation::forget_failed_paste();"),
            "{any}"
        );
    }

    #[test]
    fn meeting_pill_opens_its_session() {
        let open = body_of("pub fn open_meeting_from_pill");
        assert!(open.contains("meetings.status().id"), "{open}");
        assert!(open.contains("crate::show_main_window(&app)"), "{open}");
        assert!(
            open.contains("emit_to(\"main\", \"open-meeting\", id)"),
            "{open}"
        );
    }

    #[test]
    fn fullscreen_poll_is_gated() {
        let poll = body_of("fn start_fullscreen_poll");
        assert!(poll.contains("if !IDLE_VISIBLE.load(Ordering::SeqCst)"));
        assert!(poll.contains("std::thread::sleep(FULLSCREEN_POLL)"));
        assert!(include_str!("overlay.rs").contains(
            "const FULLSCREEN_POLL: std::time::Duration = std::time::Duration::from_secs(2);"
        ));
        // The poll starts only when the idle pill is wanted.
        let apply = body_of("pub fn apply_idle_setting");
        assert!(apply.find("if wanted {").unwrap() < apply.find("start_fullscreen_poll").unwrap());
    }

    #[test]
    fn snooze_hides_only_the_idle_pill_for_an_hour() {
        use std::time::Duration;
        assert_eq!(IDLE_SNOOZE.as_secs(), 3_600);
        // Snoozed until `until`, not after; 0 is never snoozed.
        assert!(is_snoozed(1_000, 999));
        assert!(!is_snoozed(1_000, 1_000));
        assert!(!is_snoozed(0, 1));
        // A snoozed pill counts as not wanted: the rest hides it, the meeting keeps its state.
        let rest = body_of("fn current_rest");
        assert!(rest.contains(
            "IDLE_VISIBLE.load(Ordering::SeqCst) && !snoozed(IDLE_SNOOZED_UNTIL.load(Ordering::SeqCst))"
        ));
        assert_eq!(
            idle_refresh(
                Shown::IdleHover,
                rest_after_hide(false, MeetingIndicator::None, false)
            ),
            IdleRefresh::Hide
        );
        // The timer sleeps until the end, waking at least every minute, so a clock change or a
        // suspend never strands the pill.
        assert_eq!(snooze_wait(10_000, 9_000), Some(Duration::from_secs(1)));
        assert_eq!(snooze_wait(10_000_000, 0), Some(SNOOZE_CHECK));
        assert_eq!(SNOOZE_CHECK, Duration::from_secs(60));
        assert_eq!(snooze_wait(10_000, 10_000), None);
        assert_eq!(snooze_wait(0, 5), None);
        // Shown and brought back through `refresh_idle`, which never replaces a live state.
        let snooze = body_of("pub fn snooze_idle");
        let timer = snooze.find("while let Some(wait) = snooze_wait(").unwrap();
        assert!(snooze[..timer].contains("refresh_idle(app_handle)"));
        assert!(snooze[timer..].contains("refresh_idle(&handle)"));
        assert!(snooze[..timer].contains("if SNOOZE_TIMER_RUNNING.swap(true, Ordering::SeqCst) {"));
        // The menu opens only over the idle pill.
        let menu = body_of("pub async fn open_pill_menu");
        assert!(menu.contains("if !matches!(Shown::load(), Shown::Idle | Shown::IdleHover)"));
        assert!(
            menu.find("remember_foreground").unwrap() < menu.find("popup_menu").unwrap()
                && menu.find("popup_menu").unwrap() < menu.find("return_focus_from").unwrap()
        );
        // The page never hears the pointer leave during the menu: the card closes after it.
        let reset = menu
            .find("!pointer_over(&handle, &overlay_window)")
            .unwrap();
        assert!(menu.find("popup_menu").unwrap() < reset);
        assert!(menu[reset..].contains("show_overlay_state_on_main(&handle, \"idle\")"));
        // The focus goes back only from the pill: a click that closed the menu elsewhere stays.
        let focus = include_str!("last_dictation.rs");
        let focus = &focus[focus.find("pub(super) fn return_focus_from").unwrap()..];
        assert!(
            focus.contains("if unsafe { GetForegroundWindow() } != owner {\n            return;")
        );
        // Turning the idle pill on in Configurações ends the snooze.
        let toggle = include_str!("shortcut/mod.rs");
        let toggle = &toggle[toggle
            .find("pub fn change_pill_idle_visible_setting")
            .unwrap()..];
        assert!(
            toggle.find("end_idle_snooze()").unwrap() < toggle.find("apply_idle_setting").unwrap()
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn fullscreen_states() {
        use windows::Win32::UI::Shell::*;
        for state in [
            QUNS_BUSY,
            QUNS_RUNNING_D3D_FULL_SCREEN,
            QUNS_PRESENTATION_MODE,
        ] {
            assert!(is_fullscreen_state(state), "{state:?}");
        }
        for state in [
            QUNS_NOT_PRESENT,
            QUNS_ACCEPTS_NOTIFICATIONS,
            QUNS_QUIET_TIME,
            QUNS_APP,
        ] {
            assert!(!is_fullscreen_state(state), "{state:?}");
        }
    }

    #[test]
    fn monitor_hit_test_uses_half_open_physical_bounds() {
        let position = PhysicalPosition::new(-2560, -200);
        let size = PhysicalSize::new(2560, 1440);

        assert!(is_mouse_within_monitor((-2560, -200), &position, &size));
        assert!(is_mouse_within_monitor((-1, 1239), &position, &size));
        assert!(!is_mouse_within_monitor((0, 0), &position, &size));
        assert!(!is_mouse_within_monitor((-1, 1240), &position, &size));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_cursor_hit_test_does_not_scale_physical_monitor_bounds() {
        let position = PhysicalPosition::new(1920, 0);
        let size = PhysicalSize::new(3840, 2160);
        let cursor = (5000, 1000);

        assert!(is_mouse_within_monitor(cursor, &position, &size));

        // This is the old mixed-coordinate comparison. It excludes a cursor
        // that is visibly inside a secondary display running at 150%.
        let scale = 1.5;
        let logical_position = PhysicalPosition::new(
            (position.x as f64 / scale) as i32,
            (position.y as f64 / scale) as i32,
        );
        let logical_size = PhysicalSize::new(
            (size.width as f64 / scale) as u32,
            (size.height as f64 / scale) as u32,
        );
        assert!(!is_mouse_within_monitor(
            cursor,
            &logical_position,
            &logical_size
        ));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_overlay_bounds_use_destination_monitor_scale() {
        let monitor_position = PhysicalPosition::new(1920, 0);
        let monitor_size = PhysicalSize::new(3840, 2160);

        assert_eq!(
            windows_overlay_bounds(
                monitor_position,
                monitor_size,
                1.5,
                1.0,
                OVERLAY_WIDTH,
                OVERLAY_HEIGHT,
                OverlayPosition::Bottom,
                PILL_CENTERED,
            ),
            (3648, 2025, 384, 75)
        );
        assert_eq!(
            windows_overlay_bounds(
                monitor_position,
                monitor_size,
                1.5,
                1.0,
                OVERLAY_WIDTH,
                OVERLAY_HEIGHT,
                OverlayPosition::Top,
                PILL_CENTERED,
            ),
            (3648, 6, 384, 75)
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn dragged_pill_slides_along_its_edge() {
        // Centered, the left edge is the old centering formula.
        assert_eq!(
            pill_left(1920, 3840, 384, PILL_CENTERED),
            1920 + (3840 - 384) / 2
        );
        assert_eq!(pill_left(-2560, 2560, 500, PILL_CENTERED), -1530);
        // Anywhere along the edge, kept inside the monitor.
        assert_eq!(pill_left(0, 1000, 100, 0.25), 200);
        assert_eq!(pill_left(0, 1000, 100, 0.0), 0);
        assert_eq!(pill_left(0, 1000, 100, 1.0), 900);
        assert_eq!(pill_left(0, 100, 300, 0.5), 0);
        // The center follows the cursor's travel; dragging past the monitor clamps.
        assert_eq!(dragged_center(0, 1000, 500, -250), 0.25);
        assert_eq!(dragged_center(-1000, 1000, -500, 0), 0.5);
        assert_eq!(dragged_center(0, 1000, 500, 900), 1.0);
        assert_eq!(dragged_center(0, 1000, 500, -900), 0.0);
        // The bounds follow the center; the edge (y) does not move.
        let at = |center| {
            windows_overlay_bounds(
                PhysicalPosition::new(0, 0),
                PhysicalSize::new(1000, 800),
                1.0,
                1.0,
                OVERLAY_IDLE_WIDTH,
                OVERLAY_IDLE_HEIGHT,
                OverlayPosition::Bottom,
                center,
            )
        };
        assert_eq!(at(0.25), (218, 740, 64, 20));
        assert_eq!(at(PILL_CENTERED), (468, 740, 64, 20));
    }

    #[test]
    fn pill_center_setting_is_applied_and_reset() {
        set_pill_center(Some(f64::NAN));
        assert_eq!(
            f64::from_bits(PILL_CENTER.load(Ordering::SeqCst)),
            PILL_CENTERED
        );
        // The drag saves where it ended, once the button is up.
        let source = include_str!("overlay.rs");
        let production = &source[..source
            .find(
                "#[cfg(test)]
mod tests",
            )
            .unwrap()];
        let start = &production[production.find("fn start_windows_drag").unwrap()..];
        let saved = start.find("settings.pill_center_x = Some(center)").unwrap();
        assert!(start.find("drag_until_release(raw").unwrap() < saved);
        let release = &production[production.find("fn drag_until_release").unwrap()..];
        assert!(release.contains("while unsafe { GetAsyncKeyState(i32::from(primary.0)) } < 0 {"));
        // A flick that never moved the pill saves nothing.
        assert!(release.contains("let mut moved_to = None;"));
        assert!(release[..release.find("\n}\n").unwrap()].ends_with("    moved_to"));
        // Placement reads the center; startup applies it before the idle pill shows.
        assert!(body_of("fn place_windows_overlay").contains("pill_center(),"));
        let lib = include_str!("lib.rs");
        assert!(
            lib.find("overlay::set_pill_center(settings.pill_center_x)")
                .unwrap()
                < lib
                    .find("overlay::apply_idle_setting(app_handle, &settings)")
                    .unwrap()
        );
        // Choosing the edge recenters the pill.
        let shortcut = include_str!("shortcut/mod.rs");
        let position = &shortcut[shortcut
            .find("pub fn change_overlay_position_setting")
            .unwrap()..];
        let position = &position[..position
            .find(
                "
}
",
            )
            .unwrap()];
        assert!(position.contains("settings.pill_center_x = None;"));
        assert!(position.contains("crate::overlay::set_pill_center(None);"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_overlay_bounds_support_negative_monitor_origins() {
        assert_eq!(
            windows_overlay_bounds(
                PhysicalPosition::new(-2560, -200),
                PhysicalSize::new(2560, 1440),
                1.25,
                1.0,
                OVERLAY_STREAM_WIDTH,
                OVERLAY_STREAM_HEIGHT,
                OverlayPosition::Bottom,
                PILL_CENTERED,
            ),
            (-1530, 1040, 500, 150)
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_overlay_bounds_grow_with_text_scale_without_moving_the_anchored_edge() {
        let monitor_position = PhysicalPosition::new(-2560, -200);
        let monitor_size = PhysicalSize::new(2560, 1440);

        let (x, y, width, height) = windows_overlay_bounds(
            monitor_position,
            monitor_size,
            1.25,
            1.1,
            OVERLAY_STREAM_WIDTH,
            OVERLAY_STREAM_HEIGHT,
            OverlayPosition::Bottom,
            PILL_CENTERED,
        );
        // 400x120 logical at 1.25 DPI x 1.1 text, still centered horizontally.
        assert_eq!((x, y, width, height), (-1555, 1025, 550, 165));
        // Bottom edge unchanged from the 1.0 case above (1040 + 150).
        assert_eq!(y + height, 1190);

        let (_, top_y, _, _) = windows_overlay_bounds(
            monitor_position,
            monitor_size,
            1.25,
            1.1,
            OVERLAY_STREAM_WIDTH,
            OVERLAY_STREAM_HEIGHT,
            OverlayPosition::Top,
            PILL_CENTERED,
        );
        // Top offset rides the DPI scale alone, so the top edge doesn't move.
        assert_eq!(top_y, -195);
    }
}
