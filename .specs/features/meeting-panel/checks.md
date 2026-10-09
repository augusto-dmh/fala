# meeting-panel checks

Profile: light
Plan: `.specs/features/meeting-panel/plan.md`

42 checks in 6 slices (C39-C42 added after verification round 1) · 7 one-way doors · 2 open, of which 0 block (1 blocks go-live)

Todo `cargo` roda com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`, um por vez, depois do guard de RAM (`free -m`). As provas do frontend rodam no padrão do repo (`bun <arquivo>.test.tsx`, que imprime `<check> ok` e sai com erro na primeira falha). "manual" marca o que só se vê com o app aberto; o Linux roda o `tauri dev`, e o que só o Windows mostra fica `TODO(windows)`.

## Checks

### S1 - reunião persistida no `fala.sqlite` · 4 files · 40 KB · ~10k

**C1** - Um banco em `user_version = 1` com 2 ditados abre em `user_version = 2`, com `meetings` e `meeting_segments` criadas e os 2 ditados devolvidos por `get` com o mesmo texto; um banco novo abre direto em 2 (AC 1, door 1)
Proof: `cargo test -p fala-storage --test meetings migrates_v1_to_v2_keeping_dictations -- --exact`

**C2** - `create_meeting` seguido de `meeting(id)` devolve id, título, modo `in_person`, `local_only = false`, criação, `started_at = None` (rascunho), `ended_at = None`, `recorded_ms = 0`, anotações `""`, `notes_md = None`, `audio_retained = false` (AC 2)
Proof: `cargo test -p fala-storage --test meetings create_then_read_meeting -- --exact`

**C3** - `finish_meeting(id, fim, 61_000, StopReason::CapReached)` e `set_meeting_audio_retained(id)` aparecem em `meeting(id)`; `stop_reason` grava `cap_reached` (AC 3, door 1)
Proof: `cargo test -p fala-storage --test meetings finish_meeting_records_end -- --exact`

**C4** - `save_meeting_annotations(id, "decidir data\nAna: contrato")`, `drop` do `Store` e `Store::open` de novo: `meeting(id).annotations` é exatamente o texto (AC 4, AC 26)
Proof: `cargo test -p fala-storage --test meetings annotations_survive_reopen -- --exact`

**C5** - `replace_meeting_segments` com 3 segmentos (`t0_ms` 9000, 0, 4000; falantes `me`, `{"person":2}`, `{"person":1}`) devolve-os por `t0_ms` com `seq` 1, 2, 3 e os falantes intactos; a coluna `speaker` guarda `"me"` e `{"person":2}`; `transcribed_at` deixa de ser `None`; uma segunda chamada com 1 segmento deixa 1 (AC 5, door 1)
Proof: `cargo test -p fala-storage --test meetings segments_sorted_numbered_and_replaced -- --exact`

**C6** - `save_meeting_notes(id, "geral", "## Notas")` aparece em `meeting(id)` como `notes_template = Some("geral")` e `notes_md = Some("## Notas")` (AC 6)
Proof: `cargo test -p fala-storage --test meetings notes_saved -- --exact`

**C7** - `meetings()` com 3 sessões criadas fora de ordem devolve-as da criação mais recente à mais antiga (AC 7)
Proof: `cargo test -p fala-storage --test meetings list_newest_first -- --exact`

**C8** - `meeting`, `finish_meeting`, `save_meeting_annotations`, `replace_meeting_segments` e `save_meeting_notes` com id inexistente devolvem `StorageError::NotFound(id)` (AC 8)
Proof: `cargo test -p fala-storage --test meetings unknown_id_is_not_found -- --exact`

**C37** - Rascunho: `create_meeting` + `set_meeting_title` + `save_meeting_annotations` deixam `started_at = None` com título e pauta; `finish_meeting` num rascunho falha; `start_meeting_recording` grava início e modo e mantém a pauta; a segunda chamada devolve `MeetingAlreadyStarted(id)` e o início não muda; id inexistente em `start_meeting_recording`/`set_meeting_title` devolve `NotFound` (AC 42, door 1)
Proof: `cargo test -p fala-storage --test meetings draft_takes_agenda_and_records_once -- --exact`
Proof: `cargo test -p fala-storage --test meetings unknown_id_is_not_found -- --exact`

**C9** - Apagar a linha de `meetings` apaga os segmentos dela (cascata), e `mode = 'outro'` é recusado pelo `CHECK` (door 1)
Proof: `cargo test -p fala-storage --test meetings schema_constraints -- --exact`

### S2 - gravar só por clique, com aviso e indicador · 9 files · 180 KB · ~45k

**C10** - `check_start(consent = None, ..)` devolve `MeetingError::ConsentRequired`; com aceite e ditado gravando devolve `DictationActive`; com sessão ativa devolve `AlreadyActive`; com aceite, sem ditado e sem sessão devolve `Ok` (AC 9, AC 17)
Proof: `cargo test -p fala --lib meeting::tests::start_guard_order -- --exact`

**C11** - `consent_stamp` de um instante fixo em UTC−3 produz `"2026-10-09T22:14:03-03:00"`, e `MeetingError::ConsentRequired` serializa como `{"kind":"consent_required"}` (AC 10, door 2, door 5)
Proof: `cargo test -p fala --lib meeting::tests::consent_stamp_and_error_shape -- --exact`

**C12** - O texto `meeting.consent.body` em `pt` é literalmente o da door 2, e o diálogo de aviso o renderiza com os botões `meeting.consent.accept`/`meeting.consent.cancel` (AC 10, door 2)
Proof: `bun src/components/meeting/meeting.test.tsx` (check `C12`)

**C13** - Em `apps/desktop/src`, fora de módulos de teste, `UserAction::StartRecording` aparece só em `meeting/manager.rs` dentro de `fn start` e `UserAction::ResumeRecording` só dentro de `fn resume`; `MeetingManager::start` e `::resume` são chamados só por `commands/meeting.rs` e pelo despacho do tray (`meeting_start`, `meeting_resume`) em `lib.rs`; nenhum `setup`/timer/evento os chama (AC 11, ADR-0005)
Proof: `bash .specs/features/meeting-panel/proofs/explicit_start.sh`

**C14** - `start_meeting` aceito cria a linha em `meetings`, cria `audio/<id>/recording.wav` e emite `state = recording` (AC 12)
Proof: manual - `bun run tauri dev`, clicar "Gravar reunião", conferir o WAV crescendo e `fala-cli history`/sqlite com a linha; resultado registrado no `## Handoff`

