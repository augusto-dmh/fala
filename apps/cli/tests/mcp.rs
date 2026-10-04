//! `fala-cli mcp` pela fronteira: sobe o binário, fala JSON-RPC pelo stdin/stdout e confere as
//! respostas, o exit code, o stderr e o que ficou em disco.

// `allow-unwrap-in-tests` não cobre os helpers de um crate de teste de integração.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{DateTime, FixedOffset, SecondsFormat};
use fala_core::{AppContext, Dictation, Editor, Language, Transcript};
use fala_storage::Store;
use serde_json::{json, Value};

const DISABLED: &str =
    "Fala MCP is disabled: set enabled = true in mcp.toml in the Fala data folder";
const INVALID_PARAMS: i64 = -32602;
const MISSING: &str = "01999999-0000-7000-8000-000000000000";

// ---------------------------------------------------------------- fixture

/// Uma pasta de dados limpa por teste.
struct Data {
    dir: PathBuf,
}

impl Data {
    fn new(name: &str) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("mcp")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Data { dir }
    }

    fn enabled(name: &str) -> Self {
        let data = Data::new(name);
        data.toml("enabled = true\n");
        data
    }

    fn toml(&self, text: &str) {
        fs::write(self.dir.join("mcp.toml"), text).unwrap();
    }

    fn db(&self) -> PathBuf {
        self.dir.join("fala.sqlite")
    }

    fn store(&self) -> Store {
        Store::open(&self.db(), &self.dir.join("notas")).unwrap()
    }

    fn session(&self) -> Session {
        Session::start(
            &["mcp", "--data-dir", self.dir.to_str().unwrap()],
            &[],
            true,
        )
    }
}

fn at(minute: u32) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(&format!("2026-10-01T10:{minute:02}:00-03:00")).unwrap()
}

fn plain(raw: &str) -> Dictation {
    Dictation {
        raw: Transcript {
            text: raw.to_string(),
            language: Language::PtBr,
        },
        final_text: raw.to_string(),
        editor: Editor::None,
        app: AppContext { app_name: None },
    }
}

fn edited(raw: &str, final_text: &str, app: &str) -> Dictation {
    Dictation {
        raw: Transcript {
            text: raw.to_string(),
            language: Language::PtBr,
        },
        final_text: final_text.to_string(),
        editor: Editor::Llm,
        app: AppContext {
            app_name: Some(app.to_string()),
        },
    }
}

// ---------------------------------------------------------------- session

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {},
    })
}

struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    rx: Receiver<String>,
    lines: Vec<String>,
    next_id: u64,
    /// Manda o `_meta` de 2026-07-28 em cada request.
    modern: bool,
}

struct Finished {
    status: ExitStatus,
    elapsed: Duration,
    stdout: Vec<String>,
    stderr: String,
}

impl Session {
    fn start(args: &[&str], envs: &[(&str, &Path)], modern: bool) -> Self {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_fala-cli"));
        cmd.args(args)
            .env_remove("RUST_LOG")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in envs {
            cmd.env(key, value);
        }
        let mut child = cmd.spawn().unwrap();
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        Session {
            child,
            stdin,
            rx,
            lines: Vec::new(),
            next_id: 0,
            modern,
        }
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        let mut params = params;
        if self.modern {
            params["_meta"] = meta();
        }
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        self.response(id)
    }

    fn response(&mut self, id: u64) -> Value {
        loop {
            let line = self
                .rx
                .recv_timeout(Duration::from_secs(10))
                .expect("sem resposta do servidor em 10 s");
            self.lines.push(line.clone());
            let value: Value = serde_json::from_str(&line).unwrap();
            if value["id"] == json!(id) {
                return value;
            }
        }
    }

    fn discover(&mut self) -> Value {
        self.request("server/discover", json!({}))
    }

    fn call(&mut self, tool: &str, arguments: Value) -> Value {
        self.request(
            "tools/call",
            json!({ "name": tool, "arguments": arguments }),
        )
    }

