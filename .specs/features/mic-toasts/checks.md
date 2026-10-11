# mic-toasts checks

Profile: light
Plan: `.specs/features/mic-toasts/plan.md`

17 checks in 3 slices · 0 one-way doors · 0 open, of which 0 block

Cargo com `CARGO_TARGET_DIR=C:\f\mic-toasts`, na raiz do worktree, um comando por vez. As provas
estruturais por `grep`/`awk` leem só código de produção (antes de `mod tests`) e falham quando o
alvo some (L-002).

## Checks

### S1 - o aviso de erro no gesto · 5 files

**C1** - Sem modelo, `TranscribeAction::start` chama `show_notice(.., NoticeKind::ModelMissing, None)` e `play_error_chime` no ramo do `warn!("Not starting recording ...")`, antes do `return` e antes de `try_start_recording` (AC 1)
Proof: `awk '/Not starting recording: no model/,/return;/' apps/desktop/src/actions.rs | grep -c 'NoticeKind::ModelMissing\|play_error_chime'` = 2

**C2** - A falha de `try_start_recording` é classificada por `start_error_kind`: acesso negado -> (`microphone_permission_denied`, `MicDenied`), sem dispositivo -> (`no_input_device`, `NoMic`), outro -> (`unknown`, `MicFailed`); o ramo de erro mostra o aviso, toca o som de erro, emite `recording-error` e não chama mais `hide_recording_overlay` (AC 2)
Proof: `cargo test -p fala --lib actions::tests::start_errors_map_to_event_and_notice`
Proof: `awk '/if recording_error.is_some\(\)/,/^        }$/' apps/desktop/src/actions.rs > $T; grep -q 'show_notice' $T && grep -q 'play_error_chime' $T && grep -q '"recording-error"' $T && ! grep -q 'hide_recording_overlay' $T`

**C3** - `play_error_chime` toca `play_test_sound(&app, SoundType::Stop)` duas vezes com 150 ms entre elas e não chama `play_feedback_sound` (AC 3)
Proof: `awk '/^pub fn play_error_chime/,/^}/' apps/desktop/src/audio_feedback.rs` contém 2× `play_test_sound(&app, SoundType::Stop)`, `from_millis(150)` e nenhum `play_feedback_sound`

**C4** - `show_notice` com `alone = true` não olha `overlay_style` (mostra a janela pelo caminho de `show_window_on_main`, sem o retorno antecipado de `show_overlay_state`), e agenda um `hide_recording_overlay` depois de `notice_duration(kind)` só se `OVERLAY_SHOW_GENERATION` não mudou (AC 4)
Proof: `awk '/^pub fn show_notice/,/^}/' apps/desktop/src/overlay.rs > $T; ! grep -q 'overlay_style' $T && grep -q 'OVERLAY_SHOW_GENERATION' $T && grep -q 'hide_recording_overlay' $T && grep -q 'notice_duration' $T`

**C5** - `notice_duration` dá 1500 ms para `MicInUse`, 6000 ms para `MicMuted`, `MicDenied`, `NoMic`, `MicFailed` e 5000 ms para `ModelMissing`; `NoticeKind` serializa como `mic_in_use`, `mic_muted`, `mic_denied`, `no_mic`, `mic_failed`, `model_missing`; a janela do aviso mede 340×110 (AC 5, 6 tipos)
Proof: `cargo test -p fala --lib overlay::tests::notice_durations_and_codes`

**C6** - O ramo `!model_info.is_downloaded` emite `loading_failed` com `error_code` `model_not_downloaded`, e o ramo `None` de `get_model_info` (modelo desconhecido ou seleção vazia) com `model_not_found`; `App.tsx` mostra `t("errors.modelNotDownloaded")` e `t("errors.modelNotFound")` para cada código, e as duas chaves existem em pt e en (AC 6)
Proof: `awk '/if !model_info.is_downloaded/,/return Err/' apps/desktop/src/managers/transcription.rs | grep -q 'model_not_downloaded'`
Proof: `awk '/let model_info = match self.model_manager.get_model_info/,/return Err/' apps/desktop/src/managers/transcription.rs | grep -q 'model_not_found'`
Proof: `bun src/overlay/notice.test.tsx` (linha `C6 ok`)

### S2 - microfone mudo e ditado vazio · 3 files

**C7** - `SignalMeter`: 0,5 s de zeros dá `Silent`; um bloco com RMS 0,001 dá `Heard` e um com RMS 0,000999 dá `Silent`; 0,5 s menos uma amostra dá `Unknown`; nada alimentado dá `Unknown` (AC 7, bordas do limiar e da duração)
Proof: `cargo test -p fala --lib dictation_capture::tests::signal_meter_verdicts`

