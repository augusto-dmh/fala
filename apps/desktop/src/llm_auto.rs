//! The automatic dictation LLM (ADR-0004): every `transcribe` dictation goes through the
//! `fala-postproc` `Postprocessor`. The local rules always run; the Gemini formats the text when
//! the LLM is on, the `gemini` key is set, the app is not in `llm_disabled_apps` and the text has
//! more than 15 words. An unknown app keeps the LLM on (D4 of the phase 1 plan).
//!
//! `custom_words` is the personal dictionary: the rules apply its spelling and the Gemini gets it
//! in the prompt. While the LLM is configured, the inherited fuzzy correction waits until the
//! LLM is out of the way and only touches text the LLM did not write.

use std::panic::{catch_unwind, AssertUnwindSafe};

use fala_core::{AppContext, Dictionary, Editor, Language, Transcript};
use fala_postproc::{
    FormatContext, Formatter, Gemini, LlmConfig, Postprocessor, Rules, DEFAULT_BASE_URL,
};
use fala_secrets::ApiKey;
use log::{debug, error};

use crate::audio_toolkit::apply_custom_words;
use crate::settings::{AppSettings, GEMINI_PROVIDER_ID};

/// The text to paste and whether the LLM produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AutoFormatted {
    pub final_text: String,
    pub llm_produced: bool,
}

/// The `Postprocessor` the settings describe, the personal dictionary and the fuzzy correction
/// left for text the LLM does not write.
pub(crate) struct AutoFormatter {
    postprocessor: Postprocessor,
    dictionary: Dictionary,
    fuzzy: Option<Fuzzy>,
}

/// The formatter for one dictation. `use_llm = false` keeps the LLM off for it. The key comes
/// from `post_process_api_keys`, which the settings load fills from the OS keyring.
pub(crate) fn formatter(settings: &AppSettings, use_llm: bool) -> AutoFormatter {
    formatter_at(settings, use_llm, DEFAULT_BASE_URL)
}

fn formatter_at(settings: &AppSettings, use_llm: bool, base_url: &str) -> AutoFormatter {
    let gemini = gemini_key(settings).map(|key| Gemini::new(key).with_base_url(base_url));
    AutoFormatter {
        postprocessor: Postprocessor::new(LlmConfig {
            enabled: settings.llm_enabled && use_llm,
            gemini,
            disabled_apps: settings.llm_disabled_apps.clone(),
        }),
        dictionary: Dictionary::new(&settings.custom_words),
        fuzzy: Fuzzy::deferred(settings),
    }
}

fn gemini_key(settings: &AppSettings) -> Option<ApiKey> {
    settings
        .post_process_api_keys
        .get(GEMINI_PROVIDER_ID)
        .and_then(|key| ApiKey::new(key.clone()).ok())
}

/// The LLM can format dictations: it is on and the `gemini` key is set. While it can, the
/// transcription step leaves the fuzzy correction to the formatter, so the LLM reads what the
/// ASR heard.
pub(crate) fn llm_configured(settings: &AppSettings) -> bool {
    settings.llm_enabled && gemini_key(settings).is_some()
}

/// The inherited fuzzy correction (`apply_custom_words`) the transcription step skipped.
#[derive(Debug, Clone)]
pub(crate) struct Fuzzy {
    words: Vec<String>,
    threshold: f64,
}

impl Fuzzy {
    /// `Some` only when the transcription step skipped the correction because the LLM is
    /// configured, so it never runs twice.
    pub(crate) fn deferred(settings: &AppSettings) -> Option<Self> {
        (llm_configured(settings) && !settings.custom_words.is_empty()).then(|| Self {
            words: settings.custom_words.clone(),
            threshold: settings.word_correction_threshold,
        })
    }

    /// Like the transcription step, a panic in the matcher keeps the text untouched.
    pub(crate) fn apply(&self, text: String) -> String {
        match catch_unwind(AssertUnwindSafe(|| {
            apply_custom_words(&text, &self.words, self.threshold)
        })) {
            Ok(corrected) => corrected,
            Err(_) => {
                error!("Custom-word correction panicked; keeping the text");
                text
            }
        }
    }
}