    /// `structuredContent` de uma `tools/call` que deu certo.
    fn ok(&mut self, tool: &str, arguments: Value) -> Value {
        let response = self.call(tool, arguments);
        assert_eq!(response["result"]["isError"], json!(false), "{response}");
        response["result"]["structuredContent"].clone()
    }

    fn search(&mut self, arguments: Value) -> Vec<Value> {
        self.ok("search_dictations", arguments)["items"]
            .as_array()
            .unwrap()
            .clone()
    }

    /// Fecha o stdin e espera o processo sair, no máximo 5 s.
    fn finish(mut self) -> Finished {
        drop(self.stdin.take());
        let started = Instant::now();
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            if started.elapsed() > Duration::from_secs(5) {
                let _ = self.child.kill();
                panic!("o servidor não saiu em 5 s depois do EOF");
            }
            thread::sleep(Duration::from_millis(10));
        };
        let elapsed = started.elapsed();
        while let Ok(line) = self.rx.recv_timeout(Duration::from_secs(2)) {
            self.lines.push(line);
        }
        let mut stderr = String::new();
        self.child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        Finished {
            status,
            elapsed,
            stdout: self.lines,
            stderr,
        }
    }
}

fn ids(items: &[Value]) -> Vec<String> {
    items
        .iter()
        .map(|i| i["id"].as_str().unwrap().to_string())
        .collect()
}

fn keys(value: &Value) -> BTreeSet<String> {
    value.as_object().unwrap().keys().cloned().collect()
}

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}

fn tool_text(response: &Value) -> String {
    response["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string()
}

// ---------------------------------------------------------------- S1

#[test]
fn discover_reports_fala_with_tools_and_resources() {
    let data = Data::enabled("discover");
    let mut s = data.session();
    let result = s.discover()["result"].clone();
    assert!(
        result["supportedVersions"]
            .as_array()
            .unwrap()
            .contains(&json!("2026-07-28")),
        "{result}"
    );
    assert!(result["capabilities"]["tools"].is_object(), "{result}");
    assert!(result["capabilities"]["resources"].is_object(), "{result}");
    assert_eq!(
        result["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        json!("fala")
    );
    s.finish();
}

#[test]
fn initialize_legacy_reports_fala() {
    let data = Data::enabled("initialize");
    let mut s = Session::start(
        &["mcp", "--data-dir", data.dir.to_str().unwrap()],
        &[],
        false,
    );
    let response = s.request(
        "initialize",
        json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "0" },
        }),
    );
    let result = &response["result"];
    assert_eq!(result["serverInfo"]["name"], json!("fala"), "{response}");
    assert!(result["capabilities"]["tools"].is_object(), "{response}");
    assert!(
        result["capabilities"]["resources"].is_object(),
        "{response}"
    );
    s.finish();
}

#[test]
fn tools_list_has_two_read_only_tools_in_order() {
    let data = Data::enabled("tools-list");
    let mut s = data.session();
    let tools = s.request("tools/list", json!({}))["result"]["tools"].clone();
    let names: Vec<&str> = tools
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["search_dictations", "get_dictation"]);
    for tool in tools.as_array().unwrap() {
        let a = &tool["annotations"];
        assert_eq!(a["readOnlyHint"], json!(true), "{tool}");
        assert_eq!(a["destructiveHint"], json!(false), "{tool}");
        assert_eq!(a["idempotentHint"], json!(true), "{tool}");
        assert_eq!(a["openWorldHint"], json!(false), "{tool}");
    }
    s.finish();
}

#[test]
fn stdout_is_only_json_rpc_lines() {
    let data = Data::enabled("stdout-json");
    let id = data
        .store()
        .add(&edited("acao de amanha", "Ação de amanhã.", "Slack"), at(1))
        .unwrap()
        .id;
    let mut s = data.session();
    s.discover();
    s.request("tools/list", json!({}));
    s.search(json!({ "query": "acao" }));
    s.ok("get_dictation", json!({ "id": id }));
    s.request(
        "resources/read",
        json!({ "uri": format!("fala://dictation/{id}") }),
    );
    let done = s.finish();
    assert!(done.stdout.len() >= 5, "{:?}", done.stdout);
    for line in &done.stdout {
        let value: Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("linha do stdout não é JSON ({e}): {line}"));
        assert_eq!(value["jsonrpc"], json!("2.0"), "{line}");
    }
    assert!(done.stderr.contains("mcp:"), "o log devia estar no stderr");
}

