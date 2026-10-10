//! Microfone pelo `cpal`: mono f32 na taxa do dispositivo, num ring lock-free. O stream abre no
//! formato nativo do dispositivo (o mix format do WASAPI) e cada amostra vira f32 no callback.
//!
//! Sem `cfg(target_os)`: o `cpal` escolhe o host (ALSA/PipeWire no Linux, WASAPI no Windows).

use std::sync::atomic::{AtomicU64, Ordering};
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
}

impl Mic {
    /// Confere que alguma entrada tem `needle` no nome, sem abrir stream.
    pub fn check(needle: &str) -> Result<(), AudioError> {
        find(&cpal::default_host(), needle).map(|_| ())
    }

    /// Abre a entrada cujo nome contém `needle`, ou a entrada padrão.
    pub fn open(needle: Option<&str>) -> Result<Self, AudioError> {
        // Nunca abrir o mic enquanto as variáveis do monitor do PipeWire estão no ambiente.
        let _guard = crate::meeting::ENV_OPEN
            .lock()
            .map_err(|_| AudioError::Device("trava do ambiente envenenada".to_owned()))?;
        let host = cpal::default_host();
        let device = match needle {
            Some(needle) => find(&host, needle)?,
            None => host
                .default_input_device()
                .ok_or_else(|| AudioError::Device("não há entrada padrão".to_owned()))?,
        };
        let name = device.name().unwrap_or_default();

        // O formato nativo, nunca um f32 forçado: com os efeitos do driver ligados (Realtek
        // "Voice clarity"), um cliente WASAPI em f32 recebe só zeros sem erro nenhum, enquanto o
        // mix format traz o sinal (cjpais/Handy#2141).
        let default = device
            .default_input_config()
            .map_err(|e| AudioError::Device(e.to_string()))?;
        let format = default.sample_format();
        let config = default.config();
        let rate = config.sample_rate.0;
        let channels = usize::from(config.channels).max(1);

        let (producer, consumer) = RingBuffer::new(rate as usize * RING_SECONDS);
        let dropped = Arc::new(AtomicU64::new(0));
        let dropped_cb = Arc::clone(&dropped);
        let stream = match format {
            SampleFormat::I8 => build::<i8>(&device, &config, producer, channels, dropped_cb),
            SampleFormat::I16 => build::<i16>(&device, &config, producer, channels, dropped_cb),
            SampleFormat::I32 => build::<i32>(&device, &config, producer, channels, dropped_cb),
            SampleFormat::I64 => build::<i64>(&device, &config, producer, channels, dropped_cb),
            SampleFormat::U8 => build::<u8>(&device, &config, producer, channels, dropped_cb),
            SampleFormat::U16 => build::<u16>(&device, &config, producer, channels, dropped_cb),
            SampleFormat::U32 => build::<u32>(&device, &config, producer, channels, dropped_cb),
            SampleFormat::U64 => build::<u64>(&device, &config, producer, channels, dropped_cb),
            SampleFormat::F32 => build::<f32>(&device, &config, producer, channels, dropped_cb),
            SampleFormat::F64 => build::<f64>(&device, &config, producer, channels, dropped_cb),
            other => Err(AudioError::UnsupportedConfig(format!(
                "`{name}` entrega amostras em {other:?}, que o Fala não converte"
            ))),
        }?;
        stream
            .play()
            .map_err(|e| AudioError::Stream(e.to_string()))?;
        log::info!("microfone: {name}, {rate} Hz, {channels} canal(is), {format:?}");
        Ok(Self {
            _stream: stream,
            consumer,
            rate,
            name,
            dropped,
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

/// Média dos canais de um quadro, já em f32 no intervalo [-1, 1].
fn downmix<T>(frame: &[T]) -> f32
where
    T: Sample,
    f32: FromSample<T>,
{
    let sum: f32 = frame.iter().map(|&s| f32::from_sample(s)).sum();
    sum / frame.len().max(1) as f32
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut producer: Producer<f32>,
    channels: usize,
    dropped: Arc<AtomicU64>,
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
                    let mono = downmix(frame);
                    if producer.push(mono).is_err() {
                        lost += 1;
                    }
                }
                if lost > 0 {
                    dropped.fetch_add(lost, Ordering::Relaxed);
                }
            },
            |e| log::warn!("stream do microfone: {e}"),
            None,
        )
        .map_err(|e| AudioError::Stream(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::downmix;

    fn close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-3,
            "{actual} longe de {expected}"
        );
    }

    #[test]
    fn downmix_converts_every_native_format_to_f32() {
        close(downmix(&[0.5f32, -0.25]), 0.125);
        close(downmix(&[i16::MAX, i16::MAX]), 1.0);
        close(downmix(&[i16::MIN]), -1.0);
        close(downmix(&[16_384i16, 0]), 0.25);
        close(downmix(&[i32::MIN, i32::MIN]), -1.0);
        close(downmix(&[1_073_741_824i32]), 0.5);
        close(downmix(&[128u8, 128]), 0.0);
        close(downmix(&[255u8]), 127.0 / 128.0);
        close(downmix(&[0.5f64, 0.5]), 0.5);
    }

    #[test]
    fn downmix_keeps_a_signal_that_is_not_silence() {
        // O sintoma do bug era um buffer só de zeros; um sinal inteiro precisa sobreviver.
        let frame = [8_192i16, 8_192];
        assert!(downmix(&frame).abs() > 0.2);
    }
}
