//! As saídas de áudio: a lista para o seletor "Som da reunião" e o pico de cada uma.
//!
//! A lista vem do `cpal`, com os mesmos nomes que [`SystemAudio::open`] aceita. O pico vem do
//! `IAudioMeterInformation` de cada endpoint de render ativo no Windows, só leitura: nada abre um
//! stream nem grava. Nos outros SOs o medidor é `Device` até a fase 3 (ADR-0007: o `cfg` fica
//! neste crate de plataforma).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait};

use super::system::{is_alsa, SystemAudio};
use crate::AudioError;

/// Uma saída que o gravador pode abrir como loopback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputDevice {
    pub name: String,
    pub is_default: bool,
}

/// O maior pico de uma saída desde a última leitura, de 0 a 1.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputPeak {
    pub name: String,
    pub peak: f32,
}

/// Intervalo entre leituras: o medidor dá o pico do último período do motor (~10 ms), então ler
/// devagar perderia fala entre as leituras.
const POLL: Duration = Duration::from_millis(50);
/// De quanto em quanto tempo a lista de endpoints é relida (fone plugado no meio da call).
const REFRESH: Duration = Duration::from_secs(3);

impl SystemAudio {
    /// As saídas, com a padrão marcada. No ALSA só a padrão: o `cpal` não vê os nós do PipeWire
    /// (ADR-0013), e a troca de saída no Linux fica para a fase 3.
    pub fn outputs() -> Result<Vec<OutputDevice>, AudioError> {
        let default = SystemAudio::default_name().ok();
        let host = cpal::default_host();
        if is_alsa(&host) {
            return Ok(default
                .map(|name| OutputDevice {
                    name,
                    is_default: true,
                })
                .into_iter()
                .collect());
        }
        let devices = host
            .output_devices()
            .map_err(|e| AudioError::Device(e.to_string()))?;
        let names = devices.filter_map(|d| d.name().ok()).collect();
        Ok(mark_default(names, default.as_deref()))
    }
}

/// Os nomes sem vazios nem repetidos, na ordem do SO, com `default` marcado.
fn mark_default(names: Vec<String>, default: Option<&str>) -> Vec<OutputDevice> {
    let mut out: Vec<OutputDevice> = Vec::new();
    for name in names {
        if name.is_empty() || out.iter().any(|d| d.name == name) {
            continue;
        }
        out.push(OutputDevice {
            is_default: default == Some(name.as_str()),
            name,
        });
    }
    out
}

/// Junta `peaks` em `acc` guardando o maior pico de cada nome.
fn keep_max(acc: &mut Vec<OutputPeak>, peaks: impl IntoIterator<Item = (String, f32)>) {
    for (name, peak) in peaks {
        match acc.iter_mut().find(|p| p.name == name) {
            Some(p) => p.peak = p.peak.max(peak),
            None => acc.push(OutputPeak { name, peak }),
        }
    }
}

/// Lê o pico de todas as saídas ativas numa thread própria até ser solto.
pub struct OutputMeter {
    peaks: Arc<Mutex<Vec<OutputPeak>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl OutputMeter {
    /// Começa a ler. Falha no SO sem medidor ou se a primeira leitura falhar.
    pub fn start() -> Result<Self, AudioError> {
        let peaks = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("output-meter".to_owned())
            .spawn({
                let peaks = Arc::clone(&peaks);
                let stop = Arc::clone(&stop);
                move || {
                    let mut meters = match imp::Meters::open() {
                        Ok(meters) => {
                            let _ = ready_tx.send(Ok(()));
                            meters
                        }
                        Err(e) => {
                            let _ = ready_tx.send(Err(e));
                            return;
                        }
                    };
                    let mut since_refresh = Duration::ZERO;
                    while !stop.load(Ordering::Relaxed) {
                        let read = meters.read();
                        if let Ok(mut acc) = peaks.lock() {
                            keep_max(&mut acc, read);
                        }
                        std::thread::sleep(POLL);
                        since_refresh += POLL;
                        if since_refresh >= REFRESH {
                            since_refresh = Duration::ZERO;
                            if let Err(e) = meters.refresh() {
                                log::debug!("medidor das saídas: {e}");
                            }
                        }
                    }
                }
            })
            .map_err(|e| AudioError::Device(format!("thread do medidor: {e}")))?;
        ready_rx
            .recv()
            .map_err(|_| AudioError::Device("o medidor das saídas parou ao abrir".to_owned()))??;
        Ok(Self {
            peaks,
            stop,
            thread: Some(thread),
        })
    }

    /// O maior pico de cada saída desde a última chamada; zera a contagem.
    pub fn take(&self) -> Vec<OutputPeak> {
        self.peaks
            .lock()
            .map(|mut acc| std::mem::take(&mut *acc))
            .unwrap_or_default()
    }
}

impl Drop for OutputMeter {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(windows)]
mod imp {
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
    use windows::Win32::Media::Audio::{
        eRender, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
    };
    use windows::Win32::System::Com::StructuredStorage::PropVariantToBSTR;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
        STGM_READ,
    };