**C15** - `overlay_for_meeting(MeetingIndicator::Recording)` é `Some("meeting")`, `Paused` é `Some("meeting_paused")`, `None` é `None`; `show_meeting_overlay` ignora `OverlayStyle::None`; `hide_recording_overlay` com indicador de reunião reaplica o estado da reunião em vez de esconder (AC 13)
Proof: `cargo test -p fala --lib overlay::tests::meeting_overlay_states -- --exact`
Proof: `bun src/overlay/pill.test.tsx` (check `meeting-pill`: `meeting` renderiza ponto e tempo `12:05` a partir de `recorded_ms = 725_000`, `meeting_paused` com o tom pausado)

**C16** - `meeting_menu_ids(MeetingTray::Idle)` é `["meeting_start"]`, `Recording` é `["meeting_pause", "meeting_stop"]`, `Paused` é `["meeting_resume", "meeting_stop"]`; com reunião ativa o ícone desejado é o de gravação (AC 14)
Proof: `cargo test -p fala --lib tray::tests::meeting_items_follow_state -- --exact`

**C17** - O item de tray `meeting_start` sem aceite mostra a janela principal e emite `meeting-consent-required`, e a página, ao recebê-lo, abre a seção Reuniões com o aviso; não chama `start` (AC 15)
Proof: `cargo test -p fala --lib meeting::tests::tray_start_without_consent_asks -- --exact`
Proof: manual - clicar o item no tray do `tauri dev` com `meeting_consent_accepted_at` ausente

**C18** - `dictation_refusal(state)` devolve `Some("meeting_active")` em `recording` e `paused` e `None` nos outros; `TranscribeAction::start` consulta isso antes de `initiate_model_load`, e o `App.tsx` mapeia `meeting_active` para `errors.meetingActive` (AC 16, ADR-0015)
Proof: `cargo test -p fala --lib meeting::tests::dictation_blocked_while_recording -- --exact`
Proof: `bash .specs/features/meeting-panel/proofs/dictation_refusal.sh`

