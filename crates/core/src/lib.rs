//! Tipos compartilhados, configuração e eventos do Fala.
//!
//! Define `Event`, `Settings`, `Utterance`, `Session`, o dicionário pessoal e os erros comuns.
//! Todo crate de `crates/` depende deste; a UI conversa com o core só por `Event`.
//! Sem dependência de plataforma nem de `tauri` (invariante de arquitetura).
//! Nasce vazio no dia 1; os tipos migram do `apps/desktop` na fase 1.