    use crate::AudioError;

    /// Os medidores dos endpoints de render ativos, com o COM da thread que os criou.
    pub(super) struct Meters {
        list: Vec<(String, IAudioMeterInformation)>,
        com: bool,
    }

    fn com_err(what: &str, e: windows::core::Error) -> AudioError {
        AudioError::Device(format!("{what}: {e}"))
    }

    impl Meters {
        pub(super) fn open() -> Result<Self, AudioError> {
            // SAFETY: sem ponteiro reservado; o par com `CoUninitialize` fica no `Drop`. A thread
            // é nossa, então o apartment é o que pedimos.
            let com = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
            let mut meters = Self {
                list: Vec::new(),
                com,
            };
            meters.refresh()?;
            Ok(meters)
        }

        pub(super) fn refresh(&mut self) -> Result<(), AudioError> {
            // SAFETY: chamadas COM na thread inicializada em `open`; as interfaces se soltam no
            // `Drop` delas.
            unsafe {
                let devices: IMMDeviceEnumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                        .map_err(|e| com_err("MMDeviceEnumerator", e))?;
                let collection = devices
                    .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
                    .map_err(|e| com_err("EnumAudioEndpoints", e))?;
                let count = collection.GetCount().map_err(|e| com_err("GetCount", e))?;
                let mut list = Vec::new();
                for i in 0..count {
                    let Ok(device) = collection.Item(i) else {
                        continue;
                    };
                    let Some(name) = friendly_name(&device) else {
                        continue;
                    };
                    let Ok(meter) = device.Activate::<IAudioMeterInformation>(CLSCTX_ALL, None)
                    else {
                        continue;
                    };
                    list.push((name, meter));
                }
                self.list = list;
            }
            Ok(())
        }

        /// O pico de agora de cada endpoint; um medidor que falha fica de fora.
        pub(super) fn read(&self) -> Vec<(String, f32)> {
            self.list
                .iter()
                // SAFETY: o medidor foi ativado nesta thread e vive em `self`.
                .filter_map(|(name, meter)| {
                    let peak = unsafe { meter.GetPeakValue() }.ok()?;
                    Some((name.clone(), peak))
                })
                .collect()
        }
    }

    /// O nome amigável do endpoint, o mesmo que o `cpal` dá ao dispositivo no WASAPI.
    fn friendly_name(device: &IMMDevice) -> Option<String> {
        // SAFETY: só chamado de `refresh`, numa thread com COM; o PROPVARIANT se limpa no `Drop`.
        unsafe {
            let store = device.OpenPropertyStore(STGM_READ).ok()?;
            let value = store.GetValue(&PKEY_Device_FriendlyName).ok()?;
            let name = PropVariantToBSTR(&value).ok()?.to_string();
            (!name.is_empty()).then_some(name)
        }
    }

    impl Drop for Meters {
        fn drop(&mut self) {
            self.list.clear();
            if self.com {
                // SAFETY: equilibra o `CoInitializeEx` que deu certo nesta mesma thread, depois
                // de soltar as interfaces.
                unsafe { CoUninitialize() };
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use crate::AudioError;

    pub(super) struct Meters;

    impl Meters {
        pub(super) fn open() -> Result<Self, AudioError> {
            Err(AudioError::Device(
                "o pico das saídas só é lido no Windows".to_owned(),
            ))
        }

        pub(super) fn refresh(&mut self) -> Result<(), AudioError> {
            Ok(())
        }

        pub(super) fn read(&self) -> Vec<(String, f32)> {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_marked_and_names_are_unique() {
        let names = ["Alto-falantes", "", "Fone USB", "Alto-falantes", "HDMI"]
            .map(String::from)
            .to_vec();
        let out = mark_default(names, Some("Fone USB"));
        let got: Vec<(&str, bool)> = out
            .iter()
            .map(|d| (d.name.as_str(), d.is_default))
            .collect();
        assert_eq!(
            got,
            [
                ("Alto-falantes", false),
                ("Fone USB", true),
                ("HDMI", false)
            ]
        );
        assert!(mark_default(vec!["A".into()], None)
            .iter()
            .all(|d| !d.is_default));
    }

    #[test]
    fn peaks_keep_the_max_per_name_until_taken() {
        let mut acc = Vec::new();
        keep_max(&mut acc, [("A".to_owned(), 0.2), ("B".to_owned(), 0.0)]);
        keep_max(&mut acc, [("A".to_owned(), 0.1), ("B".to_owned(), 0.5)]);
        assert_eq!(
            acc,
            [
                OutputPeak {
                    name: "A".into(),
                    peak: 0.2
                },
                OutputPeak {
                    name: "B".into(),
                    peak: 0.5
                }
            ]
        );
    }
}
