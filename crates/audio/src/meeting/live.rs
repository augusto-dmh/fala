//! Prévia ao vivo da reunião (ADR-0019): um canal do WAV de trabalho (48 kHz, i16) entra, trechos
//! de fala de 16 kHz com o tempo em ms do canal saem. As regras de início e de silêncio são as do
//! ditado (`capture`); o teto é menor, para o texto não atrasar numa fala longa. Um ganho
//! automático por canal leva a fala baixa de uma call (o loopback chega a −54 dBFS) ao nível que o
//! VAD e o modelo esperam.

use super::wav::MEETING_RATE;
use crate::capture::Session;
use crate::resample::{Resampler, OUT_RATE};
use crate::vad::{VoiceDetector, FRAME_SAMPLES};
use crate::AudioError;

/// 8 s a 16 kHz: um trecho sem pausa é cortado aqui.
pub const LIVE_MAX_SAMPLES: usize = 128_000;

/// O pico que o ganho automático busca: −6 dBFS.
const AGC_TARGET: f32 = 0.5;
/// Ganho máximo, +40 dB: uma call a −54 dBFS chega ao alvo, e o chiado de fundo não vira fala.
const AGC_MAX_GAIN: f32 = 100.0;
/// Quanto o envelope cai por quadro de 30 ms sem pico maior: metade em ~2 s.
const AGC_RELEASE: f32 = 0.9897;

/// Ganho automático: ataque imediato (o pico de um quadro nunca passa do alvo, nada clipa) e
/// soltura lenta, para o nível não respirar entre as palavras.
#[derive(Default)]
struct Agc {
    level: f32,
}

impl Agc {
    fn apply(&mut self, samples: &mut [f32]) {
        let peak = samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
        self.level = (self.level * AGC_RELEASE).max(peak);
        let gain = AGC_TARGET / self.level.max(AGC_TARGET / AGC_MAX_GAIN);
        samples.iter_mut().for_each(|s| *s *= gain);
    }
}

/// Um trecho de fala de um canal: `[t0_ms, t1_ms)` desde o começo do canal, mono a 16 kHz.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveUtterance {
    pub t0_ms: u64,
    pub t1_ms: u64,
    pub samples: Vec<f32>,
}

pub struct LiveSegmenter {
    resampler: Resampler,
    vad: Box<dyn VoiceDetector>,
    session: Session,
    /// Amostras de 16 kHz já classificadas, em quadros inteiros.
    position: u64,
    /// O que ainda não forma um quadro.
    pending: Vec<f32>,
    input: Vec<f32>,
    /// O último quadro era silêncio digital: o estado do VAD é de antes dele.
    quiet: bool,
    agc: Agc,
}

impl LiveSegmenter {
    pub fn new(vad: Box<dyn VoiceDetector>) -> Result<Self, AudioError> {
        Ok(Self {
            resampler: Resampler::new(MEETING_RATE)?,
            vad,
            session: Session::default(),
            position: 0,
            pending: Vec::new(),
            input: Vec::new(),
            quiet: false,
            agc: Agc::default(),
        })
    }

    /// Os frames seguintes do canal; devolve os trechos que fecharam.
    pub fn feed(&mut self, frames: &[i16]) -> Result<Vec<LiveUtterance>, AudioError> {
        self.input.clear();
        self.input
            .extend(frames.iter().map(|s| f32::from(*s) / 32_768.0));
        self.resampler.push(&self.input, &mut self.pending)?;
        Ok(self.process())
    }

    /// O fim do canal: esvazia o filtro e entrega o trecho aberto, se houver.
    pub fn finish(&mut self) -> Result<Vec<LiveUtterance>, AudioError> {
        self.resampler.finish(&mut self.pending)?;
        let mut done = self.process();
        if let Some(mut samples) = self.session.take_open() {
            // Os quadros já contam em `position`; o resto que não formava quadro, não.
            let end = self.position + self.pending.len() as u64;
            self.agc.apply(&mut self.pending);
            samples.append(&mut self.pending);
            done.push(utterance(end, samples));
        }
        self.pending.clear();
        Ok(done)
    }

    /// O VAD ouviu voz e o trecho ainda não fechou.
    pub fn speaking(&self) -> bool {
        self.session.speaking()
    }

    fn process(&mut self) -> Vec<LiveUtterance> {
        let full = self.pending.len() / FRAME_SAMPLES * FRAME_SAMPLES;
        let frames: Vec<f32> = self.pending.drain(..full).collect();
        let mut done = Vec::new();
        for frame in frames.as_chunks::<FRAME_SAMPLES>().0 {
            let mut frame = *frame;
            // Silêncio digital (nada tocando, ou o mic zerado por um ditado) não passa pelo VAD;
            // o som que volta depois dele começa com o VAD zerado.
            let silent = frame.iter().all(|s| *s == 0.0);
            if !silent {
                self.agc.apply(&mut frame);
            }
            if !silent && std::mem::replace(&mut self.quiet, false) {
                self.vad.reset();
            }
            self.quiet = silent;
            // Um erro do VAD conta como voz, como no ditado: o trecho segue, sem quadro repetido.
            let voice = !silent
                && self.vad.is_voice(&frame).unwrap_or_else(|e| {
                    log::warn!("prévia: VAD falhou num quadro: {e}");
                    true
                });
            self.position += FRAME_SAMPLES as u64;
            if let Some((samples, _)) = self.session.step(&frame, voice, LIVE_MAX_SAMPLES) {
                done.push(utterance(self.position, samples));
            }
        }
        done
    }
}

