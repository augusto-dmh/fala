use crate::actions::ACTION_MAP;
use crate::managers::audio::AudioRecordingManager;
use crate::settings::ShortcutActivation;
use log::{debug, error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

const DEBOUNCE: Duration = Duration::from_millis(30);
const RELEASE_GRACE: Duration = Duration::from_millis(50);
/// Push-to-talk with double-tap: a second key-down at most this long after
/// the first one locks the session hands-free. Without it, a short tap is
/// discarded when the window ends.
const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(500);
/// A continuous recording, counted from the key-down that opened it, warns
/// at `SESSION_WARNING` and is cut at `SESSION_LIMIT`. The cut is a regular
/// stop: what was said is transcribed and delivered, never discarded.
const SESSION_WARNING: Duration = Duration::from_secs(19 * 60);
const SESSION_LIMIT: Duration = Duration::from_secs(20 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PttAction {
    Passthrough,
    DeferRelease,
    CancelRelease,
}

/// A key-up deferred by `RELEASE_GRACE` so a synthesized X11 auto-repeat
/// press can cancel it (#1539). When the grace elapses the hold is resolved
/// by [`CoordinatorState::finish_hold`] (recording) or
/// [`CoordinatorState::finish_pending_hold`] (press remembered while busy).
struct PendingRelease {
    binding_id: String,
    hotkey_string: String,
    deadline: Instant,
    /// When the key actually went up. The hold duration is measured to this
    /// instant, not to the grace expiry.
    released_at: Instant,
    /// Holds at least this long stop recording; shorter ones lock it on.
    /// Push-to-talk passes zero so every release stops.
    hold_threshold: Duration,
    /// The activation mode of the release, which decides what a short tap
    /// means: lock (hold-or-toggle) or wait for a second tap (double-tap).
    mode: ShortcutActivation,
}

/// A press that arrived while the pipeline was still busy processing the
/// previous transcription. Toggle-style triggers (SIGUSR2, CLI flags, some
/// pedal setups) flip state on every edge, so dropping a busy press desyncs
/// the parity: the next edge starts a recording nobody will ever stop.
struct PendingPress {
    binding_id: String,
    hotkey_string: String,
    /// The real key-down time, so a hold that straddles the drain is still
    /// measured from when the user pressed, not from when recording began.
    pressed_at: Instant,
    /// The recording will start locked on when the pipeline drains: set from
    /// the start for toggle, and for hold-or-toggle once the key came back up
    /// within the threshold (a tap). An unlocked pending press is a key we
    /// believe is still held.
    locked: bool,
}

impl PendingPress {
    fn remembered(&self) -> Remembered {
        if self.locked {
            Remembered::Locked
        } else {
            Remembered::Held
        }
    }
}

/// What kind of press is already waiting for the pipeline to drain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Remembered {
    /// The key is still down as far as we know.
    Held,
    /// A toggle press or a classified tap: it will start a locked session.
    Locked,
}

/// Bookkeeping for the key press that started the current recording.
struct Hold {
    pressed_at: Instant,
    /// Recording outlives the key: the next press stops it, releases are
    /// ignored. Always set for toggle; set for hold-or-toggle once a release
    /// has been classified as a tap.
    locked: bool,
    /// Push-to-talk with double-tap, after a short tap: a second key-down up to
    /// this instant locks the session; reaching it without one discards it.
    tap_deadline: Option<Instant>,
    /// The hotkey that started the recording, for the stop at the session limit.
    hotkey_string: String,
    /// The session-limit warning has been given for this recording.
    limit_warned: bool,
}

/// What to do with an input that arrives while the pipeline is busy
/// (`Stage::Processing`). `remembered` is the press for the same binding
/// already waiting for the pipeline to drain, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BusyAction {
    /// Ignore the input entirely.
    Ignore,
    /// Remember the press; start recording when the pipeline finishes.
    Remember,
    /// This press cancels a previously remembered press: two presses during
    /// one busy window net to no-op, exactly as a press stops a locked
    /// session once recording.
    Forget,
}

fn classify_busy_input(
    is_pressed: bool,
    mode: ShortcutActivation,
    remembered: Option<Remembered>,
) -> BusyAction {
    use ShortcutActivation::*;
    match (mode, is_pressed, remembered) {
        // Toggle: presses alternate remember/forget to preserve parity.
        (Toggle, true, Some(_)) => BusyAction::Forget,
        (Toggle, true, None) => BusyAction::Remember,
        // Toggle mode ignores releases.
        (Toggle, false, _) => BusyAction::Ignore,
        // Hold modes: a press while busy means the user is holding the key —
        // start as soon as the pipeline drains. A press on a queued tap stops
        // it (parity); a press while the key is already down is a repeat.
        (PushToTalk | HoldOrToggle | PushToTalkDoubleTap, true, None) => BusyAction::Remember,
        (PushToTalk | HoldOrToggle | PushToTalkDoubleTap, true, Some(Remembered::Locked)) => {
            BusyAction::Forget
        }
        (PushToTalk | HoldOrToggle | PushToTalkDoubleTap, true, Some(Remembered::Held)) => {
            BusyAction::Ignore
        }
        // Releases of a held pending press are deferred by the grace window
        // before reaching here and resolved by `finish_pending_hold`; any
        // other release (no press remembered, or already locked) is noise.
        (PushToTalk | HoldOrToggle | PushToTalkDoubleTap, false, _) => BusyAction::Ignore,
    }
}

/// Pipeline lifecycle.
#[derive(Debug, PartialEq, Eq)]
enum Stage {
    Idle,
    Recording(String), // binding_id
    Processing,
}

/// A keyboard/signal edge for a transcribe binding.
struct InputEvent {
    binding_id: String,
    hotkey_string: String,
    is_pressed: bool,
    mode: ShortcutActivation,
    /// Hold-or-toggle: minimum press duration that counts as a hold.
    hold_threshold: Duration,
    /// External triggers (SIGUSR2, CLI flags) rather than physical keys.
    /// They fire on every edge by design and must never be debounced —
    /// dropping one desyncs toggle parity and wedges recording on.
    external: bool,
}

impl InputEvent {
    /// The hold duration at or above which a release stops recording.
    fn effective_hold_threshold(&self) -> Duration {
        match self.mode {
            ShortcutActivation::HoldOrToggle | ShortcutActivation::PushToTalkDoubleTap => {
                self.hold_threshold
            }
            // Every release stops; toggle never defers releases at all.
            ShortcutActivation::PushToTalk | ShortcutActivation::Toggle => Duration::ZERO,
        }
    }
}

/// A side effect decided by [`CoordinatorState`]; the coordinator thread is
/// the only executor. Keeping decisions pure lets tests drive the exact
/// production transitions without a Tauri `AppHandle` or real timers.
#[derive(Debug, PartialEq, Eq)]
enum Effect {
    Start {
        binding_id: String,
        hotkey_string: String,
    },
    Stop {
        binding_id: String,
        hotkey_string: String,
    },
    /// End the recording without processing it: a lone short tap whose
    /// double-tap window ran out.
    Discard { binding_id: String },
    /// The recording is about to reach the session limit; it keeps going.
    LimitWarning { binding_id: String },
}

/// Commands processed sequentially by the coordinator thread.
enum Command {
    Input(InputEvent),
    Cancel { recording_was_active: bool },
    ProcessingFinished,
}

/// Decide whether a key-up should be deferred (so auto-repeat can cancel it)
/// or a key-down cancels a deferred release. `hold_to_talk` is whether a
/// release currently ends the session: true for push-to-talk and for an
/// unlocked hold-or-toggle session, false for toggle and a locked session.
/// `held_binding` is the binding whose key we believe is down — the one
/// recording, or the one remembered while the pipeline is busy.
fn classify_ptt_event(
    pending_release_binding: Option<&str>,
    is_pressed: bool,
    hold_to_talk: bool,
    binding_id: &str,
    held_binding: Option<&str>,
) -> PttAction {
    if !hold_to_talk {
        return PttAction::Passthrough;
    }

    if is_pressed {
        if pending_release_binding == Some(binding_id) {
            PttAction::CancelRelease
        } else {
            PttAction::Passthrough
        }
    } else if held_binding == Some(binding_id) && pending_release_binding.is_none() {
        PttAction::DeferRelease
    } else {
        PttAction::Passthrough
    }
}

/// Pure lifecycle state machine: owns every transition decision (release
/// grace, hold-vs-tap classification, debounce, busy-pipeline
/// remember/forget, cancel, drain). Produces [`Effect`]s instead of touching
/// the app, so unit tests exercise the real production logic.
///
/// All three activation modes run through one machine. A recording starts on
/// key-down in every mode; what differs is how it ends:
///
/// * push-to-talk — every release stops (hold threshold of zero)
/// * toggle — releases are ignored, the next press stops (locked from the start)
/// * hold-or-toggle — a release after a long hold stops; a release after a
///   short tap locks the session, and the next press stops
/// * push-to-talk with double-tap — a release after a long hold stops; after a
///   short tap, a second press within `DOUBLE_TAP_WINDOW` of the first locks
///   the session (the next press stops), and no second press discards it
struct CoordinatorState {
    stage: Stage,
    hold: Option<Hold>,
    last_press: Option<Instant>,
    pending_release: Option<PendingRelease>,
    pending_press: Option<PendingPress>,
}

impl CoordinatorState {
    fn new() -> Self {
        Self {
            stage: Stage::Idle,
            hold: None,
            last_press: None,
            pending_release: None,
            pending_press: None,
        }
    }

    /// Deadline of the deferred release, if any — drives `recv_timeout`.
    fn grace_deadline(&self) -> Option<Instant> {
        self.pending_release.as_ref().map(|p| p.deadline)
    }

    /// The earliest instant the coordinator thread must wake up at: the
    /// deferred release's grace or the double-tap window, whichever is first.
    fn next_deadline(&self) -> Option<Instant> {
        let tap = self.hold.as_ref().and_then(|h| h.tap_deadline);
        match (self.grace_deadline(), tap) {
            (Some(grace), Some(tap)) => Some(grace.min(tap)),
            (grace, tap) => grace.or(tap),
        }
    }

    /// The next session mark of the live recording: the warning until it has
    /// been given, then the cut. `None` unless recording.
    fn session_deadline(&self) -> Option<Instant> {
        if !matches!(self.stage, Stage::Recording(_)) {
            return None;
        }
        let hold = self.hold.as_ref()?;
        let mark = if hold.limit_warned {
            SESSION_LIMIT
        } else {
            SESSION_WARNING
        };
        Some(hold.pressed_at + mark)
    }

    /// When the coordinator thread must wake up: the earliest input deadline
    /// ([`CoordinatorState::next_deadline`]) or the session mark.
    fn wake_deadline(&self) -> Option<Instant> {
        match (self.next_deadline(), self.session_deadline()) {
            (Some(input), Some(session)) => Some(input.min(session)),
            (input, session) => input.or(session),
        }
    }

