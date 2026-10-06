//! Servidor MCP local do Fala: stdio, só leitura, sobre o histórico do `fala-storage`.
//!
//! Expõe `search_dictations`, `get_dictation` e as resources `fala://dictation/<id>`. Fica
//! desligado até `<data-dir>/mcp.toml` ter `enabled = true`; desligado, não abre o banco. Um
//! ditado `sensitive` é inexistente para o MCP. Nunca escreve: só abre `fala.sqlite` se ele já
//! existe e só chama `Store::search` e `Store::get`. O stdout é do protocolo; o log vai para o
//! stderr. Desenho na ADR-0010 (proposed).

mod config;
mod server;

use std::path::Path;

use fala_storage::{StorageError, Store};
use rmcp::service::ServerInitializeError;
use rmcp::ServiceExt;

use crate::server::FalaMcp;

/// O arquivo do histórico, na pasta de dados.
const DB_FILE: &str = "fala.sqlite";

/// Erros de `fala-mcp`.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    /// `fala.sqlite` existe, mas não abriu.
    #[error("abrir o histórico: {0}")]
    Open(#[source] StorageError),
    /// A sessão não abriu (primeira mensagem inválida) ou o transporte falhou.
    #[error("sessão MCP: {0}")]
    Protocol(String),
    /// O runtime do tokio não subiu.
    #[error("runtime: {0}")]
    Runtime(#[from] std::io::Error),
}

/// Serve o MCP pelo stdin/stdout até o cliente fechar o stdin.
///
/// `data_dir` é a pasta do `fala.sqlite` e do `mcp.toml`, a mesma do `fala-cli history`.
pub fn run_stdio(data_dir: &Path) -> Result<(), McpError> {
    let enabled = config::enabled(data_dir);
    let store = if enabled { open_store(data_dir)? } else { None };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?;
    let result = runtime.block_on(serve(FalaMcp::new(enabled, store)));
    // A leitura do stdin roda numa thread bloqueante do tokio; se a sessão acabou por erro com o
    // stdin ainda aberto, esperar por ela prenderia o processo.
    runtime.shutdown_background();
    result
}

/// Abre o banco só se ele já existe: o MCP nunca cria `fala.sqlite`.
fn open_store(data_dir: &Path) -> Result<Option<Store>, McpError> {
    let db = data_dir.join(DB_FILE);
    if !db.is_file() {
        log::info!("mcp: sem {DB_FILE}; o histórico está vazio");
        return Ok(None);
    }
    // O `Store` só escreve em `notas/` ao gravar ou desfazer, e o MCP não chama nenhum dos dois.
    Store::open(&db, &data_dir.join("notas"))
        .map(Some)
        .map_err(McpError::Open)
}

async fn serve(server: FalaMcp) -> Result<(), McpError> {
    match server.serve(rmcp::transport::stdio()).await {
        Ok(running) => {
            let reason = running
                .waiting()
                .await
                .map_err(|e| McpError::Protocol(e.to_string()))?;
            log::info!("mcp: sessão encerrada ({reason:?})");
            Ok(())
        }
        // O cliente fechou o stdin antes de mandar qualquer coisa: fim normal.
        Err(ServerInitializeError::ConnectionClosed(_)) => Ok(()),
        Err(e) => Err(McpError::Protocol(e.to_string())),
    }
}
