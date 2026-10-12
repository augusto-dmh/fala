//! Pós-processamento do texto ditado.
//!
//! Trait `Formatter`: regras determinísticas pt-BR (`Rules`) e o Gemini (`Gemini`) acima de 15
//! palavras. O LLM recebe só o texto, o nome do app e o dicionário; nunca áudio, tela ou campo
//! ativo (ADR-0004). O nível de limpeza, o destino (derivado do nome do app) e o estilo que o
//! usuário escolheu para o app só mudam o texto do prompt. O `Postprocessor` espera o LLM por 2 s; depois disso fica o texto das
//! regras, e a resposta que chegar até o prazo tardio vira uma `LateEdit` para "aplicar edição".
//! Os snippets chegam ao LLM como marcadores opacos e voltam ao texto depois (`Masked`).

mod destination;
mod gemini;
mod prompt;
mod rules;

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use fala_core::{AppContext, Dictation, Dictionary, Editor, Language, Transcript};
use fala_secrets::ApiKey;

pub use destination::{destination_for, Destination};
pub use gemini::{Gemini, DEFAULT_BASE_URL, DEFAULT_MODEL};
pub use prompt::{
    CleanupLevel, UnknownCleanupLevel, SNIPPET_INSTRUCTION, STYLE_INSTRUCTION, SYSTEM_PROMPT,
};
pub use rules::{Masked, Rules};

/// Quanto o texto espera pelo LLM antes de ser inserido com as regras (ADR-0004).
pub const INSERT_DEADLINE: Duration = Duration::from_secs(2);
/// Até quando, desde o início da request, a resposta ainda vira uma `LateEdit`.
pub const DEFAULT_LATE_DEADLINE: Duration = Duration::from_secs(10);
/// O LLM só formata saídas das regras com mais palavras que isto (ADR-0004).
pub const LLM_MIN_WORDS_EXCLUSIVE: usize = 15;

/// O que um `Formatter` sabe sobre o ditado além do texto.
#[derive(Debug, Clone, Copy)]
pub struct FormatContext<'a> {
    pub app: &'a AppContext,
    pub dictionary: &'a Dictionary,
    pub language: &'a Language,
    /// Quanto o LLM mexe no texto; as regras ignoram.
    pub cleanup: CleanupLevel,
    /// A instrução do estilo do app, escrita pelo usuário; `None` mantém o prompt sem estilo.
    pub style: Option<&'a str>,
}

/// Transforma texto ditado em texto final.
pub trait Formatter: Send + Sync {
    fn format(&self, text: &str, ctx: &FormatContext<'_>) -> Result<String, PostprocError>;
}

/// Por que o texto final ficou com as regras e não com o LLM. Nunca carrega o ditado, a chave
/// ou o corpo da resposta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Fallback {
    #[error("timeout")]
    Timeout,
    #[error("http {0}")]
    Http(u16),
    #[error("rede")]
    Network,
    #[error("resposta inválida")]
    InvalidResponse,
}

/// Erros de `fala-postproc`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PostprocError {
    #[error("o LLM não formatou o texto: {0}")]
    Llm(Fallback),
}

/// Como o LLM entra no `Postprocessor`.
#[derive(Debug, Clone, Default)]
pub struct LlmConfig {
    /// O usuário ligou o pós-processamento por LLM.
    pub enabled: bool,
    /// O cliente, com a chave; `None` quando não há chave.
    pub gemini: Option<Gemini>,
    /// Apps onde o LLM fica desligado, comparados sem caixa.
    pub disabled_apps: Vec<String>,
}

impl LlmConfig {
    /// LLM ligado com a chave dada e o modelo padrão.
    pub fn gemini(key: ApiKey) -> Self {
        Self {
            enabled: true,
            gemini: Some(Gemini::new(key)),
            disabled_apps: Vec::new(),
        }
    }
}

/// O resultado de um ditado formatado.
#[derive(Debug)]
pub struct Formatted {
    pub dictation: Dictation,
    /// Por que o LLM foi tentado e não ficou; `None` se ele não foi tentado ou deu certo.
    pub fallback: Option<Fallback>,
    /// A resposta que ainda pode chegar; só existe quando `fallback` é `Timeout`.
    pub late_edit: Option<LateEdit>,
    /// Os pares "falado → escrito" que as regras aplicaram, como índices em
    /// `Dictionary::replacements()`, um por ocorrência. Valem com ou sem o LLM, que recebe o
    /// texto já trocado.
    pub replaced: Vec<usize>,
    /// Os snippets aplicados, como índices em `Dictionary::snippets()`, um por ocorrência.
    pub snippets: Vec<usize>,
}

