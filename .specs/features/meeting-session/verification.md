# meeting-session verification

**Verdict**: PASS
**Profile**: light
**Diff range**: 008102f..d9bac98
**Round**: 2 - scoped
**Verifier**: independent sub-agent (author != verifier)

Resumo: a correção `7b79063..d9bac98` fecha o único motivo do FAIL da rodada 1. `StartRecording` numa sessão em `Recording` agora tem prova: `pause::invalid_action_keeps_state` ganhou o caso (`crates/meeting/tests/session.rs:350-356`), que exige `InvalidTransition` e o estado inalterado. As lacunas de precisão de C1, C21 e C27 foram resolvidas: C1 mudou o texto do check, C21 e C27 ganharam assertivas. O contrato de `CaptureFinalized` foi fixado em doc comment, mas segue sem prova, porque é obrigação do gravador (trilha F). A lacuna de C15 não foi tocada e continua como nota, sem afetar o veredito. As 34 provas rodaram de novo em d9bac98 e todas saíram `ok`.

## Round 1

Rodada 1, `full`, em 7b79063, diff `008102f..7b79063`. Veredito: FAIL. Os 30 checks estavam provados e cada um tinha assertiva localizada. Nenhum contradizia a ADR-0005 nem a decisão 6 do roadmap. O FAIL vinha de um membro de cobertura: a tabela `Coverage` de `checks.md` atribuía `StartRecording` em `Recording` a C12, e nenhum teste exercitava esse caso. Achados que não fechavam o veredito:

- C1: o texto falava em construir "por literal", e o doctest testa a atribuição ao campo privado.
- C15: a segunda prova compara nomes escritos à mão.
- C21: o rearme do aviso depois de `ExtendCap` em `Paused` não era exercitado.
- Contrato `CaptureFinalized`: havia o risco de um `FinalizeCapture` duplo depois de `Suspended` e de `Process` nunca sair se o gravador não respondesse.
- `SessionId`: a recusa do primeiro caractere acima de `7` não tinha teste.

## Binding sources

Carried from 7b79063. A correção não toca a interface: só um doc comment em `session.rs` e testes. A extensão de C12, C21 e C27 cabe nas mesmas fontes.

| Source | Opened | Contradiction | Uncovered |
| --- | --- | --- | --- |
| ADR-0005, seção Confirmação (`docs/decisions/0005-reuniao-transcrita-na-nuvem-com-audio-retido-localmente.md`) | yes - lida na rodada 1 (carried from 7b79063) | none. O novo caso de C12, clique duplo em gravar sem reabrir a captura, é justamente a confirmação da ADR | - |
| Roadmap 2026-10-02, decisão 6 (`fala-research/plans/roadmap-proposta-2026-10-02.md` §4) | yes - lida na rodada 1 (carried from 7b79063) | none. O rearme do aviso em `Paused` (C21) concorda com "mais 1 h" | - |
| Roadmap 2026-10-02, decisão 2 | yes - lida na rodada 1 (carried from 7b79063) | none | - |

## Checks

