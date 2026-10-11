//! Para onde o som da call foi: o canal gravado em zero digital e as saídas do sistema.
//!
//! Duas regras, as duas só de aviso, sobre a duração gravada (o tempo pausado não conta):
//!
//! - [`ZeroWatch`]: um canal com zero digital (pico até [`ZERO_PEAK`]) por [`ZERO_WARN_AFTER`]
//!   acende o ponto âmbar da pill. É mais rápido que o [`crate::MuteWatch`] de 2 min porque zero
//!   digital não é conversa em pausa: um mic de verdade tem ruído de fundo bem acima disso.
//! - [`OutputWatch`]: o canal do sistema em zero digital enquanto **outra** saída mostra pico por
//!   [`ELSEWHERE_AFTER`] quer dizer que a call toca num dispositivo que não está sendo gravado.
//!   É a comparação do Granola entre "capturei zeros" e o pico do endpoint.

use std::time::Duration;

use crate::{SessionMode, MUTE_WARN_AFTER};

/// Pico absoluto até o qual um intervalo é zero digital.
pub const ZERO_PEAK: f32 = 1e-6;
/// Zero digital seguido num canal que deixa o ponto da pill âmbar.
pub const ZERO_WARN_AFTER: Duration = Duration::from_secs(3);
/// Pico acima do qual uma saída está tocando algo.
pub const OUTPUT_PEAK: f32 = 1e-6;
/// Tempo de som noutra saída, com o sistema em zero, que acende o aviso.
pub const ELSEWHERE_AFTER: Duration = Duration::from_secs(10);
/// Silêncio numa saída que zera o tempo de som dela: pausas de fala não zeram, um som que acabou
/// sim.
pub const ELSEWHERE_GAP: Duration = Duration::from_secs(10);

// O aviso rápido existe porque o de canal mudo demora; se o mudo ficasse mais rápido, este sobra.
const _: () = assert!(ZERO_WARN_AFTER.as_secs() < MUTE_WARN_AFTER.as_secs());

/// Quais canais estão em zero digital há [`ZERO_WARN_AFTER`] ou mais.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Zeroed {
    pub mic: bool,
    pub system: bool,
}

/// O ponto âmbar da pill depois de ~3 s de zero digital num canal.
#[derive(Debug, Clone)]
pub struct ZeroWatch {
    mode: SessionMode,
    last: Duration,
    mic_zero: Duration,
    system_zero: Duration,
}

impl ZeroWatch {
    pub fn new(mode: SessionMode) -> Self {
        Self {
            mode,
            last: Duration::ZERO,
            mic_zero: Duration::ZERO,
            system_zero: Duration::ZERO,
        }
    }

    /// Recebe a duração gravada e o pico absoluto de cada canal desde a última observação.
    pub fn observe(&mut self, recorded: Duration, mic_peak: f32, system_peak: f32) -> Zeroed {
        let delta = recorded.saturating_sub(self.last);
        self.last = self.last.max(recorded);
        if self.mode.has_mic() {
            zero_run(&mut self.mic_zero, mic_peak, delta);
        }
        zero_run(&mut self.system_zero, system_peak, delta);
        self.zeroed()
    }

    pub fn zeroed(&self) -> Zeroed {
        Zeroed {
            mic: self.mode.has_mic() && self.mic_zero >= ZERO_WARN_AFTER,
            system: self.system_zero >= ZERO_WARN_AFTER,
        }
    }
}

fn zero_run(run: &mut Duration, peak: f32, delta: Duration) {
    if peak <= ZERO_PEAK {
        *run += delta;
    } else {
        *run = Duration::ZERO;
    }
}

#[derive(Debug, Clone)]
struct Playing {
    name: String,
    sound: Duration,
    quiet: Duration,
}

/// O aviso "o som da call está saindo por <dispositivo>".
#[derive(Debug, Clone)]
pub struct OutputWatch {
    recording: String,
    last: Duration,
    outputs: Vec<Playing>,
}

impl OutputWatch {
    /// `recording` é o nome da saída cujo loopback vira o canal do sistema.
    pub fn new(recording: &str) -> Self {
        Self {
            recording: recording.to_owned(),
            last: Duration::ZERO,
            outputs: Vec::new(),
        }
    }

    /// Passa a gravar de `name` (troca a quente): a contagem recomeça.
    pub fn set_recording(&mut self, name: &str) {
        self.recording = name.to_owned();
        self.outputs.clear();
    }

    pub fn recording(&self) -> &str {
        &self.recording
    }

    /// Recebe a duração gravada, o pico do canal do sistema e o pico de cada saída ativa, todos
    /// desde a última observação. Devolve a saída por onde a call parece tocar, se houver.
    pub fn observe<'a>(
        &mut self,
        recorded: Duration,
        system_peak: f32,
        outputs: impl IntoIterator<Item = (&'a str, f32)>,
    ) -> Option<&str> {
        let delta = recorded.saturating_sub(self.last);
        self.last = self.last.max(recorded);
        if system_peak > ZERO_PEAK {
            // O canal tem sinal: o que quer que toque noutro lugar não é a call perdida.
            self.outputs.clear();
            return None;
        }
        let mut seen = Vec::new();
        for (name, peak) in outputs {
            if name == self.recording || seen.contains(&name) {
                continue;
            }
            seen.push(name);
            let index = match self.outputs.iter().position(|p| p.name == name) {
                Some(index) => index,
                None => {
                    self.outputs.push(Playing {
                        name: name.to_owned(),
                        sound: Duration::ZERO,
                        quiet: Duration::ZERO,
                    });
                    self.outputs.len() - 1
                }
            };
            let playing = &mut self.outputs[index];
            if peak > OUTPUT_PEAK {
                playing.sound += delta;
                playing.quiet = Duration::ZERO;
            } else {
                quiet(playing, delta);
            }
        }
        // Uma saída que sumiu da lista (desplugada) conta como quieta.
        for playing in self.outputs.iter_mut() {
            if !seen.contains(&playing.name.as_str()) {
                quiet(playing, delta);
            }
        }
        self.elsewhere()
    }

    /// A saída com mais som acumulado, se passou de [`ELSEWHERE_AFTER`].
    pub fn elsewhere(&self) -> Option<&str> {
        self.outputs
            .iter()
            .filter(|p| p.sound >= ELSEWHERE_AFTER)
            .fold(None::<&Playing>, |best, p| match best {
                Some(b) if b.sound >= p.sound => Some(b),
                _ => Some(p),
            })
            .map(|p| p.name.as_str())
    }
}

fn quiet(playing: &mut Playing, delta: Duration) {
    playing.quiet += delta;
    if playing.quiet >= ELSEWHERE_GAP {
        playing.sound = Duration::ZERO;
    }
}
