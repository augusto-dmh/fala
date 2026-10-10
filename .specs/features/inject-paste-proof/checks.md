# inject-paste-proof checks

Profile: light
Plan: `.specs/features/inject-paste-proof/plan.md`

15 checks in 2 slices · 5 one-way doors · 0 open, of which 0 block

## Checks

Prefixo de todo `cargo` abaixo (PowerShell, Windows): `$env:CARGO_TARGET_DIR='C:\f\inj'`. Os testes da sequência ficam em `crates/inject/src/paste.rs`; o do acorde, em `crates/inject/src/paste/windows.rs`.

### S1 - sequência com prova de leitura

**C1** - acorde ok e texto lido: a sequência devolve `Ok(())` e o registro é `ReadText`(, `ReadImage`), `Offer("ditado")`, `Sleep(delay_before)`, `Chord`, `WaitRead(read_timeout)`, `Sleep(delay_after)`, `Unchanged`, restauração (`WriteText("antes")` / `WriteImage` / `Clear`), `EndOffer`; para os três acordes e os três conteúdos salvos (AC 1, 6)
Proof: `cargo test -p fala-inject paste::tests::read_paste_restores_in_order -- --exact`

**C2** - acorde ok e ninguém lê: devolve `Err(PasteNotRead)` e o registro termina na mesma restauração seguida de `EndOffer`, para os três conteúdos salvos (AC 2, 6, door 1)
Proof: `cargo test -p fala-inject paste::tests::unread_paste_is_reported_and_restored -- --exact`

**C3** - clipboard mudou: lido ou não, nenhum `WriteText`/`WriteImage`/`Clear` depois do acorde, o registro termina em `Unchanged`, `EndOffer`, e o resultado é `Ok` (lido) ou `PasteNotRead` (não lido) (AC 3)
Proof: `cargo test -p fala-inject paste::tests::changed_clipboard_is_not_restored -- --exact`

**C4** - acorde falha: nenhum `WaitRead`, restaura e devolve `Keystroke`, terminando em `EndOffer` (AC 4, 6)
Proof: `cargo test -p fala-inject paste::tests::failed_chord_still_restores -- --exact`

**C5** - oferta falha: devolve `Clipboard`, nenhum `Chord` no registro (AC 5)
Proof: `cargo test -p fala-inject paste::tests::failed_offer_skips_chord -- --exact`

**C6** - `PasteConfig::default()` é CtrlV / 60 / 60 / 100 / 1500 ms (AC 7, door 1)
Proof: `cargo test -p fala-inject paste::tests::default_config_matches_desktop -- --exact`

**C7** - nenhum log contém o texto ditado nem o conteúdo salvo, nos caminhos lido, não lido, clipboard mudou, acorde falho e restauração falha; e o caminho "clipboard mudou" loga em `debug` (AC 3, 8)
Proof: `cargo test -p fala-inject paste::tests::paste_never_logs_text_or_clipboard_content -- --exact`

### S2 - adaptador Windows

**C8** - `ShiftInsert` mapeia para modificador `Shift` e tecla `VK_INSERT` (0x2D); `CtrlV` para `Control` + 0x56; `CtrlShiftV` para `Control`, `Shift` + 0x56 (AC 12)
Proof: `cargo test -p fala-inject paste::windows::tests::chords_map_to_keys -- --exact`

**C9** - Bloco de Notas, Windows Terminal e VS Code: 3 colas cada pelo exemplo, cada uma com `result=ok` e `restored=y`, e o texto visível no destino (print) (AC 9; manual)
Proof: `cargo run -p fala-inject --example paste -- --rounds 3` com o destino focado

**C10** - janela sem campo (a do próprio exemplo, `--blank`): `result=not_read` em ~1,5 s e `restored=y` (AC 10; manual)
Proof: `cargo run -p fala-inject --example paste -- --blank`

**C11** - depois das colas, o histórico do Win+V não tem o marcador do ditado (AC 11; manual: print do Win+V)
Proof: Win+V aberto e capturado depois de C9

**C12** - clippy do workspace sem avisos no Windows (AC 13)
Proof: `cargo clippy --workspace --all-targets -- -D warnings`

**C13** - `cargo fmt --all -- --check`, `scripts/check-no-tauri-in-crates.sh` e `scripts/check-brand.sh` saem com 0
Proof: os três comandos

**C14** - `cargo test -p fala-inject` inteiro verde (o contrato do #57 continua: texto e imagem restaurados)
Proof: `cargo test -p fala-inject`

**C15** - a oferta no clipboard real do Windows, sem janela visível: não conta leitura sem leitor; uma leitura pelo `arboard` entrega o texto e conta como lida sem parecer escrita alheia; uma cópia alheia é detectada; os três formatos de exclusão estão no clipboard; a oferta não lida é entregue quando a janela fecha (AC 1, 3, 11; door 3, 4). Os testes são `#[ignore]` porque sobrescrevem o clipboard de quem roda
Proof: `cargo test -p fala-inject offer -- --ignored`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| resultados de `insert` (4) | `Ok` C1 · `PasteNotRead` C2 · `Keystroke` C4 · `Clipboard` C5 | - |
| conteúdo salvo (3) | texto, imagem, nada: C1, C2, C4 table-driven | - |
| `PasteChord` (3) | C1 (sequência), C8 (teclas) | `CtrlShiftV` e `ShiftInsert` sem prova manual em app real |
| destinos manuais (4) | Notepad, Terminal, VS Code C9 · sem campo C10 | - |
| one-way doors (5) | prazo C2/C6 · lido-não-colado: documentado, sem prova · janela por cola C9/C10 · exclusão C11 · dependências C12 | door 2 não tem prova automática por natureza |

- O teste `non_windows_platform_injector_is_unsupported` é `cfg(not(windows))` e não roda nesta máquina; o CI Linux o roda

## Swept

- failure modes: C2, C3, C4, C5
- privacy: C7, C11
