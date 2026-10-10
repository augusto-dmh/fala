# desktop-pipeline-audio checks

Profile: light
Plan: `.specs/features/desktop-pipeline-audio/plan.md`

15 checks in 3 slices · 4 one-way doors · 0 open, of which 0 block

## Checks

Prefixo de todo `cargo` abaixo, no Windows: `CARGO_TARGET_DIR=C:\f\pipe`. A lógica nova do desktop fica em `apps/desktop/src/dictation_capture.rs`; o `Processor` dele não abre mic, então os testes o alimentam com amostras e com um detector falso que lê um código no primeiro valor do quadro.

### S1 - o desktop grava pelo `fala-audio`

**C1** - 500 ms a 16 kHz antes do `Start` e 200 ms depois, com detector que sempre diz voz: o `stop` devolve exatamente as amostras 3 200..11 200 (os últimos 300 ms antes do `Start` e tudo depois) (AC 1, door 1)
Proof: `cargo test -p fala --lib dictation_capture::tests::prebuffer_leads_the_recording -- --exact`

**C2** - política `Disabled` com detector que sempre diz silêncio: o `stop` devolve pré-buffer + sessão inteiros; com `Offline` e o mesmo detector, vazio (AC 2)
Proof: `cargo test -p fala --lib dictation_capture::tests::disabled_policy_keeps_everything -- --exact`

**C3** - duas falas separadas por 600 ms de silêncio: o callback de áudio recebe 2 blocos na ordem, e o `stop` devolve a concatenação deles, sem o silêncio do meio além de pre-roll e hangover (AC 3, door 3)
Proof: `cargo test -p fala --lib dictation_capture::tests::utterances_reach_router_and_recording -- --exact`

**C4** - detector que devolve erro: a sessão inteira volta como voz e o recorder não entra em pânico (AC 4)
Proof: `cargo test -p fala --lib dictation_capture::tests::detector_error_counts_as_voice -- --exact`

**C5** - o receptor de prontidão recebe `()` no primeiro bloco depois do `Start`, não antes; um `stop` antes de qualquer bloco solta o emissor (`recv` dá erro) (AC 5)
Proof: `cargo test -p fala --lib dictation_capture::tests::readiness_fires_on_first_block_after_start -- --exact`

**C6** - blocos fora da sessão não chamam o callback de níveis; dentro, chamam (AC 6)
Proof: `cargo test -p fala --lib dictation_capture::tests::levels_only_while_recording -- --exact`

**C7** - o `DictationRecorder` guarda o detector num `Arc` criado no `new`, e o `open` só o clona (AC 7)
Proof: leitura de `DictationRecorder::new` e `open`; `grep -c "SileroVad::load" apps/desktop/src/dictation_capture.rs apps/desktop/src/managers/audio.rs` soma 1

**C8** - `needs_reopen()` lê o `Mic::failed()` que o worker copia a cada volta, além do worker ter terminado (AC 8)
Proof: leitura de `run_worker` e `needs_reopen`; `cargo test -p fala-audio mic::tests -- ` passa

### S2 - abertura do `Mic`

**C9** - `mono(&[0.1, 0.5, 0.9], Some(1))` = 0.5; `Some(3)` e `None` = 0.5 (média); mono `[0.2]` com `Some(0)` = 0.2 (AC 9, door 4)
Proof: `cargo test -p fala-audio mic::tests::mono_picks_channel_or_averages -- --exact`

**C10** - `fala-cli dictate`/`meeting` continuam chamando `Mic::open(needle)`, que delega para `open_device`; os testes do `fala-audio` e do `fala-cli` passam (AC 10)
Proof: `cargo test -p fala-audio` e `cargo test -p fala-cli`

### S3 - app e documentos

**C11** - `managers/audio.rs` não importa mais nada de `audio_toolkit::vad` nem `AudioRecorder`; o seletor de dispositivo chama `update_selected_device`, que reabre pelo recorder novo (AC 11)
Proof: `grep -nE "AudioRecorder|SmoothedVad|EarshotVad|audio_toolkit::vad" apps/desktop/src/managers/audio.rs apps/desktop/src/actions.rs apps/desktop/src/commands/audio.rs` vazio

**C12** - no Windows, `bun run tauri dev` com o store portátil: 5 ditados com `ctrl+shift+space` no Bloco de Notas colam texto; trocar o microfone nos settings e ditar de novo cola texto; o log tem `microfone: ` (crate) e `dictation capture: first samples` com o tempo (AC 12; `TODO(windows)` resolvido aqui, manual)
Proof: manual, registrado no `verification.md` com as linhas do log

**C13** - `ARCHITECTURE.md`: o parágrafo de estado diz que o desktop usa `audio` para captura e VAD, e a linha "Pré-buffer de áudio" cita a door 1 (AC 13)
Proof: `grep -n "Pré-buffer" ARCHITECTURE.md` e leitura do parágrafo "Estado"

**C14** - o seletor Earshot sumiu: nenhum `VadBackendSelector` nem `change_vad_backend_setting` em `src/` e `apps/desktop/src`, fora de `bindings.ts` regenerado; um store com `"vad_backend": "earshot"` ainda carrega (door 2)
Proof: `grep -rn "VadBackendSelector\|changeVadBackendSetting\|change_vad_backend_setting" src apps/desktop/src` vazio; `cargo test -p fala --lib settings::tests::earshot_store_still_loads -- --exact`

### Gate

**C15** - `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p fala --lib`, `cargo test -p fala-audio`, `scripts/check-no-tauri-in-crates.sh`, `scripts/check-brand.sh`, `bun run lint`, `bun run check:translations` saem com 0
Proof: os comandos, em sequência

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| políticas de VAD (3) | Offline C2/C3 · Disabled C2 · Streaming = Offline (door 3 do plano, leitura de `DictationVad`) | - |
| modos do mic (3) | sob demanda C12 · always-on C1 (stream aberto antes do `Start`) · lazy close C1 | always-on no app real: não exercitado no manual |
| fontes de amostra do `stop` (3) | pré-buffer C1 · utterances fechadas C3 · utterance aberta no stop C1 | - |
