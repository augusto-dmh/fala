use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use ogg::{PacketReader, PacketWriteEndInfo, PacketWriter};
use opus::{Application, Bitrate, Channels, Decoder, Encoder};

use crate::{RetentionError, MIC_FILE, SYSTEM_FILE};

/// Taxa do WAV de trabalho e do Opus retido.
pub const SAMPLE_RATE_HZ: u32 = 48_000;
/// Alvo do encoder por canal (ADR-0014).
pub const BITRATE_BPS: i32 = 24_000;
/// 20 ms a 48 kHz.
const FRAME: usize = 960;
/// Maior pacote que o libopus devolve.
const MAX_PACKET: usize = 4000;
/// Maior quadro que um pacote decodifica (120 ms a 48 kHz).
const MAX_DECODED: usize = 5760;
/// Um page Ogg por segundo de áudio: dá pontos de busca para tocar um trecho (2.F9).
const PACKETS_PER_PAGE: usize = 50;
const VENDOR: &str = "fala-retention (libopus)";

/// Os dois arquivos Opus retidos de uma sessão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedAudio {
    pub mic: PathBuf,
    pub system: PathBuf,
    /// Amostras por canal, igual ao número de quadros do WAV.
    pub samples: u64,
}

/// Converte o WAV de trabalho em `mic.opus` (L) e `sys.opus` (R) dentro de `session_dir`,
/// valida os dois por decodificação e só então apaga o WAV.
///
/// Os arquivos são escritos como `.part` e renomeados depois da validação; uma tentativa
/// anterior interrompida é sobrescrita. Em qualquer erro o WAV continua no disco.
pub fn retain_wav(wav: &Path, session_dir: &Path) -> Result<RetainedAudio, RetentionError> {
    retain_wav_with(wav, session_dir, validate_opus)
}

/// `retain_wav` com o validador injetável, para provar que uma validação recusada mantém o WAV.
fn retain_wav_with(
    wav: &Path,
    session_dir: &Path,
    validate: impl Fn(&Path, u64) -> Result<(), RetentionError>,
) -> Result<RetainedAudio, RetentionError> {
    let reader = hound::WavReader::open(wav).map_err(|e| wav_error(wav, e))?;
    let spec = reader.spec();
    if spec.sample_rate != SAMPLE_RATE_HZ
        || spec.channels != 2
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
    {
        return Err(RetentionError::UnsupportedWav {
            sample_rate: spec.sample_rate,
            channels: spec.channels,
            bits: spec.bits_per_sample,
        });
    }

    fs::create_dir_all(session_dir).map_err(|e| RetentionError::io(session_dir, e))?;
    let mic = session_dir.join(MIC_FILE);
    let system = session_dir.join(SYSTEM_FILE);
    let mic_part = part_path(&mic);
    let system_part = part_path(&system);

    let samples = encode_channels(reader, wav, &mic_part, &system_part)?;
    validate(&mic_part, samples)?;
    validate(&system_part, samples)?;

    fs::rename(&mic_part, &mic).map_err(|e| RetentionError::io(&mic, e))?;
    fs::rename(&system_part, &system).map_err(|e| RetentionError::io(&system, e))?;
    sync_dir(session_dir);
    fs::remove_file(wav).map_err(|e| RetentionError::io(wav, e))?;

    Ok(RetainedAudio {
        mic,
        system,
        samples,
    })
}

/// Confere que `path` é um Ogg Opus mono de 48 kHz completo com exatamente
/// `expected_samples` amostras depois do pre-skip.
pub fn validate_opus(path: &Path, expected_samples: u64) -> Result<(), RetentionError> {
    let failed = |reason: String| RetentionError::ValidationFailed {
        path: path.to_path_buf(),
        reason,
    };
    let file = File::open(path).map_err(|e| RetentionError::io(path, e))?;
    let mut reader = PacketReader::new(BufReader::new(file));
    let mut next = || {
        reader
            .read_packet()
            .map_err(|e| failed(format!("Ogg ilegível: {e}")))
    };

    let head = next()?.ok_or_else(|| failed("sem OpusHead".into()))?;
    let pre_skip = parse_head(&head.data).map_err(&failed)?;
    let tags = next()?.ok_or_else(|| failed("sem OpusTags".into()))?;
    if !tags.data.starts_with(b"OpusTags") {
        return Err(failed("segundo pacote não é OpusTags".into()));
    }

    let mut decoder = Decoder::new(SAMPLE_RATE_HZ, Channels::Mono).map_err(opus_error)?;
    let mut pcm = vec![0i16; MAX_DECODED];
    let mut decoded: u64 = 0;
    let mut final_granule = None;
    while let Some(packet) = next()? {
        let n = decoder
            .decode(&packet.data, &mut pcm, false)
            .map_err(|e| failed(format!("pacote indecodificável: {e}")))?;
        decoded += n as u64;
        if packet.last_in_stream() {
            final_granule = Some(packet.absgp_page());
        }
    }

    let final_granule = final_granule.ok_or_else(|| failed("stream sem fim (EOS)".into()))?;
    let pre_skip = u64::from(pre_skip);
    let by_granule = final_granule.saturating_sub(pre_skip);
    if by_granule != expected_samples {
        return Err(failed(format!(
            "granule indica {by_granule} amostras, esperado {expected_samples}"
        )));
    }
    let by_decoding = decoded.saturating_sub(pre_skip);
    if by_decoding < expected_samples || by_decoding >= expected_samples + FRAME as u64 {
        return Err(failed(format!(
            "decodificadas {by_decoding} amostras, esperado {expected_samples}"
        )));
    }
    Ok(())
}

