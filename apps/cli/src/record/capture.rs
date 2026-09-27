//! Gravação dupla: dois streams de entrada do `cpal` (mic e sistema) em rings lock-free, pareados
//! por índice num WAV estéreo sem ressincronizar, e um stream de saída que toca os cliques.
//!
//! Não há `cfg(target_os)` (ADR-0007): Linux e Windows se separam em tempo de execução pelo nome
//! do host do `cpal`. No ALSA, o sistema é o monitor do sink pelo plugin do PipeWire; nos outros
//! hosts, um dispositivo de saída aberto como entrada (loopback do WASAPI).

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, SampleRate, Stream, StreamConfig};
use rtrb::{Consumer, Producer, RingBuffer};

use super::{failed, input, print_table, Failure};

pub const RATE: u32 = 48_000;
/// 2 s por stream; o escritor acorda a cada 100 ms.
const RING_FRAMES: usize = 96_000;
/// Frames iniciais comparados para detectar o sistema caindo no microfone (2 s).
const FALLBACK_FRAMES: usize = 96_000;
const STALL: Duration = Duration::from_secs(5);
const TICK: Duration = Duration::from_millis(100);
const PROGRESS: Duration = Duration::from_secs(60);
const CLICK_AT: Duration = Duration::from_secs(2);

const COLUMNS: &str = "rate wall_s mic_frames sys_frames mic_ppm sys_ppm rel_drift_ms dropped_mic \
     dropped_sys stream_errors click_1_s click_2_s mic_peak mic_rms_dbfs sys_peak sys_rms_dbfs";

pub struct Options {
    pub out: PathBuf,
    pub duration: Duration,
    pub system: String,
    pub mic: Option<String>,
    pub click: bool,
    pub flush: Duration,
}

/// Contadores que o callback de um stream atualiza sem bloquear.
#[derive(Default)]
pub struct Counters {
    pub frames: AtomicU64,
    pub dropped: AtomicU64,
    pub errors: AtomicU64,
}

/// Mistura os canais de cada frame em mono e empurra no ring; o que não cabe é descartado e
/// contado, nunca esperado.
pub fn push_mono(producer: &mut Producer<f32>, data: &[f32], channels: usize, c: &Counters) {
    let mut dropped = 0u64;
    let mut frames = 0u64;
    for frame in data.chunks_exact(channels) {
        frames += 1;
        let mono = frame.iter().sum::<f32>() / channels as f32;
        if producer.push(mono).is_err() {
            dropped += 1;
        }
    }
    c.frames.fetch_add(frames, Ordering::Relaxed);
    c.dropped.fetch_add(dropped, Ordering::Relaxed);
}

/// Primeiro nome que contém `needle`; senão, erro listando todos.
pub fn pick_device(names: &[String], needle: &str, kind: &str) -> anyhow::Result<usize> {
    names
        .iter()
        .position(|n| n.contains(needle))
        .ok_or_else(|| {
            anyhow!(
                "nenhum dispositivo de {kind} contém `{needle}`; {kind}s disponíveis:\n  {}",
                names.join("\n  ")
            )
        })
}

/// Os 96 000 primeiros frames de L e R idênticos: a captura do sistema caiu no microfone.
pub fn fallback(l: &[i16], r: &[i16]) -> bool {
    l.len() >= FALLBACK_FRAMES
        && r.len() >= FALLBACK_FRAMES
        && l[..FALLBACK_FRAMES] == r[..FALLBACK_FRAMES]
}

/// Nomeia o stream cuja contagem não cresce há 5 s.
pub struct Watchdog {
    last: [(u64, Instant); 2],
}

impl Watchdog {
    pub fn new(now: Instant) -> Self {
        Watchdog {
            last: [(0, now), (0, now)],
        }
    }

    pub fn observe(&mut self, mic: u64, sys: u64, now: Instant) -> Option<&'static str> {
        for ((last, name), count) in self.last.iter_mut().zip(["mic", "system"]).zip([mic, sys]) {
            if count > last.0 {
                *last = (count, now);
            } else if now.duration_since(last.1) >= STALL {
                return Some(name);
            }
        }
        None
    }
}

pub fn ppm(frames: u64, rate: u32, wall_s: f64) -> f64 {
    (frames as f64 / (f64::from(rate) * wall_s) - 1.0) * 1e6
}

pub fn rel_drift_ms(mic: u64, sys: u64, rate: u32) -> f64 {
    (mic as f64 - sys as f64) / f64::from(rate) * 1000.0
}

/// Pico e soma dos quadrados de um canal, em escala cheia.
#[derive(Default)]
pub struct Level {
    peak: f64,
    sum_sq: f64,
    n: u64,
}

