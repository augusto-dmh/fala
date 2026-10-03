//! Comportamento da sessão de reunião, pela API pública (checks de `meeting-session`).

// Os helpers abaixo ficam fora de `#[test]`, onde o `allow-unwrap-in-tests` do clippy não chega.
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use fala_meeting::{
    Effect, Gap, GapKind, IndicatorKind, Input, Levels, MeetingSession, RecordingCap,
    SessionConfig, SessionError, SessionId, SessionMode, SessionState, StopReason, UnixMillis,
    UserAction,
};

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;
const MIN: Duration = Duration::from_secs(60);
const HOUR: Duration = Duration::from_secs(3600);
const T0: UnixMillis = UnixMillis(1_727_000_000_000);

fn mins(m: u64) -> Duration {
    Duration::from_secs(m * 60)
}

fn at(minutes: u64) -> UnixMillis {
    UnixMillis(T0.0 + minutes * 60_000)
}

fn session(mode: SessionMode) -> MeetingSession {
    session_with_cap(mode, RecordingCap::default())
}

fn session_with_cap(mode: SessionMode, cap: RecordingCap) -> MeetingSession {
    MeetingSession::new(SessionConfig {
        id: SessionId::from_parts(T0.0, 7),
        mode,
        title: "Reunião de teste".to_string(),
        local_only: false,
        cap,
    })
}

fn start() -> Input {
    Input::User(UserAction::StartRecording {
        free_disk_bytes: 10 * GIB,
    })
}

fn user(action: UserAction) -> Input {
    Input::User(action)
}

fn tick(recorded: Duration, mic: f32, system: f32) -> Input {
    Input::Tick {
        recorded,
        levels: Levels { mic, system },
    }
}

fn loud(recorded: Duration) -> Input {
    tick(recorded, 0.2, 0.2)
}

fn recording(mode: SessionMode) -> MeetingSession {
    let mut s = session(mode);
    s.apply(T0, start()).unwrap();
    s
}

fn paused() -> MeetingSession {
    let mut s = recording(SessionMode::Meeting);
    s.apply(at(1), user(UserAction::PauseRecording)).unwrap();
    s
}

fn suspended() -> MeetingSession {
    let mut s = recording(SessionMode::Meeting);
    s.apply(at(1), Input::OsSuspended).unwrap();
    s
}

fn stopping() -> MeetingSession {
    let mut s = recording(SessionMode::Meeting);
    s.apply(at(1), user(UserAction::StopRecording)).unwrap();
    s
}

fn stopped() -> MeetingSession {
    let mut s = stopping();
    s.apply(at(2), Input::CaptureFinalized).unwrap();
    s
}

/// Todas as entradas possíveis, uma de cada forma.
fn every_input() -> Vec<Input> {
    vec![
        start(),
        user(UserAction::PauseRecording),
        user(UserAction::ResumeRecording),
        user(UserAction::StopRecording),
        user(UserAction::ExtendCap),
        tick(Duration::from_secs(30), 0.0, 0.0),
        Input::OsSuspended,
        Input::OsResumed,
        Input::CaptureFinalized,
    ]
}

fn kind(s: &MeetingSession) -> Option<IndicatorKind> {
    s.indicator().map(|i| i.kind())
}

fn opens_capture(effects: &[Effect]) -> bool {
    effects
        .iter()
        .any(|e| matches!(e, Effect::StartCapture | Effect::ResumeCapture))
}

mod start {
    use super::*;

    #[test]
    fn new_session_is_idle() {
        let s = session(SessionMode::Meeting);
        assert_eq!(s.state(), SessionState::Idle);
        assert_eq!(s.indicator(), None);
        assert_eq!(s.recorded(), Duration::ZERO);
        assert_eq!(s.started_at(), None);
    }

    #[test]
    fn with_enough_disk_shows_indicator_before_capture() {
        let mut s = session(SessionMode::Meeting);
        let effects = s
            .apply(
                T0,
                user(UserAction::StartRecording {
                    free_disk_bytes: 2 * GIB,
                }),
            )
            .unwrap();
        assert_eq!(
            effects,
            vec![
                Effect::ShowIndicator(IndicatorKind::Recording),
                Effect::StartCapture
            ]
        );
        assert!(matches!(s.state(), SessionState::Recording { .. }));
        assert_eq!(s.started_at(), Some(T0));
    }