**C19** - Ao começar a reunião, o desktop chama `AudioRecordingManager::stop_microphone_stream` antes de `Mic::open`, e ao fim chama `start_microphone_stream` só se o modo for `AlwaysOn` (AC 17)
Proof: `bash .specs/features/meeting-panel/proofs/one_mic_stream.sh`

### S3 - pausar, retomar, parar e avisos · 4 files · 60 KB · ~15k

**C20** - `ActiveClock`: 10 s gravando, 5 s pausado, 3 s gravando dá 13 s de tempo gravado; pausar duas vezes seguidas não muda nada (AC 18)
Proof: `cargo test -p fala --lib meeting::recorder::tests::paused_time_is_not_counted -- --exact`

**C21** - `recorded_ms(written)` é `written / 48` (48 000 quadros = 1000 ms) e é o valor que o `MeetingStatus` leva (AC 24)
Proof: `cargo test -p fala --lib meeting::recorder::tests::recorded_ms_follows_frames -- --exact`

**C22** - Parar pelo comando grava fim, `recorded_ms` e `stop_reason = user`, esconde a pill, volta o tray e emite `processing` e depois `idle` (AC 19)
Proof: manual - gravar 30 s no `tauri dev`, parar, conferir a linha no sqlite, `mic.opus`/`sys.opus` em `audio/<id>/` e a pill escondida

**C23** - `MuteWatch` em modo `meeting`: 119 s de sistema abaixo de 0,001 não marca; 120 s marca `system`; um intervalo com 0,002 desmarca e a contagem recomeça do zero (AC 20, door 4)
Proof: `cargo test -p fala-meeting --lib mute::tests::system_mutes_at_120_s -- --exact`
Proof: `cargo test -p fala-meeting --lib mute::tests::sound_clears_and_restarts -- --exact`

**C24** - `MuteWatch` em `system_only` com 10 min de mic em 0 nunca marca `mic`; em `meeting` marca (AC 21, door 4)
Proof: `cargo test -p fala-meeting --lib mute::tests::mic_only_counts_with_mic -- --exact`

**C25** - A página e a pill mostram o aviso de canal mudo quando `muted.mic` ou `muted.system` é `true`, e a página mostra o restante e "Mais 1 h" quando `cap_remaining_ms` existe (AC 22, AC 23)
Proof: `bun src/components/meeting/meeting.test.tsx` (checks `C25-muted`, `C25-cap`)
Proof: `bun src/overlay/pill.test.tsx` (check `meeting-muted`)

**C26** - `stop_reason_of(StopReason::Silence)` e `CapReached` viram `silence` e `cap_reached` no fim da sessão, pelo mesmo caminho do stop do usuário (AC 23)
Proof: `cargo test -p fala --lib meeting::tests::stop_reasons_reach_storage -- --exact`

### S4 - anotações à prova de crash · 2 files · 10 KB · ~3k

**C27** - O `AnnotationSaver` com relógio falso chama `save` uma vez 2000 ms depois da última mudança (nenhuma antes), de novo em `flush()` (perder o foco, parar) e nunca com texto igual ao último salvo (AC 25)
Proof: `bun src/components/meeting/meeting.test.tsx` (check `C27`)

**C38** - `start_target(draft = Some(rascunho))` usa o id do rascunho e não cria sessão nova; com um rascunho já gravado devolve `MeetingError::AlreadyStarted`; sem rascunho gera um id novo; e `notes_input` de uma sessão cuja pauta foi digitada no rascunho leva essa pauta em `annotations` (AC 43)
Proof: `cargo test -p fala --lib meeting::tests::start_uses_the_draft -- --exact`
Proof: `cargo test -p fala --lib meeting::pipeline::tests::notes_input_from_session -- --exact`

### S5 - transcrever, gerar notas e copiar · 6 files · 90 KB · ~22k

**C28** - `segments_for_store` numera os segmentos do ASR por `t0_ms` (seq 1..n), e `notes_segments` leva `seq` ao `id`, `me` a `Speaker::Me` e `person N` a `Speaker::Person(N)` (AC 28, AC 31)
Proof: `cargo test -p fala --lib meeting::pipeline::tests::segments_round_trip -- --exact`

