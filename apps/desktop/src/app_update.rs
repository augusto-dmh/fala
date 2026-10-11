//! Signed self-update (ADR-0018). The app checks the GitHub feed at startup and every ~6 h,
//! downloads a newer version in the background and installs it only on a click ("Reiniciar para
//! atualizar", in the tray and in About) or on quit. [`install_refusal`] is the one place that
//! decides whether installing may close the app now; the frontend has no other way to install.

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};
use tauri_specta::Event;

use crate::meeting::MeetingManager;
use crate::settings::{self, StoredReleaseNote};
use crate::transcription_coordinator::TranscriptionCoordinator;

/// The next minisign public key, for a rotation (ADR-0018 §5). Empty until one is prepared: then
/// it holds the new key, and an update whose signature fails against `plugins.updater.pubkey` is
/// tried against this one before it is refused.
pub const UPDATER_PUBKEY_NEXT: &str = "";

const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const CHECK_JITTER_SECS: u64 = 30 * 60;
/// The first check waits for startup (model load, tray) to settle.
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(20);
/// Points the checks at another manifest, to validate a published prerelease by updating to it
/// before it becomes `latest` (docs/RELEASE.md). The signature is checked against the same keys.
const MANIFEST_OVERRIDE_ENV: &str = "FALA_UPDATE_MANIFEST_URL";
/// Passed to the NSIS installer when the update runs on quit: the template skips the relaunch.
const NO_RELAUNCH_ARG: &str = "/NORELAUNCH";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum UpdatePhase {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Downloading,
    /// Downloaded and verified; installs on click or on quit.
    Ready,
    /// Found, but this copy cannot install it (portable or development build).
    Available,
    Installing,
    Failed,
}

/// Why installing now would lose work. Each one names what is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum InstallRefusal {
    Dictation,
    MeetingRecording,
    PostProcessing,
}

/// What the About section and the tray show; emitted on every change.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type, Event)]
pub struct UpdateStatus {
    pub phase: UpdatePhase,
    /// The version on offer, from `latest.json`.
    pub version: Option<String>,
    pub downloaded: u64,
    pub total: Option<u64>,
    /// The installer in the manifest, for a portable copy to download by hand.
    pub installer_url: Option<String>,
    /// Set when the last install attempt was refused; cleared by the next change.
    pub refusal: Option<InstallRefusal>,
    pub error: Option<String>,
}

/// What is running right now, as far as an install is concerned.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Activity {
    /// Recording, transcribing, formatting or pasting a dictation.
    pub dictation: bool,
    /// Recording, paused, stopping or retaining a meeting's audio.
    pub meeting_recording: bool,
    /// A meeting transcription or notes job.
    pub meeting_processing: bool,
    /// An LLM answer still on its way after the paste (late edit).
    pub llm_late_edit: bool,
}

/// The single install gate (ADR-0018 §3): the NSIS installer closes the app, so nothing that
/// would lose audio or text may be running.
pub fn install_refusal(activity: Activity) -> Option<InstallRefusal> {
    if activity.meeting_recording {
        Some(InstallRefusal::MeetingRecording)
    } else if activity.dictation {
        Some(InstallRefusal::Dictation)
    } else if activity.meeting_processing || activity.llm_late_edit {
        Some(InstallRefusal::PostProcessing)
    } else {
        None
    }
}

/// No downgrade: only a strictly newer version is an update (`0.2.0-rc.1` < `0.2.0`).
fn offers_update<V: PartialOrd>(running: &V, offered: &V) -> bool {
    offered > running
}

/// "Depois" dismisses that version for the automatic checks; a newer one, or "Verificar agora",
/// offers again.
fn should_offer(version: &str, dismissed: Option<&str>, manual: bool) -> bool {
    manual || dismissed != Some(version)
}

/// 6 h plus up to 30 min, so installs spread out instead of hitting GitHub at the same minute.
fn next_check_delay(seed: u32) -> Duration {
    CHECK_INTERVAL + Duration::from_secs(u64::from(seed) % (CHECK_JITTER_SECS + 1))
}

