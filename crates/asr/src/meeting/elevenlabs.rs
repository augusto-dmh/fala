//! ElevenLabs Scribe v2 em batch (ADR-0005), com a chave do usuário (BYOK, ADR-0008).
//!
//! Uma requisição por canal, em sequência: o mic com `diarize=false` fora do modo presencial,
//! o sistema sempre com `diarize=true`. O que sai da máquina é exatamente a door 3 do plano
//! `meeting-asr`: a chave só no header `xi-api-key` e, no multipart, `model_id`, `file`,
//! `language_code` (omitido em "detectar"), `diarize`, `timestamps_granularity` e um
//! `keyterms` por termo do dicionário quando o ajuste está ligado. Nada mais.

use std::io::{self, Read};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use fala_core::{Dictionary, Language};
use fala_meeting::SessionMode;
use fala_secrets::ApiKey;

use super::segments::{assign_speakers, group_words, ChannelSegment, ScribeResponse};
use super::{CancelToken, Channel, MeetingRecording, MeetingTranscriber, Progress, Segment, Stage};
use crate::AsrError;

/// Raiz da API da ElevenLabs.
pub const SCRIBE_BASE_URL: &str = "https://api.elevenlabs.io";
const PATH: &str = "/v1/speech-to-text";
const MODEL_ID: &str = "scribe_v2";
/// Teto do JSON de resposta: horas de palavras com tempos cabem com folga.
const MAX_RESPONSE_BYTES: u64 = 64 * 1024 * 1024;
/// De quanto em quanto tempo a espera olha o cancelamento.
const POLL: Duration = Duration::from_millis(100);
/// Intervalo do progresso enquanto um canal sobe ou espera resposta (orçamento: ≤ 1 s).
const HEARTBEAT: Duration = Duration::from_millis(500);

/// Os ajustes que mudam a requisição.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScribeOptions {
    /// Idioma da reunião; `None` é "detectar" e omite `language_code`.
    pub language: Option<Language>,
    /// Manda o dicionário como `keyterms` (ligado por padrão).
    pub keyterms: bool,
    pub dictionary: Dictionary,
}

impl Default for ScribeOptions {
    fn default() -> Self {
        Self {
            language: Some(Language::PtBr),
            keyterms: true,
            dictionary: Dictionary::default(),
        }
    }
}

/// O backend Scribe v2. Só transcreve [`MeetingRecording`]; não implementa
/// [`crate::Transcriber`].
pub struct ElevenLabsScribe {
    key: ApiKey,
    options: ScribeOptions,
    base_url: String,
}

impl ElevenLabsScribe {
    pub fn new(key: ApiKey, options: ScribeOptions) -> Self {
        Self {
            key,
            options,
            base_url: SCRIBE_BASE_URL.to_string(),
        }
    }

    /// Troca a raiz da API (servidor falso nos testes).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Os campos de texto do multipart de um canal, em ordem.
    fn fields(&self, diarize: bool) -> Vec<(&'static str, String)> {
        let mut fields = vec![("model_id", MODEL_ID.to_string())];
        if let Some(language) = self.options.language {
            fields.push(("language_code", language_code(language).to_string()));
        }
        fields.push(("diarize", diarize.to_string()));
        fields.push(("timestamps_granularity", "word".to_string()));
        if self.options.keyterms {
            for term in self.options.dictionary.terms() {
                fields.push(("keyterms", term.clone()));
            }
        }
        fields
    }

