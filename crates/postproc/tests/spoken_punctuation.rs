//! A pontuação falada chega ao `Postprocessor` e o flag de `Rules` a desliga.

use fala_core::{AppContext, Dictionary, Editor, Language, Transcript};
use fala_postproc::{LlmConfig, Postprocessor, Rules};

fn final_text(processor: &Postprocessor, text: &str) -> (String, Editor) {
    let formatted = processor.process(
        Transcript {
            text: text.to_string(),
            language: Language::PtBr,
        },
        AppContext::default(),
        &Dictionary::default(),
    );
    (formatted.dictation.final_text, formatted.dictation.editor)
}

#[test]
fn postprocessor_honors_the_flag() {
    let on = Postprocessor::new(LlmConfig::default());
    assert_eq!(
        final_text(&on, "sim vírgula não"),
        ("Sim, não".to_string(), Editor::Rules)
    );

    let off = Postprocessor::new(LlmConfig::default()).with_rules(Rules {
        spoken_punctuation: false,
    });
    assert_eq!(
        final_text(&off, "sim vírgula não"),
        ("Sim vírgula não".to_string(), Editor::Rules)
    );
}
