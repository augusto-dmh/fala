//! A stop's transcript never just doesn't happen (ADR-0005: nothing is lost if the transcription
//! fails). An automatic transcription that fails without network, or because Scribe answered
//! `429`/`5xx`, waits in a `RetryQueue` and runs again on its own: on the backoff, as soon as a
//! probe of Scribe's host connects again (every 10 s while offline), a few seconds after the PC
//! wakes, and at the next start (`transcript-pending` next to the audio). The notes
//! the stop would have written follow the transcript that arrives, by a retry or a click. When
//! Scribe gives up (offline for the 4th time, `429`/`5xx` for the 4th, or any other error), the
//! local model transcribes the session (ADR-0005), marked local; a local failure is final.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, MutexGuard};
use std::time::{Duration, Instant};

use fala_asr::meeting::{reachable, Failure, Next, RetryQueue, PROBE_EVERY, SCRIBE_BASE_URL};
use fala_meeting::SessionId;
use fala_storage::MeetingTake;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Manager;

use super::pipeline::{MeetingLine, ProgressStage, Route};
use super::{MeetingError, MeetingManager};

/// In `audio/<id>/` while the session waits for a retry: empty for the whole session, or the
/// number of the take to transcribe.
const MARKER: &str = "transcript-pending";
/// Another transcription held the slot, or the session records again: try later, without
/// counting a failure.
const BUSY_RETRY: Duration = Duration::from_secs(30);
/// After a wake or a start, the network gets this long to come back before the retry.
const SETTLE: Duration = Duration::from_secs(10);
/// How long the notes owed to a local transcript wait for the network before trying anyway.
const NOTES_NETWORK_WAIT: Duration = Duration::from_secs(60 * 60);
/// How long the probe of Scribe's host waits for the TCP connection.
const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

/// What a meeting shows about an automatic transcript that has not arrived.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TranscriptWait {
    /// It runs again on its own.
    Waiting { reason: WaitReason },
    /// Retrying would not help, or the retries ran out: "Tentar de novo" is a click.
    Failed { error: MeetingError },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum WaitReason {
    /// The last try had no network.
    Offline,
    /// Scribe answered `429` or `5xx`.
    Provider,
    /// Not tried yet in this run (after a start, or behind another transcription).
    Queued,
    /// Scribe gave up: the local model transcribes it next.
    Local,
}

#[derive(Default)]
pub(crate) struct Retries {
    queue: RetryQueue<SessionId>,
    /// The take a retry transcribes; absent, the whole session.
    takes: HashMap<SessionId, MeetingTake>,
    failed: HashMap<SessionId, MeetingError>,
    /// The jobs the local model runs, after Scribe gave up.
    local: HashSet<SessionId>,
    /// Wakes the `meeting-retry` thread, started with the first retry.
    wake: Option<Sender<()>>,
}

/// What an automatic failure leads to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Retry(Failure),
    Later,
    Drop,
    Fail,
}

fn outcome(error: &MeetingError) -> Outcome {
    match error {
        MeetingError::Unreachable { status: None, .. } => Outcome::Retry(Failure::Offline),
        MeetingError::Unreachable {
            status: Some(_), ..
        } => Outcome::Retry(Failure::Busy),
        MeetingError::Busy | MeetingError::StillRecording => Outcome::Later,
        // Scribe refused the key: the meeting says so ("Tentar de novo" once it is fixed) rather
        // than hiding it behind a local transcript.
        MeetingError::InvalidKey => Outcome::Fail,
        // The person cancelled, or nothing is left to send.
        MeetingError::Cancelled
        | MeetingError::NoAudio
        | MeetingError::NotFound
        | MeetingError::MissingKey => Outcome::Drop,
        _ => Outcome::Retry(Failure::Final),
    }
}

/// Whether a transcript that arrived for `arrived` (a take, or the whole session) covers the job
/// that waits for `queued`.
fn covers(arrived: Option<u32>, queued: Option<u32>) -> bool {
    arrived.is_none() || arrived == queued
}

