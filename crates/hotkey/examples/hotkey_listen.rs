//! Verificação manual do atalho global (`TODO(windows)`).
//!
//! Uso: `cargo run -p fala-hotkey --example hotkey_listen -- <acelerador> <toques>`, por exemplo
//! `-- f9 60`. Registra o acelerador, conta `Pressed` e `Released` até chegar a `<toques>`
//! solturas ou passar 15 min sem atingir, imprime `pressed=<n> released=<n>` e sai com 0 só se
//! as duas contagens forem iguais a `<toques>`.

use std::process::ExitCode;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use fala_hotkey::{platform_hotkey, KeyState};

const LIMIT: Duration = Duration::from_secs(15 * 60);

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let accelerator = args.next().unwrap_or_else(|| "f9".to_owned());
    let expected: u32 = match args.next().map(|n| n.parse()) {
        None => 60,
        Some(Ok(n)) => n,
        Some(Err(_)) => {
            eprintln!("<toques> precisa ser um número");
            return ExitCode::FAILURE;
        }
    };

    let (tx, rx) = mpsc::channel();
    let mut hotkey = match platform_hotkey(tx) {
        Ok(hotkey) => hotkey,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(err) = hotkey.register("listen", &accelerator) {
        eprintln!("{err}");
        return ExitCode::FAILURE;
    }
    println!("toque {accelerator} {expected} vezes (limite de 15 min)");

    let (mut pressed, mut released) = (0u32, 0u32);
    let start = Instant::now();
    while released < expected && start.elapsed() < LIMIT {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(event) => match event.state {
                KeyState::Pressed => pressed += 1,
                KeyState::Released => released += 1,
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    drop(hotkey);

    println!("pressed={pressed} released={released}");
    if pressed == expected && released == expected {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
