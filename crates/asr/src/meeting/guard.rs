//! O guarda da ADR-0003 no caminho da reunião: só uma sessão de reunião vira entrada de um
//! backend de rede.

use std::path::{Path, PathBuf};

use fala_meeting::SessionMode;

use crate::AsrError;

/// Arquivo do canal do microfone dentro de `audio/<id>/`.
pub const MIC_FILE: &str = "mic.opus";
/// Arquivo do canal do sistema dentro de `audio/<id>/`.
pub const SYSTEM_FILE: &str = "sys.opus";

/// Os dois arquivos retidos de uma sessão de reunião e o modo dela. Só existe a partir da pasta
/// da sessão ([`MeetingRecording::from_session_dir`]); não há conversão de áudio de ditado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingRecording {
    mic: PathBuf,
    system: PathBuf,
    mode: SessionMode,
}

impl MeetingRecording {
    /// Lê `audio/<id>/`: exige `mic.opus` e `sys.opus`.
    pub fn from_session_dir(dir: &Path, mode: SessionMode) -> Result<Self, AsrError> {
        let channel = |name: &'static str| {
            let path = dir.join(name);
            if path.is_file() {
                Ok(path)
            } else {
                Err(AsrError::MissingChannel(name))
            }
        };
        Ok(Self {
            mic: channel(MIC_FILE)?,
            system: channel(SYSTEM_FILE)?,
            mode,
        })
    }

    pub fn mic(&self) -> &Path {
        &self.mic
    }

    pub fn system(&self) -> &Path {
        &self.system
    }

    pub fn mode(&self) -> SessionMode {
        self.mode
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use std::fs;

    use static_assertions::assert_not_impl_any;

    use super::*;
    use crate::meeting::{CancelToken, ElevenLabsScribe, MeetingTranscriber, Progress, Segment};
    use crate::Transcriber;

    type ScribeSend = fn(
        &mut ElevenLabsScribe,
        &MeetingRecording,
        &CancelToken,
        &mut dyn FnMut(Progress),
    ) -> Result<Vec<Segment>, AsrError>;

    #[test]
    fn scribe_is_not_a_dictation_transcriber() {
        assert_not_impl_any!(ElevenLabsScribe: Transcriber);
        // O envio da Scribe só aceita uma gravação de reunião.
        let _send: ScribeSend = <ElevenLabsScribe as MeetingTranscriber>::transcribe_session;
    }

    fn session_dir(name: &str, files: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("fala-asr-guard-{}", std::process::id()))
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for file in files {
            fs::write(dir.join(file), b"OggS").unwrap();
        }
        dir
    }

    #[test]
    fn from_session_dir_needs_both_channels() {
        let both = session_dir("both", &[MIC_FILE, SYSTEM_FILE]);
        let recording = MeetingRecording::from_session_dir(&both, SessionMode::InPerson).unwrap();
        assert_eq!(recording.mic(), both.join("mic.opus"));
        assert_eq!(recording.system(), both.join("sys.opus"));
        assert_eq!(recording.mode(), SessionMode::InPerson);

        let no_sys = session_dir("no-sys", &[MIC_FILE]);
        let err = MeetingRecording::from_session_dir(&no_sys, SessionMode::Meeting).unwrap_err();
        assert!(
            matches!(err, AsrError::MissingChannel("sys.opus")),
            "{err:?}"
        );
        assert!(err.to_string().contains("sys.opus"), "{err}");

        let no_mic = session_dir("no-mic", &[SYSTEM_FILE]);
        let err = MeetingRecording::from_session_dir(&no_mic, SessionMode::Meeting).unwrap_err();
        assert!(
            matches!(err, AsrError::MissingChannel("mic.opus")),
            "{err:?}"
        );
        assert!(err.to_string().contains("mic.opus"), "{err}");
    }
}
