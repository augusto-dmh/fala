//! O backend Scribe contra um servidor falso em `127.0.0.1`: o que sairia da máquina, provado
//! byte a byte na requisição crua. Nenhum teste fala com a rede de verdade nem usa chave real.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use fala_asr::meeting::{
    CancelToken, ElevenLabsScribe, MeetingRecording, MeetingTranscriber, Progress, ScribeOptions,
    Segment,
};
use fala_asr::AsrError;
use fala_core::Dictionary;
use fala_meeting::SessionMode;
use fala_secrets::ApiKey;

const KEY: &str = "sk-teste-123";
const MIC_BYTES: &[u8] = b"OggS\x00\x02mic-channel\r\n--not-a-boundary\x00\xff\xfe";
const SYS_BYTES: &[u8] = b"OggS\x00\x02system-channel\x01\x02\x03\r\n\r\n";

/// Uma resposta da Scribe com uma palavra.
const OK_JSON: &str = r#"{"language_code":"por","text":"oi","words":[{"text":"oi","start":0.0,"end":0.5,"type":"word","speaker_id":"speaker_0"}]}"#;

/// Uma requisição como chegou no socket.
#[derive(Debug, Clone)]
struct Recorded {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Recorded {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// As partes do multipart, em ordem: (nome, bytes).
    fn parts(&self) -> Vec<(String, Vec<u8>)> {
        let content_type = self.header("content-type").unwrap();
        let boundary = content_type
            .split("boundary=")
            .nth(1)
            .expect("content-type sem boundary");
        let delimiter = format!("--{boundary}").into_bytes();
        let mut parts = Vec::new();
        let chunks = split(&self.body, &delimiter);
        assert!(chunks[0].is_empty(), "lixo antes do primeiro boundary");
        let last = chunks.last().unwrap();
        assert!(last.starts_with(b"--"), "multipart sem o boundary final");
        for chunk in &chunks[1..chunks.len() - 1] {
            let chunk = chunk.strip_prefix(b"\r\n").unwrap();
            let chunk = chunk.strip_suffix(b"\r\n").unwrap();
            let split_at = find(chunk, b"\r\n\r\n").unwrap();
            let head = String::from_utf8(chunk[..split_at].to_vec()).unwrap();
            let data = chunk[split_at + 4..].to_vec();
            let name = head
                .split("name=\"")
                .nth(1)
                .and_then(|rest| rest.split('"').next())
                .unwrap()
                .to_string();
            parts.push((name, data));
        }
        parts
    }

    fn field(&self, name: &str) -> Vec<String> {
        self.parts()
            .into_iter()
            .filter(|(n, _)| n == name)
            .map(|(_, data)| String::from_utf8(data).unwrap())
            .collect()
    }

    fn file(&self) -> Vec<u8> {
        let files: Vec<_> = self
            .parts()
            .into_iter()
            .filter(|(n, _)| n == "file")
            .collect();
        assert_eq!(files.len(), 1);
        files[0].1.clone()
    }

    fn field_names(&self) -> std::collections::BTreeSet<String> {
        self.parts().into_iter().map(|(n, _)| n).collect()
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn split(mut data: &[u8], delimiter: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    while let Some(at) = find(data, delimiter) {
        out.push(data[..at].to_vec());
        data = &data[at + delimiter.len()..];
    }
    out.push(data.to_vec());
    out
}

#[derive(Clone)]
enum Reply {
    /// Status e corpo JSON depois de `delay`.
    Json {
        status: u16,
        body: String,
        delay: Duration,
    },
    /// Lê a requisição e fecha sem responder.
    Close,
}

fn ok() -> Reply {
    Reply::Json {
        status: 200,
        body: OK_JSON.to_string(),
        delay: Duration::ZERO,
    }
}

struct FakeServer {
    url: String,
    requests: Arc<Mutex<Vec<Recorded>>>,
    connections: Arc<AtomicUsize>,
}

impl FakeServer {
    fn start(reply: Reply) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let connections = Arc::new(AtomicUsize::new(0));
        let (seen, count) = (Arc::clone(&requests), Arc::clone(&connections));
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                count.fetch_add(1, Ordering::SeqCst);
                let (seen, reply) = (Arc::clone(&seen), reply.clone());
                thread::spawn(move || handle(stream, &seen, &reply));
            }
        });
        Self {
            url,
            requests,
            connections,
        }
    }

    fn requests(&self) -> Vec<Recorded> {
        self.requests.lock().unwrap().clone()
    }

    fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }
}

