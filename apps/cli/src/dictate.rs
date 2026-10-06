//! `fala-cli dictate`: mic (ou WAV) → `DictationCapture` → Parakeet → uma linha no stdout.
//!
//! No modo mic, cada linha do stdin alterna começar/terminar um ditado (simula o push-to-talk) e
//! o EOF encerra. Com `--wav`, o arquivo entra no ritmo do relógio e o fim dele é o soltar.
//! As utterances que o VAD fecha durante a gravação vão para a thread de ASR na hora, de modo que
//! ao soltar só falte a última (design doc §3.3). O stdout é só o texto; com `FALA_TRACE=1`, o
//! stderr ganha o trace das doors 3 e 4 do plano `pipeline-headless`, nunca com texto ditado.

use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context};
use clap::Args;
use fala_asr::{AsrError, Parakeet, Transcriber};
use fala_audio::{AudioError, DictationCapture, Mic, SileroVad};
use fala_core::{DictationAudio, Language};

const DEFAULT_VAD: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../apps/desktop/resources/models/silero_vad_v4.onnx"
);
/// O passo do laço: um bloco de 10 ms, como um callback do `cpal`.
const TICK: Duration = Duration::from_millis(10);

#[derive(Args)]
pub struct DictateArgs {
    /// Pasta `parakeet-tdt-0.6b-v3-int8`.
    #[arg(long)]
    model: PathBuf,
    /// Modelo Silero VAD v4 (padrão: o versionado no repo).
    #[arg(long, default_value = DEFAULT_VAD)]
    vad: PathBuf,
    /// Parte do nome do microfone (padrão: a entrada padrão).
    #[arg(long, conflicts_with = "wav")]
    mic: Option<String>,
    /// Toca este WAV pelo pipeline no lugar do microfone.
    #[arg(long)]
    wav: Option<PathBuf>,
    /// Idioma do ditado: pt-BR ou en.
    #[arg(long, default_value = "pt-BR")]
    language: String,
}

/// Falha com o código de saída do contrato: 1 em tempo de execução, 2 entrada inválida.
pub struct Failure {
    pub code: u8,
    pub error: anyhow::Error,
}

fn input(error: anyhow::Error) -> Failure {
    Failure { code: 2, error }
}

fn failed(error: anyhow::Error) -> Failure {
    Failure { code: 1, error }
}

impl From<anyhow::Error> for Failure {
    fn from(error: anyhow::Error) -> Self {
        failed(error)
    }
}

pub fn run(args: DictateArgs) -> Result<(), Failure> {
    let trace = Trace::from_env();
    let language: Language = args
        .language
        .parse()
        .map_err(|e| input(anyhow!("--language: {e}")))?;
    let wav = match &args.wav {
        Some(path) => Some(read_wav(path).map_err(input)?),
        None => None,
    };

    let started = Instant::now();
    let vad = SileroVad::load(&args.vad).map_err(|e| input(e.into()))?;
    let vad_ms = started.elapsed();
    if wav.is_none()
        && let Some(needle) = &args.mic
    {
        Mic::check(needle).map_err(|e| input(e.into()))?;
    }
    let started = Instant::now();
    let model = Parakeet::load(&args.model).map_err(|e| input(e.into()))?;
    trace.emit(format_args!(
        "event=load model_ms={} vad_ms={}",
        started.elapsed().as_millis(),
        vad_ms.as_millis()
    ));

    let asr = AsrWorker::spawn(Box::new(model), language);
    let mut session = Session {
        asr,
        trace,
        dictation: 0,
        utterances: 0,
        audio_samples: 0,
    };
    match wav {
        Some((rate, samples)) => {
            let capture = DictationCapture::new(rate, Box::new(vad)).map_err(audio_failure)?;
            run_wav(&mut session, capture, rate, &samples)
        }
        None => {
            let mic = Mic::open(args.mic.as_deref()).map_err(audio_failure)?;
            let capture =
                DictationCapture::new(mic.sample_rate(), Box::new(vad)).map_err(audio_failure)?;
            run_mic(&mut session, capture, mic)
        }
    }
}

