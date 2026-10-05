//! Resample de mono f32 da taxa do dispositivo para os 16 kHz do ASR, com `rubato`.

use rubato::{FftFixedIn, Resampler as _};

use crate::AudioError;

/// Taxa de saída: a do `DictationAudio`.
pub const OUT_RATE: u32 = fala_core::DictationAudio::SAMPLE_RATE_HZ;
const CHUNK_IN: usize = 1024;

/// Reamostrador em fluxo. A saída é alinhada à entrada: o atraso do filtro é descontado no
/// começo e devolvido no `finish`, de modo que a saída total é `in_len × out_rate / in_rate`.
pub struct Resampler {
    inner: Option<FftFixedIn<f32>>,
    in_rate: u32,
    out_rate: u32,
    chunk: Vec<f32>,
    in_count: u64,
    out_count: u64,
    /// Amostras do atraso do filtro ainda por descartar no começo da saída.
    skip: usize,
}

impl Resampler {
    /// Para os 16 kHz do ditado.
    pub fn new(in_rate: u32) -> Result<Self, AudioError> {
        Self::to(in_rate, OUT_RATE)
    }

    /// Para uma taxa de saída qualquer (os 48 kHz da reunião, por exemplo).
    pub fn to(in_rate: u32, out_rate: u32) -> Result<Self, AudioError> {
        let inner = if in_rate == out_rate {
            None
        } else {
            Some(
                FftFixedIn::<f32>::new(in_rate as usize, out_rate as usize, CHUNK_IN, 1, 1)
                    .map_err(|e| AudioError::Resample(e.to_string()))?,
            )
        };
        let skip = inner.as_ref().map_or(0, |r| r.output_delay());
        Ok(Self {
            inner,
            in_rate,
            out_rate,
            chunk: Vec::with_capacity(CHUNK_IN),
            in_count: 0,
            out_count: 0,
            skip,
        })
    }

    /// Reamostra `src` e acrescenta a saída disponível a `out`.
    pub fn push(&mut self, mut src: &[f32], out: &mut Vec<f32>) -> Result<(), AudioError> {
        if self.inner.is_none() {
            out.extend_from_slice(src);
            return Ok(());
        }
        self.in_count += src.len() as u64;
        while !src.is_empty() {
            let take = (CHUNK_IN - self.chunk.len()).min(src.len());
            self.chunk.extend_from_slice(&src[..take]);
            src = &src[take..];
            if self.chunk.len() == CHUNK_IN {
                let produced = match self.inner.as_mut() {
                    Some(r) => r
                        .process(&[&self.chunk[..]], None)
                        .map_err(|e| AudioError::Resample(e.to_string()))?,
                    None => return Ok(()),
                };
                self.chunk.clear();
                self.emit(&produced[0], out);
            }
        }
        Ok(())
    }

    /// Esvazia o filtro: completa a saída até `in_len × out_rate / in_rate` e volta ao estado
    /// inicial, pronto para um fluxo novo.
    pub fn finish(&mut self, out: &mut Vec<f32>) -> Result<(), AudioError> {
        let Some(inner) = self.inner.as_mut() else {
            return Ok(());
        };
        let expected = self.in_count * u64::from(self.out_rate) / u64::from(self.in_rate);
        let mut tail = Vec::new();
        if !self.chunk.is_empty() {
            let produced = inner
                .process_partial(Some(&[&self.chunk[..]]), None)
                .map_err(|e| AudioError::Resample(e.to_string()))?;
            tail.extend_from_slice(&produced[0]);
        }
        let mut rounds = 0;
        while self.out_count + (tail.len().saturating_sub(self.skip) as u64) < expected
            && rounds < 16
        {
            rounds += 1;
            let produced = inner
                .process_partial::<&[f32]>(None, None)
                .map_err(|e| AudioError::Resample(e.to_string()))?;
            tail.extend_from_slice(&produced[0]);
        }
        let mut aligned = Vec::new();
        self.emit(&tail, &mut aligned);
        let missing = expected.saturating_sub(self.out_count - aligned.len() as u64);
        aligned.truncate(usize::try_from(missing).unwrap_or(usize::MAX));
        out.extend_from_slice(&aligned);
        self.reset();
        Ok(())
    }

