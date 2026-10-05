//! O que um sidecar `fala-mcp` seria: só o servidor, sem o resto da CLI (onnx, whisper.cpp, cpal).
//!
//! Existe para medir o binário: `cargo build --release -p fala-mcp --example stdio`.
//! Uso: `stdio <data-dir>`; sai com 2 sem a pasta e com 1 se o servidor falhar.

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(dir) = std::env::args_os().nth(1) else {
        return ExitCode::from(2);
    };
    match fala_mcp::run_stdio(Path::new(&dir)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
