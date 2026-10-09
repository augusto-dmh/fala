//! Adaptador Windows da colagem: o texto vai ao clipboard por delayed rendering ([`offer`]), o
//! clipboard é salvo e restaurado pelo `arboard`, e o acorde sai pelo `enigo` (as bibliotecas
//! que o desktop usa e que o spike 03 mediu).

mod offer;

use std::time::Duration;

use arboard::{ImageData, SetExtWindows as _};
use enigo::{Direction, Enigo, Key, Keyboard as _, Settings};

use self::offer::DelayedOffer;
use super::{paste_with_restore, Clipboard, Injector, Keyboard, PasteChord, PasteConfig};
use crate::InjectError;

/// Teclas virtuais, independentes do layout do teclado.
const VK_V: u32 = 0x56;
const VK_INSERT: u32 = 0x2D;

pub(crate) struct WindowsInjector {
    clipboard: SystemClipboard,
    keyboard: SystemKeyboard,
    config: PasteConfig,
}

impl WindowsInjector {
    pub(crate) fn new(config: PasteConfig) -> Result<Self, InjectError> {
        let clipboard = arboard::Clipboard::new().map_err(clipboard_error)?;
        let enigo = Enigo::new(&Settings::default())
            .map_err(|err| InjectError::Keystroke(err.to_string()))?;
        Ok(Self {
            clipboard: SystemClipboard {
                arboard: clipboard,
                offer: None,
            },
            keyboard: SystemKeyboard(enigo),
            config,
        })
    }
}

impl Injector for WindowsInjector {
    fn insert(&mut self, text: &str) -> Result<(), InjectError> {
        paste_with_restore(
            &mut self.clipboard,
            &mut self.keyboard,
            &mut std::thread::sleep,
            &self.config,
            text,
        )
    }
}

fn clipboard_error(err: arboard::Error) -> InjectError {
    InjectError::Clipboard(err.to_string())
}

fn key_error(err: enigo::InputError) -> InjectError {
    InjectError::Keystroke(err.to_string())
}

struct SystemClipboard {
    arboard: arboard::Clipboard,
    offer: Option<DelayedOffer>,
}

impl Clipboard for SystemClipboard {
    type Image = ImageData<'static>;

    fn read_text(&mut self) -> Result<String, InjectError> {
        self.arboard.get_text().map_err(clipboard_error)
    }

    fn read_image(&mut self) -> Result<Self::Image, InjectError> {
        self.arboard.get_image().map_err(clipboard_error)
    }

    fn offer_text(&mut self, text: &str) -> Result<(), InjectError> {
        self.offer = None;
        self.offer = Some(DelayedOffer::start(text)?);
        Ok(())
    }

    fn wait_read(&mut self, timeout: Duration) -> bool {
        self.offer
            .as_mut()
            .is_some_and(|offer| offer.wait_read(timeout))
    }

    fn unchanged_since_offer(&mut self) -> bool {
        self.offer.as_mut().is_some_and(DelayedOffer::unchanged)
    }

    fn end_offer(&mut self) {
        self.offer = None;
    }

    // O conteúdo restaurado já passou pelo histórico quando a pessoa o copiou; não entra de novo.
    fn write_text(&mut self, text: &str) -> Result<(), InjectError> {
        self.arboard
            .set()
            .exclude_from_history()
            .exclude_from_cloud()
            .text(text)
            .map_err(clipboard_error)
    }

    fn write_image(&mut self, image: &Self::Image) -> Result<(), InjectError> {
        self.arboard
            .set()
            .exclude_from_history()
            .exclude_from_cloud()
            .image(image.clone())
            .map_err(clipboard_error)
    }

    fn clear(&mut self) -> Result<(), InjectError> {
        self.arboard.clear().map_err(clipboard_error)
    }
}

/// Os modificadores e a tecla de cada acorde.
fn chord_keys(chord: PasteChord) -> (&'static [Key], Key) {
    match chord {
        PasteChord::CtrlV => (&[Key::Control], Key::Other(VK_V)),
        PasteChord::CtrlShiftV => (&[Key::Control, Key::Shift], Key::Other(VK_V)),
        // O `enigo` marca o VK_INSERT como tecla estendida, o Insert de verdade e não o 0 do
        // teclado numérico.
        PasteChord::ShiftInsert => (&[Key::Shift], Key::Other(VK_INSERT)),
    }
}

struct SystemKeyboard(Enigo);

impl Keyboard for SystemKeyboard {
    fn send_chord(
        &mut self,
        chord: PasteChord,
        modifier_hold: Duration,
    ) -> Result<(), InjectError> {
        let (modifiers, key) = chord_keys(chord);
        let enigo = &mut self.0;
        let mut pressed = 0;
        let mut result = Ok(());
        for &modifier in modifiers {
            result = enigo.key(modifier, Direction::Press).map_err(key_error);
            if result.is_err() {
                break;
            }
            pressed += 1;
        }
        if result.is_ok() {
            result = enigo.key(key, Direction::Click).map_err(key_error);
            std::thread::sleep(modifier_hold);
        }
        // Solta, na ordem inversa, todo modificador que chegou a ser pressionado, mesmo depois de
        // uma falha: um Ctrl preso estragaria o que a pessoa digitar em seguida.
        for &modifier in modifiers[..pressed].iter().rev() {
            let released = enigo.key(modifier, Direction::Release).map_err(key_error);
            if result.is_ok() {
                result = released;
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chords_map_to_keys() {
        assert_eq!(
            chord_keys(PasteChord::CtrlV),
            (&[Key::Control][..], Key::Other(0x56))
        );
        assert_eq!(
            chord_keys(PasteChord::CtrlShiftV),
            (&[Key::Control, Key::Shift][..], Key::Other(0x56))
        );
        assert_eq!(
            chord_keys(PasteChord::ShiftInsert),
            (&[Key::Shift][..], Key::Other(0x2D))
        );
    }
}
