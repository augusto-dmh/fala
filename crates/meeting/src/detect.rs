use std::time::Duration;

use crate::UnixMillis;

/// Tempo que um app de call precisa segurar o mic antes do aviso (o Granola usa 2 s).
pub const OFFER_DELAY: Duration = Duration::from_secs(2);
/// Intervalo mínimo entre dois avisos de "Gravar reunião?".
pub const OFFER_MIN_GAP: Duration = Duration::from_secs(2 * 60);
/// Tempo sem o mic para contar que o app o largou, e não que trocou de dispositivo.
pub const RELEASE_DELAY: Duration = Duration::from_secs(2);
/// Duração gravada mínima para perguntar "Parar a gravação?" quando o app larga o mic.
pub const ASK_STOP_AFTER: Duration = Duration::from_secs(3 * 60);

/// Um app de call reconhecido pelo exe que captura o mic.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct CallApp {
    /// Id estável, o que a lista "não avisar nestes apps" guarda.
    pub id: &'static str,
    /// Nome mostrado no aviso.
    pub name: &'static str,
    exes: &'static [&'static str],
}

macro_rules! apps {
    ($($id:literal, $name:literal, [$($exe:literal),+ $(,)?];)+) => {
        &[$(CallApp { id: $id, name: $name, exes: &[$($exe),+] }),+]
    };
}

/// Os apps reconhecidos, da lista do Granola (`research/19` § 1.3), sem o próprio Fala. Os
/// navegadores entram porque o Meet e as calls web capturam pelo processo do navegador, e não
/// dá para saber qual aba.
pub const CALL_APPS: &[CallApp] = apps! {
    "teams", "Teams", ["ms-teams.exe", "ms-teams_modulehost.exe", "msteamsvdi.exe", "teams.exe"];
    "zoom", "Zoom", ["zoom.exe"];
    "meet", "Google Meet", ["google meet.exe"];
    "whatsapp", "WhatsApp", ["whatsapp.exe", "whatsapp.root.exe"];
    "slack", "Slack", ["slack.exe"];
    "discord", "Discord", ["discord.exe"];
    "webex", "Webex", ["webex.exe", "ciscocollabhost.exe", "atmgr.exe"];
    "8x8", "8x8 Work", ["8x8 work.exe"];
    "aircall", "Aircall", ["aircall.exe"];
    "dialpad", "Dialpad", ["dialpad.exe", "dialpad meetings.exe"];
    "voov", "VooV Meeting", ["voovmeetingapp.exe"];
    "tuple", "Tuple", ["tuple.exe"];
    "gather", "Gather", ["gather.exe"];
    "clickup", "ClickUp", ["clickup.exe"];
    "lark", "Lark", ["lark.exe", "feishu.exe"];
    "edge", "Edge", ["msedge.exe"];
    "chrome", "Chrome", ["chrome.exe"];
    "firefox", "Firefox", ["firefox.exe"];
    "brave", "Brave", ["brave.exe"];
    "arc", "Arc", ["arc.exe"];
    "dia", "Dia", ["dia.exe"];
    "vivaldi", "Vivaldi", ["vivaldi.exe"];
    "opera", "Opera", ["opera.exe"];
    "zen", "Zen", ["zen.exe"];
    "comet", "Comet", ["comet.exe"];
};

/// Processos que capturam em nome de quem os abriu: o app é o ancestral (o Teams novo e o
/// WhatsApp capturam por um WebView2 filho).
const HOSTS: &[&str] = &["msedgewebview2.exe"];

/// O exe do Fala, que nunca é uma call, mesmo como ancestral de um WebView2.
const FALA_EXES: &[&str] = &["fala.exe", "fala-cli.exe"];

impl CallApp {
    /// O app pelo id guardado nas configurações.
    pub fn by_id(id: &str) -> Option<&'static CallApp> {
        CALL_APPS.iter().find(|app| app.id == id)
    }
}

/// O app de call de um processo que captura o mic, pela cadeia exe → pai → avô (minúsculas).
///
/// Vale o próprio exe; um host como o `msedgewebview2.exe` passa a vez ao ancestral. Uma cadeia
/// com o Fala ou com `own_exe` é sempre `None`.
pub fn identify<'a>(
    chain: impl IntoIterator<Item = &'a str>,
    own_exe: &str,
) -> Option<&'static CallApp> {
    let chain: Vec<&str> = chain.into_iter().collect();
    if chain
        .iter()
        .any(|exe| FALA_EXES.contains(exe) || (!own_exe.is_empty() && *exe == own_exe))
    {
        return None;
    }
    for exe in chain {
        if let Some(app) = CALL_APPS.iter().find(|app| app.exes.contains(&exe)) {
            return Some(app);
        }
        if !HOSTS.contains(&exe) {
            return None;
        }
    }
    None
}