fn audio_failure(error: AudioError) -> Failure {
    match error {
        AudioError::NoDevice { .. } => input(error.into()),
        _ => failed(error.into()),
    }
}

fn run_wav(
    session: &mut Session,
    mut capture: DictationCapture,
    rate: u32,
    samples: &[f32],
) -> Result<(), Failure> {
    let block = (rate as usize / 100).max(1);
    session.begin(&mut capture)?;
    let start = Instant::now();
    for (i, piece) in samples.chunks(block).enumerate() {
        let due = start + TICK * u32::try_from(i).unwrap_or(u32::MAX);
        if let Some(wait) = due.checked_duration_since(Instant::now()) {
            thread::sleep(wait);
        }
        session.feed(&mut capture, piece)?;
    }
    // O último bloco também dura 10 ms antes de o arquivo "soltar a tecla".
    let end = start + TICK * u32::try_from(samples.len().div_ceil(block)).unwrap_or(u32::MAX);
    if let Some(wait) = end.checked_duration_since(Instant::now()) {
        thread::sleep(wait);
    }
    session.release(&mut capture)
}

fn run_mic(
    session: &mut Session,
    mut capture: DictationCapture,
    mut mic: Mic,
) -> Result<(), Failure> {
    log::info!("Enter começa um ditado, Enter termina; Ctrl+D sai");
    let (tx, events) = mpsc::channel();
    thread::spawn(move || read_events(std::io::stdin().lock(), &tx));
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        mic.drain_into(&mut buffer);
        session.feed(&mut capture, &buffer)?;
        let event = match events.recv_timeout(TICK) {
            Ok(event) => event,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => StdinEvent::Eof,
        };
        // O que chegou do mic até o Enter entra antes de começar ou terminar.
        buffer.clear();
        mic.drain_into(&mut buffer);
        session.feed(&mut capture, &buffer)?;
        match next_action(capture.is_recording(), event) {
            Action::Start => {
                log::info!("gravando…");
                session.begin(&mut capture)?;
            }
            Action::Stop => session.release(&mut capture)?,
            Action::StopAndExit => {
                session.release(&mut capture)?;
                break;
            }
            Action::Exit => break,
        }
    }
    if mic.dropped() > 0 {
        log::warn!(
            "o ring do microfone transbordou: {} amostras perdidas",
            mic.dropped()
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StdinEvent {
    Line,
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Start,
    Stop,
    StopAndExit,
    Exit,
}

/// Cada linha do stdin vira um `Line`; o fim (ou um erro de leitura) vira um `Eof` e encerra.
fn read_events(mut reader: impl BufRead, tx: &Sender<StdinEvent>) {
    let mut line = String::new();
    loop {
        line.clear();
        let event = match reader.read_line(&mut line) {
            Ok(0) | Err(_) => StdinEvent::Eof,
            Ok(_) => StdinEvent::Line,
        };
        if tx.send(event).is_err() || event == StdinEvent::Eof {
            return;
        }
    }
}

fn next_action(recording: bool, event: StdinEvent) -> Action {
    match (recording, event) {
        (false, StdinEvent::Line) => Action::Start,
        (true, StdinEvent::Line) => Action::Stop,
        (true, StdinEvent::Eof) => Action::StopAndExit,
        (false, StdinEvent::Eof) => Action::Exit,
    }
}

/// Um ditado em curso: numera as utterances, manda cada uma ao ASR e fecha a linha no soltar.
struct Session {
    asr: AsrWorker,
    trace: Trace,
    dictation: usize,
    utterances: usize,
    audio_samples: usize,
}

impl Session {
    fn begin(&mut self, capture: &mut DictationCapture) -> Result<(), Failure> {
        self.dictation += 1;
        self.utterances = 0;
        self.audio_samples = 0;
        let ready = capture.start().map_err(audio_failure)?;
        self.dispatch(ready, true);
        Ok(())
    }

    fn feed(&mut self, capture: &mut DictationCapture, samples: &[f32]) -> Result<(), Failure> {
        let ready = capture.feed(samples).map_err(audio_failure)?;
        self.dispatch(ready, true);
        Ok(())
    }

    fn dispatch(&mut self, ready: Vec<DictationAudio>, before_release: bool) {
        for audio in ready {
            self.audio_samples += audio.samples().len();
            self.asr.send(Job {
                dictation: self.dictation,
                index: self.utterances,
                audio,
                before_release,
            });
            self.utterances += 1;
        }
    }

    /// O soltar: esvazia a captura, espera a fila de ASR e imprime a linha.
    fn release(&mut self, capture: &mut DictationCapture) -> Result<(), Failure> {
        let released = Instant::now();
        let tail = capture.stop().map_err(audio_failure)?;
        let flushed = Instant::now();
        self.dispatch(tail, false);
        let mut texts = Texts::default();
        for _ in 0..self.utterances {
            let done = self.asr.recv()?;
            self.trace.emit(format_args!(
                "event=utterance dictation={} index={} audio_ms={} asr_ms={} before_release={}",
                done.dictation,
                done.index,
                done.audio_ms,
                done.asr_ms,
                u8::from(done.before_release)
            ));
            texts.push(done.index, done.text);
        }
        let line = texts.join().map_err(|e| failed(e.into()))?;
        let mut stdout = std::io::stdout().lock();
        writeln!(stdout, "{line}")
            .and_then(|()| stdout.flush())
            .context("stdout")?;
        let now = Instant::now();
        self.trace.emit(format_args!(
            "event=dictation dictation={} audio_ms={} utterances={} flush_ms={} tail_asr_ms={} release_to_text_ms={}",
            self.dictation,
            samples_ms(self.audio_samples),
            self.utterances,
            (flushed - released).as_millis(),
            (now - flushed).as_millis(),
            (now - released).as_millis()
        ));
        Ok(())
    }
}

fn samples_ms(samples: usize) -> u128 {
    samples as u128 * 1000 / u128::from(DictationAudio::SAMPLE_RATE_HZ)
}

/// Os textos de um ditado, juntados na ordem das utterances seja qual for a ordem de chegada.
#[derive(Default)]
struct Texts {
    parts: BTreeMap<usize, Result<String, AsrError>>,
}

impl Texts {
    fn push(&mut self, index: usize, text: Result<String, AsrError>) {
        self.parts.insert(index, text);
    }

    fn join(self) -> Result<String, AsrError> {
        let mut line = String::new();
        for text in self.parts.into_values() {
            let text = text?;
            let text = text.trim();
            if text.is_empty() {
                continue;
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(text);
        }
        Ok(line)
    }
}

struct Job {
    dictation: usize,
    index: usize,
    audio: DictationAudio,
    before_release: bool,
}

struct Done {
    dictation: usize,
    index: usize,
    text: Result<String, AsrError>,
    audio_ms: u128,
    asr_ms: u128,
    before_release: bool,
}

/// A thread de ASR: transcreve as utterances na ordem em que chegam, enquanto a gravação segue.
struct AsrWorker {
    jobs: Sender<Job>,
    done: Receiver<Done>,
}

impl AsrWorker {
    fn spawn(mut model: Box<dyn Transcriber>, language: Language) -> Self {
        let (jobs, job_rx) = mpsc::channel::<Job>();
        let (done_tx, done) = mpsc::channel();
        thread::spawn(move || {
            for job in job_rx {
                let started = Instant::now();
                let text = model.transcribe(&job.audio, language).map(|t| t.text);
                let finished = Done {
                    dictation: job.dictation,
                    index: job.index,
                    text,
                    audio_ms: samples_ms(job.audio.samples().len()),
                    asr_ms: started.elapsed().as_millis(),
                    before_release: job.before_release,
                };
                if done_tx.send(finished).is_err() {
                    return;
                }
            }
        });
        Self { jobs, done }
    }

    fn send(&self, job: Job) {
        // Se a thread morreu, o `recv` do soltar devolve o erro.
        let _ = self.jobs.send(job);
    }

    fn recv(&self) -> Result<Done, Failure> {
        self.done
            .recv()
            .map_err(|_| failed(anyhow!("a thread de ASR terminou sem responder")))
    }
}

/// O trace das doors 3 e 4: só com `FALA_TRACE=1`, pelo `log` com target `fala_trace`.
#[derive(Clone, Copy)]
struct Trace {
    on: bool,
}

impl Trace {
    fn from_env() -> Self {
        Self {
            on: std::env::var("FALA_TRACE").is_ok_and(|v| v == "1"),
        }
    }

    fn emit(self, message: std::fmt::Arguments<'_>) {
        if self.on {
            log::info!(target: "fala_trace", "{message}");
        }
    }
}

/// Lê o WAV inteiro como mono f32 na taxa do arquivo.
fn read_wav(path: &Path) -> anyhow::Result<(u32, Vec<f32>)> {
    let mut reader = hound::WavReader::open(path)
        .with_context(|| format!("não consegui ler o WAV {}", path.display()))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels).max(1);
    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>(),
        hound::SampleFormat::Int => {
            let scale = 2f32.powi(i32::from(spec.bits_per_sample) - 1);
            reader
                .samples::<i32>()
                .map(|s| s.map(|s| s as f32 / scale))
                .collect::<Result<_, _>>()
        }
    }
    .with_context(|| format!("não consegui ler as amostras de {}", path.display()))?;
    let mono = interleaved
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect();
    Ok((spec.sample_rate, mono))
}

#[cfg(test)]
mod tests {
    use fala_audio::VoiceDetector;
    use fala_core::Transcript;

    use super::*;

    struct FailingAsr;

    impl Transcriber for FailingAsr {
        fn transcribe(&mut self, _: &DictationAudio, _: Language) -> Result<Transcript, AsrError> {
            Err(AsrError::Inference("stub".to_owned()))
        }
    }

    struct AlwaysVoice;

    impl VoiceDetector for AlwaysVoice {
        fn is_voice(&mut self, _: &[f32]) -> Result<bool, AudioError> {
            Ok(true)
        }
        fn reset(&mut self) {}
    }

    #[test]
    fn inference_error_exits_1() {
        let mut session = Session {
            asr: AsrWorker::spawn(Box::new(FailingAsr), Language::PtBr),
            trace: Trace { on: false },
            dictation: 0,
            utterances: 0,
            audio_samples: 0,
        };
        let mut capture = DictationCapture::new(16_000, Box::new(AlwaysVoice)).unwrap();
        assert!(session.begin(&mut capture).is_ok());
        assert!(session.feed(&mut capture, &vec![0.1; 8_000]).is_ok());
        let failure = session.release(&mut capture).expect_err("devia falhar");
        assert_eq!(failure.code, 1);
        assert!(format!("{:#}", failure.error).contains("stub"));
    }

    #[test]
    fn texts_join_in_utterance_order() {
        let mut texts = Texts::default();
        texts.push(2, Ok("  terceira.".to_owned()));
        texts.push(0, Ok("Primeira ".to_owned()));
        texts.push(3, Ok(String::new()));
        texts.push(1, Ok("segunda,".to_owned()));
        assert_eq!(texts.join().unwrap(), "Primeira segunda, terceira.");

        assert_eq!(Texts::default().join().unwrap(), "");

        let mut failing = Texts::default();
        failing.push(1, Ok("ok".to_owned()));
        failing.push(0, Err(AsrError::Inference("stub".to_owned())));
        assert!(matches!(failing.join(), Err(AsrError::Inference(_))));
    }

    #[test]
    fn stdin_lines_toggle_and_eof_ends() {
        let (tx, rx) = mpsc::channel();
        read_events(std::io::Cursor::new("\nqualquer coisa\n\n"), &tx);
        let events: Vec<StdinEvent> = rx.try_iter().collect();
        assert_eq!(
            events,
            [
                StdinEvent::Line,
                StdinEvent::Line,
                StdinEvent::Line,
                StdinEvent::Eof
            ]
        );
        let mut recording = false;
        let mut actions = Vec::new();
        for event in events {
            let action = next_action(recording, event);
            recording = matches!(action, Action::Start);
            actions.push(action);
        }
        assert_eq!(
            actions,
            [
                Action::Start,
                Action::Stop,
                Action::Start,
                Action::StopAndExit
            ]
        );
        assert_eq!(next_action(false, StdinEvent::Eof), Action::Exit);
    }
}