/// The job a new failure leaves queued: the whole session wins over a take, and two different
/// takes become the whole session.
fn merged_take(queued: Option<Option<u32>>, failed: Option<u32>) -> Option<u32> {
    match queued {
        None => failed,
        Some(queued) if queued == failed => failed,
        Some(_) => None,
    }
}

fn wait_reason(failures: u32, offline: bool) -> WaitReason {
    match (failures, offline) {
        (0, _) => WaitReason::Queued,
        (_, true) => WaitReason::Offline,
        (_, false) => WaitReason::Provider,
    }
}

/// The take a marker names (`Some(None)` for the whole session, `None` without a marker) and
/// whether Scribe already gave up (an `L` before the take).
fn read_marker(dir: &Path) -> Option<(Option<u32>, bool)> {
    let text = std::fs::read_to_string(dir.join(MARKER)).ok()?;
    let text = text.trim();
    let take = text.trim_start_matches('L');
    Some((take.parse().ok(), take.len() < text.len()))
}

impl MeetingManager {
    fn retries(&self) -> MutexGuard<'_, Retries> {
        self.retries.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// An automatic transcription failed: the session waits for its retry or for the local model,
    /// or the meeting shows the failure with "Tentar de novo".
    pub(crate) fn transcript_failed(
        &self,
        id: SessionId,
        take: Option<&MeetingTake>,
        error: MeetingError,
    ) {
        let now = Instant::now();
        // Without the key (or "local only") the try was already local.
        let local_route = self
            .with_store(|store| Ok(store.meeting(id)?))
            .is_ok_and(|record| self.route(&record) == Route::Local);
        let mut retries = self.retries();
        let queued = retries
            .queue
            .get(id)
            .map(|_| retries.takes.get(&id).map(|t| t.take));
        let was_local = local_route || retries.local.contains(&id);
        let outcome = match outcome(&error) {
            Outcome::Retry(_) if was_local => Outcome::Fail,
            other => other,
        };
        let next = match outcome {
            Outcome::Drop => {
                log::info!("meeting {id}: automatic transcription stopped: {error}");
                retries.queue.forget(id);
                retries.takes.remove(&id);
                retries.failed.remove(&id);
                retries.local.remove(&id);
                drop(retries);
                self.clear_marker(id);
                return;
            }
            // The local model failed too, or the key was refused: nothing else to try on its own.
            Outcome::Fail => {
                log::warn!("meeting {id}: automatic transcription failed: {error}");
                retries.queue.forget(id);
                retries.takes.remove(&id);
                retries.local.remove(&id);
                retries.failed.insert(id, error);
                drop(retries);
                self.clear_marker(id);
                return;
            }
            // Already waiting (the slot was busy, or the session records again): try later, quietly.
            Outcome::Later if queued.is_some() => {
                retries.queue.later(id, now + BUSY_RETRY);
                drop(retries);
                log::debug!("meeting {id}: retry put off ({error})");
                return;
            }
            Outcome::Later => {
                retries.queue.later(id, now + BUSY_RETRY);
                Next::RetryAt(now + BUSY_RETRY)
            }
            Outcome::Retry(failure) => retries.queue.failed(id, failure, now),
        };
        let job = merged_take(queued, take.map(|t| t.take));
        match (job, take) {
            (Some(_), Some(take)) => {
                retries.takes.insert(id, take.clone());
            }
            _ => {
                retries.takes.remove(&id);
            }
        }
        match next {
            Next::RetryAt(at) => {
                let waiting = retries.queue.get(id);
                self.run_retries(&mut retries);
                drop(retries);
                let secs = at.saturating_duration_since(now).as_secs();
                log::warn!("meeting {id}: transcription failed ({error}); retrying in {secs} s");
                self.write_marker(id, job, false);
                let reason =
                    waiting.map_or(WaitReason::Queued, |w| wait_reason(w.failures, w.offline));
                self.progress(id, ProgressStage::Waiting { reason });
            }
            // Scribe gave up: the local model takes the session, at once (ADR-0005).
            Next::GiveUp => {
                retries.local.insert(id);
                retries.queue.later(id, now);
                self.run_retries(&mut retries);
                drop(retries);
                log::warn!("meeting {id}: transcription failed ({error}); transcribing locally");
                self.write_marker(id, job, true);
                let reason = WaitReason::Local;
                self.progress(id, ProgressStage::Waiting { reason });
            }
        }
    }

