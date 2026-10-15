//! The local transcript: the Parakeet fallback of ADR-0005 (offline, or Scribe gave up) and the
//! path without an ElevenLabs key (ADR-0023). The retained Opus files go through the dictation
//! model already loaded in the app, in 60 s windows (`fala_asr::meeting::LocalMeeting`). Nothing
//! here opens a connection.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use fala_asr::meeting::{LocalMeeting, MeetingTranscriber, SYSTEM_FILE};
use fala_asr::{AsrError, Transcriber};
use fala_core::{DictationAudio, Language, Transcript};
use tauri::{AppHandle, Manager};

use crate::managers::transcription::TranscriptionManager;

/// Ten minutes of 48 kHz audio per decode: a long meeting is never whole at 48 kHz in memory.
const DECODE_CHUNK: u64 = fala_retention::SAMPLE_RATE_HZ as u64 * 600;
/// The start sound plays into the system channel during the first second: never "them".
const START_SOUND_SAMPLES: usize = 16_000;
/// Tries of a window while the engine is out (a live dictation) or reloading, 1 s apart.
const ENGINE_TRIES: u32 = 60;

/// The app's dictation model as a [`Transcriber`].
struct AppModel(Arc<TranscriptionManager>);

impl Transcriber for AppModel {
    fn transcribe(
        &mut self,
        audio: &DictationAudio,
        language: Language,
    ) -> Result<Transcript, AsrError> {
        let mut tries = 0;
        loop {
            // "Unload immediately" unloads the model after every window.
            if !self.0.is_model_loaded() {
                self.0.initiate_model_load();
            }
            match self.0.transcribe(audio.samples().to_vec()) {
                Ok(text) => return Ok(Transcript { text, language }),
                Err(e) if tries < ENGINE_TRIES => {
                    log::debug!("meeting local window waits for the model: {e}");
                    tries += 1;
                    std::thread::sleep(Duration::from_secs(1));
                }
                Err(e) => return Err(AsrError::Inference(e.to_string())),
            }
        }
    }
}

/// The local backend for a session, with the model loading in the background if it is not yet.
/// `recorded`: a recording, whose system channel starts with the start sound (not an import).
pub(crate) fn local_meeting(
    app: &AppHandle,
    language: Language,
    recorded: bool,
) -> Result<Box<dyn MeetingTranscriber>, AsrError> {
    let Some(model) = app.try_state::<Arc<TranscriptionManager>>() else {
        return Err(AsrError::Inference(
            "no transcription model in this app".into(),
        ));
    };
    let model = Arc::clone(&model);
    model.initiate_model_load();
    Ok(Box::new(LocalMeeting::new(
        AppModel(model),
        move |path: &Path| decode_16k(path, recorded),
        language,
    )))
}

/// A retained channel as mono f32 at 16 kHz.
fn decode_16k(path: &Path, recorded: bool) -> Result<Vec<f32>, AsrError> {
    let unreadable = |reason: String| AsrError::ReadAudio {
        path: path.to_path_buf(),
        reason,
    };
    let mut resampler = fala_audio::Resampler::new(fala_retention::SAMPLE_RATE_HZ)
        .map_err(|e| unreadable(e.to_string()))?;
    let mut out = Vec::new();
    let mut start = 0;
    loop {
        let pcm = fala_retention::decode_range(path, start, start + DECODE_CHUNK)
            .map_err(|e| unreadable(e.to_string()))?;
        let samples: Vec<f32> = pcm.iter().map(|s| f32::from(*s) / 32_768.0).collect();
        resampler
            .push(&samples, &mut out)
            .map_err(|e| unreadable(e.to_string()))?;
        if (pcm.len() as u64) < DECODE_CHUNK {
            break;
        }
        start += DECODE_CHUNK;
    }
    resampler
        .finish(&mut out)
        .map_err(|e| unreadable(e.to_string()))?;
    if recorded && path.file_name().is_some_and(|name| name == SYSTEM_FILE) {
        mute_start_sound(&mut out);
    }
    Ok(out)
}

fn mute_start_sound(samples: &mut [f32]) {
    let end = START_SOUND_SAMPLES.min(samples.len());
    samples[..end].fill(0.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_retained_channel_comes_back_at_16_khz() {
        // Two seconds of a 440 Hz tone, retained as the app does, decoded for the local model.
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("recording.wav");
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: fala_retention::SAMPLE_RATE_HZ,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&wav, spec).unwrap();
        for i in 0..96_000 {
            let s = ((i as f32 * 440.0 * std::f32::consts::TAU / 48_000.0).sin() * 8_000.0) as i16;
            writer.write_sample(s).unwrap();
            writer.write_sample(s).unwrap();
        }
        writer.finalize().unwrap();
        fala_retention::retain_wav(&wav, dir.path()).unwrap();
        let samples = decode_16k(&dir.path().join(SYSTEM_FILE), true).unwrap();
        assert!(
            (31_900..=32_100).contains(&samples.len()),
            "{}",
            samples.len()
        );
        assert!(
            samples[..16_000].iter().all(|s| *s == 0.0),
            "start sound muted"
        );
        let peak = samples[16_000..]
            .iter()
            .fold(0.0_f32, |m, s| m.max(s.abs()));
        assert!((0.2..0.3).contains(&peak), "{peak}");
        // An import never played the start sound: its first second stays.
        let imported = decode_16k(&dir.path().join(SYSTEM_FILE), false).unwrap();
        assert!(imported[1_000..16_000].iter().any(|s| *s != 0.0));
    }
}
