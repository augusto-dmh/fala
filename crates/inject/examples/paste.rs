//! Verificação manual da colagem com restore (`TODO(windows)`).
//!
//! Uso: `cargo run -p fala-inject --example paste -- ["texto"]`, com o Bloco de Notas aberto.
//! Põe um texto marcador no clipboard, espera 3 s (foque o Bloco de Notas), cola o texto e confere
//! que o marcador voltou; depois repete com uma imagem 8×8 no clipboard. Imprime
//! `text_restored=<y|n> image_restored=<y|n>` e sai com 0 só se as duas colagens deram certo e o
//! clipboard voltou nas duas. Que o texto apareceu duas vezes no Bloco de Notas, a pessoa confere.

use std::process::ExitCode;

use fala_inject::{platform_injector, Injector, PasteConfig};

fn main() -> ExitCode {
    let text = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "teste um dois três".to_owned());
    let mut injector = match platform_injector(PasteConfig::default()) {
        Ok(injector) => injector,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };
    check(injector.as_mut(), &text)
}

#[cfg(windows)]
fn check(injector: &mut dyn Injector, text: &str) -> ExitCode {
    use std::borrow::Cow;
    use std::time::Duration;

    let mut clipboard = match arboard::Clipboard::new() {
        Ok(clipboard) => clipboard,
        Err(err) => {
            eprintln!("clipboard: {err}");
            return ExitCode::FAILURE;
        }
    };
    let paste = |injector: &mut dyn Injector| {
        println!("foque o Bloco de Notas: colando em 3 s");
        std::thread::sleep(Duration::from_secs(3));
        let result = injector.insert(text);
        if let Err(err) = &result {
            eprintln!("colagem: {err}");
        }
        std::thread::sleep(Duration::from_millis(500));
        result.is_ok()
    };

    let marker = format!("FALA-EXEMPLO-{}", std::process::id());
    let text_ok = clipboard.set_text(marker.as_str()).is_ok()
        && paste(injector)
        && clipboard.get_text().ok().as_deref() == Some(marker.as_str());

    // 8×8 RGBA opaca, com bytes que variam por pixel.
    let bytes: Vec<u8> = (0u8..64)
        .flat_map(|i| [i, i.wrapping_mul(3), 255 - i, 255])
        .collect();
    let image = arboard::ImageData {
        width: 8,
        height: 8,
        bytes: Cow::Owned(bytes.clone()),
    };
    let image_ok = clipboard.set_image(image).is_ok()
        && paste(injector)
        && clipboard
            .get_image()
            .is_ok_and(|back| back.width == 8 && back.height == 8 && *back.bytes == *bytes);

    let yn = |ok: bool| if ok { "y" } else { "n" };
    println!(
        "text_restored={} image_restored={}",
        yn(text_ok),
        yn(image_ok)
    );
    if text_ok && image_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Fora do Windows `platform_injector` já devolveu `Unsupported`; não há o que conferir.
#[cfg(not(windows))]
fn check(_injector: &mut dyn Injector, _text: &str) -> ExitCode {
    ExitCode::FAILURE
}
