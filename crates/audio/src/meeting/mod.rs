//! Gravação de reunião: mic e áudio do sistema num WAV estéreo 48 kHz à prova de crash.
//!
//! `MeetingRecorder` não abre dispositivo nenhum: quem chama empurra os frames de cada canal e
//! marca o relógio. Assim o mesmo `Mic` pode alimentar o ditado e o gravador (decisão 3 do roadmap
//! de 2026-10-02). `SystemAudio` abre o monitor do sink (ALSA/PipeWire, ADR-0013) ou o loopback
//! (WASAPI), separados pelo nome do host do `cpal`, sem `cfg` de sistema (ADR-0007).

mod recorder;
mod system;
mod wav;

pub use recorder::{ChannelStats, MeetingRecorder, LAG_FRAMES, MAX_BACKLOG_FRAMES};
pub use system::SystemAudio;
pub(crate) use system::ENV_OPEN;
pub use wav::{MeetingWav, FLUSH_EVERY, MEETING_RATE};