fn jitter_seed() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0)
}

/// Tries `attempt` with the configured key and, after a signature failure only, with
/// [`UPDATER_PUBKEY_NEXT`] when there is one.
async fn with_key_fallback<T, E, F, Fut>(
    next_key: &str,
    is_signature_error: impl Fn(&E) -> bool,
    mut attempt: F,
) -> Result<T, E>
where
    F: FnMut(Option<String>) -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    match attempt(None).await {
        Err(e) if is_signature_error(&e) && !next_key.is_empty() => {
            warn!("Update signature does not verify with the current key; trying the next key");
            attempt(Some(next_key.to_string())).await
        }
        other => other,
    }
}

fn is_signature_error(e: &tauri_plugin_updater::Error) -> bool {
    use tauri_plugin_updater::Error;
    matches!(
        e,
        Error::Minisign(_) | Error::Base64(_) | Error::SignatureUtf8(_)
    )
}

struct Ready {
    update: Update,
    /// The same release built with [`NO_RELAUNCH_ARG`], for the install on quit.
    on_quit: Option<Update>,
    bytes: Vec<u8>,
}

#[derive(Default)]
pub struct UpdateManager {
    status: Mutex<UpdateStatus>,
    ready: Mutex<Option<Ready>>,
    /// One check or download at a time.
    in_flight: AtomicBool,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl UpdateManager {
    pub fn status(&self) -> UpdateStatus {
        lock(&self.status).clone()
    }

    pub fn ready_version(&self) -> Option<String> {
        lock(&self.ready).as_ref().map(|r| r.update.version.clone())
    }
}

fn manager(app: &AppHandle) -> Option<tauri::State<'_, UpdateManager>> {
    app.try_state::<UpdateManager>()
}

fn set_status(app: &AppHandle, change: impl FnOnce(&mut UpdateStatus)) {
    let Some(manager) = manager(app) else { return };
    let status = {
        let mut status = lock(&manager.status);
        status.refusal = None;
        status.error = None;
        change(&mut status);
        status.clone()
    };
    if let Err(e) = status.emit(app) {
        debug!("Failed to emit the update status: {e}");
    }
}

/// Checks are on: the build ships the updater, the user keeps the toggle on and
/// `FALA_DISABLE_UPDATER` is not set.
fn checks_enabled(app: &AppHandle) -> bool {
    settings::UPDATER_ENABLED
        && settings::update_checks_effectively_enabled(&settings::get_settings(app))
}

/// Only an installed release bundle can replace itself.
fn installable() -> bool {
    !cfg!(debug_assertions) && !crate::portable::is_portable()
}

fn current_activity(app: &AppHandle) -> Activity {
    let dictation = app
        .try_state::<TranscriptionCoordinator>()
        .is_some_and(|c| c.is_busy());
    let (meeting_recording, meeting_processing) = app
        .try_state::<Arc<MeetingManager>>()
        .map(|m| m.update_activity())
        .unwrap_or_default();
    Activity {
        dictation,
        meeting_recording,
        meeting_processing,
        llm_late_edit: crate::llm_auto::late_edit_in_flight(),
    }
}

async fn check_with_key(
    app: &AppHandle,
    key: Option<String>,
    no_relaunch: bool,
) -> Result<Option<Update>, tauri_plugin_updater::Error> {
    let mut builder = app
        .updater_builder()
        .version_comparator(|running, release| offers_update(&running, &release.version));
    if let Some(key) = key {
        builder = builder.pubkey(key);
    }
    if let Some(url) = std::env::var(MANIFEST_OVERRIDE_ENV)
        .ok()
        .and_then(|v| v.parse::<tauri::Url>().ok())
    {
        builder = builder.endpoints(vec![url])?;
    }
    if no_relaunch {
        builder = builder.installer_arg(NO_RELAUNCH_ARG);
    }
    builder.build()?.check().await
}

