//! `fala-cli mcp`: o servidor MCP do `fala-mcp` pelo stdin/stdout.
//!
//! O stdout é do protocolo: nada aqui imprime; o log do `env_logger` vai para o stderr.

use std::path::PathBuf;

use anyhow::anyhow;
use clap::Args;

use crate::history::{Failure, APP_DIR};

#[derive(Args)]
pub struct McpArgs {
    /// Pasta do `fala.sqlite` e do `mcp.toml` (padrão: `<pasta de dados do SO>/br.com.augusto.fala`).
    #[arg(long)]
    data_dir: Option<PathBuf>,
}

pub fn run(args: McpArgs) -> Result<(), Failure> {
    let data_dir = match args.data_dir {
        Some(dir) => dir,
        None => dirs::data_dir()
            .ok_or_else(|| Failure {
                code: 2,
                error: anyhow!("sem pasta de dados no SO; passe --data-dir"),
            })?
            .join(APP_DIR),
    };
    fala_mcp::run_stdio(&data_dir).map_err(|e| Failure {
        code: 1,
        error: e.into(),
    })
}