#[test]
fn eof_exits_zero_within_five_seconds() {
    let data = Data::enabled("eof");
    let mut s = data.session();
    s.discover();
    let done = s.finish();
    assert_eq!(done.status.code(), Some(0), "{}", done.stderr);
    assert!(done.elapsed <= Duration::from_secs(5));

    let silent = data.session().finish();
    assert_eq!(silent.status.code(), Some(0), "{}", silent.stderr);
    assert!(silent.elapsed <= Duration::from_secs(5));
}

#[test]
fn cacheable_results_carry_ttl_and_private_scope() {
    let data = Data::enabled("cacheable");
    let id = data.store().add(&plain("um ditado"), at(1)).unwrap().id;
    let mut s = data.session();
    let uri = format!("fala://dictation/{id}");
    for (method, params) in [
        ("tools/list", json!({})),
        ("resources/list", json!({})),
        ("resources/templates/list", json!({})),
        ("resources/read", json!({ "uri": uri })),
    ] {
        let response = s.request(method, params);
        let result = &response["result"];
        assert!(result["ttlMs"].is_u64(), "{method}: {response}");
        assert_eq!(
            result["cacheScope"],
            json!("private"),
            "{method}: {response}"
        );
    }
    s.finish();
}

#[test]
fn bad_opening_message_exits_one() {
    let data = Data::enabled("bad-opening");
    let mut s = data.session();
    s.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
    let done = s.finish();
    assert_eq!(done.status.code(), Some(1), "{}", done.stderr);
    assert!(
        done.stderr
            .contains("sessão MCP: expect initialized request, but received:"),
        "{}",
        done.stderr
    );

    let mut s = data.session();
    s.modern = false;
    let response = s.request("server/discover", json!({}));
    assert_eq!(
        response["error"]["code"],
        json!(INVALID_PARAMS),
        "{response}"
    );
    let done = s.finish();
    assert_eq!(done.status.code(), Some(1), "{}", done.stderr);
    assert!(
        done.stderr
            .contains("sessão MCP: expect initialized request, but received:"),
        "{}",
        done.stderr
    );
}

// ---------------------------------------------------------------- S2

/// Três ditados: "ação" no final do 1, só no bruto do 2, e nada no 3.
fn three(data: &Data) -> (String, String, String) {
    let store = data.store();
    let a = store
        .add(&edited("acao de amanha", "Ação de amanhã.", "Slack"), at(1))
        .unwrap()
        .id;
    let b = store
        .add(&edited("a ação do contrato", "O contrato.", "Gmail"), at(2))
        .unwrap()
        .id;
    let c = store.add(&plain("reunião de pauta"), at(3)).unwrap().id;
    (a, b, c)
}

#[test]
fn search_finds_without_accent_newest_first() {
    let data = Data::enabled("search-accent");
    let (a, b, _c) = three(&data);
    let mut s = data.session();
    let items = s.search(json!({ "query": "acao" }));
    assert_eq!(ids(&items), [b.clone(), a.clone()]);
    assert_eq!(ids(&s.search(json!({ "query": "amanh" }))), [a]);
    assert_eq!(ids(&s.search(json!({ "query": "acao contr" }))), [b]);
    s.finish();
}