/// Formats one dictation. Blocks for at most `fala_postproc::INSERT_DEADLINE` when the LLM is
/// asked; past that, the rules text stays.
pub(crate) fn format(
    formatter: &AutoFormatter,
    text: &str,
    language: Language,
    app: AppContext,
) -> AutoFormatted {
    let raw = Transcript {
        text: text.to_string(),
        language,
    };
    let formatted = formatter
        .postprocessor
        .process(raw, app, &formatter.dictionary);
    let editor = formatted.dictation.editor;
    debug!("post-processing: editor={editor:?}");
    let llm_produced = editor == Editor::Llm;
    let dictation = formatted.dictation;
    // Text the LLM did not write goes through the fuzzy correction and then the rules, the
    // order it had before the LLM: the fuzzy matcher reads ASR text, and the rules fix the
    // spelling after it.
    let refixed = match &formatter.fuzzy {
        Some(fuzzy) if !llm_produced => {
            let ctx = FormatContext {
                app: &dictation.app,
                dictionary: &formatter.dictionary,
                language: &dictation.raw.language,
            };
            Rules
                .format(&fuzzy.apply(dictation.raw.text.clone()), &ctx)
                .ok()
        }
        _ => None,
    };
    let final_text = refixed.unwrap_or(dictation.final_text);
    AutoFormatted {
        final_text,
        llm_produced,
    }
}

/// The personal dictionary as stored: `Dictionary::new` trims, drops blanks and keeps the first
/// spelling of terms that differ only in case.
pub(crate) fn normalize_words(words: Vec<String>) -> Vec<String> {
    Dictionary::new(words).terms().to_vec()
}

