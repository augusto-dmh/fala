use std::time::Duration;

use crate::{RecordingCap, SessionError, SessionId, SessionMode};

/// Abaixo disso a gravação não começa (500 MiB).
pub const MIN_FREE_DISK_BYTES: u64 = 500 * 1024 * 1024;
/// Abaixo disso a gravação começa com aviso (2 GiB; o WAV de trabalho cresce ~690 MB/h).
pub const LOW_DISK_WARN_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// Nível RMS linear abaixo do qual um canal conta como silêncio (≈ −60 dBFS).
pub const SILENCE_RMS: f32 = 0.001;
/// Silêncio contínuo nos canais considerados que para a gravação (decisão D6 do pitch).
pub const SILENCE_STOP_AFTER: Duration = Duration::from_secs(15 * 60);
/// Antecedência do aviso de teto.
const CAP_WARNING_BEFORE: Duration = Duration::from_secs(10 * 60);

/// Instante de parede em milissegundos desde a época Unix, fornecido pelo chamador.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnixMillis(pub u64);

/// Ação explícita do usuário. É o único caminho para iniciar e retomar uma gravação (ADR-0005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UserAction {
    /// "Gravar reunião", com o espaço livre medido no disco de destino no momento do clique.
    StartRecording {
        free_disk_bytes: u64,
    },
    PauseRecording,
    /// "Retomar" depois de uma pausa, ou "continuar" depois de uma suspensão do SO.
    ResumeRecording,
    StopRecording,
    /// "Mais 1 h" no aviso do teto.
    ExtendCap,
}

/// Nível RMS linear (0 a 1) de cada canal no último intervalo.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Levels {
    pub mic: f32,
    pub system: f32,
}

/// Tudo o que move a sessão.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Input {
    User(UserAction),
    /// Relato periódico do gravador: duração total já gravada na sessão e níveis do intervalo.
    Tick {
        recorded: Duration,
        levels: Levels,
    },
    /// O SO vai suspender.
    OsSuspended,
    /// O SO voltou da suspensão.
    OsResumed,
    /// O gravador fechou o cabeçalho e o arquivo da sessão.
    ///
    /// Contrato com o gravador: só responde assim a um [`Effect::FinalizeCapture`] pedido em
    /// `Stopping`. O fechamento pedido pela suspensão do SO não gera resposta (em `Suspended`
    /// esta entrada é ignorada), e parar depois de uma suspensão pede um segundo
    /// `FinalizeCapture`, que o gravador trata como já feito e confirma. Sem esta resposta a
    /// sessão fica em `Stopping` e `Process` não sai.
    CaptureFinalized,
}

/// O que o indicador visível mostra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndicatorKind {
    Recording,
    Paused,
    /// Suspensão do SO: aguardando "continuar ou parar".
    Suspended,
}

/// Indicador de gravação obrigatório (ADR-0005). Só `fala-meeting` o constrói, então nenhum
/// estado que o carrega existe fora de uma transição deste crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Indicator {
    kind: IndicatorKind,
}

impl Indicator {
    fn new(kind: IndicatorKind) -> Self {
        Self { kind }
    }

    pub fn kind(self) -> IndicatorKind {
        self.kind
    }
}

/// Por que a sessão parou.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StopReason {
    User,
    /// 15 min de silêncio nos canais considerados.
    Silence,
    /// A duração gravada bateu no teto.
    CapReached,
}

/// Estado da sessão.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionState {
    Idle,
    Recording {
        indicator: Indicator,
    },
    Paused {
        indicator: Indicator,
    },
    Suspended {
        indicator: Indicator,
    },
    /// Esperando o gravador fechar o arquivo.
    Stopping {
        reason: StopReason,
    },
    Stopped {
        reason: StopReason,
    },
}

/// O que o chamador executa depois de uma transição, na ordem devolvida.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Espaço livre abaixo de 2 GiB; a gravação começa mesmo assim.
    WarnLowDisk {
        free_bytes: u64,
    },
    ShowIndicator(IndicatorKind),
    HideIndicator,
    StartCapture,
    /// Para de escrever; o intervalo pausado não entra no arquivo.
    PauseCapture,
    ResumeCapture,
    /// Fecha o cabeçalho e o arquivo da sessão.
    FinalizeCapture,
    CapWarning {
        remaining: Duration,
    },
    /// Depois da suspensão: perguntar "continuar ou parar".
    AskContinueOrStop,
    /// Entregar a sessão ao pipeline (conversão, ASR, notas).
    Process,
}

/// Intervalo de parede em que a sessão não gravou.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GapKind {
    Pause,
    Suspend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Gap {
    pub kind: GapKind,
    /// Posição no áudio gravado onde o intervalo fica.
    pub audio_offset: Duration,
    pub from: UnixMillis,
    /// `None` enquanto o intervalo está aberto.
    pub to: Option<UnixMillis>,
}

