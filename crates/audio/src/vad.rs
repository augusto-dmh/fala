//! Detecção de voz por quadro: o trait que o segmentador consulta e o Silero v4 (ADR-0009).

use std::path::Path;

use crate::AudioError;

/// Quadro de 30 ms a 16 kHz, o que o Silero v4 classifica.
pub const FRAME_SAMPLES: usize = 480;
/// Limiar de probabilidade de voz, o mesmo do desktop.
pub const SILERO_THRESHOLD: f32 = 0.3;

/// Classifica um quadro de `FRAME_SAMPLES` amostras a 16 kHz como voz ou não.
pub trait VoiceDetector: Send {
    fn is_voice(&mut self, frame: &[f32]) -> Result<bool, AudioError>;
    /// Esquece o contexto recorrente, para um ditado novo não herdar o anterior.
    fn reset(&mut self);
}

/// Silero VAD v4 pelo `vad-rs` (ONNX Runtime).
pub struct SileroVad {
    engine: vad_rs::Vad,
}

impl SileroVad {
    pub fn load(path: &Path) -> Result<Self, AudioError> {
        let engine = vad_rs::Vad::new(path, 16_000).map_err(|e| AudioError::VadLoad {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;
        Ok(Self { engine })
    }
}

impl VoiceDetector for SileroVad {
    fn is_voice(&mut self, frame: &[f32]) -> Result<bool, AudioError> {
        let result = self
            .engine
            .compute(frame)
            .map_err(|e| AudioError::Vad(e.to_string()))?;
        Ok(result.prob > SILERO_THRESHOLD)
    }

    fn reset(&mut self) {
        self.engine.reset();
    }
}
