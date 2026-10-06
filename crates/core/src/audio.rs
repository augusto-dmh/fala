use std::time::Duration;

/// Áudio de um ditado: samples mono f32 a 16 kHz, a entrada do ASR.
///
/// Não implementa `Serialize` de propósito: o áudio de ditado nunca sai da máquina
/// (ADR-0003, `ARCHITECTURE.md` § Invariantes). Um teste de compilação garante isso.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DictationAudio {
    samples: Vec<f32>,
}

impl DictationAudio {
    pub const SAMPLE_RATE_HZ: u32 = 16_000;

    /// Recebe samples já em mono a `SAMPLE_RATE_HZ`.
    pub fn new(samples: Vec<f32>) -> Self {
        Self { samples }
    }

    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    pub fn into_samples(self) -> Vec<f32> {
        self.samples
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.samples.len() as f64 / f64::from(Self::SAMPLE_RATE_HZ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictation_audio_is_not_serializable() {
        static_assertions::assert_not_impl_any!(DictationAudio: serde::Serialize);
    }

    #[test]
    fn one_second_of_samples() {
        let audio = DictationAudio::new(vec![0.0; 16_000]);
        assert_eq!(audio.duration(), Duration::from_secs(1));
        assert_eq!(DictationAudio::SAMPLE_RATE_HZ, 16_000);
    }

    #[test]
    fn empty_audio() {
        let audio = DictationAudio::new(vec![]);
        assert_eq!(audio.duration(), Duration::ZERO);
        assert!(audio.is_empty());
    }
}
