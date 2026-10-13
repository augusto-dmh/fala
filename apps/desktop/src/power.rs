//! The PC and sleep while a meeting records (`meeting-power-guard`).
//!
//! [`PowerGuard`] asks the OS not to sleep for inactivity; the display may still turn off. The
//! request belongs to the thread that makes it (Windows' `SetThreadExecutionState`), so the
//! meeting recorder thread holds the guard and the request ends with the thread, even on a panic.
//! Outside Windows it is a no-op for now.

/// The OS call behind [`PowerGuard`]; tests count the calls with a fake.
pub(crate) trait PowerApi {
    /// `true`: the system stays awake while this thread runs; `false`: back to the power plan.
    fn keep_awake(&self, on: bool);
}

/// Keeps the system awake while held and set; released on drop.
pub(crate) struct PowerGuard<A: PowerApi> {
    api: A,
    held: bool,
}

impl<A: PowerApi> PowerGuard<A> {
    /// Asks nothing yet.
    pub(crate) fn new(api: A) -> Self {
        Self { api, held: false }
    }

    /// Calls the OS only when the request changes.
    pub(crate) fn set(&mut self, awake: bool) {
        if awake != self.held {
            self.api.keep_awake(awake);
            self.held = awake;
        }
    }
}

impl<A: PowerApi> Drop for PowerGuard<A> {
    fn drop(&mut self) {
        self.set(false);
    }
}

/// The power API of this OS.
pub(crate) struct Os;

#[cfg(windows)]
impl PowerApi for Os {
    fn keep_awake(&self, on: bool) {
        use windows::Win32::System::Power::{
            SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED,
        };
        // Never ES_DISPLAY_REQUIRED: the screen may go dark, the system may not sleep.
        let flags = if on {
            ES_CONTINUOUS | ES_SYSTEM_REQUIRED
        } else {
            ES_CONTINUOUS
        };
        // SAFETY: plain Win32 call with valid flags; it only changes this thread's request.
        let previous = unsafe { SetThreadExecutionState(flags) };
        if previous.0 == 0 {
            log::warn!("the system did not take the keep-awake request ({on})");
        } else {
            log::debug!("keep the system awake: {on}");
        }
    }
}

#[cfg(not(windows))]
impl PowerApi for Os {
    fn keep_awake(&self, _on: bool) {}
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Records every `keep_awake` call.
    #[derive(Clone, Default)]
    pub(crate) struct FakePower(pub(crate) Arc<Mutex<Vec<bool>>>);

    impl FakePower {
        pub(crate) fn calls(&self) -> Vec<bool> {
            self.0.lock().unwrap().clone()
        }
    }

    impl PowerApi for FakePower {
        fn keep_awake(&self, on: bool) {
            self.0.lock().unwrap().push(on);
        }
    }

    #[test]
    fn the_guard_calls_the_os_only_on_change_and_lets_go_on_drop() {
        let fake = FakePower::default();
        {
            let mut guard = PowerGuard::new(fake.clone());
            assert_eq!(fake.calls(), Vec::<bool>::new());
            guard.set(true);
            guard.set(true);
            guard.set(false);
            guard.set(true);
        }
        assert_eq!(fake.calls(), vec![true, false, true, false]);
    }

    #[test]
    fn a_guard_never_set_asks_nothing() {
        let fake = FakePower::default();
        drop(PowerGuard::new(fake.clone()));
        assert!(fake.calls().is_empty());
    }

    #[test]
    fn a_panic_lets_go_too() {
        let fake = FakePower::default();
        let held = fake.clone();
        let result = std::thread::spawn(move || {
            let mut guard = PowerGuard::new(held);
            guard.set(true);
            panic!("recorder thread dies");
        })
        .join();
        assert!(result.is_err());
        assert_eq!(fake.calls(), vec![true, false]);
    }
}
