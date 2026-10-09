//! O áudio do sistema como entrada: monitor do sink pelo plugin ALSA do PipeWire (ADR-0013) ou
//! dispositivo de saída aberto como entrada (loopback do WASAPI).
//!
//! Sem `cfg` de sistema (ADR-0007): o host do `cpal` decide em tempo de execução, como no spike 04.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use rtrb::{Consumer, RingBuffer};

use crate::AudioError;

/// `PIPEWIRE_NODE`/`PIPEWIRE_ALSA` valem para o processo inteiro: quem abre um stream com elas
/// segura esta trava (ADR-0013).
pub(crate) static ENV_OPEN: Mutex<()> = Mutex::new(());
const RING_SECONDS: usize = 2;

/// Stream do áudio do sistema aberto e rodando. Fechar = soltar o valor.
pub struct SystemAudio {
    _stream: Stream,
    consumer: Consumer<f32>,
    rate: u32,
    dropped: Arc<AtomicU64>,
}

fn is_alsa(host: &cpal::Host) -> bool {
    host.id().name() == "ALSA"
}

impl SystemAudio {
    /// Confere que `name` existe, sem abrir stream: no ALSA, o `node.name` de um nó do PipeWire
    /// (pela saída de `pw-dump`); nos outros hosts, parte do nome de um dispositivo de saída.
    pub fn check(name: &str) -> Result<(), AudioError> {
        let host = cpal::default_host();
        if is_alsa(&host) {
            let out = Command::new("pw-dump")
                .output()
                .map_err(|e| AudioError::Device(format!("pw-dump: {e}")))?;
            let dump = String::from_utf8_lossy(&out.stdout);
            let mut sinks: Vec<String> = Vec::new();
            for line in dump.lines() {
                if let Some(rest) = line.trim().strip_prefix("\"node.name\": \"") {
                    let node = rest.trim_end_matches(',').trim_end_matches('"');
                    if node == name {
                        return Ok(());
                    }
                    sinks.push(node.to_owned());
                }
            }
            return Err(AudioError::NoSystem {
                name: name.to_owned(),
                available: sinks,
            });
        }
        find_output(&host, name).map(|_| ())
    }

    /// O nome da saída padrão, no formato que [`SystemAudio::open`] aceita: no ALSA, o
    /// `node.name` do sink padrão do PipeWire (pela saída de `pw-metadata`, das mesmas
    /// ferramentas do `pw-dump` que o [`SystemAudio::check`] já usa); nos
    /// outros hosts, o nome do dispositivo de saída padrão.
    pub fn default_name() -> Result<String, AudioError> {
        let host = cpal::default_host();
        if is_alsa(&host) {
            let out = Command::new("pw-metadata")
                .args(["0", "default.audio.sink"])
                .output()
                .map_err(|e| AudioError::Device(format!("pw-metadata: {e}")))?;
            return parse_default_sink(&String::from_utf8_lossy(&out.stdout)).ok_or_else(|| {
                AudioError::Device("o PipeWire não informou um sink padrão".to_owned())
            });
        }
        host.default_output_device()
            .and_then(|d| d.name().ok())
            .filter(|n| !n.is_empty())
            .ok_or_else(|| AudioError::Device("não há saída padrão".to_owned()))
    }

