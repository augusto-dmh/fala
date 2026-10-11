//! Um trecho do áudio retido, sem decodificar o arquivo inteiro (2.F9, tocar o trecho).

use std::fs::File;
use std::io::{BufReader, Read, SeekFrom};
use std::path::Path;

use ogg::PacketReader;
use opus::{Channels, Decoder};

use crate::encode::{parse_head, MAX_DECODED, SAMPLE_RATE_HZ};
use crate::RetentionError;

/// Quanto decodificar antes do início pedido. Um decoder novo precisa de histórico para as
/// primeiras amostras saírem certas (RFC 7845 §4.6 pede ao menos 80 ms); 1 s sobra.
pub const PREROLL_SAMPLES: u64 = SAMPLE_RATE_HZ as u64;

/// Cabeçalho fixo de um page Ogg, antes da tabela de segmentos.
const PAGE_HEADER: usize = 27;
/// `header_type`: o primeiro pacote do page continua do page anterior.
const CONTINUED: u8 = 0x01;
/// Granule de um page em que nenhum pacote termina.
const NO_GRANULE: u64 = u64::MAX;

/// O PCM mono 48 kHz de `[start, end)` de um Ogg Opus retido, em amostras depois do pre-skip.
///
/// O trecho é recortado no fim do arquivo; um início além do fim devolve vazio. Os pages antes
/// do trecho são pulados pelo granule, lendo só o cabeçalho, e a decodificação começa num
/// page ao menos [`PREROLL_SAMPLES`] antes de `start`.
pub fn decode_range(path: &Path, start: u64, end: u64) -> Result<Vec<i16>, RetentionError> {
    decode_range_counted(path, start, end).map(|(pcm, _)| pcm)
}

/// `decode_range` e o número de pacotes que ele decodificou.
pub(crate) fn decode_range_counted(
    path: &Path,
    start: u64,
    end: u64,
) -> Result<(Vec<i16>, usize), RetentionError> {
    let unreadable = |reason: String| RetentionError::Unreadable {
        path: path.to_path_buf(),
        reason,
    };
    if end <= start {
        return Ok((Vec::new(), 0));
    }
    let file = File::open(path).map_err(|e| RetentionError::io(path, e))?;
    let mut reader = PacketReader::new(BufReader::new(file));
    let mut next = || {
        reader
            .read_packet()
            .map_err(|e| unreadable(format!("Ogg ilegível: {e}")))
    };
    let head = next()?.ok_or_else(|| unreadable("sem OpusHead".into()))?;
    let pre_skip = u64::from(parse_head(&head.data).map_err(&unreadable)?);

    // Posições no stream do decoder, que conta o pre-skip.
    let first = start + pre_skip;
    let last = end + pre_skip;
    let (offset, mut position) = seek_point(path, first.saturating_sub(PREROLL_SAMPLES))?;
    reader
        .seek_bytes(SeekFrom::Start(offset))
        .map_err(|e| RetentionError::io(path, e))?;

    let mut decoder = Decoder::new(SAMPLE_RATE_HZ, Channels::Mono)
        .map_err(|e| RetentionError::Opus(e.to_string()))?;
    let mut pcm = vec![0i16; MAX_DECODED];
    let mut out = Vec::with_capacity(usize::try_from(end - start).unwrap_or(0));
    let mut decoded = 0;
    while position < last {
        let Some(packet) = reader
            .read_packet()
            .map_err(|e| unreadable(format!("Ogg ilegível: {e}")))?
        else {
            break;
        };
        let n = decoder
            .decode(&packet.data, &mut pcm, false)
            .map_err(|e| unreadable(format!("pacote indecodificável: {e}")))?;
        decoded += 1;
        let mut valid = n as u64;
        if packet.last_in_stream() {
            // O granule final corta o enchimento do último quadro (RFC 7845 §4).
            valid = valid.min(packet.absgp_page().saturating_sub(position));
        }
        let from = first.max(position);
        let to = last.min(position + valid);
        if to > from {
            out.extend_from_slice(&pcm[(from - position) as usize..(to - position) as usize]);
        }
        position += n as u64;
        if packet.last_in_stream() {
            break;
        }
    }
    Ok((out, decoded))
}