/// O que o detector pede à UI. Só pergunta: nenhuma variante grava, retoma ou para (ADR-0005).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallPrompt {
    /// "Parece uma chamada no <app>. Gravar reunião?" com [Gravar] [Agora não] [Não avisar].
    OfferRecording { app: &'static CallApp },
    /// "O <app> largou o microfone. Parar a gravação?"; sem resposta, a gravação continua.
    AskStop { app: &'static CallApp },
}

/// O que o detector precisa saber do resto do app a cada leitura.
#[derive(Debug, Clone, Copy, Default)]
pub struct DetectContext<'a> {
    /// "Avisar quando uma chamada começar".
    pub enabled: bool,
    /// Ids da lista "não avisar nestes apps".
    pub muted: &'a [String],
    /// A duração gravada, se uma reunião grava ou está pausada agora.
    pub recording: Option<Duration>,
}

#[derive(Debug, Clone)]
struct Hold {
    app: &'static CallApp,
    since: UnixMillis,
    /// Já avisou (ou decidiu não avisar) por esta posse do mic.
    settled: bool,
    gone_since: Option<UnixMillis>,
}

/// Detector de call: recebe, a cada leitura, os apps de call que capturam o mic e devolve no
/// máximo um [`CallPrompt`].
///
/// Lógica pura, como a sessão: o chamador lê o relógio e o SO. Um aviso por posse do mic
/// (sumir por menos de [`RELEASE_DELAY`] não é outra posse), [`OFFER_DELAY`] de espera,
/// [`OFFER_MIN_GAP`] entre avisos, nada durante uma gravação, nada para app silenciado. Quando
/// os apps vistos durante a gravação largam o mic depois de [`ASK_STOP_AFTER`] gravados, pede
/// a pergunta "parar?" uma vez.
#[derive(Debug, Clone, Default)]
pub struct CallDetector {
    holds: Vec<Hold>,
    last_offer: Option<UnixMillis>,
    /// O app de call visto com o mic durante a gravação atual.
    recording_app: Option<&'static CallApp>,
}

fn elapsed(from: UnixMillis, now: UnixMillis) -> Duration {
    Duration::from_millis(now.0.saturating_sub(from.0))
}

impl CallDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Os apps que seguram o mic agora, já confirmados (sem os que sumiram há pouco).
    pub fn holding(&self) -> impl Iterator<Item = &'static CallApp> + '_ {
        self.holds
            .iter()
            .filter(|h| h.gone_since.is_none())
            .map(|h| h.app)
    }

    /// Uma leitura no instante `now`: os apps de call com sessão de captura ativa.
    pub fn observe(
        &mut self,
        now: UnixMillis,
        present: &[&'static CallApp],
        ctx: DetectContext<'_>,
    ) -> Option<CallPrompt> {
        self.update_holds(now, present);
        match ctx.recording {
            Some(recorded) => self.while_recording(recorded),
            None => {
                self.recording_app = None;
                self.offer(now, ctx)
            }
        }
    }

    fn update_holds(&mut self, now: UnixMillis, present: &[&'static CallApp]) {
        for hold in &mut self.holds {
            if present.contains(&hold.app) {
                hold.gone_since = None;
            } else if hold.gone_since.is_none() {
                hold.gone_since = Some(now);
            }
        }
        self.holds.retain(|hold| {
            hold.gone_since
                .is_none_or(|gone| elapsed(gone, now) < RELEASE_DELAY)
        });
        for &app in present {
            if !self.holds.iter().any(|hold| hold.app == app) {
                self.holds.push(Hold {
                    app,
                    since: now,
                    settled: false,
                    gone_since: None,
                });
            }
        }
    }

    fn while_recording(&mut self, recorded: Duration) -> Option<CallPrompt> {
        // Durante a gravação não há aviso, nem depois dela pela mesma posse do mic.
        for hold in &mut self.holds {
            hold.settled = true;
            if hold.gone_since.is_none() {
                self.recording_app = Some(hold.app);
            }
        }
        let app = self.recording_app?;
        if !self.holds.is_empty() {
            return None;
        }
        // Todos os apps de call largaram o mic (há pelo menos RELEASE_DELAY).
        self.recording_app = None;
        (recorded >= ASK_STOP_AFTER).then_some(CallPrompt::AskStop { app })
    }

    fn offer(&mut self, now: UnixMillis, ctx: DetectContext<'_>) -> Option<CallPrompt> {
        let gap_open = self
            .last_offer
            .is_none_or(|last| elapsed(last, now) >= OFFER_MIN_GAP);
        let hold = self.holds.iter_mut().find(|hold| {
            !hold.settled && hold.gone_since.is_none() && elapsed(hold.since, now) >= OFFER_DELAY
        })?;
        hold.settled = true;
        let muted = ctx.muted.iter().any(|id| id == hold.app.id);
        if !ctx.enabled || muted || !gap_open {
            return None;
        }
        self.last_offer = Some(now);
        Some(CallPrompt::OfferRecording { app: hold.app })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_exes_are_unique_and_lowercase() {
        let mut ids: Vec<&str> = CALL_APPS.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), CALL_APPS.len());
        let mut exes: Vec<&str> = CALL_APPS
            .iter()
            .flat_map(|a| a.exes.iter().copied())
            .collect();
        assert!(exes.iter().all(|e| *e == e.to_lowercase()));
        let total = exes.len();
        exes.sort_unstable();
        exes.dedup();
        assert_eq!(exes.len(), total);
        for fala in FALA_EXES {
            assert!(CALL_APPS.iter().all(|a| !a.exes.contains(fala)));
        }
    }
}
