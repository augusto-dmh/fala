//! Um stream por dispositivo de entrada, distribuído a quem assinou (ADR-0015).
//!
//! O ditado e o gravador de reunião pedem o mesmo microfone ao `MicHub` e recebem um `MicTap`
//! cada; o dispositivo tem um único `Mic`, aberto e drenado numa thread própria (o stream do
//! `cpal` não é `Send` em todo host), que reduz cada quadro ao canal de cada tap. O último tap a
//! cair fecha o stream. Abrir e fechar streams no mesmo mic é a suspeita dos degraus do spike 04.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, MutexGuard, Weak};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::DeviceTrait;

use crate::mic::{downmix, Mic};
use crate::AudioError;

/// Intervalo entre duas drenagens do stream: cada tap recebe o áudio com no máximo isso de atraso.
const DRAIN_EVERY: Duration = Duration::from_millis(5);
/// Segundos que um tap guarda sem ninguém drenar; o excesso é descartado e contado.
const TAP_SECONDS: usize = 2;

/// O stream de um dispositivo visto pelo hub: quadros intercalados na taxa do dispositivo.
pub trait MicSource {
    fn sample_rate(&self) -> u32;
    fn channels(&self) -> usize;
    fn name(&self) -> &str;
    fn failed(&self) -> bool;
    fn drain_frames(&mut self, out: &mut Vec<f32>);
}

impl MicSource for Mic {
    fn sample_rate(&self) -> u32 {
        Mic::sample_rate(self)
    }
    fn channels(&self) -> usize {
        Mic::channels(self)
    }
    fn name(&self) -> &str {
        Mic::name(self)
    }
    fn failed(&self) -> bool {
        Mic::failed(self)
    }
    fn drain_frames(&mut self, out: &mut Vec<f32>) {
        Mic::drain_frames(self, out);
    }
}

type Opener = Box<dyn FnOnce() -> Result<Box<dyn MicSource>, AudioError> + Send>;

/// Os streams abertos, por nome de dispositivo.
#[derive(Default)]
pub struct MicHub {
    feeds: Mutex<HashMap<String, Weak<Feed>>>,
}

impl MicHub {
    pub fn new() -> Self {
        Self::default()
    }

    /// Um tap do dispositivo: o stream aberto dele, ou um novo. `channel` como em
    /// `Mic::open_device`.
    pub fn open(&self, device: cpal::Device, channel: Option<usize>) -> Result<MicTap, AudioError> {
        let key = device
            .name()
            .map_err(|e| AudioError::Device(e.to_string()))?;
        self.open_with(&key, channel, move || {
            Ok(Box::new(Mic::open_device(device, None)?) as Box<dyn MicSource>)
        })
    }

    /// Como `open`, com quem abre o stream do dispositivo `key` dado por quem chama (os testes).
    pub fn open_with(
        &self,
        key: &str,
        channel: Option<usize>,
        opener: impl FnOnce() -> Result<Box<dyn MicSource>, AudioError> + Send + 'static,
    ) -> Result<MicTap, AudioError> {
        let mut feeds = self.feeds.lock().unwrap_or_else(|e| e.into_inner());
        feeds.retain(|_, feed| feed.strong_count() > 0);
        // Um stream com erro não serve a ninguém: o tap novo abre outro, e o velho fecha com o
        // último tap dele.
        // A trava fica durante a abertura: dois pedidos ao mesmo tempo não abrem dois streams.
        if let Some(feed) = feeds
            .get(key)
            .and_then(Weak::upgrade)
            .filter(|feed| !feed.shared.failed.load(Ordering::Relaxed))
        {
            return Ok(feed.tap(channel));
        }
        let (feed, tap) = Feed::start(key, channel, Box::new(opener))?;
        feeds.insert(key.to_owned(), Arc::downgrade(&feed));
        Ok(tap)
    }
}

/// O que a thread do dispositivo e os taps dividem.
struct Shared {
    taps: Mutex<Vec<Arc<TapQueue>>>,
    failed: AtomicBool,
    stop: AtomicBool,
}

