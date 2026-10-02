//! `fala-cli format`: o pós-processamento sem UI. Lê o texto do stdin, aplica as regras e,
//! com `--llm`, o Gemini; imprime o texto final no stdout e quem editou no stderr.

use std::io::{Read, Write};
use std::path::PathBuf;

use clap::Args;
use fala_core::{AppContext, Dictionary, Editor, Language, Transcript};
use fala_postproc::{Gemini, LlmConfig, Postprocessor, DEFAULT_BASE_URL, DEFAULT_MODEL};
use fala_secrets::{SecretError, SecretStore};

/// Id da chave do Gemini no keyring.
const GEMINI_PROVIDER: &str = "gemini";

#[derive(Args)]
pub struct FormatArgs {
    /// Usa o Gemini acima de 15 palavras (precisa de `fala-cli key set gemini`).
    #[arg(long)]
    llm: bool,
    /// Nome do app ativo, enviado ao LLM.
    #[arg(long)]
    app: Option<String>,
    /// Modelo do Gemini.
    #[arg(long, default_value = DEFAULT_MODEL)]
    model: String,
    /// Idioma do ditado: pt-BR ou en.
    #[arg(long, default_value = "pt-BR")]
    lang: String,
    /// Arquivo do dicionário pessoal, um termo por linha.
    #[arg(long)]
    dictionary: Option<PathBuf>,
    /// App onde o LLM fica desligado (repetível; compara sem caixa).
    #[arg(long = "disable-app")]
    disable_app: Vec<String>,
    /// Endpoint do Gemini; só para testes contra um servidor falso.
    #[arg(long, hide = true, default_value = DEFAULT_BASE_URL)]
    gemini_base_url: String,
}