    #[test]
    fn only_start_recording_leaves_idle() {
        let inputs = every_input();
        assert_eq!(inputs.len(), 9);
        for input in inputs.into_iter().skip(1) {
            let mut s = session(SessionMode::Meeting);
            let result = s.apply(T0, input);
            if let Ok(effects) = &result {
                assert!(!opens_capture(effects), "{input:?} -> {effects:?}");
            }
            assert_eq!(s.state(), SessionState::Idle, "{input:?}");
            assert_eq!(s.indicator(), None, "{input:?}");
        }
    }

    #[test]
    fn indicator_present_exactly_while_capturing_or_waiting() {
        assert_eq!(kind(&session(SessionMode::Meeting)), None);
        assert_eq!(
            kind(&recording(SessionMode::Meeting)),
            Some(IndicatorKind::Recording)
        );
        assert_eq!(kind(&paused()), Some(IndicatorKind::Paused));
        assert_eq!(kind(&suspended()), Some(IndicatorKind::Suspended));
        assert_eq!(kind(&stopping()), None);
        assert_eq!(kind(&stopped()), None);
    }

    #[test]
    fn refused_below_500_mib() {
        let mut s = session(SessionMode::Meeting);
        let result = s.apply(
            T0,
            user(UserAction::StartRecording {
                free_disk_bytes: 400 * MIB,
            }),
        );
        assert_eq!(
            result,
            Err(SessionError::InsufficientDisk {
                free_bytes: 400 * MIB,
                missing_bytes: 100 * MIB,
            })
        );
        assert_eq!(s.state(), SessionState::Idle);

        let mut s = session(SessionMode::Meeting);
        let effects = s
            .apply(
                T0,
                user(UserAction::StartRecording {
                    free_disk_bytes: 500 * MIB,
                }),
            )
            .unwrap();
        assert!(effects.contains(&Effect::StartCapture));
        assert!(matches!(s.state(), SessionState::Recording { .. }));
    }

    #[test]
    fn warns_below_2_gib() {
        let mut s = session(SessionMode::Meeting);
        let effects = s
            .apply(
                T0,
                user(UserAction::StartRecording {
                    free_disk_bytes: GIB,
                }),
            )
            .unwrap();
        assert_eq!(
            effects,
            vec![
                Effect::WarnLowDisk { free_bytes: GIB },
                Effect::ShowIndicator(IndicatorKind::Recording),
                Effect::StartCapture,
            ]
        );

        let mut s = session(SessionMode::Meeting);
        let effects = s
            .apply(
                T0,
                user(UserAction::StartRecording {
                    free_disk_bytes: 2 * GIB,
                }),
            )
            .unwrap();
        assert!(!effects
            .iter()
            .any(|e| matches!(e, Effect::WarnLowDisk { .. })));
    }

    #[test]
    fn import_mode_does_not_record() {
        let mut s = session(SessionMode::Import);
        assert_eq!(
            s.apply(T0, start()),
            Err(SessionError::ModeDoesNotRecord(SessionMode::Import))
        );
        assert_eq!(s.state(), SessionState::Idle);

        for mode in [
            SessionMode::Meeting,
            SessionMode::InPerson,
            SessionMode::SystemOnly,
        ] {
            let s = recording(mode);
            assert!(
                matches!(s.state(), SessionState::Recording { .. }),
                "{mode:?}"
            );
        }
    }
}

mod pause {
    use super::*;

    #[test]
    fn from_recording() {
        let mut s = recording(SessionMode::Meeting);
        let effects = s.apply(at(1), user(UserAction::PauseRecording)).unwrap();
        assert_eq!(
            effects,
            vec![
                Effect::PauseCapture,
                Effect::ShowIndicator(IndicatorKind::Paused)
            ]
        );
        assert!(matches!(s.state(), SessionState::Paused { .. }));
    }

    #[test]
    fn resume_returns_to_recording() {
        let mut s = paused();
        let effects = s.apply(at(2), user(UserAction::ResumeRecording)).unwrap();
        assert_eq!(
            effects,
            vec![
                Effect::ShowIndicator(IndicatorKind::Recording),
                Effect::ResumeCapture
            ]
        );
        assert!(matches!(s.state(), SessionState::Recording { .. }));
    }