/// Lê o WAV em blocos de 20 ms e codifica L e R em dois streams Ogg Opus.
fn encode_channels(
    reader: hound::WavReader<BufReader<File>>,
    wav: &Path,
    mic_part: &Path,
    system_part: &Path,
) -> Result<u64, RetentionError> {
    let total = u64::from(reader.duration());
    let mut mic = OpusStream::create(mic_part, 1)?;
    let mut system = OpusStream::create(system_part, 2)?;

    // O libopus atrasa o sinal em `pre_skip` amostras: codifica quadros de zeros até cobrir
    // `total + pre_skip`, e o granule final corta o excesso (RFC 7845 §4).
    let target = total + mic.pre_skip;
    let mut samples = reader.into_samples::<i16>();
    let mut left = Vec::with_capacity(FRAME);
    let mut right = Vec::with_capacity(FRAME);
    let mut read: u64 = 0;
    let mut encoded: u64 = 0;
    loop {
        left.clear();
        right.clear();
        while left.len() < FRAME {
            let Some(l) = samples.next() else { break };
            let r = samples
                .next()
                .ok_or_else(|| wav_message(wav, "quadro estéreo incompleto"))?;
            left.push(l.map_err(|e| wav_error(wav, e))?);
            right.push(r.map_err(|e| wav_error(wav, e))?);
        }
        read += left.len() as u64;
        left.resize(FRAME, 0);
        right.resize(FRAME, 0);
        encoded += FRAME as u64;
        let last = encoded >= target;
        let granule = if last { target } else { encoded };
        mic.write_frame(&left, granule, last)?;
        system.write_frame(&right, granule, last)?;
        if last {
            break;
        }
    }
    if read != total || samples.next().is_some() {
        return Err(wav_message(
            wav,
            &format!("o cabeçalho diz {total} quadros e a leitura não bate"),
        ));
    }
    mic.finish(mic_part)?;
    system.finish(system_part)?;
    Ok(total)
}

/// Um stream Ogg Opus mono sendo escrito.
struct OpusStream {
    writer: PacketWriter<'static, BufWriter<File>>,
    encoder: Encoder,
    serial: u32,
    pre_skip: u64,
    packets: usize,
    out: Vec<u8>,
}

impl OpusStream {
    fn create(path: &Path, serial: u32) -> Result<Self, RetentionError> {
        let mut encoder = new_encoder()?;
        let pre_skip = encoder.get_lookahead().map_err(opus_error)?;
        let pre_skip = u16::try_from(pre_skip)
            .map_err(|_| RetentionError::Opus(format!("lookahead inesperado: {pre_skip}")))?;
        let file = File::create(path).map_err(|e| RetentionError::io(path, e))?;
        let mut writer = PacketWriter::new(BufWriter::new(file));

        let mut head = Vec::with_capacity(19);
        head.extend_from_slice(b"OpusHead");
        head.push(1); // versão
        head.push(1); // canais
        head.extend_from_slice(&pre_skip.to_le_bytes());
        head.extend_from_slice(&SAMPLE_RATE_HZ.to_le_bytes());
        head.extend_from_slice(&0i16.to_le_bytes()); // ganho de saída
        head.push(0); // mapeamento 0: mono/estéreo
        writer
            .write_packet(head, serial, PacketWriteEndInfo::EndPage, 0)
            .map_err(|e| RetentionError::io(path, e))?;

        let mut tags = Vec::new();
        tags.extend_from_slice(b"OpusTags");
        tags.extend_from_slice(&(VENDOR.len() as u32).to_le_bytes());
        tags.extend_from_slice(VENDOR.as_bytes());
        tags.extend_from_slice(&0u32.to_le_bytes()); // nenhum comentário
        writer
            .write_packet(tags, serial, PacketWriteEndInfo::EndPage, 0)
            .map_err(|e| RetentionError::io(path, e))?;

        Ok(Self {
            writer,
            encoder,
            serial,
            pre_skip: u64::from(pre_skip),
            packets: 0,
            out: vec![0u8; MAX_PACKET],
        })
    }

