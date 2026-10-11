//! Detecção de call com atividade de mic simulada (checks de `meeting-detection`).

#![allow(clippy::unwrap_used)]

use std::time::Duration;

use fala_meeting::{
    identify, CallApp, CallDetector, CallPrompt, DetectContext, Input, MeetingSession,
    RecordingCap, SessionConfig, SessionId, SessionMode, SessionState, UnixMillis, UserAction,
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

/// Lê uma vez por segundo de `from` a `to` (inclusive) e devolve `(segundo, aviso)`.
fn run(
    detector: &mut CallDetector,
    from: u64,
    to: u64,
    present: &[&'static CallApp],
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
    assert_eq!(prompts, [(12, CallPrompt::OfferRecording { app: teams })]);
    assert_eq!(d.holding().collect::<Vec<_>>(), [teams]);
}

#[test]
fn a_short_probe_or_a_flicker_is_not_a_new_call() {
    let teams = app("teams");
    let mut d = CallDetector::new();
    // O teste de mic de 1 s antes de entrar não avisa.
    assert!(run(&mut d, 0, 0, &[teams], idle(&[])).is_empty());
    assert!(run(&mut d, 1, 5, &[], idle(&[])).is_empty());
    let first = run(&mut d, 6, 20, &[teams], idle(&[]));
    assert_eq!(first, [(8, CallPrompt::OfferRecording { app: teams })]);
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
        [(602, CallPrompt::OfferRecording { app: teams })]
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
    assert_eq!(prompts, [(182, CallPrompt::AskStop { app: teams })]);

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
    let teams = app("teams");
    let edge = app("edge");
    let mut d = CallDetector::new();
    run(&mut d, 0, 300, &[teams, edge], recording(300));
    // O Edge segue com o mic: ambíguo, não pergunta.
    assert!(run(&mut d, 301, 600, &[edge], recording(600)).is_empty());
    assert_eq!(
        run(&mut d, 601, 700, &[], recording(700)),
        [(603, CallPrompt::AskStop { app: edge })]
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