    /// A dictation is in flight: recording, or the pipeline still processing
    /// it. The cancel key is armed exactly while this holds.
    fn is_busy(&self) -> bool {
        !matches!(self.stage, Stage::Idle)
    }

    /// A short tap is waiting for its second press (double-tap mode).
    fn awaiting_second_tap(&self) -> bool {
        self.hold.as_ref().is_some_and(|h| h.tap_deadline.is_some())
    }

    /// Resolve whichever deadline has passed by `now`.
    fn on_deadline(&mut self, now: Instant) -> Option<Effect> {
        if self.grace_deadline().is_some_and(|d| d <= now) {
            return self.on_grace_expired();
        }
        if self
            .hold
            .as_ref()
            .and_then(|h| h.tap_deadline)
            .is_some_and(|d| d <= now)
        {
            return self.on_tap_window_expired();
        }
        if self.session_deadline().is_some_and(|d| d <= now) {
            return self.on_session_mark(now);
        }
        None
    }

    /// The recording reached a session mark: warn once at `SESSION_WARNING`,
    /// and at `SESSION_LIMIT` stop it like the shortcut would, so it is
    /// processed and delivered. A thread that wakes past the limit without
    /// having warned cuts straight away.
    fn on_session_mark(&mut self, now: Instant) -> Option<Effect> {
        let Stage::Recording(binding_id) = &self.stage else {
            return None;
        };
        let binding_id = binding_id.clone();
        let hold = self.hold.as_mut()?;
        if now.saturating_duration_since(hold.pressed_at) >= SESSION_LIMIT {
            let hotkey_string = hold.hotkey_string.clone();
            info!("Recording for '{binding_id}' reached the session limit: stopping to process it");
            // A release still in its grace belongs to the session being cut.
            self.pending_release = None;
            return Some(self.begin_processing(binding_id, hotkey_string));
        }
        hold.limit_warned = true;
        debug!("Recording for '{binding_id}' is close to the session limit");
        Some(Effect::LimitWarning { binding_id })
    }

    /// The double-tap window ran out with no second press: the lone tap is
    /// discarded rather than transcribed.
    fn on_tap_window_expired(&mut self) -> Option<Effect> {
        self.hold.as_mut()?.tap_deadline.take()?;
        let Stage::Recording(binding_id) = &self.stage else {
            return None;
        };
        let binding_id = binding_id.clone();
        debug!("No second tap for '{binding_id}': discarding the short tap");
        Some(self.begin_discard(binding_id))
    }

    /// Whether the current session (recording, or remembered for the drain)
    /// outlives the key, so releases are ignored and the next press ends it.
    fn is_locked(&self) -> bool {
        self.hold.as_ref().is_some_and(|h| h.locked)
            || self.pending_press.as_ref().is_some_and(|p| p.locked)
    }

    fn on_input(&mut self, input: InputEvent, now: Instant) -> Option<Effect> {
        let pending_release_binding = self
            .pending_release
            .as_ref()
            .map(|pending| pending.binding_id.as_str());
        let held_binding = match &self.stage {
            Stage::Recording(id) => Some(id.as_str()),
            Stage::Processing => self.pending_press.as_ref().map(|p| p.binding_id.as_str()),
            Stage::Idle => None,
        };
        // While a short tap waits for its second press, releases mean nothing.
        let hold_to_talk = input.mode != ShortcutActivation::Toggle
            && !self.is_locked()
            && !self.awaiting_second_tap();

        match classify_ptt_event(
            pending_release_binding,
            input.is_pressed,
            hold_to_talk,
            &input.binding_id,
            held_binding,
        ) {
            PttAction::CancelRelease => {
                self.pending_release = None;
                return None;
            }
            PttAction::DeferRelease => {
                self.pending_release = Some(PendingRelease {
                    hold_threshold: input.effective_hold_threshold(),
                    mode: input.mode,
                    binding_id: input.binding_id,
                    hotkey_string: input.hotkey_string,
                    deadline: now + RELEASE_GRACE,
                    released_at: now,
                });
                return None;
            }
            PttAction::Passthrough => {}
        }

        // Debounce rapid-fire press events (key repeat / double-tap).
        // Releases in the hold modes are deferred above to absorb X11 auto-repeat.
        // External triggers are exempt: each one is a deliberate edge from the
        // user's own integration, and dropping it desyncs toggle parity.
        if input.is_pressed && !input.external {
            if self
                .last_press
                .is_some_and(|t| now.duration_since(t) < DEBOUNCE)
            {
                debug!("Debounced press for '{}'", input.binding_id);
                return None;
            }
            self.last_press = Some(now);
        }

        // A busy pipeline can't accept lifecycle changes now: classify the
        // input against any already-remembered press instead of dropping it
        // silently.
        if let Stage::Processing = self.stage {
            // Only one press can be remembered. Once a binding has claimed it,
            // inputs for a different binding are ignored — the same rule as a
            // different binding pressed while recording — rather than silently
            // replacing the remembered press and breaking its parity.
            if let Some(pending) = &self.pending_press {
                if pending.binding_id != input.binding_id {
                    debug!(
                        "Ignoring input for '{}': '{}' is already pending",
                        input.binding_id, pending.binding_id
                    );
                    return None;
                }
            }
            let remembered = self.pending_press.as_ref().map(|p| p.remembered());
            match classify_busy_input(input.is_pressed, input.mode, remembered) {
                BusyAction::Remember => {
                    debug!(
                        "Remembering press for '{}': pipeline busy",
                        input.binding_id
                    );
                    self.pending_press = Some(PendingPress {
                        // Toggle never ends on a release: locked from the start.
                        locked: input.mode == ShortcutActivation::Toggle,
                        binding_id: input.binding_id,
                        hotkey_string: input.hotkey_string,
                        pressed_at: now,
                    });
                }
                BusyAction::Forget => {
                    debug!("Forgetting remembered press for '{}'", input.binding_id);
                    self.pending_press = None;
                }
                BusyAction::Ignore => {
                    debug!("Ignoring input for '{}': pipeline busy", input.binding_id);
                }
            }
            return None;
        }

        if input.is_pressed {
            match &self.stage {
                Stage::Idle => {
                    // Toggle never ends on a release: locked from the start.
                    let locked = input.mode == ShortcutActivation::Toggle;
                    return Some(self.begin_recording(
                        input.binding_id,
                        input.hotkey_string,
                        now,
                        locked,
                    ));
                }
                Stage::Recording(id) if id == &input.binding_id => {
                    // A locked session ends on the next press. In toggle mode
                    // every press ends it, even if the recording began under a
                    // hold mode (the setting changed mid-recording) — otherwise
                    // nothing but Escape could stop it.
                    if self.is_locked() || input.mode == ShortcutActivation::Toggle {
                        return Some(self.begin_processing(input.binding_id, input.hotkey_string));
                    }
                    // Double-tap: the second press of a short tap locks the
                    // session, unless the window already ran out.
                    if let Some(deadline) = self.hold.as_ref().and_then(|h| h.tap_deadline) {
                        if now > deadline {
                            debug!(
                                "Second tap for '{}' after the window: discarding",
                                input.binding_id
                            );
                            return Some(self.begin_discard(input.binding_id));
                        }
                        if let Some(hold) = &mut self.hold {
                            hold.tap_deadline = None;
                            hold.locked = true;
                        }
                        debug!(
                            "Double tap for '{}': recording locked on until the next press",
                            input.binding_id
                        );
                        return None;
                    }
                    // The key is still held (its release will end this
                    // recording), so a repeated press means nothing.
                    debug!("Ignoring press for '{}': key is held", input.binding_id);
                }
                _ => debug!(
                    "Ignoring press for '{}': another binding is recording",
                    input.binding_id
                ),
            }
        } else if hold_to_talk
            && matches!(&self.stage, Stage::Recording(id) if id == &input.binding_id)
        {
            // A release that was not deferred (one is already pending for this
            // binding): resolve it immediately rather than dropping it.
            let threshold = input.effective_hold_threshold();
            return self.finish_hold(
                input.binding_id,
                input.hotkey_string,
                now,
                threshold,
                input.mode,
            );
        }
        None
    }

    /// The `RELEASE_GRACE` window elapsed with no cancelling press arriving:
    /// resolve the deferred release against whatever that binding's key was
    /// holding — the live recording, or a press remembered while busy.
    fn on_grace_expired(&mut self) -> Option<Effect> {
        let pending = self.pending_release.take()?;
        match &self.stage {
            Stage::Recording(id) if *id == pending.binding_id => self.finish_hold(
                pending.binding_id,
                pending.hotkey_string,
                pending.released_at,
                pending.hold_threshold,
                pending.mode,
            ),
            Stage::Processing => {
                self.finish_pending_hold(&pending);
                None
            }
            _ => None,
        }
    }

    /// A press remembered while the pipeline was busy has been released for
    /// real, still before the drain. A completed hold has nothing left to
    /// start; a tap queues a locked session so the drain starts it — the
    /// same hold-vs-tap rule as [`CoordinatorState::finish_hold`]. In
    /// double-tap mode a busy tap is forgotten like a push-to-talk release.
    fn finish_pending_hold(&mut self, release: &PendingRelease) {
        let Some(pending) = self
            .pending_press
            .as_mut()
            .filter(|p| p.binding_id == release.binding_id)
        else {
            return;
        };
        let held = release
            .released_at
            .saturating_duration_since(pending.pressed_at);
        if held < release.hold_threshold && release.mode != ShortcutActivation::PushToTalkDoubleTap
        {
            debug!(
                "Tap ({held:?}) for '{}' while busy: will start locked on when the pipeline drains",
                release.binding_id
            );
            pending.locked = true;
        } else {
            debug!(
                "Forgetting remembered press for '{}': released after a {held:?} hold while busy",
                release.binding_id
            );
            self.pending_press = None;
        }
    }

    /// The key that started the current recording has been released for real.
    /// A hold at least `threshold` long stops recording; anything shorter was a
    /// tap, which locks the session on until the next press — or, in
    /// double-tap mode, opens the window for the second tap.
    fn finish_hold(
        &mut self,
        binding_id: String,
        hotkey_string: String,
        released_at: Instant,
        threshold: Duration,
        mode: ShortcutActivation,
    ) -> Option<Effect> {
        let held = self
            .hold
            .as_ref()
            .map(|h| released_at.saturating_duration_since(h.pressed_at))
            // No hold bookkeeping means we cannot tell a tap from a hold;
            // stopping is the safe reading (it is what push-to-talk always did).
            .unwrap_or(Duration::MAX);
        if held >= threshold {
            return Some(self.begin_processing(binding_id, hotkey_string));
        }
        if mode == ShortcutActivation::PushToTalkDoubleTap {
            let deadline = self.hold.as_ref()?.pressed_at + DOUBLE_TAP_WINDOW;
            if released_at >= deadline {
                return Some(self.begin_discard(binding_id));
            }
            debug!("Tap ({held:?}) for '{binding_id}': waiting for a second tap");
            if let Some(hold) = &mut self.hold {
                hold.tap_deadline = Some(deadline);
            }
            return None;
        }
        if let Some(hold) = &mut self.hold {
            debug!("Tap ({held:?}) for '{binding_id}': recording locked on until the next press");
            hold.locked = true;
        }
        None
    }