/// Roda o subcomando e devolve o código de saída: 0 ok (inclusive com fallback),
/// 1 keyring indisponível, 2 uso inválido ou `--llm` sem chave.
pub fn run(
    args: FormatArgs,
    store: &dyn SecretStore,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> u8 {
    let Ok(language) = args.lang.parse::<Language>() else {
        let _ = writeln!(stderr, "erro: idioma desconhecido; use pt-BR ou en");
        return 2;
    };
    let dictionary = match &args.dictionary {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(content) => Dictionary::new(content.lines()),
            Err(error) => {
                let _ = writeln!(
                    stderr,
                    "erro: não consegui ler o dicionário {}: {error}",
                    path.display()
                );
                return 2;
            }
        },
        None => Dictionary::default(),
    };
    let gemini = if args.llm {
        match store.get(GEMINI_PROVIDER) {
            Ok(Some(key)) => Some(
                Gemini::new(key)
                    .with_model(&args.model)
                    .with_base_url(&args.gemini_base_url),
            ),
            Ok(None) => {
                let _ = writeln!(
                    stderr,
                    "erro: sem chave do Gemini; guarde uma com `fala-cli key set gemini`"
                );
                return 2;
            }
            Err(error @ (SecretError::Unavailable | SecretError::Store)) => {
                let _ = writeln!(stderr, "erro: keyring indisponível ({error})");
                return 1;
            }
            Err(error) => {
                let _ = writeln!(stderr, "erro: {error}");
                return 2;
            }
        }
    } else {
        None
    };
    let mut text = String::new();
    if let Err(error) = stdin.read_to_string(&mut text) {
        let _ = writeln!(stderr, "erro: não consegui ler o stdin: {error}");
        return 2;
    }
    let processor = Postprocessor::new(LlmConfig {
        enabled: args.llm,
        gemini,
        disabled_apps: args.disable_app,
    });
    let formatted = processor.process(
        Transcript { text, language },
        AppContext { app_name: args.app },
        &dictionary,
    );
    let editor = match formatted.dictation.editor {
        Editor::Llm => "llm",
        Editor::Rules | Editor::None => "regras",
    };
    let _ = writeln!(stdout, "{}", formatted.dictation.final_text);
    let _ = writeln!(stderr, "editor: {editor}");
    if let Some(fallback) = formatted.fallback {
        let _ = writeln!(stderr, "fallback: {fallback}");
    }
    0
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, ErrorKind};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    use clap::Parser;
    use fala_secrets::{ApiKey, MemoryStore};

    use super::*;
    use crate::key::tests::{RefusingStore, UnavailableStore};

    const SIXTEEN: &str =
        "eu acho que a gente pode mandar o relatório amanhã cedo para o time inteiro hoje";

    struct Outcome {
        code: u8,
        stdout: String,
        stderr: String,
    }

    fn format(store: &dyn SecretStore, argv: &[&str], stdin: &str) -> Outcome {
        format_bytes(store, argv, stdin.as_bytes())
    }

    fn format_bytes(store: &dyn SecretStore, argv: &[&str], stdin: &[u8]) -> Outcome {
        #[derive(Parser)]
        struct Wrapper {
            #[command(flatten)]
            args: FormatArgs,
        }
        let mut full = vec!["format"];
        full.extend_from_slice(argv);
        let args = Wrapper::try_parse_from(full).unwrap().args;
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(args, store, &mut &stdin[..], &mut out, &mut err);
        Outcome {
            code,
            stdout: String::from_utf8(out).unwrap(),
            stderr: String::from_utf8(err).unwrap(),
        }
    }

    fn store_with_key() -> MemoryStore {
        let store = MemoryStore::default();
        store
            .set(GEMINI_PROVIDER, &ApiKey::new("chave-de-teste").unwrap())
            .unwrap();
        store
    }

    /// Um servidor que aceita conexões mas que o teste só inspeciona.
    fn idle_server() -> (TcpListener, String) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        (listener, url)
    }

    fn refused_url() -> String {
        let (listener, url) = idle_server();
        drop(listener);
        url
    }

    #[test]
    fn prints_final_text_and_editor() {
        let dir = std::env::temp_dir().join(format!("fala-cli-format-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let dict = dir.join("dicionario.txt");
        std::fs::write(&dict, "ChargeBee\n").unwrap();
        let out = format(
            &MemoryStore::default(),
            &["--dictionary", dict.to_str().unwrap()],
            "hã eu eu eu acho que a charge bee",
        );
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(out.code, 0);
        assert_eq!(out.stdout, "Eu acho que a ChargeBee\n");
        assert!(out.stderr.contains("editor: regras"), "{}", out.stderr);
    }

    #[test]
    fn fallback_still_exits_0() {
        let url = refused_url();
        let out = format(
            &store_with_key(),
            &["--llm", "--gemini-base-url", &url],
            SIXTEEN,
        );
        assert_eq!(out.code, 0);
        assert_eq!(
            out.stdout,
            "Eu acho que a gente pode mandar o relatório amanhã cedo para o time inteiro hoje\n"
        );
        assert!(out.stderr.contains("editor: regras"), "{}", out.stderr);
        assert!(out.stderr.contains("fallback: rede"), "{}", out.stderr);
    }

    #[test]
    fn llm_without_key_exits_2() {
        let (listener, url) = idle_server();
        let out = format(
            &MemoryStore::default(),
            &["--llm", "--gemini-base-url", &url],
            SIXTEEN,
        );
        assert_eq!(out.code, 2);
        assert!(
            out.stderr.contains("fala-cli key set gemini"),
            "{}",
            out.stderr
        );
        assert_eq!(out.stdout, "");
        let accepted = listener.accept().map_err(|e| e.kind());
        assert_eq!(accepted.err(), Some(ErrorKind::WouldBlock));
    }

    #[test]
    fn unavailable_keyring_exits_1() {
        let out = format(&UnavailableStore, &["--llm"], SIXTEEN);
        assert_eq!(out.code, 1);
        assert!(
            out.stderr.contains("keyring indisponível"),
            "{}",
            out.stderr
        );
        assert_eq!(out.stdout, "");
    }

    /// Request line e corpo de cada request recebida.
    type Seen = Arc<Mutex<Vec<(String, String)>>>;

    /// Um Gemini falso que responde `text` a toda request e guarda request line e corpo.
    fn answering_server(text: &str) -> (String, Seen) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&seen);
        let body = format!(r#"{{"candidates":[{{"content":{{"parts":[{{"text":"{text}"}}]}}}}]}}"#);
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    let line = line.trim_end().to_ascii_lowercase();
                    if line.is_empty() {
                        break;
                    }
                    if let Some(value) = line.strip_prefix("content-length:") {
                        length = value.trim().parse().unwrap();
                    }
                }
                let mut request_body = vec![0; length];
                reader.read_exact(&mut request_body).unwrap();
                sink.lock().unwrap().push((
                    request_line.trim_end().to_string(),
                    String::from_utf8(request_body).unwrap(),
                ));
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (url, seen)
    }

    #[test]
    fn refused_keyring_exits_1() {
        let out = format(&RefusingStore, &["--llm"], SIXTEEN);
        assert_eq!(out.code, 1);
        assert!(
            out.stderr.contains("keyring indisponível"),
            "{}",
            out.stderr
        );
        assert!(!out.stderr.contains("relatório"), "{}", out.stderr);
        assert_eq!(out.stdout, "");
    }

    #[test]
    fn non_utf8_stdin_exits_2() {
        let out = format_bytes(&MemoryStore::default(), &[], &[0xff, 0xfe, b'\n']);
        assert_eq!(out.code, 2);
        assert_eq!(out.stdout, "");
        assert!(out.stderr.contains("stdin"), "{}", out.stderr);
    }

    #[test]
    fn unreadable_dictionary_or_bad_language_exits_2() {
        let missing = std::env::temp_dir().join("fala-cli-nao-existe/dicionario.txt");
        let out = format(
            &MemoryStore::default(),
            &["--dictionary", missing.to_str().unwrap()],
            "oi",
        );
        assert_eq!(out.code, 2);
        assert_eq!(out.stdout, "");
        let out = format(&MemoryStore::default(), &["--lang", "es"], "oi");
        assert_eq!(out.code, 2);
        assert_eq!(out.stdout, "");
    }

    #[test]
    fn llm_answer_prints_editor_llm() {
        let (url, seen) = answering_server("Texto do LLM.");
        let out = format(
            &store_with_key(),
            &["--llm", "--gemini-base-url", &url],
            SIXTEEN,
        );
        assert_eq!(out.code, 0);
        assert_eq!(out.stdout, "Texto do LLM.\n");
        assert!(out.stderr.contains("editor: llm"), "{}", out.stderr);
        assert!(!out.stderr.contains("fallback:"), "{}", out.stderr);
        assert_eq!(seen.lock().unwrap().len(), 1);
    }

    #[test]
    fn flags_reach_the_postprocessor() {
        let (url, seen) = answering_server("Texto do LLM.");
        let out = format(
            &store_with_key(),
            &[
                "--llm",
                "--gemini-base-url",
                &url,
                "--app",
                "Slack",
                "--model",
                "gemini-3.5-flash-lite",
            ],
            SIXTEEN,
        );
        assert_eq!(out.code, 0);
        {
            let seen = seen.lock().unwrap();
            assert_eq!(seen.len(), 1);
            assert_eq!(
                seen[0].0,
                "POST /v1beta/models/gemini-3.5-flash-lite:generateContent HTTP/1.1"
            );
            assert!(seen[0].1.contains("<app>Slack</app>"), "{}", seen[0].1);
        }

        let out = format(
            &store_with_key(),
            &[
                "--llm",
                "--gemini-base-url",
                &url,
                "--app",
                "Slack",
                "--disable-app",
                "slack",
            ],
            SIXTEEN,
        );
        assert_eq!(out.code, 0);
        assert!(out.stderr.contains("editor: regras"), "{}", out.stderr);
        assert_eq!(seen.lock().unwrap().len(), 1);

        let pt = format(&MemoryStore::default(), &[], "ahn ok");
        assert_eq!(pt.stdout, "Ok\n");
        let en = format(&MemoryStore::default(), &["--lang", "en"], "ahn ok");
        assert_eq!(en.stdout, "Ahn ok\n");
    }
}
