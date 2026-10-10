//! Per-dictation latency marks: the `FALA_TRACE` line and the row in `fala.sqlite`.
//!
//! The key clock starts in `TranscriptionCoordinator::send`, on the thread of the keyboard hook
//! (or the external trigger), before the coordinator channel and the `AudioRecordingManager`.
//! Nothing here reads the dictated text beyond counting its words.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use fala_core::Language;
use fala_postproc::Fallback;
use fala_storage::DictationMetrics;

/// A key stamp older than this when the action reads it belongs to another gesture (the
/// session-limit cut, a press remembered while the previous dictation was processing).
const STAMP_WINDOW: Duration = Duration::from_secs(1);

#[derive(Default)]
struct KeyClock {
    pressed: Option<Instant>,
    last_edge: Option<Instant>,
}

impl KeyClock {
    fn stamp(&mut self, is_pressed: bool, at: Instant) {
        if is_pressed {
            self.pressed = Some(at);
        }
        self.last_edge = Some(at);
    }

    /// The press that started the recording.
    fn press(&mut self, now: Instant) -> Instant {
        recent(self.pressed.take(), now)
    }

    /// The edge that stopped it: a push-to-talk release or a toggle press.
    fn release(&mut self, now: Instant) -> Instant {
        recent(self.last_edge.take(), now)
    }
}

fn recent(stamp: Option<Instant>, now: Instant) -> Instant {
    stamp
        .filter(|at| now.saturating_duration_since(*at) <= STAMP_WINDOW)
        .unwrap_or(now)
}

static CLOCK: Mutex<KeyClock> = Mutex::new(KeyClock {
    pressed: None,
    last_edge: None,
});
/// Key → pill of the recording in flight.
static KEY_TO_PILL: Mutex<Option<Duration>> = Mutex::new(None);
static DICTATIONS: AtomicU64 = AtomicU64::new(0);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A transcribe binding edge reached the coordinator.
pub(crate) fn key_event(is_pressed: bool) {
    lock(&CLOCK).stamp(is_pressed, Instant::now());
}

/// The instant the key that starts this recording went down.
pub(crate) fn key_pressed() -> Instant {
    lock(&CLOCK).press(Instant::now())
}

/// The pill was shown (`None` when the overlay is off).
pub(crate) fn pill_shown(key: Instant, shown: bool) {
    *lock(&KEY_TO_PILL) = shown.then(|| key.elapsed());
}

/// Key → pill of this recording and the instant the user let go.
pub(crate) fn key_released() -> (Option<Duration>, Instant) {
    let release = lock(&CLOCK).release(Instant::now());
    (lock(&KEY_TO_PILL).take(), release)
}

/// What one dictation measured, up to the start of the paste.
pub(crate) struct Measured {
    pub key_to_pill: Option<Duration>,
    pub release: Instant,
    pub asr_start: Instant,
    pub asr_done: Instant,
    pub format_start: Instant,
    pub format_done: Instant,
    pub paste_start: Instant,
    /// Samples at 16 kHz handed to the ASR.
    pub speech_samples: usize,
    pub words: usize,
    pub lang: Language,
    pub llm_attempted: bool,
    pub llm_used: bool,
    pub fallback: Option<Fallback>,
    pub model: String,
    pub app: Option<String>,
}

fn ms(d: Duration) -> u32 {
    u32::try_from(d.as_millis()).unwrap_or(u32::MAX)
}

fn fallback_str(f: Fallback) -> &'static str {
    match f {
        Fallback::Timeout => "timeout",
        Fallback::Http(_) => "http",
        Fallback::Network => "network",
        Fallback::InvalidResponse => "invalid",
    }
}

impl Measured {
    /// The row for `fala.sqlite`, and with `FALA_TRACE=1` the trace line, once the text is pasted.
    pub fn finish(self, pasted: Instant, dictation_id: Option<String>) -> DictationMetrics {
        let metrics = self.metrics(pasted, dictation_id, chrono::Local::now().fixed_offset());
        if std::env::var("FALA_TRACE").is_ok_and(|v| v == "1") {
            let n = DICTATIONS.fetch_add(1, Ordering::Relaxed) + 1;
            log::info!(target: "fala_trace", "{}", self.trace_line(n, pasted, &metrics));
        }
        metrics
    }

    fn metrics(
        &self,
        pasted: Instant,
        dictation_id: Option<String>,
        created_at: chrono::DateTime<chrono::FixedOffset>,
    ) -> DictationMetrics {
        DictationMetrics {
            dictation_id,
            created_at,
            e2e_ms: ms(pasted.saturating_duration_since(self.release)),
            asr_ms: ms(self.asr_done.saturating_duration_since(self.asr_start)),
            llm_ms: self.llm_attempted.then(|| {
                ms(self
                    .format_done
                    .saturating_duration_since(self.format_start))
            }),
            paste_ms: ms(pasted.saturating_duration_since(self.paste_start)),
            speech_ms: u32::try_from(self.speech_samples / 16).unwrap_or(u32::MAX),
            words: u32::try_from(self.words).unwrap_or(u32::MAX),
            lang: self.lang,
            llm_used: self.llm_used,
            fallback: self.fallback.map(|f| fallback_str(f).to_string()),
            model: self.model.clone(),
            app: self.app.clone(),
        }
    }

