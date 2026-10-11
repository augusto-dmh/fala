//! Lista as saídas de áudio e o pico de cada uma por 10 s, uma linha por segundo.
//!
//! `cargo run -p fala-audio --example outputs [segundos]`. Só leitura: não abre loopback nem
//! grava. Toque algo num fone que não é a saída padrão para ver o pico aparecer nele.

use std::time::Duration;

use fala_audio::{OutputMeter, SystemAudio};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let seconds: u64 = std::env::args()
        .nth(1)
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(10);
    let outputs = SystemAudio::outputs()?;
    println!("saídas:");
    for output in &outputs {
        let mark = if output.is_default { " (padrão)" } else { "" };
        println!("  {}{mark}", output.name);
    }
    let meter = match OutputMeter::start() {
        Ok(meter) => meter,
        Err(e) => {
            println!("sem medidor de pico: {e}");
            return Ok(());
        }
    };
    meter.take();
    for second in 1..=seconds {
        std::thread::sleep(Duration::from_secs(1));
        let peaks: Vec<String> = meter
            .take()
            .iter()
            .map(|p| format!("{} {:.4}", p.name, p.peak))
            .collect();
        println!("{second:>3}s  {}", peaks.join("  |  "));
    }
    Ok(())
}