    #[test]
    fn only_resume_recording_resumes() {
        for (name, make) in [
            ("paused", paused as fn() -> MeetingSession),
            ("suspended", suspended),
        ] {
            for input in every_input() {
                if input == user(UserAction::ResumeRecording) {
                    continue;
                }
                let mut s = make();
                if let Ok(effects) = s.apply(at(5), input) {
                    assert!(!opens_capture(&effects), "{name}: {input:?} -> {effects:?}");
                }
                assert!(
                    !matches!(s.state(), SessionState::Recording { .. }),
                    "{name}: {input:?}"
                );
            }
        }
    }

    #[test]
    fn gap_recorded() {
        let mut s = recording(SessionMode::Meeting);
        s.apply(at(10), loud(10 * MIN)).unwrap();
        s.apply(at(10), user(UserAction::PauseRecording)).unwrap();
        s.apply(at(13), user(UserAction::ResumeRecording)).unwrap();
        assert_eq!(
            s.gaps(),
            &[Gap {
                kind: GapKind::Pause,
                audio_offset: 10 * MIN,
                from: at(10),
                to: Some(at(13)),
            }]
        );
    }

    #[test]
    fn invalid_action_keeps_state() {
        let cases = [
            (paused(), UserAction::PauseRecording),
            (session(SessionMode::Meeting), UserAction::StopRecording),
            (
                stopping(),
                UserAction::StartRecording {
                    free_disk_bytes: 10 * GIB,
                },
            ),
            (stopped(), UserAction::ResumeRecording),
            // Clique duplo em "Gravar": não reinicia a captura.
            (
                recording(SessionMode::Meeting),
                UserAction::StartRecording {
                    free_disk_bytes: 10 * GIB,
                },
            ),
        ];
        for (mut s, action) in cases {
            let before = s.state();
            assert_eq!(
                s.apply(at(9), user(action)),
                Err(SessionError::InvalidTransition {
                    state: before,
                    action
                })
            );
            assert_eq!(s.state(), before);
        }
    }
}

mod stop {
    use super::*;

    #[test]
    fn by_user_from_each_live_state() {
        for (name, mut s) in [
            ("recording", recording(SessionMode::Meeting)),
            ("paused", paused()),
            ("suspended", suspended()),
        ] {
            let effects = s.apply(at(5), user(UserAction::StopRecording)).unwrap();
            assert_eq!(effects, vec![Effect::FinalizeCapture], "{name}");
            assert_eq!(
                s.state(),
                SessionState::Stopping {
                    reason: StopReason::User
                },
                "{name}"
            );
        }
    }

    #[test]
    fn finalized_hides_indicator_then_processes() {
        let mut s = stopping();
        let effects = s.apply(at(2), Input::CaptureFinalized).unwrap();
        assert_eq!(effects, vec![Effect::HideIndicator, Effect::Process]);
        assert_eq!(
            s.state(),
            SessionState::Stopped {
                reason: StopReason::User
            }
        );
    }

    fn run(s: &mut MeetingSession, steps: &[(UnixMillis, Input)]) -> Vec<Effect> {
        let mut all = Vec::new();
        for (now, input) in steps {
            all.extend(s.apply(*now, *input).unwrap());
        }
        all
    }

    #[test]
    fn every_path_processes_once() {
        let silent = |m: u64| tick(mins(m), 0.0, 0.0);
        let paths: Vec<(&str, Vec<(UnixMillis, Input)>)> = vec![
            (
                "user",
                vec![
                    (T0, start()),
                    (at(1), user(UserAction::StopRecording)),
                    (at(1), Input::CaptureFinalized),
                ],
            ),
            (
                "silence",
                vec![
                    (T0, start()),
                    (at(5), silent(5)),
                    (at(15), silent(15)),
                    (at(15), Input::CaptureFinalized),
                ],
            ),
            (
                "cap",
                vec![
                    (T0, start()),
                    (at(180), loud(3 * HOUR)),
                    (at(180), Input::CaptureFinalized),
                ],
            ),
            (
                "suspend then stop",
                vec![
                    (T0, start()),
                    (at(1), Input::OsSuspended),
                    (at(30), Input::OsResumed),
                    (at(31), user(UserAction::StopRecording)),
                    (at(31), Input::CaptureFinalized),
                ],
            ),
        ];
        for (name, steps) in paths {
            let mut s = session(SessionMode::Meeting);
            let effects = run(&mut s, &steps);
            assert!(
                matches!(s.state(), SessionState::Stopped { .. }),
                "{name}: {:?}",
                s.state()
            );
            let processed = effects.iter().filter(|e| **e == Effect::Process).count();
            assert_eq!(processed, 1, "{name}: {effects:?}");
        }
    }

