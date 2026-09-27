//! Atalho global com press e release, para push-to-talk.
//!
//! Trait `Hotkey { press, release }` com um adaptador por sistema operacional.
//! Windows: hook `WH_KEYBOARD_LL` via `rdev`. Linux (fase 3): portal GlobalShortcuts via `ashpd`.
//! `#[cfg(windows)]` e afins ficam aqui dentro, nunca fora dos crates de plataforma (ADR-0007).
//! Nasce vazio no dia 1; a lógica herdada (`apps/desktop/src/shortcut`) migra na fase 1.
