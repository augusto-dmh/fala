//! As duas engines do `bench`: Parakeet ONNX (`transcribe-rs`, o runtime do produto) e GGUF
//! (`transcribe-cpp`: whisper, Nemotron, Voxtral…, com threads e backend de GPU).

use std::path::Path;
use std::time::Instant;

use anyhow::{anyhow, bail, Result};
use transcribe_cpp::{Backend, DeviceType, ModelOptions, RunOptions, SessionOptions};
use transcribe_rs::onnx::parakeet::ParakeetModel;
use transcribe_rs::onnx::Quantization;
use transcribe_rs::{SpeechModel, TranscribeOptions};

use super::Failure;

/// Idioma pedido à engine gguf quando `--language` não é passado.
pub const DEFAULT_LANGUAGE: &str = "pt-BR";

pub enum Engine {
    Parakeet(Box<ParakeetModel>),
    Gguf {
        session: transcribe_cpp::Session,
        /// Código já resolvido contra a lista do modelo (`resolve_language`).
        language: String,
    },
}

/// O que a legenda precisa saber sobre a engine carregada.
pub struct Loaded {
    pub engine: Engine,
    pub threads: Option<usize>,
    pub device: String,
    pub load_s: f64,
}

pub fn load_parakeet(dir: &Path) -> Result<Loaded> {
    let start = Instant::now();
    let model = ParakeetModel::load(dir, &Quantization::Int8)
        .map_err(|e| anyhow!("não consegui carregar o Parakeet de {}: {e}", dir.display()))?;
    Ok(Loaded {
        engine: Engine::Parakeet(Box::new(model)),
        threads: None,
        device: "cpu".to_owned(),
        load_s: start.elapsed().as_secs_f64(),
    })
}

/// `gpu` exige um backend de GPU compilado e um dispositivo GPU/iGPU; nunca cai para CPU.
/// `requested` é resolvido contra os idiomas que o modelo anuncia; um idioma sem
/// correspondência é entrada inválida (código 2), não falha de engine.
pub fn load_gguf(
    path: &Path,
    threads: usize,
    gpu: bool,
    requested: &str,
) -> Result<Loaded, Failure> {
    transcribe_cpp::init_logging();
    transcribe_cpp::init_backends_default()?;
    let options = if gpu {
        let compiled: Vec<&str> = [
            ("vulkan", cfg!(feature = "vulkan")),
            ("cuda", cfg!(feature = "cuda")),
        ]
        .into_iter()
        .filter_map(|(name, on)| on.then_some(name))
        .collect();
        if compiled.is_empty() {
            return Err(anyhow!(
                "--device gpu: esta build não tem backend de GPU (vulkan ou cuda); \
                 recompile com --features vulkan ou --features cuda"
            )
            .into());
        }
        let device = transcribe_cpp::devices()
            .into_iter()
            .find(|d| matches!(d.device_type, DeviceType::Gpu | DeviceType::Igpu))
            .ok_or_else(|| {
                anyhow!(
                    "--device gpu: backend {} compilado, mas nenhum dispositivo GPU disponível",
                    compiled.join("/")
                )
            })?;
        log::info!(
            "dispositivo GPU: {} ({}, {})",
            device.name,
            device.description,
            device.kind
        );
        ModelOptions {
            backend: Backend::Auto,
            device: Some(device),
        }
    } else {
        ModelOptions {
            backend: Backend::Cpu,
            device: None,
        }
    };

    let start = Instant::now();
    let model = transcribe_cpp::Model::load_with(path, &options)?;
    let session = model.session_with(&SessionOptions {
        n_threads: i32::try_from(threads)?,
        ..SessionOptions::default()
    })?;
    let load_s = start.elapsed().as_secs_f64();
    let device = if gpu {
        model.device()?.name
    } else {
        "cpu".to_owned()
    };
    let language = resolve_language(requested, &model.capabilities().languages)
        .map_err(|error| Failure { code: 2, error })?;
    log::info!("idioma: {language}");
    Ok(Loaded {
        engine: Engine::Gguf { session, language },
        threads: Some(threads),
        device,
        load_s,
    })
}

