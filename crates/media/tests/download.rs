//! `fala_media::download` pela API pública, contra um servidor HTTP local do próprio teste.
//!
//! Os testes `#[ignore]` precisam de `yt-dlp`, `ffmpeg` e `ffprobe` no PATH (o job `rust` do CI
//! não os instala); rode com `cargo test -p fala-media -- --include-ignored`. Os demais usam
//! ferramentas falsas (arquivos vazios): se o download as iniciasse, o erro seria `Io`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use fala_media::{download, import, CancelToken, MediaError, ProcessHooks, Tools};

const NEEDS_TOOLS: &str = "precisa de yt-dlp, ffmpeg e ffprobe no PATH";

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("download")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// `ffmpeg` e `ffprobe` vazios e, se pedido, um `yt-dlp` vazio.
fn fake_tools(dir: &Path, with_yt_dlp: bool) -> Tools {
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let mut names = vec!["ffmpeg", "ffprobe"];
    if with_yt_dlp {
        names.push("yt-dlp");
    }
    for name in names {
        fs::write(
            bin.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)),
            b"",
        )
        .unwrap();
    }
    Tools::locate_in(bin.as_os_str()).unwrap()
}

/// Mata a árvore inteira: o `yt-dlp.exe` do Windows é um bootloader que inicia um segundo
/// processo. Sem `taskkill` (Linux), mata só o processo, que lá é o único.
fn stop_tree(child: &mut Child) {
    let killed = Command::new("taskkill")
        .args(["/T", "/F", "/PID", &child.id().to_string()])
        .output()
        .is_ok_and(|out| out.status.success());
    if !killed {
        let _ = child.kill();
    }
}

fn real_tools() -> Tools {
    Tools::locate()
        .expect(NEEDS_TOOLS)
        .with_hooks(ProcessHooks {
            prepare: |_| {},
            stop: stop_tree,
        })
}

/// Serve `body` como `audio/wav` a cada GET. Com `slow`, declara 50 MB e manda 4 KB a cada
/// 100 ms; quando a escrita falha (o cliente morreu), avisa pelo canal devolvido.
fn serve(body: Vec<u8>, slow: bool) -> (String, Receiver<Instant>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (closed, closed_rx) = mpsc::channel();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let body = body.clone();
            let closed = closed.clone();
            thread::spawn(move || answer(stream, &body, slow, &closed));
        }
    });
    (format!("http://127.0.0.1:{port}/tom.wav"), closed_rx)
}

fn answer(mut stream: TcpStream, body: &[u8], slow: bool, closed: &mpsc::Sender<Instant>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    while reader.read_line(&mut line).is_ok_and(|n| n > 2) {
        line.clear();
    }
    let length = if slow { 50_000_000 } else { body.len() };
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: audio/wav\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"
    );
    if stream.write_all(head.as_bytes()).is_err() {
        return;
    }
    if !slow {
        let _ = stream.write_all(body);
        return;
    }
    let chunk = vec![0u8; 4096];
    loop {
        if stream
            .write_all(&chunk)
            .and_then(|()| stream.flush())
            .is_err()
        {
            let _ = closed.send(Instant::now());
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn tone_wav(dir: &Path) -> Vec<u8> {
    let out = dir.join("tom-fonte.wav");
    let status = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"])
        .args(["-f", "lavfi", "-i", "sine=frequency=440:duration=1"])
        .arg(&out)
        .status()
        .expect(NEEDS_TOOLS);
    assert!(status.success());
    fs::read(out).unwrap()
}

#[test]
fn invalid_url_starts_nothing() {
    let dir = scratch("invalid-url");
    let tools = fake_tools(&dir, true);
    for url in ["file:///etc/passwd", "--exec=calc", "ftp://x/a.mp3", ""] {
        let error = download(
            &tools,
            url,
            &dir.join("out"),
            &CancelToken::new(),
            &mut |_| {},
        )
        .unwrap_err();
        assert!(
            matches!(error, MediaError::InvalidUrl(_)),
            "{url}: {error:?}"
        );
    }
    assert!(!dir.join("out").exists());
}

#[test]
fn missing_yt_dlp_is_reported_by_name() {
    let dir = scratch("missing");
    let tools = fake_tools(&dir, false);
    let error = download(
        &tools,
        "https://exemplo.com/aula",
        &dir.join("out"),
        &CancelToken::new(),
        &mut |_| {},
    )
    .unwrap_err();
    assert!(matches!(error, MediaError::ToolNotFound { tool: "yt-dlp" }));
    assert!(error.to_string().contains("yt-dlp"));
}

#[test]
#[ignore = "precisa de yt-dlp, ffmpeg e ffprobe no PATH"]
fn url_downloads_and_imports() {
    let dir = scratch("ok");
    let (url, _) = serve(tone_wav(&dir), false);
    let tools = real_tools();
    let mut progress = Vec::new();
    let got = download(
        &tools,
        &url,
        &dir.join("baixado"),
        &CancelToken::new(),
        &mut |p| progress.push(p),
    )
    .unwrap();
    assert_eq!(got.title, "tom");
    assert!(got.path.starts_with(dir.join("baixado")));
    assert!(got.path.is_file());
    assert!(!progress.is_empty());
    let audio = import(
        &tools,
        &got.path,
        &dir.join("recording.wav"),
        &CancelToken::new(),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!((audio.sample_rate, audio.channels), (48_000, 1));
    assert!(audio.duration >= Duration::from_millis(950));
}

#[test]
#[ignore = "precisa de yt-dlp, ffmpeg e ffprobe no PATH"]
fn cancel_stops_the_whole_download() {
    let dir = scratch("cancel");
    let (url, closed) = serve(Vec::new(), true);
    let tools = real_tools();
    let cancel = CancelToken::new();
    let (started, started_rx) = mpsc::channel();
    let target = dir.join("baixado");
    let worker = {
        let cancel = cancel.clone();
        let target = target.clone();
        thread::spawn(move || {
            download(&tools, &url, &target, &cancel, &mut |p| {
                if p.done_bytes > 0 {
                    let _ = started.send(());
                }
            })
        })
    };
    started_rx
        .recv_timeout(Duration::from_secs(30))
        .expect("o download não começou");
    let asked = Instant::now();
    cancel.cancel();
    let result = worker.join().unwrap();
    assert!(matches!(result, Err(MediaError::Cancelled)), "{result:?}");
    assert!(asked.elapsed() < Duration::from_secs(2));
    // Nenhum processo da árvore segue baixando: o servidor vê a conexão cair, e a pasta, sem
    // arquivo aberto, pode ser apagada (no Windows um arquivo aberto impede).
    closed
        .recv_timeout(Duration::from_secs(5))
        .expect("a conexão continuou aberta");
    fs::remove_dir_all(&target).unwrap();
}
