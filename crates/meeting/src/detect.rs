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
/// Depois de um "Parar a gravação?", tempo sem perguntar de novo (o `restopSuppressionWindowMs`
/// do Wispr).
pub const ASK_STOP_QUIET: Duration = Duration::from_secs(3 * 60);
/// Máximo de "Parar a gravação?" por gravação (o `restopMaxPerSession` do Wispr).
pub const ASK_STOP_MAX: usize = 2;
/// Nível RMS linear do canal do sistema a partir do qual algo ainda toca depois da largada
/// (≈ −70 dBFS). Abaixo de [`crate::SILENCE_RMS`] de propósito: a call chega a −45…−55 dBFS por
/// janela de 250 ms, com um quarto das janelas entre −60 e −70, e o sistema calado fica perto de
/// −100. Errar para baixo só mantém a gravação.
pub const CALL_QUIET_RMS: f32 = 0.000_316;

/// Um app de call reconhecido pelo exe que captura o mic.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct CallApp {
    /// Id estável, o que a lista "não avisar nestes apps" guarda.
    pub id: &'static str,
    /// Nome mostrado no aviso.
    pub name: &'static str,
    exes: &'static [&'static str],
    /// Navegador: com o mic, só é call quando uma janela mostra uma reunião ([`web_call`]); o
    /// modo de voz de um site ou um teste de mic não é.
    browser: bool,
}

macro_rules! apps {
    (@browser browser) => { true };
    (@browser) => { false };
    ($($id:literal, $name:literal, [$($exe:literal),+ $(,)?] $($browser:ident)?;)+) => {
        &[$(CallApp {
            id: $id,
            name: $name,
            exes: &[$($exe),+],
            browser: apps!(@browser $($browser)?),
        }),+]
    };
}

/// Os apps reconhecidos, da lista do Granola (`research/19` § 1.3), sem o próprio Fala. Os
/// navegadores (marcados `browser`) entram porque o Meet e as calls web capturam pelo processo
/// do navegador; eles só contam como call com um [`WebCall`] no título de uma janela.
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
    "edge", "Edge", ["msedge.exe"] browser;
    "chrome", "Chrome", ["chrome.exe"] browser;
    "firefox", "Firefox", ["firefox.exe"] browser;
    "brave", "Brave", ["brave.exe"] browser;
    "arc", "Arc", ["arc.exe"] browser;
    "dia", "Dia", ["dia.exe"] browser;
    "vivaldi", "Vivaldi", ["vivaldi.exe"] browser;
    "opera", "Opera", ["opera.exe"] browser;
    "zen", "Zen", ["zen.exe"] browser;
    "comet", "Comet", ["comet.exe"] browser;
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

    /// Os exes do app, em minúsculas.
    pub fn exes(&self) -> &'static [&'static str] {
        self.exes
    }

    /// Um navegador: só é call com uma reunião no título de uma janela.
    pub fn is_browser(&self) -> bool {
        self.browser
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

/// Uma call na web, reconhecida pelo título de uma janela do navegador.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct WebCall {
    /// Nome mostrado no aviso ("Google Meet no Edge").
    pub name: &'static str,
    /// Começos de título ("Meet - abc-defg-hij - Microsoft Edge").
    prefixes: &'static [&'static str],
    /// Palavras inteiras em qualquer ponto do título, com a caixa exata.
    words: &'static [&'static str],
}

/// As calls web reconhecidas, dos hosts de conferência do Wispr (`research/notes/23-wispr.md`
/// § 3), pelo nome que o site põe no título. O Meet se chama "Meet - <código>" ou
/// "Meet – <nome>", e "Meet" sozinho é palavra comum demais para valer em qualquer ponto.
pub const WEB_CALLS: &[WebCall] = &[
    WebCall {
        name: "Google Meet",
        prefixes: &["Meet - ", "Meet – ", "Meet — "],
        words: &["Google Meet"],
    },
    WebCall {
        name: "Microsoft Teams",
        prefixes: &[],
        words: &["Microsoft Teams"],
    },
    WebCall {
        name: "Zoom",
        prefixes: &[],
        words: &["Zoom"],
    },
    WebCall {
        name: "Webex",
        prefixes: &[],
        words: &["Webex"],
    },
    WebCall {
        name: "Whereby",
        prefixes: &[],
        words: &["Whereby"],
    },
    WebCall {
        name: "Jitsi",
        prefixes: &[],
        words: &["Jitsi"],
    },
    WebCall {
        name: "Discord",
        prefixes: &[],
        words: &["Discord"],
    },
    WebCall {
        name: "Slack",
        prefixes: &[],
        words: &["Huddle"],
    },
];

