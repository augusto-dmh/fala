//! Apagar um trecho do áudio de uma sessão: zerar, não cortar. A linha do tempo fica igual, então
//! os tempos dos outros trechos, as omissões do ditado e "tocar o trecho" continuam valendo.
//!
//! Em cada Opus, os pacotes em volta do trecho são recodificados com as amostras zeradas e os
//! outros são copiados, num `.part`; só depois de os dois `.part` decodificarem com a contagem
//! exata de amostras eles substituem os originais (a regra da ADR-0014 para o WAV vale para o
//! Opus reescrito).

use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use ogg::PacketReader;
use opus::{Channels, Decoder};

use crate::encode::{
    new_encoder, opus_error, parse_head, part_path, sync_dir, OpusStream, FRAME, MAX_DECODED,
    MAX_PACKET, SAMPLE_RATE_HZ,
};
use crate::{validate_opus, RetentionError, MIC_FILE, PREROLL_SAMPLES, SYSTEM_FILE};

/// Amostras por ms a 48 kHz.
const PER_MS: u64 = SAMPLE_RATE_HZ as u64 / 1000;
/// Quanto recodificar antes e depois do trecho: a emenda com os pacotes copiados fica longe dele.
const MARGIN: u64 = SAMPLE_RATE_HZ as u64;
/// Pacotes de aquecimento do encoder novo, descartados.
const WARM: u64 = 5;

/// Zera `[t0_ms, t1_ms)` de `mic.opus` e `sys.opus` em `session_dir`. Se um `.part` não sai ou
/// não valida, os originais ficam como estavam. Se a troca do segundo falhar (arquivo preso), o
/// primeiro já está zerado e o erro volta: tentar de novo zera o que falta.
pub fn zero_range(session_dir: &Path, t0_ms: u64, t1_ms: u64) -> Result<(), RetentionError> {
    zero_range_with(session_dir, t0_ms, t1_ms, validate_opus)
}

/// `zero_range` com o validador injetável, para provar que um `.part` recusado não substitui.
fn zero_range_with(
    session_dir: &Path,
    t0_ms: u64,
    t1_ms: u64,
    validate: impl Fn(&Path, u64) -> Result<(), RetentionError>,
) -> Result<(), RetentionError> {
    let range = (t0_ms.saturating_mul(PER_MS), t1_ms.saturating_mul(PER_MS));
    let files = [
        (session_dir.join(MIC_FILE), 1),
        (session_dir.join(SYSTEM_FILE), 2),
    ];
    let mut parts: Vec<PathBuf> = Vec::new();
    let written = files.iter().try_for_each(|(path, serial)| {
        let part = part_path(path);
        parts.push(part.clone());
        let samples = rewrite_zeroed(path, &part, *serial, range)?;
        validate(&part, samples)
    });
    if let Err(e) = written {
        for part in &parts {
            let _ = fs::remove_file(part);
        }
        return Err(e);
    }
    let renamed = files.iter().zip(&parts).try_for_each(|((path, _), part)| {
        fs::rename(part, path).map_err(|e| RetentionError::io(path, e))
    });
    if renamed.is_err() {
        for part in &parts {
            let _ = fs::remove_file(part);
        }
    }
    sync_dir(session_dir);
    renamed
}

/// Reescreve `src` em `part` com `[from, to)` (em amostras depois do pre-skip) zerado. Só os
/// pacotes de uma janela em volta do trecho são recodificados; os outros são copiados byte a
/// byte, então o resto do áudio não perde qualidade e o custo segue o tamanho do trecho, não o
/// da reunião. Devolve o número de amostras, igual ao de `src`.
fn rewrite_zeroed(
    src: &Path,
    part: &Path,
    serial: u32,
    (from, to): (u64, u64),
) -> Result<u64, RetentionError> {
    let stream = read_stream(src)?;
    let (pre_skip, frame) = (stream.pre_skip, FRAME as u64);
    let count = stream.packets.len() as u64;
    let total = stream.final_granule.saturating_sub(pre_skip);
    let (from, to) = (from.min(total), to.min(total));
    // A janela, em pacotes: o trecho na posição do stream (com o pre-skip) mais uma margem.
    let first = (from + pre_skip).saturating_sub(MARGIN) / frame;
    let end = (to + pre_skip + MARGIN).div_ceil(frame).min(count);
    // Um encoder novo atrasa o sinal em `pre_skip`: alimentado a partir do instante original
    // `P`, o pacote dele cai na posição `P` do stream. Os `warm` primeiros só aquecem o encoder.
    let warm = first.min(WARM);
    let start = (first - warm) * frame;
    let mut pcm = decode_positions(src, &stream, start + pre_skip, end * frame + pre_skip)?;
    for (i, sample) in pcm.iter_mut().enumerate() {
        if (from..to).contains(&(start + i as u64)) {
            *sample = 0;
        }
    }
    let mut encoder = new_encoder()?;
    let mut out = vec![0u8; MAX_PACKET];
    let mut packets = stream.packets;
    for (m, chunk) in pcm.chunks(FRAME).enumerate() {
        let mut input = chunk.to_vec();
        input.resize(FRAME, 0);
        let len = encoder.encode(&input, &mut out).map_err(opus_error)?;
        if let Some(packet) = (m as u64)
            .checked_sub(warm)
            .and_then(|k| packets.get_mut((first + k) as usize))
        {
            *packet = out[..len].to_vec();
        }
    }

    let mut writer = OpusStream::create(part, serial)?;
    if writer.pre_skip != pre_skip {
        return Err(RetentionError::Opus(format!(
            "pre-skip {} no original e {} no encoder",
            pre_skip, writer.pre_skip
        )));
    }
    for (i, packet) in packets.into_iter().enumerate() {
        let last = i as u64 + 1 == count;
        let granule = if last {
            stream.final_granule
        } else {
            (i as u64 + 1) * frame
        };
        writer.write_packet(packet, granule, last)?;
    }
    writer.finish(part)?;
    Ok(total)
}

