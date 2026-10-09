//! Inserção de texto no app em foco por clipboard + acorde de colar, com o clipboard restaurado.
//!
//! A sequência é a de `apps/desktop/src/clipboard.rs::paste_via_clipboard`, que o spike 03 mediu
//! no Windows: salva o texto do clipboard (a imagem, só quando não há texto), escreve o texto,
//! espera, manda o acorde, espera e devolve o que estava lá, ou limpa se não havia nada. A
//! sequência é neutra de plataforma; o adaptador só fornece o clipboard e o teclado.

use std::time::Duration;

use crate::InjectError;

#[cfg(windows)]
mod windows;

/// O acorde que cola no app em foco.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PasteChord {
    CtrlV,
    /// Terminais que usam Ctrl+Shift+V para colar.
    CtrlShiftV,
}

/// Acorde e tempos da colagem. O padrão é o do desktop: Ctrl+V, 60 ms antes, 60 ms depois e o
/// modificador seguro por 100 ms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasteConfig {
    pub chord: PasteChord,
    /// Espera entre escrever o texto no clipboard e mandar o acorde.
    pub delay_before: Duration,
    /// Espera entre o acorde e a restauração do clipboard.
    pub delay_after: Duration,
    /// Quanto tempo o modificador fica pressionado depois do V.
    pub modifier_hold: Duration,
}

impl Default for PasteConfig {
    fn default() -> Self {
        Self {
            chord: PasteChord::CtrlV,
            delay_before: Duration::from_millis(60),
            delay_after: Duration::from_millis(60),
            modifier_hold: Duration::from_millis(100),
        }
    }
}

/// Insere texto no app em foco.
pub trait Injector: Send {
    /// Cola `text` no app em foco e devolve ao clipboard o que estava nele, mesmo se o acorde
    /// falhar.
    fn insert(&mut self, text: &str) -> Result<(), InjectError>;
}

/// O `Injector` desta plataforma.
pub fn platform_injector(config: PasteConfig) -> Result<Box<dyn Injector>, InjectError> {
    platform::injector(config)
}

#[cfg(windows)]
mod platform {
    use super::{Injector, PasteConfig};
    use crate::InjectError;

    pub(super) fn injector(config: PasteConfig) -> Result<Box<dyn Injector>, InjectError> {
        Ok(Box::new(super::windows::WindowsInjector::new(config)?))
    }
}

/// Fora do Windows não há colagem. No GNOME Wayland, o Ctrl+V sintético não chega a apps
/// Wayland e o clipboard só é escrito via XWayland (relatório 13 §3); o adaptador por portal
/// RemoteDesktop + Clipboard é a fase 3 (ADR-0007).
#[cfg(not(windows))]
mod platform {
    use super::{Injector, PasteConfig};
    use crate::InjectError;

    const UNSUPPORTED: &str = "sem colagem fora do Windows \
                               (GNOME Wayland: portal RemoteDesktop + Clipboard, fase 3)";

    pub(super) fn injector(_config: PasteConfig) -> Result<Box<dyn Injector>, InjectError> {
        Err(InjectError::Unsupported(UNSUPPORTED))
    }
}

/// O clipboard do sistema, visto pela sequência de colagem.
// Fora do Windows os dois traits e a sequência só são usados pelos testes, até a fase 3.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) trait Clipboard {
    type Image;
    fn read_text(&mut self) -> Result<String, InjectError>;
    fn read_image(&mut self) -> Result<Self::Image, InjectError>;
    fn write_text(&mut self, text: &str) -> Result<(), InjectError>;
    fn write_image(&mut self, image: &Self::Image) -> Result<(), InjectError>;
    fn clear(&mut self) -> Result<(), InjectError>;
}

/// O teclado do sistema, visto pela sequência de colagem.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) trait Keyboard {
    fn send_chord(&mut self, chord: PasteChord, modifier_hold: Duration)
        -> Result<(), InjectError>;
}

enum Saved<I> {
    Text(String),
    Image(I),
    Nothing,
}