    /// The transcript of `take` (or of the whole session) arrived: the job it covers leaves the
    /// queue. A take never clears another take's job, nor the whole session's.
    pub(crate) fn transcript_arrived(&self, id: SessionId, take: Option<u32>) {
        let mut retries = self.retries();
        let queued = retries.queue.get(id).is_some();
        let queued_take = retries.takes.get(&id).map(|t| t.take);
        if !covers(take, queued_take) {
            return;
        }
        retries.queue.forget(id);
        retries.takes.remove(&id);
        retries.failed.remove(&id);
        retries.local.remove(&id);
        drop(retries);
        if queued {
            self.clear_marker(id);
        }
    }

    /// The automatic transcript of a recorded session waits or failed on its own: the notes its
    /// stop would have written are owed to the transcript that arrives (never to an import, whose
    /// notes are always a click).
    pub(crate) fn notes_owed(&self, id: SessionId) -> bool {
        let pending = {
            let retries = self.retries();
            retries.queue.get(id).is_some() || retries.failed.contains_key(&id)
        };
        pending
            && self
                .with_store(|store| Ok(store.meeting(id)?.mode))
                .is_ok_and(|mode| mode != fala_meeting::SessionMode::Import)
    }

    /// The notes the stop would have written, after a transcript that came late.
    pub(crate) fn spawn_owed_notes(&self, id: SessionId, lines: Vec<MeetingLine>) {
        let app = self.app.clone();
        let spawned = std::thread::Builder::new()
            .name("meeting-owed-notes".to_string())
            .spawn(move || {
                if let Some(manager) = app.try_state::<Arc<MeetingManager>>() {
                    // A local transcript may arrive offline: the notes wait for the network.
                    let mut waited = Duration::ZERO;
                    while waited < NOTES_NETWORK_WAIT
                        && !reachable(fala_notes::DEFAULT_BASE_URL, PROBE_TIMEOUT)
                    {
                        std::thread::sleep(PROBE_EVERY);
                        waited += PROBE_EVERY;
                    }
                    manager.auto_notes(id, &lines);
                }
            });
        if let Err(e) = spawned {
            log::warn!("meeting {id}: notes thread did not start: {e}");
        }
    }

    /// What the meeting shows about its automatic transcript, if it has not arrived.
    pub(crate) fn transcript_wait(&self, id: SessionId) -> Option<TranscriptWait> {
        let retries = self.retries();
        if let Some(waiting) = retries.queue.get(id) {
            let reason = if retries.local.contains(&id) {
                WaitReason::Local
            } else {
                wait_reason(waiting.failures, waiting.offline)
            };
            return Some(TranscriptWait::Waiting { reason });
        }
        retries.failed.get(&id).map(|error| TranscriptWait::Failed {
            error: error.clone(),
        })
    }

    /// The PC woke: every waiting transcript is tried again once the network had time to return.
    pub(crate) fn wake_retries(&self) {
        let mut retries = self.retries();
        if retries.queue.next_due().is_none() {
            return;
        }
        log::info!("meeting: woke up; retrying the transcripts that wait");
        retries.queue.wake(Instant::now() + SETTLE);
        self.run_retries(&mut retries);
    }