fn handle(stream: TcpStream, seen: &Mutex<Vec<Recorded>>, reply: &Reply) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    if reader.read_line(&mut line).unwrap_or(0) == 0 {
        return;
    }
    let mut words = line.split_whitespace();
    let method = words.next().unwrap_or_default().to_string();
    let target = words.next().unwrap_or_default().to_string();
    let mut headers = Vec::new();
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).unwrap();
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        let (name, value) = header.split_once(':').unwrap();
        headers.push((name.trim().to_string(), value.trim().to_string()));
    }
    let get = |name: &str| {
        headers
            .iter()
            .find(|(n, _): &&(String, String)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.clone())
    };
    let mut body = Vec::new();
    if let Some(len) = get("content-length") {
        body.resize(len.parse().unwrap(), 0);
        reader.read_exact(&mut body).unwrap();
    } else if get("transfer-encoding").is_some_and(|v| v.contains("chunked")) {
        loop {
            let mut size = String::new();
            reader.read_line(&mut size).unwrap();
            let size = usize::from_str_radix(size.trim(), 16).unwrap();
            let mut chunk = vec![0; size + 2];
            reader.read_exact(&mut chunk).unwrap();
            if size == 0 {
                break;
            }
            body.extend_from_slice(&chunk[..size]);
        }
    }
    seen.lock().unwrap().push(Recorded {
        method,
        target,
        headers,
        body,
    });
    let mut stream = stream;
    match reply {
        Reply::Close => {}
        Reply::Json {
            status,
            body,
            delay,
        } => {
            thread::sleep(*delay);
            let response = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    }
}

/// Uma pasta de sessão com `mic.opus` e `sys.opus`.
fn session(name: &str, mode: SessionMode) -> MeetingRecording {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("meeting-scribe")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("mic.opus"), MIC_BYTES).unwrap();
    std::fs::write(dir.join("sys.opus"), SYS_BYTES).unwrap();
    MeetingRecording::from_session_dir(&dir, mode).unwrap()
}

fn scribe(server: &FakeServer, options: ScribeOptions) -> ElevenLabsScribe {
    ElevenLabsScribe::new(ApiKey::new(KEY).unwrap(), options).with_base_url(&server.url)
}

fn no_keyterms() -> ScribeOptions {
    ScribeOptions {
        keyterms: false,
        ..ScribeOptions::default()
    }
}

fn run(
    scribe: &mut ElevenLabsScribe,
    recording: &MeetingRecording,
) -> Result<Vec<Segment>, AsrError> {
    scribe.transcribe_session(recording, &CancelToken::new(), &mut |_: Progress| {})
}

mod request {
    use super::*;

    #[test]
    fn meeting_sends_exactly_the_enumerated_fields() {
        let server = FakeServer::start(ok());
        let recording = session("meeting-fields", SessionMode::Meeting);
        run(&mut scribe(&server, no_keyterms()), &recording).unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(server.connections(), 2);
        for r in &requests {
            assert_eq!(r.method, "POST");
            assert_eq!(r.target, "/v1/speech-to-text");
            assert_eq!(r.field("model_id"), ["scribe_v2"]);
            assert_eq!(r.field("language_code"), ["por"]);
            assert_eq!(r.field("timestamps_granularity"), ["word"]);
            let names: Vec<String> = r.field_names().into_iter().collect();
            assert_eq!(
                names,
                [
                    "diarize",
                    "file",
                    "language_code",
                    "model_id",
                    "timestamps_granularity"
                ]
            );
            assert_eq!(r.parts().len(), 5, "campo repetido");
        }
        let mic = requests.iter().find(|r| r.file() == MIC_BYTES).unwrap();
        assert_eq!(mic.field("diarize"), ["false"]);
        let sys = requests.iter().find(|r| r.file() == SYS_BYTES).unwrap();
        assert_eq!(sys.field("diarize"), ["true"]);
    }

    #[test]
    fn keyterms_follow_the_setting() {
        let on = ScribeOptions {
            keyterms: true,
            dictionary: Dictionary::new(["Fala", "ADR"]),
            ..ScribeOptions::default()
        };
        let server = FakeServer::start(ok());
        run(
            &mut scribe(&server, on.clone()),
            &session("keyterms-on", SessionMode::Meeting),
        )
        .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        for r in &requests {
            assert_eq!(r.field("keyterms"), ["Fala", "ADR"]);
        }

        let off = ScribeOptions {
            keyterms: false,
            ..on
        };
        let server = FakeServer::start(ok());
        run(
            &mut scribe(&server, off),
            &session("keyterms-off", SessionMode::Meeting),
        )
        .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        for r in &requests {
            assert!(r.field("keyterms").is_empty());
        }
    }

    #[test]
    fn in_person_diarizes_the_mic() {
        let server = FakeServer::start(ok());
        let recording = session("in-person", SessionMode::InPerson);
        run(&mut scribe(&server, no_keyterms()), &recording).unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        let mic = requests.iter().find(|r| r.file() == MIC_BYTES).unwrap();
        assert_eq!(mic.field("diarize"), ["true"]);
        let sys = requests.iter().find(|r| r.file() == SYS_BYTES).unwrap();
        assert_eq!(sys.field("diarize"), ["true"]);
    }

    #[test]
    fn auto_language_omits_language_code() {
        let server = FakeServer::start(ok());
        let detect = ScribeOptions {
            language: None,
            ..no_keyterms()
        };
        run(
            &mut scribe(&server, detect),
            &session("auto-language", SessionMode::Meeting),
        )
        .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        for r in &requests {
            assert!(!r.field_names().contains("language_code"), "{r:?}");
        }
    }

