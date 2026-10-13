//! The automatic dictation LLM (ADR-0004): every `transcribe` dictation goes through the
//! `fala-postproc` `Postprocessor`. The local rules always run; the Gemini formats the text when
//! the LLM is on, the `gemini` key is set, the app is not in `llm_disabled_apps` and the text has
//! more than 15 words. An unknown app keeps the LLM on (D4 of the phase 1 plan).
//!
//! `custom_words` and `dictionary_replacements` are the personal dictionary: the rules apply its
//! spelling, its spoken -> written pairs and its snippets, and the Gemini gets the terms in the
//! prompt and each snippet only as an opaque marker. While the LLM is configured, the inherited
//! fuzzy correction waits until the
//! LLM is out of the way and only touches text the LLM did not write.

use std::panic::{catch_unwind, AssertUnwindSafe};

use anyhow::Result;
use fala_core::{AppContext, Dictionary, Editor, Language, Transcript};
use fala_postproc::{
    CleanupLevel, Fallback, FormatContext, Gemini, LateEdit, LlmConfig, Postprocessor, Rules,
    DEFAULT_BASE_URL,
};
use fala_secrets::ApiKey;
use log::{debug, error};

use crate::audio_toolkit::apply_custom_words;
use crate::managers::history::HistoryEntry;
use crate::settings::{AppSettings, GEMINI_PROVIDER_ID};

/// The text to paste, whether the LLM produced it, and why not when it was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AutoFormatted {
    pub final_text: String,
    pub llm_produced: bool,
    pub fallback: Option<Fallback>,
    /// The spoken form of each dictionary pair the rules applied, once per occurrence.
    pub replaced: Vec<String>,
    /// The trigger of each snippet the rules applied, once per occurrence.
    pub snippets: Vec<String>,
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
        })
        .with_cleanup_level(settings.cleanup_level.into())
        .with_app_styles(crate::app_styles::instructions(settings)),
        dictionary: crate::dictionary::dictionary(settings),
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
/// asked; past that, the rules text stays, and the LLM answer still on its way comes back as a
/// `LateEdit`: it may arrive until the late deadline and become "Aplicar edição da IA" on the
/// saved entry (ADR-0004). The late answer is LLM text, so the fuzzy correction never touches it.
pub(crate) fn format_with_late_edit(
    formatter: &AutoFormatter,
    text: &str,
    language: Language,
    app: AppContext,
) -> (AutoFormatted, Option<LateEdit>) {
    let raw = Transcript {
        text: text.to_string(),
        language,
    };
    let formatted = formatter
        .postprocessor
        .process(raw, app, &formatter.dictionary);
    let editor = formatted.dictation.editor;
    debug!(
        "post-processing: editor={editor:?} late_edit={}",
        formatted.late_edit.is_some()
    );
    let llm_produced = editor == Editor::Llm;
    let mut hits = formatted.replaced;
    let mut snippet_hits = formatted.snippets;
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
                cleanup: CleanupLevel::default(),
                style: None,
            };
            let masked = Rules::default().mask(&fuzzy.apply(dictation.raw.text.clone()), &ctx);
            hits = masked.replaced.clone();
            snippet_hits = masked.snippets.clone();
            Some(masked.unmasked())
        }
        _ => None,
    };
    let replacements = formatter.dictionary.replacements();
    let snippets = formatter.dictionary.snippets();
    let auto = AutoFormatted {
        final_text: refixed.unwrap_or(dictation.final_text),
        llm_produced,
        fallback: formatted.fallback,
        replaced: hits
            .into_iter()
            .filter_map(|i| replacements.get(i).map(|pair| pair.spoken().to_string()))
            .collect(),
        snippets: snippet_hits
            .into_iter()
            .filter_map(|i| snippets.get(i).map(|snippet| snippet.trigger().to_string()))
            .collect(),
    };
    (auto, formatted.late_edit)
}

/// The app is one where the LLM stays off; its dictations are saved as sensitive. Compared
/// without case, as the `Postprocessor` does.
pub(crate) fn is_disabled_app(settings: &AppSettings, app: &AppContext) -> bool {
    app.app_name.as_deref().is_some_and(|name| {
        let name = name.to_lowercase();
        settings
            .llm_disabled_apps
            .iter()
            .any(|disabled| disabled.to_lowercase() == name)
    })
}

