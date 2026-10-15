//! Fallback local (ADR-0005): offline ou "só local", cada canal passa pelo Parakeet em janelas
//! de 60 s e sai na mesma forma de [`Segment`]. Nada aqui abre conexão.

use std::path::Path;

use fala_core::{DictationAudio, Language};

use super::segments::{assign_speakers, ChannelSegment};
use super::{CancelToken, Channel, MeetingRecording, MeetingTranscriber, Progress, Segment, Stage};
use crate::{AsrError, Transcriber};

/// Duração de cada janela local.
pub const WINDOW_MS: u64 = 60_000;
/// Taxa que o Parakeet espera.
const SAMPLE_RATE_HZ: u64 = 16_000;
const WINDOW_SAMPLES: usize = (WINDOW_MS * SAMPLE_RATE_HZ / 1_000) as usize;
/// Pico abaixo do qual a janela é silêncio e nem vai ao modelo (−60 dBFS).
const SILENCE_PEAK: f32 = 1e-3;
/// O pico que cada janela passa a ter (−6 dBFS).
const TARGET_PEAK: f32 = 0.5;
/// Ganho máximo de uma janela (+40 dB): o pico de uma call baixa (−44 dBFS medido) chega perto
/// do alvo, e o chiado de um canal quieto não é empurrado até virar "fala" num Whisper.
const MAX_GAIN: f32 = 100.0;

/// A janela com o pico em [`TARGET_PEAK`], sem passar de [`MAX_GAIN`] e sem atenuar (o pico
/// nunca passa de 1, então nada clipa); `None` para uma janela em silêncio. O canal do sistema de
/// uma call chega baixo (−54 dBFS de mediana medido numa reunião real) e o Parakeet o tomaria
/// por silêncio.
fn normalized(window: &[f32]) -> Option<Vec<f32>> {
    let peak = window.iter().fold(0.0_f32, |max, s| max.max(s.abs()));
    if peak < SILENCE_PEAK {
        return None;
    }
    let gain = (TARGET_PEAK / peak).clamp(1.0, MAX_GAIN);
    Some(window.iter().map(|s| s * gain).collect())
}

/// Um [`Transcriber`] local (o Parakeet) aplicado a uma sessão de reunião, em janelas de 60 s.
/// Sem diarização: cada canal tem um falante só.
pub struct LocalMeeting<T, D> {
    transcriber: T,
    decode: D,
    language: Language,
}

impl<T, D> LocalMeeting<T, D>
where
    T: Transcriber,
    D: FnMut(&Path) -> Result<Vec<f32>, AsrError> + Send,
{
    /// `decode` lê o arquivo de um canal e devolve mono f32 a 16 kHz; quem grava o Opus (2.F3)
    /// fornece a leitura. `language` só rotula a chamada ao modelo.
    pub fn new(transcriber: T, decode: D, language: Language) -> Self {
        Self {
            transcriber,
            decode,
            language,
        }
    }

    fn channel(
        &mut self,
        channel: Channel,
        path: &Path,
        cancel: &CancelToken,
        on_progress: &mut dyn FnMut(Progress),
    ) -> Result<Vec<ChannelSegment>, AsrError> {
        if cancel.is_cancelled() {
            return Err(AsrError::Cancelled);
        }
        let samples = (self.decode)(path)?;
        let total_ms = samples_to_ms(samples.len());
        let mut out = Vec::new();
        for (index, window) in samples.chunks(WINDOW_SAMPLES).enumerate() {
            if cancel.is_cancelled() {
                return Err(AsrError::Cancelled);
            }
            let t0_ms = index as u64 * WINDOW_MS;
            on_progress(Progress {
                channel,
                stage: Stage::Transcribing {
                    done_ms: t0_ms,
                    total_ms,
                },
            });
            let Some(window_audio) = normalized(window) else {
                continue;
            };
            let transcript = self
                .transcriber
                .transcribe(&DictationAudio::new(window_audio), self.language)?;
            let text = transcript.text.trim();
            if !text.is_empty() {
                out.push(ChannelSegment {
                    label: None,
                    t0_ms,
                    t1_ms: t0_ms + samples_to_ms(window.len()),
                    text: text.to_string(),
                });
            }
        }
        Ok(out)
    }
}

impl<T, D> MeetingTranscriber for LocalMeeting<T, D>
where
    T: Transcriber,
    D: FnMut(&Path) -> Result<Vec<f32>, AsrError> + Send,
{
    fn transcribe_session(
        &mut self,
        recording: &MeetingRecording,
        cancel: &CancelToken,
        on_progress: &mut dyn FnMut(Progress),
    ) -> Result<Vec<Segment>, AsrError> {
        let mode = recording.mode();
        let mic = if mode.has_mic() {
            self.channel(Channel::Mic, recording.mic(), cancel, on_progress)?
        } else {
            Vec::new()
        };
        let system = self.channel(Channel::System, recording.system(), cancel, on_progress)?;
        Ok(assign_speakers(mode, mic, system))
    }
}

