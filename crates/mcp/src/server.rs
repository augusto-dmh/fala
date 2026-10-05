//! O `ServerHandler` escrito à mão: duas tools e as resources `fala://dictation/<id>`.

use std::sync::{Mutex, PoisonError};

use chrono::SecondsFormat;
use fala_core::Editor;
use fala_storage::{DictationRecord, StorageError, Store};
use rmcp::model::{
    CacheScope, CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock,
    Implementation, JsonObject, ListResourceTemplatesResult, ListResourcesResult, ListToolsResult,
    PaginatedRequestParams, ProtocolVersion, ReadResourceRequestParams, ReadResourceResponse,
    ReadResourceResult, Resource, ResourceContents, ResourceTemplate, ServerCapabilities,
    ServerConfig, Tool, ToolAnnotations,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};
use serde_json::{json, Value};

const URI_PREFIX: &str = "fala://dictation/";
const URI_TEMPLATE: &str = "fala://dictation/{id}";
const MIME: &str = "application/json";
const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 50;
const RESOURCE_LIST_LIMIT: usize = 50;
const DISABLED: &str =
    "Fala MCP is disabled: set enabled = true in mcp.toml in the Fala data folder";
const STORAGE_ERROR: &str = "storage error";

/// Por que uma leitura não devolveu o item.
enum ReadError {
    /// Não existe, ou é sensível: para o MCP, as duas coisas são a mesma.
    NotFound,
    /// Falha do banco; o detalhe vai só para o log.
    Storage(String),
}

impl From<StorageError> for ReadError {
    fn from(e: StorageError) -> Self {
        match e {
            StorageError::NotFound(_) => ReadError::NotFound,
            other => ReadError::Storage(other.to_string()),
        }
    }
}

impl<T> From<PoisonError<T>> for ReadError {
    fn from(_: PoisonError<T>) -> Self {
        ReadError::Storage("conexão envenenada por um pânico anterior".to_string())
    }
}

/// O servidor: o consentimento lido na abertura e, se ligado e o banco existe, o `Store`.
pub(crate) struct FalaMcp {
    enabled: bool,
    store: Option<Mutex<Store>>,
}

impl FalaMcp {
    pub(crate) fn new(enabled: bool, store: Option<Store>) -> Self {
        FalaMcp {
            enabled,
            store: store.map(Mutex::new),
        }
    }

    /// Busca como o `Store::search`, sem os sensíveis, completando o `limit` com os seguintes.
    fn search(&self, query: &str, limit: usize) -> Result<Vec<DictationRecord>, ReadError> {
        let Some(store) = &self.store else {
            return Ok(Vec::new());
        };
        let store = store.lock()?;
        let mut fetch = limit;
        loop {
            let rows = store.search(query, fetch)?;
            let exhausted = rows.len() < fetch;
            let visible: Vec<_> = rows
                .into_iter()
                .filter(|r| !r.sensitive)
                .take(limit)
                .collect();
            if visible.len() == limit || exhausted {
                return Ok(visible);
            }
            fetch = fetch.saturating_mul(2);
        }
    }

    /// Lê um item; um sensível responde como inexistente.
    fn get(&self, id: &str) -> Result<DictationRecord, ReadError> {
        let Some(store) = &self.store else {
            return Err(ReadError::NotFound);
        };
        let record = store.lock()?.get(id)?;
        if record.sensitive {
            return Err(ReadError::NotFound);
        }
        Ok(record)
    }

    fn call_search(&self, args: &JsonObject) -> Result<CallToolResult, ErrorData> {
        if !self.enabled {
            return Ok(text_error(DISABLED));
        }
        reject_unknown(args, &["query", "limit"])?;
        let query = match args.get("query") {
            None | Some(Value::Null) => "",
            Some(Value::String(q)) => q.as_str(),
            Some(_) => return Err(invalid("`query` must be a string")),
        };
        let limit = match args.get("limit") {
            None | Some(Value::Null) => DEFAULT_LIMIT,
            Some(v) => v
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .filter(|n| (1..=MAX_LIMIT).contains(n))
                .ok_or_else(|| invalid("`limit` must be an integer from 1 to 50"))?,
        };
        match self.search(query, limit) {
            Ok(records) => {
                log::info!("mcp: search_dictations -> {} itens", records.len());
                let items: Vec<Value> = records.iter().map(summary).collect();
                Ok(CallToolResult::structured(json!({ "items": items })))
            }
            Err(e) => Ok(read_failure(e, "")),
        }
    }

