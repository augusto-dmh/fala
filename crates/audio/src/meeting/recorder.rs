//! O relógio manda: cada canal escreve até `relógio − 700 ms`, com zeros no que não chegou.
//!
//! O spike (`fala-cli record`) pareava os frames por ordem de chegada, e um canal sem frames
//! travava o outro; no Windows o loopback não entrega nada enquanto nenhum app toca áudio. Aqui o
//! tempo de parede decide quantos pares existem, e um canal mudo vira silêncio (door 2 do plano
//! `meeting-recorder`).
//!
//! O canal do mic passa antes por um hold de 450 ms em memória, contado em amostras da fonte: um
//! ditado durante a reunião (ADR-0015) abre um intervalo com `begin_omit`, que pode começar no
//! passado (o pré-buffer do ditado), e toda amostra do intervalo sai do hold já zerada. O que nunca
//! saiu do hold nunca foi escrito, então nem um crash põe voz de ditado no WAV.

use std::collections::VecDeque;
use std::time::Duration;

use super::wav::{MeetingWav, FLUSH_EVERY, MEETING_RATE};
use crate::resample::Resampler;
use crate::AudioError;

/// O canal do mic fica este tempo em memória antes do resample: o pré-buffer de 300 ms do ditado
/// mais a margem do jitter (ADR-0015).
pub const MIC_HOLD: Duration = Duration::from_millis(450);
/// 700 ms de folga antes de preencher: o hold do mic mais os 250 ms do jitter do callback e do
/// período do ring.
pub const LAG_FRAMES: u64 = 33_600;
/// Um canal com mais de 1 s à frente do escrito perde o excesso mais antigo.
pub const MAX_BACKLOG_FRAMES: usize = 48_000;

/// O que aconteceu com um canal, em frames a 48 kHz.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ChannelStats {
    /// Frames entregues pela fonte (depois do resample).
    pub delivered: u64,
    /// Zeros escritos porque a fonte não entregou a tempo.
    pub filled: u64,
    /// Frames descartados por excesso de atraso acumulado.
    pub dropped: u64,
    /// Amostras da fonte que um intervalo omitido pedia mas que já tinham saído do hold.
    pub omit_late: u64,
}

/// Um trecho do mic zerado por um ditado, em frames do WAV (48 kHz, `[start, end)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OmittedSpan {
    pub start: u64,
    pub end: u64,
}

/// Um intervalo do ditado em amostras da fonte do mic; `end` aberto enquanto o ditado dura. Os
/// frames do WAV são anotados quando a primeira e a última amostra saem do hold.
#[derive(Debug, Clone, Copy)]
struct Omit {
    start: u64,
    end: Option<u64>,
    wav_start: Option<u64>,
}

/// As amostras do mic que ainda podem virar ditado. Índices contam as amostras da fonte desde o
/// começo da gravação.
struct Hold {
    samples: VecDeque<f32>,
    /// Índice da primeira amostra de `samples`.
    first: u64,
    capacity: usize,
    rate: u32,
    /// Uma amostra da fonte e o frame de saída do mic (48 kHz) que ela vira: o ponto de partida
    /// da conta, que muda quando o mic passa a outra taxa.
    origin: (u64, u64),
    omits: Vec<Omit>,
    closed: Vec<OmittedSpan>,
    batch: Vec<f32>,
}

impl Hold {
    fn new(rate: u32) -> Self {
        Self {
            samples: VecDeque::new(),
            first: 0,
            capacity: capacity(rate),
            rate,
            origin: (0, 0),
            omits: Vec::new(),
            closed: Vec::new(),
            batch: Vec::new(),
        }
    }

    /// O mic passa a vir em `rate`, com o hold já vazio e o resampler velho esvaziado em
    /// `output` frames: um intervalo ainda aberto segue zerando a fonte nova.
    fn switch_rate(&mut self, rate: u32, output: u64) {
        let old = u128::from(self.rate.max(1));
        let first = self.first;
        for omit in &mut self.omits {
            if let Some(end) = omit.end.filter(|e| *e > first) {
                omit.end = Some(first + (u128::from(end - first) * u128::from(rate) / old) as u64);
            }
        }
        self.rate = rate;
        self.capacity = capacity(rate);
        self.origin = (first, output);
    }

    /// Índice da próxima amostra a entrar.
    fn next(&self) -> u64 {
        self.first + self.samples.len() as u64
    }

    fn frames(&self, duration: Duration) -> u64 {
        (u128::from(self.rate) * duration.as_nanos() / 1_000_000_000) as u64
    }