impl Shared {
    fn taps(&self) -> MutexGuard<'_, Vec<Arc<TapQueue>>> {
        self.taps.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Um dispositivo aberto. Vive enquanto houver tap; soltar o último para e junta a thread, o que
/// fecha o stream.
struct Feed {
    shared: Arc<Shared>,
    rate: u32,
    name: String,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Feed {
    fn start(
        key: &str,
        channel: Option<usize>,
        opener: Opener,
    ) -> Result<(Arc<Feed>, MicTap), AudioError> {
        // O primeiro tap entra antes da primeira drenagem: o que o stream captura enquanto quem
        // abriu ainda se prepara chega a ele (o pré-buffer do ditado).
        let first = TapQueue::new(channel);
        let shared = Arc::new(Shared {
            taps: Mutex::new(vec![Arc::clone(&first)]),
            failed: AtomicBool::new(false),
            stop: AtomicBool::new(false),
        });
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let thread_shared = Arc::clone(&shared);
        let thread = std::thread::Builder::new()
            .name(format!("mic-hub {key}"))
            .spawn(move || {
                let mut source = match opener() {
                    Ok(source) => source,
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                let info = (source.sample_rate(), source.name().to_owned());
                if ready_tx.send(Ok(info)).is_err() {
                    return;
                }
                pump(source.as_mut(), &thread_shared);
            })
            .map_err(|e| AudioError::Stream(format!("thread do microfone: {e}")))?;
        let (rate, name) = match ready_rx.recv() {
            Ok(Ok(info)) => info,
            Ok(Err(e)) => {
                let _ = thread.join();
                return Err(e);
            }
            Err(_) => {
                let _ = thread.join();
                return Err(AudioError::Stream(
                    "a thread do microfone terminou antes de abrir".to_owned(),
                ));
            }
        };
        let feed = Arc::new(Feed {
            shared,
            rate,
            name,
            thread: Mutex::new(Some(thread)),
        });
        let tap = MicTap::new(&feed, first);
        Ok((feed, tap))
    }

    fn tap(self: &Arc<Self>, channel: Option<usize>) -> MicTap {
        let queue = TapQueue::new(channel);
        self.shared.taps().push(Arc::clone(&queue));
        MicTap::new(self, queue)
    }
}

impl Drop for Feed {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        let thread = self
            .thread
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        if let Some(thread) = thread {
            let _ = thread.join();
        }
    }
}

/// Drena o stream e reparte cada quadro entre os taps até o feed cair.
fn pump(source: &mut dyn MicSource, shared: &Shared) {
    let rate = source.sample_rate() as usize;
    let channels = source.channels().max(1);
    let mut frames = Vec::new();
    let mut mono = Vec::new();
    while !shared.stop.load(Ordering::Relaxed) {
        std::thread::sleep(DRAIN_EVERY);
        frames.clear();
        source.drain_frames(&mut frames);
        if source.failed() {
            shared.failed.store(true, Ordering::Relaxed);
        }
        if frames.is_empty() {
            continue;
        }
        for tap in shared.taps().iter() {
            mono.clear();
            downmix(&frames, channels, tap.channel, &mut mono);
            tap.push(&mono, rate * TAP_SECONDS);
        }
    }
}

/// A fila de um consumidor, já em mono.
struct TapQueue {
    channel: Option<usize>,
    samples: Mutex<VecDeque<f32>>,
    dropped: AtomicU64,
}

impl TapQueue {
    /// `channel` fora do alcance do dispositivo vira a média, como no `Mic`.
    fn new(channel: Option<usize>) -> Arc<Self> {
        Arc::new(Self {
            channel,
            samples: Mutex::new(VecDeque::new()),
            dropped: AtomicU64::new(0),
        })
    }

    fn lock(&self) -> MutexGuard<'_, VecDeque<f32>> {
        self.samples.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Como o ring do `Mic`: cheio, o que chega é descartado e contado.
    fn push(&self, mono: &[f32], capacity: usize) {
        let mut samples = self.lock();
        let room = capacity.saturating_sub(samples.len()).min(mono.len());
        samples.extend(&mono[..room]);
        let lost = (mono.len() - room) as u64;
        if lost > 0 {
            self.dropped.fetch_add(lost, Ordering::Relaxed);
        }
    }
}

/// Um consumidor de um microfone, com a API do `Mic`. Soltar o tap sai do stream; o último a sair
/// fecha o stream.
pub struct MicTap {
    feed: Arc<Feed>,
    queue: Arc<TapQueue>,
}

impl MicTap {
    fn new(feed: &Arc<Feed>, queue: Arc<TapQueue>) -> Self {
        Self {
            feed: Arc::clone(feed),
            queue,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.feed.rate
    }

    pub fn name(&self) -> &str {
        &self.feed.name
    }

    /// Amostras deste tap descartadas porque ninguém drenou a tempo.
    pub fn dropped(&self) -> u64 {
        self.queue.dropped.load(Ordering::Relaxed)
    }

    /// O stream reportou erro (dispositivo removido, por exemplo): reabra.
    pub fn failed(&self) -> bool {
        self.feed.shared.failed.load(Ordering::Relaxed)
    }

    /// Move para `out` tudo o que chegou a este tap.
    pub fn drain_into(&mut self, out: &mut Vec<f32>) {
        out.extend(self.queue.lock().drain(..));
    }
}

impl Drop for MicTap {
    fn drop(&mut self) {
        self.feed
            .shared
            .taps()
            .retain(|q| !Arc::ptr_eq(q, &self.queue));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;
    use std::time::Instant;

    use super::*;

    /// Dois canais, 48 kHz; o canal 0 vale `n`, o canal 1 vale `-n`, para `n` = 1, 2, 3...
    struct Fake {
        next: u32,
        failed: Arc<AtomicBool>,
        open: Arc<AtomicUsize>,
    }

    impl MicSource for Fake {
        fn sample_rate(&self) -> u32 {
            48_000
        }
        fn channels(&self) -> usize {
            2
        }
        fn name(&self) -> &str {
            "fake"
        }
        fn failed(&self) -> bool {
            self.failed.load(Ordering::Relaxed)
        }
        fn drain_frames(&mut self, out: &mut Vec<f32>) {
            for _ in 0..48 {
                self.next += 1;
                out.push(self.next as f32);
                out.push(-(self.next as f32));
            }
        }
    }

    impl Drop for Fake {
        fn drop(&mut self) {
            self.open.fetch_sub(1, Ordering::SeqCst);
        }
    }

    struct Counter {
        opened: Arc<AtomicUsize>,
        open: Arc<AtomicUsize>,
        failed: Arc<AtomicBool>,
    }

    impl Counter {
        fn new() -> Self {
            Self {
                opened: Arc::default(),
                open: Arc::default(),
                failed: Arc::default(),
            }
        }

        fn opener(&self) -> impl FnOnce() -> Result<Box<dyn MicSource>, AudioError> + Send + use<> {
            let (opened, open, failed) = (
                Arc::clone(&self.opened),
                Arc::clone(&self.open),
                Arc::clone(&self.failed),
            );
            move || {
                opened.fetch_add(1, Ordering::SeqCst);
                open.fetch_add(1, Ordering::SeqCst);
                Ok(Box::new(Fake {
                    next: 0,
                    failed,
                    open,
                }) as Box<dyn MicSource>)
            }
        }
    }

    /// Drena `tap` até ter ao menos `n` amostras.
    fn take(tap: &mut MicTap, n: usize) -> Vec<f32> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut out = Vec::new();
        while out.len() < n {
            assert!(Instant::now() < deadline, "o tap parou em {}", out.len());
            tap.drain_into(&mut out);
            std::thread::sleep(Duration::from_millis(2));
        }
        out
    }

