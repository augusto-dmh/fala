# upstream-catchup PR 1 (correções pequenas do desktop e do postproc) checks

Profile: light
Plan: none - cada tema cabe numa frase (AGENTS.md, "Fluxo por feature")

## Intent

O Handy publicou, depois do ponto de fork (`8f9cf53`), correções pequenas que o Fala ainda não
tem: abrir o mic sem modelo capaz de transcrever (upstream #2161), perder a maiúscula de início de
frase quando um filler maiúsculo é removido (#2157, que no Fala também mora em
`crates/postproc/src/rules.rs`), tratar "ha" como filler em inglês (#2156), enumerar os devices do
transcribe-cpp no início a frio (#2160), o instalador NSIS sem o ícone do app (#2117) e a lista de
sons custom lida só na montagem do seletor (#1941). Com este PR, o atalho sem modelo não abre o
mic, "Isso funciona. Uhm, deixa eu ver" vira "Isso funciona. Deixa eu ver" nos dois filtros, o
instalador mostra o ícone do Fala e o seletor de som relê a pasta ao abrir.

Decisões (Confirmed? y — delegado): cherry-pick `-x` com a autoria do upstream onde aplica limpo;
a regra do postproc é reimplementada sobre tokens, e só um filler que já vinha maiúsculo passa a
maiúscula adiante (mesma regra do upstream); do #2117 só a linha `installerIcon`, sem
`signCommand` (ADR-0008 adia a assinatura).

10 checks in 1 slice · 0 one-way doors · 0 open, of which 0 block

Comandos de cargo com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`, na
raiz do worktree.

## Checks

### S1 - correções pequenas · 9 files · 150 KB · ~38k

**C1** - Em `TranscribeAction::start`, sem modelo carregado e com `get_model_path` do modelo
selecionado em erro, a função loga `Not starting recording` e retorna antes de `set_tray_state`,
do overlay e de `try_start_recording` (upstream #2161)
Proof: `awk '/fn start\(&self, app: &AppHandle, binding_id/{f=1} f&&/Not starting recording/{a=NR} f&&/set_tray_state\(app, TrayIconState::Recording\)/{b=NR} f&&/try_start_recording/{c=NR; exit} END{exit !(a && a<b && b<c)}' apps/desktop/src/actions.rs`

**C2** - O coordinator trata um start que não abriu o mic como idle: `start()` em
`transcription_coordinator.rs` devolve `is_recording()` e o teste existente de início que falhou
continua verde (o Esc não fica armado)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::is_busy_follows_the_stage`

**C3** - No filtro do desktop, "Um, so I think we should ship it." vira "So I think we should
ship it.", "That works. Um, let me check." vira "That works. Let me check." e "He said, Um, not
today." vira "He said, not today." (upstream #2157)
Proof: `cargo test -p fala --lib audio_toolkit::text::tests::test_filter_leading_filler_keeps_sentence_capital`

**C4** - No `Rules` do postproc (pt-BR), "Isso funciona. Uhm, deixa eu ver." vira "Isso funciona.
Deixa eu ver."; depois de `!`, `?` e `…` a palavra seguinte também sobe; dois fillers seguidos
("Ahn ahn,") passam a maiúscula adiante; "ele disse, Hum, hoje não." vira "Ele disse, hoje não."
e um filler minúsculo depois do ponto não cria maiúscula
Proof: `cargo test -p fala-postproc rules::tests::keeps_sentence_capital_after_removed_filler`

**C5** - Os testes de fillers que já existiam no postproc continuam verdes sem asserção editada
Proof: `cargo test -p fala-postproc rules::tests::removes_pt_br_fillers rules::tests::en_keeps_pt_only_fillers`

**C6** - Em inglês, "Ha Long Bay is beautiful." sai igual: "ha" não está mais na lista `en` (upstream #2156)
Proof: `cargo test -p fala --lib audio_toolkit::text::tests::test_filter_keeps_ha_in_english`

**C7** - `init_transcribe_backend` não chama mais `transcribe_compute_devices`; o log dos devices
está em `report_compute_devices`, chamado na thread de pre-warm do `lib.rs` e no caminho headless,
e nunca fora desses dois lugares (upstream #2160)
Proof: `awk '/^pub fn init_transcribe_backend/{f=1} f&&/^}/{exit} f&&/transcribe_compute_devices\(\)/{bad=1} END{exit bad}' apps/desktop/src/managers/transcription.rs`
Proof: `test "$(grep -c 'report_compute_devices()' apps/desktop/src/lib.rs)" -eq 2 && grep -B3 'transcription::report_compute_devices();' apps/desktop/src/lib.rs | grep -q 'std::thread::spawn'`

**C8** - `bundle.windows.nsis.installerIcon` vale `"icons/icon.ico"`, o template usa
`{{installer_icon}}` e não há `signCommand` no `tauri.conf.json` (upstream #2117)
Proof: `test "$(jq -r '.bundle.windows.nsis.installerIcon' apps/desktop/tauri.conf.json)" = icons/icon.ico && grep -q '{{installer_icon}}' apps/desktop/nsis/installer.nsi && ! grep -q signCommand apps/desktop/tauri.conf.json`

**C9** - O `Dropdown` chama `onOpen` só na abertura (não no fechamento) e o `SoundPicker` passa
`onOpen={checkCustomSounds}`; nenhum uso de `onRefresh` sobra em `src/` (upstream #1941)
Proof: `grep -q 'if (!isOpen) onOpen?.();' src/components/ui/Dropdown.tsx && grep -q 'onOpen={checkCustomSounds}' src/components/settings/SoundPicker.tsx && ! grep -rq onRefresh src`
Proof: `bun run lint && bun run build`

**C10** - Nenhuma marca do Handy entra no código
Proof: `scripts/check-brand.sh`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| commits do upstream neste PR (6) | #2161 C1, C2 · #2157 C3, C4 · #2156 C6 · #2160 C7 · #2117 C8 · #1941 C9 | - |
| filtros que removem filler (2 lugares) | `audio_toolkit/text.rs` C3 · `crates/postproc/src/rules.rs` C4 | - |
| fins de frase que abrem a próxima (4) | `.` C3, C4 · `!` C4 · `?` C4 · `…` C4 | - |
| posição do filler maiúsculo (3) | início do texto C3 · depois de fim de frase C3, C4 · meio da frase C3, C4 | - |
| lugares que listam devices (2 assemblies) | pre-warm do app C7 · headless C7 | - |

- Nenhum check afirma mais que o caso que a prova exercita; C1, C7, C8 e C9 são estruturais
  (ordem e presença no fonte), sem teste de comportamento porque o caminho exige `AppHandle`, GPU
  ou bundle do Windows.

## Swept

- validation: C8 (configuração do bundle)
- failure modes: C1 (sem modelo), C2 (start que não gravou)
- idempotency: n/a - nenhuma operação repetível nova
- authorization: n/a - nada de acesso novo
- concurrency: C7 (a enumeração sai da thread principal)
- data lifecycle: n/a - nenhum dado persistido muda
- dependency failure: C1 (modelo ausente no disco)
- state transitions: C2
- observability: C1 (o `warn!` não loga conteúdo ditado)

## Handoff

- S1 = ~38k (9 arquivos, 150 KB / 4), sob o orçamento de 150k - one builder
