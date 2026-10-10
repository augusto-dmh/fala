//! Captura do ditado pelo `fala-audio`: `Mic` abre a entrada dos settings, `DictationCapture`
//! reamostra para 16 kHz, guarda os últimos 300 ms enquanto o stream está aberto e segmenta com o
//! Silero v4 (ADR-0009). O `AudioRecordingManager` fala com o `DictationRecorder`; o `Processor`
//! não abre mic e é o que os testes exercitam.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait};
use fala_audio::{AudioError, DictationCapture, Mic, VoiceDetector};
use fala_core::DictationAudio;

use crate::audio_toolkit::audio::AudioVisualiser;

/// Intervalo entre duas drenagens do mic.
const TICK: Duration = Duration::from_millis(10);
const LEVEL_BUCKETS: usize = 16;

/// Como os quadros de uma sessão passam pelo VAD.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VadPolicy {
    /// Todo quadro conta como voz.
    Disabled,
    /// Utterances do `DictationCapture`.
    Offline,
    /// Igual ao `Offline`: o `DictationCapture` tem um hangover só (door 3 do plano).
    Streaming,
}

pub type AudioFrameCallback = Arc<dyn Fn(&[f32]) + Send + Sync + 'static>;
pub type LevelCallback = Arc<dyn Fn(Vec<f32>) + Send + Sync + 'static>;
type SharedDetector = Arc<Mutex<Box<dyn VoiceDetector>>>;

/// Canais da configuração que o `Mic` abre neste dispositivo.
pub fn input_channel_count(device: &cpal::Device) -> Result<u16, String> {
    device
        .default_input_config()
        .map(|config| config.channels())
        .map_err(|e| e.to_string())
}

/// O detector do recorder visto pelo `DictationCapture` de cada abertura: carregado uma vez,
/// ignorado na política `Disabled`, e um erro num quadro conta como voz, como no herdado.
struct DictationVad {
    inner: SharedDetector,
    bypass: Arc<AtomicBool>,
    warned: bool,
}

impl VoiceDetector for DictationVad {
    fn is_voice(&mut self, frame: &[f32]) -> Result<bool, AudioError> {
        if self.bypass.load(Ordering::Relaxed) {
            return Ok(true);
        }
        let result = match self.inner.lock() {
            Ok(mut vad) => vad.is_voice(frame),
            Err(_) => Err(AudioError::Vad("trava do VAD envenenada".to_owned())),
        };
        Ok(result.unwrap_or_else(|e| {
            if !self.warned {
                self.warned = true;
                log::warn!("dictation capture: VAD failed on a frame, kept as voice: {e}");
            }
            true
        }))
    }

    fn reset(&mut self) {
        self.warned = false;
        if let Ok(mut vad) = self.inner.lock() {
            vad.reset();
        }
    }
}

/// Uma abertura do stream: amostras na taxa do mic entram, a gravação a 16 kHz sai no `stop`.
struct Processor {
    capture: DictationCapture,
    bypass: Arc<AtomicBool>,
    visualizer: AudioVisualiser,
    level_cb: Option<LevelCallback>,
    audio_cb: Option<AudioFrameCallback>,
    recording: bool,
    samples: Vec<f32>,
    ready: Option<(mpsc::Sender<()>, Instant)>,
}

impl Processor {
    fn new(
        rate: u32,
        detector: SharedDetector,
        level_cb: Option<LevelCallback>,
        audio_cb: Option<AudioFrameCallback>,
    ) -> Result<Self, AudioError> {
        let bypass = Arc::new(AtomicBool::new(false));
        let vad = DictationVad {
            inner: detector,
            bypass: Arc::clone(&bypass),
            warned: false,
        };
        // A janela das barras da pill, como no herdado: a potência de 2 mais perto de 1/30 s.
        let target = rate as usize / 30;
        let window = [256usize, 512, 1024, 2048]
            .into_iter()
            .min_by_key(|w| w.abs_diff(target))
            .unwrap_or(1024);
        Ok(Self {
            capture: DictationCapture::new(rate, Box::new(vad))?,
            bypass,
            visualizer: AudioVisualiser::new(rate, window, LEVEL_BUCKETS, 400.0, 4000.0),
            level_cb,
            audio_cb,
            recording: false,
            samples: Vec::new(),
            ready: None,
        })
    }