    /// Abre um intervalo `lookback` antes da próxima amostra; devolve quantas amostras pedidas já
    /// tinham saído. Um intervalo que encosta no anterior o reabre: os dois viram um.
    fn begin(&mut self, lookback: Duration) -> u64 {
        let wanted = self.next().saturating_sub(self.frames(lookback));
        let start = wanted.max(self.first);
        match self.omits.last_mut() {
            Some(last) if last.end.is_none_or(|end| end >= start) => last.end = None,
            _ => self.omits.push(Omit {
                start,
                end: None,
                wav_start: None,
            }),
        }
        start - wanted
    }

    /// Fecha o intervalo aberto `margin` depois da próxima amostra.
    fn end(&mut self, margin: Duration) {
        let end = self.next() + self.frames(margin);
        if let Some(last) = self.omits.last_mut().filter(|o| o.end.is_none()) {
            last.end = Some(end);
        }
    }

    /// Solta para `out` o que passou do hold (tudo, com `all`), zerado onde um intervalo cobre;
    /// com `close`, o fim da gravação fecha os intervalos abertos. `wav_of` diz em que frame do
    /// WAV cai uma amostra da fonte que sai agora.
    fn release(&mut self, all: bool, close: bool, wav_of: impl Fn(u64) -> u64) -> &[f32] {
        let keep = if all { 0 } else { self.capacity };
        let n = self.samples.len().saturating_sub(keep);
        self.batch.clear();
        self.batch.extend(self.samples.drain(..n));
        let (from, to) = (self.first, self.first + n as u64);
        self.first = to;
        let mut done = 0;
        for omit in &mut self.omits {
            let start = omit.start.max(from);
            let end = omit.end.unwrap_or(u64::MAX).min(to);
            if start < end {
                let range = (start - from) as usize..(end - from) as usize;
                self.batch[range].fill(0.0);
                omit.wav_start.get_or_insert_with(|| wav_of(start));
            }
            // Fechado e todo já solto (ou no fim da gravação): vira um trecho do WAV.
            let over = omit.end.is_some_and(|e| e <= to) || (close && omit.start < to);
            if over {
                done += 1;
                if let Some(wav_start) = omit.wav_start {
                    let end = omit.end.unwrap_or(to).min(to);
                    self.closed.push(OmittedSpan {
                        start: wav_start,
                        end: wav_of(end).max(wav_start),
                    });
                }
            }
        }
        self.omits.drain(..done);
        if close {
            self.omits.clear();
        }
        &self.batch
    }
}

/// Amostras do hold na taxa `rate`.
fn capacity(rate: u32) -> usize {
    (u128::from(rate) * MIC_HOLD.as_millis() / 1_000) as usize
}

struct Channel {
    resampler: Resampler,
    queue: VecDeque<f32>,
    scratch: Vec<f32>,
    stats: ChannelStats,
}

impl Channel {
    fn new(rate: u32) -> Result<Self, AudioError> {
        Ok(Self {
            resampler: Resampler::to(rate, MEETING_RATE)?,
            queue: VecDeque::new(),
            scratch: Vec::new(),
            stats: ChannelStats::default(),
        })
    }

    fn push(&mut self, samples: &[f32]) -> Result<(), AudioError> {
        self.scratch.clear();
        self.resampler.push(samples, &mut self.scratch)?;
        self.accept();
        Ok(())
    }

    fn drain_resampler(&mut self) -> Result<(), AudioError> {
        self.scratch.clear();
        self.resampler.finish(&mut self.scratch)?;
        self.accept();
        Ok(())
    }

    fn accept(&mut self) {
        self.stats.delivered += self.scratch.len() as u64;
        self.queue.extend(self.scratch.iter().copied());
    }

    /// `n` frames: os reais primeiro, zeros para o que faltar.
    fn take(&mut self, n: usize, out: &mut Vec<i16>) {
        out.clear();
        let real = n.min(self.queue.len());
        out.extend(self.queue.drain(..real).map(to_i16));
        out.resize(n, 0);
        self.stats.filled += (n - real) as u64;
    }

    fn trim(&mut self) {
        let excess = self.queue.len().saturating_sub(MAX_BACKLOG_FRAMES);
        self.queue.drain(..excess);
        self.stats.dropped += excess as u64;
    }
}

