//! Confirmação da ADR-0016 (proposta): um servidor HTTP falso em `127.0.0.1` captura o que sai
//! da máquina e responde com notas, erro, recusa ou lixo. Nenhum teste toca a rede, e a chave é
//! falsa.

// Os ajudantes do servidor falso não são `#[test]`, então o `allow-unwrap-in-tests` não os cobre.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use fala_core::{Dictionary, Language};
use fala_notes::{
    builtin_templates, generate_notes, ApiKey, Channel, Claude, KeySource, NotesError, NotesInput,
    NotesPayload, Segment, Speaker, GENERATED_MARKER, SYSTEM_PROMPT,
};
use serde_json::{json, Value};

const TEST_KEY: &str = "chave-falsa-de-teste-123";

#[derive(Debug, Clone)]
struct Captured {
    request_line: String,
    headers: Vec<(String, String)>,
    body: String,
}

impl Captured {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap()
    }
}

/// Espera e responde status + corpo.
#[derive(Clone)]
struct Reply {
    delay: Duration,
    status: u16,
    body: String,
}

struct FakeServer {
    base_url: String,
    connections: Arc<AtomicUsize>,
    captured: Arc<Mutex<Vec<Captured>>>,
}

impl FakeServer {
    fn start(reply: Reply) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let connections = Arc::new(AtomicUsize::new(0));
        let captured = Arc::new(Mutex::new(Vec::new()));
        let (count, sink) = (Arc::clone(&connections), Arc::clone(&captured));
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                count.fetch_add(1, Ordering::SeqCst);
                let sink = Arc::clone(&sink);
                let reply = reply.clone();
                thread::spawn(move || serve(stream, &sink, reply));
            }
        });
        Self {
            base_url,
            connections,
            captured,
        }
    }

    fn ok(text: &str) -> Self {
        Self::start(Reply {
            delay: Duration::ZERO,
            status: 200,
            body: message("end_turn", text),
        })
    }

    fn requests(&self) -> Vec<Captured> {
        self.captured.lock().unwrap().clone()
    }

    fn connections(&self) -> usize {
        // Uma conexão aberta por engano é aceita em paralelo; dá tempo ao accept.
        thread::sleep(Duration::from_millis(100));
        self.connections.load(Ordering::SeqCst)
    }

    fn claude(&self, keys: &Arc<CountingKeys>) -> Claude {
        Claude::new(Arc::clone(keys) as Arc<dyn KeySource>).with_base_url(&self.base_url)
    }
}

fn serve(stream: TcpStream, sink: &Mutex<Vec<Captured>>, reply: Reply) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request_line = String::new();
    reader.read_line(&mut request_line).unwrap();
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }
    let length = headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("content-length"))
        .map(|(_, v)| v.parse::<usize>().unwrap())
        .unwrap_or(0);
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    sink.lock().unwrap().push(Captured {
        request_line: request_line.trim_end().to_string(),
        headers,
        body: String::from_utf8(body).unwrap(),
    });
    thread::sleep(reply.delay);
    let mut stream = stream;
    let _ = write!(
        stream,
        "HTTP/1.1 {} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        reply.status,
        reply.body.len(),
        reply.body
    );
}

/// Uma resposta da Messages API com um bloco de texto.
fn message(stop_reason: &str, text: &str) -> String {
    json!({
        "id": "msg_falso",
        "type": "message",
        "role": "assistant",
        "content": [{ "type": "text", "text": text }],
        "stop_reason": stop_reason,
    })
    .to_string()
}

const SECTIONS: &str = r#"{"sections":[{"title":"Resumo","lines":[{"text":"Lançamento em 15/11.","sources":["s12","a1"]}]}]}"#;

/// Conta as consultas e devolve a chave falsa (ou nenhuma).
struct CountingKeys {
    key: Option<&'static str>,
    calls: AtomicUsize,
}