    fn on_cancel(&mut self, recording_was_active: bool) {
        self.pending_release = None;
        // An explicit cancel abandons any remembered start too — the user
        // asked for silence, not a deferred recording.
        self.pending_press = None;
        // Don't reset during processing — wait for the pipeline to finish.
        if !matches!(self.stage, Stage::Processing)
            && (recording_was_active || matches!(self.stage, Stage::Recording(_)))
        {
            self.stage = Stage::Idle;
            self.hold = None;
        }
    }

    fn on_processing_finished(&mut self) -> Option<Effect> {
        self.stage = Stage::Idle;
        self.hold = None;
        let pending = self.pending_press.take()?;
        debug!(
            "Pipeline drained; starting remembered press for '{}'",
            pending.binding_id
        );
        Some(self.begin_recording(
            pending.binding_id,
            pending.hotkey_string,
            pending.pressed_at,
            pending.locked,
        ))
    }

    /// Reconcile the optimistic `Stage::Recording` after the executor reports
    /// whether recording actually began (microphone access can be denied).
    fn on_start_result(&mut self, binding_id: &str, started: bool) {
        if !started && matches!(&self.stage, Stage::Recording(id) if id == binding_id) {
            self.stage = Stage::Idle;
            self.hold = None;
        }
    }

    /// Optimistic transition to `Recording`; rolled back via
    /// [`CoordinatorState::on_start_result`] if the effect fails to start
    /// recording for real.
    fn begin_recording(
        &mut self,
        binding_id: String,
        hotkey_string: String,
        pressed_at: Instant,
        locked: bool,
    ) -> Effect {
        self.stage = Stage::Recording(binding_id.clone());
        self.hold = Some(Hold {
            pressed_at,
            locked,
            tap_deadline: None,
            hotkey_string: hotkey_string.clone(),
            limit_warned: false,
        });
        Effect::Start {
            binding_id,
            hotkey_string,
        }
    }

    /// Straight back to idle: the executor tears the recording down without
    /// processing it, and without reporting a cancel back to this machine.
    fn begin_discard(&mut self, binding_id: String) -> Effect {
        self.stage = Stage::Idle;
        self.hold = None;
        Effect::Discard { binding_id }
    }

    fn begin_processing(&mut self, binding_id: String, hotkey_string: String) -> Effect {
        self.stage = Stage::Processing;
        self.hold = None;
        Effect::Stop {
            binding_id,
            hotkey_string,
        }
    }
}

/// Serialises all transcription lifecycle events through a single thread
/// to eliminate race conditions between keyboard shortcuts, signals, and
/// the async transcribe-paste pipeline. The thread is a thin shell: it
/// transports commands to the pure [`CoordinatorState`] and executes the
/// returned [`Effect`]s.
pub struct TranscriptionCoordinator {
    tx: Sender<Command>,
    /// Mirror of [`CoordinatorState::is_busy`] after the last command, for
    /// the cancel key handler on other threads.
    busy: Arc<AtomicBool>,
}

/// Whether the cancel key must be registered or unregistered, given whether
/// it is armed now and whether a dictation is in flight. `None` leaves it.
fn cancel_key_change(armed: bool, busy: bool) -> Option<bool> {
    (armed != busy).then_some(busy)
}

/// Arm the cancel key while a dictation is in flight and disarm it when the
/// coordinator goes idle. Evaluated once per command, so a drain that starts
/// a remembered press goes from processing to recording without a disarm and
/// rearm whose asynchronous registrations could land out of order.
fn sync_cancel_key(app: &AppHandle, armed: &mut bool, busy_flag: &AtomicBool, busy: bool) {
    busy_flag.store(busy, Ordering::SeqCst);
    match cancel_key_change(*armed, busy) {
        Some(true) => crate::shortcut::register_cancel_shortcut(app),
        Some(false) => crate::shortcut::unregister_cancel_shortcut(app),
        None => return,
    }
    *armed = busy;
}

pub fn is_transcribe_binding(id: &str) -> bool {
    id == "transcribe" || id == "transcribe_with_post_process"
}

impl TranscriptionCoordinator {
    pub fn new(app: AppHandle) -> Self {
        let (tx, rx) = mpsc::channel();
        let busy = Arc::new(AtomicBool::new(false));
        let busy_flag = Arc::clone(&busy);

        thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut state = CoordinatorState::new();
                let mut cancel_key_armed = false;

                loop {
                    let cmd = if let Some(deadline) = state.wake_deadline() {
                        match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                            Ok(cmd) => cmd,
                            Err(mpsc::RecvTimeoutError::Timeout) => {
                                if let Some(effect) = state.on_deadline(Instant::now()) {
                                    run_effect(&app, &mut state, effect);
                                }
                                sync_cancel_key(
                                    &app,
                                    &mut cancel_key_armed,
                                    &busy_flag,
                                    state.is_busy(),
                                );
                                continue;
                            }
                            Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        }
                    } else {
                        match rx.recv() {
                            Ok(cmd) => cmd,
                            Err(_) => break,
                        }
                    };

                    match cmd {
                        Command::Input(input) => {
                            if let Some(effect) = state.on_input(input, Instant::now()) {
                                run_effect(&app, &mut state, effect);
                            }
                        }
                        Command::Cancel {
                            recording_was_active,
                        } => state.on_cancel(recording_was_active),
                        Command::ProcessingFinished => {
                            if let Some(effect) = state.on_processing_finished() {
                                run_effect(&app, &mut state, effect);
                            }
                        }
                    }
                    sync_cancel_key(&app, &mut cancel_key_armed, &busy_flag, state.is_busy());
                }
                debug!("Transcription coordinator exited");
            }));
            if let Err(e) = result {
                error!("Transcription coordinator panicked: {e:?}");
            }
        });

        Self { tx, busy }
    }

    /// Whether a dictation is in flight (recording or processing), as of the
    /// last command the coordinator handled. The cancel key acts only then.
    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::SeqCst)
    }

    /// Send a keyboard input event for a transcribe binding. `hold_threshold`
    /// only matters for [`ShortcutActivation::HoldOrToggle`].
    pub fn send_input(
        &self,
        binding_id: &str,
        hotkey_string: &str,
        is_pressed: bool,
        mode: ShortcutActivation,
        hold_threshold: Duration,
    ) {
        self.send(
            binding_id,
            hotkey_string,
            is_pressed,
            mode,
            hold_threshold,
            false,
        );
    }

    /// Send an external trigger (SIGUSR2, CLI flag). Always a toggle press,
    /// always exempt from debounce — see [`InputEvent::external`].
    pub fn send_external_input(&self, binding_id: &str, source: &str) {
        self.send(
            binding_id,
            source,
            true,
            ShortcutActivation::Toggle,
            Duration::ZERO,
            true,
        );
    }

    fn send(
        &self,
        binding_id: &str,
        hotkey_string: &str,
        is_pressed: bool,
        mode: ShortcutActivation,
        hold_threshold: Duration,
        external: bool,
    ) {
        crate::dictation_metrics::key_event(is_pressed);
        if self
            .tx
            .send(Command::Input(InputEvent {
                binding_id: binding_id.to_string(),
                hotkey_string: hotkey_string.to_string(),
                is_pressed,
                mode,
                hold_threshold,
                external,
            }))
            .is_err()
        {
            warn!("Transcription coordinator channel closed");
        }
    }

    pub fn notify_cancel(&self, recording_was_active: bool) {
        if self
            .tx
            .send(Command::Cancel {
                recording_was_active,
            })
            .is_err()
        {
            warn!("Transcription coordinator channel closed");
        }
    }

    pub fn notify_processing_finished(&self) {
        if self.tx.send(Command::ProcessingFinished).is_err() {
            warn!("Transcription coordinator channel closed");
        }
    }
}

fn run_effect(app: &AppHandle, state: &mut CoordinatorState, effect: Effect) {
    match effect {
        Effect::Start {
            binding_id,
            hotkey_string,
        } => {
            let started = start(app, &binding_id, &hotkey_string);
            state.on_start_result(&binding_id, started);
        }
        Effect::Stop {
            binding_id,
            hotkey_string,
        } => stop(app, &binding_id, &hotkey_string),
        Effect::Discard { binding_id } => {
            debug!("Discarding the recording for '{binding_id}'");
            // The machine is already idle: tear down without notifying it.
            crate::utils::abort_current_operation(app);
        }
        Effect::LimitWarning { binding_id } => {
            debug!("Warning that the recording for '{binding_id}' will be cut soon");
            crate::audio_feedback::play_limit_warning(app);
            crate::utils::emit_recording_limit_warning(app);
        }
    }
}

/// Execute a start effect; returns whether recording actually began, so the
/// state machine can roll back its optimistic transition on failure.
fn start(app: &AppHandle, binding_id: &str, hotkey_string: &str) -> bool {
    let Some(action) = ACTION_MAP.get(binding_id) else {
        warn!("No action in ACTION_MAP for '{binding_id}'");
        return false;
    };
    action.start(app, binding_id, hotkey_string);
    let recording = app
        .try_state::<Arc<AudioRecordingManager>>()
        .is_some_and(|a| a.is_recording());
    if !recording {
        debug!("Start for '{binding_id}' did not begin recording; staying idle");
    }
    recording
}

