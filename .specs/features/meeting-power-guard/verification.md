# meeting-power-guard — verification

Windows 11 (Alienware 16, Modern Standby only), `CARGO_TARGET_DIR=C:\f\meeting-power-guard`,
2026-10-12. Verificado pelo próprio executor na rodada noturna (sem Verifier separado); o veredito
vale para a tabela, não para o roteiro manual. PRs #146 (fatia 1) e #147 (fatia 2).

| Check | Result | Evidence |
| --- | --- | --- |
| C1 | PASS | `meeting::recorder::tests::{the_pc_stays_awake_exactly_while_the_capture_writes, waking_keeps_the_pause_of_the_person, with_the_setting_off_nothing_is_asked}` ok |
| C2 | PASS | `power::tests::{the_guard_calls_the_os_only_on_change_and_lets_go_on_drop, a_guard_never_set_asks_nothing, a_panic_lets_go_too, power_broadcasts}` ok |
| C3 | PASS | `cargo test -p fala-audio --lib meeting::recorder`: 9 passed |
| C4 | PASS | `cargo test -p fala-meeting`: 72 passed; os seis de `suspend::` e `resume::only_resume_recording_resumes` ok |
| C5 | PASS | `meeting::sleeps::tests` (2) ok; `bun src/components/meeting/meeting.test.tsx`: `SP-sleep-mark ok` |
| C6 | PASS | revisão: `interrupted` só com `waking` e sem `Stopping`, `finish` chama `recover_meeting`; `call_prompt::tests::answer_maps_each_meeting_card` ok |
| C7 | PASS | `settings::tests::meeting_audio_is_kept_and_capped_at_three_hours_by_default` ok; `check:translations`, `lint`, `format:check`, `tsc --noEmit` 0 |
| C8 | PASS | `cfg(windows)` novo só em `apps/desktop/src/power.rs`; nada em `crates/` |
| suite | PASS | `cargo test -p fala --lib`: 531 passed, 0 failed, 1 ignored; pre-push (`clippy --workspace -D warnings` + `test --workspace`): 1081 passed, 0 failed |

Revisão `code-review` (high) no diff inteiro: 10 achados; 6 corrigidos no commit `fix(desktop)` da
fatia 2 (aviso de silêncio através do sono, aviso de volta perdido, saída padrão após o sono, parar
durante a volta, chaves React duplicadas, nome `cardBody`) e o comentário de `flush` ajustado. Não
mudados: Modern Standby (roteiro manual), marcador ao lado do áudio (decisão do plano), estado
`suspend_ack` (simplificação adiada).

Unproven: o pedido real ao Windows e uma suspensão real. Nada foi gravado nem posto para dormir nesta
máquina (regra da noite); o roteiro de `checks.md` fica para o Augusto. Esta máquina só tem Modern
Standby, e é o passo 2 do roteiro que mostra se `ES_SYSTEM_REQUIRED` segura o sono depois que a tela
apaga. Linux não foi compilado aqui; o job `rust` do CI cobre os no-ops.