    #[test]
    fn two_consumers_of_one_device_share_one_stream() {
        let hub = MicHub::new();
        let counter = Counter::new();
        let mut dictation = hub.open_with("mic", Some(1), counter.opener()).unwrap();
        let mut meeting = hub.open_with("mic", None, counter.opener()).unwrap();
        assert_eq!(counter.opened.load(Ordering::SeqCst), 1, "uma abertura só");
        assert_eq!(meeting.sample_rate(), 48_000);
        assert_eq!(meeting.name(), "fake");

        // O mesmo quadro chega aos dois, cada um no seu canal: o ditado lê o canal 1 (-n), a
        // reunião a média (0).
        let heard = take(&mut dictation, 480);
        assert!(heard.iter().all(|&x| x < 0.0));
        assert!(take(&mut meeting, 480).iter().all(|&x| x == 0.0));
        let steps: Vec<f32> = heard.windows(2).map(|w| w[0] - w[1]).collect();
        assert!(steps.iter().all(|&d| d == 1.0), "sem buraco nem repetição");

        drop(dictation);
        assert_eq!(counter.open.load(Ordering::SeqCst), 1, "a reunião segura");
        drop(meeting);
        assert_eq!(counter.open.load(Ordering::SeqCst), 0, "o último fecha");

        // Reabrir depois que todos saíram abre de novo.
        let _again = hub.open_with("mic", None, counter.opener()).unwrap();
        assert_eq!(counter.opened.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn different_devices_have_a_stream_each() {
        let hub = MicHub::new();
        let (a, b) = (Counter::new(), Counter::new());
        let _dictation = hub.open_with("headset", None, a.opener()).unwrap();
        let _meeting = hub.open_with("webcam", None, b.opener()).unwrap();
        assert_eq!(a.opened.load(Ordering::SeqCst), 1);
        assert_eq!(b.opened.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_failed_stream_is_not_reused() {
        let hub = MicHub::new();
        let counter = Counter::new();
        let meeting = hub.open_with("mic", None, counter.opener()).unwrap();
        counter.failed.store(true, Ordering::Relaxed);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !meeting.failed() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(2));
        }
        counter.failed.store(false, Ordering::Relaxed);
        let _dictation = hub.open_with("mic", None, counter.opener()).unwrap();
        assert_eq!(counter.opened.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn an_open_error_reaches_the_caller() {
        let hub = MicHub::new();
        let err = hub
            .open_with("mic", None, || {
                Err(AudioError::Device("sem mic".to_owned()))
            })
            .err()
            .unwrap();
        assert!(err.to_string().contains("sem mic"), "{err}");
    }

    #[test]
    fn an_undrained_tap_drops_and_counts_the_excess() {
        let queue = TapQueue::new(None);
        queue.push(&[0.1; 8], 10);
        queue.push(&[0.2; 8], 10);
        assert_eq!(queue.lock().len(), 10);
        assert_eq!(queue.dropped.load(Ordering::Relaxed), 6);
    }
}