fn stop(app: &AppHandle, binding_id: &str, hotkey_string: &str) {
    let Some(action) = ACTION_MAP.get(binding_id) else {
        warn!("No action in ACTION_MAP for '{binding_id}'");
        return;
    };
    action.stop(app, binding_id, hotkey_string);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_to_talk_release_while_recording_defers_release() {
        assert_eq!(
            classify_ptt_event(None, false, true, "transcribe", Some("transcribe")),
            PttAction::DeferRelease
        );
    }

    #[test]
    fn push_to_talk_press_matching_pending_release_cancels_release() {
        assert_eq!(
            classify_ptt_event(
                Some("transcribe"),
                true,
                true,
                "transcribe",
                Some("transcribe")
            ),
            PttAction::CancelRelease
        );
    }

    #[test]
    fn toggle_mode_press_and_release_pass_through() {
        assert_eq!(
            classify_ptt_event(
                Some("transcribe"),
                true,
                false,
                "transcribe",
                Some("transcribe")
            ),
            PttAction::Passthrough
        );
        assert_eq!(
            classify_ptt_event(None, false, false, "transcribe", Some("transcribe")),
            PttAction::Passthrough
        );
    }

    #[test]
    fn press_for_different_binding_than_pending_release_passes_through() {
        assert_eq!(
            classify_ptt_event(
                Some("transcribe"),
                true,
                true,
                "transcribe_with_post_process",
                Some("transcribe")
            ),
            PttAction::Passthrough
        );
    }

    #[test]
    fn press_matching_pending_release_cancels_without_recording_state() {
        assert_eq!(
            classify_ptt_event(Some("transcribe"), true, true, "transcribe", None),
            PttAction::CancelRelease
        );
    }

    // ---------------------------------------------------------------------
    // Busy-pipeline input classification.
    //
    // Toggle-style triggers (SIGUSR2, CLI flags, pedals that signal on both
    // edges) flip state on every edge. Dropping a press that arrives while
    // the previous pipeline is still processing desyncs the parity: the next
    // edge then starts a recording no one will stop, leaving the overlay
    // waiting for input with the button long released.
    // ---------------------------------------------------------------------

    #[test]
    fn toggle_press_during_processing_remembers_start() {
        assert_eq!(
            classify_busy_input(true, ShortcutActivation::Toggle, None),
            BusyAction::Remember
        );
    }

    #[test]
    fn second_toggle_press_during_processing_forgets_press() {
        assert_eq!(
            classify_busy_input(true, ShortcutActivation::Toggle, Some(Remembered::Locked)),
            BusyAction::Forget
        );
    }

    #[test]
    fn toggle_release_during_processing_is_ignored() {
        assert_eq!(
            classify_busy_input(false, ShortcutActivation::Toggle, None),
            BusyAction::Ignore
        );
        assert_eq!(
            classify_busy_input(false, ShortcutActivation::Toggle, Some(Remembered::Locked)),
            BusyAction::Ignore
        );
    }

    #[test]
    fn hold_modes_classify_busy_inputs_by_pending_state() {
        let cases = [
            (true, None, BusyAction::Remember),
            (true, Some(Remembered::Held), BusyAction::Ignore),
            (true, Some(Remembered::Locked), BusyAction::Forget),
            (false, None, BusyAction::Ignore),
            (false, Some(Remembered::Held), BusyAction::Ignore),
            (false, Some(Remembered::Locked), BusyAction::Ignore),
        ];

        for mode in [
            ShortcutActivation::PushToTalk,
            ShortcutActivation::HoldOrToggle,
        ] {
            for (is_pressed, remembered, expected) in cases {
                assert_eq!(classify_busy_input(is_pressed, mode, remembered), expected);
            }
        }
    }

    /// Toggle parity across a busy window: an odd number of presses remembers
    /// one start, each further press flips the remembered press off/on again.
    #[test]
    fn toggle_presses_alternate_remember_and_forget_while_busy() {
        let mut remembered = None;
        for expected in [
            BusyAction::Remember,
            BusyAction::Forget,
            BusyAction::Remember,
        ] {
            let action = classify_busy_input(true, ShortcutActivation::Toggle, remembered);
            assert_eq!(action, expected);
            remembered = (action == BusyAction::Remember).then_some(Remembered::Locked);
        }
        assert!(remembered.is_some());
    }

    // ---------------------------------------------------------------------
    // Sequence-level regression coverage for issue #1539.
    //
    // Under X11 key auto-repeat, holding a push-to-talk key does not emit one
    // long press. It emits the initial press followed by a stream of
    // synthesized release/press pairs, then a single genuine release on key-up.
    // Before the fix, every synthesized release passed straight through and
    // stopped recording, so holding the key "rapidly toggled" recording on and
    // off. The fix defers each release for a short grace window and cancels it
    // when the matching auto-repeat press arrives.
    //
    // The unit tests above assert the classifiers in isolation. The harness
    // below drives the real `CoordinatorState` through whole event sequences
    // — the same `on_input` / `on_grace_expired` handlers the coordinator
    // thread runs — so a burst can be exercised deterministically without a
    // Tauri AppHandle or real timers, and the tests can never drift from the
    // production transitions.
    // ---------------------------------------------------------------------

    const BINDING: &str = "transcribe";

    #[derive(Clone, Copy)]
    enum Ev {
        /// A key-down event (real initial press or a synthesized auto-repeat press).
        Press,
        /// A key-up event (synthesized auto-repeat release or the genuine key-up).
        Release,
        /// The `RELEASE_GRACE` window elapsed with no cancelling press arriving.
        Grace,
    }

    struct DriveResult {
        starts: u32,
        stops: u32,
        stage: Stage,
    }

    fn ptt_input(is_pressed: bool) -> InputEvent {
        InputEvent {
            binding_id: BINDING.to_string(),
            hotkey_string: BINDING.to_string(),
            is_pressed,
            mode: ShortcutActivation::PushToTalk,
            hold_threshold: Duration::ZERO,
            external: false,
        }
    }

    /// Feeds an event sequence to a real [`CoordinatorState`] the way the
    /// coordinator thread would; effects are counted instead of executed.
    fn drive(events: &[Ev]) -> DriveResult {
        let mut state = CoordinatorState::new();
        let mut clock = Instant::now();
        let mut starts = 0u32;
        let mut stops = 0u32;

        for ev in events {
            // Auto-repeat events arrive a few ms apart, well inside DEBOUNCE.
            clock += Duration::from_millis(5);

            let effect = match ev {
                Ev::Grace => state.on_grace_expired(),
                Ev::Press | Ev::Release => {
                    state.on_input(ptt_input(matches!(ev, Ev::Press)), clock)
                }
            };
            match effect {
                Some(Effect::Start { .. }) => starts += 1,
                Some(Effect::Stop { .. }) => stops += 1,
                Some(Effect::Discard { .. }) => panic!("push-to-talk never discards"),
                Some(Effect::LimitWarning { .. }) => {
                    panic!("these sequences never reach the session limit")
                }
                None => {}
            }
        }

        DriveResult {
            starts,
            stops,
            stage: state.stage,
        }
    }

    /// Initial press plus several synthesized release/press pairs, as X11 emits
    /// while a push-to-talk key is held down.
    fn autorepeat_burst() -> Vec<Ev> {
        let mut events = vec![Ev::Press];
        for _ in 0..6 {
            events.push(Ev::Release);
            events.push(Ev::Press);
        }
        events
    }

    /// Regression for #1539: a burst of X11 auto-repeat release/press pairs must
    /// not stop recording. Before the fix the first synthesized release stopped
    /// recording immediately (stops == 1, stage left Recording), which produced
    /// the rapid on/off toggling. With the fix the releases are coalesced and
    /// recording stays continuously active for the whole burst.
    #[test]
    fn x11_autorepeat_burst_does_not_toggle_recording() {
        let result = drive(&autorepeat_burst());
        assert_eq!(result.starts, 1, "recording should start exactly once");
        assert_eq!(
            result.stops, 0,
            "synthesized auto-repeat releases must not stop recording mid-burst"
        );
        assert_eq!(
            result.stage,
            Stage::Recording(BINDING.to_string()),
            "recording must remain active across the entire auto-repeat burst"
        );
    }

    /// Complements the burst test: once the key is genuinely released and the
    /// grace window elapses with no re-press, recording stops exactly once. This
    /// proves the debounce only coalesces synthesized releases and does not wedge
    /// the coordinator or swallow the real key-up.
    #[test]
    fn genuine_release_after_grace_stops_recording_once() {
        let mut events = autorepeat_burst();
        events.push(Ev::Release); // genuine key-up
        events.push(Ev::Grace); // grace window elapses, no cancelling press
        let result = drive(&events);
        assert_eq!(result.starts, 1, "recording should start exactly once");
        assert_eq!(
            result.stops, 1,
            "a genuine release should stop recording exactly once"
        );
        assert_eq!(result.stage, Stage::Processing);
    }

    // ---------------------------------------------------------------------
    // Sequence-level coverage of the busy-pipeline and cancel paths, driven
    // through the real machine.
    // ---------------------------------------------------------------------

    /// PTT press while the pipeline is busy is remembered and starts recording
    /// once the pipeline drains.
    #[test]
    fn press_during_processing_starts_after_drain() {
        let mut state = CoordinatorState::new();
        let now = Instant::now();

        let effect = state.on_input(ptt_input(true), now);
        assert!(matches!(effect, Some(Effect::Start { .. })));

        let effect = state.on_input(ptt_input(false), now + Duration::from_millis(100));
        assert!(effect.is_none(), "release should be deferred, not fired");

        let effect = state.on_grace_expired();
        assert!(matches!(effect, Some(Effect::Stop { .. })));

        let effect = state.on_input(ptt_input(true), now + Duration::from_millis(200));
        assert!(effect.is_none(), "busy pipeline must remember, not start");

        let effect = state.on_processing_finished();
        assert!(
            matches!(effect, Some(Effect::Start { .. })),
            "remembered press should start once the pipeline drains"
        );
    }

    /// Two toggle presses inside one busy window net to no-op: nothing starts
    /// when the pipeline drains (toggle parity).
    #[test]
    fn toggle_presses_during_processing_net_noop_after_drain() {
        let mut state = CoordinatorState::new();
        let now = Instant::now();

        let effect = state.on_input(ptt_input(true), now);
        assert!(matches!(effect, Some(Effect::Start { .. })));
        let effect = state.on_input(ptt_input(false), now + Duration::from_millis(100));
        assert!(effect.is_none());
        let effect = state.on_grace_expired();
        assert!(matches!(effect, Some(Effect::Stop { .. })));

        let toggle = |state: &mut CoordinatorState, at: Instant| {
            state.on_input(
                InputEvent {
                    binding_id: BINDING.to_string(),
                    hotkey_string: BINDING.to_string(),
                    is_pressed: true,
                    mode: ShortcutActivation::Toggle,
                    hold_threshold: Duration::ZERO,
                    external: true,
                },
                at,
            )
        };

        let effect = toggle(&mut state, now + Duration::from_millis(200));
        assert!(effect.is_none());
        let effect = toggle(&mut state, now + Duration::from_millis(300));
        assert!(effect.is_none());

        let effect = state.on_processing_finished();
        assert!(
            effect.is_none(),
            "even number of busy toggle presses must not start recording"
        );
        assert_eq!(state.stage, Stage::Idle);
    }

    /// Cancel while processing abandons a remembered press: the pipeline drains
    /// to idle and nothing starts.
    #[test]
    fn cancel_during_processing_drops_remembered_press() {
        let mut state = CoordinatorState::new();
        let now = Instant::now();

        let effect = state.on_input(ptt_input(true), now);
        assert!(matches!(effect, Some(Effect::Start { .. })));
        let effect = state.on_input(ptt_input(false), now + Duration::from_millis(100));
        assert!(effect.is_none());
        let effect = state.on_grace_expired();
        assert!(matches!(effect, Some(Effect::Stop { .. })));

        let effect = state.on_input(ptt_input(true), now + Duration::from_millis(200));
        assert!(effect.is_none());

        state.on_cancel(false);
        assert_eq!(
            state.stage,
            Stage::Processing,
            "cancel must not reset mid-processing — the pipeline still finishes"
        );

        let effect = state.on_processing_finished();
        assert!(
            effect.is_none(),
            "cancelled session must not spawn a deferred recording"
        );
        assert_eq!(state.stage, Stage::Idle);
    }

    fn toggle_input(external: bool) -> InputEvent {
        toggle_input_for(BINDING, external)
    }

    fn toggle_input_for(binding_id: &str, external: bool) -> InputEvent {
        InputEvent {
            binding_id: binding_id.to_string(),
            hotkey_string: binding_id.to_string(),
            is_pressed: true,
            mode: ShortcutActivation::Toggle,
            hold_threshold: Duration::ZERO,
            external,
        }
    }

    /// Start and stop one toggle recording so the machine sits in `Processing`.
    fn drive_into_processing(state: &mut CoordinatorState, now: Instant) {
        let effect = state.on_input(toggle_input(true), now);
        assert!(matches!(effect, Some(Effect::Start { .. })));
        let effect = state.on_input(toggle_input(true), now + Duration::from_millis(100));
        assert!(matches!(effect, Some(Effect::Stop { .. })));
        assert_eq!(state.stage, Stage::Processing);
    }

    const OTHER_BINDING: &str = "transcribe_with_post_process";

    /// Only one press can be pending. Once a binding has claimed it, a toggle
    /// for a different binding is ignored (as it is while recording) instead of
    /// replacing the remembered press, so the pending binding's parity holds:
    /// two transcribe toggles still net to no-op.
    #[test]
    fn different_binding_does_not_replace_pending_press() {
        let mut state = CoordinatorState::new();
        let now = Instant::now();
        drive_into_processing(&mut state, now);

        let at = |ms| now + Duration::from_millis(ms);
        assert!(state.on_input(toggle_input(true), at(200)).is_none());
        assert!(state
            .on_input(toggle_input_for(OTHER_BINDING, true), at(300))
            .is_none());
        assert!(state.on_input(toggle_input(true), at(400)).is_none());

        let effect = state.on_processing_finished();
        assert!(
            effect.is_none(),
            "two transcribe toggles net to no-op; the ignored post-process toggle must not start"
        );
        assert_eq!(state.stage, Stage::Idle);
    }

    /// The binding that claimed the pending press is the one that starts on
    /// drain, regardless of other bindings toggled in between.
    #[test]
    fn drain_starts_the_pending_binding_not_a_later_one() {
        let mut state = CoordinatorState::new();
        let now = Instant::now();
        drive_into_processing(&mut state, now);

        let at = |ms| now + Duration::from_millis(ms);
        assert!(state.on_input(toggle_input(true), at(200)).is_none());
        assert!(state
            .on_input(toggle_input_for(OTHER_BINDING, true), at(300))
            .is_none());

        match state.on_processing_finished() {
            Some(Effect::Start { binding_id, .. }) => assert_eq!(binding_id, BINDING),
            other => panic!("expected Start for '{BINDING}', got {other:?}"),
        }
    }

    /// External triggers fire on every edge by design (e.g. SIGUSR2 sent on
    /// both key press and release). Two edges inside the debounce window must
    /// both be honoured, or the parity desyncs and recording wedges on.
    #[test]
    fn external_edges_inside_debounce_window_are_not_dropped() {
        let mut state = CoordinatorState::new();
        let now = Instant::now();

        let effect = state.on_input(toggle_input(true), now);
        assert!(matches!(effect, Some(Effect::Start { .. })));

        let effect = state.on_input(toggle_input(true), now + Duration::from_millis(5));
        assert!(
            matches!(effect, Some(Effect::Stop { .. })),
            "second external edge inside DEBOUNCE must stop the recording"
        );
        assert_eq!(state.stage, Stage::Processing);
    }

    /// Physical keyboard presses keep the debounce: a repeat inside the window
    /// is still dropped and recording stays active.
    #[test]
    fn keyboard_press_inside_debounce_window_is_still_dropped() {
        let mut state = CoordinatorState::new();
        let now = Instant::now();

        let effect = state.on_input(toggle_input(false), now);
        assert!(matches!(effect, Some(Effect::Start { .. })));

        let effect = state.on_input(toggle_input(false), now + Duration::from_millis(5));
        assert!(
            effect.is_none(),
            "keyboard repeat inside DEBOUNCE must be debounced"
        );
        assert_eq!(state.stage, Stage::Recording(BINDING.to_string()));
    }

    /// If the start effect fails to begin recording (e.g. microphone access
    /// denied), the optimistic transition rolls back to idle.
    #[test]
    fn failed_start_rolls_back_to_idle() {
        let mut state = CoordinatorState::new();

        let effect = state.on_input(ptt_input(true), Instant::now());
        assert!(matches!(effect, Some(Effect::Start { .. })));

        state.on_start_result(BINDING, false);
        assert_eq!(state.stage, Stage::Idle);
    }

    // ---------------------------------------------------------------------
    // Hold-or-toggle (the combined mode from #147) and the two legacy modes,
    // driven through the real machine on a synthetic clock. Recording starts
    // on key-down in every mode; the tests pin how each mode ends it.
    // ---------------------------------------------------------------------

    const HOLD_THRESHOLD: Duration = Duration::from_millis(300);

    fn input(mode: ShortcutActivation, is_pressed: bool) -> InputEvent {
        InputEvent {
            binding_id: BINDING.to_string(),
            hotkey_string: BINDING.to_string(),
            is_pressed,
            mode,
            hold_threshold: HOLD_THRESHOLD,
            external: false,
        }
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// Hold-or-toggle: a key held past the threshold is push-to-talk — the
    /// (deferred) release stops recording.
    #[test]
    fn hold_or_toggle_long_hold_stops_on_release() {
        let mode = ShortcutActivation::HoldOrToggle;
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();

        assert!(matches!(
            state.on_input(input(mode, true), t0),
            Some(Effect::Start { .. })
        ));
        assert!(state.on_input(input(mode, false), t0 + ms(800)).is_none());
        assert!(
            matches!(state.on_grace_expired(), Some(Effect::Stop { .. })),
            "an 800ms hold must stop when its release grace elapses"
        );
        assert_eq!(state.stage, Stage::Processing);
    }

    /// Hold-or-toggle: a tap keeps recording (locked on); the next press stops.
    #[test]
    fn hold_or_toggle_tap_locks_recording_until_next_press() {
        let mode = ShortcutActivation::HoldOrToggle;
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();

        assert!(matches!(
            state.on_input(input(mode, true), t0),
            Some(Effect::Start { .. })
        ));
        assert!(state.on_input(input(mode, false), t0 + ms(120)).is_none());
        assert!(
            state.on_grace_expired().is_none(),
            "a 120ms tap must not stop recording"
        );
        assert_eq!(state.stage, Stage::Recording(BINDING.to_string()));
        assert!(state.is_locked());

        // Seconds later the user presses again to finish.
        assert!(matches!(
            state.on_input(input(mode, true), t0 + ms(5000)),
            Some(Effect::Stop { .. })
        ));
        assert_eq!(state.stage, Stage::Processing);
        // The release of that stopping press lands in the busy window and is
        // ignored, so nothing is remembered for the drain.
        assert!(state.on_input(input(mode, false), t0 + ms(5080)).is_none());
        assert!(state.on_processing_finished().is_none());
        assert_eq!(state.stage, Stage::Idle);
    }

    /// Hold-or-toggle: a locked session ignores stray releases — only a press
    /// ends it.
    #[test]
    fn hold_or_toggle_locked_session_ignores_release() {
        let mode = ShortcutActivation::HoldOrToggle;
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();

        state.on_input(input(mode, true), t0);
        state.on_input(input(mode, false), t0 + ms(100));
        assert!(state.on_grace_expired().is_none());
        assert!(state.is_locked());

        assert!(state.on_input(input(mode, false), t0 + ms(900)).is_none());
        assert!(
            state.grace_deadline().is_none(),
            "no release may be deferred once locked"
        );
        assert_eq!(state.stage, Stage::Recording(BINDING.to_string()));
    }

    /// Hold-or-toggle: while the key is genuinely held, extra presses do not
    /// stop the recording (that is the release's job).
    #[test]
    fn hold_or_toggle_press_while_held_is_ignored() {
        let mode = ShortcutActivation::HoldOrToggle;
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();

        state.on_input(input(mode, true), t0);
        assert!(state.on_input(input(mode, true), t0 + ms(400)).is_none());
        assert_eq!(state.stage, Stage::Recording(BINDING.to_string()));
        assert!(!state.is_locked());
    }

    /// Hold-or-toggle under X11 auto-repeat: the synthesized release/press
    /// pairs must not be misread as taps. The hold is measured from the
    /// original key-down to the genuine key-up.
    #[test]
    fn hold_or_toggle_autorepeat_burst_is_one_long_hold() {
        let mode = ShortcutActivation::HoldOrToggle;
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        let mut clock = t0;

        assert!(matches!(
            state.on_input(input(mode, true), clock),
            Some(Effect::Start { .. })
        ));
        // ~600ms of auto-repeat pairs a few ms apart.
        for _ in 0..60 {
            clock += ms(5);
            assert!(state.on_input(input(mode, false), clock).is_none());
            clock += ms(5);
            assert!(state.on_input(input(mode, true), clock).is_none());
            assert!(
                state.grace_deadline().is_none(),
                "auto-repeat press must cancel the deferred release"
            );
        }
        assert!(!state.is_locked(), "no tap may be classified mid-burst");

        clock += ms(5);
        assert!(state.on_input(input(mode, false), clock).is_none());
        assert!(
            matches!(state.on_grace_expired(), Some(Effect::Stop { .. })),
            "the genuine release after a ~600ms hold must stop recording"
        );
    }

    /// Hold-or-toggle: a press remembered during the busy window is measured
    /// from the real key-down, so a hold that straddles the drain still counts
    /// as a hold when it is released shortly after recording actually starts.
    #[test]
    fn hold_or_toggle_remembered_press_measures_hold_from_real_key_down() {
        let mode = ShortcutActivation::HoldOrToggle;
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();

        // Previous session: hold, release, stop → Processing.
        state.on_input(input(mode, true), t0);
        state.on_input(input(mode, false), t0 + ms(800));
        assert!(matches!(
            state.on_grace_expired(),
            Some(Effect::Stop { .. })
        ));

        // Pressed again while busy; still held when the pipeline drains 700ms later.
        assert!(state.on_input(input(mode, true), t0 + ms(1000)).is_none());
        assert!(matches!(
            state.on_processing_finished(),
            Some(Effect::Start { .. })
        ));
        // Released 100ms after recording began — but 800ms after key-down.
        assert!(state.on_input(input(mode, false), t0 + ms(1800)).is_none());
        assert!(
            matches!(state.on_grace_expired(), Some(Effect::Stop { .. })),
            "held 800ms overall: must stop, not lock"
        );
    }

    /// Toggle: releases never stop, the next press does. (Toggle is the
    /// combined machine with the session locked from the start.)
    #[test]
    fn toggle_mode_ignores_release_and_stops_on_next_press() {
        let mode = ShortcutActivation::Toggle;
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();

        assert!(matches!(
            state.on_input(input(mode, true), t0),
            Some(Effect::Start { .. })
        ));
        assert!(state.is_locked());
        assert!(state.on_input(input(mode, false), t0 + ms(100)).is_none());
        assert!(
            state.grace_deadline().is_none(),
            "toggle never defers releases"
        );
        assert!(state.on_input(input(mode, false), t0 + ms(3000)).is_none());
        assert!(matches!(
            state.on_input(input(mode, true), t0 + ms(4000)),
            Some(Effect::Stop { .. })
        ));
    }

    /// Push-to-talk: even a very short press stops on release — there is no
    /// tap-to-lock in this mode (hold threshold of zero).
    #[test]
    fn push_to_talk_short_press_still_stops_on_release() {
        let mode = ShortcutActivation::PushToTalk;
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();

        assert!(matches!(
            state.on_input(input(mode, true), t0),
            Some(Effect::Start { .. })
        ));
        assert!(state.on_input(input(mode, false), t0 + ms(40)).is_none());
        assert!(matches!(
            state.on_grace_expired(),
            Some(Effect::Stop { .. })
        ));
    }

    /// Cancel (Escape) during a locked hold-or-toggle session resets cleanly so
    /// the next press starts a fresh recording rather than stopping a dead one.
    #[test]
    fn hold_or_toggle_cancel_clears_locked_session() {
        let mode = ShortcutActivation::HoldOrToggle;
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();

        state.on_input(input(mode, true), t0);
        state.on_input(input(mode, false), t0 + ms(100));
        assert!(state.on_grace_expired().is_none());
        assert!(state.is_locked());

        state.on_cancel(true);
        assert_eq!(state.stage, Stage::Idle);
        assert!(!state.is_locked());
        assert!(matches!(
            state.on_input(input(mode, true), t0 + ms(2000)),
            Some(Effect::Start { .. })
        ));
    }

    /// Switching to toggle while an unlocked hold recording is running must not
    /// strand it: in toggle mode a press always stops.
    #[test]
    fn toggle_press_stops_recording_started_as_hold() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();

        state.on_input(input(ShortcutActivation::HoldOrToggle, true), t0);
        assert!(!state.is_locked());
        assert!(matches!(
            state.on_input(input(ShortcutActivation::Toggle, true), t0 + ms(2000)),
            Some(Effect::Stop { .. })
        ));
    }

    // Hold-vs-tap classification while the previous transcription is busy.
    fn hold_or_toggle_into_processing(state: &mut CoordinatorState, t0: Instant) {
        let mode = ShortcutActivation::HoldOrToggle;
        assert!(matches!(
            state.on_input(input(mode, true), t0),
            Some(Effect::Start { .. })
        ));
        assert!(state.on_input(input(mode, false), t0 + ms(800)).is_none());
        assert!(matches!(
            state.on_grace_expired(),
            Some(Effect::Stop { .. })
        ));
        assert_eq!(state.stage, Stage::Processing);
    }

    #[test]
    fn hold_or_toggle_tap_during_processing_queues_locked_start() {
        let mode = ShortcutActivation::HoldOrToggle;
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        hold_or_toggle_into_processing(&mut state, t0);

        assert!(state.on_input(input(mode, true), t0 + ms(1000)).is_none());
        assert!(state.on_input(input(mode, false), t0 + ms(1100)).is_none());
        assert!(state.on_grace_expired().is_none());
        assert!(state.is_locked(), "a busy tap should queue a locked start");
        assert!(matches!(
            state.on_processing_finished(),
            Some(Effect::Start { .. })
        ));
        assert!(state.is_locked());
    }

    #[test]
    fn hold_or_toggle_completed_hold_during_processing_nets_noop() {
        let mode = ShortcutActivation::HoldOrToggle;
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        hold_or_toggle_into_processing(&mut state, t0);

        assert!(state.on_input(input(mode, true), t0 + ms(1000)).is_none());
        assert!(state.on_input(input(mode, false), t0 + ms(1600)).is_none());
        assert!(state.on_grace_expired().is_none());
        assert!(!state.is_locked());

        assert!(
            state.on_processing_finished().is_none(),
            "a 600ms hold that ended before the drain has nothing left to start"
        );
        assert_eq!(state.stage, Stage::Idle);
    }

    #[test]
    fn hold_or_toggle_two_taps_during_processing_net_noop() {
        let mode = ShortcutActivation::HoldOrToggle;
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        hold_or_toggle_into_processing(&mut state, t0);

        assert!(state.on_input(input(mode, true), t0 + ms(1000)).is_none());
        assert!(state.on_input(input(mode, false), t0 + ms(1100)).is_none());
        assert!(state.on_grace_expired().is_none());
        assert!(state.is_locked());

        assert!(state.on_input(input(mode, true), t0 + ms(1500)).is_none());
        assert!(
            !state.is_locked(),
            "the second tap's press forgets the queued tap"
        );
        assert!(state.on_input(input(mode, false), t0 + ms(1600)).is_none());
        assert!(state.grace_deadline().is_none());

        assert!(state.on_processing_finished().is_none());
        assert_eq!(state.stage, Stage::Idle);
    }

    #[test]
    fn ptt_tap_inside_busy_window_nets_noop() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        hold_or_toggle_into_processing(&mut state, t0);

        assert!(state.on_input(ptt_input(true), t0 + ms(1000)).is_none());
        assert!(state.on_input(ptt_input(false), t0 + ms(1040)).is_none());
        assert!(state.on_grace_expired().is_none());
        assert!(state.on_processing_finished().is_none());
    }

    /// The pipeline drains inside the 50ms grace of a busy tap: recording
    /// starts first (unlocked, from the real key-down), and the grace then
    /// resolves against the live recording, locking it as the tap it was.
    #[test]
    fn hold_or_toggle_drain_inside_busy_release_grace_still_classifies_tap() {
        let mode = ShortcutActivation::HoldOrToggle;
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        hold_or_toggle_into_processing(&mut state, t0);

        assert!(state.on_input(input(mode, true), t0 + ms(1000)).is_none());
        assert!(state.on_input(input(mode, false), t0 + ms(1100)).is_none());
        assert!(matches!(
            state.on_processing_finished(),
            Some(Effect::Start { .. })
        ));
        assert!(!state.is_locked());

        assert!(state.on_grace_expired().is_none());
        assert!(state.is_locked(), "the deferred 100ms release is a tap");
    }

    /// X11 auto-repeat while busy, key still held at the drain: recording
    /// starts measured from the first press, not from the last synthesized
    /// press before the drain. Released 400ms after the real key-down but
    /// only ~100ms after the drain — a hold, so it must stop rather than lock.
    #[test]
    fn hold_or_toggle_autorepeat_burst_straddling_drain_measures_from_first_press() {
        let mode = ShortcutActivation::HoldOrToggle;
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        hold_or_toggle_into_processing(&mut state, t0);

        let mut clock = t0 + ms(1000);
        assert!(state.on_input(input(mode, true), clock).is_none());
        for _ in 0..30 {
            clock += ms(5);
            assert!(state.on_input(input(mode, false), clock).is_none());
            clock += ms(5);
            assert!(state.on_input(input(mode, true), clock).is_none());
            assert!(state.grace_deadline().is_none());
        }

        // Drain at ~t0 + 1300ms with the key still down.
        assert!(matches!(
            state.on_processing_finished(),
            Some(Effect::Start { .. })
        ));
        assert!(!state.is_locked());

        for _ in 0..10 {
            clock += ms(5);
            assert!(state.on_input(input(mode, false), clock).is_none());
            clock += ms(5);
            assert!(state.on_input(input(mode, true), clock).is_none());
        }
        assert_eq!(clock, t0 + ms(1400));
        assert!(state.on_input(input(mode, false), clock).is_none());
        assert!(
            matches!(state.on_grace_expired(), Some(Effect::Stop { .. })),
            "held 400ms since the real key-down: must stop, not lock"
        );
        assert_eq!(state.stage, Stage::Processing);
    }

    // ---------------------------------------------------------------------
    // Push-to-talk with double-tap (the phase 1 gesture): hold to talk, two
    // taps within DOUBLE_TAP_WINDOW lock the session hands-free, a lone short
    // tap is discarded. Driven on a synthetic clock through the real machine.
    // ---------------------------------------------------------------------

    fn dt(is_pressed: bool) -> InputEvent {
        input(ShortcutActivation::PushToTalkDoubleTap, is_pressed)
    }

    /// Press at `t0`, release after `held`, then let the release grace elapse.
    /// Returns what the grace expiry emitted.
    fn dt_tap(state: &mut CoordinatorState, t0: Instant, held: Duration) -> Option<Effect> {
        assert!(matches!(
            state.on_input(dt(true), t0),
            Some(Effect::Start { .. })
        ));
        assert!(state.on_input(dt(false), t0 + held).is_none());
        state.on_deadline(t0 + held + RELEASE_GRACE)
    }

    /// A 100ms tap followed by a second press at `second_ms`: a locked session.
    fn dt_locked(state: &mut CoordinatorState, t0: Instant, second_ms: u64) {
        assert!(dt_tap(state, t0, ms(100)).is_none());
        assert!(state.on_input(dt(true), t0 + ms(second_ms)).is_none());
        assert!(state.is_locked());
    }

    #[test]
    fn double_tap_press_starts_recording_on_key_down() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();

        match state.on_input(dt(true), t0) {
            Some(Effect::Start { binding_id, .. }) => assert_eq!(binding_id, BINDING),
            other => panic!("expected Start on key-down, got {other:?}"),
        }
        assert_eq!(state.stage, Stage::Recording(BINDING.to_string()));
    }

    #[test]
    fn double_tap_long_hold_stops_after_release_grace() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();

        assert!(matches!(
            state.on_input(dt(true), t0),
            Some(Effect::Start { .. })
        ));
        assert!(
            state.on_input(dt(false), t0 + ms(300)).is_none(),
            "the release is deferred by the grace, not fired"
        );
        assert!(
            matches!(
                state.on_deadline(t0 + ms(300) + RELEASE_GRACE),
                Some(Effect::Stop { .. })
            ),
            "a 300ms hold is push-to-talk: it stops when the grace elapses"
        );
        assert_eq!(state.stage, Stage::Processing);
    }

    #[test]
    fn double_tap_short_tap_keeps_recording_and_arms_window() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();

        assert!(
            dt_tap(&mut state, t0, ms(299)).is_none(),
            "a 299ms tap must not stop or discard yet"
        );
        assert_eq!(state.stage, Stage::Recording(BINDING.to_string()));
        assert!(!state.is_locked(), "one tap does not lock");
        assert_eq!(state.next_deadline(), Some(t0 + ms(500)));
    }

    #[test]
    fn double_tap_second_press_inside_window_locks() {
        for second_ms in [499, 500] {
            let mut state = CoordinatorState::new();
            let t0 = Instant::now();

            assert!(dt_tap(&mut state, t0, ms(100)).is_none());
            assert!(
                state.on_input(dt(true), t0 + ms(second_ms)).is_none(),
                "second press at {second_ms}ms must emit nothing"
            );
            assert!(state.is_locked(), "second press at {second_ms}ms must lock");
            assert_eq!(
                state.next_deadline(),
                None,
                "locking clears the double-tap window"
            );
            assert_eq!(state.stage, Stage::Recording(BINDING.to_string()));
        }
    }

    #[test]
    fn double_tap_locked_session_ignores_release() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        dt_locked(&mut state, t0, 300);

        assert!(state.on_input(dt(false), t0 + ms(380)).is_none());
        assert_eq!(state.grace_deadline(), None, "no release may be deferred");
        assert!(state.is_locked());
        assert_eq!(state.stage, Stage::Recording(BINDING.to_string()));
    }

    #[test]
    fn double_tap_press_on_locked_session_stops() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        dt_locked(&mut state, t0, 300);
        assert!(state.on_input(dt(false), t0 + ms(380)).is_none());

        assert!(matches!(
            state.on_input(dt(true), t0 + ms(5000)),
            Some(Effect::Stop { .. })
        ));
        assert_eq!(state.stage, Stage::Processing);
        assert!(state.on_input(dt(false), t0 + ms(5080)).is_none());
        assert!(state.on_deadline(t0 + ms(5200)).is_none());
        assert!(
            state.on_processing_finished().is_none(),
            "the stopping press leaves nothing for the drain"
        );
        assert_eq!(state.stage, Stage::Idle);
    }

    #[test]
    fn double_tap_window_expiry_discards() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        let mut effects = Vec::new();

        effects.extend(state.on_input(dt(true), t0));
        effects.extend(state.on_input(dt(false), t0 + ms(100)));
        effects.extend(state.on_deadline(t0 + ms(100) + RELEASE_GRACE));
        effects.extend(state.on_deadline(t0 + ms(499)));
        assert_eq!(effects.len(), 1, "only the Start so far: {effects:?}");

        match state.on_deadline(t0 + ms(500)) {
            Some(Effect::Discard { binding_id }) => assert_eq!(binding_id, BINDING),
            other => panic!("expected Discard when the window ends, got {other:?}"),
        }
        assert!(
            !effects.iter().any(|e| matches!(e, Effect::Stop { .. })),
            "a lone tap must never be processed"
        );
        assert_eq!(state.stage, Stage::Idle);
        assert_eq!(state.next_deadline(), None);
    }

    #[test]
    fn double_tap_late_second_press_discards() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        assert!(dt_tap(&mut state, t0, ms(100)).is_none());

        // The window deadline has not been processed yet when this press lands.
        match state.on_input(dt(true), t0 + ms(501)) {
            Some(Effect::Discard { binding_id }) => assert_eq!(binding_id, BINDING),
            other => panic!("a press at 501ms must discard, got {other:?}"),
        }
        assert!(!state.is_locked());
        assert_eq!(state.stage, Stage::Idle);

        // Its release lands on an idle machine; the next press starts fresh.
        assert!(state.on_input(dt(false), t0 + ms(560)).is_none());
        assert!(matches!(
            state.on_input(dt(true), t0 + ms(2000)),
            Some(Effect::Start { .. })
        ));
    }

    #[test]
    fn double_tap_release_inside_window_is_ignored() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        assert!(dt_tap(&mut state, t0, ms(100)).is_none());

        assert!(state.on_input(dt(false), t0 + ms(200)).is_none());
        assert_eq!(state.grace_deadline(), None, "no release may be deferred");
        assert_eq!(state.next_deadline(), Some(t0 + ms(500)));
        assert_eq!(state.stage, Stage::Recording(BINDING.to_string()));
    }

    #[test]
    fn double_tap_tap_during_processing_nets_noop() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        assert!(matches!(
            dt_tap(&mut state, t0, ms(800)),
            Some(Effect::Stop { .. })
        ));
        assert_eq!(state.stage, Stage::Processing);

        assert!(state.on_input(dt(true), t0 + ms(1000)).is_none());
        assert!(state.on_input(dt(false), t0 + ms(1040)).is_none());
        assert!(state.on_deadline(t0 + ms(1040) + RELEASE_GRACE).is_none());
        assert!(
            state.on_processing_finished().is_none(),
            "a tap while busy must not start a recording on drain"
        );
        assert_eq!(state.stage, Stage::Idle);
    }

    #[test]
    fn double_tap_autorepeat_hold_is_one_hold() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        let mut clock = t0;
        let mut effects = Vec::new();

        effects.extend(state.on_input(dt(true), clock));
        // ~600ms of auto-repeat pairs a few ms apart.
        for _ in 0..60 {
            clock += ms(5);
            effects.extend(state.on_input(dt(false), clock));
            clock += ms(5);
            effects.extend(state.on_input(dt(true), clock));
            assert_eq!(
                state.next_deadline(),
                None,
                "auto-repeat press cancels the deferred release and opens no window"
            );
            assert!(!state.is_locked(), "no tap may be classified mid-burst");
        }
        clock += ms(5);
        effects.extend(state.on_input(dt(false), clock));
        effects.extend(state.on_deadline(clock + RELEASE_GRACE));

        let starts = effects
            .iter()
            .filter(|e| matches!(e, Effect::Start { .. }))
            .count();
        let stops = effects
            .iter()
            .filter(|e| matches!(e, Effect::Stop { .. }))
            .count();
        assert_eq!((starts, stops, effects.len()), (1, 1, 2), "{effects:?}");
        assert_eq!(state.stage, Stage::Processing);
    }

    #[test]
    fn double_tap_cancel_clears_window_and_locked_session() {
        // Cancel while the double-tap window is open.
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        assert!(dt_tap(&mut state, t0, ms(100)).is_none());
        state.on_cancel(true);
        assert_eq!(state.stage, Stage::Idle);
        assert!(!state.is_locked());
        assert_eq!(state.next_deadline(), None);
        assert!(matches!(
            state.on_input(dt(true), t0 + ms(2000)),
            Some(Effect::Start { .. })
        ));

        // Cancel during a locked hands-free session.
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        dt_locked(&mut state, t0, 300);
        state.on_cancel(true);
        assert_eq!(state.stage, Stage::Idle);
        assert!(!state.is_locked());
        assert_eq!(state.next_deadline(), None);
        assert!(matches!(
            state.on_input(dt(true), t0 + ms(2000)),
            Some(Effect::Start { .. })
        ));
    }

    #[test]
    fn next_deadline_is_the_earliest_pending() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        assert_eq!(state.next_deadline(), None);

        assert!(matches!(
            state.on_input(dt(true), t0),
            Some(Effect::Start { .. })
        ));
        let grace = t0 + ms(100);
        let window = t0 + ms(500);
        state.pending_release = Some(PendingRelease {
            binding_id: BINDING.to_string(),
            hotkey_string: BINDING.to_string(),
            deadline: grace,
            released_at: t0 + ms(50),
            hold_threshold: HOLD_THRESHOLD,
            mode: ShortcutActivation::PushToTalkDoubleTap,
        });
        assert_eq!(state.next_deadline(), Some(grace), "only the grace");

        state
            .hold
            .as_mut()
            .expect("recording has hold bookkeeping")
            .tap_deadline = Some(window);
        assert_eq!(state.next_deadline(), Some(grace), "grace is earlier");

        state
            .pending_release
            .as_mut()
            .expect("grace still pending")
            .deadline = t0 + ms(700);
        assert_eq!(state.next_deadline(), Some(window), "window is earlier");

        state.pending_release = None;
        assert_eq!(state.next_deadline(), Some(window), "only the window");
    }

    // ---------------------------------------------------------------------
    // cancel-anywhere: the stage arms the cancel key while a dictation is in
    // flight (recording or processing), and only a change touches it.
    // ---------------------------------------------------------------------

    #[test]
    fn is_busy_follows_the_stage() {
        let t0 = Instant::now();

        // Idle -> Recording -> Processing -> Idle.
        let mut state = CoordinatorState::new();
        assert!(!state.is_busy(), "a new coordinator is idle");
        assert!(matches!(
            state.on_input(dt(true), t0),
            Some(Effect::Start { .. })
        ));
        assert!(state.is_busy(), "recording is busy");
        assert!(state.on_input(dt(false), t0 + ms(400)).is_none());
        assert!(matches!(
            state.on_deadline(t0 + ms(400) + RELEASE_GRACE),
            Some(Effect::Stop { .. })
        ));
        assert_eq!(state.stage, Stage::Processing);
        assert!(state.is_busy(), "processing is busy");
        assert!(state.on_processing_finished().is_none());
        assert!(!state.is_busy(), "drained pipeline is idle");

        // A lone tap discarded when its window runs out.
        let mut state = CoordinatorState::new();
        assert!(dt_tap(&mut state, t0, ms(100)).is_none());
        assert!(state.is_busy(), "the open double-tap window still records");
        assert!(matches!(
            state.on_deadline(t0 + DOUBLE_TAP_WINDOW),
            Some(Effect::Discard { .. })
        ));
        assert!(!state.is_busy(), "a discard leaves the coordinator idle");

        // A cancel while recording.
        let mut state = CoordinatorState::new();
        assert!(matches!(
            state.on_input(dt(true), t0),
            Some(Effect::Start { .. })
        ));
        state.on_cancel(true);
        assert!(!state.is_busy(), "a cancel while recording leaves it idle");

        // A start the microphone refused.
        let mut state = CoordinatorState::new();
        assert!(matches!(
            state.on_input(dt(true), t0),
            Some(Effect::Start { .. })
        ));
        state.on_start_result(BINDING, false);
        assert!(!state.is_busy(), "a failed start rolls back to idle");
    }

    #[test]
    fn cancel_during_processing_stays_busy_until_drained() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        hold_or_toggle_into_processing(&mut state, t0);
        // A press remembered while busy, then the cancel.
        assert!(state
            .on_input(input(ShortcutActivation::HoldOrToggle, true), t0 + ms(1000))
            .is_none());

        state.on_cancel(false);
        assert!(
            state.is_busy(),
            "the pipeline is still running; the coordinator waits for it"
        );

        assert!(
            state.on_processing_finished().is_none(),
            "the cancel abandoned the remembered press"
        );
        assert!(!state.is_busy());
    }

    #[test]
    fn drain_with_remembered_press_stays_busy() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        hold_or_toggle_into_processing(&mut state, t0);
        assert!(state.on_input(ptt_input(true), t0 + ms(1000)).is_none());
        assert!(state.is_busy());

        let effect = state.on_processing_finished();
        assert!(matches!(effect, Some(Effect::Start { .. })));
        assert_eq!(state.stage, Stage::Recording(BINDING.to_string()));
        assert!(
            state.is_busy(),
            "processing went straight to recording; the cancel key stays armed"
        );
    }

    #[test]
    fn cancel_key_changes_only_when_busy_flips() {
        let cases = [
            (false, false, None),
            (false, true, Some(true)),
            (true, false, Some(false)),
            (true, true, None),
        ];
        for (armed, busy, expected) in cases {
            assert_eq!(
                cancel_key_change(armed, busy),
                expected,
                "armed={armed} busy={busy}"
            );
        }
    }

    // ---------------------------------------------------------------------
    // session-limit: every continuous recording warns at 19 minutes and is
    // cut at 20 with a regular `Stop`, so what was said is still processed.
    // Synthetic clock: `t0 + 20 min` is arithmetic, no test waits.
    // ---------------------------------------------------------------------

    const LIMITED_MODES: [ShortcutActivation; 4] = [
        ShortcutActivation::PushToTalk,
        ShortcutActivation::PushToTalkDoubleTap,
        ShortcutActivation::HoldOrToggle,
        ShortcutActivation::Toggle,
    ];

    fn mins(n: u64) -> Duration {
        Duration::from_secs(n * 60)
    }

    /// A recording that keeps going on its own from `t0`: push-to-talk with
    /// the key held, double-tap locked by two taps, hold-or-toggle locked by a
    /// tap, or toggle.
    fn start_continuous(state: &mut CoordinatorState, mode: ShortcutActivation, t0: Instant) {
        match mode {
            ShortcutActivation::PushToTalkDoubleTap => dt_locked(state, t0, 300),
            ShortcutActivation::HoldOrToggle => {
                assert!(matches!(
                    state.on_input(input(mode, true), t0),
                    Some(Effect::Start { .. })
                ));
                assert!(state.on_input(input(mode, false), t0 + ms(100)).is_none());
                assert!(state.on_deadline(t0 + ms(100) + RELEASE_GRACE).is_none());
                assert!(state.is_locked());
            }
            ShortcutActivation::PushToTalk | ShortcutActivation::Toggle => {
                assert!(matches!(
                    state.on_input(input(mode, true), t0),
                    Some(Effect::Start { .. })
                ));
            }
        }
        assert_eq!(state.stage, Stage::Recording(BINDING.to_string()));
    }

    fn limit_warning() -> Option<Effect> {
        Some(Effect::LimitWarning {
            binding_id: BINDING.to_string(),
        })
    }

    #[test]
    fn session_limit_warns_at_nineteen_minutes_in_every_mode() {
        for mode in LIMITED_MODES {
            let mut state = CoordinatorState::new();
            let t0 = Instant::now();
            start_continuous(&mut state, mode, t0);

            assert_eq!(
                state.on_deadline(t0 + mins(19) - ms(1)),
                None,
                "{mode:?}: nothing a millisecond before 19 minutes"
            );
            assert_eq!(
                state.on_deadline(t0 + mins(19)),
                limit_warning(),
                "{mode:?}: the warning at 19 minutes"
            );
            assert_eq!(
                state.stage,
                Stage::Recording(BINDING.to_string()),
                "{mode:?}: the warning does not stop the recording"
            );
        }
    }

    #[test]
    fn session_limit_warns_once() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        dt_locked(&mut state, t0, 300);

        assert_eq!(state.on_deadline(t0 + mins(19)), limit_warning());
        assert_eq!(
            state.on_deadline(t0 + mins(19) + Duration::from_secs(1)),
            None
        );
        assert_eq!(
            state.on_deadline(t0 + mins(19) + Duration::from_secs(30)),
            None
        );
        assert_eq!(state.session_deadline(), Some(t0 + mins(20)));
    }

    #[test]
    fn session_limit_cuts_at_twenty_minutes_with_stop() {
        for mode in LIMITED_MODES {
            let mut state = CoordinatorState::new();
            let t0 = Instant::now();
            start_continuous(&mut state, mode, t0);
            let mut effects = Vec::new();

            effects.extend(state.on_deadline(t0 + mins(19)));
            assert_eq!(
                state.on_deadline(t0 + mins(20) - ms(1)),
                None,
                "{mode:?}: nothing a millisecond before 20 minutes"
            );
            let cut = state.on_deadline(t0 + mins(20));
            assert_eq!(
                cut,
                Some(Effect::Stop {
                    binding_id: BINDING.to_string(),
                    hotkey_string: BINDING.to_string(),
                }),
                "{mode:?}: the cut is a regular stop"
            );
            effects.extend(cut);
            assert_eq!(state.stage, Stage::Processing, "{mode:?}");
            assert!(
                !effects.iter().any(|e| matches!(e, Effect::Discard { .. })),
                "{mode:?}: never discarded: {effects:?}"
            );
        }
    }

    #[test]
    fn session_limit_overslept_cuts_without_warning() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        dt_locked(&mut state, t0, 300);

        assert!(
            matches!(state.on_deadline(t0 + mins(25)), Some(Effect::Stop { .. })),
            "waking past 20 minutes cuts straight away"
        );
        assert_eq!(state.stage, Stage::Processing);
        assert_eq!(state.on_deadline(t0 + mins(25) + ms(1)), None);
    }

    #[test]
    fn wake_deadline_is_the_earliest_of_input_and_session() {
        let t0 = Instant::now();
        let ptt = ShortcutActivation::PushToTalk;

        // Push-to-talk held, nothing pending: only the session mark.
        let mut state = CoordinatorState::new();
        assert!(matches!(
            state.on_input(input(ptt, true), t0),
            Some(Effect::Start { .. })
        ));
        assert_eq!(state.next_deadline(), None);
        assert_eq!(state.wake_deadline(), Some(t0 + mins(19)));
        // A deferred release is earlier.
        assert!(state.on_input(input(ptt, false), t0 + ms(1000)).is_none());
        assert_eq!(
            state.wake_deadline(),
            Some(t0 + ms(1000) + RELEASE_GRACE),
            "the release grace is earlier"
        );

        // The double-tap window is earlier.
        let mut state = CoordinatorState::new();
        assert!(dt_tap(&mut state, t0, ms(100)).is_none());
        assert_eq!(state.wake_deadline(), Some(t0 + ms(500)));

        // Locked: no input deadline, the session mark is still there.
        let mut state = CoordinatorState::new();
        dt_locked(&mut state, t0, 300);
        assert_eq!(
            state.next_deadline(),
            None,
            "next_deadline keeps its F1 meaning"
        );
        assert_eq!(state.wake_deadline(), Some(t0 + mins(19)));
        // After the warning, the cut.
        assert_eq!(state.on_deadline(t0 + mins(19)), limit_warning());
        assert_eq!(state.wake_deadline(), Some(t0 + mins(20)));
    }

    #[test]
    fn session_deadline_clears_when_the_recording_ends() {
        let t0 = Instant::now();
        let ptt = ShortcutActivation::PushToTalk;
        assert_eq!(CoordinatorState::new().session_deadline(), None);

        // Stopped by releasing the key.
        let mut state = CoordinatorState::new();
        assert!(matches!(
            state.on_input(input(ptt, true), t0),
            Some(Effect::Start { .. })
        ));
        assert!(state.on_input(input(ptt, false), t0 + ms(1000)).is_none());
        assert!(matches!(
            state.on_deadline(t0 + ms(1000) + RELEASE_GRACE),
            Some(Effect::Stop { .. })
        ));
        assert_eq!(state.session_deadline(), None, "stop");
        assert_eq!(state.on_deadline(t0 + mins(20)), None, "stop");

        // Discarded when the double-tap window ran out.
        let mut state = CoordinatorState::new();
        assert!(dt_tap(&mut state, t0, ms(100)).is_none());
        assert!(matches!(
            state.on_deadline(t0 + DOUBLE_TAP_WINDOW),
            Some(Effect::Discard { .. })
        ));
        assert_eq!(state.session_deadline(), None, "discard");
        assert_eq!(state.on_deadline(t0 + mins(20)), None, "discard");

        // Cancelled while recording.
        let mut state = CoordinatorState::new();
        dt_locked(&mut state, t0, 300);
        state.on_cancel(true);
        assert_eq!(state.session_deadline(), None, "cancel");
        assert_eq!(state.on_deadline(t0 + mins(20)), None, "cancel");

        // A start the microphone refused.
        let mut state = CoordinatorState::new();
        assert!(matches!(
            state.on_input(input(ptt, true), t0),
            Some(Effect::Start { .. })
        ));
        state.on_start_result(BINDING, false);
        assert_eq!(state.session_deadline(), None, "failed start");
        assert_eq!(state.on_deadline(t0 + mins(20)), None, "failed start");
    }

    #[test]
    fn drained_recording_measures_limit_from_key_down() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        hold_or_toggle_into_processing(&mut state, t0);
        let t1 = t0 + ms(1000);
        assert!(state.on_input(ptt_input(true), t1).is_none());

        // The pipeline drains later (about two seconds); no clock is passed.
        assert!(matches!(
            state.on_processing_finished(),
            Some(Effect::Start { .. })
        ));
        assert_eq!(state.session_deadline(), Some(t1 + mins(19)));
        assert_eq!(state.on_deadline(t1 + mins(19)), limit_warning());
        assert!(matches!(
            state.on_deadline(t1 + mins(20)),
            Some(Effect::Stop { .. })
        ));
    }

    #[test]
    fn release_after_limit_cut_starts_nothing() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        let ptt = ShortcutActivation::PushToTalk;
        start_continuous(&mut state, ptt, t0);
        assert_eq!(state.on_deadline(t0 + mins(19)), limit_warning());
        assert!(matches!(
            state.on_deadline(t0 + mins(20)),
            Some(Effect::Stop { .. })
        ));

        let released = t0 + mins(20) + Duration::from_secs(5);
        assert_eq!(state.on_input(input(ptt, false), released), None);
        assert_eq!(state.on_deadline(released + RELEASE_GRACE), None);
        assert_eq!(state.on_processing_finished(), None);
        assert_eq!(state.stage, Stage::Idle);
    }

    #[test]
    fn tap_after_limit_cut_starts_nothing() {
        let mut state = CoordinatorState::new();
        let t0 = Instant::now();
        dt_locked(&mut state, t0, 300);
        assert_eq!(state.on_deadline(t0 + mins(19)), limit_warning());
        assert!(matches!(
            state.on_deadline(t0 + mins(20)),
            Some(Effect::Stop { .. })
        ));

        let tap = t0 + mins(20) + Duration::from_secs(2);
        assert_eq!(state.on_input(dt(true), tap), None);
        assert_eq!(state.on_input(dt(false), tap + ms(100)), None);
        assert_eq!(state.on_deadline(tap + ms(100) + RELEASE_GRACE), None);
        assert_eq!(state.on_processing_finished(), None);
        assert_eq!(state.stage, Stage::Idle);
    }
}