/// Cola `text` e restaura o clipboard. O texto colado e o conteúdo salvo nunca vão ao log.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn paste_with_restore<C: Clipboard, K: Keyboard>(
    clipboard: &mut C,
    keyboard: &mut K,
    sleep: &mut impl FnMut(Duration),
    config: &PasteConfig,
    text: &str,
) -> Result<(), InjectError> {
    // A imagem só é lida sem texto: ler imagem decodifica o bitmap inteiro.
    let saved = match clipboard.read_text() {
        Ok(saved) if !saved.is_empty() => Saved::Text(saved),
        _ => match clipboard.read_image() {
            Ok(image) => Saved::Image(image),
            Err(_) => Saved::Nothing,
        },
    };

    clipboard.write_text(text)?;
    sleep(config.delay_before);
    let chord = keyboard.send_chord(config.chord, config.modifier_hold);
    sleep(config.delay_after);

    let restored = match &saved {
        Saved::Text(saved) => clipboard.write_text(saved),
        Saved::Image(image) => clipboard.write_image(image),
        Saved::Nothing => clipboard.clear(),
    };
    if let Err(err) = restored {
        log::warn!("o clipboard não foi restaurado depois da colagem: {err}");
    }
    chord
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Mutex;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Image {
        width: usize,
        height: usize,
        bytes: Vec<u8>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Op {
        ReadText,
        ReadImage,
        WriteText(String),
        WriteImage(Image),
        Clear,
        Sleep(Duration),
        Chord(PasteChord, Duration),
    }

    type Log = Rc<RefCell<Vec<Op>>>;

    struct FakeClipboard {
        log: Log,
        /// `None`: a leitura de texto falha (não há texto).
        text: Option<String>,
        /// `None`: a leitura de imagem falha (não há imagem).
        image: Option<Image>,
        fail_write_text: bool,
        fail_restore: bool,
    }

    impl FakeClipboard {
        fn new(log: &Log, text: Option<&str>, image: Option<Image>) -> Self {
            Self {
                log: log.clone(),
                text: text.map(str::to_owned),
                image,
                fail_write_text: false,
                fail_restore: false,
            }
        }
    }

    impl Clipboard for FakeClipboard {
        type Image = Image;
        fn read_text(&mut self) -> Result<String, InjectError> {
            self.log.borrow_mut().push(Op::ReadText);
            self.text
                .clone()
                .ok_or_else(|| InjectError::Clipboard("sem texto".to_owned()))
        }
        fn read_image(&mut self) -> Result<Image, InjectError> {
            self.log.borrow_mut().push(Op::ReadImage);
            self.image
                .clone()
                .ok_or_else(|| InjectError::Clipboard("sem imagem".to_owned()))
        }
        fn write_text(&mut self, text: &str) -> Result<(), InjectError> {
            let is_restore = self
                .log
                .borrow()
                .iter()
                .any(|op| matches!(op, Op::Chord(..)));
            if self.fail_write_text || (is_restore && self.fail_restore) {
                return Err(InjectError::Clipboard("escrita recusada".to_owned()));
            }
            self.log.borrow_mut().push(Op::WriteText(text.to_owned()));
            Ok(())
        }
        fn write_image(&mut self, image: &Image) -> Result<(), InjectError> {
            if self.fail_restore {
                return Err(InjectError::Clipboard("escrita recusada".to_owned()));
            }
            self.log.borrow_mut().push(Op::WriteImage(image.clone()));
            Ok(())
        }
        fn clear(&mut self) -> Result<(), InjectError> {
            if self.fail_restore {
                return Err(InjectError::Clipboard("limpeza recusada".to_owned()));
            }
            self.log.borrow_mut().push(Op::Clear);
            Ok(())
        }
    }

    struct FakeKeyboard {
        log: Log,
        fail: bool,
    }

    impl Keyboard for FakeKeyboard {
        fn send_chord(&mut self, chord: PasteChord, hold: Duration) -> Result<(), InjectError> {
            self.log.borrow_mut().push(Op::Chord(chord, hold));
            if self.fail {
                Err(InjectError::Keystroke("SendInput recusado".to_owned()))
            } else {
                Ok(())
            }
        }
    }

    fn run(
        clipboard: &mut FakeClipboard,
        chord_fails: bool,
        config: &PasteConfig,
        text: &str,
    ) -> Result<(), InjectError> {
        let log = clipboard.log.clone();
        let mut keyboard = FakeKeyboard {
            log: log.clone(),
            fail: chord_fails,
        };
        let mut sleep = |d| log.borrow_mut().push(Op::Sleep(d));
        paste_with_restore(clipboard, &mut keyboard, &mut sleep, config, text)
    }

    fn image() -> Image {
        Image {
            width: 2,
            height: 1,
            bytes: vec![1, 2, 3, 255, 4, 5, 6, 255],
        }
    }

    fn pasted_then(config: &PasteConfig, last: Op) -> Vec<Op> {
        vec![
            Op::WriteText("ditado".to_owned()),
            Op::Sleep(config.delay_before),
            Op::Chord(config.chord, config.modifier_hold),
            Op::Sleep(config.delay_after),
            last,
        ]
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_platform_injector_is_unsupported() {
        match platform_injector(PasteConfig::default()) {
            Err(InjectError::Unsupported(reason)) => assert!(!reason.trim().is_empty()),
            Err(other) => panic!("esperava Unsupported, veio {other:?}"),
            Ok(_) => panic!("esperava Unsupported, veio um Injector"),
        }
    }

    #[test]
    fn default_config_matches_desktop() {
        assert_eq!(
            PasteConfig::default(),
            PasteConfig {
                chord: PasteChord::CtrlV,
                delay_before: Duration::from_millis(60),
                delay_after: Duration::from_millis(60),
                modifier_hold: Duration::from_millis(100),
            }
        );
    }

    #[test]
    fn text_is_restored_after_paste_in_order() {
        for chord in [PasteChord::CtrlV, PasteChord::CtrlShiftV] {
            let config = PasteConfig {
                chord,
                ..PasteConfig::default()
            };
            let log = Log::default();
            let mut clipboard = FakeClipboard::new(&log, Some("antes"), Some(image()));
            assert_eq!(run(&mut clipboard, false, &config, "ditado"), Ok(()));
            let mut expected = vec![Op::ReadText];
            expected.extend(pasted_then(&config, Op::WriteText("antes".to_owned())));
            assert_eq!(*log.borrow(), expected, "{chord:?}");
        }
    }

    #[test]
    fn image_is_restored_when_there_is_no_text() {
        let config = PasteConfig::default();
        for text in [None, Some("")] {
            let log = Log::default();
            let mut clipboard = FakeClipboard::new(&log, text, Some(image()));
            assert_eq!(run(&mut clipboard, false, &config, "ditado"), Ok(()));
            let mut expected = vec![Op::ReadText, Op::ReadImage];
            expected.extend(pasted_then(&config, Op::WriteImage(image())));
            assert_eq!(*log.borrow(), expected, "texto salvo {text:?}");
        }
    }

    #[test]
    fn empty_clipboard_is_cleared() {
        let config = PasteConfig::default();
        // Clipboard vazio (texto vazio, sem imagem) e as duas leituras falhando.
        for text in [Some(""), None] {
            let log = Log::default();
            let mut clipboard = FakeClipboard::new(&log, text, None);
            assert_eq!(run(&mut clipboard, false, &config, "ditado"), Ok(()));
            let mut expected = vec![Op::ReadText, Op::ReadImage];
            expected.extend(pasted_then(&config, Op::Clear));
            assert_eq!(*log.borrow(), expected, "texto salvo {text:?}");
        }
    }

    #[test]
    fn failed_chord_still_restores() {
        let config = PasteConfig::default();
        for (text, image, restore) in [
            (Some("antes"), None, Op::WriteText("antes".to_owned())),
            (None, Some(image()), Op::WriteImage(image())),
            (None, None, Op::Clear),
        ] {
            let log = Log::default();
            let mut clipboard = FakeClipboard::new(&log, text, image);
            let result = run(&mut clipboard, true, &config, "ditado");
            assert!(
                matches!(result, Err(InjectError::Keystroke(_))),
                "{result:?}"
            );
            assert_eq!(log.borrow().last(), Some(&restore));
        }
    }

    #[test]
    fn failed_write_skips_chord() {
        let log = Log::default();
        let mut clipboard = FakeClipboard::new(&log, Some("antes"), None);
        clipboard.fail_write_text = true;
        let result = run(&mut clipboard, false, &PasteConfig::default(), "ditado");
        assert!(
            matches!(result, Err(InjectError::Clipboard(_))),
            "{result:?}"
        );
        assert!(
            !log.borrow().iter().any(|op| matches!(op, Op::Chord(..))),
            "{:?}",
            log.borrow()
        );
    }

    static CAPTURED: Mutex<Vec<String>> = Mutex::new(Vec::new());

    struct CaptureLogger;

    impl log::Log for CaptureLogger {
        fn enabled(&self, _: &log::Metadata) -> bool {
            true
        }
        fn log(&self, record: &log::Record) {
            if let Ok(mut captured) = CAPTURED.lock() {
                captured.push(record.args().to_string());
            }
        }
        fn flush(&self) {}
    }

    #[test]
    fn paste_never_logs_text_or_clipboard_content() {
        const DICTATED: &str = "FALA-DITADO-7f3a";
        const SAVED: &str = "FALA-SALVO-9c1e";
        log::set_logger(&CaptureLogger).expect("só este teste instala um logger");
        log::set_max_level(log::LevelFilter::Trace);
        log::trace!("sonda do logger de teste");

        let config = PasteConfig::default();
        for (chord_fails, restore_fails) in [(false, false), (true, false), (true, true)] {
            let log = Log::default();
            let mut clipboard = FakeClipboard::new(&log, Some(SAVED), None);
            clipboard.fail_restore = restore_fails;
            let _ = run(&mut clipboard, chord_fails, &config, DICTATED);
        }

        let captured = CAPTURED.lock().expect("lock do logger").clone();
        assert!(
            captured.iter().any(|line| line.contains("sonda do logger")),
            "o logger de teste não capturou nada: {captured:?}"
        );
        assert!(
            captured
                .iter()
                .any(|line| line.contains("não foi restaurado")),
            "o caminho de restauração falha não logou: {captured:?}"
        );
        for line in &captured {
            assert!(
                !line.contains(DICTATED) && !line.contains(SAVED),
                "conteúdo no log: {line}"
            );
        }
    }
}
