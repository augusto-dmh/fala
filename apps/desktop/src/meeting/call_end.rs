//! The call ended and nobody answered "Parar a gravação?" (`meeting-auto-stop`): the recording
//! stops when the system channel stayed quiet since the card appeared (`fala_meeting::CallQuiet`),
//! and the stop offers [Retomar], a take of the same session. ADR-0005: resuming is a click, and
//! stopping records nothing.

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use fala_meeting::{Input, SessionId, SessionState};

use super::{millis, now_local, MeetingError, MeetingManager};

/// How often the stopped session's transcription looks at [Retomar], and the resume retries.
const POLL: Duration = Duration::from_millis(250);
/// The longest the transcription waits: the card's two minutes and a margin.
const WAIT_AT_MOST: Duration = Duration::from_secs(130);
/// [Retomar] while the stop's audio is still being retained (a few seconds; minutes for hours of
/// audio): how long the click keeps trying.
const RESUME_RETRY_FOR: Duration = Duration::from_secs(120);

/// [Retomar] for a session the call's end stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResumeOffer {
    /// On screen.
    Open(SessionId),
    /// Clicked: the take is starting.
    Resuming(SessionId),
}

impl ResumeOffer {
    fn session(self) -> SessionId {
        match self {
            ResumeOffer::Open(id) | ResumeOffer::Resuming(id) => id,
        }
    }
}

/// Whether the transcription of `id` still waits for the answer to [Retomar].
fn waits(offer: Option<ResumeOffer>, id: SessionId, card_up: bool) -> bool {
    match offer {
        Some(ResumeOffer::Open(open)) => open == id && card_up,
        Some(ResumeOffer::Resuming(resuming)) => resuming == id,
        None => false,
    }
}

impl MeetingManager {
    /// "Parar a gravação?" is on screen: the recording stops when it runs out, if the system
    /// stays quiet until then. Returns the session, while one records.
    pub(crate) fn arm_call_quiet(&self) -> Option<SessionId> {
        let mut inner = self.lock();
        let active = inner.active.as_mut()?;
        if !matches!(active.session.state(), SessionState::Recording { .. }) {
            return None;
        }
        active.call_quiet.arm();
        Some(active.session.id())
    }

    /// A pause, a sleep or [Continuar gravando]: the card no longer stops anything.
    pub(crate) fn hold_call_stop(&self) {
        let held = self
            .lock()
            .active
            .as_mut()
            .is_some_and(|active| active.call_quiet.disarm());
        if held {
            crate::call_prompt::keep_after_sound(&self.app);
        }
    }

    /// "Parar a gravação?" ran out: stops `id` if it still records with the system quiet since
    /// the card appeared. Stored as a stop for silence (the schema's reasons); the log says why.
    pub(crate) fn stop_call_ended(&self, id: &str) {
        let mut inner = self.lock();
        let Some(active) = inner.active.as_mut() else {
            return;
        };
        if active.session.id().to_string() != id {
            log::debug!("meeting {id}: call prompt ran out after the recording changed");
            return;
        }
        if !active.call_quiet.disarm() {
            log::info!("meeting {id}: call prompt ran out with sound; the recording goes on");
            return;
        }
        match active.session.apply(millis(now_local()), Input::CallEnded) {
            Ok(effects) if !effects.is_empty() => {
                active.call_ended = true;
                log::info!("meeting {id}: call ended, stopping");
                self.run_effects(&mut inner, effects);
                self.publish(&mut inner);
            }
            Ok(_) => log::debug!("meeting {id}: call ended while not recording"),
            Err(e) => log::warn!("meeting {id}: call end refused: {e}"),
        }
    }

    /// [Retomar] was clicked: the stopped session's transcription keeps waiting until the take
    /// has started (or failed to).
    pub(crate) fn resuming_after_call(&self, id: &str) {
        let mut inner = self.lock();
        if let Some(ResumeOffer::Open(open)) = inner.resume_offer {
            if open.to_string() == id {
                inner.resume_offer = Some(ResumeOffer::Resuming(open));
            }
        }
    }

    /// [Retomar]: a take of `id` (`resume_session`, the "continue" of a finished session). The
    /// stop's audio may still be retained for a few seconds; the click waits for it.
    pub(crate) fn resume_after_call(&self, id: &str) {
        let started = Instant::now();
        let result = loop {
            match self.resume_session(id) {
                Err(MeetingError::Busy) if started.elapsed() < RESUME_RETRY_FOR => {
                    std::thread::sleep(POLL);
                }
                result => break result,
            }
        };
        if let Err(e) = result {
            log::warn!("meeting {id}: not resumed after the call ended: {e}");
        }
        let mut inner = self.lock();
        if matches!(inner.resume_offer, Some(ResumeOffer::Resuming(s)) if s.to_string() == id) {
            inner.resume_offer = None;
        }
    }

    /// After a stop by the call's end, the transcription waits while [Retomar] is on screen
    /// (Granola waits ~10 s before its notes). `true` when the person resumed: the session was
    /// never transcribed, so the take's stop transcribes it whole.
    pub(super) fn resumed_after_call(&self, id: SessionId) -> bool {
        let started = Instant::now();
        loop {
            let offer = self.lock().resume_offer;
            let quitting = self.quit_after_stop.load(Ordering::SeqCst);
            if quitting
                || started.elapsed() >= WAIT_AT_MOST
                || !waits(offer, id, crate::call_prompt::offers_resume(id))
            {
                break;
            }
            std::thread::sleep(POLL);
        }
        let mut inner = self.lock();
        if inner
            .resume_offer
            .is_some_and(|offer| offer.session() == id)
        {
            inner.resume_offer = None;
        }
        let resumed = inner
            .active
            .as_ref()
            .is_some_and(|active| active.session.id() == id);
        if resumed {
            log::info!("meeting {id}: resumed after the call ended; transcribed with the take");
        }
        resumed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_transcription_waits_only_for_its_own_offer() {
        let id = SessionId::from_parts(1_727_000_000_000, 0);
        let other = SessionId::from_parts(1_727_000_000_000, 1);
        // [Retomar] on screen for this session: wait; once the card goes, transcribe.
        assert!(waits(Some(ResumeOffer::Open(id)), id, true));
        assert!(!waits(Some(ResumeOffer::Open(id)), id, false));
        // Clicked: wait even with the card gone, until the take started or failed.
        assert!(waits(Some(ResumeOffer::Resuming(id)), id, false));
        // Another session's offer, or none (a stop by the person, a take): no wait.
        assert!(!waits(Some(ResumeOffer::Open(other)), id, true));
        assert!(!waits(Some(ResumeOffer::Resuming(other)), id, false));
        assert!(!waits(None, id, true));
    }
}
