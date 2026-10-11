use crate::settings::SoundTheme;
use crate::settings::{self, AppSettings};
use cpal::traits::{DeviceTrait, HostTrait};
use log::{debug, error, warn};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

/// How long the chime output stream stays open after the last chime. A dictation's
/// start and stop chimes share one stream; an idle app holds none.
const CHIME_STREAM_IDLE: Duration = Duration::from_secs(30);
/// How long a blocking caller (the start chime before mute, the test sound) waits for
/// its chime to start playing: the stream opened and the sound decoded.
const CHIME_START_WAIT: Duration = Duration::from_secs(2);
/// The longest one chime may play. Playback is polled against a deadline instead of
/// `Sink::sleep_until_end`, which never returns once the output stream freezes.
const CHIME_PLAY_MAX: Duration = Duration::from_secs(10);
/// How long the playback thread may stay on one chime before the next chime gives up
/// on it and starts a fresh thread. The stuck one is left behind with its stream.
const CHIME_STALL: Duration = Duration::from_secs(15);

pub enum SoundType {
    Start,
    Stop,
}

fn resolve_sound_path(
    app: &AppHandle,
    settings: &AppSettings,
    sound_type: SoundType,
) -> Option<PathBuf> {
    let sound_file = get_sound_path(settings, sound_type);
    let base_dir = get_sound_base_dir(settings);
    match base_dir {
        tauri::path::BaseDirectory::AppData => {
            crate::portable::resolve_app_data(app, &sound_file).ok()
        }
        _ => app.path().resolve(&sound_file, base_dir).ok(),
    }
}

fn get_sound_path(settings: &AppSettings, sound_type: SoundType) -> String {
    match (settings.sound_theme, sound_type) {
        (SoundTheme::Custom, SoundType::Start) => "custom_start.wav".to_string(),
        (SoundTheme::Custom, SoundType::Stop) => "custom_stop.wav".to_string(),
        (_, SoundType::Start) => settings.sound_theme.to_start_path(),
        (_, SoundType::Stop) => settings.sound_theme.to_stop_path(),
    }
}

fn get_sound_base_dir(settings: &AppSettings) -> tauri::path::BaseDirectory {
    match settings.sound_theme {
        SoundTheme::Custom => tauri::path::BaseDirectory::AppData,
        _ => tauri::path::BaseDirectory::Resource,
    }
}

pub fn play_feedback_sound(app: &AppHandle, sound_type: SoundType) {
    let settings = settings::get_settings(app);
    if !settings.audio_feedback {
        return;
    }
    if let Some(path) = resolve_sound_path(app, &settings, sound_type) {
        play_sound_async(app, path);
    }
}

pub fn play_feedback_sound_blocking(app: &AppHandle, sound_type: SoundType) {
    let settings = settings::get_settings(app);
    if !settings.audio_feedback {
        return;
    }
    if let Some(path) = resolve_sound_path(app, &settings, sound_type) {
        play_sound_blocking(app, &path);
    }
}

pub fn play_test_sound(app: &AppHandle, sound_type: SoundType) {
    let settings = settings::get_settings(app);
    if let Some(path) = resolve_sound_path(app, &settings, sound_type) {
        play_sound_blocking(app, &path);
    }
}

/// The session-limit warning: the theme's start chime twice, 150 ms apart.
/// It plays even with feedback sounds off, since it announces that the
/// recording is about to be cut, at the feedback volume and output device.
pub fn play_limit_warning(app: &AppHandle) {
    let app = app.clone();
    thread::spawn(move || {
        play_test_sound(&app, SoundType::Start);
        thread::sleep(Duration::from_millis(150));
        play_test_sound(&app, SoundType::Start);
    });
}

/// A dictation that could not start (no model, or the microphone failed to open): the
/// theme's stop chime twice, 150 ms apart. Like the limit warning, it plays even with
/// feedback sounds off, since nothing else marks the gesture as failed when the pill is off.
pub fn play_error_chime(app: &AppHandle) {
    let app = app.clone();
    thread::spawn(move || {
        play_test_sound(&app, SoundType::Stop);
        thread::sleep(Duration::from_millis(150));
        play_test_sound(&app, SoundType::Stop);
    });
}

fn play_sound_async(app: &AppHandle, path: PathBuf) {
    if !send_chime(chime_request(app, path.clone(), None)) {
        warn!(
            "Sound '{}' was not played: no playback thread",
            path.display()
        );
    }
}

