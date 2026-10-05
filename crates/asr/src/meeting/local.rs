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
            let transcript = self
                .transcriber
                .transcribe(&DictationAudio::new(window.to_vec()), self.language)?;
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

    /// Devolve "<canal> <n>" e anota o tamanho de cada janela que recebeu.
    struct FakeParakeet {
        seen: Arc<Mutex<Vec<usize>>>,
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
