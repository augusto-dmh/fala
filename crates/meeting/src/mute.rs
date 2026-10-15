use std::time::Duration;

use crate::{Levels, SessionMode, SILENCE_RMS};

/// Duração gravada seguida abaixo de [`SILENCE_RMS`] que acende o aviso de canal mudo.
pub const MUTE_WARN_AFTER: Duration = Duration::from_secs(120);
/// O mesmo para um mic que ainda não deu sinal desde o começo ou desde a troca: o microfone
/// errado (ou mudo no Windows) se mostra em segundos, e a reunião não espera 2 min para avisar.
pub const MIC_EARLY_WARN_AFTER: Duration = Duration::from_secs(15);

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
    /// O mic passou do piso desde o começo ou desde a última troca.
    mic_heard: bool,
}

impl MuteWatch {
    pub fn new(mode: SessionMode) -> Self {
        Self {
            mode,
            last: Duration::ZERO,
            mic_silent: Duration::ZERO,
            system_silent: Duration::ZERO,
            mic_heard: false,
        }
    }

    /// A gravação passou a outro microfone: a contagem do mic recomeça, com a janela curta.
    pub fn mic_switched(&mut self) {
        self.mic_silent = Duration::ZERO;
        self.mic_heard = false;
    }

    /// Recebe a duração total já gravada e os níveis do intervalo desde a última observação.
    pub fn observe(&mut self, recorded: Duration, levels: Levels) -> Muted {
        let delta = recorded.saturating_sub(self.last);
        self.last = self.last.max(recorded);
        // Um ditado zera o mic de propósito: esse intervalo não é silêncio nem som.
        if self.mode.has_mic() && !levels.mic_omitted {
            accumulate(&mut self.mic_silent, levels.mic, delta);
            self.mic_heard |= levels.mic >= SILENCE_RMS;
        }
        accumulate(&mut self.system_silent, levels.system, delta);
        self.muted()
    }

    pub fn muted(&self) -> Muted {
        Muted {
            mic: self.mode.has_mic() && self.mic_silent >= self.mic_warn_after(),
            system: self.system_silent >= MUTE_WARN_AFTER,
        }
    }
}

impl MuteWatch {
    /// O mic ainda não deu sinal desde o começo ou a troca, há [`MIC_EARLY_WARN_AFTER`]: o caso
    /// do microfone errado, que a pill mostra. Depois do primeiro sinal, um mic que só fica quieto
    /// (supressão de ruído entre frases) acende só o aviso de 120 s do painel.
    pub fn mic_unheard(&self) -> bool {
        self.mode.has_mic() && !self.mic_heard && self.mic_silent >= MIC_EARLY_WARN_AFTER
    }

    fn mic_warn_after(&self) -> Duration {
        if self.mic_heard {
            MUTE_WARN_AFTER
        } else {
            MIC_EARLY_WARN_AFTER
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
            let levels = Levels {
                mic,
                system,
                mic_omitted: false,
            };
            muted = watch.observe(Duration::from_secs(s), levels);
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
    fn a_dictation_is_neither_silence_nor_sound_on_the_mic() {
        let mut watch = MuteWatch::new(SessionMode::Meeting);
        // O mic deu sinal no primeiro segundo: vale a janela de 120 s.
        run(&mut watch, 0, 1, LOUD, LOUD);
        run(&mut watch, 1, 101, QUIET, LOUD);
        // 3 min de ditado: o mic zerado não completa os 120 s nem zera a contagem.
        for s in 102..=281 {
            let levels = Levels {
                mic: QUIET,
                system: LOUD,
                mic_omitted: true,
            };
            assert!(!watch.observe(Duration::from_secs(s), levels).mic, "{s} s");
        }
        assert!(!run(&mut watch, 281, 300, QUIET, LOUD).mic);
        assert!(run(&mut watch, 300, 301, QUIET, LOUD).mic, "100 s + 20 s");
    }

    #[test]
    fn a_mic_never_heard_warns_at_15_s() {
        // O microfone errado: zero desde o começo, enquanto a call toca no sistema.
        let mut watch = MuteWatch::new(SessionMode::Meeting);
        assert!(!run(&mut watch, 0, 14, QUIET, LOUD).mic);
        assert!(!watch.mic_unheard());
        assert!(run(&mut watch, 14, 15, QUIET, LOUD).mic);
        assert!(watch.mic_unheard());
        // Um sinal apaga o aviso, e daí em diante o mic tem os 120 s de sempre.
        assert!(!run(&mut watch, 15, 16, LOUD, LOUD).mic);
        assert!(!run(&mut watch, 16, 135, QUIET, LOUD).mic);
        assert!(run(&mut watch, 135, 136, QUIET, LOUD).mic);
        assert!(!watch.mic_unheard(), "heard once: only the panel's warning");
    }

    #[test]
    fn switching_the_mic_starts_the_short_window_again() {
        let mut watch = MuteWatch::new(SessionMode::Meeting);
        run(&mut watch, 0, 10, LOUD, LOUD);
        assert!(!run(&mut watch, 10, 100, QUIET, LOUD).mic);
        watch.mic_switched();
        assert!(!watch.muted().mic);
        assert!(!run(&mut watch, 100, 114, QUIET, LOUD).mic);
        assert!(run(&mut watch, 114, 115, QUIET, LOUD).mic);
        // Sem mic, nada muda.
        let mut system_only = MuteWatch::new(SessionMode::SystemOnly);
        system_only.mic_switched();
        assert!(!run(&mut system_only, 0, 60, QUIET, LOUD).mic);
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