    fn channel(
        &self,
        channel: Channel,
        path: &Path,
        diarize: bool,
        cancel: &CancelToken,
        on_progress: &mut dyn FnMut(Progress),
    ) -> Result<Vec<ChannelSegment>, AsrError> {
        if cancel.is_cancelled() {
            return Err(AsrError::Cancelled);
        }
        let audio = std::fs::read(path).map_err(|e| AsrError::ReadAudio {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("audio.opus");
        let boundary = boundary_for(&audio);
        let body = multipart(&boundary, &self.fields(diarize), file_name, &audio);
        let total = body.len() as u64;
        on_progress(Progress {
            channel,
            stage: Stage::Uploading { sent: 0, total },
        });

        // O ureq bloqueia até a resposta e não aborta uma requisição em voo: ela roda num
        // thread, e este espera com batimento e cancelamento. Cancelada, a resposta é
        // descartada quando chegar.
        let sent = Arc::new(AtomicU64::new(0));
        let (tx, rx) = mpsc::channel();
        {
            let (url, key, sent) = (self.url(), self.key.clone(), Arc::clone(&sent));
            thread::spawn(move || {
                let _ = tx.send(post(&url, &key, &boundary, body, &sent));
            });
        }
        let mut last_beat = Instant::now();
        let raw = loop {
            if cancel.is_cancelled() {
                return Err(AsrError::Cancelled);
            }
            match rx.recv_timeout(POLL) {
                Ok(result) => break result?,
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(AsrError::Network {
                        retriable: true,
                        status: None,
                        reason: "o envio parou sem resposta".to_string(),
                    })
                }
            }
            if last_beat.elapsed() >= HEARTBEAT {
                last_beat = Instant::now();
                let sent = sent.load(Ordering::SeqCst).min(total);
                let stage = if sent < total {
                    Stage::Uploading { sent, total }
                } else {
                    Stage::Waiting
                };
                on_progress(Progress { channel, stage });
            }
        };
        let response: ScribeResponse =
            serde_json::from_str(&raw).map_err(|e| AsrError::InvalidResponse(e.to_string()))?;
        Ok(group_words(&response.words))
    }

    fn url(&self) -> String {
        format!("{}{PATH}", self.base_url.trim_end_matches('/'))
    }
}

impl MeetingTranscriber for ElevenLabsScribe {
    fn transcribe_session(
        &mut self,
        recording: &MeetingRecording,
        cancel: &CancelToken,
        on_progress: &mut dyn FnMut(Progress),
    ) -> Result<Vec<Segment>, AsrError> {
        let mode = recording.mode();
        // Sem canal de mic no modo (só sistema, importação), o arquivo dele não sai da máquina.
        let mic = if mode.has_mic() {
            let diarize = mode == SessionMode::InPerson;
            self.channel(Channel::Mic, recording.mic(), diarize, cancel, on_progress)?
        } else {
            Vec::new()
        };
        let system = self.channel(
            Channel::System,
            recording.system(),
            true,
            cancel,
            on_progress,
        )?;
        Ok(assign_speakers(mode, mic, system))
    }
}

/// ISO 639-3, como a Scribe devolve em `language_code`.
fn language_code(language: Language) -> &'static str {
    match language {
        Language::PtBr => "por",
        Language::En => "eng",
    }
}

/// Um boundary que não aparece nos bytes do arquivo.
fn boundary_for(audio: &[u8]) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let mut boundary = format!("fala-{nanos:x}");
    while audio
        .windows(boundary.len())
        .any(|w| w == boundary.as_bytes())
    {
        boundary.push('x');
    }
    boundary
}

/// `multipart/form-data` montado à mão: os campos de texto em ordem e o arquivo por último.
fn multipart(
    boundary: &str,
    fields: &[(&'static str, String)],
    file_name: &str,
    audio: &[u8],
) -> Vec<u8> {
    let mut body = Vec::with_capacity(audio.len() + 1024);
    for (name, value) in fields {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; \
             filename=\"{file_name}\"\r\nContent-Type: audio/ogg\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(audio);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

/// Uma requisição. Erro de transporte, `429` e `5xx` podem ser repetidos; o resto não. O erro
/// nunca carrega o corpo da resposta nem a chave.
fn post(
    url: &str,
    key: &ApiKey,
    boundary: &str,
    body: Vec<u8>,
    sent: &AtomicU64,
) -> Result<String, AsrError> {
    let length = body.len();
    let mut reader = Counting {
        body: io::Cursor::new(body),
        sent,
    };
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into();
    let mut response = agent
        .post(url)
        .header("xi-api-key", key.expose())
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header("content-length", length.to_string())
        .send(ureq::SendBody::from_reader(&mut reader))
        .map_err(transport_error)?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(AsrError::Network {
            retriable: status == 429 || (500..600).contains(&status),
            status: Some(status),
            reason: format!("HTTP {status}"),
        });
    }
    response
        .body_mut()
        .with_config()
        .limit(MAX_RESPONSE_BYTES)
        .read_to_string()
        .map_err(transport_error)
}

/// O corpo da requisição, contando os bytes que o ureq já leu para o socket.
struct Counting<'a> {
    body: io::Cursor<Vec<u8>>,
    sent: &'a AtomicU64,
}

impl Read for Counting<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.body.read(buf)?;
        self.sent.fetch_add(n as u64, Ordering::SeqCst);
        Ok(n)
    }
}

fn transport_error(error: ureq::Error) -> AsrError {
    AsrError::Network {
        retriable: true,
        status: None,
        reason: error.to_string(),
    }
}
