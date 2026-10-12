//! Detecção de call com atividade de mic simulada (checks de `meeting-detection`).

#![allow(clippy::unwrap_used)]

use std::time::Duration;

use fala_meeting::{
    identify, may_ask_stop, web_call, CallApp, CallDetector, CallHolder, CallPrompt, DetectContext,
    Input, MeetingSession, RecordingCap, SessionConfig, SessionId, SessionMode, SessionState,
    UnixMillis, UserAction, WebCall, ASK_STOP_MAX,
};

const T0: u64 = 1_727_000_000_000;
const GIB: u64 = 1024 * 1024 * 1024;

fn at(s: u64) -> UnixMillis {
    UnixMillis(T0 + s * 1000)
}

fn app(id: &str) -> &'static CallApp {
    CallApp::by_id(id).unwrap()
}

fn idle(muted: &[String]) -> DetectContext<'_> {
    DetectContext {
        enabled: true,
        muted,
        recording: None,
    }
}

fn recording(secs: u64) -> DetectContext<'static> {
    DetectContext {
        enabled: true,
        muted: &[],
        recording: Some(Duration::from_secs(secs)),
    }
}

fn offer(app: &'static CallApp) -> CallPrompt {
    CallPrompt::OfferRecording { app, web: None }
}

fn ask(app: &'static CallApp) -> CallPrompt {
    CallPrompt::AskStop { app, web: None }
}

fn meet() -> &'static WebCall {
    web_call(["Meet - abc-defg-hij - Microsoft Edge"]).unwrap()
}

/// O navegador com o mic e uma janela que mostra `web` (ou nenhuma reunião).
fn browser(id: &str, web: Option<&'static WebCall>) -> CallHolder {
    CallHolder { app: app(id), web }
}