/// Um Ogg Opus retido lido em memória: os pacotes de áudio (uma hora a 24 kbps são ~11 MB).
struct Stream {
    pre_skip: u64,
    packets: Vec<Vec<u8>>,
    final_granule: u64,
}

fn read_stream(path: &Path) -> Result<Stream, RetentionError> {
    let unreadable = |reason: String| RetentionError::Unreadable {
        path: path.to_path_buf(),
        reason,
    };
    let file = File::open(path).map_err(|e| RetentionError::io(path, e))?;
    let mut reader = PacketReader::new(BufReader::new(file));
    let mut next = || {
        reader
            .read_packet()
            .map_err(|e| unreadable(format!("Ogg ilegível: {e}")))
    };
    let head = next()?.ok_or_else(|| unreadable("sem OpusHead".into()))?;
    let pre_skip = u64::from(parse_head(&head.data).map_err(&unreadable)?);
    next()?.ok_or_else(|| unreadable("sem OpusTags".into()))?;
    let mut packets = Vec::new();
    while let Some(packet) = next()? {
        let last = packet.last_in_stream();
        let granule = packet.absgp_page();
        packets.push(packet.data);
        if last {
            return Ok(Stream {
                pre_skip,
                packets,
                final_granule: granule,
            });
        }
    }
    Err(unreadable("stream sem fim (EOS)".into()))
}

/// O PCM das posições `[a, b)` do stream (com o pre-skip), decodificando desde
/// [`PREROLL_SAMPLES`] antes de `a`; o que passa do fim vem em zeros.
fn decode_positions(
    path: &Path,
    stream: &Stream,
    a: u64,
    b: u64,
) -> Result<Vec<i16>, RetentionError> {
    let frame = FRAME as u64;
    let first = a.saturating_sub(PREROLL_SAMPLES) / frame;
    let mut decoder = Decoder::new(SAMPLE_RATE_HZ, Channels::Mono).map_err(opus_error)?;
    let mut pcm = vec![0i16; MAX_DECODED];
    let mut out = Vec::with_capacity(usize::try_from(b - a).unwrap_or(0));
    let mut position = first * frame;
    for packet in stream.packets.iter().skip(first as usize) {
        if position >= b {
            break;
        }
        let n = decoder
            .decode(packet, &mut pcm, false)
            .map_err(|e| RetentionError::Unreadable {
                path: path.to_path_buf(),
                reason: format!("pacote indecodificável: {e}"),
            })? as u64;
        let (lo, hi) = (a.max(position), b.min(position + n));
        if hi > lo {
            out.extend_from_slice(&pcm[(lo - position) as usize..(hi - position) as usize]);
        }
        position += n;
    }
    out.resize(usize::try_from(b - a).unwrap_or(0), 0);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_part_that_does_not_validate_replaces_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let wav = tmp.path().join("sessao.wav");
        let dir = tmp.path().join("sessao");
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: SAMPLE_RATE_HZ,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&wav, spec).unwrap();
        for i in 0..SAMPLE_RATE_HZ * 2 {
            writer.write_sample(((i % 200) as i16 - 100) * 50).unwrap();
        }
        writer.finalize().unwrap();
        crate::retain_wav(&wav, &dir).unwrap();
        let before = fs::read(dir.join(MIC_FILE)).unwrap();

        let refused = zero_range_with(&dir, 0, 500, |path, _| {
            Err(RetentionError::ValidationFailed {
                path: path.to_path_buf(),
                reason: "recusado no teste".into(),
            })
        });
        assert!(matches!(
            refused,
            Err(RetentionError::ValidationFailed { .. })
        ));
        assert_eq!(fs::read(dir.join(MIC_FILE)).unwrap(), before);
        assert!(!dir.join("mic.opus.part").exists() && !dir.join("sys.opus.part").exists());
    }
}
