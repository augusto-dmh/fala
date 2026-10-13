//! Apagar um trecho do áudio de uma sessão: zerar, não cortar. A linha do tempo fica igual, então
//! os tempos dos outros trechos, as omissões do ditado e "tocar o trecho" continuam valendo.
//!
//! Cada Opus é decodificado do início ao fim, com as amostras do trecho zeradas, e recodificado
//! num `.part`; só depois de os dois `.part` decodificarem com a contagem exata de amostras eles
//! substituem os originais (a regra da ADR-0014 para o WAV vale para o Opus reescrito).

use std::fs::{self, File};
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use ogg::PacketReader;
use opus::{Channels, Decoder};

use crate::encode::{
    opus_error, parse_head, part_path, sync_dir, OpusStream, FRAME, MAX_DECODED, SAMPLE_RATE_HZ,
};
use crate::{validate_opus, RetentionError, MIC_FILE, SYSTEM_FILE};

/// Amostras por ms a 48 kHz.
const PER_MS: u64 = SAMPLE_RATE_HZ as u64 / 1000;
/// Cabeçalho fixo de um page Ogg, antes da tabela de segmentos.
const PAGE_HEADER: usize = 27;

/// Zera `[t0_ms, t1_ms)` de `mic.opus` e `sys.opus` em `session_dir`. Em qualquer erro, os
/// originais ficam como estavam.
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
    let range = (t0_ms * PER_MS, t1_ms * PER_MS);
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
    for ((path, _), part) in files.iter().zip(&parts) {
        fs::rename(part, path).map_err(|e| RetentionError::io(path, e))?;
    }
    sync_dir(session_dir);
    Ok(())
}

/// Decodifica `src` inteiro, zera `[from, to)` (em amostras depois do pre-skip) e recodifica em
/// `part`. Devolve o número de amostras, igual ao de `src`.
fn rewrite_zeroed(
    src: &Path,
    part: &Path,
    serial: u32,
    (from, to): (u64, u64),
) -> Result<u64, RetentionError> {
    let total = stream_samples(src)?;
    let mut out = OpusStream::create(part, serial)?;
    // Como no `retain_wav`: quadros de zeros até cobrir `total + pre_skip` (RFC 7845 §4).
    let target = total + out.pre_skip;
    let mut frame = Vec::with_capacity(FRAME);
    let mut index: u64 = 0;
    let mut encoded: u64 = 0;
    decode_all(src, |pcm| {
        for &sample in pcm {
            frame.push(if (from..to).contains(&index) {
                0
            } else {
                sample
            });
            index += 1;
            if frame.len() == FRAME {
                encoded += FRAME as u64;
                out.write_frame(&frame, encoded, false)?;
                frame.clear();
            }
        }
        Ok(())
    })?;
    if index != total {
        return Err(RetentionError::Unreadable {
            path: src.to_path_buf(),
            reason: format!("decodificadas {index} amostras, o granule diz {total}"),
        });
    }
    loop {
        frame.resize(FRAME, 0);
        encoded += FRAME as u64;
        let last = encoded >= target;
        out.write_frame(&frame, if last { target } else { encoded }, last)?;
        frame.clear();
        if last {
            break;
        }
    }
    out.finish(part)?;
    Ok(total)
}

/// Entrega o PCM de um Ogg Opus mono do começo ao fim, sem o pre-skip e cortado no granule final.
fn decode_all(
    path: &Path,
    mut sink: impl FnMut(&[i16]) -> Result<(), RetentionError>,
) -> Result<(), RetentionError> {
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
    let mut decoder = Decoder::new(SAMPLE_RATE_HZ, Channels::Mono).map_err(opus_error)?;
    let mut pcm = vec![0i16; MAX_DECODED];
    let mut position: u64 = 0;
    while let Some(packet) = next()? {
        let n = decoder
            .decode(&packet.data, &mut pcm, false)
            .map_err(|e| unreadable(format!("pacote indecodificável: {e}")))?
            as u64;
        let mut valid = n;
        if packet.last_in_stream() {
            valid = valid.min(packet.absgp_page().saturating_sub(position));
        }
        let from = pre_skip.max(position);
        let to = position + valid;
        if to > from {
            sink(&pcm[(from - position) as usize..(to - position) as usize])?;
        }
        position += n;
        if packet.last_in_stream() {
            return Ok(());
        }
    }
    Err(unreadable("stream sem fim (EOS)".into()))
}

/// Amostras de um Ogg Opus mono depois do pre-skip, pelo granule do último page. Lê só os
/// cabeçalhos dos pages.
fn stream_samples(path: &Path) -> Result<u64, RetentionError> {
    let unreadable = |reason: String| RetentionError::Unreadable {
        path: path.to_path_buf(),
        reason,
    };
    let file = File::open(path).map_err(|e| RetentionError::io(path, e))?;
    let head = PacketReader::new(BufReader::new(&file))
        .read_packet()
        .map_err(|e| unreadable(format!("Ogg ilegível: {e}")))?
        .ok_or_else(|| unreadable("sem OpusHead".into()))?;
    let pre_skip = u64::from(parse_head(&head.data).map_err(&unreadable)?);
    let mut file = BufReader::new(file);
    file.seek(SeekFrom::Start(0))
        .map_err(|e| RetentionError::io(path, e))?;
    let mut header = [0u8; PAGE_HEADER];
    let mut lacing = [0u8; 255];
    let mut last = None;
    loop {
        match file.read_exact(&mut header) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(RetentionError::io(path, e)),
        }
        if &header[..4] != b"OggS" {
            return Err(unreadable("page Ogg inválido".into()));
        }
        let mut granule = [0u8; 8];
        granule.copy_from_slice(&header[6..14]);
        let segments = usize::from(header[26]);
        file.read_exact(&mut lacing[..segments])
            .map_err(|e| RetentionError::io(path, e))?;
        let body: i64 = lacing[..segments].iter().map(|&b| i64::from(b)).sum();
        file.seek_relative(body)
            .map_err(|e| RetentionError::io(path, e))?;
        last = Some(u64::from_le_bytes(granule));
    }
    let last = last.ok_or_else(|| unreadable("sem pages".into()))?;
    Ok(last.saturating_sub(pre_skip))
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
        let names: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.iter().all(|n| !n.ends_with(".part")), "{names:?}");
    }
}
