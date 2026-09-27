//! Reconhecimento de fala plugável.
//!
//! Trait `Transcriber { transcribe(segment), transcribe_file(path) }`.
//! Backend padrão: Parakeet-TDT-0.6B-v3 int8 local (ADR-0003); backends de nuvem só para reunião (ADR-0005).
//! Trocar de backend não deve exigir mudança fora deste crate.
//! Nasce vazio no dia 1; o uso de `transcribe-rs` do desktop migra na fase 1.