type LlmResult = Result<String, Fallback>;

/// A resposta do LLM que passou dos 2 s, entregue até o prazo tardio.
#[derive(Debug)]
pub struct LateEdit {
    rx: mpsc::Receiver<LlmResult>,
    deadline: Instant,
    /// As expansões dos marcadores que o LLM recebeu; vazio sem snippet.
    expansions: Vec<String>,
}

impl LateEdit {
    /// Margem para a thread entregar o resultado depois do `timeout` do cliente HTTP.
    const MARGIN: Duration = Duration::from_millis(500);

    fn resolved(result: LlmResult) -> Self {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(result);
        Self {
            rx,
            deadline: Instant::now(),
            expansions: Vec::new(),
        }
    }

    /// A resposta com os snippets de volta; marcador alterado vira resposta inválida.
    fn unmask(&self, result: LlmResult) -> LlmResult {
        if self.expansions.is_empty() {
            return result;
        }
        result.and_then(|text| {
            rules::fill(&text, &self.expansions).ok_or_else(|| {
                log::debug!("LLM alterou o marcador de um snippet; resposta tardia descartada");
                Fallback::InvalidResponse
            })
        })
    }

    /// Bloqueia até a resposta chegar ou o prazo tardio passar.
    pub fn wait(self) -> LlmResult {
        let remaining = self.deadline.saturating_duration_since(Instant::now()) + Self::MARGIN;
        match self.rx.recv_timeout(remaining) {
            Ok(result) => self.unmask(result),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(Fallback::Timeout),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(Fallback::Network),
        }
    }

    /// A resposta, se já chegou, sem bloquear.
    pub fn try_get(&self) -> Option<LlmResult> {
        match self.rx.try_recv() {
            Ok(result) => Some(self.unmask(result)),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(Fallback::Network)),
        }
    }
}

/// Compõe `Rules` e o LLM opcional e decide quem editou.
#[derive(Debug, Clone)]
pub struct Postprocessor {
    rules: Rules,
    llm: LlmConfig,
    late_deadline: Duration,
    cleanup: CleanupLevel,
    /// App em minúsculas e a instrução do seu estilo.
    app_styles: Vec<(String, String)>,
}

impl Postprocessor {
    pub fn new(llm: LlmConfig) -> Self {
        Self {
            rules: Rules::default(),
            llm,
            late_deadline: DEFAULT_LATE_DEADLINE,
            cleanup: CleanupLevel::default(),
            app_styles: Vec::new(),
        }
    }

    /// Os estilos por app: o nome do app, comparado sem caixa, e a instrução que vai no fim do
    /// prompt. O primeiro par de um app vale; instrução vazia não entra. App fora da lista fica
    /// sem estilo.
    pub fn with_app_styles<A, I>(mut self, styles: impl IntoIterator<Item = (A, I)>) -> Self
    where
        A: AsRef<str>,
        I: AsRef<str>,
    {
        self.app_styles = styles
            .into_iter()
            .map(|(app, instruction)| {
                (
                    app.as_ref().trim().to_lowercase(),
                    instruction.as_ref().trim().to_string(),
                )
            })
            .filter(|(app, instruction)| !app.is_empty() && !instruction.is_empty())
            .collect();
        self
    }

    /// A instrução do estilo do app, se ele tem um.
    fn style_for(&self, app: &AppContext) -> Option<&str> {
        let name = app.app_name.as_deref()?.trim().to_lowercase();
        self.app_styles
            .iter()
            .find(|(app, _)| *app == name)
            .map(|(_, instruction)| instruction.as_str())
    }

    /// Troca o nível de limpeza pedido ao LLM (padrão `Light`).
    pub fn with_cleanup_level(mut self, cleanup: CleanupLevel) -> Self {
        self.cleanup = cleanup;
        self
    }

    /// Troca o prazo tardio; nunca fica abaixo dos 2 s da inserção.
    pub fn with_late_deadline(mut self, deadline: Duration) -> Self {
        self.late_deadline = deadline.max(INSERT_DEADLINE);
        self
    }

