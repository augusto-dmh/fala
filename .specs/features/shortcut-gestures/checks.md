# shortcut-gestures checks

Profile: light
Plan: `.specs/features/shortcut-gestures/plan.md`

23 checks in 4 slices · 2 one-way doors · 0 open, of which 0 block

Comandos de cargo com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`,
rodados na raiz do worktree. Os testes do coordenador usam relógio sintético (`Instant` injetado
em `on_input`/`on_deadline`), como os testes herdados do mesmo módulo.

## Checks

### S1 - o gesto no coordenador · 1 file · 61 KB · ~15k

**C1** - No modo `push_to_talk_double_tap`, o key-down com o coordenador ocioso emite `Start` no próprio key-down e deixa o estágio em `Recording` (AC 1)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::double_tap_press_starts_recording_on_key_down`

**C2** - Um key-up 300 ms depois do key-down não emite nada na hora e emite `Stop` quando a carência expira; estágio `Processing` (AC 2, borda de 300 ms)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::double_tap_long_hold_stops_after_release_grace`

**C3** - Um key-up 299 ms depois do key-down não emite nada nem na carência; o estágio segue `Recording`, a sessão não fica travada e `next_deadline()` vale key-down + 500 ms (AC 3, borda de 299 ms)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::double_tap_short_tap_keeps_recording_and_arms_window`

**C4** - Depois de um toque de 100 ms, um segundo key-down a 499 ms e a 500 ms do primeiro key-down trava a sessão (`is_locked()` verdadeiro), não emite efeito e limpa o prazo da janela (AC 4)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::double_tap_second_press_inside_window_locks`

**C5** - Com a sessão travada por dois toques, um key-up não emite nada e `grace_deadline()` segue `None` (AC 5)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::double_tap_locked_session_ignores_release`

**C6** - Com a sessão travada, o key-down seguinte emite `Stop` e o estágio vai a `Processing`; o key-up desse toque, já em `Processing`, não deixa nada para o drain (AC 6)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::double_tap_press_on_locked_session_stops`

**C7** - Um toque de 100 ms sem segundo key-down: `on_deadline(t0 + 500 ms)` emite `Discard` com o binding da gravação, o estágio volta a `Idle` e nenhum `Stop` é emitido na sequência inteira (AC 7)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::double_tap_window_expiry_discards`

**C8** - Um segundo key-down a 501 ms do primeiro, antes do prazo da janela ser processado, emite `Discard` (não trava) e deixa o estágio `Idle` (AC 8, borda de 501 ms)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::double_tap_late_second_press_discards`

**C9** - Com a janela aberta, um key-up avulso não emite nada, não adia release (`grace_deadline()` `None`) e a janela continua armada em key-down + 500 ms (AC 9)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::double_tap_release_inside_window_is_ignored`

**C10** - Com o pipeline em `Processing`, um toque de 40 ms (press + release + carência) não deixa nada lembrado: `on_processing_finished()` devolve `None` e o estágio fica `Idle` (AC 10)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::double_tap_tap_during_processing_nets_noop`

**C11** - Segurar ~600 ms sob auto-repeat do X11 (pares release/press a 5 ms) produz exatamente 1 `Start` e 1 `Stop`, sem travar nem abrir janela no meio (AC 11)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::double_tap_autorepeat_hold_is_one_hold`

