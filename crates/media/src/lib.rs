//! Importação de mídia: um arquivo local de áudio ou vídeo vira o WAV que uma sessão do modo
//! "importação" entrega ao pipeline da reunião (design doc §3.4 item 5).
//!
//! Usa o `ffmpeg` e o `ffprobe` do PATH; nada é baixado (decisão D3 do pitch da fase 2, opção c).
//! A saída é WAV PCM s16le, mono, 48 kHz, o formato por canal do gravador de reunião e a entrada
//! do Opus retido (ADR-0005). `crates/meeting` chama [`import`] numa thread, com o destino dentro
//! da pasta da sessão, e repassa cada [`Progress`] como evento. Sem `cfg` de plataforma (ADR-0007).

mod ffmpeg;
mod tools;

use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub use ffmpeg::{CHANNELS, SAMPLE_RATE_HZ};
pub use tools::Tools;

/// Maior duração importável: 12 h a 96 000 bytes/s cabem nos 4 GiB de um WAV.
pub const MAX_DURATION: Duration = Duration::from_secs(12 * 3600);

/// Intervalo máximo entre duas conferências do cancelamento enquanto o ffmpeg roda.
const CANCEL_POLL: Duration = Duration::from_millis(50);

/// Erros da importação. Nenhum caminho previsível termina em panic.
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("`{tool}` não está no PATH; instale o ffmpeg (que traz o ffprobe) e deixe-o no PATH")]
    ToolNotFound { tool: &'static str },
    #[error("arquivo não encontrado: {}", .0.display())]
    InputNotFound(PathBuf),
    #[error("a saída já existe e não será sobrescrita: {}", .0.display())]
    OutputExists(PathBuf),
    #[error("o ffprobe não reconhece {}: {detail}", .path.display())]
    Unreadable { path: PathBuf, detail: String },
    #[error("{} não tem trilha de áudio", .0.display())]
    NoAudioTrack(PathBuf),
    #[error(
        "a origem dura {:.0} s e o limite da importação é {:.0} s",
        .duration.as_secs_f64(),
        .max.as_secs_f64()
    )]
    TooLong { duration: Duration, max: Duration },
    #[error("o ffmpeg falhou (status {code:?}): {detail}")]
    Ffmpeg { code: Option<i32>, detail: String },
    #[error("importação cancelada")]
    Cancelled,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Pedido de cancelamento compartilhado entre quem chama e a importação em andamento.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Avanço da conversão: tempo de áudio já escrito e a duração sondada, se o contêiner a tem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub processed: Duration,
    pub total: Option<Duration>,
}

/// O WAV importado, com o que o cabeçalho escrito diz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedAudio {
    pub path: PathBuf,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration: Duration,
}

/// Recusa uma origem que não é arquivo e uma saída que já existe. [`import`] chama primeiro;
/// exposta para quem quer validar antes de procurar o ffmpeg.
pub fn check_paths(input: &Path, out: &Path) -> Result<(), MediaError> {
    if !input.is_file() {
        return Err(MediaError::InputNotFound(input.to_path_buf()));
    }
    if out.symlink_metadata().is_ok() {
        return Err(MediaError::OutputExists(out.to_path_buf()));
    }
    Ok(())
}

