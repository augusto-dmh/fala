# meeting-mic verification

Máquina: Windows 11 (Alienware), `CARGO_TARGET_DIR=C:\f\meeting-mic`.

| Check | Evidência |
| --- | --- |
| C1 | `grep -n default_input_device apps/desktop/src/meeting/recorder.rs` só acha o fallback de `open_mic` quando a escolha é "Automático" (`None`); `open`, `reopen_mic`, `reopen` e `switch_mic` passam por `MicChoice` → `AudioRecordingManager::meeting_microphone()` |
| C2 | `switch_mic`/`replace_mic` chamam `MeetingRecorder::set_mic_rate` quando a taxa muda (coberto por `switching_the_mic_rate_keeps_the_clock` e `a_dictation_across_a_mic_rate_switch_stays_zeroed` em `fala-audio`); `apply_selected_microphone` e `set_clamshell_microphone` chamam `MeetingManager::microphone_changed` |
| C3 | `cargo test -p fala --lib -- meeting:: call_prompt:: commands::audio`: 73 passed (16 em `meeting::lifecycle` + `call_prompt::tests`), entre eles `a_call_heard_at_a_low_volume_is_not_a_mistake` (51 s de som, sem pergunta), `only_both_channels_silent_is_a_mistake`, `a_short_or_silent_stop_looks_like_a_mistake` (10 s / 3 s) e `answer_maps_each_meeting_card` (expirar = Keep). Recado de 00:10 e 01:05 incorporados |
| C4 | `bun src/components/meeting/meeting.test.tsx`: `mic-input ok` |
| C5 | Não feito: o Fala instalado estava aberto (single-instance encaminha o debug para ele). Fica no roteiro de 1 minuto do PR |

Medição que motivou C3 (áudio retido das reuniões de teste, só lido, nada versionado): RMS em
janelas de 250 ms do `sys.opus` de 100 s: mediana −54 dBFS, máximo −44, primeiro segundo 0,21 (som
de início); `mic.opus` −98 dBFS (zero digital da LifeCam).

Revisão `code-review` (high) contra `origin/main`: 10 achados. Corrigidos: cache de dispositivo velho
(`meeting_microphone` reenumera), sem fallback quando o escolhido não abre (`open_mic` cai no
padrão), troca de canal (`set_selected_channel` avisa; `switch_mic` compara nome e canal),
card substituído ou escondido sem resposta (`settle_unanswered` = Manter), retranscrição na
expiração (`keep_after_mistake` pula reunião já transcrita), erro do gravador na troca (encerra
a captura), lock no main thread (aviso em thread), texto "quiet" ("Quase nenhum som foi
gravado"). Mantidos por decisão: piso de −60 dBFS no mic (recado 00:10), limiar de 10 s
espelhado na UI (já era assim com 30 s). `cargo clippy --workspace --all-targets -D warnings`
limpo depois das correções.