#[test]
fn search_items_have_exactly_the_contract_keys() {
    let data = Data::enabled("search-keys");
    let (a, b, _c) = three(&data);
    data.store().undo(&b).unwrap();
    let mut s = data.session();
    let items = s.search(json!({ "query": "acao" }));
    let contract = set(&[
        "id",
        "created_at",
        "app",
        "language",
        "edited_by",
        "showing",
        "text",
    ]);
    for item in &items {
        assert_eq!(keys(item), contract, "{item}");
    }
    let by_id = |id: &str| items.iter().find(|i| i["id"] == json!(id)).unwrap().clone();
    assert_eq!(by_id(&b)["text"], json!("a ação do contrato"));
    assert_eq!(by_id(&b)["showing"], json!("raw"));
    assert_eq!(by_id(&a)["text"], json!("Ação de amanhã."));
    assert_eq!(by_id(&a)["showing"], json!("final"));
    s.finish();
}

#[test]
fn search_without_query_lists_most_recent() {
    let data = Data::enabled("search-empty");
    let (_a, b, c) = three(&data);
    let mut s = data.session();
    assert_eq!(
        ids(&s.search(json!({ "limit": 2 }))),
        [c.clone(), b.clone()]
    );
    assert_eq!(
        ids(&s.search(json!({ "query": "   ", "limit": 2 }))),
        [c, b]
    );
    s.finish();
}

#[test]
fn search_limit_defaults_to_twenty() {
    let data = Data::enabled("search-default-limit");
    {
        let store = data.store();
        for minute in 0..25 {
            store
                .add(&plain(&format!("item {minute}")), at(minute))
                .unwrap();
        }
    }
    let mut s = data.session();
    assert_eq!(s.search(json!({})).len(), 20);
    s.finish();
}

#[test]
fn search_limit_out_of_range_is_invalid_params() {
    let data = Data::enabled("search-limit");
    three(&data);
    let mut s = data.session();
    for bad in [json!(0), json!(51), json!(-1), json!("5"), json!(5.5)] {
        let response = s.call("search_dictations", json!({ "limit": bad }));
        assert_eq!(
            response["error"]["code"],
            json!(INVALID_PARAMS),
            "{bad}: {response}"
        );
    }
    assert_eq!(s.search(json!({ "limit": 1 })).len(), 1);
    assert_eq!(s.search(json!({ "limit": 50 })).len(), 3);
    s.finish();
}

#[test]
fn get_returns_exactly_the_contract_keys() {
    let data = Data::enabled("get-keys");
    let record = data
        .store()
        .add(&edited("acao de amanha", "Ação de amanhã.", "Slack"), at(1))
        .unwrap();
    let mut s = data.session();
    let got = s.ok("get_dictation", json!({ "id": record.id }));
    assert_eq!(
        keys(&got),
        set(&[
            "id",
            "created_at",
            "app",
            "language",
            "edited_by",
            "showing",
            "final",
            "raw",
            "uri"
        ])
    );
    assert_eq!(got["id"], json!(record.id));
    assert_eq!(got["uri"], json!(format!("fala://dictation/{}", record.id)));
    assert_eq!(
        got["created_at"],
        json!(record
            .created_at
            .to_rfc3339_opts(SecondsFormat::AutoSi, false))
    );
    assert_eq!(got["app"], json!("Slack"));
    assert_eq!(got["language"], json!("pt-BR"));
    assert_eq!(got["edited_by"], json!("llm"));
    assert_eq!(got["showing"], json!("final"));
    assert_eq!(got["final"], json!("Ação de amanhã."));
    assert_eq!(got["raw"], json!("acao de amanha"));
    s.finish();
}

#[test]
fn get_unknown_id_is_not_found_error() {
    let data = Data::enabled("get-missing");
    three(&data);
    let mut s = data.session();
    let response = s.call("get_dictation", json!({ "id": MISSING }));
    assert_eq!(response["result"]["isError"], json!(true), "{response}");
    assert_eq!(
        tool_text(&response),
        format!("dictation not found: {MISSING}")
    );
    s.finish();
}

#[test]
fn get_without_id_or_unknown_tool_is_invalid_params() {
    let data = Data::enabled("get-invalid");
    three(&data);
    let mut s = data.session();
    for response in [
        s.call("get_dictation", json!({})),
        s.call("get_dictation", json!({ "id": 7 })),
        s.call("delete_dictation", json!({ "id": MISSING })),
    ] {
        assert_eq!(
            response["error"]["code"],
            json!(INVALID_PARAMS),
            "{response}"
        );
    }
    s.finish();
}