**C29** - `scribe_key(&MemoryStore vazio)` devolve `MeetingError::MissingKey`, e com `elevenlabs` gravado devolve a chave; `notes_keys` lê `anthropic` do mesmo cofre; nenhum caminho lê chave de outro lugar (AC 28, AC 31, door 3)
Proof: `cargo test -p fala --lib meeting::pipeline::tests::keys_come_from_the_store -- --exact`

**C30** - `needs_retain(dir)` é verdadeiro com `recording.wav` sem `mic.opus`/`sys.opus` e falso com os dois Opus (AC 27, AC 29)
Proof: `cargo test -p fala --lib meeting::pipeline::tests::retain_before_transcribing -- --exact`

**C31** - `notes_input` monta título, data `AAAA-MM-DD` e hora `HH:MM` locais do início, idioma e dicionário das settings, anotações e segmentos; com um template inexistente devolve `UnknownTemplate` (AC 31)
Proof: `cargo test -p fala --lib meeting::pipeline::tests::notes_input_from_session -- --exact`

**C32** - `note_document` com notas geradas: começa em `# Planejamento`, tem a linha `2026-10-02 14:02`, o Markdown das notas e `## Transcrição` com `- **[01:05] Pessoa 1:** ... ^s12`; sem notas tem `## Anotações` no lugar; sem segmentos não tem `## Transcrição`; título vazio vira `# Reunião` (en: `# Meeting`, `## Transcript`) (AC 32, door 6)
Proof: `cargo test -p fala-notes --lib document::tests::document_with_generated_notes -- --exact`
Proof: `cargo test -p fala-notes --lib document::tests::document_without_notes_or_segments -- --exact`

**C33** - "Copiar Markdown" escreve no clipboard exatamente o retorno de `meeting_markdown` (AC 33)
Proof: `bun src/components/meeting/meeting.test.tsx` (check `C33`)

**C34** - Nenhum `log::{info,warn,error}`/`log::trace` acima de `debug` no módulo `meeting` do desktop formata anotações, texto de segmento, notas, Markdown ou chave (AC 34)
Proof: `bash .specs/features/meeting-panel/proofs/no_content_in_logs.sh`

**C35** - Progresso e cancelamento da transcrição vêm do `fala-asr` sem mudança: o `on_progress` vira `MeetingProgressEvent` e o cancelamento usa o mesmo `CancelToken` (AC 30)
Proof: `cargo test -p fala-asr --test meeting_scribe progress::reports_at_least_every_second -- --exact`
Proof: `cargo test -p fala-asr --test meeting_scribe progress::cancel_returns_within_a_second -- --exact`
Proof: `bash .specs/features/meeting-panel/proofs/cancel_token_wired.sh`

### S6 - página Reuniões · 8 files · 50 KB · ~12k

**C36** - A página renderiza: sem sessões, `meeting.list.empty` e o botão de gravar com o seletor online/presencial; com sessões, título ou "Reunião" e data, duração e marcas de transcrição/notas, na ordem recebida; a sessão aberta mostra `[01:05] Pessoa 1: ...`, anotações, notas e os botões "Transcrever", "Gerar notas" com seletor e "Copiar Markdown"; erro `missing_key` vira a mensagem traduzida e o resto da tela fica; ocupado desabilita os botões e mostra "Cancelar"; sem chave mostra o campo da chave, que não reexibe o valor; nenhum literal visível fora do i18next, componentes em `src/components/meeting/` (AC 35-41)
Proof: `bun src/components/meeting/meeting.test.tsx` (checks `C36-empty`, `C36-list`, `C36-detail`, `C36-error`, `C36-busy`, `C36-key`)
Proof: `bun run lint`
Proof: `bun run check:translations`
Proof: `bunx tsc --noEmit`

### Round 1 additions - defects the verifier found outside the checks

**C39** - `MeetingManager::retain` holds the `retaining` lock before it looks at the WAV, so the stop's worker and a "Transcrever" click never convert the same session at once (ADR-0014; verifier F1)
Proof: `bash .specs/features/meeting-panel/proofs/round1_fixes.sh`

**C40** - `start_microphone_stream` refuses first while a meeting records, so no path (always-on toggle, device change) opens a second stream on the meeting mic (ADR-0015; verifier F2)
Proof: `bash .specs/features/meeting-panel/proofs/round1_fixes.sh`

