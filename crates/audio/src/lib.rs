//! Captura de áudio, VAD e resample.
//!
//! Trait `Source`: microfone via `cpal`; loopback do sistema no Windows (fase 2); PipeWire no Linux (fase 3).
//! Silero VAD, resample com `rubato`, pré-buffer contínuo de 300 ms e gravador WAV/Opus de reunião.
//! O tipo de áudio de ditado não implementa serialização para clientes HTTP (invariante 2).
//! Nasce vazio no dia 1; `apps/desktop/src/audio_toolkit` migra na fase 1.