/// Escolhe, na lista que o modelo anuncia, o código que representa `requested`
/// (comparação sem distinção de maiúsculas; devolve a grafia do modelo):
/// o código exato; senão o prefixo sem região (`pt-BR` → `pt`, o que o whisper lista);
/// senão, para um pedido sem região, a primeira variante regional na ordem do modelo
/// (`pt` → `pt-BR`). Lista vazia (modelo sem metadado de idioma) devolve o prefixo.
pub fn resolve_language(requested: &str, advertised: &[String]) -> Result<String> {
    let bare = requested.split('-').next().unwrap_or(requested);
    if advertised.is_empty() {
        return Ok(bare.to_owned());
    }
    let find = |wanted: &str| {
        advertised
            .iter()
            .find(|code| code.eq_ignore_ascii_case(wanted))
            .cloned()
    };
    if let Some(code) = find(requested).or_else(|| find(bare)) {
        return Ok(code);
    }
    if requested == bare {
        let prefix = format!("{bare}-");
        if let Some(code) = advertised.iter().find(|code| {
            code.get(..prefix.len())
                .is_some_and(|head| head.eq_ignore_ascii_case(&prefix))
        }) {
            return Ok(code.clone());
        }
    }
    bail!(
        "idioma `{requested}` não consta na lista do modelo; ele anuncia: {}",
        advertised.join(", ")
    )
}

impl Engine {
    fn transcribe(&mut self, samples: &[f32]) -> Result<String> {
        match self {
            // `TranscribeOptions::default()` mantém os 250 ms de silêncio inicial da engine.
            Engine::Parakeet(model) => Ok(model
                .transcribe(samples, &TranscribeOptions::default())
                .map_err(|e| anyhow!("{e}"))?
                .text),
            Engine::Gguf { session, language } => {
                let options = RunOptions {
                    language: Some(language.clone()),
                    ..RunOptions::default()
                };
                Ok(session.run(samples, &options)?.text)
            }
        }
    }

    /// Transcreve o corte inteiro (`chunk_samples` = 0) ou em janelas consecutivas.
    pub fn transcribe_cut(
        &mut self,
        samples: &[f32],
        chunk_samples: usize,
    ) -> Result<(String, f64)> {
        transcribe_chunked(samples, chunk_samples, |window| self.transcribe(window))
    }
}

/// Janelas consecutivas sem sobreposição; textos aparados e juntados com um espaço;
/// `wall_s` é a soma do tempo de cada janela.
fn transcribe_chunked(
    samples: &[f32],
    chunk_samples: usize,
    mut transcribe: impl FnMut(&[f32]) -> Result<String>,
) -> Result<(String, f64)> {
    let size = if chunk_samples == 0 {
        samples.len().max(1)
    } else {
        chunk_samples
    };
    let windows: Vec<&[f32]> = if samples.is_empty() {
        vec![samples]
    } else {
        samples.chunks(size).collect()
    };
    let mut texts = Vec::with_capacity(windows.len());
    let mut wall_s = 0.0;
    for window in windows {
        let start = Instant::now();
        let text = transcribe(window)?;
        wall_s += start.elapsed().as_secs_f64();
        let text = text.trim();
        if !text.is_empty() {
            texts.push(text.to_owned());
        }
    }
    Ok((texts.join(" "), wall_s))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn list(codes: &[&str]) -> Vec<String> {
        codes.iter().map(|c| (*c).to_owned()).collect()
    }

    #[test]
    fn language_resolution_table() {
        let regional = list(&["pt-BR", "pt-PT"]);
        let bare = list(&["en", "pt"]);
        assert_eq!(resolve_language("pt-BR", &regional).unwrap(), "pt-BR");
        assert_eq!(resolve_language("pt-PT", &regional).unwrap(), "pt-PT");
        assert_eq!(resolve_language("pt-BR", &bare).unwrap(), "pt");
        assert_eq!(resolve_language("pt", &regional).unwrap(), "pt-BR");
        // A primeira na ordem do modelo, não a primeira em ordem alfabética.
        assert_eq!(
            resolve_language("pt", &list(&["pt-PT", "pt-BR"])).unwrap(),
            "pt-PT"
        );
        assert_eq!(resolve_language("pt-BR", &[]).unwrap(), "pt");
        assert_eq!(resolve_language("PT-br", &regional).unwrap(), "pt-BR");
        let err = resolve_language("pt-BR", &list(&["en", "es"]))
            .unwrap_err()
            .to_string();
        assert!(err.contains("pt-BR"), "{err}");
        assert!(err.contains("en") && err.contains("es"), "{err}");
    }

    #[test]
    fn chunks_are_consecutive_without_overlap() {
        let samples: Vec<f32> = (0..65 * 16_000).map(|i| i as f32).collect();
        let mut seen = Vec::new();
        let (text, wall_s) = transcribe_chunked(&samples, 30 * 16_000, |w| {
            seen.push((w.len(), w[0]));
            std::thread::sleep(Duration::from_millis(20));
            Ok(format!("  w{} ", seen.len()))
        })
        .unwrap();
        assert_eq!(
            seen,
            vec![(480_000, 0.0), (480_000, 480_000.0), (80_000, 960_000.0)]
        );
        assert_eq!(text, "w1 w2 w3");
        assert!(
            wall_s >= 0.060,
            "wall_s {wall_s} is not the sum of 3 windows"
        );
    }
}