    #[test]
    fn key_only_in_header_and_never_in_errors() {
        let server = FakeServer::start(ok());
        run(
            &mut scribe(&server, no_keyterms()),
            &session("key-header", SessionMode::Meeting),
        )
        .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        for r in &requests {
            assert_eq!(r.header("xi-api-key"), Some(KEY));
            assert!(!r.target.contains(KEY));
            assert!(find(&r.body, KEY.as_bytes()).is_none());
            let elsewhere = r
                .headers
                .iter()
                .filter(|(n, _)| !n.eq_ignore_ascii_case("xi-api-key"))
                .any(|(n, v)| n.contains(KEY) || v.contains(KEY));
            assert!(!elsewhere, "a chave vazou para outro header");
        }

        // Um 401 cujo corpo ecoa a chave: o erro não a carrega.
        let server = FakeServer::start(Reply::Json {
            status: 401,
            body: format!(r#"{{"detail":{{"status":"invalid_api_key","key":"{KEY}"}}}}"#),
            delay: Duration::ZERO,
        });
        let err = run(
            &mut scribe(&server, no_keyterms()),
            &session("key-401", SessionMode::Meeting),
        )
        .unwrap_err();
        assert!(
            matches!(
                err,
                AsrError::Network {
                    status: Some(401),
                    ..
                }
            ),
            "{err:?}"
        );
        assert!(!err.to_string().contains(KEY), "{err}");
        assert!(!format!("{err:?}").contains(KEY), "{err:?}");
    }
}

mod progress {
    use super::*;

    fn reply(status: u16) -> Reply {
        Reply::Json {
            status,
            body: r#"{"detail":"x"}"#.to_string(),
            delay: Duration::ZERO,
        }
    }

    #[test]
    fn failures_keep_audio_and_classify_retry() {
        let cases = [
            ("429", reply(429), true, Some(429)),
            ("503", reply(503), true, Some(503)),
            ("closed", Reply::Close, true, None),
            ("401", reply(401), false, Some(401)),
            ("422", reply(422), false, Some(422)),
        ];
        for (name, reply, want_retriable, want_status) in cases {
            let server = FakeServer::start(reply);
            let recording = session(&format!("failure-{name}"), SessionMode::Meeting);
            let err = run(&mut scribe(&server, no_keyterms()), &recording).unwrap_err();
            match err {
                AsrError::Network {
                    retriable, status, ..
                } => {
                    assert_eq!(retriable, want_retriable, "{name}");
                    if want_status.is_some() {
                        assert_eq!(status, want_status, "{name}");
                    }
                }
                other => panic!("{name}: esperava Network, veio {other:?}"),
            }
            assert_eq!(std::fs::read(recording.mic()).unwrap(), MIC_BYTES, "{name}");
            assert_eq!(
                std::fs::read(recording.system()).unwrap(),
                SYS_BYTES,
                "{name}"
            );
        }
    }

    #[test]
    fn reports_at_least_every_second() {
        let server = FakeServer::start(Reply::Json {
            status: 200,
            body: OK_JSON.to_string(),
            delay: Duration::from_secs(3),
        });
        let recording = session("progress", SessionMode::SystemOnly);
        let start = Instant::now();
        let mut calls: Vec<Instant> = Vec::new();
        scribe(&server, no_keyterms())
            .transcribe_session(&recording, &CancelToken::new(), &mut |_: Progress| {
                calls.push(Instant::now())
            })
            .unwrap();
        let end = Instant::now();
        assert_eq!(server.requests().len(), 1);
        assert!(end - start >= Duration::from_secs(3));
        assert!(calls.len() >= 3, "{} chamadas", calls.len());
        let mut marks = vec![start];
        marks.extend(&calls);
        for pair in marks.windows(2) {
            let gap = pair[1] - pair[0];
            assert!(gap <= Duration::from_secs(1), "intervalo de {gap:?}");
        }
        // Do último sinal até a resposta também não passa de 1 s.
        assert!(end - *calls.last().unwrap() <= Duration::from_secs(1));
    }

    #[test]
    fn cancel_returns_within_a_second() {
        let server = FakeServer::start(Reply::Json {
            status: 200,
            body: OK_JSON.to_string(),
            delay: Duration::from_secs(5),
        });
        let recording = session("cancel", SessionMode::Meeting);
        let cancel = CancelToken::new();
        let trigger = cancel.clone();
        let start = Instant::now();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(500));
            trigger.cancel();
        });
        let err = scribe(&server, no_keyterms())
            .transcribe_session(&recording, &cancel, &mut |_: Progress| {})
            .unwrap_err();
        let elapsed = start.elapsed();
        assert!(matches!(err, AsrError::Cancelled), "{err:?}");
        assert!(elapsed < Duration::from_millis(1_500), "{elapsed:?}");
        // Cancelado no mic: o sistema nem foi enviado.
        assert_eq!(server.requests().len(), 1);
    }
}
