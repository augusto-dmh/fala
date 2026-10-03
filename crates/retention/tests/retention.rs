//! Conversão, validação e apagamento do áudio retido (checks de `audio-retention`).

// Os helpers abaixo ficam fora de `#[test]`, onde o `allow-unwrap-in-tests` do clippy não chega.
#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use fala_meeting::SessionId;
use fala_retention::{
    delete_session_audio, retain_wav, session_audio_dir, validate_opus, RetentionError,
};

const RATE: u32 = 48_000;

/// Um gerador determinístico de ruído em [-1, 1].
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 8) as f32 / (1u32 << 23) as f32 * 2.0 - 1.0
    }
}

fn tone(i: u32) -> f32 {
    (2.0 * std::f32::consts::PI * 440.0 * i as f32 / RATE as f32).sin()
}

fn to_i16(x: f32) -> i16 {
    (x.clamp(-1.0, 1.0) * 32_767.0) as i16
}

/// Escreve um WAV com o formato dado e `frames` quadros gerados por `sample(i, canal)`.
fn write_wav(
    path: &Path,
    rate: u32,
    channels: u16,
    bits: u16,
    frames: u32,
    mut sample: impl FnMut(u32, u16) -> f32,
) {
    let spec = hound::WavSpec {
        channels,
        sample_rate: rate,
        bits_per_sample: bits,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..frames {
        for c in 0..channels {
            let x = sample(i, c);
            if bits == 16 {
                writer.write_sample(to_i16(x)).unwrap();
            } else {
                writer.write_sample((x * 8_000_000.0) as i32).unwrap();
            }
        }
    }
    writer.finalize().unwrap();
}

/// WAV de trabalho: L = tom de 440 Hz, R = silêncio digital.
fn tone_left_silent_right(path: &Path, frames: u32) {
    write_wav(path, RATE, 2, 16, frames, |i, c| {
        if c == 0 {
            0.5 * tone(i)
        } else {
            0.0
        }
    });
}

/// WAV de trabalho com tom e ruído nos dois canais.
fn tone_and_noise(path: &Path, frames: u32) {
    let mut noise = Noise(7);
    write_wav(path, RATE, 2, 16, frames, |i, _| {
        0.4 * tone(i) + 0.1 * noise.next()
    });
}

/// Os pacotes de um arquivo Ogg, em ordem.
fn packets(path: &Path) -> Vec<Vec<u8>> {
    let mut reader = ogg::PacketReader::new(BufReader::new(File::open(path).unwrap()));
    let mut out = Vec::new();
    while let Some(p) = reader.read_packet().unwrap() {
        out.push(p.data);
    }
    out
}

/// Decodifica um Ogg Opus mono e descarta o pre-skip.
fn decode(path: &Path) -> Vec<i16> {
    let all = packets(path);
    let pre_skip = u16::from_le_bytes([all[0][10], all[0][11]]) as usize;
    let mut decoder = opus::Decoder::new(RATE, opus::Channels::Mono).unwrap();
    let mut pcm = Vec::new();
    let mut buf = vec![0i16; 5760];
    for p in &all[2..] {
        let n = decoder.decode(p, &mut buf, false).unwrap();
        pcm.extend_from_slice(&buf[..n]);
    }
    pcm.split_off(pre_skip)
}

fn rms(pcm: &[i16]) -> f64 {
    let sum: f64 = pcm.iter().map(|&s| (s as f64 / 32_768.0).powi(2)).sum();
    (sum / pcm.len() as f64).sqrt()
}

fn names(dir: &Path) -> BTreeSet<String> {
    match fs::read_dir(dir) {
        Ok(entries) => entries
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect(),
        Err(_) => BTreeSet::new(),
    }
}

fn both() -> BTreeSet<String> {
    ["mic.opus", "sys.opus"]
        .into_iter()
        .map(String::from)
        .collect()
}

struct Fixture {
    _tmp: tempfile::TempDir,
    wav: PathBuf,
    dir: PathBuf,
}

fn fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let wav = tmp.path().join("sessao.wav");
    let dir = session_audio_dir(
        &tmp.path().join("audio"),
        SessionId::from_parts(1_727_000_000_000, 1),
    );
    Fixture {
        _tmp: tmp,
        wav,
        dir,
    }
}

