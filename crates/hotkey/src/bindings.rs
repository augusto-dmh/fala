//! A tabela dos atalhos registrados: `binding_id` ↔ id do backend, e o roteamento de um evento
//! do backend para o [`HotkeyEvent`] do binding. Neutra de plataforma, para ser testada fora do
//! Windows; o adaptador passa o id do backend como `u32`.

use std::collections::HashMap;

use crate::{HotkeyError, HotkeyEvent, KeyState};

#[derive(Debug, Default)]
pub(crate) struct Bindings {
    by_binding: HashMap<String, u32>,
    /// id do backend → (`binding_id`, acelerador).
    by_id: HashMap<u32, (String, String)>,
}

impl Bindings {
    /// Registra `binding_id` com o id que `backend` devolver. Um `binding_id` já registrado é
    /// recusado antes de chamar o backend.
    pub(crate) fn register_with(
        &mut self,
        binding_id: &str,
        accelerator: &str,
        backend: impl FnOnce() -> Result<u32, HotkeyError>,
    ) -> Result<(), HotkeyError> {
        if self.by_binding.contains_key(binding_id) {
            return Err(HotkeyError::AlreadyRegistered(binding_id.to_owned()));
        }
        let id = backend()?;
        self.by_binding.insert(binding_id.to_owned(), id);
        self.by_id
            .insert(id, (binding_id.to_owned(), accelerator.to_owned()));
        Ok(())
    }

    /// Remove `binding_id`, chamando `backend` com o id dele. Desconhecido não é erro.
    pub(crate) fn unregister_with(
        &mut self,
        binding_id: &str,
        backend: impl FnOnce(u32) -> Result<(), HotkeyError>,
    ) -> Result<(), HotkeyError> {
        let Some(&id) = self.by_binding.get(binding_id) else {
            return Ok(());
        };
        backend(id)?;
        self.by_binding.remove(binding_id);
        self.by_id.remove(&id);
        Ok(())
    }

    /// O evento de `binding_id` para um evento do backend, ou `None` se o id não está registrado.
    pub(crate) fn event(&self, id: u32, state: KeyState) -> Option<HotkeyEvent> {
        self.by_id
            .get(&id)
            .map(|(binding_id, accelerator)| HotkeyEvent {
                binding_id: binding_id.clone(),
                accelerator: accelerator.clone(),
                state,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_transcribe() -> Bindings {
        let mut bindings = Bindings::default();
        bindings
            .register_with("transcribe", "ctrl+shift+space", || Ok(7))
            .expect("primeiro registro");
        bindings
    }

    #[test]
    fn events_route_to_binding_in_order() {
        let bindings = with_transcribe();
        let events: Vec<HotkeyEvent> = [KeyState::Pressed, KeyState::Released]
            .into_iter()
            .filter_map(|state| bindings.event(7, state))
            .collect();
        let event = |state| HotkeyEvent {
            binding_id: "transcribe".to_owned(),
            accelerator: "ctrl+shift+space".to_owned(),
            state,
        };
        assert_eq!(
            events,
            vec![event(KeyState::Pressed), event(KeyState::Released)]
        );
    }

    #[test]
    fn unknown_or_unregistered_id_sends_nothing() {
        let mut bindings = with_transcribe();
        assert_eq!(bindings.event(9, KeyState::Pressed), None);
        bindings
            .unregister_with("transcribe", |id| {
                assert_eq!(id, 7);
                Ok(())
            })
            .expect("desregistrar");
        assert_eq!(bindings.event(7, KeyState::Pressed), None);
        assert_eq!(bindings.event(7, KeyState::Released), None);
    }

    #[test]
    fn duplicate_binding_is_rejected_and_keeps_first() {
        let mut bindings = with_transcribe();
        let mut backend_called = false;
        let result = bindings.register_with("transcribe", "f9", || {
            backend_called = true;
            Ok(8)
        });
        assert_eq!(
            result,
            Err(HotkeyError::AlreadyRegistered("transcribe".to_owned()))
        );
        assert!(!backend_called, "o backend não pode ser chamado");
        let event = bindings
            .event(7, KeyState::Pressed)
            .expect("id 7 segue registrado");
        assert_eq!(event.accelerator, "ctrl+shift+space");
        assert_eq!(bindings.event(8, KeyState::Pressed), None);
    }

    #[test]
    fn unregister_unknown_binding_is_ok() {
        let mut bindings = with_transcribe();
        let mut backend_called = false;
        let result = bindings.unregister_with("nunca", |_| {
            backend_called = true;
            Ok(())
        });
        assert_eq!(result, Ok(()));
        assert!(!backend_called, "o backend não pode ser chamado");
    }
}