    fn feed(&mut self, raw: &[f32]) {
        if raw.is_empty() {
            return;
        }
        if self.recording {
            if let (Some(levels), Some(cb)) = (self.visualizer.feed(raw), &self.level_cb) {
                cb(levels);
            }
            if let Some((ready, started)) = self.ready.take() {
                log::debug!(
                    "dictation capture: first samples {:?} after Start",
                    started.elapsed()
                );
                let _ = ready.send(());
            }
        }
        let done = self.capture.feed(raw);
        self.keep(done);
    }

    fn start(&mut self, policy: VadPolicy, ready: mpsc::Sender<()>) {
        self.bypass
            .store(policy == VadPolicy::Disabled, Ordering::Relaxed);
        self.visualizer.reset();
        self.samples.clear();
        self.recording = true;
        self.ready = Some((ready, Instant::now()));
        let done = self.capture.start();
        self.keep(done);
    }

    fn stop(&mut self) -> Vec<f32> {
        self.recording = false;
        self.ready = None;
        let done = self.capture.stop();
        self.keep(done);
        std::mem::take(&mut self.samples)
    }

    fn keep(&mut self, done: Result<Vec<DictationAudio>, AudioError>) {
        match done {
            Ok(utterances) => {
                for utterance in utterances {
                    if let Some(cb) = &self.audio_cb {
                        cb(utterance.samples());
                    }
                    self.samples.extend_from_slice(utterance.samples());
                }
            }
            Err(e) => log::warn!("dictation capture: {e}"),
        }
    }
}

enum Cmd {
    Start(VadPolicy, mpsc::Sender<()>),
    Stop(mpsc::Sender<Vec<f32>>),
    Shutdown,
}

/// O microfone do ditado. `open` sobe um worker que dona do `Mic` (o stream do `cpal` fica na
/// thread que o criou); `start` e `stop` são comandos para ele.
pub struct DictationRecorder {
    detector: SharedDetector,
    level_cb: Option<LevelCallback>,
    audio_cb: Option<AudioFrameCallback>,
    channel: Option<usize>,
    cmd_tx: Option<mpsc::Sender<Cmd>>,
    worker: Option<JoinHandle<()>>,
    failed: Arc<AtomicBool>,
}