    /// At startup: the sessions a previous run left waiting for a retry go back to the queue.
    pub(crate) fn resume_pending_transcripts(&self) {
        let ended = self.with_store(|store| {
            Ok(store
                .meetings()?
                .into_iter()
                .filter(|record| record.ended_at.is_some())
                .map(|record| record.id)
                .collect::<Vec<_>>())
        });
        for id in ended.unwrap_or_default() {
            let Some((take, local)) = read_marker(&self.session_dir(id)) else {
                continue;
            };
            let take = take.and_then(|n| {
                self.with_store(|store| Ok(store.meeting_takes(id)?))
                    .unwrap_or_default()
                    .into_iter()
                    .find(|t| t.take == n)
            });
            let mut retries = self.retries();
            if retries.queue.get(id).is_some() {
                continue;
            }
            log::info!("meeting {id}: its transcript was waiting for a retry");
            retries.queue.later(id, Instant::now() + SETTLE);
            if let Some(take) = take {
                retries.takes.insert(id, take);
            }
            if local {
                retries.local.insert(id);
            }
            self.run_retries(&mut retries);
        }
    }

    /// Wakes the `meeting-retry` thread to look at the queue again, starting it the first time.
    fn run_retries(&self, retries: &mut Retries) {
        if let Some(wake) = &retries.wake {
            if wake.send(()).is_ok() {
                return;
            }
        }
        let (wake, woken) = mpsc::channel::<()>();
        let app = self.app.clone();
        let spawned = std::thread::Builder::new()
            .name("meeting-retry".to_string())
            .spawn(move || {
                // Only a probe that connects after one that did not wakes the queue: a host that
                // answers while the request still fails must not turn the backoff into 10 s.
                let mut was_reachable = true;
                loop {
                    let Some(manager) = app.try_state::<Arc<MeetingManager>>() else {
                        return;
                    };
                    let (next, offline) = {
                        let retries = manager.retries();
                        (retries.queue.next_due(), retries.queue.any_offline())
                    };
                    let mut wait = next.map(|at| at.saturating_duration_since(Instant::now()));
                    if offline {
                        wait = Some(wait.map_or(PROBE_EVERY, |w| w.min(PROBE_EVERY)));
                    }
                    let woke = match wait {
                        Some(wait) => woken.recv_timeout(wait),
                        None => woken.recv().map_err(|_| RecvTimeoutError::Disconnected),
                    };
                    if woke == Err(RecvTimeoutError::Disconnected) {
                        return;
                    }
                    let early = next.is_some_and(|at| at > Instant::now());
                    if offline && early {
                        let now_reachable = reachable(SCRIBE_BASE_URL, PROBE_TIMEOUT);
                        if now_reachable && !was_reachable {
                            log::info!(
                                "meeting: the network is back; retrying the waiting transcripts"
                            );
                            manager.retries().queue.wake_offline(Instant::now());
                        }
                        was_reachable = now_reachable;
                    } else if !offline {
                        was_reachable = true;
                    }
                    let due = manager.retries().queue.take_due(Instant::now());
                    for id in due {
                        manager.retry(id);
                    }
                }
            });
        match spawned {
            Ok(_) => retries.wake = Some(wake),
            Err(e) => log::error!("meeting retry thread did not start: {e}"),
        }
    }

    fn retry(&self, id: SessionId) {
        let (take, route) = {
            let retries = self.retries();
            let route = retries.local.contains(&id).then_some(Route::Local);
            (retries.takes.get(&id).cloned(), route)
        };
        log::info!("meeting {id}: retrying the transcription ({route:?})");
        let owed = self.notes_owed(id);
        // On success `transcribe_by` takes the session out of the queue.
        match self.transcribe_by(id, take.as_ref(), route) {
            Ok(lines) if owed => self.spawn_owed_notes(id, lines),
            Ok(_) => {}
            // A click transcribed it meanwhile (this try found the slot busy): nothing waits.
            Err(_) if self.retries().queue.get(id).is_none() => {}
            Err(error) => self.transcript_failed(id, take.as_ref(), error),
        }
    }

