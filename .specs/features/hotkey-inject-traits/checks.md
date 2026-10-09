# hotkey-inject-traits checks

Profile: light
Plan: `.specs/features/hotkey-inject-traits/plan.md`

23 checks em 5 slices · 5 one-way doors · 0 open, 0 block. C8 e C18 são `TODO(windows)`: ficam `Unproven` e mantêm o veredito FAIL até a rodada manual no Windows (Assumptions do plano).

Prefixo de todo `cargo` abaixo: `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`. O prefixo da prova Windows cruzada é `RUSTC_BOOTSTRAP=1` com `-Zbuild-std=std,panic_abort --target x86_64-pc-windows-msvc`. Os testes `#[cfg(windows)]` só rodam no job `windows` do CI do PR; a prova deles é o log do job (`gh run view <run> --log --job <windows-job>`), em que cada nome aparece com `... ok`.

## Checks

### S1 - contrato do atalho, neutro e Unsupported no Linux · 3 files · 8 KB · ~2k

**C1** - fora do Windows, `platform_hotkey(tx)` devolve `Err(HotkeyError::Unsupported(motivo))` com `motivo.trim()` não vazio, e `rx.try_recv()` não devolve nenhum evento (AC 1, door 5)
Proof: `cargo test -p fala-hotkey tests::non_windows_platform_hotkey_is_unsupported -- --exact`

**C2** - com `transcribe` = `ctrl+shift+space` registrado sob o id 7, os eventos (7, `Pressed`) e (7, `Released`) produzem exatamente `[HotkeyEvent { binding_id: "transcribe", accelerator: "ctrl+shift+space", state: Pressed }, HotkeyEvent { .., state: Released }]`, nessa ordem (AC 2, door 1)
Proof: `cargo test -p fala-hotkey bindings::tests::events_route_to_binding_in_order -- --exact`

**C3** - um evento com id 9, nunca registrado, e um evento com id 7 depois de `unregister("transcribe")` produzem `None` os dois (AC 3)
Proof: `cargo test -p fala-hotkey bindings::tests::unknown_or_unregistered_id_sends_nothing -- --exact`

**C4** - registrar de novo `transcribe` (com `f9`) devolve `Err(HotkeyError::AlreadyRegistered("transcribe"))`, não chama o backend, e o evento do id 7 continua saindo com `accelerator: "ctrl+shift+space"` (AC 4)
Proof: `cargo test -p fala-hotkey bindings::tests::duplicate_binding_is_rejected_and_keeps_first -- --exact`

**C5** - `unregister("nunca")` devolve `Ok(())` sem chamar o backend (AC 5)
Proof: `cargo test -p fala-hotkey bindings::tests::unregister_unknown_binding_is_ok -- --exact`

### S2 - atalho no Windows sobre o `handy-keys` · 2 files · 30 KB · ~8k

**C6** - no Windows, `register` com `""`, `ctrl+banana` e `ctrl+a+b` devolve `Err(HotkeyError::InvalidAccelerator { accelerator, .. })` com o acelerador recebido, e o parser aceita `ctrl+shift+space`, `f9` e `ctrl+shift` (AC 6, door 2)
Proof: `windows::tests::invalid_accelerators_are_rejected` com `... ok` no log do job `windows` do CI do PR
Proof: `windows::tests::desktop_accelerators_parse` com `... ok` no log do job `windows` do CI do PR

**C7** - no Windows, `HotkeyState::Pressed` vira `KeyState::Pressed` e `HotkeyState::Released` vira `KeyState::Released` (AC 7)
Proof: `windows::tests::hook_states_map_to_key_state` com `... ok` no log do job `windows` do CI do PR

**C8** - no Windows, `hotkey_listen -- f9 60` com 60 toques físicos de F9 imprime `pressed=60 released=60` e sai com 0 (AC 8; `TODO(windows)`: manual, não roda no Linux)
Proof: `cargo run -p fala-hotkey --example hotkey_listen -- f9 60` no Windows 11, exit 0

**C9** - no Windows, `register` depois de a thread do manager ter acabado (canal de comandos desconectado) devolve `Err(HotkeyError::Backend(_))` (door 1: a variante `Backend`)
Proof: `windows::tests::register_after_thread_gone_is_backend_error` com `... ok` no log do job `windows` do CI do PR

### S3 - sequência de colagem com restore, neutra e Unsupported no Linux · 3 files · 14 KB · ~4k

