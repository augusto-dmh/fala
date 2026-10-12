//! Importação de mídia: um arquivo local de áudio ou vídeo vira o WAV que uma sessão do modo
//! "importação" entrega ao pipeline da reunião (design doc §3.4 item 5); uma URL vira esse
//! arquivo antes, pelo [`download`].
//!
//! Usa o `ffmpeg`, o `ffprobe` e o `yt-dlp` do PATH; nenhuma ferramenta é baixada (decisão D3
//! do pitch da fase 2, opção c).
//! A saída é WAV PCM s16le, mono, 48 kHz, o formato por canal do gravador de reunião e a entrada
//! do Opus retido (ADR-0005). `crates/meeting` chama [`import`] numa thread, com o destino dentro
//! da pasta da sessão, e repassa cada [`Progress`] como evento. Sem `cfg` de plataforma (ADR-0007).

mod ffmpeg;
mod tools;
mod ytdlp;

use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub use ffmpeg::{CHANNELS, SAMPLE_RATE_HZ};
pub use tools::{ProcessHooks, Tools};
pub use ytdlp::{check_url, DownloadProgress};

/// Maior duração importável: 12 h a 96 000 bytes/s cabem nos 4 GiB de um WAV.
pub const MAX_DURATION: Duration = Duration::from_secs(12 * 3600);

/// Intervalo máximo entre duas conferências do cancelamento enquanto o ffmpeg roda.
const CANCEL_POLL: Duration = Duration::from_millis(50);

