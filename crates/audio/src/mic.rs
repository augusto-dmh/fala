//! Microfone pelo `cpal`: mono f32 na taxa do dispositivo, num ring lock-free. O stream abre no
//! formato nativo do dispositivo (o mix format do WASAPI) e cada amostra vira f32 no callback.
//!
//! Sem `cfg(target_os)`: o `cpal` escolhe o host (ALSA/PipeWire no Linux, WASAPI no Windows).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, Stream};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::AudioError;

/// Segundos de folga no ring entre o callback e quem consome.
const RING_SECONDS: usize = 2;

/// Stream de entrada aberto e rodando. Fechar = soltar o valor.
pub struct Mic {
    _stream: Stream,
    consumer: Consumer<f32>,
    rate: u32,
    name: String,
    dropped: Arc<AtomicU64>,
    failed: Arc<AtomicBool>,
}

impl Mic {
    /// Confere que alguma entrada tem `needle` no nome, sem abrir stream.
    pub fn check(needle: &str) -> Result<(), AudioError> {
        find(&cpal::default_host(), needle).map(|_| ())
    }

    /// Abre a entrada cujo nome contém `needle`, ou a entrada padrão.
    pub fn open(needle: Option<&str>) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = match needle {
            Some(needle) => find(&host, needle)?,
            None => host
                .default_input_device()
                .ok_or_else(|| AudioError::Device("não há entrada padrão".to_owned()))?,
        };
        Self::open_device(device, None)
    }

    /// Abre um dispositivo já resolvido. Com `channel` dentro do alcance, entrega só esse canal;
    /// fora dele ou sem `channel`, a média dos canais.
    pub fn open_device(device: cpal::Device, channel: Option<usize>) -> Result<Self, AudioError> {
        // Nunca abrir o mic enquanto as variáveis do monitor do PipeWire estão no ambiente.
        let _guard = crate::meeting::ENV_OPEN
            .lock()
            .map_err(|_| AudioError::Device("trava do ambiente envenenada".to_owned()))?;
        let started = std::time::Instant::now();
        let name = device.name().unwrap_or_default();

        // O formato nativo, nunca um f32 forçado: com os efeitos do driver ligados (Realtek
        // "Voice clarity"), um cliente WASAPI em f32 recebe só zeros sem erro nenhum, enquanto o
        // mix format traz o sinal (cjpais/Handy#2141).
        let default = device
            .default_input_config()
            .map_err(|e| AudioError::Device(e.to_string()))?;
        let format = default.sample_format();
        let config = default.config();
        let configured = started.elapsed();
        let rate = config.sample_rate.0;
        let channels = usize::from(config.channels).max(1);
        let channel = channel.filter(|c| *c < channels);

        let (producer, consumer) = RingBuffer::new(rate as usize * RING_SECONDS);
        let dropped = Arc::new(AtomicU64::new(0));
        let failed = Arc::new(AtomicBool::new(false));
        let dropped_cb = Arc::clone(&dropped);
        let failed_cb = Arc::clone(&failed);
        let layout = (channels, channel);
        let stream = match format {
            SampleFormat::I8 => {
                build::<i8>(&device, &config, producer, layout, dropped_cb, failed_cb)
            }
            SampleFormat::I16 => {
                build::<i16>(&device, &config, producer, layout, dropped_cb, failed_cb)
            }
            SampleFormat::I32 => {
                build::<i32>(&device, &config, producer, layout, dropped_cb, failed_cb)
            }
            SampleFormat::I64 => {
                build::<i64>(&device, &config, producer, layout, dropped_cb, failed_cb)
            }
            SampleFormat::U8 => {
                build::<u8>(&device, &config, producer, layout, dropped_cb, failed_cb)
            }
            SampleFormat::U16 => {
                build::<u16>(&device, &config, producer, layout, dropped_cb, failed_cb)
            }
            SampleFormat::U32 => {
                build::<u32>(&device, &config, producer, layout, dropped_cb, failed_cb)
            }
            SampleFormat::U64 => {
                build::<u64>(&device, &config, producer, layout, dropped_cb, failed_cb)
            }
            SampleFormat::F32 => {
                build::<f32>(&device, &config, producer, layout, dropped_cb, failed_cb)
            }
            SampleFormat::F64 => {
                build::<f64>(&device, &config, producer, layout, dropped_cb, failed_cb)
            }
            other => Err(AudioError::UnsupportedConfig(format!(
                "`{name}` entrega amostras em {other:?}, que o Fala não converte"
            ))),
        }?;
        stream
            .play()
            .map_err(|e| AudioError::Stream(e.to_string()))?;
        log::info!("microfone: {name}, {rate} Hz, {channels} canal(is), {format:?}");
        log::debug!(
            "microfone: config {configured:?}, stream {:?}",
            started.elapsed() - configured
        );
        Ok(Self {
            _stream: stream,
            consumer,
            rate,
            name,
            dropped,
            failed,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.rate
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Amostras descartadas porque o ring encheu.
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// O `cpal` reportou erro no stream (dispositivo removido, por exemplo): reabra.
    pub fn failed(&self) -> bool {
        self.failed.load(Ordering::Relaxed)
    }

    /// Move para `out` tudo o que o callback já entregou.
    pub fn drain_into(&mut self, out: &mut Vec<f32>) {
        while let Ok(x) = self.consumer.pop() {
            out.push(x);
        }
    }
}

fn find(host: &cpal::Host, needle: &str) -> Result<cpal::Device, AudioError> {
    let devices: Vec<cpal::Device> = host
        .input_devices()
        .map_err(|e| AudioError::Device(e.to_string()))?
        .collect();
    let names: Vec<String> = devices
        .iter()
        .map(|d| d.name().unwrap_or_default())
        .collect();
    let index = names
        .iter()
        .position(|n| n.contains(needle))
        .ok_or_else(|| AudioError::NoDevice {
            needle: needle.to_owned(),
            available: names.clone(),
        })?;
    devices
        .into_iter()
        .nth(index)
        .ok_or_else(|| AudioError::Device("dispositivo sumiu da lista".to_owned()))
}

/// Um quadro intercalado reduzido a mono f32 no intervalo [-1, 1]: o canal escolhido ou a
/// média, cada amostra convertida do formato nativo.
fn mono<T>(frame: &[T], channel: Option<usize>) -> f32
where
    T: Sample,
    f32: FromSample<T>,
{
    match channel.and_then(|c| frame.get(c)) {
        Some(&x) => f32::from_sample(x),
        None => {
            let sum: f32 = frame.iter().map(|&s| f32::from_sample(s)).sum();
            sum / frame.len().max(1) as f32
        }
    }
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut producer: Producer<f32>,
    (channels, channel): (usize, Option<usize>),
    dropped: Arc<AtomicU64>,
    failed: Arc<AtomicBool>,
) -> Result<Stream, AudioError>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                let mut lost = 0u64;
                for frame in data.chunks_exact(channels) {
                    if producer.push(mono(frame, channel)).is_err() {
                        lost += 1;
                    }
                }
                if lost > 0 {
                    dropped.fetch_add(lost, Ordering::Relaxed);
                }
            },
            move |e| {
                log::warn!("stream do microfone: {e}");
                failed.store(true, Ordering::Relaxed);
            },
            None,
        )
        .map_err(|e| AudioError::Stream(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::mono;

    fn close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-3,
            "{actual} longe de {expected}"
        );
    }

    #[test]
    fn mono_picks_channel_or_averages() {
        let frame = [0.1f32, 0.5, 0.9];
        assert_eq!(mono(&frame, Some(1)), 0.5);
        assert!((mono(&frame, Some(3)) - 0.5).abs() < 1e-6);
        assert!((mono(&frame, None) - 0.5).abs() < 1e-6);
        assert_eq!(mono(&[0.2f32], Some(0)), 0.2);
    }

    #[test]
    fn mono_converts_every_native_format_to_f32() {
        close(mono(&[0.5f32, -0.25], None), 0.125);
        close(mono(&[i16::MAX, i16::MAX], None), 1.0);
        close(mono(&[i16::MIN], None), -1.0);
        close(mono(&[16_384i16, 0], None), 0.25);
        close(mono(&[i32::MIN, i32::MIN], None), -1.0);
        close(mono(&[1_073_741_824i32], None), 0.5);
        close(mono(&[128u8, 128], None), 0.0);
        close(mono(&[255u8], None), 127.0 / 128.0);
        close(mono(&[0.5f64, 0.5], None), 0.5);
        close(mono(&[0i16, 16_384], Some(1)), 0.5);
    }

    #[test]
    fn mono_keeps_a_signal_that_is_not_silence() {
        // O sintoma do bug era um buffer só de zeros; um sinal inteiro precisa sobreviver.
        let frame = [8_192i16, 8_192];
        assert!(mono(&frame, None).abs() > 0.2);
    }
}
