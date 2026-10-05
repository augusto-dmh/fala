//! O consentimento: `<data-dir>/mcp.toml` com `enabled = true`.
//!
//! Qualquer outra coisa (arquivo ausente, TOML inválido, `enabled` ausente ou que não é o
//! booleano `true`) deixa o MCP desligado. Na dúvida, fecha.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

const CONFIG_FILE: &str = "mcp.toml";

/// Se o usuário ligou o MCP.
pub(crate) fn enabled(data_dir: &Path) -> bool {
    let text = match fs::read_to_string(data_dir.join(CONFIG_FILE)) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            log::info!("mcp: sem {CONFIG_FILE}; MCP desligado");
            return false;
        }
        Err(e) => {
            log::warn!("mcp: não deu para ler {CONFIG_FILE} ({e}); MCP desligado");
            return false;
        }
    };
    let table = match text.parse::<toml::Table>() {
        Ok(table) => table,
        Err(e) => {
            log::warn!("mcp: {CONFIG_FILE} não é TOML válido ({e}); MCP desligado");
            return false;
        }
    };
    match table.get("enabled") {
        Some(toml::Value::Boolean(true)) => true,
        Some(toml::Value::Boolean(false)) => {
            log::warn!("mcp: {CONFIG_FILE} tem enabled = false; MCP desligado");
            false
        }
        None => {
            log::warn!("mcp: {CONFIG_FILE} sem a chave `enabled`; MCP desligado");
            false
        }
        Some(other) => {
            log::warn!(
                "mcp: `enabled` em {CONFIG_FILE} precisa ser true ou false, não {}; MCP desligado",
                other.type_str()
            );
            false
        }
    }
}