/// Lê uma vez por segundo de `from` a `to` (inclusive) e devolve `(segundo, aviso)`.
fn run(
    detector: &mut CallDetector,
    from: u64,
    to: u64,
    present: &[&'static CallApp],
    ctx: DetectContext<'_>,
) -> Vec<(u64, CallPrompt)> {
    let present: Vec<CallHolder> = present.iter().map(|&app| app.into()).collect();
    run_holders(detector, from, to, &present, ctx)
}

fn run_holders(
    detector: &mut CallDetector,
    from: u64,
    to: u64,
    present: &[CallHolder],
    ctx: DetectContext<'_>,
) -> Vec<(u64, CallPrompt)> {
    (from..=to)
        .filter_map(|s| detector.observe(at(s), present, ctx).map(|p| (s, p)))
        .collect()
}

#[test]
fn identifies_apps_by_exe_and_by_webview_parent() {
    let id = |chain: &[&str]| identify(chain.iter().copied(), "fala.exe").map(|a| a.id);
    assert_eq!(id(&["ms-teams.exe"]), Some("teams"));
    assert_eq!(
        id(&[
            "msedgewebview2.exe",
            "msedgewebview2.exe",
            "ms-teams.exe",
            "explorer.exe"
        ]),
        Some("teams")
    );
    assert_eq!(
        id(&["msedgewebview2.exe", "whatsapp.root.exe"]),
        Some("whatsapp")
    );
    assert_eq!(id(&["msedge.exe", "explorer.exe"]), Some("edge"));
    assert_eq!(id(&["zoom.exe"]), Some("zoom"));
    // Um app qualquer com o mic não é call, mesmo filho de um app de call.
    assert_eq!(id(&["voicemeeterpro.exe", "explorer.exe"]), None);
    assert_eq!(id(&["obs64.exe", "discord.exe"]), None);
    assert_eq!(id(&[]), None);
}

#[test]
fn fala_itself_is_never_a_call() {
    assert_eq!(identify(["fala.exe"], "fala.exe"), None);
    // O WebView2 da janela do Fala.
    assert_eq!(identify(["msedgewebview2.exe", "fala.exe"], ""), None);
    // O exe do Fala com outro nome (build de dev, portable) vem do chamador.
    assert_eq!(
        identify(["msedgewebview2.exe", "fala-dev.exe"], "fala-dev.exe"),
        None
    );
    // Mesmo que alguém o chame como um app de call.
    assert_eq!(identify(["ms-teams.exe"], "ms-teams.exe"), None);
}

#[test]
fn one_offer_per_call_after_two_seconds() {
    let teams = app("teams");
    let mut d = CallDetector::new();
    // Uma call de 10 min: o Teams pega o mic em t = 10 s.
    assert!(run(&mut d, 0, 9, &[], idle(&[])).is_empty());
    let prompts = run(&mut d, 10, 600, &[teams], idle(&[]));
    assert_eq!(prompts, [(12, offer(teams))]);
    assert_eq!(d.holding().collect::<Vec<_>>(), [CallHolder::from(teams)]);
}

#[test]
fn a_short_probe_or_a_flicker_is_not_a_new_call() {
    let teams = app("teams");
    let mut d = CallDetector::new();
    // O teste de mic de 1 s antes de entrar não avisa.
    assert!(run(&mut d, 0, 0, &[teams], idle(&[])).is_empty());
    assert!(run(&mut d, 1, 5, &[], idle(&[])).is_empty());
    let first = run(&mut d, 6, 20, &[teams], idle(&[]));
    assert_eq!(first, [(8, offer(teams))]);
    // Troca de dispositivo: 1 s sem o mic é a mesma call, mesmo depois de 2 min.
    assert!(run(&mut d, 200, 200, &[], idle(&[])).is_empty());
    assert!(run(&mut d, 201, 400, &[teams], idle(&[])).is_empty());
}

#[test]
fn a_new_call_after_the_gap_gets_a_new_offer() {
    let teams = app("teams");
    let zoom = app("zoom");
    let mut d = CallDetector::new();
    assert_eq!(run(&mut d, 0, 60, &[teams], idle(&[])).len(), 1);
    // O Teams sai em 61 s e o Zoom entra em 70 s: menos de 2 min do aviso, fica quieto.
    assert!(run(&mut d, 61, 69, &[], idle(&[])).is_empty());
    assert!(run(&mut d, 70, 100, &[zoom], idle(&[])).is_empty());
    // Outra call do Teams às 10 min: aviso de novo.
    assert!(run(&mut d, 101, 110, &[], idle(&[])).is_empty());
    assert_eq!(
        run(&mut d, 600, 620, &[teams], idle(&[])),
        [(602, offer(teams))]
    );
}

#[test]
fn muted_apps_and_the_switch_stay_silent() {
    let whatsapp = app("whatsapp");
    let muted = vec!["whatsapp".to_owned()];
    let mut d = CallDetector::new();
    assert!(run(&mut d, 0, 300, &[whatsapp], idle(&muted)).is_empty());

    let off = DetectContext {
        enabled: false,
        muted: &[],
        recording: None,
    };
    let mut d = CallDetector::new();
    assert!(run(&mut d, 0, 300, &[app("teams")], off).is_empty());
}

#[test]
fn no_offer_while_recording_nor_after_it_for_the_same_call() {
    let teams = app("teams");
    let mut d = CallDetector::new();
    // A pessoa clicou Gravar antes da call; o Teams pega o mic gravando.
    assert!(run(&mut d, 0, 100, &[teams], recording(0)).is_empty());
    // Parou a gravação com a call ainda no ar: sem aviso pela mesma call.
    assert!(run(&mut d, 101, 400, &[teams], idle(&[])).is_empty());
}

#[test]
fn the_app_leaving_the_mic_only_asks_after_three_minutes() {
    let teams = app("teams");
    let mut d = CallDetector::new();
    let mut prompts = run(&mut d, 0, 179, &[teams], recording(0));
    // Larga o mic com 3 min gravados: pergunta uma vez, depois de 2 s.
    prompts.extend(run(&mut d, 180, 400, &[], recording(180)));
    assert_eq!(prompts, [(182, ask(teams))]);

    // Com menos de 3 min gravados, não pergunta.
    let mut d = CallDetector::new();
    run(&mut d, 0, 60, &[teams], recording(0));
    assert!(run(&mut d, 61, 400, &[], recording(170)).is_empty());

    // Sem nenhum app de call durante a gravação (presencial), nunca pergunta.
    let mut d = CallDetector::new();
    assert!(run(&mut d, 0, 4000, &[], recording(4000)).is_empty());
}

#[test]
fn asks_only_when_every_call_app_left() {
    let teams = CallHolder::from(app("teams"));
    let edge = browser("edge", Some(meet()));
    let mut d = CallDetector::new();
    run_holders(&mut d, 0, 300, &[teams, edge], recording(300));
    // O Meet no Edge segue com o mic: ambíguo, não pergunta.
    assert!(run_holders(&mut d, 301, 600, &[edge], recording(600)).is_empty());
    assert_eq!(
        run_holders(&mut d, 601, 700, &[], recording(700)),
        [(
            603,
            CallPrompt::AskStop {
                app: edge.app,
                web: Some(meet())
            }
        )]
    );

    // Um navegador sem reunião (o modo de voz de um site) não segura a pergunta.
    let mut d = CallDetector::new();
    let voice = browser("edge", None);
    run_holders(&mut d, 0, 300, &[teams, voice], recording(300));
    assert_eq!(
        run_holders(&mut d, 301, 400, &[voice], recording(400)),
        [(303, ask(teams.app))]
    );
}

#[test]
fn browser_titles_name_the_web_call() {
    let name = |title: &str| web_call([title]).map(|call| call.name);
    assert_eq!(
        name("Meet - abc-defg-hij - Microsoft Edge"),
        Some("Google Meet")
    );
    assert_eq!(
        name("Meet – Daily sync e mais 2 páginas - Pessoal - Microsoft Edge"),
        Some("Google Meet")
    );
    assert_eq!(name("Google Meet - Google Chrome"), Some("Google Meet"));
    assert_eq!(
        name("Chat | Microsoft Teams - Microsoft Edge"),
        Some("Microsoft Teams")
    );
    assert_eq!(name("Zoom Meeting - Google Chrome"), Some("Zoom"));
    assert_eq!(name("Jitsi Meet — Mozilla Firefox"), Some("Jitsi"));
    assert_eq!(name("Huddle: #geral - Acme - Slack"), Some("Slack"));
    // Nada de reunião: o claude.ai, uma página sobre reuniões, um teste de mic.
    for title in [
        "Claude - Microsoft Edge",
        "Meeting notes - Notion - Microsoft Edge",
        "How to meet new people - Google Chrome",
        "Online Mic Test - Microsoft Edge",
        "",
    ] {
        assert_eq!(name(title), None, "{title}");
    }
    // Vale a primeira janela com reunião; as outras não importam.
    assert_eq!(
        web_call([
            "Claude - Microsoft Edge",
            "Meet - abc-defg-hij - Microsoft Edge"
        ])
        .map(|call| call.name),
        Some("Google Meet")
    );
    assert_eq!(web_call(std::iter::empty::<&str>()), None);
}

#[test]
fn a_browser_offers_only_once_a_window_shows_a_meeting() {
    let edge = app("edge");
    let mut d = CallDetector::new();
    // O modo de voz do claude.ai no Edge: 5 min com o mic, nenhum aviso.
    assert!(run_holders(&mut d, 0, 300, &[browser("edge", None)], idle(&[])).is_empty());
    assert!(d.holding().next().is_none());
    // A pessoa entra num Meet sem o Edge largar o mic: avisa já, com o nome da reunião.
    assert_eq!(
        run_holders(
            &mut d,
            301,
            400,
            &[browser("edge", Some(meet()))],
            idle(&[])
        ),
        [(
            301,
            CallPrompt::OfferRecording {
                app: edge,
                web: Some(meet())
            }
        )]
    );

    // Gravação presencial com o modo de voz no Edge; depois de parar, um Meet no mesmo Edge,
    // sem largar o mic, ainda avisa.
    let mut d = CallDetector::new();
    run_holders(&mut d, 0, 300, &[browser("edge", None)], recording(300));
    run_holders(&mut d, 301, 310, &[browser("edge", None)], idle(&[]));
    assert_eq!(
        run_holders(
            &mut d,
            311,
            320,
            &[browser("edge", Some(meet()))],
            idle(&[])
        ),
        [(
            311,
            CallPrompt::OfferRecording {
                app: edge,
                web: Some(meet())
            }
        )]
    );

    // O Meet abre direto: o aviso espera os mesmos 2 s de um app de call.
    let mut d = CallDetector::new();
    let prompts = run_holders(&mut d, 0, 60, &[browser("edge", Some(meet()))], idle(&[]));
    assert_eq!(prompts.len(), 1);
    assert_eq!(prompts[0].0, 2);
}

#[test]
fn switching_tabs_during_a_recorded_meet_is_not_leaving_it() {
    let mut d = CallDetector::new();
    run_holders(
        &mut d,
        0,
        300,
        &[browser("edge", Some(meet()))],
        recording(300),
    );
    // Outra aba em primeiro plano: o título perde o Meet, o Edge segue com o mic.
    assert!(run_holders(&mut d, 301, 600, &[browser("edge", None)], recording(600)).is_empty());
    assert_eq!(
        run_holders(&mut d, 601, 700, &[], recording(700)),
        [(
            603,
            CallPrompt::AskStop {
                app: app("edge"),
                web: Some(meet())
            }
        )]
    );

    // O claude.ai com o mic durante uma gravação presencial: largar o mic não pergunta.
    let mut d = CallDetector::new();
    run_holders(&mut d, 0, 300, &[browser("edge", None)], recording(300));
    assert!(run_holders(&mut d, 301, 600, &[], recording(600)).is_empty());
}

#[test]
fn ask_stop_waits_three_minutes_and_stops_after_two() {
    let t = |s: u64| at(s);
    assert!(may_ask_stop(&[], t(0)));
    assert!(!may_ask_stop(&[t(0)], t(179)));
    assert!(may_ask_stop(&[t(0)], t(180)));
    assert!(!may_ask_stop(&[t(0), t(200)], t(10_000)));
    assert_eq!(ASK_STOP_MAX, 2);

    // Uma reconexão do Teams que larga e pega o mic de novo, várias vezes, numa gravação longa.
    let teams = app("teams");
    let mut d = CallDetector::new();
    let mut asked = run(&mut d, 0, 199, &[teams], recording(200));
    // 1ª: larga em 200 s, pergunta em 202 s; [Continuar].
    asked.extend(run(&mut d, 200, 210, &[], recording(210)));
    // Volta e larga em 251 s: menos de 3 min da anterior, quieto.
    asked.extend(run(&mut d, 211, 250, &[teams], recording(250)));
    asked.extend(run(&mut d, 251, 300, &[], recording(300)));
    // Volta e larga em 401 s: 3 min passaram, 2ª pergunta em 403 s.
    asked.extend(run(&mut d, 301, 400, &[teams], recording(400)));
    asked.extend(run(&mut d, 401, 420, &[], recording(420)));
    // Mais uma, muito depois: o teto de 2 vale até o fim da gravação.
    asked.extend(run(&mut d, 421, 2000, &[teams], recording(2000)));
    asked.extend(run(&mut d, 2001, 2100, &[], recording(2100)));
    assert_eq!(asked, [(202, ask(teams)), (403, ask(teams))]);

    // Uma gravação nova recomeça a conta.
    run(&mut d, 2101, 2110, &[], idle(&[]));
    run(&mut d, 2111, 2400, &[teams], recording(300));
    assert_eq!(
        run(&mut d, 2401, 2500, &[], recording(400)),
        [(2403, ask(teams))]
    );
}

/// ADR-0005: o detector só pergunta. Com uma call inteira e ninguém clicando, a sessão não
/// sai de `Idle`; ela só grava quando o clique vira `UserAction::StartRecording`.
#[test]
fn never_starts_recording_without_a_click() {
    let teams = app("teams");
    let session = || {
        MeetingSession::new(SessionConfig {
            id: SessionId::from_parts(T0, 0),
            mode: SessionMode::Meeting,
            title: String::new(),
            local_only: false,
            cap: RecordingCap::default(),
        })
    };

    // Ninguém clica: o aviso aparece, expira, e nada grava.
    let mut nobody = session();
    let mut d = CallDetector::new();
    let prompts = run(&mut d, 0, 3600, &[teams], idle(&[]));
    assert_eq!(prompts.len(), 1);
    for (_, prompt) in &prompts {
        // Não existe conversão de `CallPrompt` em `Input`: o que a UI faz com o aviso é
        // mostrá-lo. O match exaustivo quebra se uma variante nova aparecer.
        match prompt {
            CallPrompt::OfferRecording { .. } | CallPrompt::AskStop { .. } => {}
        }
    }
    assert_eq!(nobody.state(), SessionState::Idle);
    assert!(nobody.indicator().is_none());
    // A pergunta "parar?" também não para nada: só a ação do usuário para.
    let mut d = CallDetector::new();
    let mut recorded = session();
    recorded
        .apply(
            at(0),
            Input::User(UserAction::StartRecording {
                free_disk_bytes: 10 * GIB,
            }),
        )
        .unwrap();
    run(&mut d, 0, 600, &[teams], recording(600));
    let asked = run(&mut d, 601, 700, &[], recording(700));
    assert_eq!(asked.len(), 1);
    assert!(matches!(recorded.state(), SessionState::Recording { .. }));

    // A pessoa clica [Gravar]: o clique, não o aviso, inicia a sessão.
    let effects = nobody
        .apply(
            at(3601),
            Input::User(UserAction::StartRecording {
                free_disk_bytes: 10 * GIB,
            }),
        )
        .unwrap();
    assert!(matches!(nobody.state(), SessionState::Recording { .. }));
    assert!(!effects.is_empty());
}
