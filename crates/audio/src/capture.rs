//! Segmentação do ditado: amostras na taxa do dispositivo entram, utterances de 16 kHz saem.
//!
//! Enquanto ocioso, guarda os últimos 300 ms (o pré-buffer). No `start`, esse trecho abre a
//! sessão; durante a sessão, o VAD abre uma utterance depois de 60 ms de voz, com até 450 ms de
//! pre-roll, e a fecha depois de 450 ms de silêncio ou aos 15 s. O desenho segue o `SmoothedVad`
//! do desktop.

use std::collections::VecDeque;

use fala_core::DictationAudio;

use crate::resample::Resampler;
use crate::vad::{VoiceDetector, FRAME_SAMPLES};
use crate::AudioError;

/// 300 ms a 16 kHz.
pub const PREBUFFER_SAMPLES: usize = 4_800;
/// 60 ms de voz confirmam o começo de uma utterance.
pub const ONSET_FRAMES: usize = 2;
/// 450 ms antes do primeiro quadro de voz entram na utterance.
pub const PRE_ROLL_FRAMES: usize = 15;
/// 450 ms de silêncio fecham a utterance.
pub const HANGOVER_FRAMES: usize = 15;
/// 15 s a 16 kHz: uma utterance mais longa é cortada aqui.
pub const MAX_UTTERANCE_SAMPLES: usize = 240_000;

pub struct DictationCapture {
    resampler: Resampler,
    vad: Box<dyn VoiceDetector>,
    prebuffer: VecDeque<f32>,
    session: Option<Session>,
    scratch: Vec<f32>,
}

#[derive(Default)]
struct Session {
    pending: Vec<f32>,
    recent: VecDeque<Vec<f32>>,
    onset: usize,
    silence: usize,
    utterance: Option<Vec<f32>>,
}

impl DictationCapture {
    pub fn new(in_rate: u32, vad: Box<dyn VoiceDetector>) -> Result<Self, AudioError> {
        Ok(Self {
            resampler: Resampler::new(in_rate)?,
            vad,
            prebuffer: VecDeque::with_capacity(PREBUFFER_SAMPLES),
            session: None,
            scratch: Vec::new(),
        })
    }

    pub fn is_recording(&self) -> bool {
        self.session.is_some()
    }

    /// Amostras mono na taxa do dispositivo. Fora de uma sessão só alimentam o pré-buffer;
    /// dentro, devolvem as utterances que o VAD fechou.
    pub fn feed(&mut self, samples: &[f32]) -> Result<Vec<DictationAudio>, AudioError> {
        self.scratch.clear();
        self.resampler.push(samples, &mut self.scratch)?;
        if self.session.is_none() {
            self.prebuffer.extend(self.scratch.iter().copied());
            let excess = self.prebuffer.len().saturating_sub(PREBUFFER_SAMPLES);
            self.prebuffer.drain(..excess);
            return Ok(Vec::new());
        }
        let resampled = std::mem::take(&mut self.scratch);
        let done = self.process(&resampled);
        self.scratch = resampled;
        done
    }

    /// Começa uma sessão com o pré-buffer na frente.
    pub fn start(&mut self) -> Result<Vec<DictationAudio>, AudioError> {
        if self.session.is_some() {
            return Ok(Vec::new());
        }
        self.vad.reset();
        self.session = Some(Session::default());
        let prebuffer: Vec<f32> = self.prebuffer.drain(..).collect();
        self.process(&prebuffer)
    }

    /// Termina a sessão: esvazia o resampler e entrega a utterance aberta, se houver.
    pub fn stop(&mut self) -> Result<Vec<DictationAudio>, AudioError> {
        if self.session.is_none() {
            return Ok(Vec::new());
        }
        self.scratch.clear();
        self.resampler.finish(&mut self.scratch)?;
        let tail = std::mem::take(&mut self.scratch);
        let mut done = self.process(&tail)?;
        if let Some(mut session) = self.session.take()
            && let Some(mut utterance) = session.utterance.take()
        {
            utterance.append(&mut session.pending);
            done.push(DictationAudio::new(utterance));
        }
        self.vad.reset();
        Ok(done)
    }