/// Dados com que a sessão nasce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionConfig {
    pub id: SessionId,
    pub mode: SessionMode,
    pub title: String,
    /// "Só local": o ASR roda no Parakeet (ADR-0005).
    pub local_only: bool,
    pub cap: RecordingCap,
}

/// Uma sessão de reunião. Nasce em [`SessionState::Idle`] e só avança por [`MeetingSession::apply`].
#[derive(Debug, Clone, PartialEq)]
pub struct MeetingSession {
    config: SessionConfig,
    state: SessionState,
    recorded: Duration,
    silent_for: Duration,
    cap_warned: bool,
    gaps: Vec<Gap>,
    started_at: Option<UnixMillis>,
    ended_at: Option<UnixMillis>,
}

impl MeetingSession {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            config,
            state: SessionState::Idle,
            recorded: Duration::ZERO,
            silent_for: Duration::ZERO,
            cap_warned: false,
            gaps: Vec::new(),
            started_at: None,
            ended_at: None,
        }
    }

    pub fn id(&self) -> SessionId {
        self.config.id
    }

    pub fn mode(&self) -> SessionMode {
        self.config.mode
    }

    pub fn title(&self) -> &str {
        &self.config.title
    }

    pub fn local_only(&self) -> bool {
        self.config.local_only
    }

    /// O teto em vigor, já com as extensões.
    pub fn cap(&self) -> RecordingCap {
        self.config.cap
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    /// O indicador que a UI precisa mostrar agora, se houver.
    pub fn indicator(&self) -> Option<Indicator> {
        match self.state {
            SessionState::Recording { indicator }
            | SessionState::Paused { indicator }
            | SessionState::Suspended { indicator } => Some(indicator),
            SessionState::Idle | SessionState::Stopping { .. } | SessionState::Stopped { .. } => {
                None
            }
        }
    }

    /// Duração gravada, sem pausas nem suspensões.
    pub fn recorded(&self) -> Duration {
        self.recorded
    }

    pub fn gaps(&self) -> &[Gap] {
        &self.gaps
    }

    pub fn started_at(&self) -> Option<UnixMillis> {
        self.started_at
    }

    pub fn ended_at(&self) -> Option<UnixMillis> {
        self.ended_at
    }

    /// Aplica uma entrada no instante `now`. Uma ação do usuário que não vale no estado atual
    /// devolve erro sem mudar nada; uma entrada do sistema fora de lugar não faz nada.
    pub fn apply(&mut self, now: UnixMillis, input: Input) -> Result<Vec<Effect>, SessionError> {
        use SessionState as S;
        match (self.state, input) {
            (S::Idle, Input::User(UserAction::StartRecording { free_disk_bytes })) => {
                self.start(now, free_disk_bytes)
            }
            (S::Recording { .. }, Input::User(UserAction::PauseRecording)) => {
                self.open_gap(GapKind::Pause, now);
                self.state = S::Paused {
                    indicator: Indicator::new(IndicatorKind::Paused),
                };
                Ok(vec![
                    Effect::PauseCapture,
                    Effect::ShowIndicator(IndicatorKind::Paused),
                ])
            }
            (S::Paused { .. } | S::Suspended { .. }, Input::User(UserAction::ResumeRecording)) => {
                self.close_gap(now);
                self.state = S::Recording {
                    indicator: Indicator::new(IndicatorKind::Recording),
                };
                Ok(vec![
                    Effect::ShowIndicator(IndicatorKind::Recording),
                    Effect::ResumeCapture,
                ])
            }
            (
                S::Recording { .. } | S::Paused { .. } | S::Suspended { .. },
                Input::User(UserAction::StopRecording),
            ) => Ok(self.begin_stop(now, StopReason::User)),
            (S::Recording { .. } | S::Paused { .. }, Input::User(UserAction::ExtendCap)) => {
                self.config.cap = self.config.cap.extended()?;
                self.cap_warned = false;
                Ok(Vec::new())
            }
            (state, Input::User(action)) => Err(SessionError::InvalidTransition { state, action }),
            (S::Recording { .. }, Input::Tick { recorded, levels }) => {
                Ok(self.tick(now, recorded, levels))
            }
            (S::Recording { .. } | S::Paused { .. }, Input::OsSuspended) => {
                self.close_gap(now);
                self.open_gap(GapKind::Suspend, now);
                self.state = S::Suspended {
                    indicator: Indicator::new(IndicatorKind::Suspended),
                };
                Ok(vec![
                    Effect::FinalizeCapture,
                    Effect::ShowIndicator(IndicatorKind::Suspended),
                ])
            }
            (S::Suspended { .. }, Input::OsResumed) => Ok(vec![Effect::AskContinueOrStop]),
            (S::Stopping { reason }, Input::CaptureFinalized) => {
                self.state = S::Stopped { reason };
                Ok(vec![Effect::HideIndicator, Effect::Process])
            }
            (_, Input::Tick { .. } | Input::OsSuspended | Input::OsResumed)
            | (_, Input::CaptureFinalized) => Ok(Vec::new()),
        }
    }

    fn start(
        &mut self,
        now: UnixMillis,
        free_disk_bytes: u64,
    ) -> Result<Vec<Effect>, SessionError> {
        if !self.config.mode.records() {
            return Err(SessionError::ModeDoesNotRecord(self.config.mode));
        }
        if free_disk_bytes < MIN_FREE_DISK_BYTES {
            return Err(SessionError::InsufficientDisk {
                free_bytes: free_disk_bytes,
                missing_bytes: MIN_FREE_DISK_BYTES - free_disk_bytes,
            });
        }
        let mut effects = Vec::with_capacity(3);
        if free_disk_bytes < LOW_DISK_WARN_BYTES {
            effects.push(Effect::WarnLowDisk {
                free_bytes: free_disk_bytes,
            });
        }
        self.started_at = Some(now);
        self.state = SessionState::Recording {
            indicator: Indicator::new(IndicatorKind::Recording),
        };
        effects.push(Effect::ShowIndicator(IndicatorKind::Recording));
        effects.push(Effect::StartCapture);
        Ok(effects)
    }

    fn tick(&mut self, now: UnixMillis, recorded: Duration, levels: Levels) -> Vec<Effect> {
        let delta = recorded.saturating_sub(self.recorded);
        self.recorded = self.recorded.max(recorded);

        let mic_silent = !self.config.mode.has_mic() || levels.mic < SILENCE_RMS;
        if mic_silent && levels.system < SILENCE_RMS {
            self.silent_for += delta;
        } else {
            self.silent_for = Duration::ZERO;
        }

        let cap = self.config.cap.duration();
        if self.recorded >= cap {
            return self.begin_stop(now, StopReason::CapReached);
        }
        if self.silent_for >= SILENCE_STOP_AFTER {
            return self.begin_stop(now, StopReason::Silence);
        }
        if !self.cap_warned && self.recorded >= cap.saturating_sub(CAP_WARNING_BEFORE) {
            self.cap_warned = true;
            return vec![Effect::CapWarning {
                remaining: cap - self.recorded,
            }];
        }
        Vec::new()
    }

    fn begin_stop(&mut self, now: UnixMillis, reason: StopReason) -> Vec<Effect> {
        self.close_gap(now);
        self.ended_at = Some(now);
        self.state = SessionState::Stopping { reason };
        vec![Effect::FinalizeCapture]
    }

    fn open_gap(&mut self, kind: GapKind, now: UnixMillis) {
        self.gaps.push(Gap {
            kind,
            audio_offset: self.recorded,
            from: now,
            to: None,
        });
    }

    fn close_gap(&mut self, now: UnixMillis) {
        if let Some(gap) = self.gaps.last_mut().filter(|gap| gap.to.is_none()) {
            gap.to = Some(now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_has_no_discard_variant() {
        // Match exaustivo, sem curinga: uma variante nova quebra a compilação deste teste e
        // obriga a revisitar a lista abaixo.
        fn name(effect: Effect) -> &'static str {
            match effect {
                Effect::WarnLowDisk { .. } => "WarnLowDisk",
                Effect::ShowIndicator(_) => "ShowIndicator",
                Effect::HideIndicator => "HideIndicator",
                Effect::StartCapture => "StartCapture",
                Effect::PauseCapture => "PauseCapture",
                Effect::ResumeCapture => "ResumeCapture",
                Effect::FinalizeCapture => "FinalizeCapture",
                Effect::CapWarning { .. } => "CapWarning",
                Effect::AskContinueOrStop => "AskContinueOrStop",
                Effect::Process => "Process",
            }
        }
        let all = [
            Effect::WarnLowDisk { free_bytes: 0 },
            Effect::ShowIndicator(IndicatorKind::Recording),
            Effect::HideIndicator,
            Effect::StartCapture,
            Effect::PauseCapture,
            Effect::ResumeCapture,
            Effect::FinalizeCapture,
            Effect::CapWarning {
                remaining: Duration::ZERO,
            },
            Effect::AskContinueOrStop,
            Effect::Process,
        ];
        for effect in all {
            let name = name(effect).to_lowercase();
            for forbidden in ["discard", "delete", "remove", "drop", "erase"] {
                assert!(!name.contains(forbidden), "{name}");
            }
        }
    }
}