impl CountingKeys {
    fn new(key: Option<&'static str>) -> Arc<Self> {
        Arc::new(Self {
            key,
            calls: AtomicUsize::new(0),
        })
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl KeySource for CountingKeys {
    fn api_key(&self, provider: &str) -> Result<Option<ApiKey>, NotesError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(provider, "anthropic");
        self.key.map(ApiKey::new).transpose()
    }
}

fn segment(id: u32, speaker: Speaker, t0_ms: u64, text: &str) -> Segment {
    Segment {
        id,
        channel: if speaker == Speaker::Me {
            Channel::Mic
        } else {
            Channel::System
        },
        speaker,
        t0_ms,
        t1_ms: t0_ms + 4_000,
        text: text.to_string(),
    }
}

fn input() -> NotesInput {
    NotesInput {
        title: "Planejamento do lançamento".to_string(),
        date: "2026-10-02".to_string(),
        start_time: "14:02".to_string(),
        language: Language::PtBr,
        dictionary: Dictionary::new(["Fala"]),
        template: builtin_templates().unwrap().remove(0),
        annotations: "decidir data\nAna: contrato".to_string(),
        transcript: vec![
            segment(3, Speaker::Me, 0, "Vamos falar do lançamento."),
            segment(12, Speaker::Person(1), 65_000, "Proponho 15 de novembro."),
        ],
        local_only: false,
    }
}

fn keys(value: &Value) -> BTreeSet<&str> {
    value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

#[test]
fn request_carries_key_version_and_exact_body() {
    let server = FakeServer::ok(SECTIONS);
    let keys_source = CountingKeys::new(Some(TEST_KEY));
    let input = input();
    let notes = generate_notes(&input, &server.claude(&keys_source)).unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(request.request_line, "POST /v1/messages HTTP/1.1");
    assert_eq!(request.header("x-api-key"), Some(TEST_KEY));
    assert_eq!(request.header("anthropic-version"), Some("2023-06-01"));
    assert_eq!(request.header("content-type"), Some("application/json"));

    let body = request.json();
    assert_eq!(
        keys(&body),
        BTreeSet::from(["model", "max_tokens", "system", "messages", "output_config"])
    );
    assert!(body.get("tools").is_none());
    assert_eq!(body["system"], SYSTEM_PROMPT);
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["role"], "user");
    let expected = NotesPayload::build(&input).unwrap().to_json().unwrap();
    assert_eq!(messages[0]["content"], expected.as_str());
    assert_eq!(body["output_config"]["format"]["type"], "json_schema");

    assert!(notes.generated);
    assert!(
        notes
            .markdown
            .contains("- Lançamento em 15/11. [[#^s12|01:05]] [[#^a1|a1]] <!-- fala:ia -->"),
        "{}",
        notes.markdown
    );
    assert_eq!(keys_source.calls(), 1);
}

#[test]
fn default_model_is_sonnet_5() {
    let server = FakeServer::ok(SECTIONS);
    let keys_source = CountingKeys::new(Some(TEST_KEY));
    generate_notes(&input(), &server.claude(&keys_source)).unwrap();
    generate_notes(
        &input(),
        &server.claude(&keys_source).with_model("claude-opus-5"),
    )
    .unwrap();
    let models: Vec<Value> = server
        .requests()
        .iter()
        .map(|r| r.json()["model"].clone())
        .collect();
    assert_eq!(models, [json!("claude-sonnet-5"), json!("claude-opus-5")]);
}

#[test]
fn local_only_never_connects_nor_reads_key() {
    let server = FakeServer::ok(SECTIONS);
    let keys_source = CountingKeys::new(Some(TEST_KEY));
    let mut input = input();
    input.local_only = true;
    let notes = generate_notes(&input, &server.claude(&keys_source)).unwrap();
    assert_eq!(server.connections(), 0, "o servidor recebeu conexão");
    assert!(server.requests().is_empty());
    assert_eq!(keys_source.calls(), 0);
    assert!(!notes.generated);
    assert_eq!(
        notes.markdown,
        "## Anotações\n\ndecidir data ^a1\n\nAna: contrato ^a2\n"
    );
    assert!(!notes.markdown.contains(GENERATED_MARKER));
}

#[test]
fn unmarking_local_only_reaches_server() {
    let server = FakeServer::ok(SECTIONS);
    let keys_source = CountingKeys::new(Some(TEST_KEY));
    let claude = server.claude(&keys_source);
    let mut input = input();
    input.local_only = true;
    assert!(!generate_notes(&input, &claude).unwrap().generated);
    assert_eq!(server.connections(), 0);
    input.local_only = false;
    assert!(generate_notes(&input, &claude).unwrap().generated);
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn missing_key_never_connects() {
    let server = FakeServer::ok(SECTIONS);
    let keys_source = CountingKeys::new(None);
    assert_eq!(
        generate_notes(&input(), &server.claude(&keys_source)),
        Err(NotesError::MissingKey)
    );
    assert_eq!(server.connections(), 0);
    assert_eq!(keys_source.calls(), 1);
}

#[test]
fn http_error_maps_status_without_body() {
    for status in [401, 429, 500] {
        let server = FakeServer::start(Reply {
            delay: Duration::ZERO,
            status,
            body: r#"{"error":{"message":"SENTINELA-DO-CORPO decidir data"}}"#.to_string(),
        });
        let keys_source = CountingKeys::new(Some(TEST_KEY));
        let error = generate_notes(&input(), &server.claude(&keys_source)).unwrap_err();
        assert_eq!(error, NotesError::Http(status));
        let shown = format!("{error} {error:?}");
        assert!(!shown.contains("SENTINELA"), "{shown}");
        assert!(!shown.contains(TEST_KEY), "{shown}");
    }
}

#[test]
fn slow_server_times_out() {
    let server = FakeServer::start(Reply {
        delay: Duration::from_secs(2),
        status: 200,
        body: message("end_turn", SECTIONS),
    });
    let keys_source = CountingKeys::new(Some(TEST_KEY));
    let claude = server
        .claude(&keys_source)
        .with_timeout(Duration::from_millis(300));
    assert_eq!(generate_notes(&input(), &claude), Err(NotesError::Timeout));
}

#[test]
fn refused_connection_is_network() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let keys_source = CountingKeys::new(Some(TEST_KEY));
    let claude = Claude::new(keys_source as Arc<dyn KeySource>).with_base_url(base_url);
    assert_eq!(generate_notes(&input(), &claude), Err(NotesError::Network));
}

#[test]
fn refusal_truncation_and_garbage_map_to_errors() {
    let cases = [
        (message("refusal", ""), NotesError::Refused),
        (
            message("max_tokens", "{\"sections\":["),
            NotesError::Truncated,
        ),
        (
            message("end_turn", "Aqui estão as notas: ..."),
            NotesError::InvalidResponse,
        ),
        ("não é json".to_string(), NotesError::InvalidResponse),
    ];
    for (body, expected) in cases {
        let server = FakeServer::start(Reply {
            delay: Duration::ZERO,
            status: 200,
            body: body.clone(),
        });
        let keys_source = CountingKeys::new(Some(TEST_KEY));
        assert_eq!(
            generate_notes(&input(), &server.claude(&keys_source)),
            Err(expected),
            "{body}"
        );
    }
}
