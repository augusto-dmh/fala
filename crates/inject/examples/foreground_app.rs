//! Verificação manual da detecção do app em foco (`TODO(windows)`).
//!
//! Uso: `cargo run -p fala-inject --example foreground_app -- [nome-esperado]`. Espera 3 s para
//! dar tempo de focar o app alvo, imprime o resultado e, com um nome esperado, sai com 1 se o
//! `app_name` detectado for outro.

use std::process::ExitCode;
use std::time::Duration;

fn main() -> ExitCode {
    let expected = std::env::args().nth(1);
    std::thread::sleep(Duration::from_secs(3));
    let result = fala_inject::try_foreground_app();
    println!("{result:?}");
    let Some(expected) = expected else {
        return ExitCode::SUCCESS;
    };
    let detected = result.ok().and_then(|context| context.app_name);
    if detected.as_deref() == Some(expected.as_str()) {
        ExitCode::SUCCESS
    } else {
        eprintln!("esperava {expected:?}, detectou {detected:?}");
        ExitCode::FAILURE
    }
}