/// Converte `input` num WAV mono 48 kHz em `out`.
///
/// Escreve em `<out>.part` e só renomeia para `out` com o ffmpeg em status 0; em erro ou
/// cancelamento, nenhum dos dois fica no disco. `on_progress` recebe um evento em 0 assim que o
/// ffmpeg inicia e depois um por bloco do `-progress` dele (a cada 0,5 s).
pub fn import(
    tools: &Tools,
    input: &Path,
    out: &Path,
    cancel: &CancelToken,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<ImportedAudio, MediaError> {
    check_paths(input, out)?;
    if cancel.is_cancelled() {
        return Err(MediaError::Cancelled);
    }
    let source = std::path::absolute(input)?;
    let target = std::path::absolute(out)?;

    let probe = probe(tools, input, &source)?;
    if !probe.has_audio {
        return Err(MediaError::NoAudioTrack(input.to_path_buf()));
    }
    if let Some(duration) = probe.duration.filter(|d| *d > MAX_DURATION) {
        return Err(MediaError::TooLong {
            duration,
            max: MAX_DURATION,
        });
    }
    if cancel.is_cancelled() {
        return Err(MediaError::Cancelled);
    }

    let mut part = target.clone().into_os_string();
    part.push(".part");
    let part = PathBuf::from(part);
    let converted = convert(tools, &source, &part, probe.duration, cancel, on_progress)
        .and_then(|()| finish(&part, out));
    if converted.is_err() {
        let _ = fs::remove_file(&part);
    }
    converted
}

fn probe(tools: &Tools, input: &Path, source: &Path) -> Result<ffmpeg::Probe, MediaError> {
    let args = ffmpeg::ffprobe_args(source);
    log::debug!("ffprobe {args:?}");
    let output = Command::new(tools.ffprobe())
        .args(&args)
        .stdin(Stdio::null())
        .output()?;
    if !output.status.success() {
        return Err(MediaError::Unreadable {
            path: input.to_path_buf(),
            detail: ffmpeg::stderr_tail(
                &String::from_utf8_lossy(&output.stderr),
                ffmpeg::STDERR_TAIL_BYTES,
            ),
        });
    }
    Ok(ffmpeg::parse_probe(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn convert(
    tools: &Tools,
    source: &Path,
    part: &Path,
    total: Option<Duration>,
    cancel: &CancelToken,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<(), MediaError> {
    let args = ffmpeg::ffmpeg_args(source, part);
    log::debug!("ffmpeg {args:?}");
    let mut child = Command::new(tools.ffmpeg())
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let (sender, progress) = mpsc::channel();
    if let Some(stdout) = child.stdout.take() {
        thread::spawn(move || {
            let mut parser = ffmpeg::ProgressParser::default();
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if let Some(processed) = parser.feed(&line)
                    && sender.send(processed).is_err()
                {
                    break;
                }
            }
        });
    }
    let stderr = child.stderr.take().map(|mut pipe| {
        thread::spawn(move || {
            let mut text = String::new();
            let _ = pipe.read_to_string(&mut text);
            text
        })
    });

    on_progress(Progress {
        processed: Duration::ZERO,
        total,
    });
    let mut last = Duration::ZERO;
    loop {
        if cancel.is_cancelled() {
            break;
        }
        match progress.recv_timeout(CANCEL_POLL) {
            Ok(processed) => {
                last = last.max(processed);
                on_progress(Progress {
                    processed: last,
                    total,
                });
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    if cancel.is_cancelled() {
        let _ = child.kill();
        let _ = child.wait();
        return Err(MediaError::Cancelled);
    }

    let status = child.wait()?;
    if !status.success() {
        let text = stderr
            .map(|handle| handle.join().unwrap_or_default())
            .unwrap_or_default();
        return Err(MediaError::Ffmpeg {
            code: status.code(),
            detail: ffmpeg::stderr_tail(&text, ffmpeg::STDERR_TAIL_BYTES),
        });
    }
    Ok(())
}

/// Renomeia o `.part` para `out` e lê o cabeçalho do WAV escrito.
fn finish(part: &Path, out: &Path) -> Result<ImportedAudio, MediaError> {
    if out.symlink_metadata().is_ok() {
        return Err(MediaError::OutputExists(out.to_path_buf()));
    }
    fs::rename(part, out)?;
    let reader = hound::WavReader::open(out).map_err(std::io::Error::other)?;
    let spec = reader.spec();
    let frames = u64::from(reader.duration());
    let nanos = (frames * 1_000_000_000)
        .checked_div(u64::from(spec.sample_rate))
        .ok_or_else(|| std::io::Error::other("WAV com taxa de amostragem 0"))?;
    Ok(ImportedAudio {
        path: out.to_path_buf(),
        sample_rate: spec.sample_rate,
        channels: spec.channels,
        duration: Duration::from_nanos(nanos),
    })
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::Path;
    use std::time::Duration;

    use crate::ffmpeg::{
        ffmpeg_args, ffprobe_args, parse_probe, stderr_tail, ProgressParser, STDERR_TAIL_BYTES,
    };

    fn strings(args: &[OsString]) -> Vec<String> {
        args.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    /// `flag` aparece seguido de `value`.
    fn has_pair(args: &[String], flag: &str, value: &str) -> bool {
        args.windows(2).any(|w| w[0] == flag && w[1] == value)
    }

    #[test]
    fn ffmpeg_args_match_the_door() {
        let input = Path::new("/media/aula com espaço.mp4");
        let part = Path::new("/out/aula.fala.wav.part");
        let args = strings(&ffmpeg_args(input, part));
        assert!(has_pair(&args, "-protocol_whitelist", "file"));
        assert!(has_pair(&args, "-i", "file:/media/aula com espaço.mp4"));
        assert!(has_pair(&args, "-map", "0:a:0"));
        assert!(has_pair(&args, "-ac", "1"));
        assert!(has_pair(&args, "-ar", "48000"));
        assert!(has_pair(&args, "-c:a", "pcm_s16le"));
        assert!(has_pair(&args, "-f", "wav"));
        assert!(has_pair(&args, "-progress", "pipe:1"));
        assert!(has_pair(&args, "-stats_period", "0.5"));
        assert_eq!(
            args.last().map(String::as_str),
            Some("file:/out/aula.fala.wav.part")
        );
        // A whitelist é opção de entrada: precisa vir antes do `-i`.
        let whitelist = args.iter().position(|a| a == "-protocol_whitelist");
        let input_flag = args.iter().position(|a| a == "-i");
        assert!(whitelist < input_flag);
    }

    #[test]
    fn ffprobe_args_match_the_door() {
        let args = strings(&ffprobe_args(Path::new("/media/aula.mp4")));
        assert!(has_pair(&args, "-protocol_whitelist", "file"));
        assert!(has_pair(
            &args,
            "-show_entries",
            "stream=codec_type:format=duration"
        ));
        assert!(has_pair(&args, "-of", "default=nw=1"));
        assert_eq!(
            args.last().map(String::as_str),
            Some("file:/media/aula.mp4")
        );
    }

    #[test]
    fn progress_block_parses_out_time_us() {
        let mut parser = ProgressParser::default();
        assert_eq!(parser.feed("out_time_us=N/A"), None);
        assert_eq!(parser.feed("progress=continue"), Some(Duration::ZERO));
        assert_eq!(parser.feed("out_time_us=1500000"), None);
        assert_eq!(parser.feed("total_size=96078"), None);
        assert_eq!(
            parser.feed("progress=continue"),
            Some(Duration::from_millis(1500))
        );
        assert_eq!(parser.feed("out_time_us=N/A"), None);
        assert_eq!(
            parser.feed("progress=end"),
            Some(Duration::from_millis(1500))
        );
    }

    #[test]
    fn probe_reads_audio_and_duration() {
        let probe = parse_probe("codec_type=video\ncodec_type=audio\nduration=1.044898\n");
        assert!(probe.has_audio);
        assert_eq!(probe.duration, Some(Duration::from_nanos(1_044_898_000)));
        let probe = parse_probe("codec_type=video\nduration=N/A\n");
        assert!(!probe.has_audio);
        assert_eq!(probe.duration, None);
    }

    #[test]
    fn stderr_tail_keeps_last_2000_bytes() {
        assert_eq!(STDERR_TAIL_BYTES, 2_000);
        let long = format!("{}fim", "x".repeat(5_000));
        let tail = stderr_tail(&long, STDERR_TAIL_BYTES);
        assert_eq!(tail.len(), 2_000);
        assert!(tail.ends_with("fim"));
        // Não corta no meio de um caractere de 2 bytes.
        let accents = "é".repeat(1_500);
        let tail = stderr_tail(&accents, STDERR_TAIL_BYTES);
        assert!(tail.len() <= 2_000);
        assert!(tail.chars().all(|c| c == 'é'));
        assert_eq!(stderr_tail("  erro\n", STDERR_TAIL_BYTES), "erro");
    }
}