fn play_sound_blocking(app: &AppHandle, path: &Path) {
    let (started_tx, started_rx) = mpsc::channel();
    if !send_chime(chime_request(app, path.to_path_buf(), Some(started_tx))) {
        warn!(
            "Sound '{}' was not played: no playback thread",
            path.display()
        );
        return;
    }
    if !wait_for_chime(&started_rx, CHIME_START_WAIT) {
        warn!(
            "Sound '{}' did not finish in time; continuing without it",
            path.display()
        );
    }
}

/// One chime for the playback thread. `started` receives the sound's length once it
/// starts playing, and is dropped when the chime has finished or failed.
struct ChimeRequest {
    path: PathBuf,
    device: Option<String>,
    volume: f32,
    started: Option<Sender<Option<Duration>>>,
}

fn chime_request(
    app: &AppHandle,
    path: PathBuf,
    started: Option<Sender<Option<Duration>>>,
) -> ChimeRequest {
    let settings = settings::get_settings(app);
    ChimeRequest {
        path,
        device: settings.selected_output_device.clone(),
        volume: settings.audio_feedback_volume,
        started,
    }
}

/// How long a chime of `length` may play: its length plus a second, at most
/// [`CHIME_PLAY_MAX`], which is also the bound when the length is unknown.
fn play_limit(length: Option<Duration>) -> Duration {
    length.map_or(CHIME_PLAY_MAX, |d| {
        (d + Duration::from_secs(1)).min(CHIME_PLAY_MAX)
    })
}

/// Wait for a blocking chime: up to `start_wait` for it to start, then for its length
/// (see [`play_limit`]). `true` when it finished or failed in time, `false` on a timeout.
fn wait_for_chime(started: &Receiver<Option<Duration>>, start_wait: Duration) -> bool {
    match started.recv_timeout(start_wait) {
        Ok(length) => !matches!(
            started.recv_timeout(play_limit(length)),
            Err(RecvTimeoutError::Timeout)
        ),
        Err(RecvTimeoutError::Disconnected) => true,
        Err(RecvTimeoutError::Timeout) => false,
    }
}