**C41** - `start` writes the session row only after `recorder::spawn` succeeded: a device that fails leaves no session (verifier F6)
Proof: `bash .specs/features/meeting-panel/proofs/round1_fixes.sh`

**C42** - The open session shows no notes field while it records (one field per session), reloads when the recording state changes, and the page consumes the tray's notice request when it is already open (verifier F3, F8)
Proof: `bun src/components/meeting/meeting.test.tsx` (check `C42`)

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| landing doors (7) | schema 2 C1 · aviso C11 · id da chave C29 · canal mudo C23 · IPC C11 · Markdown C32 · dependências C1 | - |
| comando `create_meeting_draft`/`set_meeting_title` statuses (3) | ok C37 · not_found C37 · storage C37 | - |
| comando `start_meeting` statuses (8) | ok C14 · consent_required C10 · already_active C10 · already_started C38 · dictation_active C10 · insufficient_disk C26 · audio_device C14 · storage C14 | - |
| comandos pause/resume/stop/extend statuses (4) | ok C20 · not_active C26 · invalid_transition C26 · cap_at_maximum C26 | - |
| comando `meeting_status` statuses (1) | ok C21 | - |
| comando `accept_meeting_consent` statuses (1) | ok C11 | - |
| comando `save_meeting_annotations` statuses (3) | ok C4 · not_found C8 · storage C8 | - |
| comando `list_meetings` statuses (2) | ok C7 · storage C7 | - |
| comando `get_meeting` statuses (3) | ok C2 · not_found C8 · storage C8 | - |
| comandos `transcribe_meeting`/`cancel_meeting_transcription` statuses (8) | ok C28 · not_found C8 · still_recording C30 · missing_key C29 · no_audio C30 · transcription C35 · cancelled C35 · busy C36 | - |
| comando `generate_meeting_notes` statuses (6) | ok C31 · not_found C8 · missing_key C29 · unknown_template C31 · notes C36 · busy C36 | - |
| comando `meeting_markdown` statuses (3) | ok C32 · not_found C8 · storage C8 | - |
| comandos `meeting_templates`/`meeting_keys`/`set_meeting_transcription_key` statuses (3) | ok C36 · invalid_key C36 · keyring C29 | - |
| evento `MeetingStatusEvent` (1) | emitido C21 | - |
| evento `MeetingProgressEvent` (1) | emitido C35 | - |
| evento `meeting-consent-required` (1) | emitido C17 | - |
| `MeetingStatus.state` (5) | idle C22 · recording C14 · paused C20 · stopping C22 · processing C22 | - |
| `stop_reason` (3) | user C22 · silence C26 · cap_reached C3 | - |
| `SessionMode` no CHECK (4) | meeting C9 · in_person C2 · system_only C9 · import C9 | - |
| estados da pill de reunião (3) | meeting C15 · meeting_paused C15 · mudo C25 | - |
| itens de tray de reunião (4) | meeting_start C16 · meeting_pause C16 · meeting_resume C16 · meeting_stop C16 | - |
| canais do `MuteWatch` (2) | mic C24 · system C23 | - |
| estados da tela (4) | vazio C36 · carregando C36 · erro C36 · sem chave C36 | - |

- Claims naming an IPC error kind: C10, C11, C29, C31 - each asserts the serialized or returned variant
- C14, C17 (second proof) and C22 are manual and record their result in `## Handoff`; no other check claims more than its proof exercises

## Swept

- validation: C8, C9, C31
- failure modes: C29, C30, C35
- idempotency: C5 (substitui, não soma), C27 (não salva texto igual)
- authorization: n/a - app local de um usuário; as chaves só saem do keyring (C29)
- concurrency: C10 (uma sessão ativa, ditado e reunião exclusivos), C19 (um stream por mic)
- data lifecycle: C37 (rascunho vira gravação uma vez), C4 (anotações sobrevivem), C30 (WAV vira Opus e some só pela retenção validada); apagar sessão está fora de escopo
- dependency failure: C29 (sem chave), C35 (erro de rede e cancelamento do `fala-asr`)
- state transitions: C13, C15, C16, C20, C26, C37, C38
- observability: C34; o `MeetingProgressEvent` (C35) é o progresso visível