    fn write_marker(&self, id: SessionId, take: Option<u32>, local: bool) {
        let take = take.map(|n| n.to_string()).unwrap_or_default();
        let text = if local { format!("L{take}") } else { take };
        if let Err(e) = std::fs::write(self.session_dir(id).join(MARKER), text) {
            log::warn!("meeting {id}: retry not marked for the next start: {e}");
        }
    }

    fn clear_marker(&self, id: SessionId) {
        match std::fs::remove_file(self.session_dir(id).join(MARKER)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => log::warn!("meeting {id}: retry marker not removed: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failures_lead_to_a_retry_a_later_try_or_nothing() {
        assert_eq!(
            outcome(&MeetingError::Unreachable {
                status: None,
                reason: "dns".into()
            }),
            Outcome::Retry(Failure::Offline)
        );
        assert_eq!(
            outcome(&MeetingError::Unreachable {
                status: Some(503),
                reason: "HTTP 503".into()
            }),
            Outcome::Retry(Failure::Busy)
        );
        assert_eq!(outcome(&MeetingError::Busy), Outcome::Later);
        assert_eq!(outcome(&MeetingError::InvalidKey), Outcome::Fail);
        assert_eq!(outcome(&MeetingError::StillRecording), Outcome::Later);
        for gone in [
            MeetingError::Cancelled,
            MeetingError::NoAudio,
            MeetingError::NotFound,
            MeetingError::MissingKey,
        ] {
            assert_eq!(outcome(&gone), Outcome::Drop, "{gone:?}");
        }
        assert_eq!(
            outcome(&MeetingError::Transcription("HTTP 401".into())),
            Outcome::Retry(Failure::Final)
        );
    }

    #[test]
    fn a_take_only_clears_its_own_job() {
        assert!(covers(None, None));
        assert!(covers(None, Some(2)), "a whole transcript covers any take");
        assert!(covers(Some(2), Some(2)));
        assert!(!covers(Some(3), Some(2)));
        assert!(
            !covers(Some(3), None),
            "a take never covers the whole session"
        );
    }

    #[test]
    fn the_whole_session_wins_over_a_take() {
        assert_eq!(merged_take(None, Some(2)), Some(2));
        assert_eq!(merged_take(None, None), None);
        assert_eq!(merged_take(Some(Some(2)), Some(2)), Some(2));
        assert_eq!(merged_take(Some(None), Some(3)), None);
        assert_eq!(merged_take(Some(Some(2)), Some(3)), None);
        assert_eq!(merged_take(Some(Some(2)), None), None);
    }

    #[test]
    fn the_wait_says_why() {
        assert_eq!(wait_reason(0, true), WaitReason::Queued);
        assert_eq!(wait_reason(2, true), WaitReason::Offline);
        assert_eq!(wait_reason(1, false), WaitReason::Provider);
        assert_eq!(
            serde_json::to_value(TranscriptWait::Waiting {
                reason: WaitReason::Offline
            })
            .unwrap(),
            serde_json::json!({ "state": "waiting", "reason": "offline" })
        );
    }

    #[test]
    fn the_marker_names_the_take() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read_marker(dir.path()), None);
        std::fs::write(dir.path().join(MARKER), "").unwrap();
        assert_eq!(read_marker(dir.path()), Some((None, false)));
        std::fs::write(dir.path().join(MARKER), "3").unwrap();
        assert_eq!(read_marker(dir.path()), Some((Some(3), false)));
        std::fs::write(dir.path().join(MARKER), "L3").unwrap();
        assert_eq!(read_marker(dir.path()), Some((Some(3), true)));
        std::fs::write(dir.path().join(MARKER), "L").unwrap();
        assert_eq!(read_marker(dir.path()), Some((None, true)));
    }
}
