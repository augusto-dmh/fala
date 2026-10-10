//! Microfone pelo `cpal`: mono f32 na taxa do dispositivo, num ring lock-free.
//!
//! Sem `cfg(target_os)`: o `cpal` escolhe o host (ALSA/PipeWire no Linux, WASAPI no Windows).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
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

        let default = device
            .default_input_config()
            .map_err(|e| AudioError::Device(e.to_string()))?;
        let config = if default.sample_format() == SampleFormat::F32 {
            default.config()
        } else {
            let rate = default.sample_rate();
            device
                .supported_input_configs()
                .map_err(|e| AudioError::Device(e.to_string()))?
                .find(|r| {
                    r.sample_format() == SampleFormat::F32
                        && r.min_sample_rate() <= rate
                        && r.max_sample_rate() >= rate
                })
                .map(|r| r.with_sample_rate(rate).config())
                .ok_or_else(|| {
                    AudioError::UnsupportedConfig(format!(
                        "`{name}` não oferece f32 a {} Hz",
                        rate.0
                    ))
                })?
        };
        let configured = started.elapsed();
        let rate = config.sample_rate.0;
        let channels = usize::from(config.channels).max(1);
        let channel = channel.filter(|c| *c < channels);

        let (producer, consumer) = RingBuffer::new(rate as usize * RING_SECONDS);
        let dropped = Arc::new(AtomicU64::new(0));
        let failed = Arc::new(AtomicBool::new(false));
        let stream = build(
            &device,
            &config,
            producer,
            (channels, channel),
            Arc::clone(&dropped),
            Arc::clone(&failed),
        )?;
        stream
            .play()
            .map_err(|e| AudioError::Stream(e.to_string()))?;
        log::info!("microfone: {name}, {rate} Hz, {channels} canal(is)");
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

/// Um quadro intercalado reduzido a mono: o canal escolhido ou a média.
fn mono(frame: &[f32], channel: Option<usize>) -> f32 {
    match channel.and_then(|c| frame.get(c)) {
        Some(x) => *x,
        None => frame.iter().sum::<f32>() / frame.len().max(1) as f32,
    }
}

fn build(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut producer: Producer<f32>,
    (channels, channel): (usize, Option<usize>),
    dropped: Arc<AtomicU64>,
    failed: Arc<AtomicBool>,
) -> Result<Stream, AudioError> {
    device
        .build_input_stream(
            config,
            move |data: &[f32], _| {
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

    #[test]
    fn mono_picks_channel_or_averages() {
        let frame = [0.1, 0.5, 0.9];
        assert_eq!(mono(&frame, Some(1)), 0.5);
        assert!((mono(&frame, Some(3)) - 0.5).abs() < 1e-6);
        assert!((mono(&frame, None) - 0.5).abs() < 1e-6);
        assert_eq!(mono(&[0.2], Some(0)), 0.2);
    }
}
