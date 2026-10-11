//! Pausing the media playing on the PC while Fala records, and resuming only what Fala paused.
//!
//! On Windows the media sessions come from the system media transport controls (SMTC). A session
//! that was not playing is never touched, so a player the user had paused never starts to play.
//! There is no blind `VK_MEDIA_PLAY_PAUSE` fallback: without SMTC, and on Linux until MPRIS in
//! phase 3, nothing is paused.
//!
//! Every pause and resume runs in order on one `media` thread, which owns the WinRT objects, so a
//! slow player never delays the dictation or the meeting that asked.

use log::{debug, warn};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;

/// Who asked for the pause. Each one resumes only what it paused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Owner {
    Dictation,
    Meeting,
}

/// What a recording event does to the media playing on the PC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MediaAction {
    Pause,
    Resume,
}

/// The playback state of one media session, as far as pausing cares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Playback {
    Playing,
    Paused,
    /// Stopped, closed, changing, or unreadable.
    Other,
}

/// The media sessions of the OS.
pub(crate) trait MediaControl {
    type Session;
    fn sessions(&self) -> Vec<Self::Session>;
    fn playback(&self, session: &Self::Session) -> Playback;
    /// `true` when the player accepted the pause.
    fn pause(&self, session: &Self::Session) -> bool;
    /// `true` when the player accepted the play.
    fn play(&self, session: &Self::Session) -> bool;
}

/// The sessions one owner paused and has not resumed yet.
struct Paused<S> {
    sessions: Vec<S>,
}

impl<S> Paused<S> {
    const fn new() -> Self {
        Self {
            sessions: Vec::new(),
        }
    }

    /// Pauses every session that is playing and remembers it. A session this owner already
    /// paused is not playing, so a second pause never records it twice.
    fn pause_playing<C: MediaControl<Session = S>>(&mut self, control: &C) -> usize {
        let mut paused = 0;
        for session in control.sessions() {
            if control.playback(&session) == Playback::Playing && control.pause(&session) {
                self.sessions.push(session);
                paused += 1;
            }
        }
        paused
    }