impl Level {
    fn add(&mut self, x: i16) {
        let v = f64::from(x) / 32768.0;
        self.peak = self.peak.max(v.abs());
        self.sum_sq += v * v;
        self.n += 1;
    }

    pub fn rms_dbfs(&self) -> f64 {
        if self.n == 0 {
            return f64::NEG_INFINITY;
        }
        20.0 * (self.sum_sq / self.n as f64).sqrt().log10()
    }
}

/// Burst senoidal de 1 000 Hz, 20 ms, amplitude 0.5.
pub fn click(rate: u32) -> Vec<f32> {
    let n = rate as usize / 50;
    (0..n)
        .map(|i| {
            let t = i as f32 / rate as f32;
            0.5 * (2.0 * std::f32::consts::PI * 1000.0 * t).sin()
        })
        .collect()
}

fn to_i16(x: f32) -> i16 {
    (x * 32768.0).round().clamp(-32768.0, 32767.0) as i16
}

/// O dispositivo cujo nome contém `needle` (ou é igual, com `exact`); senão, sai com 2 listando.
fn find_device(
    devices: Result<impl Iterator<Item = Device>, cpal::DevicesError>,
    needle: &str,
    exact: bool,
    kind: &str,
) -> Result<Device, Failure> {
    let mut devices: Vec<Device> = devices
        .context("não consegui listar os dispositivos")
        .map_err(input)?
        .collect();
    let names: Vec<String> = devices
        .iter()
        .map(|d| d.name().unwrap_or_default())
        .collect();
    let i = if exact {
        names.iter().position(|n| n == needle).ok_or_else(|| {
            anyhow!("o host não tem o dispositivo `{needle}` (no ALSA: plugin pipewire-alsa)")
        })
    } else {
        pick_device(&names, needle, kind)
    }
    .map_err(input)?;
    Ok(devices.swap_remove(i))
}