/// Checks the feed and, when a newer version is on offer, downloads and verifies it. `manual` is
/// "Verificar agora": it runs even after "Depois" and reports "up to date".
pub async fn check(app: AppHandle, manual: bool) {
    if !checks_enabled(&app) {
        return;
    }
    let Some(manager) = manager(&app) else { return };
    if manager.in_flight.swap(true, Ordering::AcqRel) {
        return;
    }
    let ready = manager.ready_version();
    if ready.is_none() {
        set_status(&app, |s| {
            *s = UpdateStatus {
                phase: UpdatePhase::Checking,
                ..Default::default()
            }
        });
    }
    let result = check_and_download(&app, manual, ready).await;
    manager.in_flight.store(false, Ordering::Release);
    if let Err(e) = result {
        warn!("Update check failed: {e}");
        set_status(&app, |s| {
            s.phase = UpdatePhase::Failed;
            s.error = Some(e.to_string());
        });
    }
}

async fn check_and_download(
    app: &AppHandle,
    manual: bool,
    ready: Option<String>,
) -> Result<(), tauri_plugin_updater::Error> {
    let Some(update) = check_with_key(app, None, false).await? else {
        if ready.is_none() {
            set_status(app, |s| s.phase = UpdatePhase::UpToDate);
        }
        return Ok(());
    };
    if ready.as_deref() == Some(update.version.as_str()) {
        return Ok(());
    }
    let dismissed = settings::get_settings(app).dismissed_update_version;
    if !should_offer(&update.version, dismissed.as_deref(), manual) {
        debug!("Update {} was dismissed; not offering it", update.version);
        set_status(app, |s| s.phase = UpdatePhase::Idle);
        return Ok(());
    }
    let version = update.version.clone();
    let installer_url = update.download_url.to_string();
    if !installable() {
        set_status(app, |s| {
            s.phase = UpdatePhase::Available;
            s.version = Some(version);
            s.installer_url = Some(installer_url);
        });
        return Ok(());
    }

    info!("Downloading update {version}");
    set_status(app, |s| {
        *s = UpdateStatus {
            phase: UpdatePhase::Downloading,
            version: Some(version.clone()),
            installer_url: Some(installer_url.clone()),
            ..Default::default()
        }
    });
    let (update, bytes) = with_key_fallback(UPDATER_PUBKEY_NEXT, is_signature_error, |key| {
        download(app, update.clone(), key)
    })
    .await?;
    // The install on quit needs the same release with another installer argument, which the
    // plugin only takes before the check.
    let on_quit = match check_with_key(app, None, true).await {
        Ok(Some(u)) if u.version == update.version => Some(u),
        _ => None,
    };
    info!("Update {version} downloaded and verified");
    if let Some(manager) = manager(app) {
        *lock(&manager.ready) = Some(Ready {
            update,
            on_quit,
            bytes,
        });
    }
    set_status(app, |s| s.phase = UpdatePhase::Ready);
    crate::tray::update_tray_menu(app);
    Ok(())
}

/// Downloads `update`; with `key`, checks again under that key first, since an `Update` keeps
/// the key it was found with.
async fn download(
    app: &AppHandle,
    update: Update,
    key: Option<String>,
) -> Result<(Update, Vec<u8>), tauri_plugin_updater::Error> {
    let update = match key {
        None => update,
        Some(key) => match check_with_key(app, Some(key), false).await? {
            Some(u) => u,
            None => return Err(tauri_plugin_updater::Error::ReleaseNotFound),
        },
    };
    let (mut downloaded, mut shown) = (0u64, 0u64);
    let progress_app = app.clone();
    let bytes = update
        .download(
            move |chunk, total| {
                downloaded += chunk as u64;
                // One event per percent (per MiB without a length), not one per chunk.
                let step = total.map_or(1 << 20, |t| (t / 100).max(1));
                if downloaded / step != shown / step {
                    shown = downloaded;
                    set_status(&progress_app, |s| {
                        s.downloaded = downloaded;
                        s.total = total;
                    });
                }
            },
            || {},
        )
        .await?;
    Ok((update, bytes))
}