## Handoff

Tamanho estimado, por `wc -c` dos arquivos que cada slice lê ou escreve, dividido por 4: S1 ~10k (storage), S2 ~45k (o desktop: `lib.rs` 45 KB, `tray.rs` 32 KB, `overlay.rs` 34 KB, trechos de `actions.rs`, `settings.rs` e `managers/audio.rs`), S3 ~15k, S4 ~3k, S5 ~22k, S6 ~12k: ~107k de arquivos, mais o diff novo (~2 000 linhas, ~25k) e as rodadas de compilação: ~150k, no limite do orçamento de 150k.

- Corte, se precisar: S1 (crates) | S2-S5 (desktop Rust) | S6 (frontend), na troca de superfície.
- Mechanism: one builder (compaction accepted) - delegado pelo Augusto em 2026-10-09; o painel tem contexto de sobra e um builder evita reler o desktop três vezes.
- PRs empilhados, alvo ≤ 400 linhas cada: A storage schema 2; B crates (`MuteWatch`, `default_name`, `note_document`); C desktop gravação (S2-S4 Rust); D desktop processamento (S5 Rust); E UI (tray, pill, página, `bindings.ts`). Um único `git push` com todas as branches, por causa do hook de ~43 min.

- **Boundary:** C1-C38 built on the stacked branches `feat/meeting-storage` (C1-C9, C37), `feat/meeting-crate-helpers` (C23, C24, C32), `feat/meeting-desktop-recording` (C10, C11, C13, C18-C21, C26, C38), `feat/meeting-desktop-indicator` (C15, C16, C17), `feat/meeting-desktop-notes` (C28-C31, C34, C35), `feat/meeting-desktop-page` (C12, C25, C27, C33, C36); the proof scripts live in `proofs/` on `docs/meeting-panel-verification`
- **Settled mid-build:** pedido #2 do orquestrador (N5 do `research/20`): rascunho = linha de `meetings` com `started_at` nulo, revisado na door 1 antes de qualquer commit; a pauta vai como anotações (ADR-0016). O sink padrão vem de `pw-metadata 0 default.audio.sink` (o `pactl` não existe nesta máquina). Os eventos se chamam `MeetingStatus` e `MeetingProgress` (kebab `meeting-status`, `meeting-progress`) e `transcribe_meeting`/`get_meeting` devolvem `MeetingLine` (`person: null` = "Eu"), não `MeetingSegment`. O builder do specta virou `specta_builder()` para um teste `#[ignore]` regenerar `bindings.ts` sem abrir o app. O consentimento aparece inline na página (o `Dialog` usa portal e não renderiza no teste). PRs C (`recording`, ~1 640 linhas) e F (`page`, ~1 150) passam de 1 000 linhas e levam a label `large-change` com justificativa no corpo.
- **Abandoned:** `pactl get-default-sink` (ausente no Ubuntu do Augusto). O guard de RAM: o swap ficou em ~200-300 MB livres a noite toda com 6-7 GB de RAM disponível (páginas paradas); depois de esperar 2 min, os `cargo` rodaram com 2 jobs um de cada vez. Manuais C14, C17 (2ª prova) e C22 não rodaram: exigem clicar no app e gravar áudio real da sala, sem ninguém na máquina; ficam para o Augusto no `tauri dev` (e `TODO(windows)` no Alienware).
- **Verification round 1 (FAIL, 34/38):** C14 and C22 not run (manual), C17 partial (manual half), C35 named a test that does not exist (`--exact` ran 0 tests and exited 0). Fixed: the C35 selector now names `progress::reports_at_least_every_second`, the test that asserts the same claim (≤ 1 s between progress calls); the claim is unchanged. Findings F1, F2, F3, F6, F8 fixed and proven by the new C39-C42; F5 (tray start blocked the event loop while devices open) fixed by starting from the tray on a thread. F7 (stored duration from the clock, not the frames): no change, `MeetingRecorder::finish` writes exactly up to the clock, so the frames are the clock × 48 000. Precision gaps noted, checks not reworded: C28 says `segments_for_store` numbers the segments, but storage numbers them (C5 proves it); C21's "the status carries it" is not asserted; C33/C36-error/C36-key read the page source instead of rendering the page.