/// `word` aparece em `title` sem letra ou dígito colado dos lados.
fn has_word(title: &str, word: &str) -> bool {
    title.match_indices(word).any(|(at, _)| {
        let before = title[..at].chars().next_back();
        let after = title[at + word.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

impl WebCall {
    fn matches(&self, title: &str) -> bool {
        self.prefixes.iter().any(|p| title.starts_with(p))
            || self.words.iter().any(|w| has_word(title, w))
    }
}

/// A call web que um dos títulos de janela do navegador mostra, se houver.
///
/// Os títulos são contexto local (ADR-0012): servem só para esta decisão, e nem o título nem
/// parte dele sai daqui.
pub fn web_call<'a>(titles: impl IntoIterator<Item = &'a str>) -> Option<&'static WebCall> {
    titles
        .into_iter()
        .find_map(|title| WEB_CALLS.iter().find(|call| call.matches(title)))
}

/// Um app de call com o mic numa leitura e, para navegador, a reunião que uma janela mostra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallHolder {
    pub app: &'static CallApp,
    pub web: Option<&'static WebCall>,
}

impl From<&'static CallApp> for CallHolder {
    fn from(app: &'static CallApp) -> Self {
        Self { app, web: None }
    }
}

/// Se cabe mais um "Parar a gravação?" nesta gravação, dados os instantes das perguntas já
/// feitas: no máximo [`ASK_STOP_MAX`], e nenhuma a menos de [`ASK_STOP_QUIET`] da anterior.
pub fn may_ask_stop(asked: &[UnixMillis], now: UnixMillis) -> bool {
    asked.len() < ASK_STOP_MAX
        && asked
            .last()
            .is_none_or(|&last| elapsed(last, now) >= ASK_STOP_QUIET)
}

/// O que o detector pede à UI. Só pergunta: nenhuma variante grava ou retoma (ADR-0005).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallPrompt {
    /// "Parece uma chamada no <app>. Gravar reunião?" com [Gravar] [Agora não] [Não avisar].
    /// `web` é a reunião do navegador ("Google Meet no Edge").
    OfferRecording {
        app: &'static CallApp,
        web: Option<&'static WebCall>,
    },
    /// "O <app> largou o microfone. Parar a gravação?". Sem resposta, só para se o sistema
    /// ficou calado desde então ([`CallQuiet`]).
    AskStop {
        app: &'static CallApp,
        web: Option<&'static WebCall>,
    },
}

/// Se o canal do sistema ficou calado desde o "Parar a gravação?": decide se o card que expira
/// para a gravação. Armado quando o card aparece, e não no instante da largada, para que o som de
/// "saiu da call" do app não conte. Um nível ≥ [`CALL_QUIET_RMS`] ou uma pausa (a pessoa está lá,
/// e pausada não há nível) desarma até o próximo card.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CallQuiet {
    armed: bool,
}

impl CallQuiet {
    /// O card apareceu: daqui em diante o sistema precisa ficar calado.
    pub fn arm(&mut self) {
        self.armed = true;
    }

    /// Um relato do canal do sistema. `true` só na vez em que o som desarma.
    pub fn observe(&mut self, system: f32) -> bool {
        if self.armed && system >= CALL_QUIET_RMS {
            self.armed = false;
            return true;
        }
        false
    }

    /// Pausa ou suspensão: desarma. `true` se estava armado.
    pub fn disarm(&mut self) -> bool {
        std::mem::take(&mut self.armed)
    }

    /// Armado e sem som desde então: o card que expira para a gravação.
    pub fn quiet(&self) -> bool {
        self.armed
    }
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
    /// A última reunião vista num título durante esta posse; fica mesmo se a aba muda.
    web: Option<&'static WebCall>,
    since: UnixMillis,
    /// Já avisou (ou decidiu não avisar) por esta posse do mic.
    settled: bool,
    gone_since: Option<UnixMillis>,
}

impl Hold {
    /// Um app de call, ou um navegador que mostrou uma reunião durante esta posse.
    fn is_call(&self) -> bool {
        !self.app.is_browser() || self.web.is_some()
    }

    fn holder(&self) -> CallHolder {
        CallHolder {
            app: self.app,
            web: self.web,
        }
    }
}

/// Detector de call: recebe, a cada leitura, os apps de call que capturam o mic e devolve no
/// máximo um [`CallPrompt`].
///
/// Lógica pura, como a sessão: o chamador lê o relógio e o SO. Um aviso por posse do mic
/// (sumir por menos de [`RELEASE_DELAY`] não é outra posse), [`OFFER_DELAY`] de espera,
/// [`OFFER_MIN_GAP`] entre avisos, nada durante uma gravação, nada para app silenciado. Quando
/// os apps vistos durante a gravação largam o mic depois de [`ASK_STOP_AFTER`] gravados, pede
/// a pergunta "parar?", no ritmo de [`may_ask_stop`]. Um navegador só conta como call depois
/// de mostrar uma reunião ([`CallHolder::web`]).
#[derive(Debug, Clone, Default)]
pub struct CallDetector {
    holds: Vec<Hold>,
    last_offer: Option<UnixMillis>,
    /// O app de call visto com o mic durante a gravação atual.
    recording_app: Option<CallHolder>,
    /// Quando o "parar?" foi pedido nesta gravação.
    asked_stop: Vec<UnixMillis>,
}