mod encode {
    use super::*;

    #[test]
    fn writes_two_mono_ogg_opus_files() {
        let f = fixture();
        tone_and_noise(&f.wav, 10 * RATE);
        let retained = retain_wav(&f.wav, &f.dir).unwrap();
        assert_eq!(retained.mic, f.dir.join("mic.opus"));
        assert_eq!(retained.system, f.dir.join("sys.opus"));
        for path in [&retained.mic, &retained.system] {
            let all = packets(path);
            let head = &all[0];
            assert!(head.starts_with(b"OpusHead"), "{path:?}");
            assert_eq!(head[9], 1, "canais em {path:?}");
            assert_eq!(
                u32::from_le_bytes([head[12], head[13], head[14], head[15]]),
                48_000
            );
            assert!(all[1].starts_with(b"OpusTags"), "{path:?}");
            for packet in &all[2..] {
                assert_eq!(
                    opus::packet::get_nb_samples(packet, RATE).unwrap(),
                    960,
                    "pacote fora de 20 ms em {path:?}"
                );
            }
        }
    }

    #[test]
    fn left_is_mic_right_is_system() {
        let f = fixture();
        tone_left_silent_right(&f.wav, 10 * RATE);
        let retained = retain_wav(&f.wav, &f.dir).unwrap();
        let mic = rms(&decode(&retained.mic));
        let system = rms(&decode(&retained.system));
        assert!(mic > 0.05, "rms do mic = {mic}");
        assert!(system < 0.001, "rms do sistema = {system}");
    }

    #[test]
    fn ten_seconds_fit_in_40_kb() {
        let f = fixture();
        tone_and_noise(&f.wav, 10 * RATE);
        let retained = retain_wav(&f.wav, &f.dir).unwrap();
        for path in [&retained.mic, &retained.system] {
            let size = fs::metadata(path).unwrap().len();
            assert!(size <= 40_000, "{path:?} tem {size} bytes");
        }
    }

    #[test]
    fn rejects_other_formats() {
        for (rate, channels, bits) in [(48_000, 1, 16), (44_100, 2, 16), (48_000, 2, 24)] {
            let f = fixture();
            write_wav(&f.wav, rate, channels, bits, RATE, |i, _| 0.3 * tone(i));
            let result = retain_wav(&f.wav, &f.dir);
            assert!(
                matches!(
                    result,
                    Err(RetentionError::UnsupportedWav { sample_rate, channels: ch, bits: b })
                        if sample_rate == rate && ch == channels && b == bits
                ),
                "{rate} Hz, {channels} canais, {bits} bits: {result:?}"
            );
            assert!(names(&f.dir).is_empty(), "{:?}", names(&f.dir));
            assert!(f.wav.exists());
        }
    }
}

mod validate {
    use super::*;

    #[test]
    fn sample_count_matches_wav() {
        let f = fixture();
        tone_and_noise(&f.wav, 10 * RATE + 7);
        let retained = retain_wav(&f.wav, &f.dir).unwrap();
        assert_eq!(retained.samples, 480_007);
        validate_opus(&retained.mic, 480_007).unwrap();
        validate_opus(&retained.system, 480_007).unwrap();
    }

    #[test]
    fn wav_deleted_only_after_success() {
        let f = fixture();
        tone_and_noise(&f.wav, 2 * RATE);
        retain_wav(&f.wav, &f.dir).unwrap();
        assert!(!f.wav.exists());
        assert_eq!(names(&f.dir), both());
    }

