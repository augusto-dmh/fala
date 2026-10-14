//! Apagar um trecho do áudio retido: `zero_range` nos dois Opus e `zero_wav` no WAV de trabalho.

#![allow(clippy::unwrap_used)]

use std::path::Path;

use fala_retention::{decode_range, retain_wav, zero_range, zero_wav, MIC_FILE, SYSTEM_FILE};

const RATE: u32 = 48_000;
const PER_MS: usize = 48;

/// Fala de mentira: dois tons por canal, diferentes no mic e no sistema.
fn sample(i: u32, channel: u32) -> i16 {
    let t = f64::from(i) / f64::from(RATE);
    let (a, b) = if channel == 0 {
        (220.0, 660.0)
    } else {
        (330.0, 990.0)
    };
    let x = (t * a * std::f64::consts::TAU).sin() + 0.5 * (t * b * std::f64::consts::TAU).sin();
    (x * 9_000.0) as i16
}

fn write_wav(path: &Path, seconds: u32) {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..seconds * RATE {
        writer.write_sample(sample(i, 0)).unwrap();
        writer.write_sample(sample(i, 1)).unwrap();
    }
    writer.finalize().unwrap();
}

fn decode_all(path: &Path, samples: u64) -> Vec<i16> {
    decode_range(path, 0, samples).unwrap()
}

/// The raw packets of an Ogg Opus, headers included.
fn packets(path: &Path) -> Vec<Vec<u8>> {
    let file = std::io::BufReader::new(std::fs::File::open(path).unwrap());
    let mut reader = ogg::PacketReader::new(file);
    std::iter::from_fn(|| reader.read_packet().unwrap().map(|p| p.data)).collect()
}

fn rms(pcm: &[i16]) -> f64 {
    let sum: f64 = pcm.iter().map(|&s| f64::from(s).powi(2)).sum();
    (sum / pcm.len() as f64).sqrt()
}

fn correlation(a: &[i16], b: &[i16]) -> f64 {
    let mean = |x: &[i16]| x.iter().map(|&s| f64::from(s)).sum::<f64>() / x.len() as f64;
    let (ma, mb) = (mean(a), mean(b));
    let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
    for (&x, &y) in a.iter().zip(b) {
        let (x, y) = (f64::from(x) - ma, f64::from(y) - mb);
        ab += x * y;
        aa += x * x;
        bb += y * y;
    }
    ab / (aa * bb).sqrt()
}

#[test]
fn zero_range_silences_both_channels_and_keeps_the_rest() {
    let tmp = tempfile::tempdir().unwrap();
    let wav = tmp.path().join("recording.wav");
    let dir = tmp.path().join("sessao");
    write_wav(&wav, 6);
    let retained = retain_wav(&wav, &dir).unwrap();
    let files = [dir.join(MIC_FILE), dir.join(SYSTEM_FILE)];
    let before: Vec<Vec<i16>> = files
        .iter()
        .map(|f| decode_all(f, retained.samples))
        .collect();
    let packets_before: Vec<Vec<Vec<u8>>> = files.iter().map(|f| packets(f)).collect();

    zero_range(&dir, 2_000, 3_500).unwrap();

    for (file, original) in files.iter().zip(&before) {
        let after = decode_all(file, retained.samples + 10_000);
        assert_eq!(after.len() as u64, retained.samples, "{file:?}");
        // Dentro do trecho, longe 40 ms das bordas (o Opus espalha um quadro e o lookahead em
        // cada borda): abaixo de -90 dBFS. Medido: a partir dos 40 ms, zero exato.
        let inside = &after[2_040 * PER_MS..3_460 * PER_MS];
        let dbfs = 20.0 * (rms(inside).max(1e-9) / 32_768.0).log10();
        assert!(dbfs < -90.0, "{file:?}: {dbfs:.1} dBFS no trecho");
        // Fora dele, o mesmo som (recodificado).
        for (from, to) in [(100, 1_900), (3_600, 5_900)] {
            let range = from * PER_MS..to * PER_MS;
            let c = correlation(&original[range.clone()], &after[range]);
            assert!(c > 0.9, "{file:?} {from}-{to} ms: correlação {c:.3}");
        }
    }
    // Away from the stretch, the Opus packets are the same bytes: no second-generation loss.
    for (file, before) in files.iter().zip(&packets_before) {
        let after = packets(file);
        assert_eq!(after.len(), before.len());
        assert_eq!(after[..40], before[..40], "{file:?}");
        assert_eq!(after[240..], before[240..], "{file:?}");
        assert_ne!(after[100..150], before[100..150], "{file:?}");
    }
    assert!(std::fs::read_dir(&dir).unwrap().all(|e| !e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .ends_with(".part")));
}

#[test]
fn zero_wav_blanks_the_range_in_place() {
    let tmp = tempfile::tempdir().unwrap();
    let wav = tmp.path().join("recording.wav");
    write_wav(&wav, 2);
    let size = std::fs::metadata(&wav).unwrap().len();

    zero_wav(&wav, 500, 1_000).unwrap();
    // Um trecho além do fim é recortado.
    zero_wav(&wav, 1_900, 9_000).unwrap();

    assert_eq!(std::fs::metadata(&wav).unwrap().len(), size);
    let samples: Vec<i16> = hound::WavReader::open(&wav)
        .unwrap()
        .into_samples::<i16>()
        .map(Result::unwrap)
        .collect();
    assert_eq!(samples.len(), 2 * 2 * RATE as usize);
    for (frame, pair) in samples.chunks(2).enumerate() {
        let ms = frame / PER_MS;
        let zeroed = (500..1_000).contains(&ms) || ms >= 1_900;
        let expected = if zeroed {
            [0, 0]
        } else {
            [sample(frame as u32, 0), sample(frame as u32, 1)]
        };
        assert_eq!(pair, expected, "quadro {frame}");
    }
}