/// Erros da importação. Nenhum caminho previsível termina em panic.
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("`{tool}` não está no PATH; instale-o e deixe-o no PATH (o ffmpeg traz o ffprobe)")]
    ToolNotFound { tool: &'static str },
    #[error("só URLs http:// ou https:// são aceitas: {0}")]
    InvalidUrl(String),
    #[error("o yt-dlp não baixou a URL (status {code:?}): {detail}")]
    Download { code: Option<i32>, detail: String },
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

/// O arquivo que o [`download`] deixou no disco e o título que a página tinha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Downloaded {
    pub path: PathBuf,
    pub title: String,
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

/// Baixa o áudio de `url` para dentro de `dir` com o `yt-dlp`, para depois passar ao [`import`].
///
/// Recusa o que não é `http(s)://` antes de iniciar processo. O cancelamento para o yt-dlp pelo
/// `stop` dos [`ProcessHooks`]; o que ele deixou em `dir` fica para quem chama apagar.
pub fn download(
    tools: &Tools,
    url: &str,
    dir: &Path,
    cancel: &CancelToken,
    on_progress: &mut dyn FnMut(DownloadProgress),
) -> Result<Downloaded, MediaError> {
    check_url(url)?;
    let yt_dlp = tools.yt_dlp()?;
    if cancel.is_cancelled() {
        return Err(MediaError::Cancelled);
    }
    fs::create_dir_all(dir)?;
    let args = ytdlp::ytdlp_args(url, &std::path::absolute(dir)?);
    log::debug!("yt-dlp {args:?}");
    let mut command = Command::new(yt_dlp);
    command.args(&args);
    let mut output = ytdlp::YtDlpOutput::default();
    let (status, stderr) = supervise(tools, command, cancel, &mut |line| {
        if let Some(progress) = output.feed(line) {
            on_progress(progress);
        }
    })?;
    let failed = |detail: String| MediaError::Download {
        code: status.code(),
        detail,
    };
    if !status.success() {
        return Err(failed(stderr));
    }
    let path = output
        .filepath
        .filter(|path| path.is_file())
        .ok_or_else(|| failed("o yt-dlp terminou sem deixar um arquivo".to_owned()))?;
    Ok(Downloaded {
        path,
        title: output.title.unwrap_or_default(),
    })
}

fn probe(tools: &Tools, input: &Path, source: &Path) -> Result<ffmpeg::Probe, MediaError> {
    let args = ffmpeg::ffprobe_args(source);
    log::debug!("ffprobe {args:?}");
    let mut command = Command::new(tools.ffprobe());
    (tools.hooks().prepare)(&mut command);
    let output = command.args(&args).stdin(Stdio::null()).output()?;
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
    let mut command = Command::new(tools.ffmpeg());
    command.args(&args);
    on_progress(Progress {
        processed: Duration::ZERO,
        total,
    });
    let mut parser = ffmpeg::ProgressParser::default();
    let mut last = Duration::ZERO;
    let (status, detail) = supervise(tools, command, cancel, &mut |line| {
        if let Some(processed) = parser.feed(line) {
            last = last.max(processed);
            on_progress(Progress {
                processed: last,
                total,
            });
        }
    })?;
    if !status.success() {
        return Err(MediaError::Ffmpeg {
            code: status.code(),
            detail,
        });
    }
    Ok(())
}

/// Roda `command` até o fim, passando cada linha do stdout a `on_line`, e devolve o status e o
/// final do stderr. Confere o cancelamento a cada [`CANCEL_POLL`]; cancelado, para o processo
/// pelo `stop` dos hooks e devolve [`MediaError::Cancelled`].
fn supervise(
    tools: &Tools,
    mut command: Command,
    cancel: &CancelToken,
    on_line: &mut dyn FnMut(&str),
) -> Result<(ExitStatus, String), MediaError> {
    (tools.hooks().prepare)(&mut command);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let (sender, lines) = mpsc::channel();
    if let Some(stdout) = child.stdout.take() {
        // Linhas em bytes, decodificadas com perda: um título em outra codificação não encerra a
        // leitura.
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = Vec::new();
            while reader.read_until(b'\n', &mut line).is_ok_and(|n| n > 0) {
                let text = String::from_utf8_lossy(&line).trim_end().to_owned();
                line.clear();
                if sender.send(text).is_err() {
                    break;
                }
            }
        });
    }
    let stderr = child.stderr.take().map(|mut pipe| {
        thread::spawn(move || {
            let mut text = Vec::new();
            let _ = pipe.read_to_end(&mut text);
            String::from_utf8_lossy(&text).into_owned()
        })
    });

    loop {
        if cancel.is_cancelled() {
            break;
        }
        match lines.recv_timeout(CANCEL_POLL) {
            Ok(line) => on_line(&line),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    if cancel.is_cancelled() {
        (tools.hooks().stop)(&mut child);
        let _ = child.wait();
        return Err(MediaError::Cancelled);
    }

    let status = child.wait()?;
    let text = stderr
        .map(|handle| handle.join().unwrap_or_default())
        .unwrap_or_default();
    Ok((
        status,
        ffmpeg::stderr_tail(&text, ffmpeg::STDERR_TAIL_BYTES),
    ))
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
    use crate::ytdlp::{check_url, ytdlp_args, DownloadProgress, YtDlpOutput};
    use crate::MediaError;

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

    #[test]
    fn only_http_and_https_urls_pass() {
        for ok in [
            "https://www.youtube.com/watch?v=jNQXAC9IVRw",
            "HTTP://example.com",
            "http://127.0.0.1:8080/aula.mp3",
        ] {
            assert!(check_url(ok).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "ftp://example.com/a.mp3",
            "file:///C:/aula.mp4",
            "javascript:alert(1)",
            "--exec calc",
            "https://",
            "https:///caminho",
            "https://exemplo.com/a b",
            "https://exemplo.com/\nx",
            "C:\\aula.mp4",
        ] {
            assert!(
                matches!(check_url(bad), Err(MediaError::InvalidUrl(ref u)) if *u == bad),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn ytdlp_args_keep_the_url_as_one_last_argument() {
        let url = "https://exemplo.com/v?a=1&b=2;rm -rf";
        let args = strings(&ytdlp_args(url, Path::new("/sessao/baixado")));
        let n = args.len();
        assert_eq!(args[n - 2], "--");
        assert_eq!(args[n - 1], url);
        assert!(args.contains(&"--no-playlist".to_owned()));
        // Uma URL de playlist ou canal baixa só o primeiro item.
        assert!(has_pair(&args, "--playlist-items", "1"));
        assert!(has_pair(&args, "--encoding", "utf-8"));
        assert!(has_pair(&args, "--format", "bestaudio/best"));
        assert!(has_pair(&args, "--paths", "/sessao/baixado"));
        assert!(has_pair(&args, "--output", "source.%(ext)s"));
        assert!(has_pair(
            &args,
            "--print",
            "after_move:filepath:%(filepath)s"
        ));
        // Nada de execução depois do download nem de lista de URLs vinda de arquivo.
        assert!(!args.iter().any(|a| a == "--exec" || a == "--batch-file"));
    }

    #[test]
    fn ytdlp_output_reads_title_path_and_progress() {
        let mut out = YtDlpOutput::default();
        assert_eq!(out.feed("title:Me at the zoo"), None);
        assert_eq!(
            out.feed("progress:1024:252182:NA"),
            Some(DownloadProgress {
                done_bytes: 1024,
                total_bytes: Some(252_182),
            })
        );
        assert_eq!(
            out.feed("progress:2048:NA:300000.5"),
            Some(DownloadProgress {
                done_bytes: 2048,
                total_bytes: Some(300_000),
            })
        );
        assert_eq!(
            out.feed("progress:4096:NA:NA"),
            Some(DownloadProgress {
                done_bytes: 4096,
                total_bytes: None,
            })
        );
        assert_eq!(out.feed("progress:NA:NA:NA"), None);
        assert_eq!(out.feed("filepath:C:/sessão/source.webm\r"), None);
        assert_eq!(out.title.as_deref(), Some("Me at the zoo"));
        assert_eq!(
            out.filepath.as_deref(),
            Some(Path::new("C:/sessão/source.webm"))
        );
    }
}
