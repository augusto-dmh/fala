//! O `Transcriber` tem a forma da door 2 e cabe num `Box<dyn Transcriber>`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use fala_asr::{AsrError, Transcriber};
use fala_core::{DictationAudio, Language, Transcript};

struct Stub;

impl Transcriber for Stub {
    fn transcribe(
        &mut self,
        audio: &DictationAudio,
        language: Language,
    ) -> Result<Transcript, AsrError> {
        Ok(Transcript {
            text: audio.samples().len().to_string(),
            language,
        })
    }
}

fn assert_send<T: Send + ?Sized>() {}

#[test]
fn stub_implements_transcriber() {
    assert_send::<Box<dyn Transcriber>>();
    let mut boxed: Box<dyn Transcriber> = Box::new(Stub);
    let t = boxed
        .transcribe(&DictationAudio::new(vec![0.0; 3]), Language::En)
        .unwrap();
    assert_eq!(t.text, "3");
    assert_eq!(t.language, Language::En);
    let load = AsrError::ModelLoad {
        path: "/x".into(),
        reason: "r".into(),
    };
    let inference = AsrError::Inference("falhou".into());
    assert!(matches!(load, AsrError::ModelLoad { .. }));
    assert!(matches!(inference, AsrError::Inference(_)));
}