    #[test]
    fn after_15_min_of_silence() {
        let mut s = recording(SessionMode::Meeting);
        for m in 1..=14 {
            let effects = s.apply(at(m), tick(mins(m), 0.0005, 0.0005)).unwrap();
            assert!(!effects.contains(&Effect::FinalizeCapture), "minute {m}");
        }
        let effects = s.apply(at(15), tick(15 * MIN, 0.0005, 0.0005)).unwrap();
        assert_eq!(effects, vec![Effect::FinalizeCapture]);
        assert_eq!(
            s.state(),
            SessionState::Stopping {
                reason: StopReason::Silence
            }
        );

        // Só sistema: o mic não conta.
        let mut s = recording(SessionMode::SystemOnly);
        for m in 1..=14 {
            s.apply(at(m), tick(mins(m), 0.5, 0.0005)).unwrap();
            assert!(matches!(s.state(), SessionState::Recording { .. }));
        }
        s.apply(at(15), tick(15 * MIN, 0.5, 0.0005)).unwrap();
        assert_eq!(
            s.state(),
            SessionState::Stopping {
                reason: StopReason::Silence
            }
        );

        // Reunião: o mic alto mantém a gravação.
        let mut s = recording(SessionMode::Meeting);
        for m in 1..=20 {
            s.apply(at(m), tick(mins(m), 0.5, 0.0005)).unwrap();
        }
        assert!(matches!(s.state(), SessionState::Recording { .. }));
    }

    #[test]
    fn silence_counter_resets_on_sound() {
        let mut s = recording(SessionMode::Meeting);
        for m in 1..=13 {
            s.apply(at(m), tick(mins(m), 0.0, 0.0)).unwrap();
        }
        s.apply(at(14), tick(14 * MIN, 0.01, 0.0)).unwrap();
        for m in 15..=28 {
            s.apply(at(m), tick(mins(m), 0.0, 0.0)).unwrap();
            assert!(
                matches!(s.state(), SessionState::Recording { .. }),
                "minute {m}"
            );
        }
        s.apply(at(29), tick(29 * MIN, 0.0, 0.0)).unwrap();
        assert_eq!(
            s.state(),
            SessionState::Stopping {
                reason: StopReason::Silence
            }
        );
    }
}

mod cap {
    use super::*;

    #[test]
    fn warns_once_ten_minutes_before() {
        let mut s = recording(SessionMode::Meeting);
        let effects = s.apply(at(169), loud(169 * MIN)).unwrap();
        assert!(effects.is_empty());
        let effects = s.apply(at(170), loud(170 * MIN)).unwrap();
        assert_eq!(
            effects,
            vec![Effect::CapWarning {
                remaining: 10 * MIN
            }]
        );
        for m in 171..=179 {
            let effects = s.apply(at(m), loud(mins(m))).unwrap();
            assert!(effects.is_empty(), "minute {m}: {effects:?}");
        }
    }

    #[test]
    fn reached_stops() {
        let mut s = recording(SessionMode::Meeting);
        s.apply(at(179), loud(179 * MIN)).unwrap();
        let effects = s.apply(at(180), loud(180 * MIN)).unwrap();
        assert_eq!(effects, vec![Effect::FinalizeCapture]);
        assert_eq!(
            s.state(),
            SessionState::Stopping {
                reason: StopReason::CapReached
            }
        );
    }