#[test]
fn templates_list_has_dictation_template() {
    let data = Data::enabled("templates");
    let mut s = data.session();
    let response = s.request("resources/templates/list", json!({}));
    let templates = response["result"]["resourceTemplates"].as_array().unwrap();
    assert_eq!(templates.len(), 1, "{response}");
    assert_eq!(templates[0]["uriTemplate"], json!("fala://dictation/{id}"));
    s.finish();
}

#[test]
fn resources_list_newest_first_with_uri_and_mime() {
    let data = Data::enabled("resources-list");
    let mut all = Vec::new();
    {
        let store = data.store();
        for minute in 0..52 {
            all.push(
                store
                    .add(&plain(&format!("item {minute}")), at(minute))
                    .unwrap()
                    .id,
            );
        }
    }
    let newest = all.last().unwrap().clone();
    let expected: Vec<String> = all
        .iter()
        .rev()
        .take(50)
        .map(|id| format!("fala://dictation/{id}"))
        .collect();
    let mut s = data.session();
    let response = s.request("resources/list", json!({}));
    let resources = response["result"]["resources"].as_array().unwrap();
    assert_eq!(resources.len(), 50);
    assert_eq!(
        resources[0]["uri"],
        json!(format!("fala://dictation/{newest}"))
    );
    let uris: Vec<String> = resources
        .iter()
        .map(|r| r["uri"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(uris, expected);
    for r in resources {
        assert_eq!(r["mimeType"], json!("application/json"), "{r}");
    }
    s.finish();
}

#[test]
fn read_returns_same_object_as_get() {
    let data = Data::enabled("read-get");
    let (a, _b, _c) = three(&data);
    let mut s = data.session();
    let got = s.ok("get_dictation", json!({ "id": a }));
    let response = s.request(
        "resources/read",
        json!({ "uri": format!("fala://dictation/{a}") }),
    );
    let contents = &response["result"]["contents"][0];
    assert_eq!(
        contents["mimeType"],
        json!("application/json"),
        "{response}"
    );
    let read: Value = serde_json::from_str(contents["text"].as_str().unwrap()).unwrap();
    assert_eq!(read, got);
    s.finish();
}

#[test]
fn read_unknown_or_foreign_uri_is_not_found() {
    let data = Data::enabled("read-missing");
    three(&data);
    let mut s = data.session();
    for uri in [
        format!("fala://dictation/{MISSING}"),
        "file:///etc/passwd".to_string(),
        "fala://meeting/x".to_string(),
    ] {
        let response = s.request("resources/read", json!({ "uri": uri }));
        assert_eq!(
            response["error"]["code"],
            json!(INVALID_PARAMS),
            "{uri}: {response}"
        );
    }
    s.finish();
}

// ---------------------------------------------------------------- S3

/// Três normais e um sensível mais recente, com uma palavra só dele.
fn with_sensitive(data: &Data) -> (Vec<String>, String) {
    let store = data.store();
    let normal = (1..=3)
        .map(|m| store.add(&plain(&format!("nota {m}")), at(m)).unwrap().id)
        .collect();
    let secret = store
        .add_sensitive(&plain("o segredo do cofre"), at(4))
        .unwrap()
        .id;
    (normal, secret)
}

#[test]
fn sensitive_is_omitted_from_search_and_limit_is_filled() {
    let data = Data::enabled("sensitive-search");
    let (normal, secret) = with_sensitive(&data);
    let mut s = data.session();
    assert!(s.search(json!({ "query": "segredo" })).is_empty());
    assert!(!ids(&s.search(json!({}))).contains(&secret));
    let newest_three: Vec<String> = normal.iter().rev().cloned().collect();
    assert_eq!(ids(&s.search(json!({ "limit": 3 }))), newest_three);
    s.finish();
}

#[test]
fn sensitive_get_matches_nonexistent() {
    let data = Data::enabled("sensitive-get");
    let (_normal, secret) = with_sensitive(&data);
    let mut s = data.session();
    let sensitive = s.call("get_dictation", json!({ "id": secret }))["result"].clone();
    let missing = s.call("get_dictation", json!({ "id": MISSING }))["result"].clone();
    assert_eq!(sensitive["isError"], json!(true));
    assert_eq!(
        sensitive.to_string().replace(&secret, "<id>"),
        missing.to_string().replace(MISSING, "<id>")
    );
    assert!(sensitive
        .to_string()
        .contains(&format!("dictation not found: {secret}")));
    s.finish();
}

#[test]
fn sensitive_read_matches_nonexistent_and_list_omits() {
    let data = Data::enabled("sensitive-read");
    let (_normal, secret) = with_sensitive(&data);
    let mut s = data.session();
    let sensitive = s.request(
        "resources/read",
        json!({ "uri": format!("fala://dictation/{secret}") }),
    )["error"]
        .clone();
    let missing = s.request(
        "resources/read",
        json!({ "uri": format!("fala://dictation/{MISSING}") }),
    )["error"]
        .clone();
    assert_eq!(sensitive["code"], missing["code"]);
    assert_eq!(
        sensitive["message"]
            .as_str()
            .unwrap()
            .replace(&secret, "<id>"),
        missing["message"]
            .as_str()
            .unwrap()
            .replace(MISSING, "<id>")
    );
    let list = s.request("resources/list", json!({}));
    assert!(!list.to_string().contains(&secret), "{list}");
    s.finish();
}

#[test]
fn responses_never_contain_data_dir_or_sensitive_key() {
    let data = Data::enabled("no-leak");
    let (normal, secret) = with_sensitive(&data);
    let mut s = data.session();
    s.discover();
    s.request("tools/list", json!({}));
    s.search(json!({}));
    s.call("get_dictation", json!({ "id": normal[0] }));
    s.call("get_dictation", json!({ "id": secret }));
    s.call("get_dictation", json!({}));
    s.request("resources/list", json!({}));
    s.request("resources/templates/list", json!({}));
    s.request(
        "resources/read",
        json!({ "uri": format!("fala://dictation/{}", normal[0]) }),
    );
    s.request("resources/read", json!({ "uri": "file:///etc/passwd" }));
    let done = s.finish();
    let dir = data.dir.to_str().unwrap();
    for line in &done.stdout {
        assert!(!line.contains(dir), "caminho vazou: {line}");
        assert!(
            !line.contains("\"sensitive\""),
            "chave sensitive vazou: {line}"
        );
    }
}

#[test]
fn storage_failure_is_opaque_error() {
    let data = Data::enabled("storage-failure");
    let (a, _b, _c) = three(&data);
    let mut s = data.session();
    assert_eq!(s.search(json!({ "query": "acao" })).len(), 2);
    rusqlite::Connection::open(data.db())
        .unwrap()
        .execute_batch("ALTER TABLE dictations RENAME TO broken;")
        .unwrap();
    for response in [
        s.call("search_dictations", json!({ "query": "acao" })),
        s.call("get_dictation", json!({ "id": a })),
    ] {
        assert_eq!(response["result"]["isError"], json!(true), "{response}");
        assert_eq!(tool_text(&response), "storage error");
        assert_eq!(
            response["result"]["content"].as_array().unwrap().len(),
            1,
            "{response}"
        );
        assert!(
            !response.to_string().contains("no such table"),
            "{response}"
        );
    }
    let done = s.finish();
    assert!(
        done.stderr
            .contains("falha ao ler o histórico: banco: no such table"),
        "{}",
        done.stderr
    );
    assert!(done.stdout.iter().all(|l| !l.contains("no such table")));
}

#[test]
fn resource_storage_failure_is_opaque_error() {
    let data = Data::enabled("resource-storage-failure");
    let (a, _b, _c) = three(&data);
    let mut s = data.session();
    assert_eq!(s.search(json!({ "query": "acao" })).len(), 2);
    rusqlite::Connection::open(data.db())
        .unwrap()
        .execute_batch("ALTER TABLE dictations RENAME TO broken;")
        .unwrap();
    for response in [
        s.request("resources/list", json!({})),
        s.request(
            "resources/read",
            json!({ "uri": format!("fala://dictation/{a}") }),
        ),
    ] {
        assert_eq!(response["error"]["code"], json!(-32603), "{response}");
        assert_eq!(
            response["error"]["message"],
            json!("storage error"),
            "{response}"
        );
        assert!(response["error"]["data"].is_null(), "{response}");
    }
    let done = s.finish();
    assert!(
        done.stderr
            .contains("falha ao ler o histórico: banco: no such table"),
        "{}",
        done.stderr
    );
    assert!(done.stdout.iter().all(|l| !l.contains("no such table")));
}

#[test]
fn legacy_read_missing_is_resource_not_found() {
    let data = Data::enabled("legacy-read-missing");
    three(&data);
    let mut s = Session::start(
        &["mcp", "--data-dir", data.dir.to_str().unwrap()],
        &[],
        false,
    );
    s.request(
        "initialize",
        json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "0" },
        }),
    );
    s.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
    let response = s.request(
        "resources/read",
        json!({ "uri": format!("fala://dictation/{MISSING}") }),
    );
    assert_eq!(response["error"]["code"], json!(-32002), "{response}");
    s.finish();
}