fn elapsed(from: UnixMillis, now: UnixMillis) -> Duration {
    Duration::from_millis(now.0.saturating_sub(from.0))
}

impl CallDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// As calls com o mic agora (sem as que sumiram há pouco, nem navegador sem reunião).
    pub fn holding(&self) -> impl Iterator<Item = CallHolder> + '_ {
        self.holds
            .iter()
            .filter(|h| h.gone_since.is_none() && h.is_call())
            .map(Hold::holder)
    }

    /// Uma leitura no instante `now`: os apps de call com sessão de captura ativa.
    pub fn observe(
        &mut self,
        now: UnixMillis,
        present: &[CallHolder],
        ctx: DetectContext<'_>,
    ) -> Option<CallPrompt> {
        self.update_holds(now, present);
        match ctx.recording {
            Some(recorded) => self.while_recording(now, recorded),
            None => {
                self.recording_app = None;
                self.asked_stop.clear();
                self.offer(now, ctx)
            }
        }
    }

    fn update_holds(&mut self, now: UnixMillis, present: &[CallHolder]) {
        for hold in &mut self.holds {
            match present.iter().find(|seen| seen.app == hold.app) {
                Some(seen) => {
                    hold.gone_since = None;
                    if seen.web.is_some() {
                        hold.web = seen.web;
                    }
                }
                None => {
                    hold.gone_since.get_or_insert(now);
                }
            }
        }
        self.holds.retain(|hold| {
            hold.gone_since
                .is_none_or(|gone| elapsed(gone, now) < RELEASE_DELAY)
        });
        for seen in present {
            if !self.holds.iter().any(|hold| hold.app == seen.app) {
                self.holds.push(Hold {
                    app: seen.app,
                    web: seen.web,
                    since: now,
                    settled: false,
                    gone_since: None,
                });
            }
        }
    }

    fn while_recording(&mut self, now: UnixMillis, recorded: Duration) -> Option<CallPrompt> {
        // Durante a gravação não há aviso, nem depois dela pela mesma posse do mic. Um navegador
        // ainda sem reunião fica em aberto: se mostrar uma depois da gravação, avisa.
        for hold in self.holds.iter_mut().filter(|hold| hold.is_call()) {
            hold.settled = true;
            if hold.gone_since.is_none() {
                self.recording_app = Some(hold.holder());
            }
        }
        let holder = self.recording_app?;
        if self.holds.iter().any(Hold::is_call) {
            return None;
        }
        // Todos os apps de call largaram o mic (há pelo menos RELEASE_DELAY).
        self.recording_app = None;
        if recorded < ASK_STOP_AFTER || !may_ask_stop(&self.asked_stop, now) {
            return None;
        }
        self.asked_stop.push(now);
        Some(CallPrompt::AskStop {
            app: holder.app,
            web: holder.web,
        })
    }

    fn offer(&mut self, now: UnixMillis, ctx: DetectContext<'_>) -> Option<CallPrompt> {
        let gap_open = self
            .last_offer
            .is_none_or(|last| elapsed(last, now) >= OFFER_MIN_GAP);
        // Um navegador sem reunião fica em aberto: avisa se uma janela passar a mostrá-la.
        let hold = self.holds.iter_mut().find(|hold| {
            !hold.settled
                && hold.gone_since.is_none()
                && hold.is_call()
                && elapsed(hold.since, now) >= OFFER_DELAY
        })?;
        hold.settled = true;
        let muted = ctx.muted.iter().any(|id| id == hold.app.id);
        if !ctx.enabled || muted || !gap_open {
            return None;
        }
        self.last_offer = Some(now);
        Some(CallPrompt::OfferRecording {
            app: hold.app,
            web: hold.web,
        })
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

    #[test]
    fn browsers_are_marked() {
        let browsers: Vec<&str> = CALL_APPS
            .iter()
            .filter(|app| app.is_browser())
            .map(|app| app.id)
            .collect();
        assert_eq!(
            browsers,
            [
                "edge", "chrome", "firefox", "brave", "arc", "dia", "vivaldi", "opera", "zen",
                "comet"
            ]
        );
    }

    #[test]
    fn words_need_a_boundary() {
        assert!(has_word("Zoom Meeting", "Zoom"));
        assert!(has_word("(2) Zoom", "Zoom"));
        assert!(has_word("Huddle: #geral – Slack", "Huddle"));
        assert!(!has_word("Zoomer - YouTube", "Zoom"));
        assert!(!has_word("ZoomZoom", "Zoom"));
        assert!(!has_word("Discordância", "Discord"));
    }
}
