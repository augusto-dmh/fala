//! `--analyze`: acha o clique de cada canal nos 15 s iniciais e finais e calcula o drift (door 3).

use std::path::Path;

use anyhow::{anyhow, Context};

use super::{failed, input, Failure};

pub const COLUMNS: [&str; 8] = [
    "onset_mic_start_s",
    "onset_sys_start_s",
    "offset_start_ms",
    "onset_mic_end_s",
    "onset_sys_end_s",
    "offset_end_ms",
    "drift_ms",
    "drift_ppm",
];

const WINDOW_S: u32 = 15;

/// Uma janela de frames estéreo intercalados (L, R) que começa no frame `first` do arquivo.
pub struct Window<'a> {
    pub interleaved: &'a [i16],
    pub first: u64,
}

pub fn analyze(path: &Path, threshold: f32) -> Result<Vec<String>, Failure> {
    let mut reader = hound::WavReader::open(path)
        .with_context(|| format!("{} não é um WAV legível", path.display()))
        .map_err(input)?;
    let spec = reader.spec();
    if spec.channels != 2
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
    {
        let format = match spec.sample_format {
            hound::SampleFormat::Int => "int",
            hound::SampleFormat::Float => "float",
        };
        return Err(input(anyhow!(
            "{}: esperado 2 canais PCM int de 16 bits; encontrado {} canal(is), {} bits {format}",
            path.display(),
            spec.channels,
            spec.bits_per_sample
        )));
    }
    let rate = spec.sample_rate;
    let total = u64::from(reader.duration());
    // Em arquivos com menos de 30 s as janelas se sobreporiam; cada uma fica com metade.
    let window = u64::from(WINDOW_S * rate).min(total / 2);
    let start = read_window(&mut reader, 0, window).map_err(failed)?;
    let end_first = total - window;
    let end = read_window(&mut reader, end_first, window).map_err(failed)?;
    let row = drift_row(
        &Window {
            interleaved: &start,
            first: 0,
        },
        &Window {
            interleaved: &end,
            first: end_first,
        },
        rate,
        threshold,
    )
    .map_err(failed)?;
    Ok(row)
}

fn read_window(
    reader: &mut hound::WavReader<std::io::BufReader<std::fs::File>>,
    first: u64,
    frames: u64,
) -> anyhow::Result<Vec<i16>> {
    reader.seek(u32::try_from(first)?)?;
    reader
        .samples::<i16>()
        .take(usize::try_from(frames * 2)?)
        .map(|s| s.map_err(Into::into))
        .collect()
}

/// Offset e drift pela door 3; um canal sem onset numa janela é erro nomeando canal/janela e pico.
pub fn drift_row(
    start: &Window,
    end: &Window,
    rate: u32,
    threshold: f32,
) -> anyhow::Result<Vec<String>> {
    let rate = f64::from(rate);
    let onset = |w: &Window, channel: usize, label: &str| -> anyhow::Result<f64> {
        let samples = w.interleaved.iter().skip(channel).step_by(2);
        match samples
            .clone()
            .position(|&x| f32::from(x).abs() / 32768.0 >= threshold)
        {
            Some(i) => Ok((w.first + i as u64) as f64 / rate),
            None => {
                let peak = samples.map(|&x| i32::from(x).abs()).max().unwrap_or(0);
                Err(anyhow!(
                    "{label}: nenhum onset ≥ {threshold} na janela; pico {:.3}",
                    f64::from(peak) / 32768.0
                ))
            }
        }
    };
    let mic_start = onset(start, 0, "mic/start")?;
    let sys_start = onset(start, 1, "sys/start")?;
    let mic_end = onset(end, 0, "mic/end")?;
    let sys_end = onset(end, 1, "sys/end")?;

    let offset_start_ms = (sys_start - mic_start) * 1000.0;
    let offset_end_ms = (sys_end - mic_end) * 1000.0;
    let drift_ms = offset_end_ms - offset_start_ms;
    let drift_ppm = drift_ms / ((mic_end - mic_start) * 1000.0) * 1e6;
    Ok(vec![
        format!("{mic_start:.3}"),
        format!("{sys_start:.3}"),
        format!("{offset_start_ms:.1}"),
        format!("{mic_end:.3}"),
        format!("{sys_end:.3}"),
        format!("{offset_end_ms:.1}"),
        format!("{drift_ms:.1}"),
        format!("{drift_ppm:.1}"),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    /// 15 s de silêncio estéreo a partir de `first_s`, com um pulso em L e outro em R.
    fn window(first_s: u32, l_s: f64, r_s: f64) -> (Vec<i16>, u64) {
        let first = u64::from(first_s * RATE);
        let mut v = vec![0i16; (WINDOW_S * RATE * 2) as usize];
        for (channel, at) in [(0usize, l_s), (1, r_s)] {
            let frame = (at * f64::from(RATE)).round() as u64 - first;
            v[frame as usize * 2 + channel] = 16_000;
        }
        (v, first)
    }

    #[test]
    fn one_hour_click_pair_drift_is_20ms() {
        let (s, s0) = window(0, 2.000, 2.010);
        let (e, e0) = window(3600 - 15, 3598.000, 3598.030);
        let row = drift_row(
            &Window {
                interleaved: &s,
                first: s0,
            },
            &Window {
                interleaved: &e,
                first: e0,
            },
            RATE,
            0.1,
        )
        .unwrap();
        assert_eq!(row[2], "10.0");
        assert_eq!(row[5], "30.0");
        assert_eq!(row[6], "20.0");
    }
}
