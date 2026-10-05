//! O WAV de reunião: 48 kHz, 2 canais, i16, L = mic, R = sistema, cabeçalho a cada ≤ 1 s.

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::AudioError;

pub const MEETING_RATE: u32 = 48_000;
/// O cabeçalho é reescrito pelo menos a cada segundo (design doc §3.4).
pub const FLUSH_EVERY: Duration = Duration::from_secs(1);

pub struct MeetingWav {
    writer: hound::WavWriter<BufWriter<File>>,
    path: PathBuf,
}

impl MeetingWav {
    pub fn create(path: &Path) -> Result<Self, AudioError> {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: MEETING_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let writer = hound::WavWriter::create(path, spec).map_err(|e| AudioError::Wav {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;
        Ok(Self {
            writer,
            path: path.to_path_buf(),
        })
    }

    /// Escreve pares (L, R); as duas fatias têm o mesmo tamanho.
    pub fn write(&mut self, left: &[i16], right: &[i16]) -> Result<(), AudioError> {
        for (l, r) in left.iter().zip(right) {
            self.writer.write_sample(*l).map_err(|e| self.error(e))?;
            self.writer.write_sample(*r).map_err(|e| self.error(e))?;
        }
        Ok(())
    }

    /// Reescreve o cabeçalho com o tamanho atual e esvazia o buffer no disco.
    pub fn flush(&mut self) -> Result<(), AudioError> {
        self.writer.flush().map_err(|e| self.error(e))
    }

    pub fn finalize(self) -> Result<(), AudioError> {
        let path = self.path;
        self.writer.finalize().map_err(|e| AudioError::Wav {
            path,
            reason: e.to_string(),
        })
    }

    fn error(&self, e: hound::Error) -> AudioError {
        AudioError::Wav {
            path: self.path.clone(),
            reason: e.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fala-audio-wav-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    fn read(path: &Path) -> (hound::WavSpec, Vec<i16>) {
        let mut reader = hound::WavReader::open(path).unwrap();
        let spec = reader.spec();
        let samples = reader.samples::<i16>().map(Result::unwrap).collect();
        (spec, samples)
    }

    #[test]
    fn killed_writer_leaves_flushed_frames() {
        assert!(FLUSH_EVERY <= Duration::from_secs(1));
        let path = tmp("killed.wav");
        let mut wav = MeetingWav::create(&path).unwrap();
        let left: Vec<i16> = (0..4_800).map(|i| i as i16).collect();
        let right: Vec<i16> = (0..4_800).map(|i| -(i as i16)).collect();
        wav.write(&left, &right).unwrap();
        wav.flush().unwrap();
        // Depois do flush: escrito, mas nunca anunciado no cabeçalho.
        wav.write(&[7; 1_000], &[7; 1_000]).unwrap();
        // O processo "morre": nada de `finalize` nem de `Drop`.
        std::mem::forget(wav);
        let (spec, samples) = read(&path);
        assert_eq!(spec.channels, 2);
        assert_eq!(samples.len(), 2 * 4_800);
        assert_eq!(samples[0..2], [0, 0]);
        assert_eq!(samples[2 * 4_799..], [4_799, -4_799]);
    }

    #[test]
    fn finished_wav_is_48k_stereo_mic_left() {
        let path = tmp("finished.wav");
        let mut wav = MeetingWav::create(&path).unwrap();
        wav.write(&[100, 101, 102], &[-1, -2, -3]).unwrap();
        wav.finalize().unwrap();
        let (spec, samples) = read(&path);
        assert_eq!(spec.sample_rate, 48_000);
        assert_eq!(spec.channels, 2);
        assert_eq!(spec.bits_per_sample, 16);
        assert_eq!(spec.sample_format, hound::SampleFormat::Int);
        assert_eq!(samples, [100, -1, 101, -2, 102, -3]);
    }
}
