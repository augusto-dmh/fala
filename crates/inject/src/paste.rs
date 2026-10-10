//! Inserção de texto no app em foco por clipboard + acorde de colar, com prova de leitura e o
//! clipboard restaurado.
//!
//! A base é a sequência de `apps/desktop/src/clipboard.rs::paste_via_clipboard`, que o spike 03
//! mediu no Windows: salva o texto do clipboard (a imagem, só quando não há texto), oferece o
//! texto, espera, manda o acorde e devolve o que estava lá, ou limpa se não havia nada. A oferta
//! só entrega o texto quando um app o lê (no Windows, delayed rendering), e é essa leitura que
//! prova a cola: se ninguém lê em `read_timeout`, `insert` devolve `PasteNotRead`. Um app que lê
//! e não cola conta como cola feita; não há como distinguir sem ler o campo. A restauração só
//! acontece se ninguém escreveu no clipboard desde a oferta. A sequência é neutra de plataforma;
//! o adaptador só fornece o clipboard e o teclado.

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
    /// Cola em terminais e apps que não aceitam Ctrl+V.
    ShiftInsert,
}

/// Acorde e tempos da colagem. O padrão é o do desktop: Ctrl+V, 60 ms antes, 60 ms depois e o
/// modificador seguro por 100 ms; e 1,5 s para o destino ler o texto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasteConfig {
    pub chord: PasteChord,
    /// Espera entre oferecer o texto no clipboard e mandar o acorde.
    pub delay_before: Duration,
    /// Espera entre a leitura do texto (ou o fim de `read_timeout`) e a restauração.
    pub delay_after: Duration,
    /// Quanto tempo o modificador fica pressionado depois da tecla.
    pub modifier_hold: Duration,
    /// Prazo para algum app ler o texto depois do acorde; sem leitura, `PasteNotRead`.
    pub read_timeout: Duration,
}

impl Default for PasteConfig {
    fn default() -> Self {
        Self {
            chord: PasteChord::CtrlV,
            delay_before: Duration::from_millis(60),
            delay_after: Duration::from_millis(60),
            modifier_hold: Duration::from_millis(100),
            read_timeout: Duration::from_millis(1500),
        }
    }
}

/// Insere texto no app em foco.
pub trait Injector: Send {
    /// Cola `text` no app em foco e devolve ao clipboard o que estava nele, mesmo se o acorde
    /// falhar, a menos que alguém tenha escrito no clipboard nesse meio-tempo. Devolve
    /// `PasteNotRead` se nenhum app leu o texto em `read_timeout`; o texto continua com quem
    /// chamou.
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
    /// Põe `text` no clipboard para ser entregue só quando um app o ler, fora do histórico e da
    /// nuvem.
    fn offer_text(&mut self, text: &str) -> Result<(), InjectError>;
    /// Espera até `timeout` que um app leia o texto oferecido; `true` se leu.
    fn wait_read(&mut self, timeout: Duration) -> bool;
    /// `true` se ninguém escreveu no clipboard desde a oferta (ou desde a entrega do texto).
    fn unchanged_since_offer(&mut self) -> bool;
    /// Encerra a oferta. Sempre chamado depois da restauração.
    fn end_offer(&mut self);
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

    if let Err(err) = clipboard.offer_text(text) {
        clipboard.end_offer();
        return Err(err);
    }
    sleep(config.delay_before);
    let result = keyboard
        .send_chord(config.chord, config.modifier_hold)
        .and_then(|()| {
            if clipboard.wait_read(config.read_timeout) {
                Ok(())
            } else {
                Err(InjectError::PasteNotRead)
            }
        });
    sleep(config.delay_after);

