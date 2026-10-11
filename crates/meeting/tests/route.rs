//! A regra do aviso de saída e o zero digital, com níveis simulados em passos de 250 ms (o ritmo
//! dos relatórios do gravador).

use std::time::Duration;

use fala_meeting::{
    MuteWatch, OutputWatch, SessionMode, ZeroWatch, Zeroed, ELSEWHERE_AFTER, MUTE_WARN_AFTER,
    SILENCE_RMS, ZERO_WARN_AFTER,
};

const STEP: Duration = Duration::from_millis(250);
const SPEAKERS: &str = "Alto-falantes (Realtek(R) Audio)";
const HEADSET: &str = "Fone de ouvido (USB Audio Device)";
const VOICE: f32 = 0.3;
const MIC_NOISE: f32 = 0.0004;

/// Observa `steps` passos a partir de `from` com os mesmos picos; devolve o último aviso.
fn outputs(
    watch: &mut OutputWatch,
    from: &mut Duration,
    steps: u32,
    system: f32,
    peaks: &[(&str, f32)],
) -> Option<String> {
    let mut last = None;
    for _ in 0..steps {
        *from += STEP;
        last = watch
            .observe(*from, system, peaks.iter().copied())
            .map(str::to_owned);
    }
    last
}

#[test]
fn silent_channel_and_sound_elsewhere_for_10_s_warns() {
    let mut watch = OutputWatch::new(SPEAKERS);
    let mut t = Duration::ZERO;
    let peaks = [(SPEAKERS, 0.0), (HEADSET, VOICE)];
    // 9,75 s: ainda não.
    assert_eq!(outputs(&mut watch, &mut t, 39, 0.0, &peaks), None);
    // 10 s: avisa, com o nome do fone.
    assert_eq!(
        outputs(&mut watch, &mut t, 1, 0.0, &peaks).as_deref(),
        Some(HEADSET)
    );
    assert_eq!(ELSEWHERE_AFTER, Duration::from_secs(10));
}

#[test]
fn both_silent_leaves_it_to_the_mute_watch() {
    let mut watch = OutputWatch::new(SPEAKERS);
    let mut mute = MuteWatch::new(SessionMode::Meeting);
    let mut t = Duration::ZERO;
    let peaks = [(SPEAKERS, 0.0), (HEADSET, 0.0)];
    let steps = (MUTE_WARN_AFTER.as_millis() / STEP.as_millis()) as u32;
    assert_eq!(outputs(&mut watch, &mut t, steps, 0.0, &peaks), None);
    let mut muted = mute.muted();
    for i in 1..=steps {
        let levels = fala_meeting::Levels {
            mic: SILENCE_RMS * 4.0,
            system: 0.0,
        };
        muted = mute.observe(STEP * i, levels);
    }
    assert!(muted.system, "o aviso de 2 min continua o mesmo");
}

#[test]
fn sound_on_the_recorded_channel_resets() {
    let mut watch = OutputWatch::new(SPEAKERS);
    let mut t = Duration::ZERO;
    let elsewhere = [(SPEAKERS, 0.0), (HEADSET, VOICE)];
    assert!(outputs(&mut watch, &mut t, 40, 0.0, &elsewhere).is_some());
    // O canal gravado recebe sinal: some na hora, e a contagem recomeça.
    assert_eq!(outputs(&mut watch, &mut t, 1, VOICE, &elsewhere), None);
    assert_eq!(outputs(&mut watch, &mut t, 39, 0.0, &elsewhere), None);
    assert!(outputs(&mut watch, &mut t, 1, 0.0, &elsewhere).is_some());
}

#[test]
fn the_recorded_output_never_counts_as_elsewhere() {
    let mut watch = OutputWatch::new(HEADSET);
    let mut t = Duration::ZERO;
    // O próprio dispositivo gravado com pico e o canal em zero: não é "outro lugar".
    let peaks = [(HEADSET, VOICE)];
    assert_eq!(outputs(&mut watch, &mut t, 200, 0.0, &peaks), None);
}

