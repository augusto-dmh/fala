//! Lista os processos que capturam o microfone agora, com os ancestrais.
//!
//! `cargo run -p fala-audio --example mic_holders [segundos]`: sem argumento, uma leitura; com
//! um número, uma leitura por segundo durante esse tempo, impressa só quando muda. Só leitura:
//! não abre nenhum stream.

use std::time::Duration;

use fala_audio::{platform_activity, MicHolder};

fn line(holder: &MicHolder) -> String {
    let mut text = format!("{:>6}  {}", holder.pid, holder.exe);
    if !holder.ancestors.is_empty() {
        text.push_str("  <- ");
        text.push_str(&holder.ancestors.join(" <- "));
    }
    text
}

fn main() -> std::process::ExitCode {
    let seconds: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let mut activity = platform_activity();
    let mut last: Option<Vec<MicHolder>> = None;
    for tick in 0..=seconds {
        if tick > 0 {
            std::thread::sleep(Duration::from_secs(1));
        }
        let holders = match activity.mic_holders() {
            Ok(holders) => holders,
            Err(e) => {
                eprintln!("mic_holders: {e}");
                return std::process::ExitCode::FAILURE;
            }
        };
        if last.as_ref() == Some(&holders) {
            continue;
        }
        println!(
            "[{tick:>3} s] {} processo(s) capturando o mic",
            holders.len()
        );
        for holder in &holders {
            println!("{}", line(holder));
        }
        last = Some(holders);
    }
    std::process::ExitCode::SUCCESS
}