fn samples_to_ms(samples: usize) -> u64 {
    samples as u64 * 1_000 / SAMPLE_RATE_HZ
}

/// Por onde uma sessão é transcrita.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeetingRoute {
    /// Só a nuvem; uma falha volta para quem chamou.
    Cloud,
    /// A nuvem e, se ela falhar com [`AsrError::Network`], o caminho local.
    CloudThenLocal,
    /// O ajuste "só local": a nuvem não é chamada.
    LocalOnly,
}

/// Transcreve pela rota pedida. Em [`MeetingRoute::CloudThenLocal`], qualquer
/// [`AsrError::Network`] da nuvem (sem rede, `429`, `5xx`, chave recusada) cai no local; os
/// outros erros, como [`AsrError::Cancelled`], voltam como estão.
pub fn transcribe_with_fallback(
    route: MeetingRoute,
    cloud: &mut dyn MeetingTranscriber,
    local: &mut dyn MeetingTranscriber,
    recording: &MeetingRecording,
    cancel: &CancelToken,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<Vec<Segment>, AsrError> {
    match route {
        MeetingRoute::LocalOnly => local.transcribe_session(recording, cancel, on_progress),
        MeetingRoute::Cloud => cloud.transcribe_session(recording, cancel, on_progress),
        MeetingRoute::CloudThenLocal => {
            match cloud.transcribe_session(recording, cancel, on_progress) {
                Err(AsrError::Network { .. }) => {
                    local.transcribe_session(recording, cancel, on_progress)
                }
                other => other,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use std::num::NonZeroU32;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use fala_core::Transcript;
    use fala_meeting::SessionMode;

    use super::*;
    use crate::meeting::{Speaker, MIC_FILE, SYSTEM_FILE};

    /// Devolve "<canal> <n>" e anota o tamanho e o pico de cada janela que recebeu.
    struct FakeParakeet {
        seen: Arc<Mutex<Vec<usize>>>,
        peaks: Arc<Mutex<Vec<f32>>>,
        calls: usize,
    }

    impl Transcriber for FakeParakeet {
        fn transcribe(
            &mut self,
            audio: &DictationAudio,
            language: Language,
        ) -> Result<Transcript, AsrError> {
            self.calls += 1;
            self.seen.lock().unwrap().push(audio.samples().len());
            let peak = audio.samples().iter().fold(0.0_f32, |m, s| m.max(s.abs()));
            self.peaks.lock().unwrap().push(peak);
            Ok(Transcript {
                text: format!(" janela {} ", self.calls),
                language,
            })
        }
    }

    fn session(name: &str, mode: SessionMode) -> MeetingRecording {
        let dir: PathBuf = std::env::temp_dir()
            .join(format!("fala-asr-local-{}", std::process::id()))
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(MIC_FILE), b"OggS-mic").unwrap();
        std::fs::write(dir.join(SYSTEM_FILE), b"OggS-sys").unwrap();
        MeetingRecording::from_session_dir(&dir, mode).unwrap()
    }

    type Decode = Box<dyn FnMut(&Path) -> Result<Vec<f32>, AsrError> + Send>;

    /// Um "decodificador" que devolve `secs` segundos a 16 kHz para qualquer canal.
    fn seconds(secs: usize) -> Decode {
        Box::new(move |_| Ok(vec![0.01; secs * 16_000]))
    }

    fn fake_local(secs: usize) -> (LocalMeeting<FakeParakeet, Decode>, Arc<Mutex<Vec<usize>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let fake = FakeParakeet {
            seen: Arc::clone(&seen),
            peaks: Arc::default(),
            calls: 0,
        };
        (LocalMeeting::new(fake, seconds(secs), Language::PtBr), seen)
    }

    #[test]
    fn windows_of_60_s_shift_timestamps() {
        let (mut local, seen) = fake_local(130);
        let recording = session("windows", SessionMode::SystemOnly);
        let mut stages = Vec::new();
        let segments = local
            .transcribe_session(&recording, &CancelToken::new(), &mut |p: Progress| {
                stages.push(p.stage)
            })
            .unwrap();
        assert_eq!(*seen.lock().unwrap(), [960_000, 960_000, 160_000]);
        let got: Vec<_> = segments
            .iter()
            .map(|s| (s.channel, s.t0_ms, s.t1_ms, s.text.as_str()))
            .collect();
        assert_eq!(
            got,
            [
                (Channel::System, 0, 60_000, "janela 1"),
                (Channel::System, 60_000, 120_000, "janela 2"),
                (Channel::System, 120_000, 130_000, "janela 3"),
            ]
        );
        assert_eq!(stages.len(), 3);

        // Com mic (modo `meeting`): o mic é `me`, o sistema é a pessoa 1, e o merge intercala.
        let (mut local, _) = fake_local(70);
        let recording = session("windows-meeting", SessionMode::Meeting);
        let segments = local
            .transcribe_session(&recording, &CancelToken::new(), &mut |_| {})
            .unwrap();
        let person1 = Speaker::Person(NonZeroU32::MIN);
        let got: Vec<_> = segments
            .iter()
            .map(|s| (s.channel, s.speaker, s.t0_ms))
            .collect();
        assert_eq!(
            got,
            [
                (Channel::Mic, Speaker::Me, 0),
                (Channel::System, person1, 0),
                (Channel::Mic, Speaker::Me, 60_000),
                (Channel::System, person1, 60_000),
            ]
        );
    }

    #[test]
    fn quiet_windows_are_raised_and_silent_ones_skipped() {
        // Uma call a −50 dBFS (pico 0,00316) e, depois, 60 s de silêncio digital.
        let quiet = 10_f32.powf(-50.0 / 20.0);
        let mut samples: Vec<f32> = (0..WINDOW_SAMPLES)
            .map(|i| quiet * (i as f32 * 0.05).sin())
            .collect();
        samples.extend(std::iter::repeat_n(0.0, WINDOW_SAMPLES));
        let peaks = Arc::new(Mutex::new(Vec::new()));
        let fake = FakeParakeet {
            seen: Arc::default(),
            peaks: Arc::clone(&peaks),
            calls: 0,
        };
        let decode: Decode = Box::new(move |_| Ok(samples.clone()));
        let mut local = LocalMeeting::new(fake, decode, Language::PtBr);
        let recording = session("quiet", SessionMode::SystemOnly);
        let segments = local
            .transcribe_session(&recording, &CancelToken::new(), &mut |_| {})
            .unwrap();
        let peaks = peaks.lock().unwrap().clone();
        assert_eq!(peaks.len(), 1, "a janela em silêncio não vai ao modelo");
        // +40 dB: de −50 para −10 dBFS, longe do silêncio que o modelo ignoraria.
        assert!((peaks[0] - quiet * MAX_GAIN).abs() < 0.01, "{peaks:?}");
        assert_eq!(segments.len(), 1);

        // Alto já está bom: nada é atenuado nem clipa; o chiado não ganha mais de 50 dB.
        let loud = normalized(&[0.9, -0.2]).unwrap();
        assert_eq!(loud, [0.9, -0.2]);
        let hiss = normalized(&[0.002, -0.002]).unwrap();
        assert!((hiss[0] - 0.002 * MAX_GAIN).abs() < 1e-4, "{hiss:?}");
        assert_eq!(
            normalized(&[0.0004; 4]),
            None,
            "−68 dBFS de chiado fica fora"
        );
    }

    /// Uma nuvem que sempre falha com o erro dado e conta as chamadas.
    struct FailingCloud {
        error: fn() -> AsrError,
        calls: usize,
    }

    impl MeetingTranscriber for FailingCloud {
        fn transcribe_session(
            &mut self,
            _: &MeetingRecording,
            _: &CancelToken,
            _: &mut dyn FnMut(Progress),
        ) -> Result<Vec<Segment>, AsrError> {
            self.calls += 1;
            Err((self.error)())
        }
    }

    fn offline() -> AsrError {
        AsrError::Network {
            retriable: true,
            status: None,
            reason: "sem rede".to_string(),
        }
    }

    #[test]
    fn network_error_falls_back_to_local() {
        let recording = session("fallback", SessionMode::Meeting);
        let mut cloud = FailingCloud {
            error: offline,
            calls: 0,
        };
        let (mut local, seen) = fake_local(30);
        let segments = transcribe_with_fallback(
            MeetingRoute::CloudThenLocal,
            &mut cloud,
            &mut local,
            &recording,
            &CancelToken::new(),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(cloud.calls, 1);
        assert_eq!(seen.lock().unwrap().len(), 2, "mic e sistema pelo local");
        let got: Vec<_> = segments
            .iter()
            .map(|s| (s.channel, s.t0_ms, s.t1_ms))
            .collect();
        assert_eq!(
            got,
            [(Channel::Mic, 0, 30_000), (Channel::System, 0, 30_000)]
        );

        // Sem pedir o fallback, o erro de rede volta.
        let (mut local, seen) = fake_local(30);
        let err = transcribe_with_fallback(
            MeetingRoute::Cloud,
            &mut cloud,
            &mut local,
            &recording,
            &CancelToken::new(),
            &mut |_| {},
        )
        .unwrap_err();
        assert!(matches!(err, AsrError::Network { .. }), "{err:?}");
        assert!(seen.lock().unwrap().is_empty());

        // Cancelar não é erro de rede: não cai no local.
        let mut cancelled = FailingCloud {
            error: || AsrError::Cancelled,
            calls: 0,
        };
        let err = transcribe_with_fallback(
            MeetingRoute::CloudThenLocal,
            &mut cancelled,
            &mut local,
            &recording,
            &CancelToken::new(),
            &mut |_| {},
        )
        .unwrap_err();
        assert!(matches!(err, AsrError::Cancelled), "{err:?}");
        assert!(seen.lock().unwrap().is_empty());
    }
}