/// The disabled-apps list as stored: each item reduced to the app name `fala-inject` reports,
/// blanks and repeats dropped, order kept.
pub(crate) fn normalize_apps(apps: Vec<String>) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for app in apps {
        if let Some(name) = fala_inject::app_name_from_exe_path(app.trim()) {
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::auto_processed;
    use crate::managers::history_dictations::dictation_for;
    use fala_postproc::SYSTEM_PROMPT;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    /// 16 words after the rules, which drop the leading filler.
    const SIXTEEN: &str =
        "hã eu acho que a gente pode mandar o relatório amanhã cedo para o time inteiro hoje";
    const SIXTEEN_RULES: &str =
        "Eu acho que a gente pode mandar o relatório amanhã cedo para o time inteiro hoje";
    /// 15 words after the rules.
    const FIFTEEN: &str =
        "eu acho que a gente pode mandar o relatório amanhã cedo para o time todo";
    const LLM_TEXT: &str = "Acho que dá para mandar o relatório amanhã cedo.";

    /// A Gemini on `127.0.0.1` that answers every request after `delay` and keeps the bodies.
    struct FakeGemini {
        base_url: String,
        bodies: Arc<Mutex<Vec<String>>>,
    }

    impl FakeGemini {
        fn start(delay: Duration, status: u16, body: String) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let base_url = format!("http://{}", listener.local_addr().unwrap());
            let bodies = Arc::new(Mutex::new(Vec::new()));
            let seen = Arc::clone(&bodies);
            thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(mut stream) = stream else { return };
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut length = 0;
                    loop {
                        let mut line = String::new();
                        if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                            break;
                        }
                        if let Some((name, value)) = line.split_once(':') {
                            if name.eq_ignore_ascii_case("content-length") {
                                length = value.trim().parse().unwrap_or(0);
                            }
                        }
                    }
                    let mut request = vec![0; length];
                    let _ = reader.read_exact(&mut request);
                    seen.lock()
                        .unwrap()
                        .push(String::from_utf8_lossy(&request).into_owned());
                    thread::sleep(delay);
                    let _ = write!(
                        stream,
                        "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                }
            });
            Self { base_url, bodies }
        }

        fn ok(delay: Duration) -> Self {
            let body = serde_json::json!({
                "candidates": [{ "content": { "parts": [{ "text": LLM_TEXT }] } }]
            });
            Self::start(delay, 200, body.to_string())
        }

        fn requests(&self) -> usize {
            self.bodies.lock().unwrap().len()
        }
    }

    fn settings_with_key() -> AppSettings {
        let mut settings = crate::settings::get_default_settings();
        settings
            .post_process_api_keys
            .insert(GEMINI_PROVIDER_ID.to_string(), "chave-de-teste".to_string());
        settings
    }

    fn app(name: &str) -> AppContext {
        AppContext {
            app_name: Some(name.to_string()),
        }
    }

    fn run(
        settings: &AppSettings,
        server: &FakeGemini,
        text: &str,
        app: AppContext,
    ) -> AutoFormatted {
        format(
            &formatter_at(settings, true, &server.base_url),
            text,
            Language::PtBr,
            app,
        )
    }

    /// The `systemInstruction` text of request `index`.
    fn system_prompt(server: &FakeGemini, index: usize) -> String {
        let body: serde_json::Value =
            serde_json::from_str(&server.bodies.lock().unwrap()[index]).unwrap();
        body["systemInstruction"]["parts"][0]["text"]
            .as_str()
            .unwrap()
            .to_string()
    }

    fn words(terms: &[&str]) -> Vec<String> {
        terms.iter().map(|term| term.to_string()).collect()
    }

    #[test]
    fn dictionary_reaches_the_llm_prompt() {
        let server = FakeGemini::ok(Duration::ZERO);
        let mut settings = settings_with_key();
        settings.custom_words = words(&[" Augusto ", "augusto", "ChargeBee"]);

        let auto = run(&settings, &server, SIXTEEN, app("notepad"));

        assert!(auto.llm_produced);
        assert_eq!(server.requests(), 1);
        assert_eq!(
            system_prompt(&server, 0),
            format!("{SYSTEM_PROMPT}\n\nDicionário pessoal:\n- Augusto\n- ChargeBee")
        );
    }

    #[test]
    fn empty_dictionary_keeps_the_prompt() {
        let server = FakeGemini::ok(Duration::ZERO);
        for (index, terms) in [words(&[]), words(&["", "  "])].into_iter().enumerate() {
            let mut settings = settings_with_key();
            settings.custom_words = terms;
            let auto = run(&settings, &server, SIXTEEN, app("notepad"));
            assert!(auto.llm_produced, "case {index}");
            assert_eq!(system_prompt(&server, index), SYSTEM_PROMPT, "case {index}");
        }
    }

    /// What the pipeline gives without the LLM configured: the fuzzy correction in the
    /// transcription step, then the rules.
    fn without_llm(terms: &[&str], text: &str) -> String {
        let mut keyless = crate::settings::get_default_settings();
        keyless.custom_words = words(terms);
        let corrected = apply_custom_words(
            text,
            &keyless.custom_words,
            keyless.word_correction_threshold,
        );
        let server = FakeGemini::ok(Duration::ZERO);
        run(&keyless, &server, &corrected, app("notepad")).final_text
    }

    #[test]
    fn rules_apply_the_dictionary_without_llm() {
        let server = FakeGemini::ok(Duration::ZERO);
        // 16 words after the rules, which join "charge bee" into one; without "hoje", 15.
        let long =
            "a charge bee mandou o relatório amanhã cedo para o time inteiro de vendas da empresa hoje";
        let fifteen =
            "a charge bee mandou o relatório amanhã cedo para o time inteiro de vendas da empresa";

        let mut off = settings_with_key();
        off.llm_enabled = false;
        let mut keyless = crate::settings::get_default_settings();
        keyless.llm_enabled = true;
        for (case, mut settings) in [("off", off), ("keyless", keyless)] {
            settings.custom_words = words(&["ChargeBee"]);
            let auto = run(&settings, &server, long, app("notepad"));
            assert!(
                auto.final_text.starts_with("A ChargeBee mandou"),
                "{case}: {}",
                auto.final_text
            );
            assert!(!auto.llm_produced, "{case}");
        }

        // Configured but not asked: the deferred fuzzy runs before the rules, so the text is
        // the one the pipeline gives without the LLM.
        let mut configured = settings_with_key();
        configured.custom_words = words(&["ChargeBee"]);
        for (case, text, app) in [
            ("disabled app", long, app("keepassxc")),
            ("15 words", fifteen, app("notepad")),
        ] {
            let auto = run(&configured, &server, text, app);
            assert_eq!(auto.final_text, without_llm(&["ChargeBee"], text), "{case}");
            assert!(auto.final_text.contains("ChargeBee mandou"), "{case}");
            assert!(!auto.llm_produced, "{case}");
        }
        assert_eq!(server.requests(), 0);
    }

    #[test]
    fn deferred_fuzzy_matches_the_pipeline_without_llm() {
        let terms = ["ChargeBee", "Augusto"];
        let mut configured = settings_with_key();
        configured.custom_words = words(&terms);
        let server = FakeGemini::ok(Duration::ZERO);
        for text in [
            "a charge bee mandou isso",
            "o augusto mandou isso",
            "o agusto mandou isso",
        ] {
            let auto = run(&configured, &server, text, app("notepad"));
            assert_eq!(auto.final_text, without_llm(&terms, text), "{text}");
            assert!(!auto.final_text.contains("CHARGEBEE"), "{text}");
            assert!(!auto.final_text.contains("AUGUSTO"), "{text}");
        }
        assert_eq!(server.requests(), 0);
    }

    #[test]
    fn rules_only_retry_keeps_the_deferred_fuzzy() {
        let server = FakeGemini::ok(Duration::ZERO);
        let mut settings = settings_with_key();
        settings.custom_words = words(&["Augusto"]);
        let text =
            "o agusto acho que pode mandar o relatório amanhã cedo para o time inteiro todo hoje";

        let auto = format(
            &formatter_at(&settings, false, &server.base_url),
            text,
            Language::PtBr,
            AppContext::default(),
        );

        assert!(
            auto.final_text.starts_with("O Augusto acho"),
            "{}",
            auto.final_text
        );
        assert!(!auto.llm_produced);
        assert_eq!(server.requests(), 0);
    }

    #[test]
    fn llm_text_is_not_fuzzy_corrected() {
        let answer = "O agusto mandou o relatório.";
        let body = serde_json::json!({
            "candidates": [{ "content": { "parts": [{ "text": answer }] } }]
        });
        let server = FakeGemini::start(Duration::ZERO, 200, body.to_string());
        let mut settings = settings_with_key();
        settings.custom_words = words(&["Augusto"]);
        let text =
            "o agusto acho que pode mandar o relatório amanhã cedo para o time inteiro todo hoje";

        let auto = run(&settings, &server, text, app("notepad"));

        assert_eq!(server.requests(), 1);
        assert!(server.bodies.lock().unwrap()[0].contains("O agusto acho"));
        assert_eq!(
            auto,
            AutoFormatted {
                final_text: answer.to_string(),
                llm_produced: true
            }
        );
    }

    #[test]
    fn deferred_fuzzy_fixes_text_the_llm_did_not_write() {
        let long =
            "o agusto acho que pode mandar o relatório amanhã cedo para o time inteiro todo hoje";
        let mut settings = settings_with_key();
        settings.custom_words = words(&["Augusto"]);

        let quiet = FakeGemini::ok(Duration::ZERO);
        let short = run(&settings, &quiet, "o agusto mandou isso", app("notepad"));
        assert_eq!(short.final_text, "O Augusto mandou isso");
        let listed = run(&settings, &quiet, long, app("keepassxc"));
        assert!(listed.final_text.starts_with("O Augusto acho"), "keepassxc");
        assert_eq!(quiet.requests(), 0);

        let failing = FakeGemini::start(Duration::ZERO, 500, "{}".to_string());
        let failed = run(&settings, &failing, long, app("notepad"));
        assert!(failed.final_text.starts_with("O Augusto acho"), "http 500");
        assert!(!failed.llm_produced);
        assert_eq!(failing.requests(), 1);
    }

    #[test]
    fn no_second_fuzzy_without_llm() {
        let mut settings = crate::settings::get_default_settings();
        settings.custom_words = words(&["Augusto"]);
        assert!(Fuzzy::deferred(&settings).is_none());

        let server = FakeGemini::ok(Duration::ZERO);
        let auto = run(&settings, &server, "agusto mandou", app("notepad"));
        assert_eq!(auto.final_text, "Agusto mandou");
        assert_eq!(server.requests(), 0);
    }

    #[test]
    fn custom_words_are_normalized() {
        assert_eq!(
            normalize_words(words(&[" Fala ", "fala", "", "ChargeBee", "  "])),
            ["Fala", "ChargeBee"]
        );
    }

    #[test]
    fn llm_text_is_pasted_and_recorded_as_llm() {
        let server = FakeGemini::ok(Duration::from_millis(500));
        let auto = run(&settings_with_key(), &server, SIXTEEN, app("notepad"));
        assert_eq!(
            auto,
            AutoFormatted {
                final_text: LLM_TEXT.to_string(),
                llm_produced: true
            }
        );
        assert_eq!(server.requests(), 1);
        assert!(server.bodies.lock().unwrap()[0].contains("<app>notepad</app>"));

        let processed = auto_processed(SIXTEEN, auto);
        assert_eq!(processed.post_processed_text.as_deref(), Some(LLM_TEXT));
        let dictation = dictation_for(
            SIXTEEN,
            &processed.final_text,
            processed.llm_produced,
            Language::PtBr,
            app("notepad"),
        )
        .unwrap();
        assert_eq!(dictation.editor, Editor::Llm);
    }

    #[test]
    fn no_request_below_threshold_disabled_or_without_key() {
        let server = FakeGemini::ok(Duration::ZERO);

        let fifteen = run(&settings_with_key(), &server, FIFTEEN, app("notepad"));
        assert_eq!(
            fifteen.final_text,
            "Eu acho que a gente pode mandar o relatório amanhã cedo para o time todo"
        );
        assert!(!fifteen.llm_produced);

        let mut off = settings_with_key();
        off.llm_enabled = false;
        let disabled = run(&off, &server, SIXTEEN, app("notepad"));
        assert_eq!(disabled.final_text, SIXTEEN_RULES);
        assert!(!disabled.llm_produced);

        let no_key = crate::settings::get_default_settings();
        let keyless = run(&no_key, &server, SIXTEEN, app("notepad"));
        assert_eq!(keyless.final_text, SIXTEEN_RULES);
        assert!(!keyless.llm_produced);

        assert_eq!(server.requests(), 0);
    }

    #[test]
    fn no_request_for_disabled_app() {
        let server = FakeGemini::ok(Duration::ZERO);

        let mut listed = settings_with_key();
        listed.llm_disabled_apps = vec!["Code".to_string()];
        let code = run(&listed, &server, SIXTEEN, app("code"));
        assert_eq!(code.final_text, SIXTEEN_RULES);
        assert!(!code.llm_produced);

        let default_list = run(&settings_with_key(), &server, SIXTEEN, app("keepassxc"));
        assert_eq!(default_list.final_text, SIXTEEN_RULES);
        assert!(!default_list.llm_produced);

        assert_eq!(server.requests(), 0);
    }

    #[test]
    fn unknown_app_still_uses_llm() {
        let server = FakeGemini::ok(Duration::ZERO);
        let auto = run(
            &settings_with_key(),
            &server,
            SIXTEEN,
            AppContext::default(),
        );
        assert_eq!(auto.final_text, LLM_TEXT);
        assert!(auto.llm_produced);
        assert_eq!(server.requests(), 1);
        // The system prompt mentions `<app>`; only a known app adds the closing tag.
        assert!(!server.bodies.lock().unwrap()[0].contains("</app>"));
    }

    #[test]
    fn slow_failed_or_invalid_llm_keeps_rules_text() {
        let candidates_empty = r#"{"candidates":[]}"#.to_string();
        for (case, server) in [
            ("3 s", FakeGemini::ok(Duration::from_secs(3))),
            (
                "http 500",
                FakeGemini::start(Duration::ZERO, 500, "{}".to_string()),
            ),
            (
                "invalid body",
                FakeGemini::start(Duration::ZERO, 200, candidates_empty),
            ),
        ] {
            let started = Instant::now();
            let auto = run(&settings_with_key(), &server, SIXTEEN, app("notepad"));
            assert!(started.elapsed() < Duration::from_millis(2500), "{case}");
            assert_eq!(auto.final_text, SIXTEEN_RULES, "{case}");
            assert!(!auto.llm_produced, "{case}");
            assert_eq!(server.requests(), 1, "{case}");

            let processed = auto_processed(SIXTEEN, auto);
            assert_eq!(
                processed.post_processed_text.as_deref(),
                Some(SIXTEEN_RULES),
                "{case}"
            );
            assert!(!processed.llm_produced, "{case}");
        }
    }

    #[test]
    fn disabled_apps_are_normalized() {
        let apps = [" Chrome.exe ", "chrome", "", "C:\\Tools\\Slack.exe"]
            .map(String::from)
            .to_vec();
        assert_eq!(normalize_apps(apps), ["chrome", "slack"]);
    }
}