    fn process(&mut self, samples: &[f32]) -> Result<Vec<DictationAudio>, AudioError> {
        let mut done = Vec::new();
        let Some(session) = self.session.as_mut() else {
            return Ok(done);
        };
        session.pending.extend_from_slice(samples);
        let full = session.pending.len() / FRAME_SAMPLES * FRAME_SAMPLES;
        let frames: Vec<f32> = session.pending.drain(..full).collect();
        for frame in frames.as_chunks::<FRAME_SAMPLES>().0 {
            let voice = self.vad.is_voice(frame)?;
            if let Some(utterance) = session.step(frame, voice) {
                done.push(DictationAudio::new(utterance));
            }
        }
        Ok(done)
    }
}

impl Session {
    /// Um quadro classificado; devolve a utterance que ele fecha, se fechar.
    fn step(&mut self, frame: &[f32], voice: bool) -> Option<Vec<f32>> {
        if let Some(utterance) = self.utterance.as_mut() {
            utterance.extend_from_slice(frame);
            self.silence = if voice { 0 } else { self.silence + 1 };
            if self.silence >= HANGOVER_FRAMES {
                self.silence = 0;
                return self.utterance.take();
            }
            if utterance.len() >= MAX_UTTERANCE_SAMPLES {
                return self.utterance.replace(Vec::new());
            }
            return None;
        }
        self.recent.push_back(frame.to_vec());
        if self.recent.len() > PRE_ROLL_FRAMES + ONSET_FRAMES {
            self.recent.pop_front();
        }
        if !voice {
            self.onset = 0;
            return None;
        }
        self.onset += 1;
        if self.onset >= ONSET_FRAMES {
            self.onset = 0;
            self.silence = 0;
            self.utterance = Some(self.recent.drain(..).flatten().collect());
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Voz quando o primeiro valor do quadro é ≥ 1000: os testes marcam cada quadro com um
    /// código e o VAD lê o código.
    struct CodeVad;

    impl VoiceDetector for CodeVad {
        fn is_voice(&mut self, frame: &[f32]) -> Result<bool, AudioError> {
            Ok(frame[0] >= 1000.0)
        }
        fn reset(&mut self) {}
    }

    struct Always(bool);

    impl VoiceDetector for Always {
        fn is_voice(&mut self, _: &[f32]) -> Result<bool, AudioError> {
            Ok(self.0)
        }
        fn reset(&mut self) {}
    }

    fn capture(rate: u32, vad: impl VoiceDetector + 'static) -> DictationCapture {
        DictationCapture::new(rate, Box::new(vad)).unwrap()
    }

    fn ramp(range: std::ops::Range<usize>) -> Vec<f32> {
        range.map(|i| i as f32).collect()
    }

    fn frames(codes: &[f32]) -> Vec<f32> {
        codes
            .iter()
            .flat_map(|c| std::iter::repeat_n(*c, FRAME_SAMPLES))
            .collect()
    }

    fn concat(utterances: &[DictationAudio]) -> Vec<f32> {
        utterances
            .iter()
            .flat_map(|u| u.samples().to_vec())
            .collect()
    }

    #[test]
    fn prebuffer_opens_session_with_last_300_ms() {
        let mut c = capture(16_000, Always(true));
        let before = ramp(0..10_000);
        for piece in before.chunks(333) {
            assert!(c.feed(piece).unwrap().is_empty());
        }
        let mut out = c.start().unwrap();
        out.extend(c.feed(&ramp(10_000..12_000)).unwrap());
        out.extend(c.stop().unwrap());
        assert_eq!(concat(&out), ramp(5_200..12_000));
    }

    #[test]
    fn short_prebuffer_keeps_everything() {
        let mut c = capture(16_000, Always(true));
        c.feed(&ramp(0..1_000)).unwrap();
        let mut out = c.start().unwrap();
        out.extend(c.feed(&ramp(1_000..3_000)).unwrap());
        out.extend(c.stop().unwrap());
        assert_eq!(concat(&out), ramp(0..3_000));
    }

    #[test]
    fn onset_includes_450_ms_pre_roll() {
        let mut c = capture(16_000, CodeVad);
        c.start().unwrap();
        let mut codes = ramp(0..30);
        codes.extend([1_000.0, 1_001.0]);
        codes.extend(ramp(30..40));
        let mut out = c.feed(&frames(&codes)).unwrap();
        out.extend(c.stop().unwrap());
        assert_eq!(out.len(), 1);
        let first = out[0].samples();
        assert_eq!(first[0], 15.0, "o pre-roll começa 15 quadros antes da voz");
        assert_eq!(first[15 * FRAME_SAMPLES], 1_000.0);
        assert_eq!(first.len(), (15 + 2 + 10) * FRAME_SAMPLES);
    }

    #[test]
    fn pre_roll_stops_at_session_start() {
        let mut c = capture(16_000, CodeVad);
        c.start().unwrap();
        let mut codes = ramp(0..5);
        codes.extend([1_000.0, 1_001.0, 1_002.0]);
        let mut out = c.feed(&frames(&codes)).unwrap();
        out.extend(c.stop().unwrap());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].samples()[0], 0.0);
        assert_eq!(out[0].samples().len(), 8 * FRAME_SAMPLES);
    }