    fn call_get(&self, args: &JsonObject) -> Result<CallToolResult, ErrorData> {
        if !self.enabled {
            return Ok(text_error(DISABLED));
        }
        reject_unknown(args, &["id"])?;
        let Some(Value::String(id)) = args.get("id") else {
            return Err(invalid("`id` must be a string"));
        };
        match self.get(id) {
            Ok(record) => {
                log::info!("mcp: get_dictation -> 1 item");
                Ok(CallToolResult::structured(detail(&record)))
            }
            Err(e) => Ok(read_failure(e, id)),
        }
    }
}

impl ServerHandler for FalaMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::new("fala", env!("CARGO_PKG_VERSION")))
        .with_instructions(
            "Read-only access to the user's Fala dictation history (mostly Brazilian \
             Portuguese). Find dictations with search_dictations and read one with get_dictation.",
        )
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let mut result = ListToolsResult::with_all_items(tools());
        if cacheable(&context) {
            result = result.with_ttl_ms(0).with_cache_scope(CacheScope::Private);
        }
        Ok(result)
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let args = request.arguments.unwrap_or_default();
        let result = match request.name.as_ref() {
            "search_dictations" => self.call_search(&args)?,
            "get_dictation" => self.call_get(&args)?,
            other => return Err(invalid(&format!("unknown tool: {other}"))),
        };
        Ok(result.into())
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let records = if self.enabled {
            self.search("", RESOURCE_LIST_LIMIT)
                .map_err(|e| storage_protocol_error(e, "resources/list"))?
        } else {
            Vec::new()
        };
        let resources = records
            .iter()
            .map(|r| {
                Resource::new(uri(r), r.id.clone())
                    .with_title(format!("Dictation {}", timestamp(r)))
                    .with_mime_type(MIME)
            })
            .collect();
        let mut result = ListResourcesResult::with_all_items(resources);
        if cacheable(&context) {
            result = result.with_ttl_ms(0).with_cache_scope(CacheScope::Private);
        }
        Ok(result)
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        let template = ResourceTemplate::new(URI_TEMPLATE, "dictation")
            .with_title("Fala dictation")
            .with_description(
                "One dictation from the Fala history, as JSON (same as get_dictation).",
            )
            .with_mime_type(MIME);
        let mut result = ListResourceTemplatesResult::with_all_items(vec![template]);
        if cacheable(&context) {
            result = result.with_ttl_ms(0).with_cache_scope(CacheScope::Private);
        }
        Ok(result)
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        if !self.enabled {
            return Err(ErrorData::resource_not_found(DISABLED, None));
        }
        let not_found =
            || ErrorData::resource_not_found(format!("resource not found: {}", request.uri), None);
        let Some(id) = request.uri.strip_prefix(URI_PREFIX) else {
            return Err(not_found());
        };
        let record = match self.get(id) {
            Ok(record) => record,
            Err(ReadError::NotFound) => return Err(not_found()),
            Err(e) => return Err(storage_protocol_error(e, "resources/read")),
        };
        log::info!("mcp: resources/read -> 1 item");
        let contents = ResourceContents::text(detail(&record).to_string(), request.uri.clone())
            .with_mime_type(MIME);
        let mut result = ReadResourceResult::new(vec![contents]);
        if cacheable(&context) {
            result = result.with_ttl_ms(0).with_cache_scope(CacheScope::Private);
        }
        Ok(result.into())
    }
}

