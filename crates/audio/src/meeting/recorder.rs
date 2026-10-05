//! O relógio manda: cada canal escreve até `relógio − 250 ms`, com zeros no que não chegou.
//!
//! O spike (`fala-cli record`) pareava os frames por ordem de chegada, e um canal sem frames
//! travava o outro; no Windows o loopback não entrega nada enquanto nenhum app toca áudio. Aqui o
//! tempo de parede decide quantos pares existem, e um canal mudo vira silêncio (door 2 do plano
//! `meeting-recorder`).

use std::collections::VecDeque;
use std::time::Duration;

use super::wav::{MeetingWav, FLUSH_EVERY, MEETING_RATE};
use crate::resample::Resampler;
use crate::AudioError;

/// 250 ms de folga antes de preencher: cobre o jitter do callback e o período do ring.
pub const LAG_FRAMES: u64 = 12_000;
/// Um canal com mais de 500 ms à frente do escrito perde o excesso mais antigo.
pub const MAX_BACKLOG_FRAMES: usize = 24_000;

/// O que aconteceu com um canal, em frames a 48 kHz.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ChannelStats {
    /// Frames entregues pela fonte (depois do resample).
    pub delivered: u64,
    /// Zeros escritos porque a fonte não entregou a tempo.
    pub filled: u64,
    /// Frames descartados por excesso de atraso acumulado.
    pub dropped: u64,
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

fn frames_at(now: Duration) -> u64 {
    (now.as_nanos() * u128::from(MEETING_RATE) / 1_000_000_000) as u64
}

pub struct MeetingRecorder {
    mic: Channel,
    system: Channel,
    wav: MeetingWav,
    written: u64,
    last_flush: Duration,
    left: Vec<i16>,
    right: Vec<i16>,
}

impl MeetingRecorder {
    /// `mic_rate` e `system_rate` são as taxas em que `push_mic`/`push_system` vão entregar.
    pub fn new(mic_rate: u32, system_rate: u32, wav: MeetingWav) -> Result<Self, AudioError> {
        Ok(Self {
            mic: Channel::new(mic_rate)?,
            system: Channel::new(system_rate)?,
            wav,
            written: 0,
            last_flush: Duration::ZERO,
            left: Vec::new(),
            right: Vec::new(),
        })
    }

    pub fn push_mic(&mut self, samples: &[f32]) -> Result<(), AudioError> {
        self.mic.push(samples)
    }

    pub fn push_system(&mut self, samples: &[f32]) -> Result<(), AudioError> {
        self.system.push(samples)
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
    pub fn finish(mut self, now: Duration) -> Result<(ChannelStats, ChannelStats), AudioError> {
        self.mic.drain_resampler()?;
        self.system.drain_resampler()?;
        self.write_to(frames_at(now))?;
        let stats = (self.mic.stats, self.system.stats);
        self.wav.finalize()?;
        Ok(stats)
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
    fn backlog_over_500_ms_is_dropped_and_counted() {
        let path = tmp("backlog.wav");
        let wav = MeetingWav::create(&path).unwrap();
        let mut rec = MeetingRecorder::new(48_000, 48_000, wav).unwrap();
        let half = Duration::from_millis(500);
        rec.tick(half).unwrap();
        assert_eq!(rec.written(), 24_000 - LAG_FRAMES);
        let burst: Vec<f32> = (0..96_000).map(|i| signal(i, 5)).collect();
        rec.push_mic(&burst).unwrap();
        rec.push_system(&vec![0.0; 24_000]).unwrap();
        rec.tick(half).unwrap();
        let (mic, system) = rec.stats();
        assert_eq!(mic.dropped, 96_000 - MAX_BACKLOG_FRAMES as u64);
        assert_eq!(system.dropped, 0);
        // O que sobra é o fim do burst: o próximo frame escrito é o 72 000º.
        rec.finish(half + Duration::from_millis(10)).unwrap();
        let (left, _) = read(&path);
        let written_burst = &left[(24_000 - LAG_FRAMES as usize)..];
        assert_eq!(written_burst[0], to_i16(burst[72_000]));
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
    fn tick_rewrites_the_header_every_second() {
        let path = tmp("flush.wav");
        let wav = MeetingWav::create(&path).unwrap();
        let mut rec = MeetingRecorder::new(48_000, 48_000, wav).unwrap();
        let header = || hound::WavReader::open(&path).unwrap().duration();
        rec.tick(Duration::from_millis(500)).unwrap();
        assert_eq!(header(), 0, "0,5 s: ainda sem flush");
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
}
