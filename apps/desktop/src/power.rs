//! The PC and sleep while a meeting records (`meeting-power-guard`).
//!
//! [`PowerGuard`] asks the OS not to sleep for inactivity; the display may still turn off. The
//! request belongs to the thread that makes it (Windows' `SetThreadExecutionState`), so the
//! meeting recorder thread holds the guard and the request ends with the thread, even on a panic.
//! [`watch_sleep`] reports a sleep that happens anyway (lid closed, power button, battery).
//! Outside Windows both are no-ops for now.

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

/// A sleep of the whole system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SleepEvent {
    /// The system is about to sleep; the handler has about two seconds.
    Suspending,
    /// The system woke up. Windows may say so twice (automatic, then by the user).
    Resumed,
}

/// The event a Windows power broadcast means, if any.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn sleep_event(broadcast: u32) -> Option<SleepEvent> {
    // PBT_APMSUSPEND, PBT_APMRESUMESUSPEND and PBT_APMRESUMEAUTOMATIC.
    match broadcast {
        4 => Some(SleepEvent::Suspending),
        7 | 18 => Some(SleepEvent::Resumed),
        _ => None,
    }
}

#[cfg(windows)]
static ON_SLEEP: std::sync::OnceLock<Box<dyn Fn(SleepEvent) + Send + Sync>> =
    std::sync::OnceLock::new();

#[cfg(windows)]
unsafe extern "system" fn on_power_broadcast(
    _context: *const core::ffi::c_void,
    broadcast: u32,
    _setting: *const core::ffi::c_void,
) -> u32 {
    if let (Some(event), Some(handler)) = (sleep_event(broadcast), ON_SLEEP.get()) {
        log::info!("system power: {event:?}");
        handler(event);
    }
    0
}

/// Calls `handler` on every sleep and wake of the system, for the life of the app. Once only.
#[cfg(windows)]
pub(crate) fn watch_sleep(handler: impl Fn(SleepEvent) + Send + Sync + 'static) {
    use windows::Win32::Foundation::{ERROR_SUCCESS, HANDLE};
    use windows::Win32::System::Power::{
        PowerRegisterSuspendResumeNotification, DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS,
    };
    use windows::Win32::UI::WindowsAndMessaging::DEVICE_NOTIFY_CALLBACK;

    if ON_SLEEP.set(Box::new(handler)).is_err() {
        log::warn!("sleep notifications already registered");
        return;
    }
    // Leaked on purpose: the registration lives as long as the process.
    let params: &'static mut DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS =
        Box::leak(Box::new(DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS {
            Callback: Some(on_power_broadcast),
            Context: std::ptr::null_mut(),
        }));
    let mut registration = std::ptr::null_mut();
    // SAFETY: `params` is valid for the whole process and the callback matches the signature.
    let result = unsafe {
        PowerRegisterSuspendResumeNotification(
            DEVICE_NOTIFY_CALLBACK,
            HANDLE(std::ptr::from_mut(params).cast()),
            &mut registration,
        )
    };
    if result == ERROR_SUCCESS {
        log::info!("listening for system sleep");
    } else {
        log::warn!("system sleep notifications unavailable: {result:?}");
    }
}

/// Linux (logind's PrepareForSleep) comes with phase 3.
#[cfg(not(windows))]
pub(crate) fn watch_sleep(_handler: impl Fn(SleepEvent) + Send + Sync + 'static) {}

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

    #[test]
    fn power_broadcasts() {
        assert_eq!(sleep_event(4), Some(SleepEvent::Suspending));
        assert_eq!(sleep_event(7), Some(SleepEvent::Resumed));
        assert_eq!(sleep_event(18), Some(SleepEvent::Resumed));
        // PBT_POWERSETTINGCHANGE and PBT_APMPOWERSTATUSCHANGE say nothing about sleep.
        assert_eq!(sleep_event(0x8013), None);
        assert_eq!(sleep_event(10), None);
    }
}
