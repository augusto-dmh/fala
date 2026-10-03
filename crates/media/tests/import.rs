//! `fala_media::import` pela API pública, com arquivos gerados pelo próprio ffmpeg no teste.
//!
//! Os testes `#[ignore]` precisam de `ffmpeg` e `ffprobe` no PATH (o job `rust` do CI não instala
//! o ffmpeg); rode com `cargo test -p fala-media -- --include-ignored`. Os demais usam ferramentas
//! falsas (arquivos vazios, não executáveis): se a importação as iniciasse, o erro seria `Io`.

// `allow-unwrap-in-tests` não cobre os helpers de um crate de teste de integração.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::ffi::OsStr;
use std::fs;
use std::io::ErrorKind;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use fala_media::{import, CancelToken, ImportedAudio, MediaError, Progress, Tools};

const NEEDS_FFMPEG: &str = "precisa de ffmpeg e ffprobe no PATH";
const TOLERANCE: Duration = Duration::from_millis(60);

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("media")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn part_of(out: &Path) -> PathBuf {
    let mut part = out.as_os_str().to_owned();
    part.push(".part");
    PathBuf::from(part)
}

fn real_tools() -> Tools {
    Tools::locate().expect(NEEDS_FFMPEG)
}

/// Diretório com `ffmpeg` e `ffprobe` vazios: `locate_in` os acha, mas iniciá-los falha.
fn fake_tools(dir: &Path) -> Tools {
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    for name in ["ffmpeg", "ffprobe"] {
        fs::write(
            bin.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)),
            b"",
        )
        .unwrap();
    }
    Tools::locate_in(bin.as_os_str()).unwrap()
}

/// Roda o ffmpeg do PATH para gerar um arquivo de teste.
fn generate(args: &[&str], out: &Path) -> PathBuf {
    let status = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"])
        .args(args)
        .arg(out)
        .status()
        .expect(NEEDS_FFMPEG);
    assert!(status.success(), "ffmpeg não gerou {}", out.display());
    out.to_path_buf()
}

const TONE_440: [&str; 4] = ["-f", "lavfi", "-i", "sine=frequency=440:duration=1"];

fn tone_mp3(dir: &Path) -> PathBuf {
    let mut args = TONE_440.to_vec();
    args.extend(["-c:a", "libmp3lame"]);
    generate(&args, &dir.join("tom.mp3"))
}

fn tone_mp4_with_video(dir: &Path) -> PathBuf {
    let mut args = vec!["-f", "lavfi", "-i", "testsrc=duration=1:size=64x64:rate=10"];
    args.extend(TONE_440);
    // Estéreo de verdade, os dois canais com o tom inteiro (`-ac 2` subiria o mono com -3 dB).
    args.extend([
        "-af",
        "pan=stereo|c0=c0|c1=c0",
        "-c:a",
        "aac",
        "-c:v",
        "mpeg4",
        "-shortest",
    ]);
    generate(&args, &dir.join("tom.mp4"))
}

fn tone_ogg(dir: &Path) -> PathBuf {
    let mut args = TONE_440.to_vec();
    args.extend(["-c:a", "libopus"]);
    generate(&args, &dir.join("tom.ogg"))
}

fn video_only(dir: &Path) -> PathBuf {
    generate(
        &[
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=1:size=64x64:rate=10",
            "-c:v",
            "mpeg4",
        ],
        &dir.join("video.mp4"),
    )
}

fn no_progress() -> impl FnMut(Progress) {
    |_| {}
}

struct Wav {
    spec: hound::WavSpec,
    samples: Vec<i16>,
}

impl Wav {
    fn read(path: &Path) -> Self {
        let mut reader = hound::WavReader::open(path).unwrap();
        let spec = reader.spec();
        let samples = reader.samples::<i16>().map(Result::unwrap).collect();
        Self { spec, samples }
    }

    fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.samples.len() as f64 / f64::from(self.spec.sample_rate))
    }

    fn peak(&self) -> f64 {
        let peak = self.samples.iter().map(|s| s.unsigned_abs()).max().unwrap();
        f64::from(peak) / f64::from(i16::MAX)
    }

    /// Frequência por cruzamentos de zero, ignorando 0,1 s em cada ponta.
    fn frequency(&self) -> f64 {
        let edge = self.spec.sample_rate as usize / 10;
        let middle = &self.samples[edge..self.samples.len() - edge];
        let crossings = middle
            .windows(2)
            .filter(|w| (w[0] < 0) != (w[1] < 0))
            .count();
        let seconds = middle.len() as f64 / f64::from(self.spec.sample_rate);
        crossings as f64 / 2.0 / seconds
    }
}

