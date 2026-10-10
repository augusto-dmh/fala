//! The automatic dictation LLM (ADR-0004): every `transcribe` dictation goes through the
//! `fala-postproc` `Postprocessor`. The local rules always run; the Gemini formats the text when
//! the LLM is on, the `gemini` key is set, the app is not in `llm_disabled_apps` and the text has
//! more than 15 words. An unknown app keeps the LLM on (D4 of the phase 1 plan).

use fala_core::{AppContext, Dictionary, Editor, Language, Transcript};
use fala_postproc::{Fallback, Gemini, LlmConfig, Postprocessor, DEFAULT_BASE_URL};
use fala_secrets::ApiKey;
use log::debug;

use crate::settings::{AppSettings, GEMINI_PROVIDER_ID};

/// The text to paste, whether the LLM produced it, and why not when it was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AutoFormatted {
    pub final_text: String,
    pub llm_produced: bool,
    pub fallback: Option<Fallback>,
}

/// The `Postprocessor` the settings describe. The key comes from `post_process_api_keys`, which
/// the settings load fills from the OS keyring.
pub(crate) fn postprocessor(settings: &AppSettings) -> Postprocessor {
    postprocessor_at(settings, DEFAULT_BASE_URL)
}

fn postprocessor_at(settings: &AppSettings, base_url: &str) -> Postprocessor {
    let gemini = settings
        .post_process_api_keys
        .get(GEMINI_PROVIDER_ID)
        .and_then(|key| ApiKey::new(key.clone()).ok())
        .map(|key| Gemini::new(key).with_base_url(base_url));
    Postprocessor::new(LlmConfig {
        enabled: settings.llm_enabled,
        gemini,
        disabled_apps: settings.llm_disabled_apps.clone(),
    })
}

/// Formats one dictation. Blocks for at most `fala_postproc::INSERT_DEADLINE` when the LLM is
/// asked; past that, the rules text stays.
pub(crate) fn format(
    postprocessor: &Postprocessor,
    text: &str,
    language: Language,
    app: AppContext,
) -> AutoFormatted {
    let raw = Transcript {
        text: text.to_string(),
        language,
    };
    let formatted = postprocessor.process(raw, app, &Dictionary::default());
    let editor = formatted.dictation.editor;
    debug!("post-processing: editor={editor:?}");
    AutoFormatted {
        final_text: formatted.dictation.final_text,
        llm_produced: editor == Editor::Llm,
        fallback: formatted.fallback,
    }
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
            &postprocessor_at(settings, &server.base_url),
            text,
            Language::PtBr,
            app,
        )
    }

    #[test]
    fn llm_text_is_pasted_and_recorded_as_llm() {
        let server = FakeGemini::ok(Duration::from_millis(500));
        let auto = run(&settings_with_key(), &server, SIXTEEN, app("notepad"));
        assert_eq!(
            auto,
            AutoFormatted {
                final_text: LLM_TEXT.to_string(),
                llm_produced: true,
                fallback: None,
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
        assert_eq!(fifteen.fallback, None);

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
        for (case, fallback, server) in [
            (
                "3 s",
                Fallback::Timeout,
                FakeGemini::ok(Duration::from_secs(3)),
            ),
            (
                "http 500",
                Fallback::Http(500),
                FakeGemini::start(Duration::ZERO, 500, "{}".to_string()),
            ),
            (
                "invalid body",
                Fallback::InvalidResponse,
                FakeGemini::start(Duration::ZERO, 200, candidates_empty),
            ),
        ] {
            let started = Instant::now();
            let auto = run(&settings_with_key(), &server, SIXTEEN, app("notepad"));
            assert!(started.elapsed() < Duration::from_millis(2500), "{case}");
            assert_eq!(auto.final_text, SIXTEEN_RULES, "{case}");
            assert!(!auto.llm_produced, "{case}");
            assert_eq!(auto.fallback, Some(fallback), "{case}");
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
