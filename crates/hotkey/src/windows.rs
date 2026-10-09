//! Adaptador Windows sobre o `handy-keys` (hook `WH_KEYBOARD_LL`), o mesmo que o spike 02 mediu.
//!
//! O modelo de threads é o de `apps/desktop/src/shortcut/fala_keys.rs`: uma thread é dona do
//! `HotkeyManager`, recebe `register`/`unregister` por canal e repassa os eventos do hook a cada
//! 10 ms. O modo é bloqueante: o acelerador registrado não chega ao app em foco.
//!
//! TODO(windows): verificar à mão com `cargo run -p fala-hotkey --example hotkey_listen -- f9 60`
//! e 60 toques físicos de F9.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use handy_keys::{Hotkey, HotkeyId, HotkeyManager, HotkeyState};

use crate::bindings::Bindings;
use crate::{GlobalHotkey, HotkeyError, HotkeyEvent, KeyState};

const POLL: Duration = Duration::from_millis(10);

enum Command {
    Register {
        binding_id: String,
        accelerator: String,
        hotkey: Hotkey,
        response: Sender<Result<(), HotkeyError>>,
    },
    Unregister {
        binding_id: String,
        response: Sender<Result<(), HotkeyError>>,
    },
    Shutdown,
}

pub(crate) struct WindowsHotkey {
    commands: Sender<Command>,
    thread: Option<JoinHandle<()>>,
}

impl WindowsHotkey {
    /// Sobe a thread dona do `HotkeyManager` e espera o hook ser instalado.
    pub(crate) fn start(events: Sender<HotkeyEvent>) -> Result<Self, HotkeyError> {
        let (commands, command_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("fala-hotkey".to_owned())
            .spawn(move || manager_thread(command_rx, events, ready_tx))
            .map_err(|err| HotkeyError::Backend(err.to_string()))?;
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                commands,
                thread: Some(thread),
            }),
            Ok(Err(err)) => {
                let _ = thread.join();
                Err(err)
            }
            Err(_) => {
                let _ = thread.join();
                Err(HotkeyError::Backend(
                    "a thread do hook terminou antes de responder".to_owned(),
                ))
            }
        }
    }

    fn request(
        &self,
        command: impl FnOnce(Sender<Result<(), HotkeyError>>) -> Command,
    ) -> Result<(), HotkeyError> {
        let (response, response_rx) = mpsc::channel();
        self.commands
            .send(command(response))
            .map_err(|_| thread_gone())?;
        response_rx.recv().map_err(|_| thread_gone())?
    }
}

impl GlobalHotkey for WindowsHotkey {
    fn register(&mut self, binding_id: &str, accelerator: &str) -> Result<(), HotkeyError> {
        let hotkey = parse_accelerator(accelerator)?;
        self.request(|response| Command::Register {
            binding_id: binding_id.to_owned(),
            accelerator: accelerator.to_owned(),
            hotkey,
            response,
        })
    }

    fn unregister(&mut self, binding_id: &str) -> Result<(), HotkeyError> {
        self.request(|response| Command::Unregister {
            binding_id: binding_id.to_owned(),
            response,
        })
    }
}

impl Drop for WindowsHotkey {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn thread_gone() -> HotkeyError {
    HotkeyError::Backend("a thread do hook não está rodando".to_owned())
}

/// Interpreta o acelerador no formato do `handy-keys` (`ctrl+shift+space`, `f9`, `ctrl+shift`).
fn parse_accelerator(accelerator: &str) -> Result<Hotkey, HotkeyError> {
    accelerator
        .parse()
        .map_err(|err: handy_keys::Error| HotkeyError::InvalidAccelerator {
            accelerator: accelerator.to_owned(),
            reason: err.to_string(),
        })
}

fn key_state(state: HotkeyState) -> KeyState {
    match state {
        HotkeyState::Pressed => KeyState::Pressed,
        HotkeyState::Released => KeyState::Released,
    }
}

fn manager_thread(
    commands: Receiver<Command>,
    events: Sender<HotkeyEvent>,
    ready: Sender<Result<(), HotkeyError>>,
) {
    let manager = match HotkeyManager::new_with_blocking() {
        Ok(manager) => manager,
        Err(err) => {
            let _ = ready.send(Err(HotkeyError::Backend(err.to_string())));
            return;
        }
    };
    if ready.send(Ok(())).is_err() {
        return;
    }
    let mut bindings = Bindings::default();
    // O `HotkeyId` do `handy-keys` só nasce do `register`; a tabela usa o `u32` dele.
    let mut ids: HashMap<u32, HotkeyId> = HashMap::new();

    loop {
        while let Some(event) = manager.try_recv() {
            if let Some(event) = bindings.event(event.id.as_u32(), key_state(event.state)) {
                log::debug!(
                    "atalho {}: {:?} ({})",
                    event.binding_id,
                    event.state,
                    event.accelerator
                );
                let _ = events.send(event);
            }
        }

        match commands.recv_timeout(POLL) {
            Ok(Command::Register {
                binding_id,
                accelerator,
                hotkey,
                response,
            }) => {
                let result = bindings.register_with(&binding_id, &accelerator, || {
                    let id = manager
                        .register(hotkey)
                        .map_err(|err| HotkeyError::Backend(err.to_string()))?;
                    ids.insert(id.as_u32(), id);
                    Ok(id.as_u32())
                });
                let _ = response.send(result);
            }
            Ok(Command::Unregister {
                binding_id,
                response,
            }) => {
                // O id só sai da tabela depois de o hook aceitar: uma falha pode ser repetida.
                let result = bindings.unregister_with(&binding_id, |raw| {
                    if let Some(&id) = ids.get(&raw) {
                        manager
                            .unregister(id)
                            .map_err(|err| HotkeyError::Backend(err.to_string()))?;
                        ids.remove(&raw);
                    }
                    Ok(())
                });
                let _ = response.send(result);
            }
            Ok(Command::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_accelerators_are_rejected() {
        // Sem thread do outro lado: um acelerador aceito por engano devolve `Backend` em vez de
        // esperar para sempre por uma resposta.
        let (commands, command_rx) = mpsc::channel();
        drop(command_rx);
        let mut hotkey = WindowsHotkey {
            commands,
            thread: None,
        };
        for accelerator in ["", "ctrl+banana", "ctrl+a+b"] {
            match hotkey.register("transcribe", accelerator) {
                Err(HotkeyError::InvalidAccelerator {
                    accelerator: got, ..
                }) => assert_eq!(got, accelerator),
                other => panic!("{accelerator:?}: esperava InvalidAccelerator, veio {other:?}"),
            }
        }
    }

    #[test]
    fn desktop_accelerators_parse() {
        for accelerator in ["ctrl+shift+space", "f9", "ctrl+shift"] {
            assert!(
                parse_accelerator(accelerator).is_ok(),
                "{accelerator:?} deveria ser aceito"
            );
        }
    }

    #[test]
    fn hook_states_map_to_key_state() {
        assert_eq!(key_state(HotkeyState::Pressed), KeyState::Pressed);
        assert_eq!(key_state(HotkeyState::Released), KeyState::Released);
    }

    #[test]
    fn register_after_thread_gone_is_backend_error() {
        let (commands, command_rx) = mpsc::channel();
        drop(command_rx);
        let mut hotkey = WindowsHotkey {
            commands,
            thread: None,
        };
        assert!(matches!(
            hotkey.register("transcribe", "f9"),
            Err(HotkeyError::Backend(_))
        ));
    }
}