Provas executadas em HEAD d9bac98 com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`, uma invocação de cada vez:

- `cargo test -p fala-meeting`: exit 0. Rodaram 6 testes unitários, 25 de integração, 1 doctest positivo e 2 doctests `compile_fail`, e cada nome aparece individualmente como `ok`.
- `cargo test -p fala-meeting --doc compile_fail`: exit 0, 3 `ok`.
- `bash scripts/check-no-tauri-in-crates.sh`: exit 0.
- `rg 'cfg\((target_os|windows|unix)' crates/meeting`: exit 1, sem resultado.

`git diff --stat HEAD -- crates/meeting` sai vazio, então as fontes testadas são as de d9bac98. O trabalho de audio-retention no worktree não afetou a compilação de `fala-meeting`.

Os checks re-julgados (C1, C12, C21, C27) e os que citam arquivos tocados pela correção estão marcados como `verified at d9bac98`. A correção mexeu em `tests/session.rs`, `src/session.rs` e `src/id.rs`, então as citações nesses arquivos foram atualizadas. Os deslocamentos em `tests/session.rs` são +7 depois da linha 349 e +20 depois do bloco de C21; em `src/session.rs`, +6 depois da linha 55.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | só `new`, em `Idle`; de fora não se troca o `state` privado nem se constrói `Indicator`; doctest positivo por `new` | `start::new_session_is_idle ... ok`; `lib.rs - compile_fail (line 57) ... ok`, `(line 72) - compile fail ... ok`, `(line 81) - compile fail ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:124` - `assert_eq!(s.state(), SessionState::Idle)`; `crates/meeting/src/lib.rs:75` - `session.state = state;` (compile_fail, E0616 conferido na rodada 1); `crates/meeting/src/lib.rs:83` - `Indicator { kind: IndicatorKind::Recording }` (compile_fail, E0451); `crates/meeting/src/lib.rs:59,66` - `MeetingSession::new(SessionConfig {..})` e `assert_eq!(session.state(), SessionState::Idle)`. O texto novo do check descreve exatamente essas provas, e a lacuna de precisão da rodada 1 fica fechada | PASS |
| C2 | `Idle` + `StartRecording` com 2 GiB -> `Recording`, `[ShowIndicator(Recording), StartCapture]` | `start::with_enough_disk_shows_indicator_before_capture ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:141` - `assert_eq!(effects, vec![Effect::ShowIndicator(IndicatorKind::Recording), Effect::StartCapture])`; `:148` - `matches!(s.state(), SessionState::Recording { .. })` | PASS |
| C3 | em `Idle`, as outras 8 entradas deixam `Idle` sem abrir captura | `start::only_start_recording_leaves_idle ... ok` (verified at d9bac98; julgamento carried from 7b79063) | `crates/meeting/tests/session.rs:155` - `assert_eq!(inputs.len(), 9)`; `:160` - `assert!(!opens_capture(effects))`; `:162` - `assert_eq!(s.state(), SessionState::Idle)`. Só dois braços escrevem `Recording`: `crates/meeting/src/session.rs:276` (`ResumeRecording`) e `:339` (`start`, alcançado só de `Idle` em `:261`) | PASS |
| C4 | `indicator()` é `Some` em `Recording`/`Paused`/`Suspended` e `None` nos demais | `start::indicator_present_exactly_while_capturing_or_waiting ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:169-177` - `assert_eq!(kind(&session(..)), None)` até `assert_eq!(kind(&stopped()), None)`, com `Some(IndicatorKind::Paused)` e `Some(IndicatorKind::Suspended)` entre eles | PASS |
| C5 | 400 MiB -> `InsufficientDisk { 400 MiB, 100 MiB }` e segue `Idle`; 500 MiB inicia | `start::refused_below_500_mib ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:189` - `assert_eq!(result, Err(SessionError::InsufficientDisk { free_bytes: 400 * MIB, missing_bytes: 100 * MIB }))`; `:196` - `Idle`; `:207` - `effects.contains(&Effect::StartCapture)` | PASS |
| C6 | 1 GiB -> `[WarnLowDisk, ShowIndicator(Recording), StartCapture]`; 2 GiB sem aviso | `start::warns_below_2_gib ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:222` - `assert_eq!(effects, vec![Effect::WarnLowDisk { free_bytes: GIB }, ..])`; `:240` - `!effects.iter().any(.. WarnLowDisk ..)` | PASS |
| C7 | `import` recusa com `ModeDoesNotRecord`; os outros 3 modos iniciam | `start::import_mode_does_not_record ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:248` - `Err(SessionError::ModeDoesNotRecord(SessionMode::Import))`; `:252` - `Idle`; `:260` - `Recording` para os 3 modos | PASS |
| C8 | `Recording` + `PauseRecording` -> `Paused`, `[PauseCapture, ShowIndicator(Paused)]` | `pause::from_recording ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:275` - `assert_eq!(effects, vec![Effect::PauseCapture, Effect::ShowIndicator(IndicatorKind::Paused)])`; `:282` - `Paused { .. }` | PASS |
| C9 | `Paused` + `ResumeRecording` -> `Recording`, `[ShowIndicator(Recording), ResumeCapture]` | `pause::resume_returns_to_recording ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:289` - `assert_eq!(effects, vec![Effect::ShowIndicator(IndicatorKind::Recording), Effect::ResumeCapture])`; `:296` - `Recording { .. }` | PASS |
| C10 | em `Paused`/`Suspended`, as 8 entradas que não são `ResumeRecording` não abrem captura nem levam a `Recording` | `pause::only_resume_recording_resumes ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:311` - `assert!(!opens_capture(&effects))`; `:313` - `assert!(!matches!(s.state(), SessionState::Recording { .. }))` | PASS |
| C11 | pausa aos 10 min, retoma -> `Gap { Pause, 10 min, T1, Some(T2) }` | `pause::gap_recorded ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:327` - `assert_eq!(s.gaps(), &[Gap { kind: GapKind::Pause, audio_offset: 10 * MIN, from: at(10), to: Some(at(13)) }])` | PASS |
| C12 | 5 casos inválidos, incluindo `StartRecording` em `Recording`, devolvem `InvalidTransition { state, action }` sem mudar o estado | `pause::invalid_action_keeps_state ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:341-356` - os 5 casos, com o novo em `:350-356`: `(recording(SessionMode::Meeting), UserAction::StartRecording { free_disk_bytes: 10 * GIB })`; `:360-365` - `assert_eq!(s.apply(at(9), user(action)), Err(SessionError::InvalidTransition { state: before, action }))`; `:367` - `assert_eq!(s.state(), before)`. O caso novo discrimina: se o braço de `crates/meeting/src/session.rs:261` aceitasse `Recording`, sairia `Ok` com `StartCapture` e a primeira assertiva falharia. O caminho de recusa é `session.rs:293` | PASS |
| C13 | `StopRecording` em `Recording`/`Paused`/`Suspended` -> `Stopping { User }`, `[FinalizeCapture]` | `stop::by_user_from_each_live_state ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:383` - `assert_eq!(effects, vec![Effect::FinalizeCapture])`; `:384-390` - `Stopping { reason: StopReason::User }` | PASS |
| C14 | `Stopping` + `CaptureFinalized` -> `Stopped`, `[HideIndicator, Process]` | `stop::finalized_hides_indicator_then_processes ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:398` - `assert_eq!(effects, vec![Effect::HideIndicator, Effect::Process])`; `:399-404` - `Stopped { reason: StopReason::User }` | PASS |
| C15 | `Process` exatamente uma vez nos 4 caminhos; `Effect` sem variante de descarte | `stop::every_path_processes_once ... ok`; `session::tests::effect_has_no_discard_variant ... ok` (verified at d9bac98; julgamento carried from 7b79063) | `crates/meeting/tests/session.rs:464` - `assert_eq!(processed, 1)` com `:458-459` - `matches!(s.state(), SessionState::Stopped { .. })`, caminhos em `:419-453`; `crates/meeting/src/session.rs:406-417` - `match` exaustivo; `:436` - `assert!(!name.contains(forbidden))`. A nota da rodada 1 continua valendo: a segunda prova compara strings escritas à mão, e o guarda real é o `match` exaustivo mais a leitura de `Effect` em `session.rs:125-145`. A correção não tocou isso | PASS |
| C16 | 15 min de silêncio -> `Stopping { Silence }` no tick 15 e não antes; `system_only` ignora o mic; `meeting` com mic alto não para | `stop::after_15_min_of_silence ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:473` - `assert!(!effects.contains(&Effect::FinalizeCapture))`; `:476` - `assert_eq!(effects, vec![Effect::FinalizeCapture])`; `:477-482` - `Stopping { reason: StopReason::Silence }`; `:491-496` - `system_only`; `:503` - `meeting` segue `Recording` | PASS |
| C17 | som aos 14 min zera a contagem; parada 15 min depois | `stop::silence_counter_resets_on_sound ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:515-518` - `Recording` nos minutos 15-28; `:521-526` - `assert_eq!(s.state(), SessionState::Stopping { reason: StopReason::Silence })` | PASS |
| C18 | `from_hours` aceita 1 e 8, recusa 0 e 9, padrão 3 h | `cap::tests::cap_bounds ... ok` (carried from 7b79063; `cap.rs` não foi tocado; prova re-rodada `ok` em d9bac98) | `crates/meeting/src/cap.rs:59-68` - `assert_eq!(RecordingCap::from_hours(0), Err(SessionError::CapOutOfRange(0)))`, idem 9, 1 e 8; `:69-72` - `default().duration() == 3 * 3600 s` | PASS |
| C19 | `CapWarning { 10 min }` aos 2 h 50, sem repetir até 2 h 59 | `cap::warns_once_ten_minutes_before ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:537` - `effects.is_empty()` aos 169; `:539` - `assert_eq!(effects, vec![Effect::CapWarning { remaining: 10 * MIN }])`; `:547` - `assert!(effects.is_empty())` de 171 a 179 | PASS |
| C20 | tick de 3 h -> `Stopping { CapReached }`, `[FinalizeCapture]` | `cap::reached_stops ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:556` - `assert_eq!(effects, vec![Effect::FinalizeCapture])`; `:557-562` - `Stopping { reason: StopReason::CapReached }` | PASS |
| C21 | `ExtendCap` em `Recording` e em `Paused` sobe 3 -> 4 h e, nos dois, o aviso volta aos 3 h 50; em 8 h, `CapAtMaximum` e o teto fica | `cap::extend_adds_one_hour_and_rearms ... ok`; `cap::extend_refused_at_8_hours ... ok` (verified at d9bac98) | `Recording`: `crates/meeting/tests/session.rs:574` - `assert_eq!(s.cap().hours(), 4)`; `:579` - `assert_eq!(effects, vec![Effect::CapWarning { remaining: 10 * MIN }])` aos 230 min. `Paused` (novo): `:589` - `assert_eq!(effects.len(), 1)` (o aviso já saiu aos 170, que é a pré-condição do rearme); `:595` - `hours() == 4`; `:597-603` - `assert_eq!(effects, vec![Effect::CapWarning { remaining: 10 * MIN }])` no tick de 230 min gravados depois de retomar. O teste discrimina nos dois sentidos: sem rearme (`cap_warned` seguiria `true`) o efeito sairia vazio, e sem o teto novo 230 min passariam das 3 h e sairia `FinalizeCapture`. Código: `crates/meeting/src/session.rs:288-291`. 8 h: `:611` - `Err(SessionError::CapAtMaximum)`; `:615` - `hours() == 8`. A lacuna de precisão da rodada 1 fica fechada | PASS |
| C22 | em `Paused`/`Suspended`, tick de 5 h não muda nada nem devolve efeito | `cap::ticks_ignored_while_not_recording ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:624` - `assert!(effects.is_empty())`; `:625` - `assert_eq!(s.recorded(), Duration::ZERO)`; `:626` - `assert_eq!(s, before)` | PASS |
| C23 | `OsSuspended` em `Recording`/`Paused` -> `Suspended`, `[FinalizeCapture, ShowIndicator(Suspended)]` | `suspend::from_recording_and_paused ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:641` - `assert_eq!(effects, vec![Effect::FinalizeCapture, Effect::ShowIndicator(IndicatorKind::Suspended)])`; `:649-652` - `Suspended { .. }` | PASS |
| C24 | `OsResumed` em `Suspended` mantém `Suspended`, `[AskContinueOrStop]` | `suspend::os_resume_asks ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:660` - `assert_eq!(effects, vec![Effect::AskContinueOrStop])`; `:661` - `Suspended { .. }` | PASS |
| C25 | `ResumeRecording` em `Suspended` -> `Recording`, `[ShowIndicator(Recording), ResumeCapture]`, `Gap { Suspend }` | `suspend::user_resume_records_gap ... ok` (verified at d9bac98) | `crates/meeting/tests/session.rs:671` - `assert_eq!(effects, vec![Effect::ShowIndicator(IndicatorKind::Recording), Effect::ResumeCapture])`; `:679` - `assert_eq!(s.gaps(), &[Gap { kind: GapKind::Suspend, audio_offset: 4 * MIN, from: at(4), to: Some(at(41)) }])` | PASS |
| C26 | `from_parts(1_727_000_000_000, 0)` = `01J8CKHDG0` + 16 zeros, ida e volta; entropia máxima termina em 16 `Z` | `id::tests::from_parts_encodes_time_and_round_trips ... ok` (verified at d9bac98; a mudança em `id.rs` fica depois deste teste) | `crates/meeting/src/id.rs:131` - `assert_eq!(text, format!("01J8CKHDG0{}", "0".repeat(16)))`; `:133` - ida e volta; `:138` - `assert_eq!(&max[10..], "Z".repeat(16))` | PASS |
| C27 | `from_str` recusa 25 e 27 caracteres, `I`/`L`/`O`/`U` e primeiro caractere acima de `7` com `InvalidSessionId` | `id::tests::rejects_bad_text ... ok` (verified at d9bac98) | `crates/meeting/src/id.rs:148-151` - `assert_eq!(bad.parse::<SessionId>(), Err(SessionError::InvalidSessionId(bad.clone())))` para os dois tamanhos; `:155-158` - idem por letra; `:161-165` (novo) - `overflow = format!("8{}", &good[1..])` e `assert_eq!(overflow.parse::<SessionId>(), Err(SessionError::InvalidSessionId(overflow.clone())))`. O caso de borda que faltava na rodada 1 agora está no check e na prova. Nota: AC 27 do plano não cita a borda `7`; o check alega mais que o AC, o que não contradiz nada | PASS |
| C28 | dois `generate(T0)` diferem; `generate(T0 + 1)` ordena depois | `id::tests::generate_unique_and_time_ordered ... ok` (verified at d9bac98) | `crates/meeting/src/id.rs:173` - `assert_ne!(a, b)`; `:174-175` - `assert!(later.to_string() > a.to_string())`, idem `b` | PASS |
| C29 | `SessionMode` em snake_case e `SessionId` como string de 26, ida e volta em JSON | `id::tests::mode_and_id_serialized_forms ... ok` (verified at d9bac98) | `crates/meeting/src/id.rs:186-187` - `assert_eq!(serde_json::to_string(&mode).unwrap(), json)` e o inverso para os 4 modos; `:192-194` - `assert_eq!(json, format!("\"{id}\""))`, `json.len() == 26 + 2`, ida e volta | PASS |
| C30 | sem `tauri` e sem `cfg(target_os/windows/unix)` em `crates/meeting` | `bash scripts/check-no-tauri-in-crates.sh` exit 0 (`ok: no tauri in crates/`); o `rg` de C30 exit 1, sem resultado (verified at d9bac98) | `scripts/check-no-tauri-in-crates.sh:12-17` - `cargo tree -p "$pkg"` filtrado por `^tauri` sobre `crates/*/Cargo.toml`; `crates/meeting/Cargo.toml:10-13` - só `getrandom`, `serde`, `thiserror` (carried from 7b79063, arquivo não tocado) | PASS |

## Coverage

Perfil `light`: o join não foi recomputado. A única linha re-julgada é a que falhou na rodada 1 (verified at d9bac98).

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| entradas em `Recording` (6 que mudam algo, tabela do autor) | não recomputado (light); lido contra `tests/session.rs` em d9bac98 | `PauseRecording` C8 · `StopRecording` C13 · `ExtendCap` C21 · `Tick` C16 · `OsSuspended` C23 · `StartRecording` C12 (`crates/meeting/tests/session.rs:350-356`, assertivas em `:360-367`) | - |

Observações carried from 7b79063 que não fecham o veredito:

- `ResumeRecording`, `CaptureFinalized` e `OsResumed` em `Recording` seguem sem prova própria. O argumento de "mesmo caminho de código" vale pela leitura de `crates/meeting/src/session.rs:293` e `:313-314`, mas é argumento, não prova.
- `Stopping` e `Stopped` não têm linha própria de conjunto. C12 cobre um caso de cada.

## Swept

Carried from 7b79063. As restrições citadas pelas linhas `n/a` continuam no código, só com novas linhas: `apply(&mut self, ...)` está em `crates/meeting/src/session.rs:258`, e `getrandom` vira `SessionError::Entropy` em `crates/meeting/src/id.rs:34`, linha que a correção não deslocou.

## Test policy rows

`checks.md` não tem seção `Test policy`; nada a julgar.

## Faults injected

Não roda no perfil `light`.

## Findings

Achados restantes, em ordem de prioridade. Nenhum fecha o veredito.

1. **Contrato de `CaptureFinalized` documentado, não provado.** O doc comment em `crates/meeting/src/session.rs:57-61` fixa três pontos: o gravador só responde a um `FinalizeCapture` pedido em `Stopping`; em `Suspended` a entrada é ignorada; e o segundo `FinalizeCapture` pedido depois da suspensão é tratado como já feito. O lado da máquina de estados confere com o código: `session.rs:309-311` só sai de `Stopping`, e `:314` ignora a entrada no resto. Isso resolve a corrida da rodada 1 por contrato, mas a obrigação é do gravador. Fica para a trilha F, que precisa de um teste do lado do gravador. Também segue sem saída a sessão presa em `Stopping` se o gravador falhar, sem timeout. O comportamento está documentado e não é tratado.
2. **C15, segunda prova fraca.** `effect_has_no_discard_variant` (`crates/meeting/src/session.rs:406-436`) compara nomes escritos à mão. Não foi tocado nesta correção, e o guarda real continua sendo o `match` exaustivo.
3. **`SessionId` em minúsculas.** É recusado, o que bate com o plano, mas diverge do ULID padrão na leitura. Carried from 7b79063, sem mudança.

## Gate

`cargo test -p fala-meeting`, rodado com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`: 34 passed, 0 failed (6 unitários, 25 de integração, 1 doctest, 2 doctests `compile_fail`).

Demais comandos:

- `cargo test -p fala-meeting --doc compile_fail`: 3 passed, 0 failed.
- `bash scripts/check-no-tauri-in-crates.sh`: exit 0.
- `rg 'cfg\((target_os|windows|unix)' crates/meeting`: sem resultado.

Não rodados: `cargo clippy` e `cargo fmt --check`, que ficam fora das provas dos checks.

Árvore em d9bac98: `Cargo.toml` e `Cargo.lock` estão modificados, e `crates/retention/`, `.specs/features/audio-retention/` e a ADR-0014 não estão versionados. É trabalho de audio-retention, fora do escopo e não avaliado. O `cargo` compilou `fala-meeting` sem reclamar. `git diff --stat HEAD -- crates/meeting` saiu vazio.