// ---------------------------------------------------------------- S4

fn assert_disabled(s: &mut Session) {
    for (tool, args) in [
        ("search_dictations", json!({ "query": "acao" })),
        ("get_dictation", json!({ "id": MISSING })),
    ] {
        let response = s.call(tool, args);
        assert_eq!(response["result"]["isError"], json!(true), "{response}");
        assert_eq!(tool_text(&response), DISABLED);
    }
}

#[test]
fn disabled_without_toml_refuses_every_call() {
    let data = Data::new("disabled");
    three(&data);
    let mut s = data.session();
    assert_disabled(&mut s);
    let list = s.request("resources/list", json!({}));
    assert_eq!(list["result"]["resources"], json!([]), "{list}");
    s.finish();
}

#[test]
fn invalid_or_false_toml_is_disabled() {
    for (name, toml, warning) in [
        (
            "toml-yes",
            "enabled = \"yes\"\n",
            Some("precisa ser true ou false"),
        ),
        ("toml-false", "enabled = false\n", Some("enabled = false")),
        ("toml-missing", "other = 1\n", Some("sem a chave `enabled`")),
        (
            "toml-invalid",
            "enabled = = true\n",
            Some("não é TOML válido"),
        ),
    ] {
        let data = Data::new(name);
        three(&data);
        data.toml(toml);
        let mut s = data.session();
        assert_disabled(&mut s);
        let done = s.finish();
        if let Some(warning) = warning {
            assert!(done.stderr.contains(warning), "{name}: {}", done.stderr);
            assert!(done.stderr.contains("WARN"), "{name}: {}", done.stderr);
        }
    }
}