fn to_i16(x: f32) -> i16 {
    (x.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16
}

/// Recebe cada lote escrito no WAV: o índice do primeiro frame e os dois canais (mic, sistema).
pub type FrameTap = Box<dyn FnMut(u64, &[i16], &[i16]) + Send>;

fn frames_at(now: Duration) -> u64 {
    (now.as_nanos() * u128::from(MEETING_RATE) / 1_000_000_000) as u64
}

pub struct MeetingRecorder {
    hold: Hold,
    mic: Channel,
    system: Channel,
    wav: MeetingWav,
    written: u64,
    last_flush: Duration,
    left: Vec<i16>,
    right: Vec<i16>,
    tap: Option<FrameTap>,
}

impl MeetingRecorder {
    /// `mic_rate` e `system_rate` são as taxas em que `push_mic`/`push_system` vão entregar.
    pub fn new(mic_rate: u32, system_rate: u32, wav: MeetingWav) -> Result<Self, AudioError> {
        Ok(Self {
            hold: Hold::new(mic_rate),
            mic: Channel::new(mic_rate)?,
            system: Channel::new(system_rate)?,
            wav,
            written: 0,
            last_flush: Duration::ZERO,
            left: Vec::new(),
            right: Vec::new(),
            tap: None,
        })
    }

    /// Entrega a `tap` os mesmos frames que vão ao WAV, depois do hold: o mic já sai zerado nos
    /// intervalos de ditado (a prévia ao vivo, ADR-0019).
    pub fn set_tap(&mut self, tap: FrameTap) {
        self.tap = Some(tap);
    }

    pub fn push_mic(&mut self, samples: &[f32]) -> Result<(), AudioError> {
        self.hold.samples.extend(samples);
        self.release_mic(false, false)
    }

    /// Um ditado começou: o canal do mic é zerado a partir de `lookback` antes da última amostra
    /// empurrada (o pré-buffer do ditado) até `end_omit`. O que já saiu do hold não volta: é
    /// contado em `omit_late`.
    pub fn begin_omit(&mut self, lookback: Duration) {
        let late = self.hold.begin(lookback);
        if late > 0 {
            log::warn!("gravador: {late} amostras do mic já tinham saído do hold");
            self.mic.stats.omit_late += late;
        }
    }

    /// O ditado acabou: o intervalo cobre ainda `margin` depois da última amostra empurrada.
    pub fn end_omit(&mut self, margin: Duration) {
        self.hold.end(margin);
    }

    /// Os trechos omitidos já inteiros no caminho do WAV, desde a última chamada.
    pub fn take_omitted(&mut self) -> Vec<OmittedSpan> {
        std::mem::take(&mut self.hold.closed)
    }

    /// Solta o que passou do hold (tudo, com `all`) para o resample do mic; `close` como em
    /// `Hold::release`.
    fn release_mic(&mut self, all: bool, close: bool) -> Result<(), AudioError> {
        let front = self.mic.stats.delivered - self.mic.queue.len() as u64;
        let written = self.written;
        let hold = &mut self.hold;
        let (origin, rate) = (hold.origin, hold.rate);
        // O frame de saída do mic (antes de ir ao WAV) que a amostra `i` da fonte vira.
        let output_of = |i: u64| {
            let k = u128::from(i.saturating_sub(origin.0)) * u128::from(MEETING_RATE)
                / u128::from(rate.max(1));
            origin.1 + k as u64
        };
        let wav_of = |i: u64| (written + output_of(i)).saturating_sub(front);
        let batch = hold.release(all, close, wav_of);
        self.mic.push(batch)
    }

    pub fn push_system(&mut self, samples: &[f32]) -> Result<(), AudioError> {
        self.system.push(samples)
    }

    /// O áudio do sistema passa a vir noutra taxa (troca a quente da saída): o que o resampler
    /// antigo segurava entra na fila, e o relógio cobre com zeros o intervalo sem fonte.
    pub fn set_system_rate(&mut self, rate: u32) -> Result<(), AudioError> {
        self.system.drain_resampler()?;
        self.system.resampler = Resampler::to(rate, MEETING_RATE)?;
        Ok(())
    }

    /// O mic reaberto (depois de uma suspensão do SO) entrega noutra taxa: igual a
    /// [`Self::set_system_rate`]. O hold sai todo pelo resampler velho, já zerado, e um ditado
    /// em curso segue zerando o mic novo.
    pub fn set_mic_rate(&mut self, rate: u32) -> Result<(), AudioError> {
        self.release_mic(true, false)?;
        self.mic.drain_resampler()?;
        self.mic.resampler = Resampler::to(rate, MEETING_RATE)?;
        self.hold.switch_rate(rate, self.mic.stats.delivered);
        Ok(())
    }

    /// Reescreve o cabeçalho e entrega ao SO o que já foi escrito, sem esperar o próximo
    /// `FLUSH_EVERY` (o SO vai suspender).
    pub fn flush(&mut self) -> Result<(), AudioError> {
        self.wav.flush()
    }

    /// `now` é o tempo de parede desde o começo da gravação.
    pub fn tick(&mut self, now: Duration) -> Result<(), AudioError> {
        self.write_to(frames_at(now).saturating_sub(LAG_FRAMES))?;
        self.mic.trim();
        self.system.trim();
        if now.saturating_sub(self.last_flush) >= FLUSH_EVERY {
            self.wav.flush()?;
            self.last_flush = now;
        }
        Ok(())
    }

    /// Escreve até o relógio, sem a folga, e finaliza o WAV. Devolve (mic, sistema).
    pub fn finish(self, now: Duration) -> Result<(ChannelStats, ChannelStats), AudioError> {
        self.finish_omitted(now)
            .map(|(mic, system, _)| (mic, system))
    }

    /// Como `finish`, com os trechos omitidos que `take_omitted` ainda não devolveu; um ditado
    /// ainda aberto termina no fim da gravação.
    pub fn finish_omitted(
        mut self,
        now: Duration,
    ) -> Result<(ChannelStats, ChannelStats, Vec<OmittedSpan>), AudioError> {
        self.release_mic(true, true)?;
        self.mic.drain_resampler()?;
        self.system.drain_resampler()?;
        self.write_to(frames_at(now))?;
        let omitted = self.take_omitted();
        self.wav.finalize()?;
        Ok((self.mic.stats, self.system.stats, omitted))
    }

    /// Pares já escritos no WAV.
    pub fn written(&self) -> u64 {
        self.written
    }

    pub fn stats(&self) -> (ChannelStats, ChannelStats) {
        (self.mic.stats, self.system.stats)
    }

    fn write_to(&mut self, target: u64) -> Result<(), AudioError> {
        let n = usize::try_from(target.saturating_sub(self.written)).unwrap_or(usize::MAX);
        if n == 0 {
            return Ok(());
        }
        self.mic.take(n, &mut self.left);
        self.system.take(n, &mut self.right);
        self.wav.write(&self.left, &self.right)?;
        if let Some(tap) = self.tap.as_mut() {
            tap(self.written, &self.left, &self.right);
        }
        self.written += n as u64;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    const TICK: Duration = Duration::from_millis(100);

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fala-audio-rec-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    fn read(path: &Path) -> (Vec<i16>, Vec<i16>) {
        let mut reader = hound::WavReader::open(path).unwrap();
        let samples: Vec<i16> = reader.samples::<i16>().map(Result::unwrap).collect();
        let left = samples.iter().step_by(2).copied().collect();
        let right = samples.iter().skip(1).step_by(2).copied().collect();
        (left, right)
    }

    /// Valor de amostra que vira um `i16` previsível e não nulo.
    fn signal(i: usize, salt: usize) -> f32 {
        let v = ((i * 7 + salt) % 2_000) as f32 + 1.0;
        v / f32::from(i16::MAX)
    }

    /// Roda `seconds` de relógio falso; cada fonte, se ativa, entrega 10 ms por passo na sua taxa.
    struct Run {
        mic: Option<(u32, Vec<f32>)>,
        system: Option<(u32, Vec<f32>)>,
    }

    impl Run {
        fn source(rate: u32, seconds: u64, salt: usize) -> Option<(u32, Vec<f32>)> {
            let n = rate as usize * seconds as usize;
            Some((rate, (0..n).map(|i| signal(i, salt)).collect()))
        }

        fn go(&self, path: &Path, seconds: u64) -> (MeetingRecorder, u64) {
            self.go_with(path, seconds, |_, _| {})
        }

        /// Como `go`, chamando `hook(passo, gravador)` depois dos pushes de cada passo de 10 ms.
        fn go_with(
            &self,
            path: &Path,
            seconds: u64,
            mut hook: impl FnMut(u64, &mut MeetingRecorder),
        ) -> (MeetingRecorder, u64) {
            let mic_rate = self.mic.as_ref().map_or(48_000, |s| s.0);
            let sys_rate = self.system.as_ref().map_or(48_000, |s| s.0);
            let wav = MeetingWav::create(path).unwrap();
            let mut rec = MeetingRecorder::new(mic_rate, sys_rate, wav).unwrap();
            let steps = seconds * 100;
            let mut written_before_last_tick = 0;
            for step in 1..=steps {
                for (source, is_mic) in [(&self.mic, true), (&self.system, false)] {
                    if let Some((rate, samples)) = source {
                        let per = *rate as usize / 100;
                        let from = (step as usize - 1) * per;
                        let piece = &samples[from..from + per];
                        if is_mic {
                            rec.push_mic(piece).unwrap();
                        } else {
                            rec.push_system(piece).unwrap();
                        }
                    }
                }
                hook(step, &mut rec);
                let now = Duration::from_millis(step * 10);
                if now.as_millis().is_multiple_of(TICK.as_millis()) {
                    rec.tick(now).unwrap();
                    written_before_last_tick = rec.written();
                }
            }
            (rec, written_before_last_tick)
        }
    }

    fn as_i16(samples: &[f32]) -> Vec<i16> {
        samples.iter().copied().map(to_i16).collect()
    }

    #[test]
    fn silent_system_is_filled_by_the_clock() {
        let path = tmp("silent-system.wav");
        let run = Run {
            mic: Run::source(48_000, 10, 1),
            system: None,
        };
        let (rec, before) = run.go(&path, 10);
        assert_eq!(before, 480_000 - LAG_FRAMES);
        let (mic, system) = rec.finish(Duration::from_secs(10)).unwrap();
        let (left, right) = read(&path);
        assert_eq!(left.len(), 480_000);
        assert!(right.iter().all(|&r| r == 0));
        assert_eq!(left, as_i16(&run.mic.as_ref().unwrap().1));
        assert_eq!(mic.filled, 0);
        assert_eq!(system.filled, 480_000);
        assert_eq!(system.delivered, 0);
    }

    #[test]
    fn silent_mic_is_filled_by_the_clock() {
        let path = tmp("silent-mic.wav");
        let run = Run {
            mic: None,
            system: Run::source(48_000, 3, 2),
        };
        let (rec, _) = run.go(&path, 3);
        let (mic, system) = rec.finish(Duration::from_secs(3)).unwrap();
        let (left, right) = read(&path);
        assert!(left.iter().all(|&l| l == 0));
        assert_eq!(right, as_i16(&run.system.as_ref().unwrap().1));
        assert_eq!(mic.filled, 144_000);
        assert_eq!(system.filled, 0);
    }

    #[test]
    fn realtime_sources_are_written_verbatim() {
        let path = tmp("verbatim.wav");
        let run = Run {
            mic: Run::source(48_000, 4, 3),
            system: Run::source(48_000, 4, 4),
        };
        let (rec, _) = run.go(&path, 4);
        let (mic, system) = rec.finish(Duration::from_secs(4)).unwrap();
        let (left, right) = read(&path);
        assert_eq!(left, as_i16(&run.mic.as_ref().unwrap().1));
        assert_eq!(right, as_i16(&run.system.as_ref().unwrap().1));
        assert!(left.iter().all(|&l| l != 0));
        assert_eq!((mic.filled, system.filled), (0, 0));
        assert_eq!((mic.dropped, system.dropped), (0, 0));
    }

    #[test]
    fn backlog_over_1_s_is_dropped_and_counted() {
        let path = tmp("backlog.wav");
        let wav = MeetingWav::create(&path).unwrap();
        let mut rec = MeetingRecorder::new(48_000, 48_000, wav).unwrap();
        let second = Duration::from_secs(1);
        rec.tick(second).unwrap();
        assert_eq!(rec.written(), 48_000 - LAG_FRAMES);
        let burst: Vec<f32> = (0..144_000).map(|i| signal(i, 5)).collect();
        rec.push_system(&burst).unwrap();
        rec.tick(second).unwrap();
        let (mic, system) = rec.stats();
        assert_eq!(system.dropped, 144_000 - MAX_BACKLOG_FRAMES as u64);
        assert_eq!(mic.dropped, 0);
        // O que sobra é o fim do burst: o próximo frame escrito é o 96 000º.
        rec.finish(second + Duration::from_millis(10)).unwrap();
        let (_, right) = read(&path);
        let written_burst = &right[(48_000 - LAG_FRAMES as usize)..];
        assert_eq!(written_burst[0], to_i16(burst[96_000]));
    }

    #[test]
    fn any_input_rate_is_resampled_to_48k() {
        let path = tmp("rates.wav");
        let run = Run {
            mic: Run::source(16_000, 5, 6),
            system: Run::source(44_100, 5, 7),
        };
        let (rec, before) = run.go(&path, 5);
        let clock = 240_000i64 - LAG_FRAMES as i64;
        assert!((before as i64 - clock).abs() <= 480, "{before} pares");
        let (mic, system) = rec.finish(Duration::from_secs(5)).unwrap();
        let (left, right) = read(&path);
        assert_eq!(left.len(), 240_000);
        assert!((mic.delivered as i64 - 240_000).abs() <= 480, "{mic:?}");
        assert!(
            (system.delivered as i64 - 240_000).abs() <= 480,
            "{system:?}"
        );
        // Sinal de verdade nos dois canais, não só zeros.
        assert!(left.iter().filter(|&&l| l != 0).count() > 200_000);
        assert!(right.iter().filter(|&&r| r != 0).count() > 200_000);
    }

    #[test]
    fn switching_the_system_rate_keeps_the_clock_and_fills_the_gap() {
        let path = tmp("switch.wav");
        let wav = MeetingWav::create(&path).unwrap();
        let mut rec = MeetingRecorder::new(48_000, 48_000, wav).unwrap();
        // 1 s a 48 kHz, 1 s sem fonte (a troca), 1 s a 44,1 kHz.
        for step in 1..=100u64 {
            rec.push_system(&vec![0.25; 480]).unwrap();
            rec.tick(Duration::from_millis(step * 10)).unwrap();
        }
        rec.set_system_rate(44_100).unwrap();
        for step in 101..=200u64 {
            rec.tick(Duration::from_millis(step * 10)).unwrap();
        }
        for step in 201..=300u64 {
            rec.push_system(&vec![0.5; 441]).unwrap();
            rec.tick(Duration::from_millis(step * 10)).unwrap();
        }
        let (_, system) = rec.finish(Duration::from_secs(3)).unwrap();
        let (_, right) = read(&path);
        assert_eq!(right.len(), 144_000, "o relógio manda, troca ou não");
        assert!(right[..48_000].iter().all(|&r| r == to_i16(0.25)));
        // A folga de 700 ms adianta tudo o que chega depois do buraco: os zeros vão de 1 s até
        // perto de 1,3 s.
        let gap = &right[50_000..62_000];
        assert!(gap.iter().all(|&r| r == 0), "zeros no intervalo da troca");
        let after = &right[70_000..105_000];
        let want = i32::from(to_i16(0.5));
        assert!(
            after.iter().all(|&r| (i32::from(r) - want).abs() <= 2),
            "a fonte nova, reamostrada para 48 kHz"
        );
        assert!(
            (system.delivered as i64 - 96_000).abs() <= 480,
            "{system:?}"
        );
    }

    #[test]
    fn tick_rewrites_the_header_every_second() {
        let path = tmp("flush.wav");
        let wav = MeetingWav::create(&path).unwrap();
        let mut rec = MeetingRecorder::new(48_000, 48_000, wav).unwrap();
        let header = || hound::WavReader::open(&path).unwrap().duration();
        rec.tick(Duration::from_millis(900)).unwrap();
        assert_eq!(header(), 0, "0,9 s: ainda sem flush");
        rec.tick(Duration::from_secs(1)).unwrap();
        assert_eq!(header(), 48_000 - LAG_FRAMES as u32);
        rec.tick(Duration::from_millis(1_500)).unwrap();
        assert_eq!(
            header(),
            48_000 - LAG_FRAMES as u32,
            "1,5 s: o flush é a cada 1 s"
        );
        rec.tick(Duration::from_secs(2)).unwrap();
        assert_eq!(header(), 96_000 - LAG_FRAMES as u32);
        rec.tick(Duration::from_millis(2_500)).unwrap();
        rec.flush().unwrap();
        assert_eq!(
            header(),
            120_000 - LAG_FRAMES as u32,
            "flush antes de suspender"
        );
    }

    #[test]
    fn switching_the_mic_rate_keeps_the_clock() {
        let path = tmp("mic-rate.wav");
        let wav = MeetingWav::create(&path).unwrap();
        let mut rec = MeetingRecorder::new(48_000, 48_000, wav).unwrap();
        rec.push_mic(&vec![0.1; 48_000]).unwrap();
        rec.tick(Duration::from_secs(1)).unwrap();
        rec.set_mic_rate(16_000).unwrap();
        rec.push_mic(&vec![0.1; 16_000]).unwrap();
        rec.finish(Duration::from_secs(2)).unwrap();
        let (left, _) = read(&path);
        assert_eq!(left.len(), 96_000);
        // O segundo depois da troca chega reamostrado: quase todo real, não zeros.
        let real = left[48_000..].iter().filter(|s| **s != 0).count();
        assert!(real > 47_000, "{real}");
    }

    #[test]
    fn finish_writes_up_to_the_clock() {
        let path = tmp("finish.wav");
        let run = Run {
            mic: Run::source(48_000, 2, 8),
            system: Run::source(48_000, 2, 9),
        };
        let (rec, before) = run.go(&path, 2);
        assert_eq!(before, 96_000 - LAG_FRAMES);
        rec.finish(Duration::from_secs(2)).unwrap();
        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.duration(), 96_000);
    }

    const LOOKBACK: Duration = Duration::from_millis(400);
    const MARGIN: Duration = Duration::from_millis(50);
    /// O hold de 450 ms a 48 kHz.
    const HOLD: usize = 21_600;

    fn zeros(samples: &[i16]) -> bool {
        samples.iter().all(|&x| x == 0)
    }

    #[test]
    fn a_dictation_is_zeroed_from_its_prebuffer_and_nothing_else_is() {
        let path = tmp("omit.wav");
        let run = Run {
            mic: Run::source(48_000, 4, 10),
            system: Run::source(48_000, 4, 11),
        };
        // O ditado começa aos 2 s (o pré-buffer vem de antes) e acaba aos 3 s.
        let (rec, _) = run.go_with(&path, 4, |step, rec| match step {
            200 => rec.begin_omit(LOOKBACK),
            300 => rec.end_omit(MARGIN),
            _ => {}
        });
        let (mic, _, omitted) = rec.finish_omitted(Duration::from_secs(4)).unwrap();
        let (left, right) = read(&path);
        let mic_in = as_i16(&run.mic.as_ref().unwrap().1);
        let (start, end) = (96_000 - 19_200, 144_000 + 2_400);
        assert!(zeros(&left[start..end]), "zeros exatos no intervalo");
        assert_eq!(left[..start], mic_in[..start]);
        assert_eq!(left[end..], mic_in[end..]);
        assert_eq!(
            right,
            as_i16(&run.system.as_ref().unwrap().1),
            "o sistema não muda"
        );
        assert_eq!(
            omitted,
            [OmittedSpan {
                start: start as u64,
                end: end as u64
            }]
        );
        assert_eq!(mic.omit_late, 0);
    }

    #[test]
    fn the_tap_gets_what_the_wav_gets_with_the_dictation_zeroed() {
        let path = tmp("tap.wav");
        let run = Run {
            mic: Run::source(48_000, 4, 14),
            system: Run::source(48_000, 4, 15),
        };
        let tapped = std::sync::Arc::new(std::sync::Mutex::new((Vec::new(), Vec::new())));
        let sink = std::sync::Arc::clone(&tapped);
        let (rec, _) = run.go_with(&path, 4, |step, rec| match step {
            1 => {
                let sink = std::sync::Arc::clone(&sink);
                rec.set_tap(Box::new(move |first, mic, system| {
                    let mut got = sink.lock().unwrap();
                    assert_eq!(first, got.0.len() as u64, "lotes seguidos, sem buraco");
                    got.0.extend_from_slice(mic);
                    got.1.extend_from_slice(system);
                }));
            }
            200 => rec.begin_omit(LOOKBACK),
            300 => rec.end_omit(MARGIN),
            _ => {}
        });
        rec.finish(Duration::from_secs(4)).unwrap();
        let (left, right) = read(&path);
        let got = tapped.lock().unwrap();
        assert_eq!(got.0, left);
        assert_eq!(got.1, right);
        assert!(zeros(&got.0[96_000 - 19_200..144_000 + 2_400]));
    }

    #[test]
    fn a_crash_mid_dictation_leaves_only_zeros_on_disk() {
        let path = tmp("omit-crash.wav");
        let run = Run {
            mic: Run::source(48_000, 3, 12),
            system: None,
        };
        let (rec, _) = run.go_with(&path, 3, |step, rec| {
            if step == 200 {
                rec.begin_omit(LOOKBACK);
            }
        });
        // O processo morre: nada de `finish`, nem o `Drop` do WAV.
        std::mem::forget(rec);
        let (left, _) = read(&path);
        let start = 96_000 - 19_200;
        assert!(
            left.len() > start + 24_000,
            "{} frames no disco",
            left.len()
        );
        assert!(zeros(&left[start..]), "o que foi ao disco já estava zerado");
        assert!(!zeros(&left[..start]));
    }

    #[test]
    fn an_open_dictation_ends_with_the_recording() {
        let path = tmp("omit-open.wav");
        let run = Run {
            mic: Run::source(48_000, 2, 13),
            system: None,
        };
        let (rec, _) = run.go_with(&path, 2, |step, rec| {
            if step == 150 {
                rec.begin_omit(LOOKBACK);
            }
        });
        let (_, _, omitted) = rec.finish_omitted(Duration::from_secs(2)).unwrap();
        let (left, _) = read(&path);
        let start = 72_000 - 19_200;
        assert!(zeros(&left[start..]));
        assert_eq!(
            omitted,
            [OmittedSpan {
                start: start as u64,
                end: 96_000
            }]
        );
    }

    #[test]
    fn overlapping_dictations_become_one_span() {
        let path = tmp("omit-overlap.wav");
        let run = Run {
            mic: Run::source(48_000, 4, 14),
            system: None,
        };
        // Dois ditados a 200 ms um do outro: o pré-buffer do segundo cai no primeiro.
        let (mut rec, _) = run.go_with(&path, 4, |step, rec| match step {
            100 | 220 => rec.begin_omit(LOOKBACK),
            200 | 300 => rec.end_omit(MARGIN),
            _ => {}
        });
        let mut omitted = rec.take_omitted();
        let (_, _, rest) = rec.finish_omitted(Duration::from_secs(4)).unwrap();
        omitted.extend(rest);
        assert_eq!(
            omitted,
            [OmittedSpan {
                start: 48_000 - 19_200,
                end: 144_000 + 2_400
            }]
        );
    }

    #[test]
    fn a_lookback_past_the_hold_zeroes_what_is_left_and_counts_the_rest() {
        let path = tmp("omit-late.wav");
        let run = Run {
            mic: Run::source(48_000, 3, 15),
            system: None,
        };
        let (rec, _) = run.go_with(&path, 3, |step, rec| match step {
            200 => rec.begin_omit(Duration::from_secs(1)),
            250 => rec.end_omit(MARGIN),
            _ => {}
        });
        let (mic, _, _) = rec.finish_omitted(Duration::from_secs(3)).unwrap();
        assert_eq!(mic.omit_late, 48_000 - HOLD as u64);
        let (left, _) = read(&path);
        assert!(zeros(&left[96_000 - HOLD..120_000 + 2_400]));
        assert!(left[96_000 - HOLD - 1] != 0, "o que já tinha saído ficou");
    }

    #[test]
    fn a_mic_at_another_rate_is_zeroed_in_the_wav_too() {
        let path = tmp("omit-16k.wav");
        let run = Run {
            mic: Run::source(16_000, 4, 16),
            system: None,
        };
        let (rec, _) = run.go_with(&path, 4, |step, rec| match step {
            200 => rec.begin_omit(LOOKBACK),
            300 => rec.end_omit(MARGIN),
            _ => {}
        });
        let (_, _, omitted) = rec.finish_omitted(Duration::from_secs(4)).unwrap();
        let (left, _) = read(&path);
        let [span] = omitted[..] else {
            panic!("{omitted:?}")
        };
        let (start, end) = (96_000 - 19_200, 144_000 + 2_400);
        assert!(span.start.abs_diff(start as u64) <= 480, "{span:?}");
        assert!(span.end.abs_diff(end as u64) <= 480, "{span:?}");
        // A borda do filtro tem só o vazamento das amostras de fora; o miolo é zero exato.
        let edge = 480;
        assert!(zeros(&left[start + edge..end - edge]));
        assert!(left[start - 2 * edge..start - edge].iter().any(|&x| x != 0));
    }

    #[test]
    fn a_dictation_across_a_mic_rate_switch_stays_zeroed() {
        let path = tmp("omit-switch.wav");
        let wav = MeetingWav::create(&path).unwrap();
        let mut rec = MeetingRecorder::new(48_000, 48_000, wav).unwrap();
        let tick = |rec: &mut MeetingRecorder, step: u64| {
            rec.push_system(&vec![0.0; 480]).unwrap();
            rec.tick(Duration::from_millis(step * 10)).unwrap();
        };
        // 2 s a 48 kHz, o ditado começa a 1,5 s; o mic reabre a 16 kHz e o ditado acaba a 3 s.
        for step in 1..=200u64 {
            rec.push_mic(&vec![0.25; 480]).unwrap();
            if step == 150 {
                rec.begin_omit(LOOKBACK);
            }
            tick(&mut rec, step);
        }
        rec.set_mic_rate(16_000).unwrap();
        for step in 201..=400u64 {
            rec.push_mic(&vec![0.25; 160]).unwrap();
            if step == 300 {
                rec.end_omit(MARGIN);
            }
            tick(&mut rec, step);
        }
        let (mic, _, omitted) = rec.finish_omitted(Duration::from_secs(4)).unwrap();
        assert_eq!(mic.omit_late, 0);
        let (left, _) = read(&path);
        let [span] = omitted[..] else {
            panic!("{omitted:?}")
        };
        let (start, end) = (72_000 - 19_200, 144_000 + 2_400);
        assert_eq!(span.start, start as u64);
        assert!(span.end.abs_diff(end as u64) <= 480, "{span:?}");
        // Depois da troca o filtro de 16 kHz vaza até ~20 ms das amostras de fora na borda.
        assert!(
            zeros(&left[start..end - 960]),
            "zerado dos dois lados da troca"
        );
        assert!(left[..start].iter().all(|&x| x == to_i16(0.25)));
        let after = &left[end + 960..end + 24_000];
        let want = i32::from(to_i16(0.25));
        assert!(after.iter().all(|&x| (i32::from(x) - want).abs() <= 2));
    }
}
