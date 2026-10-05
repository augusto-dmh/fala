//! As formas literais das doors do plano `meeting-recorder` que um teste de unidade não pega.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use fala_audio::{AudioError, MeetingRecorder, MeetingWav, SystemAudio};

#[test]
fn system_audio_has_door_1_shape() {
    let _open: fn(&str) -> Result<SystemAudio, AudioError> = SystemAudio::open;
    let _check: fn(&str) -> Result<(), AudioError> = SystemAudio::check;
    let _rate: fn(&SystemAudio) -> u32 = SystemAudio::sample_rate;
    let _drain: fn(&mut SystemAudio, &mut Vec<f32>) = SystemAudio::drain_into;
    // Door 2: o gravador recebe frames empurrados e nunca abre dispositivo.
    let _new: fn(u32, u32, MeetingWav) -> Result<MeetingRecorder, AudioError> =
        MeetingRecorder::new;
    let _push: fn(&mut MeetingRecorder, &[f32]) -> Result<(), AudioError> =
        MeetingRecorder::push_mic;
}