#[test]
fn disabled_does_not_open_db() {
    let data = Data::new("disabled-garbage-db");
    fs::write(data.db(), "isto não é um banco SQLite").unwrap();
    let mut s = data.session();
    assert!(s.discover()["result"].is_object());
    assert_disabled(&mut s);
    let done = s.finish();
    assert_eq!(done.status.code(), Some(0), "{}", done.stderr);
}

#[test]
fn disabled_still_lists_tools() {
    let data = Data::new("disabled-tools");
    let mut s = data.session();
    let tools = s.request("tools/list", json!({}))["result"]["tools"].clone();
    let names: Vec<&str> = tools
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["search_dictations", "get_dictation"]);
    s.finish();
}

// ---------------------------------------------------------------- S5

#[test]
fn missing_db_gives_empty_and_is_not_created() {
    let data = Data::enabled("missing-db");
    let mut s = data.session();
    assert!(s.search(json!({ "query": "acao" })).is_empty());
    let response = s.call("get_dictation", json!({ "id": MISSING }));
    assert_eq!(
        tool_text(&response),
        format!("dictation not found: {MISSING}")
    );
    let done = s.finish();
    assert_eq!(done.status.code(), Some(0), "{}", done.stderr);
    assert!(!data.db().exists());
}

#[test]
fn session_leaves_db_bytes_unchanged() {
    let data = Data::enabled("read-only");
    let (a, _b, _c) = three(&data);
    let before = fs::read(data.db()).unwrap();
    let mut s = data.session();
    s.search(json!({ "query": "acao" }));
    s.ok("get_dictation", json!({ "id": a }));
    s.request("resources/list", json!({}));
    s.request(
        "resources/read",
        json!({ "uri": format!("fala://dictation/{a}") }),
    );
    let done = s.finish();
    assert_eq!(done.status.code(), Some(0), "{}", done.stderr);
    assert!(fs::read(data.db()).unwrap() == before, "fala.sqlite mudou");
}

