//! Argumentos e saídas do ffprobe e do ffmpeg.
//!
//! Os argumentos vão sempre em vetor para `Command`, nunca por shell. A origem e o destino
//! entram como `file:<caminho absoluto>` e os dois processos só aceitam o protocolo `file`, então
//! nem um caminho com cara de URL nem uma playlist local abrem rede.

use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

/// Taxa de amostragem do WAV importado.
pub const SAMPLE_RATE_HZ: u32 = 48_000;
/// Canais do WAV importado.
pub const CHANNELS: u16 = 1;
/// Maior stderr guardado num erro do ffmpeg ou do ffprobe, em bytes.
pub(crate) const STDERR_TAIL_BYTES: usize = 2_000;

fn file_url(path: &Path) -> OsString {
    let mut url = OsString::from("file:");
    url.push(path.as_os_str());
    url
}

pub(crate) fn ffprobe_args(input: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = [
        "-v",
        "error",
        "-protocol_whitelist",
        "file",
        "-show_entries",
        "stream=codec_type:format=duration",
        "-of",
        "default=nw=1",
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    args.push(file_url(input));
    args
}

pub(crate) fn ffmpeg_args(input: &Path, part: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = [
        "-nostdin",
        "-hide_banner",
        "-loglevel",
        "error",
        "-protocol_whitelist",
        "file",
        "-i",
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    args.push(file_url(input));
    let rate = SAMPLE_RATE_HZ.to_string();
    let channels = CHANNELS.to_string();
    for arg in [
        "-map",
        "0:a:0",
        "-vn",
        "-sn",
        "-dn",
        "-ac",
        &channels,
        "-ar",
        &rate,
        "-c:a",
        "pcm_s16le",
        "-f",
        "wav",
        "-progress",
        "pipe:1",
        "-stats_period",
        "0.5",
        "-nostats",
        "-y",
    ] {
        args.push(arg.into());
    }
    args.push(file_url(part));
    args
}

/// O que o ffprobe disse da origem.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Probe {
    pub has_audio: bool,
    pub duration: Option<Duration>,
}

/// Lê a saída `default=nw=1` do ffprobe: linhas `codec_type=<tipo>` e `duration=<s>|N/A`.
pub(crate) fn parse_probe(stdout: &str) -> Probe {
    let mut probe = Probe {
        has_audio: false,
        duration: None,
    };
    for line in stdout.lines().map(str::trim) {
        if line == "codec_type=audio" {
            probe.has_audio = true;
        } else if let Some(value) = line.strip_prefix("duration=") {
            probe.duration = value
                .parse::<f64>()
                .ok()
                .and_then(|s| Duration::try_from_secs_f64(s).ok());
        }
    }
    probe
}

/// Junta as linhas do `-progress pipe:1`: cada bloco termina em `progress=continue|end` e traz
/// `out_time_us`, que vem `N/A` antes do primeiro frame.
#[derive(Debug, Default)]
pub(crate) struct ProgressParser {
    out_time: Option<Duration>,
}

impl ProgressParser {
    /// Devolve o tempo processado quando a linha fecha um bloco.
    pub fn feed(&mut self, line: &str) -> Option<Duration> {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("out_time_us=") {
            if let Ok(us) = value.parse::<u64>() {
                self.out_time = Some(Duration::from_micros(us));
            }
            None
        } else if line.starts_with("progress=") {
            Some(self.out_time.unwrap_or(Duration::ZERO))
        } else {
            None
        }
    }
}

/// O final de `stderr`, sem espaços nas pontas, com no máximo `max` bytes e cortado em
/// fronteira de caractere.
pub(crate) fn stderr_tail(stderr: &str, max: usize) -> String {
    let text = stderr.trim();
    let mut start = text.len().saturating_sub(max);
    while !text.is_char_boundary(start) {
        start += 1;
    }
    text[start..].trim_start().to_owned()
}