    fn reset(&mut self) {
        if let Some(inner) = self.inner.as_mut() {
            inner.reset();
            self.skip = inner.output_delay();
        }
        self.chunk.clear();
        self.in_count = 0;
        self.out_count = 0;
    }

    fn emit(&mut self, produced: &[f32], out: &mut Vec<f32>) {
        let drop = self.skip.min(produced.len());
        self.skip -= drop;
        out.extend_from_slice(&produced[drop..]);
        self.out_count += (produced.len() - drop) as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(rate: u32, freq: f64, seconds: f64) -> Vec<f32> {
        let n = (f64::from(rate) * seconds) as usize;
        (0..n)
            .map(|i| (2.0 * std::f64::consts::PI * freq * i as f64 / f64::from(rate)).sin() as f32)
            .collect()
    }

    fn resample_all(rate: u32, input: &[f32], piece: usize) -> Vec<f32> {
        let mut r = Resampler::new(rate).unwrap();
        let mut out = Vec::new();
        for chunk in input.chunks(piece) {
            r.push(chunk, &mut out).unwrap();
        }
        r.finish(&mut out).unwrap();
        out
    }

    #[test]
    fn output_length_matches_ratio() {
        for rate in [48_000u32, 44_100] {
            for len in [rate as usize * 2 + 37, 999, rate as usize / 3] {
                let input = sine(rate, 300.0, len as f64 / f64::from(rate));
                let out = resample_all(rate, &input, 441);
                let expected = (input.len() as u64 * 16_000 / u64::from(rate)) as i64;
                let diff = (out.len() as i64 - expected).abs();
                assert!(
                    diff <= 480,
                    "rate {rate}, in {}: out {} expected {expected}",
                    input.len(),
                    out.len()
                );
            }
        }
    }

    #[test]
    fn sine_keeps_its_frequency() {
        let out = resample_all(48_000, &sine(48_000, 440.0, 2.0), 480);
        // Ignora as pontas, onde o filtro ainda acomoda.
        let body = &out[1_600..out.len() - 1_600];
        let crossings = body
            .windows(2)
            .filter(|w| (w[0] < 0.0) != (w[1] < 0.0))
            .count();
        let seconds = body.len() as f64 / 16_000.0;
        let freq = crossings as f64 / 2.0 / seconds;
        assert!((freq - 440.0).abs() <= 4.4, "frequência medida {freq} Hz");
    }

    #[test]
    fn upsamples_to_48k() {
        for len in [16_000 * 2 + 37, 999] {
            let input = sine(16_000, 300.0, len as f64 / 16_000.0);
            let mut r = Resampler::to(16_000, 48_000).unwrap();
            let mut out = Vec::new();
            for chunk in input.chunks(160) {
                r.push(chunk, &mut out).unwrap();
            }
            r.finish(&mut out).unwrap();
            let diff = (out.len() as i64 - input.len() as i64 * 3).abs();
            assert!(diff <= 480, "in {}: out {}", input.len(), out.len());
        }
    }

    #[test]
    fn finish_resets_for_next_stream() {
        let mut r = Resampler::new(48_000).unwrap();
        let mut first = Vec::new();
        r.push(&vec![0.9; 4_800], &mut first).unwrap();
        r.finish(&mut first).unwrap();
        let mut second = Vec::new();
        r.push(&vec![0.0; 9_600], &mut second).unwrap();
        r.finish(&mut second).unwrap();
        assert_eq!(second.len(), 3_200);
        let peak = second.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak < 0.01, "sobra do fluxo anterior: pico {peak}");
    }
}
