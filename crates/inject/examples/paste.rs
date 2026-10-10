//! Verificação manual da colagem com prova de leitura e restore.
//!
//! Uso: `cargo run -p fala-inject --example paste -- [--rounds N] [--blank] [--image]
//! [--shift-insert] [--delay S] ["texto"]`.
//!
//! Cada rodada põe um marcador no clipboard (uma imagem 8×8 com `--image`), espera `--delay`
//! segundos (3 por padrão) para a pessoa focar o destino, cola o texto e imprime
//! `round=<n> result=<ok|not_read|erro> restored=<y|n>`.
//! - Destino com campo (Bloco de Notas, Windows Terminal, VS Code): espera `result=ok`; que o
//!   texto apareceu no destino, a pessoa confere.
//! - `--blank`: abre uma janela sem campo de texto e cola nela; espera `result=not_read`.
//!
//! Sai com 0 só se toda rodada teve o resultado esperado e o clipboard voltou ao marcador. No
//! fim, abra o Win+V: o texto colado não deve estar no histórico.

use std::process::ExitCode;

use fala_inject::{platform_injector, Injector, PasteChord, PasteConfig};

// Fora do Windows `check` não lê os argumentos.
#[cfg_attr(not(windows), allow(dead_code))]
struct Args {
    rounds: u64,
    blank: bool,
    image: bool,
    delay_secs: u64,
    chord: PasteChord,
    text: String,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        rounds: 1,
        blank: false,
        image: false,
        delay_secs: 3,
        chord: PasteChord::CtrlV,
        text: "teste um dois três".to_owned(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut number = |name: &str| -> Result<u64, String> {
            it.next()
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| format!("{name} pede um número"))
        };
        match arg.as_str() {
            "--rounds" => args.rounds = number("--rounds")?,
            "--delay" => args.delay_secs = number("--delay")?,
            "--blank" => args.blank = true,
            "--image" => args.image = true,
            "--shift-insert" => args.chord = PasteChord::ShiftInsert,
            _ => args.text = arg,
        }
    }
    Ok(args)
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };
    let config = PasteConfig {
        chord: args.chord,
        ..PasteConfig::default()
    };
    let mut injector = match platform_injector(config) {
        Ok(injector) => injector,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };
    check(injector.as_mut(), &args)
}

#[cfg(windows)]
fn check(injector: &mut dyn Injector, args: &Args) -> ExitCode {
    use std::borrow::Cow;
    use std::time::{Duration, Instant};

    use fala_inject::InjectError;

    let mut clipboard = match arboard::Clipboard::new() {
        Ok(clipboard) => clipboard,
        Err(err) => {
            eprintln!("clipboard: {err}");
            return ExitCode::FAILURE;
        }
    };
    if args.blank {
        blank_window::open();
    }

    let marker = format!("FALA-EXEMPLO-{}", std::process::id());
    // 8×8 RGBA opaca, com bytes que variam por pixel.
    let bytes: Vec<u8> = (0u8..64)
        .flat_map(|i| [i, i.wrapping_mul(3), 255 - i, 255])
        .collect();
    let mut all_ok = true;
    for round in 1..=args.rounds {
        let saved = if args.image {
            clipboard.set_image(arboard::ImageData {
                width: 8,
                height: 8,
                bytes: Cow::Owned(bytes.clone()),
            })
        } else {
            clipboard.set_text(marker.as_str())
        };
        if let Err(err) = saved {
            eprintln!("clipboard: {err}");
            return ExitCode::FAILURE;
        }
        println!(
            "foque o destino: colando em {} s ({:?})",
            args.delay_secs, args.chord
        );
        std::thread::sleep(Duration::from_secs(args.delay_secs));
        let started = Instant::now();
        let result = injector.insert(&args.text);
        let elapsed = started.elapsed();
        std::thread::sleep(Duration::from_millis(300));

        let restored = if args.image {
            clipboard
                .get_image()
                .is_ok_and(|back| back.width == 8 && back.height == 8 && *back.bytes == *bytes)
        } else {
            clipboard.get_text().ok().as_deref() == Some(marker.as_str())
        };
        let outcome = match &result {
            Ok(()) => "ok".to_owned(),
            Err(InjectError::PasteNotRead) => "not_read".to_owned(),
            Err(err) => format!("erro ({err})"),
        };
        let expected = if args.blank {
            matches!(result, Err(InjectError::PasteNotRead))
        } else {
            result.is_ok()
        };
        all_ok &= expected && restored;
        println!(
            "round={round} result={outcome} restored={} elapsed_ms={}",
            if restored { "y" } else { "n" },
            elapsed.as_millis()
        );
    }
    println!(
        "abra o Win+V: \"{}\" não deve estar no histórico",
        args.text
    );
    if all_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Uma janela visível sem campo de texto, para a colagem que ninguém lê.
#[cfg(windows)]
mod blank_window {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DispatchMessageW, GetMessageW, SetForegroundWindow, MSG,
        WS_OVERLAPPEDWINDOW, WS_VISIBLE,
    };

    pub fn open() {
        std::thread::spawn(|| {
            let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
            let title: Vec<u16> = "Fala: janela sem campo\0".encode_utf16().collect();
            // SAFETY: `class` e `title` terminam em 0 e vivem até o fim da thread; a janela e o
            // laço de mensagens são desta thread.
            unsafe {
                let hwnd = CreateWindowExW(
                    0,
                    class.as_ptr(),
                    title.as_ptr(),
                    WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                    200,
                    200,
                    480,
                    240,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null(),
                );
                if hwnd.is_null() {
                    eprintln!("a janela sem campo não abriu");
                    return;
                }
                SetForegroundWindow(hwnd);
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    DispatchMessageW(&msg);
                }
            }
        });
    }
}

/// Fora do Windows `platform_injector` já devolveu `Unsupported`; não há o que conferir.
#[cfg(not(windows))]
fn check(_injector: &mut dyn Injector, _args: &Args) -> ExitCode {
    ExitCode::FAILURE
}