fn near(a: Duration, b: Duration) -> bool {
    a.abs_diff(b) <= TOLERANCE
}

/// C1: WAV PCM 16 bits, mono, 48 kHz, 1 s ± 60 ms, 440 ± 10 Hz, pico ≥ 0,1.
fn assert_mono_48k_tone(out: &Path, expected_hz: f64) {
    let wav = Wav::read(out);
    assert_eq!(wav.spec.channels, 1);
    assert_eq!(wav.spec.sample_rate, 48_000);
    assert_eq!(wav.spec.bits_per_sample, 16);
    assert_eq!(wav.spec.sample_format, hound::SampleFormat::Int);
    assert!(
        near(wav.duration(), Duration::from_secs(1)),
        "duração {:?}",
        wav.duration()
    );
    assert!(wav.peak() >= 0.1, "pico {}", wav.peak());
    let hz = wav.frequency();
    assert!((hz - expected_hz).abs() <= 10.0, "frequência {hz}");
}

fn import_ok(input: &Path, out: &Path) -> ImportedAudio {
    import(
        &real_tools(),
        input,
        out,
        &CancelToken::new(),
        &mut no_progress(),
    )
    .unwrap()
}

#[test]
#[ignore = "precisa de ffmpeg e ffprobe no PATH"]
fn mp3_becomes_mono_48k_wav() {
    let dir = scratch("mp3");
    let out = dir.join("tom.fala.wav");
    import_ok(&tone_mp3(&dir), &out);
    assert_mono_48k_tone(&out, 440.0);
}

#[test]
#[ignore = "precisa de ffmpeg e ffprobe no PATH"]
fn mp4_with_video_becomes_mono_48k_wav() {
    let dir = scratch("mp4");
    let out = dir.join("tom.fala.wav");
    import_ok(&tone_mp4_with_video(&dir), &out);
    assert_mono_48k_tone(&out, 440.0);
}

#[test]
#[ignore = "precisa de ffmpeg e ffprobe no PATH"]
fn ogg_becomes_mono_48k_wav() {
    let dir = scratch("ogg");
    let out = dir.join("tom.fala.wav");
    import_ok(&tone_ogg(&dir), &out);
    assert_mono_48k_tone(&out, 440.0);
}

#[test]
#[ignore = "precisa de ffmpeg e ffprobe no PATH"]
fn imported_audio_matches_written_header() {
    let dir = scratch("header");
    let out = dir.join("tom.fala.wav");
    let audio = import_ok(&tone_mp3(&dir), &out);
    let mut reader = hound::WavReader::open(&out).unwrap();
    let frames = reader.duration();
    let read = reader.samples::<i16>().count();
    assert_eq!(read, frames as usize);
    assert_eq!(audio.path, out);
    assert_eq!(audio.sample_rate, 48_000);
    assert_eq!(audio.channels, 1);
    assert_eq!(
        audio.duration,
        Duration::from_nanos(u64::from(frames) * 1_000_000_000 / 48_000)
    );
}

#[test]
#[ignore = "precisa de ffmpeg e ffprobe no PATH"]
fn first_audio_track_wins() {
    let dir = scratch("tracks");
    let input = generate(
        &[
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=880:duration=1",
            "-map",
            "0:a",
            "-map",
            "1:a",
            "-c:a",
            "aac",
        ],
        &dir.join("duas.mp4"),
    );
    let out = dir.join("duas.fala.wav");
    import_ok(&input, &out);
    assert_mono_48k_tone(&out, 440.0);
}

#[test]
fn existing_output_is_left_untouched() {
    let dir = scratch("exists");
    let input = dir.join("entrada.mp3");
    fs::write(&input, b"qualquer coisa").unwrap();
    let out = dir.join("saida.wav");
    fs::write(&out, b"conteudo anterior").unwrap();
    let result = import(
        &fake_tools(&dir),
        &input,
        &out,
        &CancelToken::new(),
        &mut no_progress(),
    );
    match result {
        Err(MediaError::OutputExists(path)) => assert_eq!(path, out),
        other => panic!("esperava OutputExists, veio {other:?}"),
    }
    assert_eq!(fs::read(&out).unwrap(), b"conteudo anterior");
}

#[test]
#[ignore = "precisa de ffmpeg e ffprobe no PATH"]
fn output_appears_only_after_success() {
    let dir = scratch("part");
    let input = tone_mp3(&dir);
    let out = dir.join("tom.fala.wav");
    let mut out_seen_at_start = None;
    let result = import(
        &real_tools(),
        &input,
        &out,
        &CancelToken::new(),
        &mut |_| {
            out_seen_at_start.get_or_insert(out.exists());
        },
    );
    result.unwrap();
    assert_eq!(out_seen_at_start, Some(false));
    assert!(out.exists());
    assert!(!part_of(&out).exists());
}