**C10** - fora do Windows, `platform_injector(PasteConfig::default())` devolve `Err(InjectError::Unsupported(motivo))` com `motivo.trim()` não vazio (AC 9, door 5)
Proof: `cargo test -p fala-inject paste::tests::non_windows_platform_injector_is_unsupported -- --exact`

**C11** - `PasteConfig::default()` é `PasteConfig { chord: PasteChord::CtrlV, delay_before: 60 ms, delay_after: 60 ms, modifier_hold: 100 ms }` (AC 10, door 3)
Proof: `cargo test -p fala-inject paste::tests::default_config_matches_desktop -- --exact`

**C12** - com o clipboard em `antes`, colar `ditado` registra exatamente `[ReadText, WriteText("ditado"), Sleep(delay_before), Chord(chord, modifier_hold), Sleep(delay_after), WriteText("antes")]` e devolve `Ok(())`, para `CtrlV` e para `CtrlShiftV` (AC 11, door 3)
Proof: `cargo test -p fala-inject paste::tests::text_is_restored_after_paste_in_order -- --exact`

**C13** - sem texto (leitura falha) e com texto vazio, com uma imagem 2×1 no clipboard, a sequência lê a imagem antes de `WriteText("ditado")` e termina com `WriteImage` da mesma imagem (largura, altura e bytes iguais), sem `Clear` (AC 12)
Proof: `cargo test -p fala-inject paste::tests::image_is_restored_when_there_is_no_text -- --exact`

**C14** - sem texto nem imagem (as duas leituras falham), a sequência manda o acorde e termina com `Clear` (AC 13)
Proof: `cargo test -p fala-inject paste::tests::empty_clipboard_is_cleared -- --exact`

**C15** - com o acorde falhando, a sequência termina com `WriteText("antes")`, `WriteImage(imagem)` ou `Clear` (um caso para cada conteúdo salvo) e devolve `Err(InjectError::Keystroke(_))` (AC 14)
Proof: `cargo test -p fala-inject paste::tests::failed_chord_still_restores -- --exact`

**C16** - com a escrita de `ditado` falhando, a sequência devolve `Err(InjectError::Clipboard(_))` e o registro não contém nenhum `Chord` (AC 15)
Proof: `cargo test -p fala-inject paste::tests::failed_write_skips_chord -- --exact`

**C17** - com um logger de teste em `Trace` capturando tudo, colar o marcador `FALA-DITADO-7f3a` sobre o marcador salvo `FALA-SALVO-9c1e`, com acorde ok e com acorde falho, não deixa nenhum registro de log contendo um dos dois marcadores; e o adaptador Windows de colagem não tem chamada de log (AC 16)
Proof: `cargo test -p fala-inject paste::tests::paste_never_logs_text_or_clipboard_content -- --exact`
Proof: `test -f crates/inject/src/paste/windows.rs && ! grep -nE '(log::|\b(trace|debug|info|warn|error)!)' crates/inject/src/paste/windows.rs`

### S4 - colagem no Windows sobre `arboard` e `enigo` · 2 files · 8 KB · ~2k

**C18** - no Windows, com o Bloco de Notas em foco, `paste -- "teste um dois três"` cola o texto duas vezes (clipboard com um texto marcador, depois com uma imagem 8×8) e sai com 0 só se o clipboard voltou ao marcador e à imagem (mesmo tamanho e bytes) (AC 17; `TODO(windows)`: manual, não roda no Linux)
Proof: `cargo run -p fala-inject --example paste -- "teste um dois três"` no Windows 11, exit 0, e o texto visível duas vezes no Bloco de Notas

### S5 - plataforma, dependências e documentos · 4 files · 70 KB · ~18k

**C19** - a árvore normal de `fala-hotkey` e `fala-inject` para `x86_64-unknown-linux-gnu` não contém `handy-keys`, `arboard`, `enigo`, `windows`, `windows-sys` nem `tauri`; a de `x86_64-pc-windows-msvc` contém `handy-keys v0.3.4` (em `fala-hotkey`) e `arboard v3.6.1` e `enigo v0.6.1` (em `fala-inject`); `scripts/check-no-tauri-in-crates.sh` sai com 0 (AC 18, door 4)
Proof: `scripts/check-no-tauri-in-crates.sh && ! cargo tree -p fala-hotkey -p fala-inject -e normal --target x86_64-unknown-linux-gnu | grep -E 'handy-keys|arboard|enigo|windows|tauri' && cargo tree -p fala-hotkey -e normal --target x86_64-pc-windows-msvc | grep -q 'handy-keys v0.3.4' && cargo tree -p fala-inject -e normal --target x86_64-pc-windows-msvc | grep -q 'arboard v3.6.1' && cargo tree -p fala-inject -e normal --target x86_64-pc-windows-msvc | grep -q 'enigo v0.6.1'`

