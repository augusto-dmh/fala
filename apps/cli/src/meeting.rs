//! `fala-cli meeting`: grava mic e áudio do sistema num WAV estéreo até Enter ou EOF.
//!
//! Só grava quando chamado e loga o indicador no começo e a cada 60 s (ADR-0005). Um canal que
//! para de entregar frames vira silêncio no ritmo do relógio; a gravação não aborta por isso.

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use clap::Args;
use fala_audio::{AudioError, MeetingRecorder, MeetingWav, Mic, SystemAudio};

const TICK: Duration = Duration::from_millis(100);
const PROGRESS: Duration = Duration::from_secs(60);

#[derive(Args)]
pub struct MeetingArgs {
    /// Arquivo WAV de saída (48 kHz, estéreo, L = mic, R = sistema).
    #[arg(long)]
    out: PathBuf,
    /// Áudio do sistema: no Linux, o `node.name` do sink do PipeWire; no Windows, parte do nome
    /// do dispositivo de saída.
    #[arg(long)]
    system: String,
    /// Parte do nome do microfone (padrão: a entrada padrão).
    #[arg(long)]
    mic: Option<String>,
}

/// Falha com o código de saída do contrato: 1 em tempo de execução, 2 entrada inválida.
pub struct Failure {
    pub code: u8,
    pub error: anyhow::Error,
}

fn audio(error: AudioError) -> Failure {
    let code = match error {
        AudioError::NoDevice { .. } | AudioError::NoSystem { .. } => 2,
        _ => 1,
    };
    Failure {
        code,
        error: error.into(),
    }
}

pub fn run(args: MeetingArgs) -> Result<(), Failure> {
    if let Some(needle) = &args.mic {
        Mic::check(needle).map_err(audio)?;
    }
    SystemAudio::check(&args.system).map_err(audio)?;
    let wav = MeetingWav::create(&args.out).map_err(|e| Failure {
        code: 2,
        error: e.into(),
    })?;
    let mut system = SystemAudio::open(&args.system).map_err(audio)?;
    let mut mic = Mic::open(args.mic.as_deref()).map_err(audio)?;
    let mut recorder =
        MeetingRecorder::new(mic.sample_rate(), system.sample_rate(), wav).map_err(audio)?;

    let (tx, stop) = mpsc::channel();
    thread::spawn(move || {
        let mut line = String::new();
        // Uma linha ou o EOF param; o que vier é só o sinal.
        let _ = std::io::stdin().read_line(&mut line);
        let _ = tx.send(());
    });

    let started = Instant::now();
    log::info!(
        "● gravando reunião em {} (mic: {}, sistema: {}); Enter ou Ctrl+D para parar",
        args.out.display(),
        mic.name(),
        args.system
    );
    let mut progress = Progress::default();
    let mut buffer = Vec::new();
    loop {
        let stopping = match stop.recv_timeout(TICK) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => true,
            Err(mpsc::RecvTimeoutError::Timeout) => false,
        };
        buffer.clear();
        mic.drain_into(&mut buffer);
        recorder.push_mic(&buffer).map_err(audio)?;
        buffer.clear();
        system.drain_into(&mut buffer);
        recorder.push_system(&buffer).map_err(audio)?;
        let now = started.elapsed();
        if stopping {
            let (mic_stats, sys_stats) = recorder.finish(now).map_err(audio)?;
            log::info!(
                "reunião gravada: wall_s={:.1} frames={} mic_filled={} mic_dropped={} sys_filled={} sys_dropped={} ring_dropped_mic={} ring_dropped_sys={}",
                now.as_secs_f64(),
                mic_stats.delivered.max(sys_stats.delivered),
                mic_stats.filled,
                mic_stats.dropped,
                sys_stats.filled,
                sys_stats.dropped,
                mic.dropped(),
                system.dropped()
            );
            return Ok(());
        }
        recorder.tick(now).map_err(audio)?;
        if let Some(minutes) = progress.due(now) {
            log::info!("● gravando reunião: {minutes} min");
        }
    }
}

/// O indicador periódico: um aviso a cada 60 s de gravação.
#[derive(Default)]
struct Progress {
    shown: u64,
}

impl Progress {
    fn due(&mut self, now: Duration) -> Option<u64> {
        let minutes = now.as_secs() / PROGRESS.as_secs();
        if minutes > self.shown {
            self.shown = minutes;
            Some(minutes)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_every_60_s() {
        let mut p = Progress::default();
        assert_eq!(p.due(Duration::from_millis(59_900)), None);
        assert_eq!(p.due(Duration::from_secs(60)), Some(1));
        assert_eq!(p.due(Duration::from_secs(61)), None);
        assert_eq!(p.due(Duration::from_millis(119_999)), None);
        assert_eq!(p.due(Duration::from_secs(120)), Some(2));
    }
}