#[test]
fn empty_path_reports_ffmpeg() {
    let error = Tools::locate_in(OsStr::new("")).unwrap_err();
    assert!(matches!(error, MediaError::ToolNotFound { tool: "ffmpeg" }));
    let message = error.to_string();
    assert!(message.contains("ffmpeg"), "{message}");
    assert!(message.contains("PATH"), "{message}");
}

#[test]
fn path_without_ffprobe_reports_ffprobe() {
    let dir = scratch("only-ffmpeg");
    fs::write(
        dir.join(format!("ffmpeg{}", std::env::consts::EXE_SUFFIX)),
        b"",
    )
    .unwrap();
    let error = Tools::locate_in(dir.as_os_str()).unwrap_err();
    assert!(
        matches!(error, MediaError::ToolNotFound { tool: "ffprobe" }),
        "{error:?}"
    );
}

#[test]
fn missing_input_is_input_not_found() {
    let dir = scratch("missing");
    let tools = fake_tools(&dir);
    let out = dir.join("saida.wav");
    for input in [dir.join("nao-existe.mp3"), dir.clone()] {
        let result = import(
            &tools,
            &input,
            &out,
            &CancelToken::new(),
            &mut no_progress(),
        );
        match result {
            Err(MediaError::InputNotFound(path)) => assert_eq!(path, input),
            other => panic!(
                "esperava InputNotFound para {}, veio {other:?}",
                input.display()
            ),
        }
        assert!(!out.exists());
        assert!(!part_of(&out).exists());
    }
}

#[test]
#[ignore = "precisa de ffmpeg e ffprobe no PATH"]
fn video_only_is_no_audio_track() {
    let dir = scratch("video-only");
    let input = video_only(&dir);
    let out = dir.join("video.fala.wav");
    let result = import(
        &real_tools(),
        &input,
        &out,
        &CancelToken::new(),
        &mut no_progress(),
    );
    match result {
        Err(MediaError::NoAudioTrack(path)) => assert_eq!(path, input),
        other => panic!("esperava NoAudioTrack, veio {other:?}"),
    }
    assert!(!out.exists());
    assert!(!part_of(&out).exists());
}

#[test]
#[ignore = "precisa de ffmpeg e ffprobe no PATH"]
fn text_file_is_unreadable() {
    let dir = scratch("text");
    let input = dir.join("notas.txt");
    fs::write(&input, "isto não é mídia\n").unwrap();
    let out = dir.join("notas.fala.wav");
    let result = import(
        &real_tools(),
        &input,
        &out,
        &CancelToken::new(),
        &mut no_progress(),
    );
    match result {
        Err(MediaError::Unreadable { path, detail }) => {
            assert_eq!(path, input);
            assert!(!detail.is_empty());
        }
        other => panic!("esperava Unreadable, veio {other:?}"),
    }
    assert!(!out.exists());
    assert!(!part_of(&out).exists());
}

#[test]
#[ignore = "precisa de ffmpeg e ffprobe no PATH"]
fn too_long_is_rejected_before_ffmpeg() {
    let dir = scratch("too-long");
    tone_mp3(&dir);
    // O demuxer concat do ffmpeg anuncia a duração declarada, sem 50 000 s de áudio no disco.
    let input = dir.join("longo.ffconcat");
    fs::write(
        &input,
        "ffconcat version 1.0\nfile tom.mp3\nduration 50000\n",
    )
    .unwrap();
    let out = dir.join("longo.fala.wav");
    let mut events = 0;
    let result = import(
        &real_tools(),
        &input,
        &out,
        &CancelToken::new(),
        &mut |_| events += 1,
    );
    match result {
        Err(MediaError::TooLong { duration, max }) => {
            assert_eq!(duration, Duration::from_secs(50_000));
            assert_eq!(max, Duration::from_secs(43_200));
        }
        other => panic!("esperava TooLong, veio {other:?}"),
    }
    assert_eq!(events, 0);
    assert!(!out.exists());
    assert!(!part_of(&out).exists());
}