#[test]
fn non_sqlite_db_exits_one_with_empty_stdout() {
    let data = Data::enabled("garbage-db");
    fs::write(data.db(), "isto não é um banco SQLite").unwrap();
    let done = data.session().finish();
    assert_eq!(done.status.code(), Some(1), "{}", done.stderr);
    assert!(done.stdout.is_empty(), "{:?}", done.stdout);
    assert!(
        done.stderr.contains("abrir o histórico") && done.stderr.contains("not a database"),
        "{}",
        done.stderr
    );
}

#[test]
fn default_log_level_never_prints_dictated_text() {
    let data = Data::enabled("log-level");
    let id = data
        .store()
        .add(
            &edited("zebra listrada falou", "A zebra listrada falou.", "Slack"),
            at(1),
        )
        .unwrap()
        .id;
    let mut s = data.session();
    s.search(json!({ "query": "zebra" }));
    s.ok("get_dictation", json!({ "id": id }));
    let done = s.finish();
    assert!(
        done.stderr.contains("mcp:"),
        "sem log no nível info: {}",
        done.stderr
    );
    for text in ["zebra listrada falou", "A zebra listrada falou."] {
        assert!(
            !done.stderr.contains(text),
            "texto ditado no log: {}",
            done.stderr
        );
    }
}

// ---------------------------------------------------------------- S6

#[test]
fn default_data_dir_is_the_history_dir() {
    // XDG_DATA_HOME only steers `dirs::data_dir()` on Linux. Elsewhere the default is the OS
    // known folder, which an env var cannot redirect, so running this would write a test
    // dictation into the real history of whoever runs the suite. Skip there, loudly.
    if std::env::consts::OS != "linux" {
        eprintln!("skipped: XDG_DATA_HOME does not redirect the default data dir on this OS");
        return;
    }
    let xdg = Data::new("xdg-default");
    let add = Command::new(env!("CARGO_BIN_EXE_fala-cli"))
        .args(["history", "add", "--raw", "ornitorrinco azul"])
        .env("XDG_DATA_HOME", &xdg.dir)
        .output()
        .unwrap();
    assert_eq!(
        add.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );
    let id = String::from_utf8(add.stdout).unwrap().trim().to_string();
    fs::write(
        xdg.dir.join("br.com.augusto.fala").join("mcp.toml"),
        "enabled = true\n",
    )
    .unwrap();
    let mut s = Session::start(&["mcp"], &[("XDG_DATA_HOME", &xdg.dir)], true);
    assert_eq!(ids(&s.search(json!({ "query": "ornitorrinco" }))), [id]);
    s.finish();
}

#[test]
fn unknown_argument_exits_two() {
    let data = Data::new("bogus");
    let mut s = Session::start(
        &["mcp", "--bogus", "--data-dir", data.dir.to_str().unwrap()],
        &[],
        true,
    );
    // O stdin fica aberto: o processo tem que sair sozinho.
    let started = Instant::now();
    let status = loop {
        if let Some(status) = s.child.try_wait().unwrap() {
            break status;
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "não saiu sem stdin"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(status.code(), Some(2));
    s.finish();
}