**C20** - `cargo clippy --all-targets -- -D warnings` para `x86_64-pc-windows-msvc` sai com 0 em `fala-hotkey` e em `fala-inject`, compilando o código `cfg(windows)`, os testes e os exemplos; e o job `windows` do CI do PR termina com `success` (AC 19)
Proof: `RUSTC_BOOTSTRAP=1 cargo clippy -Zbuild-std=std,panic_abort --target x86_64-pc-windows-msvc -p fala-hotkey -p fala-inject --all-targets -- -D warnings`
Proof: `gh pr checks <pr>` mostra o job `windows` com `pass`

**C21** - no Linux, `cargo clippy --workspace --all-targets -- -D warnings` sai com 0 e o job `rust` do CI do PR (que roda `cargo test --workspace`) termina com `success` (AC 20)
Proof: `cargo clippy --workspace --all-targets -- -D warnings`
Proof: `gh pr checks <pr>` mostra o job `rust` com `pass`

**C22** - o diff `origin/main...HEAD` (a partir da merge-base) não toca `apps/desktop/` nem `src/`, e nenhum `.rs` ou `.toml` fora de `crates/hotkey` e `crates/inject` ganha `cfg(windows)`, `cfg(not(windows))` ou `cfg(target_os` (AC 21)
Proof: `git diff --quiet origin/main...HEAD -- apps/desktop src && ! git diff origin/main...HEAD -- '*.rs' '*.toml' ':!crates/hotkey' ':!crates/inject' | grep -E '^\+.*cfg\((windows|not\(windows\)|target_os)'`

**C23** - `crates/hotkey/src/lib.rs` cita `handy-keys` e não cita `rdev`; `crates/inject/src/lib.rs` contém a frase `Se o acorde falha, o clipboard volta ao conteúdo de antes`; o `ARCHITECTURE.md` contém, no parágrafo "Estado", `` `hotkey` e `inject` têm os traits e os adaptadores Windows `` e `` o desktop ainda usa `shortcut/` e `clipboard.rs` ``, e as linhas do Code Map de `crates/hotkey` e `crates/inject` citam `GlobalHotkey` e `Injector` (AC 22)
Proof: `grep -q 'handy-keys' crates/hotkey/src/lib.rs && ! grep -q 'rdev' crates/hotkey/src/lib.rs && grep -q 'Se o acorde falha, o clipboard volta ao conteúdo de antes' crates/inject/src/lib.rs && grep -q '`hotkey` e `inject` têm os traits e os adaptadores Windows' ARCHITECTURE.md && grep -q 'o desktop ainda usa `shortcut/` e `clipboard.rs`' ARCHITECTURE.md && grep -E '^\| `crates/hotkey` ' ARCHITECTURE.md | grep -q 'GlobalHotkey' && grep -E '^\| `crates/inject` ' ARCHITECTURE.md | grep -q 'Injector'`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| `KeyState` (2) | `Pressed` C2, C7 · `Released` C2, C7 | - |
| `HotkeyError` variants, door 1 (4) | `Unsupported` C1 · `InvalidAccelerator` C6 · `AlreadyRegistered` C4 · `Backend` C9 | - |
| tabela de bindings: estado do id no evento (3) | registrado C2 · nunca registrado C3 · desregistrado C3 | - |
| aceleradores do C6, door 2 (6) | `""` C6 · `ctrl+banana` C6 · `ctrl+a+b` C6 · `ctrl+shift+space` C6 · `f9` C6 · `ctrl+shift` C6 | - |
| `InjectError` variants que a inserção produz, door 3 (3) | `Unsupported` C10 · `Clipboard` C16 · `Keystroke` C15 | - |
| `PasteChord`, door 3 (2) | `CtrlV` C12 · `CtrlShiftV` C12 | - |
| `PasteConfig::default()` fields (4) | `chord` C11 · `delay_before` C11 · `delay_after` C11 · `modifier_hold` C11 | - |
| conteúdo salvo do clipboard, acorde ok (5) | texto C12 · texto vazio + imagem C13 · sem texto + imagem C13 · nada C14 · leituras falham C14 | - |
| conteúdo salvo do clipboard, acorde falho (3) | texto C15 · imagem C15 · nada C15 | - |
| alvos de dependência, door 4 (2) | `x86_64-unknown-linux-gnu` C19 · `x86_64-pc-windows-msvc` C19 | - |
| adaptadores Windows em hardware real (2) | atalho C8 · colagem C18 | - |
| documentos (3 lugares) | `crates/hotkey/src/lib.rs` C23 · `crates/inject/src/lib.rs` C23 · `ARCHITECTURE.md` C23 | - |