#[test]
#[ignore = "precisa de ffmpeg e ffprobe no PATH"]
fn ffmpeg_failure_is_typed_with_stderr() {
    let dir = scratch("ffmpeg-fails");
    let input = tone_mp3(&dir);
    let out = dir.join("nao-existe").join("tom.fala.wav");
    let result = import(
        &real_tools(),
        &input,
        &out,
        &CancelToken::new(),
        &mut no_progress(),
    );
    match result {
        Err(MediaError::Ffmpeg { code, detail }) => {
            assert_ne!(code, Some(0));
            assert!(!detail.is_empty());
            assert!(detail.len() <= 2_000, "{} bytes", detail.len());
        }
        other => panic!("esperava Ffmpeg, veio {other:?}"),
    }
    assert!(!out.exists());
    assert!(!part_of(&out).exists());
}

#[test]
#[ignore = "precisa de ffmpeg e ffprobe no PATH"]
fn first_progress_is_zero_with_probed_total() {
    let dir = scratch("progress-first");
    let input = tone_mp3(&dir);
    let mut events = Vec::new();
    import(
        &real_tools(),
        &input,
        &dir.join("tom.fala.wav"),
        &CancelToken::new(),
        &mut |p| events.push(p),
    )
    .unwrap();
    let first = events[0];
    assert_eq!(first.processed, Duration::ZERO);
    let total = first.total.expect("o mp3 tem duração no contêiner");
    assert!(near(total, Duration::from_secs(1)), "total {total:?}");
}

#[test]
#[ignore = "precisa de ffmpeg e ffprobe no PATH"]
fn progress_is_monotonic_and_ends_at_duration() {
    let dir = scratch("progress-end");
    let input = tone_mp4_with_video(&dir);
    let mut events = Vec::new();
    let audio = import(
        &real_tools(),
        &input,
        &dir.join("tom.fala.wav"),
        &CancelToken::new(),
        &mut |p| events.push(p),
    )
    .unwrap();
    assert!(events.len() >= 2, "{events:?}");
    assert!(
        events.windows(2).all(|w| w[0].processed <= w[1].processed),
        "{events:?}"
    );
    let last = events.last().unwrap().processed;
    assert!(
        near(last, audio.duration),
        "último {last:?}, WAV {:?}",
        audio.duration
    );
}

#[test]
#[ignore = "precisa de ffmpeg e ffprobe no PATH"]
fn cancel_while_running_kills_and_cleans() {
    let dir = scratch("cancel");
    let input = tone_mp3(&dir);
    let out = dir.join("tom.fala.wav");
    let token = CancelToken::new();
    let canceller = token.clone();
    let mut cancelled_at = None;
    let result = import(&real_tools(), &input, &out, &token, &mut |_| {
        if cancelled_at.is_none() {
            cancelled_at = Some(Instant::now());
            canceller.cancel();
        }
    });
    let elapsed = cancelled_at.expect("nenhum evento de progresso").elapsed();
    assert!(
        matches!(result, Err(MediaError::Cancelled)),
        "esperava Cancelled, veio {result:?}"
    );
    assert!(elapsed <= Duration::from_secs(1), "{elapsed:?}");
    assert!(!out.exists());
    assert!(!part_of(&out).exists());
}

#[test]
fn pre_cancelled_token_spawns_nothing() {
    let dir = scratch("pre-cancelled");
    let input = dir.join("entrada.mp3");
    fs::write(&input, b"qualquer coisa").unwrap();
    let out = dir.join("saida.wav");
    let token = CancelToken::new();
    token.cancel();
    let result = import(&fake_tools(&dir), &input, &out, &token, &mut no_progress());
    assert!(
        matches!(result, Err(MediaError::Cancelled)),
        "esperava Cancelled, veio {result:?}"
    );
    assert!(!out.exists());
    assert!(!part_of(&out).exists());
}

#[test]
#[ignore = "precisa de ffmpeg e ffprobe no PATH"]
fn local_playlist_opens_no_connection() {
    let dir = scratch("playlist");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let input = dir.join("lista.m3u8");
    fs::write(
        &input,
        format!(
            "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXTINF:1.0,\nhttp://127.0.0.1:{port}/a.ts\n#EXT-X-ENDLIST\n"
        ),
    )
    .unwrap();
    let result = import(
        &real_tools(),
        &input,
        &dir.join("lista.fala.wav"),
        &CancelToken::new(),
        &mut no_progress(),
    );
    assert!(
        matches!(
            result,
            Err(MediaError::Unreadable { .. }
                | MediaError::NoAudioTrack(_)
                | MediaError::Ffmpeg { .. })
        ),
        "{result:?}"
    );
    let pending = listener.accept();
    assert!(
        matches!(&pending, Err(e) if e.kind() == ErrorKind::WouldBlock),
        "conexão recebida: {pending:?}"
    );
}