    #[test]
    fn failure_keeps_wav() {
        let f = fixture();
        tone_and_noise(&f.wav, 2 * RATE);
        let size = fs::metadata(&f.wav).unwrap().len();
        let blocker = f.wav.with_file_name("arquivo-comum");
        fs::write(&blocker, b"nao sou pasta").unwrap();
        let result = retain_wav(&f.wav, &blocker.join("sessao"));
        assert!(result.is_err(), "{result:?}");
        assert_eq!(fs::metadata(&f.wav).unwrap().len(), size);
    }

    #[test]
    fn rename_failure_after_validation_keeps_wav() {
        let f = fixture();
        tone_and_noise(&f.wav, 2 * RATE);
        let size = fs::metadata(&f.wav).unwrap().len();
        // `sys.opus` ocupado por uma pasta não vazia: o renomeio falha depois da validação.
        fs::create_dir_all(f.dir.join("sys.opus")).unwrap();
        fs::write(f.dir.join("sys.opus").join("ocupado"), b"x").unwrap();
        let result = retain_wav(&f.wav, &f.dir);
        assert!(
            matches!(result, Err(RetentionError::Io { .. })),
            "{result:?}"
        );
        assert_eq!(fs::metadata(&f.wav).unwrap().len(), size);
    }

    #[test]
    fn truncated_or_wrong_count_fails() {
        let f = fixture();
        tone_and_noise(&f.wav, 4 * RATE);
        let retained = retain_wav(&f.wav, &f.dir).unwrap();

        let bytes = fs::read(&retained.mic).unwrap();
        let truncated = f.dir.join("truncado.opus");
        fs::write(&truncated, &bytes[..bytes.len() / 2]).unwrap();
        let result = validate_opus(&truncated, retained.samples);
        assert!(
            matches!(result, Err(RetentionError::ValidationFailed { .. })),
            "{result:?}"
        );

        let expected = retained.samples + 1;
        match validate_opus(&retained.mic, expected) {
            Err(RetentionError::ValidationFailed { reason, .. }) => {
                assert!(reason.contains(&expected.to_string()), "{reason}");
                assert!(reason.contains(&retained.samples.to_string()), "{reason}");
            }
            other => panic!("esperava ValidationFailed, veio {other:?}"),
        }
    }

    #[test]
    fn rerun_overwrites_leftover_parts() {
        let f = fixture();
        tone_and_noise(&f.wav, 2 * RATE);
        fs::create_dir_all(&f.dir).unwrap();
        fs::write(
            f.dir.join("mic.opus.part"),
            b"lixo de uma tentativa anterior",
        )
        .unwrap();
        fs::write(f.dir.join("sys.opus.part"), b"lixo").unwrap();
        let retained = retain_wav(&f.wav, &f.dir).unwrap();
        assert_eq!(names(&f.dir), both());
        validate_opus(&retained.mic, retained.samples).unwrap();
        validate_opus(&retained.system, retained.samples).unwrap();
    }
}

mod delete {
    use super::*;

    #[test]
    fn removes_only_that_session() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let a = SessionId::from_parts(1_727_000_000_000, 1);
        let b = SessionId::from_parts(1_727_000_000_000, 2);
        for id in [a, b] {
            let dir = session_audio_dir(root, id);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("mic.opus"), b"x").unwrap();
        }
        fs::write(root.join("solto.txt"), b"x").unwrap();

        assert!(delete_session_audio(root, a).unwrap());
        assert!(!session_audio_dir(root, a).exists());
        assert!(session_audio_dir(root, b).join("mic.opus").exists());
        assert!(root.join("solto.txt").exists());
    }

    #[test]
    fn missing_dir_is_false() {
        let tmp = tempfile::tempdir().unwrap();
        let c = SessionId::from_parts(1_727_000_000_000, 3);
        assert!(!delete_session_audio(tmp.path(), c).unwrap());
    }
}