    /// Codifica um quadro de 20 ms; `granule` é a posição em amostras ao fim do pacote,
    /// contando o pre-skip.
    fn write_frame(
        &mut self,
        frame: &[i16],
        granule: u64,
        last: bool,
    ) -> Result<(), RetentionError> {
        let len = self
            .encoder
            .encode(frame, &mut self.out)
            .map_err(opus_error)?;
        self.packets += 1;
        let info = if last {
            PacketWriteEndInfo::EndStream
        } else if self.packets.is_multiple_of(PACKETS_PER_PAGE) {
            PacketWriteEndInfo::EndPage
        } else {
            PacketWriteEndInfo::NormalPacket
        };
        self.writer
            .write_packet(self.out[..len].to_vec(), self.serial, info, granule)
            .map_err(|e| RetentionError::Opus(format!("escrita Ogg: {e}")))
    }

    fn finish(self, path: &Path) -> Result<(), RetentionError> {
        let mut out = self.writer.into_inner();
        out.flush().map_err(|e| RetentionError::io(path, e))?;
        let file = out
            .into_inner()
            .map_err(|e| RetentionError::io(path, e.into_error()))?;
        // O WAV é apagado em seguida: o Opus precisa estar no disco, não no cache.
        file.sync_all().map_err(|e| RetentionError::io(path, e))
    }
}

fn new_encoder() -> Result<Encoder, RetentionError> {
    let mut encoder =
        Encoder::new(SAMPLE_RATE_HZ, Channels::Mono, Application::Voip).map_err(opus_error)?;
    encoder
        .set_bitrate(Bitrate::Bits(BITRATE_BPS))
        .map_err(opus_error)?;
    Ok(encoder)
}

/// Devolve o pre-skip de um `OpusHead` mono de 48 kHz.
fn parse_head(data: &[u8]) -> Result<u16, String> {
    if data.len() < 19 || !data.starts_with(b"OpusHead") {
        return Err("primeiro pacote não é OpusHead".into());
    }
    if data[9] != 1 {
        return Err(format!("{} canais, esperado 1", data[9]));
    }
    let rate = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
    if rate != SAMPLE_RATE_HZ {
        return Err(format!("{rate} Hz, esperado {SAMPLE_RATE_HZ}"));
    }
    Ok(u16::from_le_bytes([data[10], data[11]]))
}

/// Grava no disco os renomeios da pasta antes de o WAV sumir. Melhor esforço: onde o SO não
/// abre uma pasta como arquivo, os dados já estão sincronizados (`OpusStream::finish`) e o pior
/// caso de um crash aqui são `.part` íntegros com o WAV ainda no disco.
fn sync_dir(dir: &Path) {
    if let Ok(handle) = File::open(dir) {
        let _ = handle.sync_all();
    }
}

fn part_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".part");
    PathBuf::from(name)
}

fn opus_error(e: opus::Error) -> RetentionError {
    RetentionError::Opus(e.to_string())
}

fn wav_error(path: &Path, e: hound::Error) -> RetentionError {
    wav_message(path, &e.to_string())
}

fn wav_message(path: &Path, reason: &str) -> RetentionError {
    RetentionError::Wav {
        path: path.to_path_buf(),
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoder_targets_24_kbps() {
        let mut encoder = new_encoder().unwrap();
        assert_eq!(encoder.get_bitrate().unwrap(), Bitrate::Bits(24_000));
        assert_eq!(encoder.get_sample_rate().unwrap(), 48_000);
        assert!(encoder.get_vbr().unwrap());
    }

    fn stereo_wav(path: &Path, frames: u32) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: SAMPLE_RATE_HZ,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for i in 0..frames * 2 {
            writer.write_sample(((i % 200) as i16 - 100) * 50).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn rejected_validation_keeps_wav() {
        let tmp = tempfile::tempdir().unwrap();
        let wav = tmp.path().join("sessao.wav");
        let dir = tmp.path().join("audio").join("sessao");
        stereo_wav(&wav, SAMPLE_RATE_HZ);
        let size = fs::metadata(&wav).unwrap().len();
        let result = retain_wav_with(&wav, &dir, |path, _| {
            Err(RetentionError::ValidationFailed {
                path: path.to_path_buf(),
                reason: "recusado no teste".into(),
            })
        });
        assert!(matches!(
            result,
            Err(RetentionError::ValidationFailed { .. })
        ));
        assert_eq!(fs::metadata(&wav).unwrap().len(), size);
        assert!(!dir.join(MIC_FILE).exists());
        assert!(!dir.join(SYSTEM_FILE).exists());
    }

    #[test]
    fn decoded_count_below_granule_fails() {
        // Um stream com um só pacote de 20 ms cujo granule final promete 10 quadros:
        // o granule confere, a decodificação não.
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("curto.opus");
        let mut stream = OpusStream::create(&path, 1).unwrap();
        let expected = 10 * FRAME as u64;
        let granule = stream.pre_skip + expected;
        stream.write_frame(&[0i16; FRAME], granule, true).unwrap();
        stream.finish(&path).unwrap();
        match validate_opus(&path, expected) {
            Err(RetentionError::ValidationFailed { reason, .. }) => {
                assert!(reason.contains("decodificadas"), "{reason}");
                assert!(reason.contains(&expected.to_string()), "{reason}");
            }
            other => panic!("esperava ValidationFailed, veio {other:?}"),
        }
    }
}