/// As duas tools, em ordem fixa.
fn tools() -> Vec<Tool> {
    let annotations = ToolAnnotations::new()
        .read_only(true)
        .destructive(false)
        .idempotent(true)
        .open_world(false);
    let item = json!({
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "created_at": { "type": "string", "description": "RFC 3339 with the local offset" },
            "app": { "type": ["string", "null"] },
            "language": { "type": "string", "enum": ["pt-BR", "en"] },
            "edited_by": { "type": "string", "enum": ["none", "rules", "llm"] },
            "showing": { "type": "string", "enum": ["final", "raw"] },
        },
    });
    let mut summary = item.clone();
    let mut detail = item;
    if let Some(props) = summary["properties"].as_object_mut() {
        props.insert(
            "text".to_string(),
            json!({ "type": "string", "description": "the text the item shows now (final, or raw after an undo)" }),
        );
    }
    if let Some(props) = detail["properties"].as_object_mut() {
        props.insert("final".to_string(), json!({ "type": "string" }));
        props.insert("raw".to_string(), json!({ "type": "string" }));
        props.insert("uri".to_string(), json!({ "type": "string" }));
    }
    vec![
        Tool::new(
            "search_dictations",
            "Search the user's dictation history. Matching ignores accents, each word matches as a \
             prefix and all words must match. Without a query, lists the most recent dictations. \
             Newest first.",
            object(json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "words to look for; empty lists the most recent" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": MAX_LIMIT, "default": DEFAULT_LIMIT },
                },
                "additionalProperties": false,
            })),
        )
        .with_raw_output_schema(
            object(json!({
                "type": "object",
                "properties": { "items": { "type": "array", "items": summary } },
                "required": ["items"],
            }))
            .into(),
        )
        .with_annotations(annotations.clone()),
        Tool::new(
            "get_dictation",
            "Read one dictation by id: the final text, the raw ASR text and who edited it.",
            object(json!({
                "type": "object",
                "properties": { "id": { "type": "string", "description": "the id from search_dictations" } },
                "required": ["id"],
                "additionalProperties": false,
            })),
        )
        .with_raw_output_schema(object(detail).into())
        .with_annotations(annotations),
    ]
}

/// Um item da busca: a lista fechada da door 3.
fn summary(r: &DictationRecord) -> Value {
    json!({
        "id": r.id,
        "created_at": timestamp(r),
        "app": r.dictation.app.app_name,
        "language": r.dictation.raw.language.tag(),
        "edited_by": editor(r.dictation.editor),
        "showing": r.showing.as_str(),
        "text": r.shown_text(),
    })
}

/// Um item do `get_dictation` e da resource: a lista fechada da door 3.
fn detail(r: &DictationRecord) -> Value {
    json!({
        "id": r.id,
        "created_at": timestamp(r),
        "app": r.dictation.app.app_name,
        "language": r.dictation.raw.language.tag(),
        "edited_by": editor(r.dictation.editor),
        "showing": r.showing.as_str(),
        "final": r.dictation.final_text,
        "raw": r.dictation.raw.text,
        "uri": uri(r),
    })
}

fn uri(r: &DictationRecord) -> String {
    format!("{URI_PREFIX}{}", r.id)
}

/// O `created_at` no mesmo formato do banco e do espelho.
fn timestamp(r: &DictationRecord) -> String {
    r.created_at.to_rfc3339_opts(SecondsFormat::AutoSi, false)
}

/// Os literais de `edited_by` do banco (door 5 do `storage-history`).
fn editor(editor: Editor) -> &'static str {
    match editor {
        Editor::None => "none",
        Editor::Rules => "rules",
        Editor::Llm => "llm",
    }
}

/// Os campos de cache só existem a partir da 2026-07-28; antes, ficam de fora.
fn cacheable(context: &RequestContext<RoleServer>) -> bool {
    context
        .protocol_version()
        .is_some_and(|v| v.as_str() >= ProtocolVersion::V_2026_07_28.as_str())
}

fn object(value: Value) -> JsonObject {
    match value {
        Value::Object(map) => map,
        _ => JsonObject::new(),
    }
}

fn invalid(message: &str) -> ErrorData {
    ErrorData::invalid_params(message.to_string(), None)
}

fn reject_unknown(args: &JsonObject, allowed: &[&str]) -> Result<(), ErrorData> {
    match args.keys().find(|k| !allowed.contains(&k.as_str())) {
        Some(key) => Err(invalid(&format!("unknown argument: {key}"))),
        None => Ok(()),
    }
}

fn text_error(message: &str) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.to_string())])
}

/// O erro que o modelo vê; a causa de uma falha de banco fica só no stderr.
fn read_failure(e: ReadError, id: &str) -> CallToolResult {
    match e {
        ReadError::NotFound => text_error(&format!("dictation not found: {id}")),
        ReadError::Storage(cause) => {
            log::warn!("mcp: falha ao ler o histórico: {cause}");
            text_error(STORAGE_ERROR)
        }
    }
}

fn storage_protocol_error(e: ReadError, method: &str) -> ErrorData {
    if let ReadError::Storage(cause) = e {
        log::warn!("mcp: {method}: falha ao ler o histórico: {cause}");
    }
    ErrorData::internal_error(STORAGE_ERROR, None)
}