    /// Plays again what this owner paused, and forgets it. A session that stopped or closed
    /// since is left alone; one still reported as playing (the pause had not landed yet) is
    /// played anyway, which keeps it from ending up paused.
    fn resume<C: MediaControl<Session = S>>(&mut self, control: &C) -> usize {
        let mut resumed = 0;
        for session in self.sessions.drain(..) {
            if control.playback(&session) != Playback::Other && control.play(&session) {
                resumed += 1;
            }
        }
        resumed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Command {
    Pause(Owner),
    Resume(Owner),
}

/// The state of the `media` thread: the OS control, opened on first use and retried after a
/// failure, and what each owner paused.
struct Worker<C: MediaControl> {
    control: Option<C>,
    dictation: Paused<C::Session>,
    meeting: Paused<C::Session>,
}

impl<C: MediaControl> Worker<C> {
    fn new() -> Self {
        Self {
            control: None,
            dictation: Paused::new(),
            meeting: Paused::new(),
        }
    }

    fn handle(&mut self, command: Command, open: impl FnOnce() -> Option<C>) {
        if self.control.is_none() {
            self.control = open();
        }
        let Some(control) = &self.control else {
            debug!("media {command:?}: no media control on this system");
            return;
        };
        let (Command::Pause(owner) | Command::Resume(owner)) = command;
        let paused = match owner {
            Owner::Dictation => &mut self.dictation,
            Owner::Meeting => &mut self.meeting,
        };
        match command {
            Command::Pause(_) => {
                let count = paused.pause_playing(control);
                debug!("media paused for {owner:?}: {count} session(s)");
            }
            Command::Resume(_) => {
                let count = paused.resume(control);
                debug!("media resumed for {owner:?}: {count} session(s)");
            }
        }
    }
}

/// Whether one recording asked for a pause it has not resumed yet. The recording's generation is
/// checked and the pause sent under one lock, so a stop that invalidated the generation and then
/// resumed is never overtaken by a pause that arrives late.
#[derive(Debug, Default)]
pub(crate) struct PauseGate {
    paused: Mutex<bool>,
}

impl PauseGate {
    /// Runs `pause` when `still_current` holds and no pause is pending; `true` when it ran.
    pub(crate) fn pause_if(
        &self,
        still_current: impl FnOnce() -> bool,
        pause: impl FnOnce(),
    ) -> bool {
        let mut paused = self.paused.lock().unwrap_or_else(|e| e.into_inner());
        if *paused || !still_current() {
            return false;
        }
        pause();
        *paused = true;
        true
    }

    /// Runs `resume` when a pause is pending; `true` when it ran.
    pub(crate) fn resume(&self, resume: impl FnOnce()) -> bool {
        let mut paused = self.paused.lock().unwrap_or_else(|e| e.into_inner());
        if !*paused {
            return false;
        }
        resume();
        *paused = false;
        true
    }
}

static MEDIA: Mutex<Option<Sender<Command>>> = Mutex::new(None);

/// Pauses what is playing, for `owner` to resume later.
pub(crate) fn pause(owner: Owner) {
    send(Command::Pause(owner));
}

/// Plays again what `owner` paused; nothing when it paused nothing.
pub(crate) fn resume(owner: Owner) {
    send(Command::Resume(owner));
}

fn send(command: Command) {
    let mut media = MEDIA.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(tx) = media.as_ref() {
        if tx.send(command).is_ok() {
            return;
        }
        // The thread is gone (a panic ended it); what it had paused is lost with it.
        warn!("media thread is gone; starting a new one");
    }
    let (tx, rx) = mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("media".into())
        .spawn(move || run(rx));
    match spawned {
        Ok(_) => {
            if tx.send(command).is_ok() {
                *media = Some(tx);
            }
        }
        Err(e) => warn!("media {command:?} skipped: the media thread did not start: {e}"),
    }
}

fn run(commands: Receiver<Command>) {
    let mut worker = Worker::<platform::Control>::new();
    while let Ok(command) = commands.recv() {
        worker.handle(command, platform::open);
    }
}

#[cfg(windows)]
mod platform {
    use super::{MediaControl, Playback};
    use log::{debug, warn};
    use std::time::{Duration, Instant};
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession as Session,
        GlobalSystemMediaTransportControlsSessionManager as Manager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
    };
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

    /// The longest one WinRT call may take. `get()` would wait forever on a player that never
    /// answers, and every later pause and resume would queue behind it.
    const OP_WAIT: Duration = Duration::from_secs(2);

    pub(super) struct Control {
        manager: Manager,
    }

    pub(super) fn open() -> Option<Control> {
        // No-op when the thread already joined the MTA.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let manager = match Manager::RequestAsync() {
            Ok(op) => wait(|| op.Status().map(|s| s.0), || op.GetResults()),
            Err(e) => {
                warn!("media sessions unavailable: {e}");
                None
            }
        };
        if manager.is_none() {
            warn!("media sessions unavailable: the session manager did not open");
        }
        manager.map(|manager| Control { manager })
    }

