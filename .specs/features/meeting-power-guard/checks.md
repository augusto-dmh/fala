# meeting-power-guard — checks

| # | Slice | Criterion | Proof | Kind |
| --- | --- | --- | --- | --- |
| C1 | 1 | 1 | `cargo test -p fala --lib meeting::recorder`: `the_pc_stays_awake_exactly_while_the_capture_writes`, `waking_keeps_the_pause_of_the_person`, `with_the_setting_off_nothing_is_asked` (`FakePower` registra as chamadas) | test |
| C2 | 1+2 | 1 | `cargo test -p fala --lib power`: a guarda só chama o SO na mudança, solta no `Drop` e num pânico da thread; os `PBT_*` viram `Suspending`/`Resumed` | test |
| C3 | 2 | 2 | `cargo test -p fala-audio --lib meeting::recorder`: `tick_rewrites_the_header_every_second` (flush antes de suspender), `switching_the_mic_rate_keeps_the_clock` | test |
| C4 | 2 | 3 | `cargo test -p fala-meeting`: `suspend::os_resume_continues_the_recording_and_marks_the_gap`, `os_resume_keeps_a_pause`, `a_click_cannot_resume_while_the_pc_sleeps`, `resume::only_resume_recording_resumes` (pausa + sono nunca vira gravação) | test |
| C5 | 2 | 4 | `cargo test -p fala --lib meeting::sleeps`; `bun src/components/meeting/meeting.test.tsx` (`SP-sleep-mark`) | test |
| C6 | 2 | 5 | revisão do código: `Report::Finalized` com erro enquanto `waking` → `interrupted` → `recover_meeting` (`stop_reason` nulo, testado em #128) e sem "gravou por engano?"; `call_prompt` `answer_action` do card `woke` | review + test |
| C7 | 1+2 | 6 | `settings::tests::meeting_audio_is_kept_and_capped_at_three_hours_by_default` (`meeting_keep_awake` true); `bun run check:translations`, `bun run lint`, `bunx tsc --noEmit` | test + lint |
| C8 | 1+2 | ADR-0007 | `cfg(windows)` novo só em `apps/desktop/src/power.rs`; `scripts/check-no-tauri-in-crates.sh` | grep |

## Manual (Windows, Augusto; não bloqueia o veredito)

1. `powercfg /query SCHEME_CURRENT SUB_SLEEP STANDBYIDLE` para anotar o valor atual; depois
   `powercfg /change standby-timeout-ac 1`.
2. Gravar uma reunião "só o sistema" e não tocar no PC por 4 min. O PC não dorme; num terminal
   admin, `powercfg /requests` lista `fala.exe` em SYSTEM. Pausar: a linha some. Retomar: volta.
3. Gravando, `rundll32 powrprof.dll,SetSuspendState 0,1,0` (ou fechar a tampa). Acordar: o card
   "A gravação continuou depois da suspensão" aparece por 10 s; a pílula volta a gravar; ao parar e
   transcrever, o marcador "O computador dormiu por N min" aparece no ponto; o áudio depois da volta
   está na mesma reunião.
4. Desligar "Impedir a suspensão enquanto grava": numa nova gravação `powercfg /requests` não lista
   o Fala.
5. Restaurar `standby-timeout-ac` ao valor do passo 1.