**C8** - `Processor`: áudio alto antes do `start` (pré-buffer) e zeros depois dão `Silent` no `stop`; fala depois do `start` dá `Heard`; um segundo `start` zera o medidor (AC 7)
Proof: `cargo test -p fala --lib dictation_capture::tests::processor_reports_the_signal_of_the_recording_only`

**C9** - `stop_outcome`: (`Silent`, qualquer amostra) -> `Muted`; (`Heard`|`Unknown`, vazia) -> `Discard`; (`Heard`|`Unknown`, com amostras) -> `Transcribe`; `is_blank_transcription` (já existia, reusada) é verdadeiro para `""` e `" \n"` e falso para `"a"` (AC 8, 9, 10, tabela 3×2)
Proof: `cargo test -p fala --lib actions::tests::stop_outcome_table`

**C10** - No `stop`, `stop_outcome(..)` é decidido depois de `was_cancelled_since`; o ramo `outcome == StopOutcome::Muted` chama `show_notice` com `MicMuted` e não contém `save_wav_file`, `transcribe` nem `save_entry` (AC 8)
Proof: `awk '/if outcome == StopOutcome::Muted/,/outcome == StopOutcome::Discard/' apps/desktop/src/actions.rs > $T; grep -q 'NoticeKind::MicMuted' $T && ! grep -Eq 'save_wav_file|transcribe\(|save_entry' $T`

**C11** - Transcrição vazia: o ramo `is_blank_transcription(&transcription)` vem antes do pós-processamento, apaga o WAV (`remove_file`) e retorna sem `deliver_unless_cancelled` nem `save_entry` (AC 9)
Proof: `awk '/if is_blank_transcription\(&transcription\)/,/return;/' apps/desktop/src/actions.rs > $T; grep -q 'remove_file' $T && ! grep -Eq 'deliver_unless_cancelled|save_entry' $T`

**C12** - Os testes herdados de `dictation_capture` e de `actions` continuam verdes, sem asserção editada além do tipo de retorno de `stop` (regressão)
Proof: `cargo test -p fala --lib dictation_capture actions::tests`

### S3 - o aviso na pill · 6 files

**C13** - `announce_mic`: primeiro nome -> verdadeiro; mesmo nome -> falso; outro nome -> verdadeiro; volta ao primeiro -> verdadeiro. No `start`, o anúncio só roda com `overlay_style != None` e sem o painel de streaming (AC 11)
Proof: `cargo test -p fala --lib actions::tests::announce_mic_only_on_change`
Proof: `grep -B6 'NoticeKind::MicInUse' apps/desktop/src/actions.rs | grep -q 'compact_pill'`

**C14** - `troubleshooting_page(true)` = `ms-settings:privacy-microphone`, `troubleshooting_page(false)` = `ms-settings:sound`; `open_microphone_settings` e `open_microphone_troubleshooting` escondem o overlay; `open_microphone_settings` mostra a janela principal e emite `open-microphone-settings`, que `App.tsx` traduz em `settings` + `general` (AC 12)
Proof: `cargo test -p fala --lib commands::audio::tests::troubleshooting_page_follows_permission`
Proof: `bun src/overlay/notice.test.tsx` (linha `C14 ok`)

**C15** - `Notice`: para os 6 tipos, o texto vem de `t("overlay.notice.<chave>")` (com `device` em `mic_in_use`); `mic_muted` tem o ponto `warn`, os 4 erros o ponto `error`, `mic_in_use` nenhum; só `mic_muted`, `mic_denied`, `no_mic` e `mic_failed` desenham os dois botões; toda chave `overlay.notice.*` existe em pt e en; o CSS usa cores literais (AC 13, 6 tipos)
Proof: `bun src/overlay/notice.test.tsx` (linha `C15 ok`)

**C16** - `RecordingOverlay` escuta `overlay-notice`; `hide-overlay` e o `show-overlay` de `recording`/`streaming` limpam o aviso; um aviso `alone` desenha só o `Notice`, e um empilhado desenha o `Notice` e a `Pill` (AC 14, AC 11)
Proof: `bun src/overlay/notice.test.tsx` (linha `C16 ok`)

**C17** - O front e o Rust passam nos portões: `bun run lint`, `bunx tsc --noEmit`, `bun run check:translations`, `bunx prettier --check .`, `cargo fmt --all -- --check`, `cargo clippy -p fala --all-targets -- -D warnings`, `scripts/check-brand.sh`; `bun src/overlay/pill.test.tsx` segue verde
Proof: os comandos acima, exit 0

## Manual on Windows (fora da tabela, L-004)