    #[test]
    fn extend_adds_one_hour_and_rearms() {
        // Em Recording.
        let mut s = recording(SessionMode::Meeting);
        s.apply(at(170), loud(170 * MIN)).unwrap();
        assert!(s
            .apply(at(171), user(UserAction::ExtendCap))
            .unwrap()
            .is_empty());
        assert_eq!(s.cap().hours(), 4);
        let effects = s.apply(at(180), loud(180 * MIN)).unwrap();
        assert!(effects.is_empty(), "{effects:?}");
        assert!(matches!(s.state(), SessionState::Recording { .. }));
        let effects = s.apply(at(230), loud(230 * MIN)).unwrap();
        assert_eq!(
            effects,
            vec![Effect::CapWarning {
                remaining: 10 * MIN
            }]
        );

        // Em Paused, depois de o aviso já ter saído.
        let mut s = recording(SessionMode::Meeting);
        let effects = s.apply(at(170), loud(170 * MIN)).unwrap();
        assert_eq!(effects.len(), 1);
        s.apply(at(171), user(UserAction::PauseRecording)).unwrap();
        assert!(s
            .apply(at(172), user(UserAction::ExtendCap))
            .unwrap()
            .is_empty());
        assert_eq!(s.cap().hours(), 4);
        s.apply(at(173), user(UserAction::ResumeRecording)).unwrap();
        let effects = s.apply(at(233), loud(230 * MIN)).unwrap();
        assert_eq!(
            effects,
            vec![Effect::CapWarning {
                remaining: 10 * MIN
            }]
        );
    }

    #[test]
    fn extend_refused_at_8_hours() {
        let max = RecordingCap::from_hours(8).unwrap();
        let mut s = session_with_cap(SessionMode::Meeting, max);
        s.apply(T0, start()).unwrap();
        assert_eq!(
            s.apply(at(1), user(UserAction::ExtendCap)),
            Err(SessionError::CapAtMaximum)
        );
        assert_eq!(s.cap().hours(), 8);
        assert!(matches!(s.state(), SessionState::Recording { .. }));
    }

    #[test]
    fn ticks_ignored_while_not_recording() {
        for (name, mut s) in [("paused", paused()), ("suspended", suspended())] {
            let before = s.clone();
            let effects = s.apply(at(300), tick(5 * HOUR, 0.0, 0.0)).unwrap();
            assert!(effects.is_empty(), "{name}: {effects:?}");
            assert_eq!(s.recorded(), Duration::ZERO, "{name}");
            assert_eq!(s, before, "{name}");
        }
    }
}

mod suspend {
    use super::*;

    #[test]
    fn from_recording_and_paused() {
        for (name, mut s) in [
            ("recording", recording(SessionMode::Meeting)),
            ("paused", paused()),
        ] {
            let effects = s.apply(at(3), Input::OsSuspended).unwrap();
            assert_eq!(
                effects,
                vec![
                    Effect::FinalizeCapture,
                    Effect::ShowIndicator(IndicatorKind::Suspended)
                ],
                "{name}"
            );
            assert!(
                matches!(s.state(), SessionState::Suspended { .. }),
                "{name}"
            );
        }
    }

    #[test]
    fn os_resume_asks() {
        let mut s = suspended();
        let effects = s.apply(at(30), Input::OsResumed).unwrap();
        assert_eq!(effects, vec![Effect::AskContinueOrStop]);
        assert!(matches!(s.state(), SessionState::Suspended { .. }));
    }

    #[test]
    fn user_resume_records_gap() {
        let mut s = recording(SessionMode::Meeting);
        s.apply(at(4), loud(4 * MIN)).unwrap();
        s.apply(at(4), Input::OsSuspended).unwrap();
        s.apply(at(40), Input::OsResumed).unwrap();
        let effects = s.apply(at(41), user(UserAction::ResumeRecording)).unwrap();
        assert_eq!(
            effects,
            vec![
                Effect::ShowIndicator(IndicatorKind::Recording),
                Effect::ResumeCapture
            ]
        );
        assert!(matches!(s.state(), SessionState::Recording { .. }));
        assert_eq!(
            s.gaps(),
            &[Gap {
                kind: GapKind::Suspend,
                audio_offset: 4 * MIN,
                from: at(4),
                to: Some(at(41)),
            }]
        );
    }
}
