//! The automatic dictation LLM (ADR-0004): every `transcribe` dictation goes through the
//! `fala-postproc` `Postprocessor`. The local rules always run; the Gemini formats the text when
//! the LLM is on, the `gemini` key is set, the app is not in `llm_disabled_apps` and the text has
//! more than 15 words. An unknown app keeps the LLM on (D4 of the phase 1 plan).

use anyhow::Result;
use fala_core::{AppContext, Dictionary, Editor, Language, Transcript};
use fala_postproc::{Gemini, LateEdit, LlmConfig, Postprocessor, DEFAULT_BASE_URL};
use fala_secrets::ApiKey;
use log::debug;

use crate::managers::history::HistoryEntry;
use crate::settings::{AppSettings, GEMINI_PROVIDER_ID};

/// The text to paste and whether the LLM produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AutoFormatted {
    pub final_text: String,
    pub llm_produced: bool,
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
/// asked; past that, the rules text stays, and the LLM answer still on its way comes back as a
/// `LateEdit`: it may arrive until the late deadline and become "Aplicar edição da IA" on the
/// saved entry (ADR-0004).
pub(crate) fn format_with_late_edit(
    postprocessor: &Postprocessor,
    text: &str,
    language: Language,
    app: AppContext,
) -> (AutoFormatted, Option<LateEdit>) {
    let raw = Transcript {
        text: text.to_string(),
        language,
    };
    let formatted = postprocessor.process(raw, app, &Dictionary::default());
    let editor = formatted.dictation.editor;
    debug!(
        "post-processing: editor={editor:?} late_edit={}",
        formatted.late_edit.is_some()
    );
    let auto = AutoFormatted {
        final_text: formatted.dictation.final_text,
        llm_produced: editor == Editor::Llm,
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
        format_with_late_edit(
            &postprocessor_at(settings, &server.base_url),
            text,
            Language::PtBr,
            app,
        )
        .0
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
        fn deliver(&self, postprocessor: &Postprocessor) -> (HistoryEntry, Option<LateEdit>) {
            let started = Instant::now();
            let (auto, late_edit) =
                format_with_late_edit(postprocessor, SIXTEEN, Language::PtBr, app("notepad"));
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
            history.deliver(&postprocessor_at(&settings_with_key(), &server.base_url));
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
            let mut postprocessor = postprocessor_at(&settings_with_key(), &server.base_url);
            if let Some(deadline) = late_deadline {
                postprocessor = postprocessor.with_late_deadline(deadline);
            }
            let (saved, late_edit) = history.deliver(&postprocessor);
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

    #[test]
    fn disabled_apps_are_normalized() {
        let apps = [" Chrome.exe ", "chrome", "", "C:\\Tools\\Slack.exe"]
            .map(String::from)
            .to_vec();
        assert_eq!(normalize_apps(apps), ["chrome", "slack"]);
    }
}