/// O trecho que termina na amostra `end` (16 kHz).
fn utterance(end: u64, samples: Vec<f32>) -> LiveUtterance {
    let ms = |n: u64| n * 1_000 / u64::from(OUT_RATE);
    LiveUtterance {
        t0_ms: ms(end.saturating_sub(samples.len() as u64)),
        t1_ms: ms(end),
        samples,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;

    /// Voz quando o quadro tem energia; conta as chamadas.
    struct LoudVad(Arc<AtomicUsize>);

    impl VoiceDetector for LoudVad {
        fn is_voice(&mut self, frame: &[f32]) -> Result<bool, AudioError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(frame.iter().map(|s| s.abs()).sum::<f32>() / frame.len() as f32 > 0.1)
        }
        fn reset(&mut self) {}
    }

    fn segmenter() -> (LiveSegmenter, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let vad = LoudVad(Arc::clone(&calls));
        (LiveSegmenter::new(Box::new(vad)).unwrap(), calls)
    }

    /// `ms` de 48 kHz de um tom de 1 kHz com pico `peak`.
    fn tone(ms: usize, peak: i16) -> Vec<i16> {
        (0..ms * 48)
            .map(|i| if (i / 24) % 2 == 0 { peak } else { -peak })
            .collect()
    }

    /// `ms` de 48 kHz: voz é um tom alto, silêncio é zero digital.
    fn audio(ms: usize, voice: bool) -> Vec<i16> {
        (0..ms * 48)
            .map(|i| {
                if voice && (i / 24) % 2 == 0 {
                    16_000
                } else if voice {
                    -16_000
                } else {
                    0
                }
            })
            .collect()
    }

    /// Alimenta em lotes de 100 ms, como o gravador.
    fn feed(seg: &mut LiveSegmenter, frames: &[i16]) -> Vec<LiveUtterance> {
        let mut out = Vec::new();
        for chunk in frames.chunks(4_800) {
            out.extend(seg.feed(chunk).unwrap());
        }
        out
    }

    #[test]
    fn a_phrase_between_silences_is_one_utterance_with_its_time() {
        let (mut seg, _) = segmenter();
        let mut input = audio(1_000, false);
        input.extend(audio(2_000, true));
        input.extend(audio(1_000, false));
        let done = feed(&mut seg, &input);
        assert_eq!(done.len(), 1, "{done:?}");
        let u = &done[0];
        // Até 510 ms de pre-roll antes da voz; o silêncio que fecha (450 ms) vem junto.
        assert!((450..=1_000).contains(&u.t0_ms), "t0 {}", u.t0_ms);
        assert!((3_000..=3_520).contains(&u.t1_ms), "t1 {}", u.t1_ms);
        assert_eq!(u.samples.len() as u64, (u.t1_ms - u.t0_ms) * 16);
        assert!(!seg.speaking());
        assert!(seg.finish().unwrap().is_empty());
    }

    #[test]
    fn long_speech_is_cut_at_the_cap_and_the_rest_comes_at_the_end() {
        let (mut seg, _) = segmenter();
        let done = feed(&mut seg, &audio(20_000, true));
        assert_eq!(done.len(), 2, "{done:?}");
        // O corte vem no primeiro quadro inteiro que alcança o teto.
        let len = done[0].samples.len();
        assert!((LIVE_MAX_SAMPLES..LIVE_MAX_SAMPLES + FRAME_SAMPLES).contains(&len));
        assert_eq!(done[0].t1_ms, done[1].t0_ms, "trechos seguidos");
        assert!(seg.speaking(), "a fala continua aberta");
        let rest = seg.finish().unwrap();
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].t0_ms, done[1].t1_ms);
        assert_eq!(rest[0].t1_ms, 20_000);
    }

    #[test]
    fn the_cap_in_a_pause_ends_the_stretch_without_a_silent_one() {
        let (mut seg, _) = segmenter();
        // 7,75 s de voz: o teto (8 s) cai nos primeiros 300 ms da pausa.
        let mut input = audio(7_750, true);
        input.extend(audio(2_000, false));
        let mut done = feed(&mut seg, &input);
        done.extend(seg.finish().unwrap());
        assert_eq!(done.len(), 1, "{done:?}");
        assert_eq!(done[0].t0_ms, 0);
    }

    #[test]
    fn a_quiet_call_is_raised_to_speech_level_without_clipping() {
        let (mut seg, _) = segmenter();
        // −80 dBFS de chiado, depois fala a −50 dBFS (pico 104 de 32 768), depois silêncio.
        let mut input = tone(1_000, 3);
        input.extend(tone(2_000, 104));
        input.extend(audio(1_000, false));
        let done = feed(&mut seg, &input);
        assert_eq!(done.len(), 1, "{done:?}");
        assert!(
            done[0].t0_ms >= 450,
            "o chiado não vira fala: {}",
            done[0].t0_ms
        );
        let peak = done[0].samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
        assert!((0.3..=AGC_TARGET + 1e-6).contains(&peak), "pico {peak}");
    }

    #[test]
    fn digital_silence_never_reaches_the_vad() {
        let (mut seg, calls) = segmenter();
        assert!(feed(&mut seg, &audio(5_000, false)).is_empty());
        assert!(seg.finish().unwrap().is_empty());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn speaking_turns_on_while_the_voice_lasts() {
        let (mut seg, _) = segmenter();
        feed(&mut seg, &audio(500, false));
        assert!(!seg.speaking());
        feed(&mut seg, &audio(500, true));
        assert!(seg.speaking());
    }
}