/// 48 kHz f32 com os canais padrão do dispositivo; sem isso, sai com 2 nomeando o que existe.
fn config_48k(device: &Device, loopback: bool) -> Result<StreamConfig, Failure> {
    let ranges: Vec<_> = if loopback {
        device.supported_output_configs().map(|c| c.collect())
    } else {
        device.supported_input_configs().map(|c| c.collect())
    }
    .context("não consegui listar as configurações do dispositivo")
    .map_err(input)?;
    let fits = ranges.iter().find(|r| {
        r.sample_format() == SampleFormat::F32
            && r.min_sample_rate() <= SampleRate(RATE)
            && r.max_sample_rate() >= SampleRate(RATE)
    });
    match fits {
        Some(range) => Ok(StreamConfig {
            channels: range.channels(),
            sample_rate: SampleRate(RATE),
            buffer_size: cpal::BufferSize::Default,
        }),
        None => Err(input(anyhow!(
            "o dispositivo não oferece 48000 Hz f32; oferece: {}",
            ranges
                .iter()
                .map(|r| format!(
                    "{}-{} Hz {:?} {} ch",
                    r.min_sample_rate().0,
                    r.max_sample_rate().0,
                    r.sample_format(),
                    r.channels()
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

fn input_stream(
    device: &Device,
    config: &StreamConfig,
    mut producer: Producer<f32>,
    counters: Arc<Counters>,
    name: &'static str,
) -> Result<Stream, Failure> {
    let channels = usize::from(config.channels);
    let on_error = Arc::clone(&counters);
    device
        .build_input_stream(
            config,
            move |data: &[f32], _| push_mono(&mut producer, data, channels, &counters),
            move |e| {
                on_error.errors.fetch_add(1, Ordering::Relaxed);
                log::warn!("stream {name}: {e}");
            },
            None,
        )
        .with_context(|| format!("não consegui abrir o stream {name}"))
        .map_err(failed)
}

/// Stream de saída que toca um clique a cada incremento de `pending`.
fn click_stream(device: &Device, pending: Arc<AtomicUsize>) -> Result<Stream, Failure> {
    let no_click = |e: anyhow::Error| {
        input(e.context("a saída padrão não abre para o clique; --no-click pula o clique"))
    };
    let supported = device
        .default_output_config()
        .map_err(|e| no_click(e.into()))?;
    let config = supported.config();
    let channels = usize::from(config.channels);
    let burst = click(config.sample_rate.0);
    let mut pos = burst.len();
    device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _| {
                for frame in data.chunks_exact_mut(channels) {
                    if pos >= burst.len()
                        && pending
                            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |p| p.checked_sub(1))
                            .is_ok()
                    {
                        pos = 0;
                    }
                    let x = burst.get(pos).copied().unwrap_or(0.0);
                    pos = pos.saturating_add(1);
                    frame.fill(x);
                }
            },
            |e| log::warn!("stream do clique: {e}"),
            None,
        )
        .map_err(|e| no_click(e.into()))
}

fn drain(consumer: &mut Consumer<f32>, queue: &mut VecDeque<i16>) {
    while let Ok(x) = consumer.pop() {
        queue.push_back(to_i16(x));
    }
}

/// O WAV (door 2) e o que se mede do que foi escrito nele.
struct Sink {
    wav: hound::WavWriter<std::io::BufWriter<std::fs::File>>,
    mic_level: Level,
    sys_level: Level,
    head_l: Vec<i16>,
    head_r: Vec<i16>,
}

impl Sink {
    fn create(path: &std::path::Path) -> Result<Self, Failure> {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let wav = hound::WavWriter::create(path, spec)
            .with_context(|| format!("não consegui criar {}", path.display()))
            .map_err(input)?;
        Ok(Sink {
            wav,
            mic_level: Level::default(),
            sys_level: Level::default(),
            head_l: Vec::with_capacity(FALLBACK_FRAMES),
            head_r: Vec::with_capacity(FALLBACK_FRAMES),
        })
    }

    /// Pareia L = mic e R = sistema por índice de chegada, sem ressincronizar. Com `pad`, o lado
    /// que acabou antes vira zero.
    fn write_pairs(
        &mut self,
        mic: &mut VecDeque<i16>,
        sys: &mut VecDeque<i16>,
        pad: bool,
    ) -> anyhow::Result<()> {
        loop {
            let (l, r) = match (mic.pop_front(), sys.pop_front()) {
                (Some(l), Some(r)) => (l, r),
                (None, None) => return Ok(()),
                (l, r) if pad => (l.unwrap_or(0), r.unwrap_or(0)),
                (l, r) => {
                    // Sem par ainda: devolve para a próxima volta.
                    if let Some(l) = l {
                        mic.push_front(l);
                    }
                    if let Some(r) = r {
                        sys.push_front(r);
                    }
                    return Ok(());
                }
            };
            self.wav.write_sample(l).context("escrita do WAV")?;
            self.wav.write_sample(r).context("escrita do WAV")?;
            self.mic_level.add(l);
            self.sys_level.add(r);
            if self.head_l.len() < FALLBACK_FRAMES {
                self.head_l.push(l);
                self.head_r.push(r);
            }
        }
    }
}

pub fn record(opt: &Options) -> Result<(), Failure> {
    let host = cpal::default_host();
    let alsa = host.id().name() == "ALSA";

    // A saída do clique é sondada antes de tudo (abre e fecha, sem stream): sem ela, sai com 2.
    let output = if opt.click {
        let no_click = |why: String| input(anyhow!("{why}; --no-click pula o clique"));
        let device = host
            .default_output_device()
            .ok_or_else(|| no_click("não há saída padrão para o clique".to_owned()))?;
        device
            .default_output_config()
            .map_err(|e| no_click(format!("a saída padrão não abre para o clique ({e})")))?;
        Some(device)
    } else {
        None
    };

    let mic_device = match &opt.mic {
        Some(needle) => find_device(host.input_devices(), needle, false, "entrada")?,
        None => host
            .default_input_device()
            .ok_or_else(|| input(anyhow!("não há entrada padrão")))?,
    };
    let mic_config = config_48k(&mic_device, false)?;

    let (mic_prod, mut mic_cons) = RingBuffer::<f32>::new(RING_FRAMES);
    let (sys_prod, mut sys_cons) = RingBuffer::<f32>::new(RING_FRAMES);
    let mic_counters = Arc::new(Counters::default());
    let sys_counters = Arc::new(Counters::default());

    let sys_stream = if alsa {
        // SAFETY: nenhuma thread existe ainda (nenhum stream do cpal foi criado; a sondagem acima
        // abre e fecha o PCM). O plugin ALSA do PipeWire lê as duas variáveis ao abrir o PCM; elas
        // são removidas logo depois, antes de abrir o mic. Mecanismo de spike (door 1 do plano).
        unsafe {
            std::env::set_var("PIPEWIRE_NODE", &opt.system);
            std::env::set_var("PIPEWIRE_ALSA", "{ stream.capture.sink = true }");
        }
        let opened = find_device(host.input_devices(), "pipewire", true, "entrada").and_then(|d| {
            let config = config_48k(&d, false)?;
            input_stream(&d, &config, sys_prod, Arc::clone(&sys_counters), "system")
        });
        // SAFETY: como acima; o stream do sistema já abriu o PCM e não relê o ambiente.
        unsafe {
            std::env::remove_var("PIPEWIRE_NODE");
            std::env::remove_var("PIPEWIRE_ALSA");
        }
        opened?
    } else {
        let device = find_device(host.output_devices(), &opt.system, false, "saída")?;
        let config = config_48k(&device, true)?;
        input_stream(
            &device,
            &config,
            sys_prod,
            Arc::clone(&sys_counters),
            "system",
        )?
    };

    let pending_clicks = Arc::new(AtomicUsize::new(0));
    let click_out = match &output {
        Some(device) => Some(click_stream(device, Arc::clone(&pending_clicks))?),
        None => None,
    };
    let mic_stream = input_stream(
        &mic_device,
        &mic_config,
        mic_prod,
        Arc::clone(&mic_counters),
        "mic",
    )?;

    let mut sink = Sink::create(&opt.out)?;

    if let Some(s) = &click_out {
        s.play()
            .context("não consegui tocar a saída do clique")
            .map_err(failed)?;
    }
    sys_stream.play().context("stream system").map_err(failed)?;
    mic_stream.play().context("stream mic").map_err(failed)?;
    let t0 = Instant::now();
    log::info!(
        "gravando {} s em {} (system={}, mic={})",
        opt.duration.as_secs(),
        opt.out.display(),
        opt.system,
        mic_device.name().unwrap_or_default()
    );

    let mut mic_q = VecDeque::new();
    let mut sys_q = VecDeque::new();
    let mut watchdog = Watchdog::new(t0);
    let mut clicks: [Option<f64>; 2] = [None, None];
    let second_click = opt.duration.saturating_sub(CLICK_AT);
    let (mut last_flush, mut last_progress) = (t0, t0);
    let mut fallback_checked = false;

    let outcome: Result<(), Failure> = loop {
        std::thread::sleep(TICK);
        let now = Instant::now();
        let elapsed = now.duration_since(t0);
        drain(&mut mic_cons, &mut mic_q);
        drain(&mut sys_cons, &mut sys_q);
        if let Err(e) = sink.write_pairs(&mut mic_q, &mut sys_q, false) {
            break Err(failed(e));
        }
        if !fallback_checked && sink.head_l.len() == FALLBACK_FRAMES {
            fallback_checked = true;
            if fallback(&sink.head_l, &sink.head_r) {
                break Err(failed(anyhow!(
                    "a captura do sistema caiu no microfone: os primeiros {FALLBACK_FRAMES} frames de L e R são idênticos"
                )));
            }
        }
        let (mic_n, sys_n) = (
            mic_counters.frames.load(Ordering::Relaxed),
            sys_counters.frames.load(Ordering::Relaxed),
        );
        if let Some(name) = watchdog.observe(mic_n, sys_n, now) {
            break Err(failed(anyhow!(
                "o stream {name} não entrega frames há {} s",
                STALL.as_secs()
            )));
        }
        if opt.click {
            for (slot, at) in clicks.iter_mut().zip([CLICK_AT, second_click]) {
                if slot.is_none() && elapsed >= at {
                    pending_clicks.fetch_add(1, Ordering::AcqRel);
                    *slot = Some(elapsed.as_secs_f64());
                }
            }
        }
        if now.duration_since(last_flush) >= opt.flush {
            if let Err(e) = sink.wav.flush() {
                break Err(failed(anyhow!(e).context("flush do WAV")));
            }
            last_flush = now;
        }
        if now.duration_since(last_progress) >= PROGRESS {
            eprintln!(
                "{} s: mic_frames={mic_n} sys_frames={sys_n}",
                elapsed.as_secs()
            );
            last_progress = now;
        }
        if elapsed >= opt.duration {
            break Ok(());
        }
    };
    let wall_s = t0.elapsed().as_secs_f64();
    drop(mic_stream);
    drop(sys_stream);
    drop(click_out);

    // O que sobrou sem par vai com zero do outro lado: o arquivo tem o tamanho do stream maior.
    drain(&mut mic_cons, &mut mic_q);
    drain(&mut sys_cons, &mut sys_q);
    sink.write_pairs(&mut mic_q, &mut sys_q, true)
        .map_err(failed)?;
    let Sink {
        wav,
        mic_level,
        sys_level,
        ..
    } = sink;
    wav.finalize()
        .context("não consegui fechar o WAV")
        .map_err(failed)?;

    let mic_n = mic_counters.frames.load(Ordering::Relaxed);
    let sys_n = sys_counters.frames.load(Ordering::Relaxed);
    let click_s = |c: Option<f64>| c.map_or_else(|| "-".to_owned(), |s| format!("{s:.3}"));
    let row = vec![
        RATE.to_string(),
        format!("{wall_s:.3}"),
        mic_n.to_string(),
        sys_n.to_string(),
        format!("{:.1}", ppm(mic_n, RATE, wall_s)),
        format!("{:.1}", ppm(sys_n, RATE, wall_s)),
        format!("{:.1}", rel_drift_ms(mic_n, sys_n, RATE)),
        mic_counters.dropped.load(Ordering::Relaxed).to_string(),
        sys_counters.dropped.load(Ordering::Relaxed).to_string(),
        (mic_counters.errors.load(Ordering::Relaxed) + sys_counters.errors.load(Ordering::Relaxed))
            .to_string(),
        click_s(clicks[0]),
        click_s(clicks[1]),
        format!("{:.3}", mic_level.peak),
        format!("{:.1}", mic_level.rms_dbfs()),
        format!("{:.3}", sys_level.peak),
        format!("{:.1}", sys_level.rms_dbfs()),
    ];
    print_table(&COLUMNS.split_whitespace().collect::<Vec<_>>(), &row);
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_needs_96000_identical_frames() {
        let l: Vec<i16> = (0..96_000).map(|i| (i % 1000) as i16).collect();
        assert!(fallback(&l, &l.clone()));
        let mut r = l.clone();
        r[95_999] += 1;
        assert!(!fallback(&l, &r));
        assert!(!fallback(&l[..95_999], &l[..95_999]));
    }

    #[test]
    fn watchdog_names_stream_stalled_5s() {
        let t0 = Instant::now();
        let mut w = Watchdog::new(t0);
        assert_eq!(w.observe(100, 100, t0 + Duration::from_secs(1)), None);
        // `mic` cresce, `system` parou em 100 desde t0 + 1 s.
        assert_eq!(w.observe(200, 100, t0 + Duration::from_millis(5_900)), None);
        assert_eq!(
            w.observe(300, 100, t0 + Duration::from_secs(6)),
            Some("system")
        );

        let mut w = Watchdog::new(t0);
        assert_eq!(w.observe(0, 10, t0 + Duration::from_millis(4_900)), None);
        assert_eq!(w.observe(0, 20, t0 + Duration::from_secs(5)), Some("mic"));
    }

    #[test]
    fn metrics_follow_door_3() {
        assert_eq!(format!("{:.1}", ppm(48_048_000, 48_000, 1000.0)), "1000.0");
        assert_eq!(format!("{:.1}", rel_drift_ms(2000, 1000, 48_000)), "20.8");
        let mut square = Level::default();
        for i in 0..1000 {
            square.add(if i % 2 == 0 { i16::MIN } else { i16::MAX });
        }
        assert_eq!(format!("{:.1}", square.rms_dbfs().abs()), "0.0");
        assert_eq!(square.peak, 1.0);
    }

    #[test]
    fn pick_device_matches_or_lists() {
        let names = vec![
            "Alto-falantes (Realtek)".to_owned(),
            "Fone (USB)".to_owned(),
        ];
        assert_eq!(pick_device(&names, "Fone", "saída").unwrap(), 1);
        let err = pick_device(&names, "Voicemeeter", "saída")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("Alto-falantes (Realtek)") && err.contains("Fone (USB)"),
            "{err}"
        );
    }

    #[test]
    fn full_ring_drops_and_counts() {
        let (mut p, mut c) = RingBuffer::<f32>::new(10);
        let counters = Counters::default();
        let stereo: Vec<f32> = (0..30).map(|i| i as f32).collect();
        push_mono(&mut p, &stereo, 2, &counters);
        assert_eq!(counters.frames.load(Ordering::Relaxed), 15);
        assert_eq!(counters.dropped.load(Ordering::Relaxed), 5);
        assert_eq!(c.slots(), 10);
        assert_eq!(c.pop(), Ok(0.5));
    }

    #[test]
    fn click_is_1khz_20ms_half_amplitude() {
        let c = click(48_000);
        assert_eq!(c.len(), 960);
        let peak = c.iter().fold(0f32, |m, x| m.max(x.abs()));
        assert!((0.49..=0.5).contains(&peak), "peak {peak}");
        // 1 kHz a 48 kHz: meio período = 24 amostras; o sinal troca de sinal a cada 24.
        for half in 0..39 {
            let a = c[half * 24 + 12];
            let b = c[(half + 1) * 24 + 12];
            assert!(
                a * b < 0.0,
                "no sign change around sample {}",
                (half + 1) * 24
            );
        }
    }
}