/// Poll `done` every `poll` until it holds or `limit` passes; `true` when it held.
fn wait_until(done: impl Fn() -> bool, limit: Duration, poll: Duration) -> bool {
    let started = Instant::now();
    while !done() {
        if started.elapsed() >= limit {
            return false;
        }
        thread::sleep(poll);
    }
    true
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// A playback thread: where to send chimes, and since when it has been on the current
/// one (milliseconds since the Unix epoch, 0 while idle).
struct ChimeThread<R> {
    tx: Sender<R>,
    busy_since: Arc<AtomicU64>,
}

/// The live playback thread. It is replaced when it has been stuck on one chime for
/// longer than the stall bound, or when it is gone (a panic ended it), so one frozen
/// output stream costs the chimes in flight and never every later one.
struct ChimePlayer<R> {
    current: Option<ChimeThread<R>>,
}

impl<R> ChimePlayer<R> {
    const fn new() -> Self {
        Self { current: None }
    }

    fn send(
        &mut self,
        request: R,
        now_ms: u64,
        stall: Duration,
        spawn: impl Fn() -> std::io::Result<ChimeThread<R>>,
    ) -> bool {
        if let Some(thread) = &self.current {
            let since = thread.busy_since.load(Ordering::SeqCst);
            if since != 0 && now_ms.saturating_sub(since) > stall.as_millis() as u64 {
                warn!(
                    "Sound playback has been stuck for over {:?}; starting a new playback thread",
                    stall
                );
                self.current = None;
            }
        }
        let mut request = request;
        for attempt in 0..2 {
            if self.current.is_none() {
                match spawn() {
                    Ok(thread) => self.current = Some(thread),
                    Err(e) => {
                        error!("Failed to start the sound playback thread: {}", e);
                        return false;
                    }
                }
            }
            let Some(thread) = &self.current else {
                return false;
            };
            match thread.tx.send(request) {
                Ok(()) => return true,
                Err(mpsc::SendError(unsent)) => {
                    if attempt == 0 {
                        warn!("Sound playback thread is gone; starting a new one");
                    }
                    request = unsent;
                    self.current = None;
                }
            }
        }
        false
    }
}

static CHIME_PLAYER: Mutex<ChimePlayer<ChimeRequest>> = Mutex::new(ChimePlayer::new());

/// Hand a chime to the playback thread; `false` when no thread could take it.
fn send_chime(request: ChimeRequest) -> bool {
    let mut player = CHIME_PLAYER.lock().unwrap_or_else(|e| e.into_inner());
    player.send(request, now_ms(), CHIME_STALL, spawn_chime_thread)
}

fn spawn_chime_thread() -> std::io::Result<ChimeThread<ChimeRequest>> {
    let (tx, rx) = mpsc::channel();
    let busy_since = Arc::new(AtomicU64::new(0));
    let busy = Arc::clone(&busy_since);
    thread::Builder::new()
        .name("chimes".into())
        .spawn(move || chime_worker(rx, &busy))?;
    Ok(ChimeThread { tx, busy_since })
}

/// The output stream kept between chimes, with the device key it was opened for.
/// Opening a WASAPI output stream for every chime, concurrently with the mic stream
/// stopping on the same path, is what wedged the audio engine in cjpais/Handy#1712.
struct CachedOutput<S> {
    key: Option<String>,
    stream: Option<S>,
}

impl<S> CachedOutput<S> {
    fn new() -> Self {
        Self {
            key: None,
            stream: None,
        }
    }

    /// The stream for `key`, opened only when there is none or the device changed. A
    /// stream for another device is closed before the new one opens.
    fn get_or_open<E>(&mut self, key: &str, open: impl FnOnce() -> Result<S, E>) -> Result<&S, E> {
        let cached = self.stream.take();
        let stream = match cached {
            Some(stream) if self.key.as_deref() == Some(key) => stream,
            stale => {
                drop(stale);
                self.key = None;
                open()?
            }
        };
        self.key = Some(key.to_owned());
        Ok(self.stream.insert(stream))
    }

    fn current(&self) -> Option<&S> {
        self.stream.as_ref()
    }

    fn is_open(&self) -> bool {
        self.stream.is_some()
    }

    fn close(&mut self) {
        self.stream = None;
        self.key = None;
    }
}

/// An open output stream and whether its error callback has fired since (a device
/// unplugged or invalidated), in which case the next chime reopens it.
struct ChimeStream {
    stream: OutputStream,
    failed: Arc<AtomicBool>,
}

/// Plays chimes one after another on a stream reused while chimes keep coming, and
/// closes it after [`CHIME_STREAM_IDLE`] without one or after a failed play.
fn chime_worker(requests: Receiver<ChimeRequest>, busy_since: &AtomicU64) {
    let mut output: CachedOutput<ChimeStream> = CachedOutput::new();
    loop {
        let request = if output.is_open() {
            match requests.recv_timeout(CHIME_STREAM_IDLE) {
                Ok(request) => request,
                Err(RecvTimeoutError::Timeout) => {
                    debug!("Closing the idle sound output stream");
                    output.close();
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match requests.recv() {
                Ok(request) => request,
                Err(_) => break,
            }
        };
        busy_since.store(now_ms().max(1), Ordering::SeqCst);
        if let Err(e) = play_chime(&mut output, &request) {
            error!("Failed to play sound '{}': {}", request.path.display(), e);
            output.close();
        }
        busy_since.store(0, Ordering::SeqCst);
        // Dropping the request releases a caller waiting on `started`.
    }
}

fn play_chime(
    output: &mut CachedOutput<ChimeStream>,
    request: &ChimeRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    if output
        .current()
        .is_some_and(|chime| chime.failed.load(Ordering::SeqCst))
    {
        debug!("Reopening the sound output stream after a stream error");
        output.close();
    }
    let key = output_device_key(request.device.as_deref());
    let chime = output.get_or_open(&key, || open_output_stream(request.device.clone()))?;

    let file = File::open(&request.path)?;
    let source = Decoder::new(BufReader::new(file))?;
    let length = source.total_duration();
    let sink = Sink::connect_new(chime.stream.mixer());
    sink.set_volume(request.volume);
    sink.append(source);
    if let Some(started) = &request.started {
        let _ = started.send(length);
    }

    let limit = play_limit(length);
    if !wait_until(|| sink.empty(), limit, Duration::from_millis(20)) {
        sink.stop();
        return Err(format!("playback did not finish within {limit:?}").into());
    }
    Ok(())
}

/// Which device a stream for `selected` would play on. "Default" resolves to the
/// current OS default's name, so switching headphones and speakers reopens it.
fn output_device_key(selected: Option<&str>) -> String {
    match selected {
        Some(name) if name != "Default" => format!("device:{name}"),
        _ => {
            let host = crate::audio_toolkit::get_cpal_host();
            let name = host
                .default_output_device()
                .and_then(|d| d.name().ok())
                .unwrap_or_default();
            format!("default:{name}")
        }
    }
}

fn open_output_stream(
    selected_device: Option<String>,
) -> Result<ChimeStream, Box<dyn std::error::Error>> {
    let stream_builder = if let Some(device_name) = selected_device {
        if device_name == "Default" {
            debug!("Using default device");
            OutputStreamBuilder::from_default_device()?
        } else {
            let host = crate::audio_toolkit::get_cpal_host();
            let devices = host.output_devices()?;

            let mut found_device = None;
            for device in devices {
                if device.name()? == device_name {
                    found_device = Some(device);
                    break;
                }
            }

            match found_device {
                Some(device) => OutputStreamBuilder::from_device(device)?,
                None => {
                    warn!("Device '{}' not found, using default device", device_name);
                    OutputStreamBuilder::from_default_device()?
                }
            }
        }
    } else {
        debug!("Using default device");
        OutputStreamBuilder::from_default_device()?
    };

    let failed = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&failed);
    let stream = stream_builder
        .with_error_callback(move |e| {
            warn!("Sound output stream error: {}", e);
            flag.store(true, Ordering::SeqCst);
        })
        .open_stream()?;
    Ok(ChimeStream { stream, failed })
}

#[cfg(test)]
mod tests {
    use super::{
        play_limit, wait_for_chime, wait_until, CachedOutput, ChimePlayer, ChimeThread,
        CHIME_PLAY_MAX,
    };
    use std::cell::{Cell, RefCell};
    use std::sync::atomic::AtomicU64;
    use std::sync::{mpsc, Arc};
    use std::time::{Duration, Instant};

    #[test]
    fn chime_stream_is_reused_while_the_device_stays_the_same() {
        let opened = Cell::new(0);
        let open = || -> Result<u32, String> {
            opened.set(opened.get() + 1);
            Ok(opened.get())
        };
        let mut output = CachedOutput::new();
        assert_eq!(output.get_or_open("default:Speakers", open), Ok(&1));
        assert_eq!(output.get_or_open("default:Speakers", open), Ok(&1));
        assert_eq!(opened.get(), 1);
        // The OS default moved to the headphones: a new stream.
        assert_eq!(output.get_or_open("default:Headphones", open), Ok(&2));
        // A failed play or the idle timeout closes it; the next chime reopens.
        output.close();
        assert!(!output.is_open());
        assert_eq!(output.get_or_open("default:Headphones", open), Ok(&3));
        assert_eq!(opened.get(), 3);
    }

    #[test]
    fn chime_stream_that_fails_to_open_is_retried_next_time() {
        let mut output: CachedOutput<u32> = CachedOutput::new();
        assert_eq!(
            output.get_or_open("default:Speakers", || Err("busy".to_string())),
            Err("busy".to_string())
        );
        assert!(!output.is_open());
        assert_eq!(
            output.get_or_open("default:Speakers", || Ok::<u32, String>(7)),
            Ok(&7)
        );
    }

    #[test]
    fn blocking_chime_wait_is_bounded() {
        // Nothing starts: the caller gives up after the start wait.
        let (started_tx, started_rx) = mpsc::channel::<Option<Duration>>();
        let begun = Instant::now();
        assert!(!wait_for_chime(&started_rx, Duration::from_millis(50)));
        assert!(begun.elapsed() < Duration::from_secs(1));
        // A chime that failed or finished drops its sender: no wait at all.
        drop(started_tx);
        assert!(wait_for_chime(&started_rx, Duration::from_millis(50)));
        // A 100 ms chime that never finishes is given up after its length plus 1 s.
        let (started_tx, started_rx) = mpsc::channel();
        started_tx.send(Some(Duration::from_millis(100))).ok();
        let begun = Instant::now();
        assert!(!wait_for_chime(&started_rx, Duration::from_millis(50)));
        let waited = begun.elapsed();
        assert!(waited >= Duration::from_millis(1_000), "{waited:?}");
        assert!(waited < Duration::from_millis(2_000), "{waited:?}");
        drop(started_tx);
    }

    #[test]
    fn a_long_start_sound_is_waited_out_before_mute() {
        // A 5 s custom sound that takes 300 ms here still counts as finished, where a
        // fixed 3 s bound would have muted it mid-play.
        let (started_tx, started_rx) = mpsc::channel();
        let player = std::thread::spawn(move || {
            started_tx.send(Some(Duration::from_secs(5))).ok();
            std::thread::sleep(Duration::from_millis(300));
            drop(started_tx);
        });
        assert!(wait_for_chime(&started_rx, Duration::from_millis(500)));
        player.join().ok();
        assert_eq!(
            play_limit(Some(Duration::from_secs(5))),
            Duration::from_secs(6)
        );
        assert_eq!(play_limit(Some(Duration::from_secs(60))), CHIME_PLAY_MAX);
        assert_eq!(play_limit(None), CHIME_PLAY_MAX);
    }

    #[test]
    fn frozen_playback_is_abandoned_at_its_limit() {
        let begun = Instant::now();
        assert!(!wait_until(
            || false,
            Duration::from_millis(80),
            Duration::from_millis(10)
        ));
        let waited = begun.elapsed();
        assert!(waited >= Duration::from_millis(80) && waited < Duration::from_secs(1));
        let polls = Cell::new(0);
        assert!(wait_until(
            || {
                polls.set(polls.get() + 1);
                polls.get() >= 3
            },
            Duration::from_secs(1),
            Duration::from_millis(1)
        ));
    }

    /// A fake playback thread whose receiver the test controls.
    fn fake_thread(busy_since: u64) -> (ChimeThread<u32>, mpsc::Receiver<u32>) {
        let (tx, rx) = mpsc::channel();
        let thread = ChimeThread {
            tx,
            busy_since: Arc::new(AtomicU64::new(busy_since)),
        };
        (thread, rx)
    }

    #[test]
    fn a_dead_playback_thread_is_replaced() {
        let mut player = ChimePlayer::new();
        let (dead, dead_rx) = fake_thread(0);
        drop(dead_rx);
        player.current = Some(dead);
        let (fresh, fresh_rx) = fake_thread(0);
        let fresh = RefCell::new(Some(fresh));
        let spawned = Cell::new(0);
        let spawn = || {
            spawned.set(spawned.get() + 1);
            fresh
                .borrow_mut()
                .take()
                .ok_or_else(|| std::io::Error::other("spawned twice"))
        };
        assert!(player.send(7, 1_000, Duration::from_secs(15), spawn));
        assert_eq!(fresh_rx.try_recv(), Ok(7));
        assert_eq!(spawned.get(), 1);
    }

    #[test]
    fn a_stuck_playback_thread_is_replaced_and_a_busy_one_kept() {
        let stall = Duration::from_secs(15);
        let mut player = ChimePlayer::new();
        // Busy for 5 s: still within the bound, keeps the chime.
        let (busy, busy_rx) = fake_thread(10_000);
        player.current = Some(busy);
        let no_spawn = || -> std::io::Result<ChimeThread<u32>> {
            Err(std::io::Error::other("must not spawn"))
        };
        assert!(player.send(1, 15_000, stall, no_spawn));
        assert_eq!(busy_rx.try_recv(), Ok(1));
        // Busy for 20 s: stuck, the next chime goes to a new thread.
        let (fresh, fresh_rx) = fake_thread(0);
        let fresh = RefCell::new(Some(fresh));
        let spawn = || {
            fresh
                .borrow_mut()
                .take()
                .ok_or_else(|| std::io::Error::other("spawned twice"))
        };
        assert!(player.send(2, 30_000, stall, spawn));
        assert_eq!(fresh_rx.try_recv(), Ok(2));
        assert!(busy_rx.try_recv().is_err());
    }

    #[test]
    fn a_thread_that_cannot_start_reports_the_chime_lost() {
        let mut player: ChimePlayer<u32> = ChimePlayer::new();
        let spawn =
            || -> std::io::Result<ChimeThread<u32>> { Err(std::io::Error::other("no threads")) };
        assert!(!player.send(1, 1_000, Duration::from_secs(15), spawn));
    }
}