/// Installs the downloaded update if nothing is running. On Windows the installer takes over and
/// this process exits; `relaunch` is false on quit, so the app does not come back.
pub fn install(app: &AppHandle, relaunch: bool) -> Result<(), InstallRefusal> {
    if let Some(refusal) = install_refusal(current_activity(app)) {
        info!("Update install refused: {refusal:?}");
        set_status(app, |s| s.refusal = Some(refusal));
        return Err(refusal);
    }
    let Some(manager) = manager(app) else {
        return Ok(());
    };
    let ready = {
        let mut slot = lock(&manager.ready);
        if !relaunch && slot.as_ref().is_some_and(|r| r.on_quit.is_none()) {
            debug!("No on-quit installer for this update; it waits for the next run");
            return Ok(());
        }
        match slot.take() {
            Some(ready) => ready,
            None => return Ok(()),
        }
    };
    let update = match (&ready.on_quit, relaunch) {
        (Some(on_quit), false) => on_quit,
        _ => &ready.update,
    };
    settings::remember_release_notes(
        app,
        StoredReleaseNote {
            version: update.version.clone(),
            markdown: update.body.clone().unwrap_or_default(),
        },
    );
    info!("Installing update {}", update.version);
    set_status(app, |s| s.phase = UpdatePhase::Installing);
    crate::tray::update_tray_menu(app);
    match update.install(&ready.bytes) {
        // Windows never gets here; elsewhere the files are replaced in place.
        Ok(()) => {
            if relaunch {
                app.restart();
            }
        }
        Err(e) => {
            warn!("Update install failed: {e}");
            set_status(app, |s| {
                s.phase = UpdatePhase::Failed;
                s.error = Some(e.to_string());
            });
        }
    }
    Ok(())
}

/// The install on quit (ADR-0018 §3); a refusal keeps the update for the next run.
pub fn install_on_quit(app: &AppHandle) {
    if manager(app).is_some_and(|m| m.ready_version().is_some()) {
        let _ = install(app, false);
    }
}

/// Startup check, then one every ~6 h. A disabled toggle skips the check but keeps the loop, so
/// turning it back on needs no restart.
pub fn spawn_checks(app: &AppHandle) {
    if !settings::UPDATER_ENABLED {
        return;
    }
    app.manage(UpdateManager::default());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK_DELAY).await;
        loop {
            check(app.clone(), false).await;
            tokio::time::sleep(next_check_delay(jitter_seed())).await;
        }
    });
}

#[tauri::command]
#[specta::specta]
pub fn update_status(app: AppHandle) -> UpdateStatus {
    manager(&app).map(|m| m.status()).unwrap_or_default()
}

#[tauri::command]
#[specta::specta]
pub async fn check_for_updates_now(app: AppHandle) -> UpdateStatus {
    check(app.clone(), true).await;
    update_status(app)
}

#[tauri::command]
#[specta::specta]
pub fn install_update(app: AppHandle) -> Result<(), InstallRefusal> {
    install(&app, true)
}