    pub fn open(name: &str) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let (producer, consumer) = RingBuffer::new(192_000 * RING_SECONDS);
        let dropped = Arc::new(AtomicU64::new(0));
        let (stream, rate) = if is_alsa(&host) {
            with_monitor_env(name, || {
                host.input_devices()
                    .map_err(|e| AudioError::Device(e.to_string()))
                    .and_then(|mut devices| {
                        devices
                            .find(|d| d.name().is_ok_and(|n| n == "pipewire"))
                            .ok_or_else(|| {
                                AudioError::Device(
                                    "o ALSA não tem o dispositivo `pipewire` (plugin pipewire-alsa)"
                                        .to_owned(),
                                )
                            })
                    })
                    .and_then(|device| {
                        let config = f32_config(device.default_input_config())?;
                        build(&device, config, producer, Arc::clone(&dropped))
                    })
            })??
        } else {
            let device = find_output(&host, name)?;
            let config = f32_config(device.default_output_config())?;
            build(&device, config, producer, Arc::clone(&dropped))?
        };
        stream
            .play()
            .map_err(|e| AudioError::Stream(e.to_string()))?;
        log::info!("áudio do sistema: {name}, {rate} Hz");
        Ok(Self {
            _stream: stream,
            consumer,
            rate,
            dropped,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.rate
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

/// Roda `open` com `PIPEWIRE_NODE=<name>` e `PIPEWIRE_ALSA` no ambiente, segurando `ENV_OPEN`, e
/// remove as duas antes de soltar a trava, dê `open` certo ou errado.
fn with_monitor_env<T>(name: &str, open: impl FnOnce() -> T) -> Result<T, AudioError> {
    let _guard = ENV_OPEN
        .lock()
        .map_err(|_| AudioError::Device("trava do ambiente envenenada".to_owned()))?;
    // SAFETY: a trava serializa, dentro do processo, toda abertura de stream que mexe nestas
    // variáveis ou que poderia herdá-las (o `Mic::open` também a segura); o plugin pipewire-alsa
    // as lê ao abrir o PCM e nenhuma thread do `cpal` as relê depois. O mesmo mecanismo do
    // spike 04 (door 1 do `cli-record`, ADR-0013).
    unsafe {
        std::env::set_var("PIPEWIRE_NODE", name);
        std::env::set_var("PIPEWIRE_ALSA", "{ stream.capture.sink = true }");
    }
    let opened = open();
    // SAFETY: como acima; o PCM já abriu (ou falhou), e o mic que vier depois não pode herdar o
    // alvo do monitor.
    unsafe {
        std::env::remove_var("PIPEWIRE_NODE");
        std::env::remove_var("PIPEWIRE_ALSA");
    }
    Ok(opened)
}

/// O `name` do valor JSON de `default.audio.sink` na saída de `pw-metadata`, numa linha como
/// `update: id:0 key:'default.audio.sink' value:'{"name":"alsa_output..."}' type:'...'`.
fn parse_default_sink(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .filter(|line| line.contains("key:'default.audio.sink'"))
        .find_map(|line| {
            let rest = &line[line.find("\"name\":\"")? + "\"name\":\"".len()..];
            let name = &rest[..rest.find('"')?];
            (!name.is_empty()).then(|| name.to_owned())
        })
}

fn find_output(host: &cpal::Host, name: &str) -> Result<cpal::Device, AudioError> {
    let devices: Vec<cpal::Device> = host
        .output_devices()
        .map_err(|e| AudioError::Device(e.to_string()))?
        .collect();
    let names: Vec<String> = devices
        .iter()
        .map(|d| d.name().unwrap_or_default())
        .collect();
    let index =
        names
            .iter()
            .position(|n| n.contains(name))
            .ok_or_else(|| AudioError::NoSystem {
                name: name.to_owned(),
                available: names.clone(),
            })?;
    devices
        .into_iter()
        .nth(index)
        .ok_or_else(|| AudioError::Device("dispositivo sumiu da lista".to_owned()))
}

fn f32_config(
    default: Result<cpal::SupportedStreamConfig, cpal::DefaultStreamConfigError>,
) -> Result<(cpal::StreamConfig, u32), AudioError> {
    let default = default.map_err(|e| AudioError::Device(e.to_string()))?;
    if default.sample_format() != SampleFormat::F32 {
        return Err(AudioError::UnsupportedConfig(format!(
            "o áudio do sistema vem em {:?}, não f32",
            default.sample_format()
        )));
    }
    let rate = default.sample_rate().0;
    Ok((default.config(), rate))
}

fn build(
    device: &cpal::Device,
    (config, rate): (cpal::StreamConfig, u32),
    mut producer: rtrb::Producer<f32>,
    dropped: Arc<AtomicU64>,
) -> Result<(Stream, u32), AudioError> {
    let channels = usize::from(config.channels).max(1);
    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _| {
                let mut lost = 0u64;
                for frame in data.chunks_exact(channels) {
                    let mono = frame.iter().sum::<f32>() / channels as f32;
                    if producer.push(mono).is_err() {
                        lost += 1;
                    }
                }
                if lost > 0 {
                    dropped.fetch_add(lost, Ordering::Relaxed);
                }
            },
            |e| log::warn!("stream do sistema: {e}"),
            None,
        )
        .map_err(|e| AudioError::Stream(e.to_string()))?;
    Ok((stream, rate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_sink_from_pw_metadata() {
        let out = "Found \"default\" metadata 40\n\
            update: id:0 key:'default.audio.sink' \
            value:'{\"name\":\"alsa_output.pci-0000_00_1f.3.analog-stereo\"}' type:'Spa:String:JSON'\n";
        assert_eq!(
            parse_default_sink(out).as_deref(),
            Some("alsa_output.pci-0000_00_1f.3.analog-stereo")
        );
        assert_eq!(parse_default_sink("Found \"default\" metadata 40\n"), None);
        assert_eq!(parse_default_sink(""), None);
    }

    #[test]
    fn monitor_env_is_set_only_while_opening_under_the_lock() {
        let seen = with_monitor_env("sink-de-teste", || {
            (
                std::env::var("PIPEWIRE_NODE").ok(),
                std::env::var("PIPEWIRE_ALSA").ok(),
                ENV_OPEN.try_lock().is_err(),
            )
        })
        .unwrap();
        assert_eq!(seen.0.as_deref(), Some("sink-de-teste"));
        assert_eq!(seen.1.as_deref(), Some("{ stream.capture.sink = true }"));
        assert!(seen.2, "a trava devia estar presa durante a abertura");
        assert!(std::env::var_os("PIPEWIRE_NODE").is_none());
        assert!(std::env::var_os("PIPEWIRE_ALSA").is_none());
        assert!(
            ENV_OPEN.try_lock().is_ok(),
            "a trava devia estar solta depois"
        );
    }
}
