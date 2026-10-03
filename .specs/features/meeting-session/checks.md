# meeting-session checks

Profile: light
Plan: `.specs/features/meeting-session/plan.md`

30 checks in 6 slices · 5 one-way doors · 0 open, of which 0 block

Todas as provas rodam com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2` (regra da rodada).

## Checks

### S1 - início só por ação explícita, com indicador · ~6 files · ~25 KB · ~7k

**C1** - `MeetingSession::new` devolve uma sessão em `Idle`, e nenhum código fora do crate consegue construir `MeetingSession` em outro estado nem um `Indicator` (AC 1)
Proof: `cargo test -p fala-meeting --test session start::new_session_is_idle -- --exact`
Proof: `cargo test -p fala-meeting --doc compile_fail` (doctests do módulo oculto `compile_fail`: atribuir ao campo privado `state` de `MeetingSession` e construir `Indicator` fora do crate não compilam; um doctest positivo do mesmo módulo cria a sessão por `new`)

**C2** - Em `Idle`, `StartRecording` com 2 GiB livres leva a `Recording` e devolve exatamente `[ShowIndicator(Recording), StartCapture]` (AC 2)
Proof: `cargo test -p fala-meeting --test session start::with_enough_disk_shows_indicator_before_capture -- --exact`

**C3** - Em `Idle`, cada uma das outras 8 entradas (`PauseRecording`, `ResumeRecording`, `StopRecording`, `ExtendCap`, `Tick`, `OsSuspended`, `OsResumed`, `CaptureFinalized`) deixa a sessão em `Idle` e nenhuma devolve `StartCapture` nem `ResumeCapture` (AC 3)
Proof: `cargo test -p fala-meeting --test session start::only_start_recording_leaves_idle -- --exact`

**C4** - `indicator()` é `Some(Recording)`, `Some(Paused)`, `Some(Suspended)` nesses três estados e `None` em `Idle`, `Stopping` e `Stopped` (AC 4)
Proof: `cargo test -p fala-meeting --test session start::indicator_present_exactly_while_capturing_or_waiting -- --exact`

**C5** - `StartRecording` com 400 MiB livres devolve `InsufficientDisk { free_bytes: 400 MiB, missing_bytes: 100 MiB }` e a sessão segue em `Idle`; com 500 MiB exatos, inicia (AC 5)
Proof: `cargo test -p fala-meeting --test session start::refused_below_500_mib -- --exact`

**C6** - `StartRecording` com 1 GiB livre devolve `[WarnLowDisk { free_bytes: 1 GiB }, ShowIndicator(Recording), StartCapture]`; com 2 GiB exatos não há `WarnLowDisk` (AC 6)
Proof: `cargo test -p fala-meeting --test session start::warns_below_2_gib -- --exact`

**C7** - Uma sessão `import` recusa `StartRecording` com `ModeDoesNotRecord` e segue em `Idle`; `meeting`, `in_person` e `system_only` iniciam (AC 7)
Proof: `cargo test -p fala-meeting --test session start::import_mode_does_not_record -- --exact`

### S2 - pausar e retomar · ~2 files · ~15 KB · ~4k

**C8** - `PauseRecording` em `Recording` leva a `Paused` e devolve `[PauseCapture, ShowIndicator(Paused)]` (AC 8)
Proof: `cargo test -p fala-meeting --test session pause::from_recording -- --exact`

**C9** - `ResumeRecording` em `Paused` leva a `Recording` e devolve `[ShowIndicator(Recording), ResumeCapture]` (AC 9)
Proof: `cargo test -p fala-meeting --test session pause::resume_returns_to_recording -- --exact`

**C10** - Em `Paused` e em `Suspended`, cada entrada diferente de `ResumeRecording` (as outras 4 ações e as 4 entradas do sistema) nunca devolve `StartCapture` nem `ResumeCapture` e nunca leva a `Recording` (AC 10)
Proof: `cargo test -p fala-meeting --test session pause::only_resume_recording_resumes -- --exact`

**C11** - Pausar aos 10 min gravados em T1 e retomar em T2 registra `Gap { kind: Pause, audio_offset: 10 min, from: T1, to: Some(T2) }` (AC 11)
Proof: `cargo test -p fala-meeting --test session pause::gap_recorded -- --exact`

**C12** - Ação inválida devolve `InvalidTransition { state, action }` sem mudar o estado, para os 5 casos: `PauseRecording` em `Paused`, `StopRecording` em `Idle`, `StartRecording` em `Stopping`, `ResumeRecording` em `Stopped`, `StartRecording` em `Recording` (AC 12)
Proof: `cargo test -p fala-meeting --test session pause::invalid_action_keeps_state -- --exact`

### S3 - parar e processar, nunca descartar · ~2 files · ~15 KB · ~4k

**C13** - `StopRecording` em `Recording`, `Paused` e `Suspended` leva a `Stopping` com `StopReason::User` e devolve `[FinalizeCapture]` (AC 13)
Proof: `cargo test -p fala-meeting --test session stop::by_user_from_each_live_state -- --exact`

**C14** - `CaptureFinalized` em `Stopping` leva a `Stopped` e devolve `[HideIndicator, Process]` (AC 14)
Proof: `cargo test -p fala-meeting --test session stop::finalized_hides_indicator_then_processes -- --exact`

**C15** - Nos 4 caminhos até `Stopped` (usuário, silêncio, teto, suspensão seguida de parar), a soma dos efeitos tem `Process` exatamente uma vez; e um `match` exaustivo sobre `Effect` no teste lista as variantes sem nenhuma de apagar ou descartar (AC 15)
Proof: `cargo test -p fala-meeting --test session stop::every_path_processes_once -- --exact`
Proof: `cargo test -p fala-meeting --lib session::tests::effect_has_no_discard_variant -- --exact`

**C16** - Com níveis 0,0005 nos dois canais, ticks de 1 min levam a `Stopping` com `StopReason::Silence` no tick que completa 15 min gravados, e não antes; no `system_only`, mic em 0,5 e sistema em 0,0005 também param aos 15 min, e no `meeting` o mesmo par não para (AC 16)
Proof: `cargo test -p fala-meeting --test session stop::after_15_min_of_silence -- --exact`

**C17** - Um tick com o mic em 0,01 aos 14 min zera a contagem: a parada só vem 15 min depois desse tick (AC 17)
Proof: `cargo test -p fala-meeting --test session stop::silence_counter_resets_on_sound -- --exact`

### S4 - teto de duração · ~2 files · ~15 KB · ~4k

**C18** - `RecordingCap::from_hours` aceita 1 e 8, recusa 0 e 9 com `CapOutOfRange`, e o padrão é 3 h (AC 18)
Proof: `cargo test -p fala-meeting --lib cap::tests::cap_bounds -- --exact`

**C19** - Com o teto padrão, o tick que alcança 2 h 50 min gravados devolve `[CapWarning { remaining: 10 min }]`, e os ticks seguintes até 2 h 59 min não repetem o aviso (AC 19)
Proof: `cargo test -p fala-meeting --test session cap::warns_once_ten_minutes_before -- --exact`

**C20** - O tick que alcança 3 h gravados leva a `Stopping` com `StopReason::CapReached` e devolve `[FinalizeCapture]` (AC 20)
Proof: `cargo test -p fala-meeting --test session cap::reached_stops -- --exact`

**C21** - `ExtendCap` em `Recording` e em `Paused` sobe o teto de 3 h para 4 h, e nos dois casos o aviso volta a sair aos 3 h 50 min; com teto de 8 h, `ExtendCap` devolve `CapAtMaximum` e o teto segue em 8 h (AC 21)
Proof: `cargo test -p fala-meeting --test session cap::extend_adds_one_hour_and_rearms -- --exact`
Proof: `cargo test -p fala-meeting --test session cap::extend_refused_at_8_hours -- --exact`

**C22** - Em `Paused` e em `Suspended`, um tick com 5 h gravados e níveis em zero não muda a duração gravada, não dispara aviso, teto nem silêncio, e não devolve efeito (AC 22)
Proof: `cargo test -p fala-meeting --test session cap::ticks_ignored_while_not_recording -- --exact`

### S5 - suspensão do SO · ~2 files · ~10 KB · ~3k

**C23** - `OsSuspended` em `Recording` e em `Paused` leva a `Suspended` e devolve `[FinalizeCapture, ShowIndicator(Suspended)]` (AC 23)
Proof: `cargo test -p fala-meeting --test session suspend::from_recording_and_paused -- --exact`

**C24** - `OsResumed` em `Suspended` mantém `Suspended` e devolve `[AskContinueOrStop]` (AC 24)
Proof: `cargo test -p fala-meeting --test session suspend::os_resume_asks -- --exact`

**C25** - `ResumeRecording` em `Suspended` leva a `Recording`, devolve `[ShowIndicator(Recording), ResumeCapture]` e registra um `Gap` com `kind: Suspend` (AC 25)
Proof: `cargo test -p fala-meeting --test session suspend::user_resume_records_gap -- --exact`

### S6 - identidade e modo da sessão · ~3 files · ~10 KB · ~3k

**C26** - `SessionId::from_parts(1_727_000_000_000, 0)` é `01J8CKHDG0` seguido de 16 zeros (26 caracteres), e `from_str` do texto devolve o mesmo id; um id com entropia máxima termina em 16 `Z` (AC 26)
Proof: `cargo test -p fala-meeting --lib id::tests::from_parts_encodes_time_and_round_trips -- --exact`

**C27** - `from_str` recusa com `InvalidSessionId` um texto de 25 e um de 27 caracteres, textos com `I`, `L`, `O` e `U`, e um primeiro caractere acima de `7` (AC 27)
Proof: `cargo test -p fala-meeting --lib id::tests::rejects_bad_text -- --exact`

**C28** - Dois `generate(1_727_000_000_000)` são diferentes, e `generate(1_727_000_000_001)` ordena depois de ambos como texto (AC 28)
Proof: `cargo test -p fala-meeting --lib id::tests::generate_unique_and_time_ordered -- --exact`

**C29** - `SessionMode` serializa como `"meeting"`, `"in_person"`, `"system_only"`, `"import"` e `SessionId` como a string de 26 caracteres, e os dois voltam iguais do JSON (AC 29)
Proof: `cargo test -p fala-meeting --lib id::tests::mode_and_id_serialized_forms -- --exact`

**C30** - `scripts/check-no-tauri-in-crates.sh` sai com 0 e `rg 'cfg\((target_os|windows|unix)' crates/meeting` não encontra nada (AC 30)
Proof: `bash scripts/check-no-tauri-in-crates.sh` (exit 0)
Proof: `test -z "$(rg 'cfg\((target_os|windows|unix)' crates/meeting)"`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| entradas em `Idle` (9) | `StartRecording` C2 · `PauseRecording` C3 · `ResumeRecording` C3 · `StopRecording` C3 · `ExtendCap` C3 · `Tick` C3 · `OsSuspended` C3 · `OsResumed` C3 · `CaptureFinalized` C3 | - |
| entradas em `Paused` (9) | `ResumeRecording` C9 · `StopRecording` C13 · `ExtendCap` C21 · `OsSuspended` C23 · `PauseRecording` C10 · `StartRecording` C10 · `Tick` C22 · `OsResumed` C10 · `CaptureFinalized` C10 | - |
| entradas em `Suspended` (9) | `ResumeRecording` C25 · `StopRecording` C13 · `OsResumed` C24 · `Tick` C22 · `StartRecording` C10 · `PauseRecording` C10 · `ExtendCap` C10 · `OsSuspended` C10 · `CaptureFinalized` C10 | - |
| entradas em `Recording` (6 que mudam algo) | `PauseRecording` C8 · `StopRecording` C13 · `ExtendCap` C21 · `Tick` C16 · `OsSuspended` C23 · `StartRecording` C12 | - |
| estados com e sem indicador (6) | `Idle` C4 · `Recording` C4 · `Paused` C4 · `Suspended` C4 · `Stopping` C4 · `Stopped` C4 | - |
| `StopReason` (3) | `User` C13 · `Silence` C16 · `CapReached` C20 | - |
| caminhos até `Stopped` (4) | usuário C15 · silêncio C15 · teto C15 · suspensão + parar C15 | - |
| `SessionMode` (4) | `meeting` C7 · `in_person` C7 · `system_only` C7 · `import` C7 | - |
| limites de disco (4 bordas) | 400 MiB C5 · 500 MiB C5 · 1 GiB C6 · 2 GiB C6 | - |
| limites do teto (4 bordas) | 0 C18 · 1 C18 · 8 C18 · 9 C18 | - |
| `GapKind` (2) | `Pause` C11 · `Suspend` C25 | - |
| Landing doors (5) | crate novo C30 · máquina de estados C3 · formato do `SessionId` C26 · forma do `SessionMode` C29 · `getrandom` C28 | - |

- `Recording` recebendo `ResumeRecording` e `CaptureFinalized`: o primeiro é ação inválida pela regra de C12 (mesmo caminho de código, coberto pelo caso `PauseRecording` em `Paused`), o segundo é entrada de sistema fora de lugar; nenhuma das duas muda o estado, o que C3/C10 provam para os outros estados
- Nenhum check alega mais que os casos que a prova exercita

## Swept

- validation: C5, C6, C18, C27
- failure modes: C5, C12 - recusa sem mudar o estado; falha de I/O é do gravador (trilha F), fora do crate puro
- idempotency: C19 (aviso uma vez por teto), C24 (`OsResumed` repetido só pergunta de novo), C15 (`Process` uma vez)
- authorization: C3, C10 - só a ação explícita do usuário inicia e retoma (ADR-0005)
- concurrency: n/a - `MeetingSession::apply` recebe `&mut self`; o chamador serializa as entradas, e o borrow checker impede duas aplicações simultâneas
- data lifecycle: C15 - nenhum caminho descarta; apagar e reter áudio são da 2.F3
- dependency failure: n/a - o crate não chama nada externo; o único recurso do SO é a entropia do `getrandom`, cuja falha vira `SessionError::Entropy` sem pânico (coberto pela forma do erro, sem como provocar a falha no teste)
- state transitions: C2, C3, C8, C9, C10, C12, C13, C14, C20, C23, C25
- observability: n/a - o crate não loga; os efeitos devolvidos são o rastro, e quem os executa loga (nenhum conteúdo ditado passa por aqui)

## Handoff

- S1-S6 ≈ 25k tokens (crate novo de ~4 arquivos, ~600 linhas com testes, mais `Cargo.toml`, `Cargo.lock` e `ARCHITECTURE.md` lidos por trecho), todos em `fala-meeting`, sob o orçamento de 150k - one builder
- Mechanism: one builder (cabe no orçamento; sem pergunta)