**C12** - `on_cancel(true)` com a janela aberta e com a sessão travada leva o estágio a `Idle`, limpa trava e prazo, e o key-down seguinte emite `Start` (AC 12, dois casos)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::double_tap_cancel_clears_window_and_locked_session`

**C13** - Com carência e janela pendentes ao mesmo tempo, `next_deadline()` devolve a menor das duas, nas duas ordens; com só uma, devolve essa; sem nenhuma, `None` (AC 13)
Proof: `cargo test -p fala --lib transcription_coordinator::tests::next_deadline_is_the_earliest_pending`

**C14** - Os testes herdados do coordenador (toggle, push-to-talk, hold-or-toggle, fila durante o processamento, #1539) continuam verdes sem edição (regressão dos modos antigos, door 1)
Proof: `cargo test -p fala --lib transcription_coordinator`

### S2 - o descarte não deixa rastro · 2 files · 68 KB · ~2k (só `run_effect` e `utils.rs`)

**C15** - `run_effect` encaminha `Effect::Discard` para `utils::abort_current_operation`, e o corpo dessa função não chama `notify_cancel` (AC 15)
Proof: `grep -n "Effect::Discard" -A3 apps/desktop/src/transcription_coordinator.rs | grep -q "abort_current_operation"`
Proof: `! awk '/^pub fn abort_current_operation/,/^}/' apps/desktop/src/utils.rs | grep -q notify_cancel`

**C16** - `cancel_current_operation` executa o mesmo teardown chamando `abort_current_operation` e só então notifica o coordenador, de modo que Esc e o botão X da pill mantêm o comportamento de antes (AC 14, sem regressão do cancel)
Proof: `awk '/^pub fn cancel_current_operation/,/^}/' apps/desktop/src/utils.rs | grep -q "abort_current_operation(app)"`
Proof: `awk '/^pub fn cancel_current_operation/,/^}/' apps/desktop/src/utils.rs | grep -q "notify_cancel"`

**C17** - TODO(windows): no build do Windows, com `audio_feedback` ligado e o store novo, um toque curto isolado em `Ctrl+Shift+Space` mostra e esconde a pill, não toca som de fim, não cola nada no Bloco de Notas em foco e não cria entrada no histórico; Esc durante uma gravação travada por dois toques faz o mesmo (AC 14, AC 12 no hook real)
Proof: `TODO(windows)` manual - 10 toques isolados, 10 dois-toques seguidos de fala e toque de parada, 5 dois-toques cancelados por Esc; contar colagens e entradas no histórico (esperado 0, 10, 0)

### S3 - padrões de instalação nova · 1 file · 63 KB · ~16k

**C18** - `get_default_settings().shortcut_activation` e `serde_json::from_value::<AppSettings>(json!({})).shortcut_activation` valem `ShortcutActivation::PushToTalkDoubleTap` (AC 16, os dois lugares do padrão)
Proof: `cargo test -p fala --lib settings::tests::default_shortcut_activation_is_push_to_talk_double_tap`

**C19** - `PushToTalkDoubleTap` serializa para a string `"push_to_talk_double_tap"` e essa string desserializa para a mesma variante (AC 17)
Proof: `cargo test -p fala --lib settings::tests::push_to_talk_double_tap_round_trips_through_serde`

**C20** - Em Windows e Linux, os padrões têm `transcribe` = `"ctrl+shift+space"` e `transcribe_with_post_process` = `"ctrl+space"`, em `default_binding` e em `current_binding` (AC 18; o teste é `#[cfg(any(target_os = "windows", target_os = "linux"))]`, roda aqui no Linux)
Proof: `cargo test -p fala --lib settings::tests::default_bindings_put_dictation_on_ctrl_shift_space`

**C21** - Um store com `settings_schema_version: 2` e `shortcut_activation` em `"hold_or_toggle"`, `"push_to_talk"` ou `"toggle"` desserializa estrito e, depois de `apply_settings_migrations`, mantém exatamente o valor guardado (AC 19, tabela com os 3)
Proof: `cargo test -p fala --lib settings::tests::stored_activation_modes_load_unchanged`

### S4 - o seletor do modo · 3 files · 64 KB · ~4k (só os trechos tocados)

**C22** - O primeiro `value:` das opções em `ShortcutActivation.tsx` é `"push_to_talk_double_tap"`, o fallback do `selected` é `"push_to_talk_double_tap"`, e `settings.general.shortcutActivation.options.pushToTalkDoubleTap` vale "Segurar ou dois toques" em pt e "Hold or double-tap" em en, com `descriptions.push_to_talk_double_tap` não vazio nos dois (AC 20, AC 21)
Proof: `grep -m1 'value: "' src/components/settings/ShortcutActivation.tsx | grep -q '"push_to_talk_double_tap"'`
Proof: `grep -q '"push_to_talk_double_tap") as ShortcutActivation' src/components/settings/ShortcutActivation.tsx`
Proof: `python3 -c 'import json,sys; g=lambda l: json.load(open(f"src/i18n/locales/{l}/translation.json"))["settings"]["general"]["shortcutActivation"]; p,e=g("pt"),g("en"); sys.exit(0 if p["options"]["pushToTalkDoubleTap"]=="Segurar ou dois toques" and e["options"]["pushToTalkDoubleTap"]=="Hold or double-tap" and p["descriptions"]["push_to_talk_double_tap"].strip() and e["descriptions"]["push_to_talk_double_tap"].strip() else 1)'`
Proof: `bun run check:translations`
Proof: `bun run lint`
Proof: `bunx tsc --noEmit`

