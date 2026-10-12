//! O consentimento: `<data-dir>/mcp.toml` com `enabled = true` e, para as reuniões, também
//! `meetings = true`.
//!
//! Qualquer outra coisa (arquivo ausente, TOML inválido, chave ausente ou que não é o booleano
//! `true`) deixa desligado. Na dúvida, fecha.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

const CONFIG_FILE: &str = "mcp.toml";

/// O que o usuário ligou.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Consent {
    /// O MCP inteiro (e os ditados).
    pub(crate) enabled: bool,
    /// As reuniões, que só valem com `enabled`.
    pub(crate) meetings: bool,
}

/// Lê o consentimento da pasta de dados.
pub(crate) fn consent(data_dir: &Path) -> Consent {
    let text = match fs::read_to_string(data_dir.join(CONFIG_FILE)) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            log::info!("mcp: sem {CONFIG_FILE}; MCP desligado");
            return Consent::default();
        }
        Err(e) => {
            log::warn!("mcp: não deu para ler {CONFIG_FILE} ({e}); MCP desligado");
            return Consent::default();
        }
    };
    let table = match text.parse::<toml::Table>() {
        Ok(table) => table,
        Err(e) => {
            log::warn!("mcp: {CONFIG_FILE} não é TOML válido ({e}); MCP desligado");
            return Consent::default();
        }
    };
    let enabled = flag(&table, "enabled", "MCP desligado", true);
    // Sem `meetings`, reuniões ficam desligadas sem aviso: é o padrão.
    let meetings = enabled && flag(&table, "meetings", "reuniões desligadas", false);
    Consent { enabled, meetings }
}

/// Se `key` é o booleano `true`; `off` fecha o aviso e `warn_missing` avisa quando a chave falta.
fn flag(table: &toml::Table, key: &str, off: &str, warn_missing: bool) -> bool {
    match table.get(key) {
        Some(toml::Value::Boolean(true)) => true,
        Some(toml::Value::Boolean(false)) => {
            log::warn!("mcp: {CONFIG_FILE} tem {key} = false; {off}");
            false
        }
        None if warn_missing => {
            log::warn!("mcp: {CONFIG_FILE} sem a chave `{key}`; {off}");
            false
        }
        None => {
            log::info!("mcp: {CONFIG_FILE} sem a chave `{key}`; {off}");
            false
        }
        Some(other) => {
            log::warn!(
                "mcp: `{key}` em {CONFIG_FILE} precisa ser true ou false, não {}; {off}",
                other.type_str()
            );
            false
        }
    }
}