Registrados no `verification.md` como evidência ou como "Not tested"; não mudam o veredito dos
checks acima.

- M1 (AC 1, 3, 6): build de debug portátil com a pasta de modelos vazia; apertar o atalho toca dois toques e mostra "Nenhum modelo de voz baixado" por 5 s, com a pill ligada e com a pill desligada
- M2 (AC 8, 12): microfone mudo nas configurações de som do Windows; ditar 2 s: "O microfone está mudo?" aparece, nenhuma entrada nova no histórico; [Resolver] abre as configurações de som; [Escolher microfone] abre Configurações > Geral
- M3 (AC 11): primeiro ditado depois de abrir o app mostra "Usando <mic>" sobre a pill; o segundo não
- M4 (AC 2): mic ausente ou ocupado — Not tested se não houver jeito limpo de reproduzir

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| tipos de aviso (6) | `mic_in_use` C5, C13, C15 · `mic_muted` C5, C10, C15 · `mic_denied` C2, C5, C15 · `no_mic` C2, C5, C15 · `mic_failed` C2, C5, C15 · `model_missing` C1, C5, C15 | - |
| classes de erro ao abrir o mic (3) | negado C2 · sem dispositivo C2 · outro C2 | - |
| vereditos do sinal (3) | `Silent` C7, C8 · `Heard` C7, C8 · `Unknown` C7 | - |
| bordas do sinal (2) | RMS 0,001 / 0,000999 C7 · 0,5 s / 0,5 s − 1 amostra C7 | - |
| desfechos do stop (3) | `Muted` C9, C10 · `Discard` C9 · `Transcribe` C9 | - |
| caminhos que gravam histórico (3) | `save_entry` no `Ok` (pula transcrição vazia) C11 · `save_entry` no `Err` (falha de transcrição, mantém a entrada vazia para re-transcrever, existing) · re-transcrever (atualiza entrada existente, não cria; fora do gesto) | - |
| destinos de [Resolver] (3) | Windows negado C14 · Windows liberado C14 · fora do Windows (abre o seletor) C14 | - |
| onde o aviso some (3) | `hide-overlay` C16 · `show-overlay` recording C16 · `show-overlay` streaming C16 | - |
| pill para o anúncio do mic (3) | compacta C13 · Live com streaming C13 · desligada C13 | - |

- Claims naming a value at a boundary: C7 asserts both sides of the RMS and of the duration edge
- No check claims more than the cases its proof exercises

## Swept

- validation: n/a - nenhuma entrada nova do usuário
- failure modes: C2 (mic que não abre), C1 (sem modelo), C10 (mic mudo); falha ao tocar o som é logada pelo `play_sound_blocking` herdado
- idempotency: C13 (o anúncio não repete para o mesmo mic)
- authorization: n/a
- concurrency: C4 (o `hide` agendado respeita a geração: uma sessão nova não é escondida pelo aviso anterior)
- data lifecycle: C10, C11 (ditado mudo ou vazio não deixa entrada nem WAV)
- dependency failure: existing - falha ao abrir as configurações do Windows volta como `Err` do comando e é logada no front
- state transitions: C9
- observability: o veredito do sinal e o aviso logam em `debug!` com o tipo, nunca com áudio ou texto; o nome do mic só em `debug!`

## Handoff

- S1 + S2 + S3 passam de 1000 linhas com os specs: dois PRs empilhados. **PR1** (`feat/mic-toasts`): S1 — C1-C6, C17 e C15/C16 com os 4 avisos de erro (sem ações, sem empilhar). **PR2** (`feat/mic-toasts-mute`, sobre o PR1): S2 e S3 — C7-C14, e C15/C16 completos com os 6 avisos, as ações e o aviso empilhado sobre a pill
- **Settled mid-build:** `is_empty_transcription` virou `is_blank_transcription`, que já existia em `actions.rs` (pula o LLM em transcrição vazia); C9 e C11 citam a existente
- **Settled mid-build:** a borda de C7 usa blocos de 2 amostras: com 160 valores iguais, a soma em f32 arredonda o RMS de 0,001 para baixo
- **Settled mid-build (M1):** no Windows, a partida limpa um `selected_model` que não está em disco, então o caso real do S11 é `Model not found: ` com seleção vazia, não `Model not downloaded`. O ramo `None` de `get_model_info` ganhou `error_code: "model_not_found"` e a chave `errors.modelNotFound` (AC 6 e C6 ampliados)
- **Settled mid-build:** o `show-overlay` "notice" lê idioma e posição numa função à parte, sem `await` novo no handler (`upstream-catchup` PR2 C7 conta 2 `await`s nele)