    /// Waits for an operation by polling its status (0 started, 1 completed), at most
    /// [`OP_WAIT`]; `None` on a timeout, a cancel or an error.
    fn wait<T>(
        status: impl Fn() -> windows::core::Result<i32>,
        results: impl FnOnce() -> windows::core::Result<T>,
    ) -> Option<T> {
        let started = Instant::now();
        loop {
            match status() {
                Ok(0) => {}
                Ok(1) => return results().ok(),
                _ => return None,
            }
            if started.elapsed() >= OP_WAIT {
                return None;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn app(session: &Session) -> String {
        session
            .SourceAppUserModelId()
            .map(|id| id.to_string())
            .unwrap_or_default()
    }

    impl MediaControl for Control {
        type Session = Session;

        fn sessions(&self) -> Vec<Session> {
            let Ok(sessions) = self.manager.GetSessions() else {
                return Vec::new();
            };
            let count = sessions.Size().unwrap_or(0);
            (0..count).filter_map(|i| sessions.GetAt(i).ok()).collect()
        }

        fn playback(&self, session: &Session) -> Playback {
            match session
                .GetPlaybackInfo()
                .and_then(|info| info.PlaybackStatus())
            {
                Ok(Status::Playing) => Playback::Playing,
                Ok(Status::Paused) => Playback::Paused,
                _ => Playback::Other,
            }
        }

        fn pause(&self, session: &Session) -> bool {
            let accepted = session
                .TryPauseAsync()
                .ok()
                .and_then(|op| wait(|| op.Status().map(|s| s.0), || op.GetResults()))
                == Some(true);
            debug!("media pause of '{}': accepted={accepted}", app(session));
            accepted
        }

        fn play(&self, session: &Session) -> bool {
            let accepted = session
                .TryPlayAsync()
                .ok()
                .and_then(|op| wait(|| op.Status().map(|s| s.0), || op.GetResults()))
                == Some(true);
            debug!("media play of '{}': accepted={accepted}", app(session));
            accepted
        }
    }
}

/// Linux (MPRIS in phase 3) and every other system: no media control, nothing is paused.
#[cfg(not(windows))]
mod platform {
    use super::{MediaControl, Playback};

    pub(super) struct Control;

    pub(super) fn open() -> Option<Control> {
        None
    }

    impl MediaControl for Control {
        type Session = ();

        fn sessions(&self) -> Vec<()> {
            Vec::new()
        }

        fn playback(&self, _session: &()) -> Playback {
            Playback::Other
        }

        fn pause(&self, _session: &()) -> bool {
            false
        }

        fn play(&self, _session: &()) -> bool {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, MediaControl, Owner, PauseGate, Paused, Playback, Worker};
    use std::cell::RefCell;

    /// Media sessions by index, with every pause and play the code sent.
    struct FakeMedia {
        state: RefCell<Vec<Playback>>,
        calls: RefCell<Vec<(&'static str, usize)>>,
        /// Players that refuse a pause.
        refuses_pause: Vec<usize>,
    }

    impl FakeMedia {
        fn new(state: &[Playback]) -> Self {
            Self {
                state: RefCell::new(state.to_vec()),
                calls: RefCell::new(Vec::new()),
                refuses_pause: Vec::new(),
            }
        }

        fn set(&self, session: usize, playback: Playback) {
            self.state.borrow_mut()[session] = playback;
        }

        fn add(&self, playback: Playback) -> usize {
            let mut state = self.state.borrow_mut();
            state.push(playback);
            state.len() - 1
        }

        fn plays(&self) -> Vec<usize> {
            self.calls
                .borrow()
                .iter()
                .filter(|(call, _)| *call == "play")
                .map(|(_, session)| *session)
                .collect()
        }

        fn pauses(&self) -> Vec<usize> {
            self.calls
                .borrow()
                .iter()
                .filter(|(call, _)| *call == "pause")
                .map(|(_, session)| *session)
                .collect()
        }
    }

    impl MediaControl for &FakeMedia {
        type Session = usize;

        fn sessions(&self) -> Vec<usize> {
            (0..self.state.borrow().len()).collect()
        }

        fn playback(&self, session: &usize) -> Playback {
            self.state.borrow()[*session]
        }

        fn pause(&self, session: &usize) -> bool {
            self.calls.borrow_mut().push(("pause", *session));
            if self.refuses_pause.contains(session) {
                return false;
            }
            self.set(*session, Playback::Paused);
            true
        }

        fn play(&self, session: &usize) -> bool {
            self.calls.borrow_mut().push(("play", *session));
            self.set(*session, Playback::Playing);
            true
        }
    }

    use Playback::{Other, Paused as P, Playing};

    #[test]
    fn resumes_only_what_it_paused() {
        // Spotify playing, a YouTube tab playing, a podcast app the user had paused.
        let media = FakeMedia::new(&[Playing, Playing, P]);
        let mut paused = Paused::new();
        assert_eq!(paused.pause_playing(&&media), 2);
        assert_eq!(media.pauses(), vec![0, 1]);
        assert_eq!(*media.state.borrow(), vec![P, P, P]);

        assert_eq!(paused.resume(&&media), 2);
        assert_eq!(media.plays(), vec![0, 1]);
        assert_eq!(*media.state.borrow(), vec![Playing, Playing, P]);
        // Nothing left to resume.
        assert_eq!(paused.resume(&&media), 0);
        assert_eq!(media.plays(), vec![0, 1]);
    }

    #[test]
    fn a_paused_player_never_starts_playing() {
        let media = FakeMedia::new(&[P, Other, P]);
        let mut paused = Paused::new();
        assert_eq!(paused.pause_playing(&&media), 0);
        assert_eq!(paused.resume(&&media), 0);
        assert!(media.calls.borrow().is_empty());
    }

    #[test]
    fn a_refused_pause_is_not_resumed() {
        let mut media = FakeMedia::new(&[Playing, Playing]);
        media.refuses_pause = vec![1];
        let mut paused = Paused::new();
        assert_eq!(paused.pause_playing(&&media), 1);
        assert_eq!(paused.resume(&&media), 1);
        assert_eq!(media.plays(), vec![0]);
    }

    #[test]
    fn a_player_closed_or_stopped_meanwhile_is_left_alone() {
        let media = FakeMedia::new(&[Playing, Playing]);
        let mut paused = Paused::new();
        paused.pause_playing(&&media);
        // The user stopped one player during the dictation.
        media.set(1, Other);
        assert_eq!(paused.resume(&&media), 1);
        assert_eq!(media.plays(), vec![0]);
    }

    #[test]
    fn a_second_pause_does_not_record_a_session_twice() {
        let media = FakeMedia::new(&[Playing]);
        let mut paused = Paused::new();
        assert_eq!(paused.pause_playing(&&media), 1);
        // A player that started meanwhile is paused too; the first one is not counted again.
        let late = media.add(Playing);
        assert_eq!(paused.pause_playing(&&media), 1);
        assert_eq!(media.pauses(), vec![0, late]);
        assert_eq!(paused.resume(&&media), 2);
        assert_eq!(media.plays(), vec![0, late]);
    }

    #[test]
    fn dictation_and_meeting_resume_their_own_pauses() {
        let media = FakeMedia::new(&[Playing]);
        let mut worker = Worker::<&FakeMedia>::new();
        worker.handle(Command::Pause(Owner::Meeting), || Some(&media));
        let second = media.add(Playing);
        worker.handle(Command::Pause(Owner::Dictation), || Some(&media));
        assert_eq!(media.pauses(), vec![0, second]);

        worker.handle(Command::Resume(Owner::Dictation), || Some(&media));
        assert_eq!(media.plays(), vec![second]);
        worker.handle(Command::Resume(Owner::Meeting), || Some(&media));
        assert_eq!(media.plays(), vec![second, 0]);
    }

    #[test]
    fn without_media_control_nothing_happens_and_open_is_retried() {
        let media = FakeMedia::new(&[Playing]);
        let mut worker = Worker::<&FakeMedia>::new();
        let mut opens = 0;
        worker.handle(Command::Pause(Owner::Dictation), || {
            opens += 1;
            None
        });
        worker.handle(Command::Resume(Owner::Dictation), || {
            opens += 1;
            None
        });
        assert_eq!(opens, 2);
        assert!(media.calls.borrow().is_empty());
        // SMTC came back: the next pause uses it, and the control is kept.
        worker.handle(Command::Pause(Owner::Dictation), || Some(&media));
        worker.handle(Command::Resume(Owner::Dictation), || {
            opens += 1;
            None
        });
        assert_eq!(opens, 2);
        assert_eq!(media.plays(), vec![0]);
    }

    #[test]
    fn a_stale_pause_never_follows_its_resume() {
        let gate = PauseGate::default();
        let sent = RefCell::new(Vec::new());
        // The stop resumed first (nothing pending), then the late pause sees a stale generation.
        assert!(!gate.resume(|| sent.borrow_mut().push("resume")));
        assert!(!gate.pause_if(|| false, || sent.borrow_mut().push("pause")));
        assert!(sent.borrow().is_empty());

        // In order: one pause, one resume; a duplicate of either does nothing.
        assert!(gate.pause_if(|| true, || sent.borrow_mut().push("pause")));
        assert!(!gate.pause_if(|| true, || sent.borrow_mut().push("pause")));
        assert!(gate.resume(|| sent.borrow_mut().push("resume")));
        assert!(!gate.resume(|| sent.borrow_mut().push("resume")));
        assert_eq!(*sent.borrow(), vec!["pause", "resume"]);
    }
}