**C23** - `src/bindings.ts`, regenerado pelo `tauri-specta` (export de `lib.rs` em build de debug) e não editado à mão, inclui `"push_to_talk_double_tap"` no tipo `ShortcutActivation` (door 1, Impact `generated`)
Proof: `awk '/^export type ShortcutActivation/,/^export type ShortcutBinding/' src/bindings.ts | grep -q '"push_to_talk_double_tap"'`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| transições do modo novo (12) | Idle→Recording por key-down C1 · segurando→Processing por key-up ≥ 300 ms C2 · segurando→janela por key-up < 300 ms C3 · janela→travada por key-down ≤ 500 ms C4 · janela→Idle por prazo C7 · janela→Idle por key-down tardio C8 · janela + key-up avulso C9 · travada + key-up C5 · travada→Processing por key-down C6 · janela→Idle por cancel C12 · travada→Idle por cancel C12 · Processing + toque→drain vazio C10 | - |
| borda do segundo toque (3) | 499 ms C4 · 500 ms C4 · 501 ms C8 | - |
| borda do segurar (2) | 299 ms C3 · 300 ms C2 | - |
| efeitos do coordenador no modo novo (3) | `Start` C1 · `Stop` C2, C6 · `Discard` C7, C8 | - |
| prazos do laço (2) | carência C13 · janela C13, C3 | - |
| modos guardados que não migram (3) | `hold_or_toggle` C21 · `push_to_talk` C21 · `toggle` C21, table-driven | - |
| bindings padrão (2) | `transcribe` C20 · `transcribe_with_post_process` C20 | - |
| startup config: origem do padrão do modo (2 lugares) | `get_default_settings` C18 · `#[serde(default)]` com a chave ausente C18 | - |
| caminhos de teardown (2) | `Discard` C15 · cancel (Esc, botão X, tray) C16 | - |
| locales (2) | pt C22 · en C22 | - |
| door 1 (valor do enum) (4) | serde C19 · padrão C18 · modos antigos intactos C14, C21 · tipo TS gerado C23 | - |
| door 2 (atalhos padrão) (1) | C20 | - |

- Claims naming a value at a boundary: C2, C3, C4, C8 - each asserts both sides of its edge
- C17 is the only check not settled on Linux; it is `TODO(windows)` and stays unchecked
- Duas asserções herdadas fixavam o padrão antigo `HoldOrToggle` (`empty_store_parses_with_defaults` e `shortcut_activation_defaults_to_hold_or_toggle_without_legacy_key`). O AC 16 muda esse padrão de propósito, então as duas passam a afirmar `PushToTalkDoubleTap` e a segunda ganha o nome do padrão novo. Nenhuma outra asserção herdada muda

## Swept

- validation: C21 (valores guardados aceitos sem migrar); `hold_threshold_ms` não ganha limite novo - campo herdado, ajustável só na aba Debug
- failure modes: C8 (segundo toque atrasado vira descarte, não trava); C15 (o descarte não reentra no coordenador)
- idempotency: C10 (toques durante o processamento se anulam no drain); C12 (cancel repetido deixa `Idle`)
- authorization: n/a - comando local da própria janela, sem chamador externo
- concurrency: C15 (sem `Cancel` de volta ao coordenador, um key-down enfileirado durante o teardown não é zerado por um cancel atrasado); C13 (um único laço, acordado pelo prazo mais próximo)
- data lifecycle: C21 (stores existentes não são reescritos); o descarte não grava WAV nem histórico, C17
- dependency failure: existing - `on_start_result` herdado desfaz o `Start` se o microfone falhar, coberto por `failed_start_rolls_back_to_idle` (C14)
- state transitions: C1-C12, conjunto "transições do modo novo"
- observability: n/a - só linhas `debug!` sem conteúdo ditado (AGENTS.md), sem métrica nem trace nesta feature; o trace por etapa é da trilha A

## Handoff

- S1 = 15k, S2 = 2k, S3 = 16k, S4 = 4k (+12k para ler o `bindings.ts` gerado) = ~49k, todos em `apps/desktop` e no front, abaixo do budget de 150k - one builder
- **Settled mid-build:** o helper herdado `drive` (testes de #1539) faz `match` exaustivo sobre `Option<Effect>`; com a variante `Discard` ele não compilava. Ganhou um braço `Some(Effect::Discard { .. }) => panic!("push-to-talk never discards")`. Nenhuma asserção herdada mudou e o braço só torna o helper mais estrito; C14 deve ser lido como "nenhuma asserção herdada editada"