/// Waits for the late answer and applies it to the saved entry; `announce` gets the updated
/// entry. An LLM error, the late deadline or a deleted entry change nothing and only reach the
/// log at `debug`. Blocks until the answer or the late deadline.
pub(crate) fn finish_late_edit<A, E>(late_edit: LateEdit, apply: A, announce: E)
where
    A: FnOnce(&str) -> Result<HistoryEntry>,
    E: FnOnce(HistoryEntry),
{
    let text = match late_edit.wait() {
        Ok(text) => text,
        Err(fallback) => {
            debug!("late LLM answer not applied: {fallback}");
            return;
        }
    };
    match apply(&text) {
        Ok(entry) => announce(entry),
        Err(e) => debug!("late LLM answer not applied: {e}"),
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
    use std::sync::{mpsc, Arc, Mutex};
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
            Self::start_with(move || thread::sleep(delay), status, body)
        }

        /// Runs `wait` between reading a request and answering it.
        fn start_with(mut wait: impl FnMut() + Send + 'static, status: u16, body: String) -> Self {
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
                    wait();
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
            Self::start(delay, 200, ok_body())
        }

        /// A Gemini that holds its answer until the sender sends (or is dropped): a test sees a
        /// block give up on the answer without timing the wall clock, which a loaded machine
        /// stretches.
        fn held() -> (Self, mpsc::Sender<()>) {
            let (release, held) = mpsc::channel::<()>();
            let server = Self::start_with(
                move || {
                    let _ = held.recv();
                },
                200,
                ok_body(),
            );
            (server, release)
        }

        fn requests(&self) -> usize {
            self.bodies.lock().unwrap().len()
        }
    }

    fn ok_body() -> String {
        serde_json::json!({
            "candidates": [{ "content": { "parts": [{ "text": LLM_TEXT }] } }]
        })
        .to_string()
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
        format_with_late_edit(
            &formatter_at(settings, true, &server.base_url),
            text,
            Language::PtBr,
            app,
        )
        .0
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
        // The v2 prompt puts the cleanup level between the fixed text and the dictionary.
        let prompt = system_prompt(&server, 0);
        assert!(prompt.starts_with(SYSTEM_PROMPT), "{prompt}");
        assert!(
            prompt.ends_with(
                "

Personal dictionary:
- Augusto
- ChargeBee"
            ),
            "{prompt}"
        );
    }

    #[test]
    fn cleanup_level_reaches_the_llm_prompt() {
        use crate::settings::CleanupLevel as Setting;
        let server = FakeGemini::ok(Duration::ZERO);
        let levels = [
            (Setting::Verbatim, CleanupLevel::Verbatim),
            (Setting::Light, CleanupLevel::Light),
            (Setting::Medium, CleanupLevel::Medium),
            (Setting::Heavy, CleanupLevel::Heavy),
        ];
        for (index, (setting, level)) in levels.into_iter().enumerate() {
            let mut settings = settings_with_key();
            settings.cleanup_level = setting;
            assert!(run(&settings, &server, SIXTEEN, app("notepad")).llm_produced);
            let prompt = system_prompt(&server, index);
            for other in CleanupLevel::ALL {
                assert_eq!(
                    prompt.contains(other.instruction()),
                    other == level,
                    "{setting:?} asked, {other} in the prompt? {prompt}"
                );
            }
        }

        // The default store asks for the light level, as before the setting existed.
        assert!(run(&settings_with_key(), &server, SIXTEEN, app("notepad")).llm_produced);
        assert!(system_prompt(&server, levels.len()).contains(CleanupLevel::Light.instruction()));
    }

    #[test]
    fn app_style_ends_the_llm_prompt() {
        use crate::app_styles::AppStyle;
        use fala_postproc::STYLE_INSTRUCTION;
        let server = FakeGemini::ok(Duration::ZERO);
        let mut settings = settings_with_key();
        settings.custom_words = words(&["ChargeBee"]);
        settings.writing_styles[1].instruction = "Formal, com \"Olá, <nome>\".".to_string();
        settings.app_styles = vec![
            AppStyle {
                app: "chrome".to_string(),
                style: "email".to_string(),
            },
            AppStyle {
                app: "1password".to_string(),
                style: "email".to_string(),
            },
        ];
        for name in ["Chrome", "notepad", "1password"] {
            run(&settings, &server, SIXTEEN, app(name));
        }
        // The app the LLM stays off in sends nothing, style or not.
        assert_eq!(server.requests(), 2);
        let styled = system_prompt(&server, 0);
        let plain = system_prompt(&server, 1);
        assert!(plain.ends_with("\n- ChargeBee"), "{plain}");
        assert_eq!(
            styled,
            format!("{plain}\n\n{STYLE_INSTRUCTION}\nFormal, com \"Olá, <nome>\".")
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
            let prompt = system_prompt(&server, index);
            assert!(prompt.starts_with(SYSTEM_PROMPT), "case {index}: {prompt}");
            assert!(
                !prompt.contains("Personal dictionary"),
                "case {index}: {prompt}"
            );
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

        let (auto, late_edit) = format_with_late_edit(
            &formatter_at(&settings, false, &server.base_url),
            text,
            Language::PtBr,
            AppContext::default(),
        );
        assert!(late_edit.is_none());

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
                llm_produced: true,
                fallback: None,
                replaced: Vec::new(),
                snippets: Vec::new(),
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
    fn replacements_are_applied_and_reported() {
        let server = FakeGemini::ok(Duration::ZERO);
        let mut settings = crate::settings::get_default_settings();
        settings.dictionary_replacements = vec![crate::dictionary::DictionaryReplacement {
            spoken: "fala sql".to_string(),
            written: "Fala.sqlite".to_string(),
            uses: 0,
        }];
        let auto = run(
            &settings,
            &server,
            "abre o fala sql e o Fala-SQL",
            app("notepad"),
        );
        assert_eq!(auto.final_text, "Abre o Fala.sqlite e o Fala.sqlite");
        assert_eq!(auto.replaced, ["fala sql", "fala sql"]);
        assert!(!auto.llm_produced);

        // With the LLM configured, text it did not write goes through the deferred fuzzy and
        // the rules again; the uses come from that second pass, not twice.
        let mut keyed = settings_with_key();
        keyed.custom_words = words(&["Augusto"]);
        keyed.dictionary_replacements = settings.dictionary_replacements.clone();
        let short = run(&keyed, &server, "o agusto abriu o fala sql", app("notepad"));
        assert_eq!(short.final_text, "O Augusto abriu o Fala.sqlite");
        assert_eq!(short.replaced, ["fala sql"]);
        assert_eq!(server.requests(), 0);
    }

    #[test]
    fn snippets_are_applied_and_reported_with_or_without_the_llm() {
        let server = FakeGemini::ok(Duration::ZERO);
        let mut settings = settings_with_key();
        settings.custom_words = words(&["Augusto"]);
        settings.dictionary_snippets = vec![crate::dictionary::DictionaryReplacement {
            spoken: "minha assinatura".to_string(),
            written: "Augusto\nfala.dev".to_string(),
            uses: 0,
        }];
        // Short text, LLM configured: the deferred fuzzy and the rules run again; one count.
        let short = run(
            &settings,
            &server,
            "o agusto e minha assinatura",
            app("notepad"),
        );
        assert_eq!(short.final_text, "O Augusto e Augusto\nfala.dev");
        assert_eq!(short.snippets, ["minha assinatura"]);
        // The whole dictation is the trigger: the text as is, no request.
        let whole = run(&settings, &server, "Minha assinatura.", app("notepad"));
        assert_eq!(whole.final_text, "Augusto\nfala.dev");
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
                llm_produced: true,
                fallback: None,
                replaced: Vec::new(),
                snippets: Vec::new(),
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

    // ---- late edit (llm-late-edit C6, C7) --------------------------------------------------

    /// Captures this thread's log lines.
    struct Capture;

    thread_local! {
        static LINES: std::cell::RefCell<Vec<(log::Level, String)>> =
            const { std::cell::RefCell::new(Vec::new()) };
    }

    impl log::Log for Capture {
        fn enabled(&self, _: &log::Metadata<'_>) -> bool {
            true
        }
        fn log(&self, record: &log::Record<'_>) {
            LINES.with(|lines| {
                lines
                    .borrow_mut()
                    .push((record.level(), record.args().to_string()))
            });
        }
        fn flush(&self) {}
    }

    static CAPTURE: Capture = Capture;

    fn capture_log() {
        let _ = log::set_logger(&CAPTURE);
        log::set_max_level(log::LevelFilter::Trace);
        LINES.with(|lines| lines.borrow_mut().clear());
    }

    fn captured() -> Vec<(log::Level, String)> {
        LINES.with(|lines| lines.borrow().clone())
    }

    /// history.db in memory and fala.sqlite in a temp dir.
    struct History {
        dir: tempfile::TempDir,
        conn: rusqlite::Connection,
        store: fala_storage::Store,
    }

    fn history() -> History {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        rusqlite_migration::Migrations::new(crate::managers::history::MIGRATIONS.to_vec())
            .to_latest(&mut conn)
            .unwrap();
        let store =
            fala_storage::Store::open(&dir.path().join("fala.sqlite"), &dir.path().join("notas"))
                .unwrap();
        History { dir, conn, store }
    }

    impl History {
        /// Formats like the stop path, saves what it pastes, and returns the pending late edit.
        fn deliver(&self, formatter: &AutoFormatter) -> (HistoryEntry, Option<LateEdit>) {
            let started = Instant::now();
            let (auto, late_edit) =
                format_with_late_edit(formatter, SIXTEEN, Language::PtBr, app("notepad"));
            assert!(started.elapsed() < Duration::from_millis(2500));
            assert_eq!(auto.final_text, SIXTEEN_RULES);
            assert!(!auto.llm_produced);
            let processed = auto_processed(SIXTEEN, auto);
            let saved = crate::managers::history::HistoryManager::save_entry_with(
                &self.conn,
                Some(&self.store),
                crate::managers::history::NewEntry {
                    file_name: "fala-1.wav".to_string(),
                    post_process_requested: false,
                    texts: crate::managers::history::EntryTexts {
                        transcription_text: SIXTEEN.to_string(),
                        post_processed_text: processed.post_processed_text,
                        post_process_prompt: None,
                        pasted_text: processed.final_text,
                        llm_produced: processed.llm_produced,
                    },
                    app: app("notepad"),
                    sensitive: false,
                    paste_failed: false,
                    llm_partial: false,
                },
                Language::PtBr,
                1,
            )
            .unwrap();
            (saved, late_edit)
        }

        fn apply(&self, id: i64, text: &str) -> anyhow::Result<HistoryEntry> {
            crate::managers::history::HistoryManager::apply_late_edit_with(
                &self.conn,
                Some(&self.store),
                id,
                text,
            )
        }

        fn post_processed_text(&self, id: i64) -> Option<String> {
            self.conn
                .query_row(
                    "SELECT post_processed_text FROM transcription_history WHERE id = ?1",
                    [id],
                    |r| r.get(0),
                )
                .unwrap()
        }
    }

    #[test]
    fn late_answer_is_applied_and_announced() {
        let server = FakeGemini::ok(Duration::from_secs(3));
        let history = history();
        let (saved, late_edit) =
            history.deliver(&formatter_at(&settings_with_key(), true, &server.base_url));
        let dictation_id = saved.dictation_id.clone().unwrap();

        let mut announced = Vec::new();
        finish_late_edit(
            late_edit.expect("a late edit for a 3 s answer"),
            |text| history.apply(saved.id, text),
            |entry| announced.push(entry),
        );

        let record = history.store.get(&dictation_id).unwrap();
        assert_eq!(record.dictation.final_text, LLM_TEXT);
        assert_eq!(record.dictation.editor, Editor::Llm);
        assert_eq!(record.showing, fala_storage::Showing::Raw);
        assert_eq!(
            history.post_processed_text(saved.id).as_deref(),
            Some(SIXTEEN_RULES)
        );
        assert_eq!(announced.len(), 1);
        assert_eq!(announced[0].id, saved.id);
        let dictation = announced[0].dictation.as_ref().unwrap();
        assert_eq!(dictation.final_text, LLM_TEXT);
        assert_eq!(
            dictation.showing,
            crate::managers::history_dictations::HistoryShowing::Raw
        );
    }

    #[test]
    fn late_answer_errors_change_nothing() {
        let error = FakeGemini::start(Duration::from_secs(3), 500, "{}".to_string());
        let too_late = FakeGemini::ok(Duration::from_secs(4));
        let deleted = FakeGemini::ok(Duration::from_secs(3));
        for (case, server, late_deadline, delete) in [
            ("http 500 after 3 s", &error, None, false),
            (
                "after the deadline",
                &too_late,
                Some(Duration::from_secs(3)),
                false,
            ),
            ("entry deleted", &deleted, None, true),
        ] {
            let history = history();
            let mut formatter = formatter_at(&settings_with_key(), true, &server.base_url);
            if let Some(deadline) = late_deadline {
                formatter.postprocessor = formatter.postprocessor.with_late_deadline(deadline);
            }
            let (saved, late_edit) = history.deliver(&formatter);
            let late_edit = late_edit.expect(case);
            if delete {
                crate::managers::history::HistoryManager::delete_entry_with(
                    &history.conn,
                    Some(&history.store),
                    history.dir.path(),
                    saved.id,
                )
                .unwrap();
            }
            let before = history.store.search("", 10).unwrap();

            capture_log();
            let mut announced = Vec::new();
            finish_late_edit(
                late_edit,
                |text| history.apply(saved.id, text),
                |entry| announced.push(entry),
            );

            assert!(announced.is_empty(), "{case}");
            assert_eq!(history.store.search("", 10).unwrap(), before, "{case}");
            if !delete {
                let record = history
                    .store
                    .get(saved.dictation_id.as_deref().unwrap())
                    .unwrap();
                assert_eq!(record.dictation.final_text, SIXTEEN_RULES, "{case}");
                assert_eq!(record.showing, fala_storage::Showing::Final, "{case}");
            }
            let log = captured();
            assert!(!log.is_empty(), "{case}");
            assert!(
                log.iter().all(|(level, _)| *level >= log::Level::Debug),
                "{case}: {log:?}"
            );
            assert!(
                log.iter().all(|(_, line)| !line.contains("relatório")),
                "{case}: {log:?}"
            );
        }
    }

    // ---- long dictation formatted block by block -------------------------------------------

    /// One block formatted as the queue formats it (`process_transcription_output` in Auto).
    fn block(formatter: &AutoFormatter) -> crate::actions::ProcessedTranscription {
        let (auto, late_edit) =
            format_with_late_edit(formatter, SIXTEEN, Language::PtBr, app("notepad"));
        crate::actions::ProcessedTranscription {
            late_edit,
            ..auto_processed(SIXTEEN, auto)
        }
    }

    impl History {
        /// Saves a long dictation as the stop path does.
        fn deliver_long(
            &self,
            raw: &str,
            composed: &crate::long_dictation::Composed,
        ) -> HistoryEntry {
            crate::managers::history::HistoryManager::save_entry_with(
                &self.conn,
                Some(&self.store),
                crate::managers::history::NewEntry {
                    file_name: "fala-1.wav".to_string(),
                    post_process_requested: false,
                    texts: crate::managers::history::EntryTexts {
                        transcription_text: raw.to_string(),
                        post_processed_text: composed.processed.post_processed_text.clone(),
                        post_process_prompt: None,
                        pasted_text: composed.processed.final_text.clone(),
                        llm_produced: composed.processed.llm_produced,
                    },
                    app: app("notepad"),
                    sensitive: false,
                    paste_failed: false,
                    llm_partial: composed.partial,
                },
                Language::PtBr,
                1,
            )
            .unwrap()
        }

        fn llm_partial(&self, id: i64) -> bool {
            self.conn
                .query_row(
                    "SELECT llm_partial FROM transcription_history WHERE id = ?1",
                    [id],
                    |r| r.get(0),
                )
                .unwrap()
        }
    }

    #[test]
    fn every_block_answering_pastes_the_llm_text() {
        let server = FakeGemini::ok(Duration::ZERO);
        let formatter = formatter_at(&settings_with_key(), true, &server.base_url);
        let raw = [SIXTEEN; 3].join(" ");
        let blocks = vec![block(&formatter), block(&formatter), block(&formatter)];
        assert_eq!(server.requests(), 3, "one request per block");
        let composed = crate::long_dictation::compose(&raw, blocks);
        assert_eq!(composed.processed.final_text, [LLM_TEXT; 3].join(" "));
        assert!(composed.processed.llm_produced);
        assert!(!composed.partial);
        assert!(composed.late.is_none());

        let history = history();
        let saved = history.deliver_long(&raw, &composed);
        assert!(!saved.llm_partial);
        let dictation = saved.dictation.unwrap();
        assert_eq!(dictation.raw_text, raw);
        assert_eq!(
            dictation.editor,
            crate::managers::history_dictations::HistoryEditor::Llm
        );
    }

    #[test]
    fn a_slow_block_keeps_its_rules_until_the_late_answer() {
        let fast = FakeGemini::ok(Duration::ZERO);
        // The slow answer exists only after `answer.send`, so the blocks below can only return
        // with rules if the slow one stopped waiting first (after the 2 s, not the 10 s late
        // deadline, or no late answer would be left to wait for). No wall-clock bound.
        let (slow, answer) = FakeGemini::held();
        let settings = settings_with_key();
        let raw = [SIXTEEN; 3].join(" ");
        let blocks = vec![
            block(&formatter_at(&settings, true, &fast.base_url)),
            block(&formatter_at(&settings, true, &slow.base_url)),
            block(&formatter_at(&settings, true, &fast.base_url)),
        ];
        let composed = crate::long_dictation::compose(&raw, blocks);
        assert_eq!(
            composed.processed.final_text,
            format!("{LLM_TEXT} {SIXTEEN_RULES} {LLM_TEXT}"),
            "rules in the slow block, the LLM in the others"
        );
        assert!(composed.processed.llm_produced);
        assert_eq!(composed.processed.fallback, Some(Fallback::Timeout));
        assert!(composed.partial);

        let history = history();
        let saved = history.deliver_long(&raw, &composed);
        assert!(saved.llm_partial);
        assert!(history.llm_partial(saved.id));

        answer.send(()).unwrap();
        let (text, partial) = composed
            .late
            .expect("the slow block can still answer")
            .wait()
            .expect("it answers once released");
        assert_eq!(text, [LLM_TEXT; 3].join(" "));
        assert!(!partial, "every block has the LLM text now");
        let entry = crate::managers::history::HistoryManager::apply_blocks_late_edit_with(
            &history.conn,
            Some(&history.store),
            saved.id,
            &text,
            partial,
        )
        .unwrap();
        assert!(!entry.llm_partial);
        assert!(!history.llm_partial(saved.id));
        let dictation = entry.dictation.unwrap();
        assert_eq!(dictation.final_text, text);
        assert_eq!(
            dictation.showing,
            crate::managers::history_dictations::HistoryShowing::Raw,
            "the whole LLM text waits behind \"Aplicar edição da IA\""
        );
    }

    #[test]
    fn a_block_past_the_late_deadline_stays_partial() {
        let fast = FakeGemini::ok(Duration::ZERO);
        let too_late = FakeGemini::ok(Duration::from_secs(4));
        let settings = settings_with_key();
        let mut late_formatter = formatter_at(&settings, true, &too_late.base_url);
        late_formatter.postprocessor = late_formatter
            .postprocessor
            .with_late_deadline(Duration::from_secs(3));
        let raw = [SIXTEEN; 2].join(" ");
        let composed = crate::long_dictation::compose(
            &raw,
            vec![
                block(&formatter_at(&settings, true, &fast.base_url)),
                block(&late_formatter),
            ],
        );
        assert!(composed.partial);
        assert_eq!(
            composed.late.expect("pending").wait(),
            None,
            "no answer arrived, so nothing to apply"
        );
    }

    #[test]
    fn disabled_apps_are_normalized() {
        let apps = [" Chrome.exe ", "chrome", "", "C:\\Tools\\Slack.exe"]
            .map(String::from)
            .to_vec();
        assert_eq!(normalize_apps(apps), ["chrome", "slack"]);
    }
}
