# media-pause-and-sounds — checks

| # | Criterion | Proof | Kind |
| --- | --- | --- | --- |
| C1 | 1 | `cargo test -p fala --lib media_pause::tests::resumes_only_what_it_paused` | test |
| C2 | 1 | `media_pause::tests::a_refused_pause_is_not_resumed`, `a_player_closed_or_stopped_meanwhile_is_left_alone`, `a_second_pause_does_not_record_a_session_twice` | test |
| C3 | 2 | `media_pause::tests::a_paused_player_never_starts_playing` e o terceiro player de C1 (pausado antes, nunca recebe `play`) | test |
| C4 | 3 | `media_pause::tests::dictation_and_meeting_resume_their_own_pauses` | test |
| C5 | 4 | `media_pause::tests::without_media_control_nothing_happens_and_open_is_retried`; `cargo check --workspace` no Linux (CI `rust`) compila o `platform` sem SMTC | test + CI |
| C6 | 1 (ditado) | `media_pause::tests::a_stale_pause_never_follows_its_resume` (gate de geração) | test |
| C7 | 5 | `meeting::tests::each_session_step_plays_its_sound_and_moves_the_media` | test |
| C8 | 6 | `audio_feedback::tests::every_theme_has_its_meeting_sounds` (lê os `.wav` de `resources/`) | test |
| C9 | 7 | `settings::tests::media_pause_is_off_for_dictation_and_on_for_meetings` | test |
| C10 | 8 | `bun run check:translations`, `bun run lint`, `bunx tsc --noEmit` (bindings regenerados) | lint |
| C11 | ADR-0007 | `git diff origin/main...HEAD` não tem `cfg(windows)` fora de `apps/desktop` e crates de plataforma; `scripts/check-no-tauri-in-crates.sh` | grep |
| C12 | definição | nenhuma tecla de mídia enviada: `git diff origin/main...HEAD -- apps/desktop/src \| grep '^+' \| grep VK_MEDIA \| grep -vc '//'` = 0 (a única linha é o comentário que proíbe o fallback); `grep -c VK_MEDIA apps/desktop/src/media_pause.rs` = 1 prova que o alvo existe | grep |

## Manual (Windows, fora da tabela: o Augusto roda, não bloqueia o veredito)

Roteiro no corpo do PR. Esta noite não foi rodado: nenhuma mídia real foi tocada nem pausada.

1. Spotify tocando, YouTube no Edge tocando, um terceiro player pausado. Ligar "Pausar a mídia ao
   ditar". Ditar: os dois pausam depois do som de início; soltar: os dois voltam; o terceiro
   continua pausado.
2. Desligar a opção: ditar não pausa nada.
3. Gravar reunião (opção da reunião ligada, padrão): a música pausa e o som de início toca. Pausar a
   reunião: som de pausa, música volta. Retomar: som de retomada, música pausa. Parar: música volta.
4. Com "Feedback de Áudio" desligado, a reunião não toca nenhum dos três sons.