    if clipboard.unchanged_since_offer() {
        let restored = match &saved {
            Saved::Text(saved) => clipboard.write_text(saved),
            Saved::Image(image) => clipboard.write_image(image),
            Saved::Nothing => clipboard.clear(),
        };
        if let Err(err) = restored {
            log::warn!("o clipboard não foi restaurado depois da colagem: {err}");
        }
    } else {
        log::debug!("o clipboard mudou desde a colagem; o conteúdo anterior não foi restaurado");
    }
    clipboard.end_offer();
    result
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
        Offer(String),
        WaitRead(Duration),
        Unchanged,
        EndOffer,
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
        /// Algum app lê o texto oferecido.
        read: bool,
        /// Alguém escreve no clipboard depois da oferta.
        changed: bool,
        fail_offer: bool,
        fail_restore: bool,
    }

    impl FakeClipboard {
        fn new(log: &Log, text: Option<&str>, image: Option<Image>) -> Self {
            Self {
                log: log.clone(),
                text: text.map(str::to_owned),
                image,
                read: true,
                changed: false,
                fail_offer: false,
                fail_restore: false,
            }
        }

        fn push(&self, op: Op) {
            self.log.borrow_mut().push(op);
        }

        fn restore(&self, op: Op) -> Result<(), InjectError> {
            if self.fail_restore {
                return Err(InjectError::Clipboard("escrita recusada".to_owned()));
            }
            self.push(op);
            Ok(())
        }
    }

    impl Clipboard for FakeClipboard {
        type Image = Image;
        fn read_text(&mut self) -> Result<String, InjectError> {
            self.push(Op::ReadText);
            self.text
                .clone()
                .ok_or_else(|| InjectError::Clipboard("sem texto".to_owned()))
        }
        fn read_image(&mut self) -> Result<Image, InjectError> {
            self.push(Op::ReadImage);
            self.image
                .clone()
                .ok_or_else(|| InjectError::Clipboard("sem imagem".to_owned()))
        }
        fn offer_text(&mut self, text: &str) -> Result<(), InjectError> {
            if self.fail_offer {
                return Err(InjectError::Clipboard("oferta recusada".to_owned()));
            }
            self.push(Op::Offer(text.to_owned()));
            Ok(())
        }
        fn wait_read(&mut self, timeout: Duration) -> bool {
            self.push(Op::WaitRead(timeout));
            self.read
        }
        fn unchanged_since_offer(&mut self) -> bool {
            self.push(Op::Unchanged);
            !self.changed
        }
        fn end_offer(&mut self) {
            self.push(Op::EndOffer);
        }
        fn write_text(&mut self, text: &str) -> Result<(), InjectError> {
            self.restore(Op::WriteText(text.to_owned()))
        }
        fn write_image(&mut self, image: &Image) -> Result<(), InjectError> {
            self.restore(Op::WriteImage(image.clone()))
        }
        fn clear(&mut self) -> Result<(), InjectError> {
            self.restore(Op::Clear)
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

    /// Texto e imagem salvos, as leituras que a sequência faz e a restauração esperada.
    type SavedCase = (Option<&'static str>, Option<Image>, Vec<Op>, Op);

    /// Os conteúdos salvos: texto; imagem sem texto (ou com texto vazio); nada.
    fn saved_cases() -> Vec<SavedCase> {
        let image_reads = || vec![Op::ReadText, Op::ReadImage];
        vec![
            (
                Some("antes"),
                Some(image()),
                vec![Op::ReadText],
                Op::WriteText("antes".to_owned()),
            ),
            (None, Some(image()), image_reads(), Op::WriteImage(image())),
            (
                Some(""),
                Some(image()),
                image_reads(),
                Op::WriteImage(image()),
            ),
            (None, None, image_reads(), Op::Clear),
            (Some(""), None, image_reads(), Op::Clear),
        ]
    }

    /// Da oferta ao fim, com o acorde enviado.
    fn pasted_then(config: &PasteConfig, restore: Option<Op>) -> Vec<Op> {
        let mut ops = vec![
            Op::Offer("ditado".to_owned()),
            Op::Sleep(config.delay_before),
            Op::Chord(config.chord, config.modifier_hold),
            Op::WaitRead(config.read_timeout),
            Op::Sleep(config.delay_after),
            Op::Unchanged,
        ];
        ops.extend(restore);
        ops.push(Op::EndOffer);
        ops
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
                read_timeout: Duration::from_millis(1500),
            }
        );
    }

    #[test]
    fn read_paste_restores_in_order() {
        for chord in [
            PasteChord::CtrlV,
            PasteChord::CtrlShiftV,
            PasteChord::ShiftInsert,
        ] {
            let config = PasteConfig {
                chord,
                ..PasteConfig::default()
            };
            for (text, image, reads, restore) in saved_cases() {
                let log = Log::default();
                let mut clipboard = FakeClipboard::new(&log, text, image);
                assert_eq!(run(&mut clipboard, false, &config, "ditado"), Ok(()));
                let mut expected = reads;
                expected.extend(pasted_then(&config, Some(restore)));
                assert_eq!(*log.borrow(), expected, "{chord:?} texto salvo {text:?}");
            }
        }
    }

    #[test]
    fn unread_paste_is_reported_and_restored() {
        let config = PasteConfig::default();
        for (text, image, reads, restore) in saved_cases() {
            let log = Log::default();
            let mut clipboard = FakeClipboard::new(&log, text, image);
            clipboard.read = false;
            assert_eq!(
                run(&mut clipboard, false, &config, "ditado"),
                Err(InjectError::PasteNotRead)
            );
            let mut expected = reads;
            expected.extend(pasted_then(&config, Some(restore)));
            assert_eq!(*log.borrow(), expected, "texto salvo {text:?}");
        }
    }

    #[test]
    fn changed_clipboard_is_not_restored() {
        let config = PasteConfig::default();
        for (read, outcome) in [(true, Ok(())), (false, Err(InjectError::PasteNotRead))] {
            let log = Log::default();
            let mut clipboard = FakeClipboard::new(&log, Some("antes"), None);
            clipboard.read = read;
            clipboard.changed = true;
            assert_eq!(run(&mut clipboard, false, &config, "ditado"), outcome);
            let mut expected = vec![Op::ReadText];
            expected.extend(pasted_then(&config, None));
            assert_eq!(*log.borrow(), expected, "lido: {read}");
        }
    }

    #[test]
    fn failed_chord_still_restores() {
        let config = PasteConfig::default();
        for (text, image, reads, restore) in saved_cases() {
            let log = Log::default();
            let mut clipboard = FakeClipboard::new(&log, text, image);
            let result = run(&mut clipboard, true, &config, "ditado");
            assert!(
                matches!(result, Err(InjectError::Keystroke(_))),
                "{result:?}"
            );
            let mut expected = reads;
            expected.extend([
                Op::Offer("ditado".to_owned()),
                Op::Sleep(config.delay_before),
                Op::Chord(config.chord, config.modifier_hold),
                Op::Sleep(config.delay_after),
                Op::Unchanged,
                restore,
                Op::EndOffer,
            ]);
            assert_eq!(*log.borrow(), expected, "texto salvo {text:?}");
        }
    }

    #[test]
    fn failed_offer_skips_chord() {
        let log = Log::default();
        let mut clipboard = FakeClipboard::new(&log, Some("antes"), None);
        clipboard.fail_offer = true;
        let result = run(&mut clipboard, false, &PasteConfig::default(), "ditado");
        assert!(
            matches!(result, Err(InjectError::Clipboard(_))),
            "{result:?}"
        );
        assert_eq!(*log.borrow(), vec![Op::ReadText, Op::EndOffer]);
    }

    static CAPTURED: Mutex<Vec<String>> = Mutex::new(Vec::new());

    struct CaptureLogger;

    impl log::Log for CaptureLogger {
        fn enabled(&self, _: &log::Metadata) -> bool {
            true
        }
        fn log(&self, record: &log::Record) {
            if let Ok(mut captured) = CAPTURED.lock() {
                captured.push(format!("{} {}", record.level(), record.args()));
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
        // (acorde falha, restauração falha, lido, clipboard mudou)
        for (chord_fails, restore_fails, read, changed) in [
            (false, false, true, false),
            (false, false, false, false),
            (false, false, true, true),
            (false, false, false, true),
            (true, false, true, false),
            (true, true, true, false),
        ] {
            let log = Log::default();
            let mut clipboard = FakeClipboard::new(&log, Some(SAVED), None);
            clipboard.fail_restore = restore_fails;
            clipboard.read = read;
            clipboard.changed = changed;
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
                .any(|line| line.contains("não foi restaurado depois")),
            "o caminho de restauração falha não logou: {captured:?}"
        );
        assert!(
            captured
                .iter()
                .any(|line| line.starts_with("DEBUG") && line.contains("mudou desde a colagem")),
            "o caminho de clipboard mudado não logou em debug: {captured:?}"
        );
        for line in &captured {
            assert!(
                !line.contains(DICTATED) && !line.contains(SAVED),
                "conteúdo no log: {line}"
            );
        }
    }
}