#[test]
fn speech_pauses_do_not_reset_but_a_long_quiet_does() {
    let mut watch = OutputWatch::new(SPEAKERS);
    let mut t = Duration::ZERO;
    let talk = [(HEADSET, VOICE)];
    let pause = [(HEADSET, 0.0)];
    // 6 s de fala, 3 s de pausa, 4 s de fala: 10 s de som com a pausa no meio.
    assert_eq!(outputs(&mut watch, &mut t, 24, 0.0, &talk), None);
    assert_eq!(outputs(&mut watch, &mut t, 12, 0.0, &pause), None);
    assert!(outputs(&mut watch, &mut t, 16, 0.0, &talk).is_some());
    // 10 s sem som no fone: o aviso apaga.
    assert_eq!(outputs(&mut watch, &mut t, 40, 0.0, &pause), None);
    // Uma saída que some da lista (desplugada) também conta como quieta.
    let mut unplugged = OutputWatch::new(SPEAKERS);
    let mut t = Duration::ZERO;
    assert!(outputs(&mut unplugged, &mut t, 40, 0.0, &talk).is_some());
    assert_eq!(outputs(&mut unplugged, &mut t, 40, 0.0, &[]), None);
}

#[test]
fn the_loudest_run_names_the_device_and_a_switch_restarts() {
    let mut watch = OutputWatch::new(SPEAKERS);
    let mut t = Duration::ZERO;
    let other = "HDMI (NVIDIA High Definition Audio)";
    assert!(outputs(&mut watch, &mut t, 20, 0.0, &[(other, VOICE)]).is_none());
    assert_eq!(
        outputs(
            &mut watch,
            &mut t,
            60,
            0.0,
            &[(other, 0.0), (HEADSET, VOICE)]
        )
        .as_deref(),
        Some(HEADSET)
    );
    watch.set_recording(HEADSET);
    assert_eq!(watch.recording(), HEADSET);
    assert_eq!(watch.elsewhere(), None);
    assert_eq!(
        outputs(&mut watch, &mut t, 80, 0.0, &[(HEADSET, VOICE)]),
        None
    );
}

#[test]
fn paused_time_does_not_count_for_either_rule() {
    let mut watch = OutputWatch::new(SPEAKERS);
    let mut zero = ZeroWatch::new(SessionMode::Meeting);
    // 2 s gravados e depois a pausa: observar o mesmo valor não soma nada.
    let at = Duration::from_secs(2);
    for _ in 0..1_000 {
        watch.observe(at, 0.0, [(HEADSET, VOICE)]);
        zero.observe(at, 0.0, 0.0);
    }
    assert_eq!(watch.elsewhere(), None);
    assert_eq!(zero.zeroed(), Zeroed::default());
}

#[test]
fn digital_zero_turns_the_pill_amber_after_3_s() {
    let mut zero = ZeroWatch::new(SessionMode::Meeting);
    let steps = (ZERO_WARN_AFTER.as_millis() / STEP.as_millis()) as u32;
    let mut t = Duration::ZERO;
    let mut seen = Zeroed::default();
    for _ in 0..steps - 1 {
        t += STEP;
        seen = zero.observe(t, MIC_NOISE, 0.0);
    }
    assert_eq!(seen, Zeroed::default(), "2,75 s: ainda não");
    t += STEP;
    assert_eq!(
        zero.observe(t, MIC_NOISE, 0.0),
        Zeroed {
            mic: false,
            system: true
        }
    );
    // O sinal volta: apaga na hora.
    t += STEP;
    assert_eq!(zero.observe(t, MIC_NOISE, VOICE), Zeroed::default());
}

#[test]
fn ordinary_silence_is_not_digital_zero() {
    // Um mic numa sala quieta fica abaixo do piso de silêncio do MuteWatch, mas não em zero.
    const { assert!(MIC_NOISE < SILENCE_RMS) };
    let mut zero = ZeroWatch::new(SessionMode::Meeting);
    let mut t = Duration::ZERO;
    for _ in 0..400 {
        t += STEP;
        zero.observe(t, MIC_NOISE, VOICE);
    }
    assert!(!zero.zeroed().mic);
    // Um mic mudo pelo SO entrega zeros: acende.
    for _ in 0..12 {
        t += STEP;
        zero.observe(t, 0.0, VOICE);
    }
    assert!(zero.zeroed().mic);
}

#[test]
fn the_mic_only_counts_in_modes_with_a_mic() {
    let mut zero = ZeroWatch::new(SessionMode::SystemOnly);
    let mut t = Duration::ZERO;
    for _ in 0..40 {
        t += STEP;
        zero.observe(t, 0.0, VOICE);
    }
    assert!(!zero.zeroed().mic);
}