impl DictationRecorder {
    /// O detector é carregado uma vez aqui e reaproveitado em cada abertura do stream.
    pub fn new(detector: Box<dyn VoiceDetector>) -> Self {
        Self {
            detector: Arc::new(Mutex::new(detector)),
            level_cb: None,
            audio_cb: None,
            channel: None,
            cmd_tx: None,
            worker: None,
            failed: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_level_callback(mut self, cb: impl Fn(Vec<f32>) + Send + Sync + 'static) -> Self {
        self.level_cb = Some(Arc::new(cb));
        self
    }

    /// Recebe cada utterance que o VAD fecha, na ordem, na thread do worker.
    pub fn with_audio_callback(mut self, cb: impl Fn(&[f32]) + Send + Sync + 'static) -> Self {
        self.audio_cb = Some(Arc::new(cb));
        self
    }

    pub fn with_selected_channel(mut self, channel: Option<u16>) -> Self {
        self.set_selected_channel(channel);
        self
    }

    /// Vale na próxima abertura.
    pub fn set_selected_channel(&mut self, channel: Option<u16>) {
        self.channel = channel.map(usize::from);
    }

    /// Abre `device`, ou a entrada padrão. Já aberto e vivo, não faz nada.
    pub fn open(&mut self, device: Option<cpal::Device>) -> Result<(), String> {
        if self.worker.is_some() {
            if !self.needs_reopen() {
                return Ok(());
            }
            log::warn!("dictation capture: stream failed; rebuilding it");
            self.close();
        }
        let device = match device {
            Some(device) => device,
            None => crate::audio_toolkit::get_cpal_host()
                .default_input_device()
                .ok_or_else(|| "No input device found".to_owned())?,
        };
        self.failed.store(false, Ordering::Relaxed);

        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (init_tx, init_rx) = mpsc::sync_channel(1);
        let detector = Arc::clone(&self.detector);
        let level_cb = self.level_cb.clone();
        let audio_cb = self.audio_cb.clone();
        let channel = self.channel;
        let failed = Arc::clone(&self.failed);
        let worker = std::thread::spawn(move || {
            let started = Instant::now();
            let mic = match Mic::open_device(device, channel) {
                Ok(mic) => mic,
                Err(e) => {
                    let _ = init_tx.send(Err(e.to_string()));
                    return;
                }
            };
            let opened = started.elapsed();
            // The mic is already capturing into its 2 s ring while the resampler and the
            // level FFT are built (~1-2 ms): that audio lands in the pre-buffer.
            match Processor::new(mic.sample_rate(), detector, level_cb, audio_cb) {
                Ok(processor) => {
                    let _ = init_tx.send(Ok(()));
                    log::debug!(
                        "dictation capture: mic open {opened:?}, processor {:?}",
                        started.elapsed() - opened
                    );
                    run_worker(mic, processor, &cmd_rx, &failed);
                }
                Err(e) => {
                    let _ = init_tx.send(Err(e.to_string()));
                }
            }
        });

        match init_rx.recv() {
            Ok(Ok(())) => {
                self.cmd_tx = Some(cmd_tx);
                self.worker = Some(worker);
                Ok(())
            }
            Ok(Err(e)) => {
                let _ = worker.join();
                Err(e)
            }
            Err(e) => {
                let _ = worker.join();
                Err(format!("Failed to initialize microphone worker: {e}"))
            }
        }
    }

    /// Começa uma sessão; o receptor resolve quando o mic entrega o primeiro bloco depois dela.
    pub fn start(&self, policy: VadPolicy) -> Result<mpsc::Receiver<()>, String> {
        let (ready_tx, ready_rx) = mpsc::channel();
        self.send(Cmd::Start(policy, ready_tx))?;
        Ok(ready_rx)
    }

    /// Termina a sessão e devolve a gravação a 16 kHz mono.
    pub fn stop(&self) -> Result<Vec<f32>, String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send(Cmd::Stop(reply_tx))?;
        reply_rx.recv().map_err(|e| e.to_string())
    }

    /// O stream precisa ser reaberto: o `cpal` reportou erro ou o worker terminou.
    pub fn needs_reopen(&self) -> bool {
        self.failed.load(Ordering::Relaxed) || self.worker.as_ref().is_some_and(|h| h.is_finished())
    }

    pub fn close(&mut self) {
        if let Some(tx) = self.cmd_tx.take() {
            let _ = tx.send(Cmd::Shutdown);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }

    fn send(&self, cmd: Cmd) -> Result<(), String> {
        self.cmd_tx
            .as_ref()
            .ok_or_else(|| "Recorder is not open".to_owned())?
            .send(cmd)
            .map_err(|e| e.to_string())
    }
}

fn run_worker(
    mut mic: Mic,
    mut processor: Processor,
    commands: &mpsc::Receiver<Cmd>,
    failed: &AtomicBool,
) {
    let mut buffer = Vec::new();
    let mut dropped = mic.dropped();
    loop {
        let command = commands.recv_timeout(TICK);
        // O que o mic entregou até o comando entra antes dele.
        buffer.clear();
        mic.drain_into(&mut buffer);
        processor.feed(&buffer);
        if mic.failed() {
            failed.store(true, Ordering::Relaxed);
        }
        let now_dropped = mic.dropped();
        if now_dropped > dropped && processor.recording {
            log::warn!(
                "dictation capture: microphone ring dropped {} samples",
                now_dropped - dropped
            );
        }
        dropped = now_dropped;
        match command {
            Ok(Cmd::Start(policy, ready)) => processor.start(policy, ready),
            Ok(Cmd::Stop(reply)) => {
                let _ = reply.send(processor.stop());
            }
            Ok(Cmd::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => return,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fala_audio::FRAME_SAMPLES;

    /// Voz quando o primeiro valor do quadro é ≥ 1000.
    struct CodeVad;

    impl VoiceDetector for CodeVad {
        fn is_voice(&mut self, frame: &[f32]) -> Result<bool, AudioError> {
            Ok(frame[0] >= 1000.0)
        }
        fn reset(&mut self) {}
    }

    struct Always(bool);

    impl VoiceDetector for Always {
        fn is_voice(&mut self, _: &[f32]) -> Result<bool, AudioError> {
            Ok(self.0)
        }
        fn reset(&mut self) {}
    }

    struct Broken;

    impl VoiceDetector for Broken {
        fn is_voice(&mut self, _: &[f32]) -> Result<bool, AudioError> {
            Err(AudioError::Vad("quebrado".to_owned()))
        }
        fn reset(&mut self) {}
    }

    fn processor(vad: impl VoiceDetector + 'static) -> Processor {
        Processor::new(16_000, Arc::new(Mutex::new(Box::new(vad))), None, None).unwrap()
    }

    fn ramp(range: std::ops::Range<usize>) -> Vec<f32> {
        range.map(|i| i as f32).collect()
    }

    fn feed_in_blocks(p: &mut Processor, samples: &[f32]) {
        for block in samples.chunks(160) {
            p.feed(block);
        }
    }

    fn frames(codes: &[f32]) -> Vec<f32> {
        codes
            .iter()
            .flat_map(|c| std::iter::repeat_n(*c, FRAME_SAMPLES))
            .collect()
    }

    #[test]
    fn prebuffer_leads_the_recording() {
        let mut p = processor(Always(true));
        feed_in_blocks(&mut p, &ramp(0..8_000));
        p.start(VadPolicy::Offline, mpsc::channel().0);
        feed_in_blocks(&mut p, &ramp(8_000..11_200));
        assert_eq!(p.stop(), ramp(3_200..11_200));
    }

    #[test]
    fn disabled_policy_keeps_everything() {
        let input = vec![0.01; 4_800];
        let mut p = processor(Always(false));
        feed_in_blocks(&mut p, &input[..1_600]);
        p.start(VadPolicy::Disabled, mpsc::channel().0);
        feed_in_blocks(&mut p, &input[1_600..]);
        assert_eq!(p.stop(), input);

        let mut p = processor(Always(false));
        feed_in_blocks(&mut p, &input[..1_600]);
        p.start(VadPolicy::Offline, mpsc::channel().0);
        feed_in_blocks(&mut p, &input[1_600..]);
        assert!(p.stop().is_empty());
    }

    #[test]
    fn utterances_reach_router_and_recording() {
        let routed = Arc::new(Mutex::new(Vec::<Vec<f32>>::new()));
        let sink = Arc::clone(&routed);
        let mut p = Processor::new(
            16_000,
            Arc::new(Mutex::new(Box::new(CodeVad))),
            None,
            Some(Arc::new(move |u: &[f32]| {
                sink.lock().unwrap().push(u.to_vec())
            })),
        )
        .unwrap();
        p.start(VadPolicy::Offline, mpsc::channel().0);
        let mut codes: Vec<f32> = (0..10).map(|i| 1_000.0 + i as f32).collect();
        codes.extend(ramp(0..40));
        codes.extend((10..20).map(|i| 1_000.0 + i as f32));
        feed_in_blocks(&mut p, &frames(&codes));
        let recording = p.stop();

        let routed = routed.lock().unwrap();
        let lens: Vec<usize> = routed.iter().map(Vec::len).collect();
        assert_eq!(lens, [25 * FRAME_SAMPLES, 25 * FRAME_SAMPLES]);
        assert_eq!(recording, routed.concat());
        assert!(recording.len() < codes.len() * FRAME_SAMPLES);
    }

    #[test]
    fn detector_error_counts_as_voice() {
        let mut p = processor(Broken);
        p.start(VadPolicy::Offline, mpsc::channel().0);
        feed_in_blocks(&mut p, &vec![0.01; 3_200]);
        assert_eq!(p.stop().len(), 3_200);
    }

    #[test]
    fn readiness_fires_on_first_block_after_start() {
        let mut p = processor(Always(true));
        let (tx, rx) = mpsc::channel();
        p.start(VadPolicy::Offline, tx);
        assert!(rx.try_recv().is_err());
        p.feed(&[0.0; 160]);
        assert!(rx.try_recv().is_ok());
        p.stop();

        let (tx, rx) = mpsc::channel();
        p.start(VadPolicy::Offline, tx);
        p.stop();
        assert!(rx.recv().is_err());
    }

    #[test]
    fn levels_only_while_recording() {
        let count = Arc::new(Mutex::new(0usize));
        let seen = Arc::clone(&count);
        let mut p = Processor::new(
            16_000,
            Arc::new(Mutex::new(Box::new(Always(false)))),
            Some(Arc::new(move |_: Vec<f32>| *seen.lock().unwrap() += 1)),
            None,
        )
        .unwrap();
        feed_in_blocks(&mut p, &vec![0.1; 16_000]);
        assert_eq!(*count.lock().unwrap(), 0);
        p.start(VadPolicy::Offline, mpsc::channel().0);
        feed_in_blocks(&mut p, &vec![0.1; 16_000]);
        assert!(*count.lock().unwrap() > 0);
    }
}