/// O byte do último page de áudio cujo primeiro pacote começa em `target` ou antes, e a
/// posição (com pre-skip) em que esse pacote começa. Lê só os cabeçalhos dos pages.
///
/// Os pages de cabeçalho (`OpusHead`, `OpusTags`) têm granule 0; um page que continua um
/// pacote do anterior não serve de ponto de partida.
fn seek_point(path: &Path, target: u64) -> Result<(u64, u64), RetentionError> {
    let file = File::open(path).map_err(|e| RetentionError::io(path, e))?;
    let mut file = BufReader::new(file);
    let mut offset: u64 = 0;
    let mut before: u64 = 0;
    let mut found = None;
    let mut header = [0u8; PAGE_HEADER];
    let mut lacing = [0u8; 255];
    loop {
        match file.read_exact(&mut header) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(RetentionError::io(path, e)),
        }
        if &header[..4] != b"OggS" {
            return Err(RetentionError::Unreadable {
                path: path.to_path_buf(),
                reason: format!("sem page Ogg no byte {offset}"),
            });
        }
        let continued = header[5] & CONTINUED != 0;
        let mut granule = [0u8; 8];
        granule.copy_from_slice(&header[6..14]);
        let granule = u64::from_le_bytes(granule);
        let segments = usize::from(header[26]);
        file.read_exact(&mut lacing[..segments])
            .map_err(|e| RetentionError::io(path, e))?;
        let body: u64 = lacing[..segments].iter().map(|&b| u64::from(b)).sum();

        if granule != 0 {
            if before > target {
                break;
            }
            if !continued {
                found = Some((offset, before));
            }
        }
        if granule != NO_GRANULE {
            before = granule;
        }
        let skip = i64::try_from(body).map_err(|_| RetentionError::Unreadable {
            path: path.to_path_buf(),
            reason: "page maior que o possível".into(),
        })?;
        file.seek_relative(skip)
            .map_err(|e| RetentionError::io(path, e))?;
        offset += (PAGE_HEADER + segments) as u64 + body;
    }
    found.ok_or_else(|| RetentionError::Unreadable {
        path: path.to_path_buf(),
        reason: "nenhum page de áudio".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ogg::{PacketWriteEndInfo, PacketWriter};
    use opus::{Application, Encoder};
    use std::io::BufWriter;

    const FRAME: u64 = 960;
    const HOUR: u64 = 3_600 * SAMPLE_RATE_HZ as u64;

    /// Um Ogg Opus de `frames` quadros de silêncio, 50 pacotes por page como o `retain_wav`.
    /// O mesmo pacote codificado se repete: duas horas cabem em poucos MB e um segundo.
    fn silent_stream(path: &Path, frames: u64) -> u64 {
        let mut encoder = Encoder::new(SAMPLE_RATE_HZ, Channels::Mono, Application::Voip).unwrap();
        let pre_skip = encoder.get_lookahead().unwrap() as u64;
        let mut packet = vec![0u8; 4000];
        let len = encoder
            .encode(&[0i16; FRAME as usize], &mut packet)
            .unwrap();
        packet.truncate(len);

        let mut writer = PacketWriter::new(BufWriter::new(File::create(path).unwrap()));
        let mut head = b"OpusHead".to_vec();
        head.extend_from_slice(&[1, 1]);
        head.extend_from_slice(&(pre_skip as u16).to_le_bytes());
        head.extend_from_slice(&SAMPLE_RATE_HZ.to_le_bytes());
        head.extend_from_slice(&[0, 0, 0]);
        writer
            .write_packet(head, 1, PacketWriteEndInfo::EndPage, 0)
            .unwrap();
        let mut tags = b"OpusTags".to_vec();
        tags.extend_from_slice(&[0; 8]);
        writer
            .write_packet(tags, 1, PacketWriteEndInfo::EndPage, 0)
            .unwrap();
        for i in 1..=frames {
            let info = if i == frames {
                PacketWriteEndInfo::EndStream
            } else if i % 50 == 0 {
                PacketWriteEndInfo::EndPage
            } else {
                PacketWriteEndInfo::NormalPacket
            };
            writer
                .write_packet(packet.clone(), 1, info, i * FRAME)
                .unwrap();
        }
        writer
            .into_inner()
            .into_inner()
            .unwrap()
            .sync_all()
            .unwrap();
        pre_skip
    }

    #[test]
    fn two_hours_in_decodes_only_a_few_seconds() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("longo.opus");
        let frames = 2 * HOUR / FRAME + 50;
        let pre_skip = silent_stream(&path, frames);
        let total = frames * FRAME - pre_skip;

        let start = 2 * HOUR - SAMPLE_RATE_HZ as u64;
        let (pcm, decoded) =
            decode_range_counted(&path, start, start + SAMPLE_RATE_HZ as u64).unwrap();
        assert_eq!(pcm.len(), SAMPLE_RATE_HZ as usize);
        // 1 s de trecho + 1 s de pré-rolagem + até 1 page de recuo, em pacotes de 20 ms.
        assert!(decoded <= 150, "{decoded} pacotes decodificados");
        assert!(decoded >= 50, "{decoded} pacotes decodificados");

        let (tail, _) = decode_range_counted(&path, total - 100, total + 10_000).unwrap();
        assert_eq!(tail.len(), 100, "o fim é recortado no granule final");
        let (beyond, decoded) = decode_range_counted(&path, total + 1, total + 9_000).unwrap();
        assert!(beyond.is_empty());
        assert!(decoded <= 100, "{decoded} pacotes depois do fim");
    }

    #[test]
    fn empty_range_reads_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nao-existe.opus");
        assert_eq!(decode_range(&path, 10, 10).unwrap(), Vec::<i16>::new());
        assert!(matches!(
            decode_range(&path, 0, 10),
            Err(RetentionError::Io { .. })
        ));
    }
}
