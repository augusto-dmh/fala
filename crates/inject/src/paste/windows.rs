//! Adaptador Windows da colagem: clipboard pelo `arboard` e acorde pelo `enigo`, as bibliotecas
//! que o desktop usa e que o spike 03 mediu (`tauri-plugin-clipboard-manager` embrulha o
//! `arboard`; `input.rs::send_paste_ctrl_v` usa `Key::Control` + `Key::Other(0x56)`).
//!
//! TODO(windows): verificar à mão com `cargo run -p fala-inject --example paste` e o Bloco de
//! Notas em foco.

// Ligada à sequência de colagem no PR seguinte da pilha.
#[allow(dead_code)]
mod offer;

use std::time::Duration;

use arboard::ImageData;
use enigo::{Direction, Enigo, Key, Keyboard as _, Settings};

use super::{paste_with_restore, Clipboard, Injector, Keyboard, PasteChord, PasteConfig};
use crate::InjectError;

/// Tecla virtual `VK_V`, independente do layout do teclado.
const VK_V: u32 = 0x56;

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
            clipboard: SystemClipboard(clipboard),
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

struct SystemClipboard(arboard::Clipboard);

impl Clipboard for SystemClipboard {
    type Image = ImageData<'static>;

    fn read_text(&mut self) -> Result<String, InjectError> {
        self.0.get_text().map_err(clipboard_error)
    }

    fn read_image(&mut self) -> Result<Self::Image, InjectError> {
        self.0.get_image().map_err(clipboard_error)
    }

    fn write_text(&mut self, text: &str) -> Result<(), InjectError> {
        self.0.set_text(text).map_err(clipboard_error)
    }

    fn write_image(&mut self, image: &Self::Image) -> Result<(), InjectError> {
        self.0.set_image(image.clone()).map_err(clipboard_error)
    }

    fn clear(&mut self) -> Result<(), InjectError> {
        self.0.clear().map_err(clipboard_error)
    }
}

struct SystemKeyboard(Enigo);

impl Keyboard for SystemKeyboard {
    fn send_chord(
        &mut self,
        chord: PasteChord,
        modifier_hold: Duration,
    ) -> Result<(), InjectError> {
        let modifiers: &[Key] = match chord {
            PasteChord::CtrlV => &[Key::Control],
            PasteChord::CtrlShiftV => &[Key::Control, Key::Shift],
        };
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
            result = enigo
                .key(Key::Other(VK_V), Direction::Click)
                .map_err(key_error);
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