    fn trace_line(&self, n: u64, pasted: Instant, m: &DictationMetrics) -> String {
        let since_release = |at: Instant| ms(at.saturating_duration_since(self.release));
        let or_dash = |v: Option<u32>| v.map_or("-".to_string(), |v| v.to_string());
        format!(
            "event=dictation dictation={n} key_to_pill_ms={} release_to_asr_ms={} \
             release_to_llm_ms={} release_to_text_ms={} asr_ms={} llm_ms={} paste_ms={} \
             speech_ms={} words={} lang={} llm_used={} fallback={}",
            or_dash(self.key_to_pill.map(ms)),
            since_release(self.asr_done),
            since_release(self.format_done),
            since_release(pasted),
            m.asr_ms,
            or_dash(m.llm_ms),
            m.paste_ms,
            m.speech_ms,
            m.words,
            m.lang.tag(),
            u8::from(m.llm_used),
            self.fallback.map_or("none", fallback_str),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measured(release: Instant, llm: Option<Fallback>, attempted: bool) -> Measured {
        let at = |ms: u64| release + Duration::from_millis(ms);
        Measured {
            key_to_pill: Some(Duration::from_millis(30)),
            release,
            asr_start: at(10),
            asr_done: at(200),
            format_start: at(200),
            format_done: at(if attempted { 1100 } else { 201 }),
            paste_start: at(if attempted { 1100 } else { 201 }),
            speech_samples: 32_000,
            words: 17,
            lang: Language::PtBr,
            llm_attempted: attempted,
            llm_used: attempted && llm.is_none(),
            fallback: llm,
            model: "parakeet".to_string(),
            app: Some("notepad".to_string()),
        }
    }

    fn created_at() -> chrono::DateTime<chrono::FixedOffset> {
        chrono::Local::now().fixed_offset()
    }

    #[test]
    fn key_clock_uses_recent_stamps() {
        let now = Instant::now();
        let mut clock = KeyClock::default();
        clock.stamp(true, now - Duration::from_millis(200));
        assert_eq!(clock.press(now), now - Duration::from_millis(200));
        // Taken once: a second read without a new press falls back to now.
        assert_eq!(clock.press(now), now);

        clock.stamp(true, now - Duration::from_secs(2));
        assert_eq!(clock.press(now), now);

        // Push-to-talk: press, then the release is the last edge.
        clock.stamp(true, now - Duration::from_millis(900));
        clock.stamp(false, now - Duration::from_millis(60));
        assert_eq!(clock.release(now), now - Duration::from_millis(60));
        // Toggle: the stopping press is the last edge.
        clock.stamp(false, now - Duration::from_millis(500));
        clock.stamp(true, now - Duration::from_millis(5));
        assert_eq!(clock.release(now), now - Duration::from_millis(5));
        // The session-limit cut: no edge since, so now.
        assert_eq!(clock.release(now), now);
    }

    #[test]
    fn marks_become_metrics_and_trace() {
        let release = Instant::now();
        let pasted = release + Duration::from_millis(1140);
        let m = measured(release, None, true);
        let metrics = m.metrics(pasted, Some("id".to_string()), created_at());
        assert_eq!(
            (
                metrics.e2e_ms,
                metrics.asr_ms,
                metrics.llm_ms,
                metrics.paste_ms
            ),
            (1140, 190, Some(900), 40)
        );
        assert_eq!((metrics.speech_ms, metrics.words), (2000, 17));
        assert_eq!(metrics.dictation_id.as_deref(), Some("id"));
        assert_eq!(
            m.trace_line(3, pasted, &metrics),
            "event=dictation dictation=3 key_to_pill_ms=30 release_to_asr_ms=200 \
             release_to_llm_ms=1100 release_to_text_ms=1140 asr_ms=190 llm_ms=900 paste_ms=40 \
             speech_ms=2000 words=17 lang=pt-BR llm_used=1 fallback=none"
        );

        let timeout = measured(release, Some(Fallback::Timeout), true);
        let metrics = timeout.metrics(pasted, None, created_at());
        assert_eq!(metrics.fallback.as_deref(), Some("timeout"));
        assert!(!metrics.llm_used);
        assert!(timeout
            .trace_line(4, pasted, &metrics)
            .ends_with("llm_used=0 fallback=timeout"));

        let pasted = release + Duration::from_millis(241);
        let rules = measured(release, None, false);
        let metrics = rules.metrics(pasted, None, created_at());
        assert_eq!((metrics.e2e_ms, metrics.llm_ms), (241, None));
        let line = rules.trace_line(5, pasted, &metrics);
        assert!(line.contains(" release_to_llm_ms=201 "), "{line}");
        assert!(line.contains(" llm_ms=- "), "{line}");
    }
}