    /// Troca as regras locais (padrão: `Rules::default()`, com a pontuação falada ligada).
    pub fn with_rules(mut self, rules: Rules) -> Self {
        self.rules = rules;
        self
    }

    pub fn process(&self, raw: Transcript, app: AppContext, dictionary: &Dictionary) -> Formatted {
        let style = self.style_for(&app);
        let ctx = FormatContext {
            app: &app,
            dictionary,
            language: &raw.language,
            cleanup: self.cleanup,
            style,
        };
        let masked = self.rules.mask(&raw.text, &ctx);
        let mut formatted = Formatted {
            dictation: Dictation {
                final_text: masked.unmasked(),
                editor: Editor::Rules,
                raw,
                app,
            },
            fallback: None,
            late_edit: None,
            replaced: masked.replaced.clone(),
            snippets: masked.snippets.clone(),
        };
        // Um ditado que é só o gatilho entra como a expansão está, sem LLM.
        if masked.is_whole_snippet() {
            return formatted;
        }
        let Some(gemini) = self.llm_for(&formatted.dictation, &masked.text) else {
            return formatted;
        };
        let answer = self
            .ask(
                gemini,
                &formatted.dictation,
                &masked.text,
                dictionary,
                style,
            )
            .and_then(|text| {
                masked.restore(&text).ok_or_else(|| {
                    log::debug!("LLM alterou o marcador de um snippet; fica o texto das regras");
                    (Fallback::InvalidResponse, None)
                })
            });
        match answer {
            Ok(text) => {
                formatted.dictation.final_text = text;
                formatted.dictation.editor = Editor::Llm;
            }
            Err((fallback, late_edit)) => {
                log::debug!("LLM não usado: {fallback}");
                formatted.fallback = Some(fallback);
                formatted.late_edit = late_edit.map(|late| LateEdit {
                    expansions: masked.expansions().to_vec(),
                    ..late
                });
            }
        }
        formatted
    }

    /// O cliente, quando as quatro condições do LLM valem para este ditado. As palavras são as do
    /// texto que o LLM receberia, com cada snippet como um marcador.
    fn llm_for(&self, dictation: &Dictation, masked: &str) -> Option<&Gemini> {
        if !self.llm.enabled {
            return None;
        }
        let gemini = self.llm.gemini.as_ref()?;
        if let Some(app) = &dictation.app.app_name {
            let disabled = self
                .llm
                .disabled_apps
                .iter()
                .any(|d| d.to_lowercase() == app.to_lowercase());
            if disabled {
                return None;
            }
        }
        let words = masked.split_whitespace().count();
        (words > LLM_MIN_WORDS_EXCLUSIVE).then_some(gemini)
    }

    /// Uma request numa thread com o prazo tardio; espera por ela só até `INSERT_DEADLINE`.
    fn ask(
        &self,
        gemini: &Gemini,
        dictation: &Dictation,
        text: &str,
        dictionary: &Dictionary,
        style: Option<&str>,
    ) -> Result<String, (Fallback, Option<LateEdit>)> {
        let started = Instant::now();
        let (tx, rx) = mpsc::channel();
        let gemini = gemini.clone();
        let text = text.to_string();
        let app = dictation.app.clone();
        let dictionary = dictionary.clone();
        let language = dictation.raw.language;
        let cleanup = self.cleanup;
        let style = style.map(str::to_string);
        let late_deadline = self.late_deadline;
        thread::spawn(move || {
            let ctx = FormatContext {
                app: &app,
                dictionary: &dictionary,
                language: &language,
                cleanup,
                style: style.as_deref(),
            };
            let _ = tx.send(gemini.call(&text, &ctx, late_deadline));
        });
        match rx.recv_timeout(INSERT_DEADLINE) {
            Ok(Ok(text)) => Ok(text),
            Ok(Err(Fallback::Timeout)) => Err((
                Fallback::Timeout,
                Some(LateEdit::resolved(Err(Fallback::Timeout))),
            )),
            Ok(Err(fallback)) => Err((fallback, None)),
            Err(mpsc::RecvTimeoutError::Timeout) => Err((
                Fallback::Timeout,
                Some(LateEdit {
                    rx,
                    deadline: started + late_deadline,
                    expansions: Vec::new(),
                }),
            )),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err((Fallback::Network, None)),
        }
    }
}
