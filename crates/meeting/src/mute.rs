use std::time::Duration;

use crate::{Levels, SessionMode, SILENCE_RMS};

/// Duração gravada seguida abaixo de [`SILENCE_RMS`] que acende o aviso de canal mudo.
pub const MUTE_WARN_AFTER: Duration = Duration::from_secs(120);

/// Quais canais estão mudos agora.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Muted {
    pub mic: bool,
    pub system: bool,
}

/// O aviso de canal mudo do painel e da pill (pitch F2): um canal que fica ~2 min de duração
/// gravada abaixo do piso de silêncio é como o Granola perde reuniões sem ninguém perceber.
///
/// Só avisa: quem para a sessão por silêncio é a máquina de estados (15 min nos dois canais).
/// O tempo pausado não conta, porque a entrada é a duração gravada, como no `Tick`.
#[derive(Debug, Clone)]
pub struct MuteWatch {
    mode: SessionMode,
    last: Duration,
    mic_silent: Duration,
    system_silent: Duration,
}

impl MuteWatch {
    pub fn new(mode: SessionMode) -> Self {
        Self {
            mode,
            last: Duration::ZERO,
            mic_silent: Duration::ZERO,
            system_silent: Duration::ZERO,
        }
    }

    /// Recebe a duração total já gravada e os níveis do intervalo desde a última observação.
    pub fn observe(&mut self, recorded: Duration, levels: Levels) -> Muted {
        let delta = recorded.saturating_sub(self.last);
        self.last = self.last.max(recorded);
        if self.mode.has_mic() {
            accumulate(&mut self.mic_silent, levels.mic, delta);
        }
        accumulate(&mut self.system_silent, levels.system, delta);
        self.muted()
    }

    pub fn muted(&self) -> Muted {
        Muted {
            mic: self.mode.has_mic() && self.mic_silent >= MUTE_WARN_AFTER,
            system: self.system_silent >= MUTE_WARN_AFTER,
        }
    }
}

fn accumulate(silent: &mut Duration, level: f32, delta: Duration) {
    if level < SILENCE_RMS {
        *silent += delta;
    } else {
        *silent = Duration::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOUD: f32 = 0.002;
    const QUIET: f32 = 0.0;

    /// Observa em passos de 1 s até `until`, com os níveis dados.
    fn run(watch: &mut MuteWatch, from: u64, until: u64, mic: f32, system: f32) -> Muted {
        let mut muted = watch.muted();
        for s in from + 1..=until {
            muted = watch.observe(Duration::from_secs(s), Levels { mic, system });
        }
        muted
    }

    #[test]
    fn system_mutes_at_120_s() {
        let mut watch = MuteWatch::new(SessionMode::Meeting);
        let at_119 = run(&mut watch, 0, 119, LOUD, QUIET);
        assert_eq!(at_119, Muted::default());
        let at_120 = run(&mut watch, 119, 120, LOUD, QUIET);
        assert_eq!(
            at_120,
            Muted {
                mic: false,
                system: true
            }
        );
    }

    #[test]
    fn sound_clears_and_restarts() {
        let mut watch = MuteWatch::new(SessionMode::Meeting);
        assert!(run(&mut watch, 0, 130, LOUD, QUIET).system);
        // Um intervalo com som desmarca na hora.
        assert!(!run(&mut watch, 130, 131, LOUD, LOUD).system);
        // E a contagem recomeça do zero: 119 s de silêncio de novo não marcam.
        assert!(!run(&mut watch, 131, 250, LOUD, QUIET).system);
        assert!(run(&mut watch, 250, 251, LOUD, QUIET).system);
    }

    #[test]
    fn mic_only_counts_with_mic() {
        let mut system_only = MuteWatch::new(SessionMode::SystemOnly);
        assert!(!run(&mut system_only, 0, 600, QUIET, LOUD).mic);

        let mut meeting = MuteWatch::new(SessionMode::Meeting);
        assert!(run(&mut meeting, 0, 600, QUIET, LOUD).mic);
        let mut in_person = MuteWatch::new(SessionMode::InPerson);
        assert!(run(&mut in_person, 0, 120, QUIET, LOUD).mic);
    }

    #[test]
    fn paused_time_does_not_count() {
        // A duração gravada não anda durante a pausa: observar o mesmo valor não soma nada.
        let mut watch = MuteWatch::new(SessionMode::Meeting);
        run(&mut watch, 0, 100, LOUD, QUIET);
        for _ in 0..1000 {
            watch.observe(Duration::from_secs(100), Levels::default());
        }
        assert!(!watch.muted().system);
    }
}