    #[test]
    fn single_voiced_frame_opens_nothing() {
        let mut c = capture(16_000, CodeVad);
        c.start().unwrap();
        let mut codes = ramp(0..10);
        codes.push(1_000.0);
        codes.extend(ramp(10..40));
        let mut out = c.feed(&frames(&codes)).unwrap();
        out.extend(c.stop().unwrap());
        assert!(out.is_empty());
    }

    #[test]
    fn silence_of_450_ms_closes_utterance() {
        let mut c = capture(16_000, CodeVad);
        c.start().unwrap();
        let voiced: Vec<f32> = (0..12).map(|i| 1_000.0 + i as f32).collect();
        assert!(c.feed(&frames(&voiced)).unwrap().is_empty());
        assert!(
            c.feed(&frames(&ramp(0..14))).unwrap().is_empty(),
            "14 quadros de silêncio ainda não fecham"
        );
        let closed = c.feed(&frames(&ramp(14..15))).unwrap();
        assert_eq!(
            closed.len(),
            1,
            "o 15º quadro de silêncio fecha a utterance"
        );
        assert_eq!(closed[0].samples().len(), (12 + 15) * FRAME_SAMPLES);
        assert!(c.is_recording(), "a sessão continua");
        assert!(c.stop().unwrap().is_empty());
    }

    #[test]
    fn continuous_speech_splits_at_15_s_without_loss() {
        let mut c = capture(16_000, Always(true));
        c.start().unwrap();
        let input = ramp(0..640_000);
        let mut out = Vec::new();
        for piece in input.chunks(160) {
            out.extend(c.feed(piece).unwrap());
        }
        out.extend(c.stop().unwrap());
        let lens: Vec<usize> = out.iter().map(|u| u.samples().len()).collect();
        assert_eq!(lens, vec![240_000, 240_000, 160_000]);
        assert_eq!(concat(&out), input);
    }

    #[test]
    fn stop_flushes_open_utterance_and_resampler() {
        let mut c = capture(48_000, Always(true));
        c.start().unwrap();
        let input: Vec<f32> = (0..96_300)
            .map(|i| ((i as f32) * 0.01).sin() * 0.5)
            .collect();
        let mut out = Vec::new();
        for piece in input.chunks(480) {
            out.extend(c.feed(piece).unwrap());
        }
        out.extend(c.stop().unwrap());
        assert_eq!(concat(&out).len(), 32_100);
    }

    #[test]
    fn no_voice_no_utterance() {
        let mut c = capture(16_000, Always(false));
        c.feed(&vec![0.0; 8_000]).unwrap();
        assert!(c.start().unwrap().is_empty());
        for _ in 0..20 {
            assert!(c.feed(&vec![0.1; 1_600]).unwrap().is_empty());
        }
        assert!(c.stop().unwrap().is_empty());
    }
}