/// "Depois": that version is not offered again by the automatic checks.
#[tauri::command]
#[specta::specta]
pub fn dismiss_update(app: AppHandle) {
    let Some(manager) = manager(&app) else { return };
    let version = lock(&manager.ready)
        .take()
        .map(|r| r.update.version)
        .or_else(|| manager.status().version);
    if let Some(version) = version {
        let mut settings = settings::get_settings(&app);
        settings.dismissed_update_version = Some(version);
        settings::write_settings(&app, settings);
    }
    set_status(&app, |s| *s = UpdateStatus::default());
    crate::tray::update_tray_menu(&app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn refuses_during_each_capture() {
        let idle = Activity::default();
        assert_eq!(install_refusal(idle), None);
        let cases = [
            (
                Activity {
                    dictation: true,
                    ..idle
                },
                InstallRefusal::Dictation,
            ),
            (
                Activity {
                    meeting_recording: true,
                    ..idle
                },
                InstallRefusal::MeetingRecording,
            ),
            (
                Activity {
                    meeting_processing: true,
                    ..idle
                },
                InstallRefusal::PostProcessing,
            ),
            (
                Activity {
                    llm_late_edit: true,
                    ..idle
                },
                InstallRefusal::PostProcessing,
            ),
            (
                Activity {
                    dictation: true,
                    meeting_recording: true,
                    ..idle
                },
                InstallRefusal::MeetingRecording,
            ),
        ];
        for (activity, want) in cases {
            assert_eq!(install_refusal(activity), Some(want), "{activity:?}");
        }
    }

    #[derive(Debug, PartialEq)]
    enum Fake {
        Signature,
        Network,
    }

    fn run(
        next_key: &str,
        results: Vec<Result<u8, Fake>>,
    ) -> (Result<u8, Fake>, Vec<Option<String>>) {
        let calls = RefCell::new(Vec::new());
        let results = RefCell::new(results.into_iter());
        let out = tauri::async_runtime::block_on(with_key_fallback(
            next_key,
            |e: &Fake| *e == Fake::Signature,
            |key| {
                calls.borrow_mut().push(key);
                let next = results.borrow_mut().next().unwrap_or(Err(Fake::Network));
                async move { next }
            },
        ));
        (out, calls.into_inner())
    }

    #[test]
    fn second_key_only_after_a_signature_failure() {
        let next = Some("NEXT".to_string());
        assert_eq!(run("NEXT", vec![Ok(1)]), (Ok(1), vec![None]));
        assert_eq!(
            run("NEXT", vec![Err(Fake::Signature), Ok(2)]),
            (Ok(2), vec![None, next.clone()])
        );
        assert_eq!(
            run("NEXT", vec![Err(Fake::Signature), Err(Fake::Signature)]),
            (Err(Fake::Signature), vec![None, next])
        );
        assert_eq!(
            run("NEXT", vec![Err(Fake::Network)]),
            (Err(Fake::Network), vec![None])
        );
        assert_eq!(
            run("", vec![Err(Fake::Signature)]),
            (Err(Fake::Signature), vec![None])
        );
    }

    fn version(v: &str) -> impl PartialOrd {
        let release: tauri_plugin_updater::RemoteRelease = serde_json::from_value(
            serde_json::json!({ "version": v, "url": "https://example.invalid/x", "signature": "" }),
        )
        .expect("a valid manifest");
        release.version
    }

    #[test]
    fn only_a_newer_version_is_offered() {
        assert!(!offers_update(&version("0.1.0"), &version("0.1.0")));
        assert!(!offers_update(&version("0.1.0"), &version("0.0.9")));
        assert!(offers_update(&version("0.1.0"), &version("0.1.1")));
        assert!(offers_update(&version("0.1.9"), &version("0.2.0-rc.1")));
        assert!(!offers_update(&version("0.2.0"), &version("0.2.0-rc.1")));
    }

    #[test]
    fn later_dismisses_that_version_only() {
        assert!(!should_offer("0.2.0", Some("0.2.0"), false));
        assert!(should_offer("0.2.1", Some("0.2.0"), false));
        assert!(should_offer("0.2.0", Some("0.2.0"), true));
        assert!(should_offer("0.2.0", None, false));
    }

    #[test]
    fn checks_every_six_hours_with_jitter() {
        let low = Duration::from_secs(6 * 3600);
        let high = Duration::from_secs(6 * 3600 + 30 * 60);
        for seed in [0, 1, 1800, 1801, u32::MAX] {
            let delay = next_check_delay(seed);
            assert!(delay >= low && delay <= high, "{seed}: {delay:?}");
        }
    }
}