- C8 e C18 têm prova nomeada, mas manual no Windows (`TODO(windows)`): o Verifier no Linux os registra `Unproven`, e o veredito fica FAIL até a rodada manual
- C20 e C21 afirmam build e lint, não comportamento: a prova é o comando inteiro, de propósito
- Nenhum check afirma uma rota, status ou forma de resposta: a Surface do plano é `None`
- C12 é table-driven sobre os dois acordes; C15 sobre os três conteúdos salvos; C13 sobre texto ausente e texto vazio; os outros checks não afirmam mais do que o caso que a prova exercita

## Swept

- validation: C6
- failure modes: C14, C15, C16, C9
- idempotency: C4, C5
- authorization: n/a - biblioteca sem identidade de chamador; a barreira de integridade do Windows (UIPI) sobre o `SendInput` é a mesma do desktop e só aparece em hardware real (C18)
- concurrency: existing - uma thread só é dona do `HotkeyManager` e os comandos chegam serializados por canal, o modelo de `apps/desktop/src/shortcut/fala_keys.rs` copiado em `crates/hotkey/src/windows.rs`; exercitado em hardware por C8
- data lifecycle: n/a - nada persiste; o conteúdo salvo do clipboard vive só na pilha de uma colagem e volta ao clipboard (C12)
- dependency failure: C16, C14, C9
- state transitions: C2, C3
- observability: C17

## Handoff

- Leitura de referência: `fala_keys.rs` 21 KB, `clipboard.rs` 40 KB (só ~5 KB lidos), `input.rs` 9 KB, `foreground.rs` 8 KB, crates atuais 2 KB; escrita estimada: `fala-hotkey` ~14 KB, `fala-inject` ~16 KB, exemplos ~5 KB, docs ~3 KB. S1-S5 = (21+5+9+8+2+14+16+5+3) KB = 83 KB / 4 ≈ 21k tokens de arquivo, mais as saídas de cargo (~20k) ≈ 41k, abaixo do budget de 150k - one builder
- Mechanism: one builder (cabe no budget)
- **Boundary:** C1-C5 fechados pelo commit que adiciona esta linha (`cargo test -p fala-hotkey`: 5 passed); C6, C7 e C9 compilam no cross-clippy msvc e fecham com o job `windows` do PR; C8 fica `TODO(windows)`
- **Boundary:** C10-C17, C19 e C23 fechados pelo commit que adiciona esta linha (`cargo test -p fala-inject`: 13 passed; provas de grep e `cargo tree` com exit 0); C20 local (cross-clippy msvc dos dois crates) com exit 0; C18 fica `TODO(windows)`; C20 (job `windows`), C21 e C22 fecham no PR
- **Settled mid-build:** o teste de C7 virou `hook_states_map_to_key_state` (era `handy_states_...`), porque `scripts/check-brand.sh` barra "handy" fora de `handy-keys`/`handy_keys::`; a afirmação e a asserção não mudaram
- **Settled mid-build (round 1 do Verifier):** C22 passou a comparar a partir da merge-base (`origin/main...HEAD`), porque o `origin/main` andou durante a verificação e o diff de dois pontos mostrava os commits novos do `main` como se fossem desta branch; a afirmação não mudou. A branch foi rebaseada em `616af4f` (conflito só no parágrafo "Estado" do `ARCHITECTURE.md`, resolvido mantendo o texto do `main`). Quatro defeitos menores apontados no round 1 foram corrigidos: `unregister` só tira o id depois de o hook aceitar, `thread::Builder` no lugar de `thread::spawn`, o teste de C6 não pode mais travar, e o acorde solta os modificadores já pressionados quando uma tecla falha
