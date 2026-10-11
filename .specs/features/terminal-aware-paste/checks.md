# terminal-aware-paste checks

Profile: light
Plan: `.specs/features/terminal-aware-paste/plan.md`

10 checks in 2 slices · 3 one-way doors · 0 open, of which 0 block

## Checks

Prefixo de todo `cargo` abaixo (PowerShell, Windows): `$env:CARGO_TARGET_DIR='C:\f\pipe'`. Os testes do crate ficam em `crates/inject/src/terminal.rs`; os do desktop, em `apps/desktop/src/clipboard.rs`.

### S1 - acorde por app no crate

**C1** - terminais conhecidos, com e sem caixa, dão `true`; navegador, editor, apps de IA, vazio e só espaços dão `false` (AC 1, 2, door 1)
Proof: `cargo test -p fala-inject terminal::tests::terminals_by_app_name -- --exact`

**C2** - `PasteChord::for_app`: terminal → `ShiftInsert`; outro app e `app_name: None` → `CtrlV` (AC 3, door 3)
Proof: `cargo test -p fala-inject terminal::tests::chord_for_app -- --exact`

**C3** - toda entrada de `TERMINALS` está em minúsculas, sem `.exe`, sem espaços e sem repetição (AC 4)
Proof: `cargo test -p fala-inject terminal::tests::list_is_normalized -- --exact`

**C4** - o crate inteiro continua verde
Proof: `cargo test -p fala-inject`

### S2 - o desktop cola com o acorde efetivo

**C5** - `effective_paste_method(CtrlV, terminal)` é `ShiftInsert`; `(CtrlV, notepad)` e `(CtrlV, desconhecido)` são `CtrlV` (AC 5, 6)
Proof: `cargo test -p fala --lib clipboard::tests::ctrl_v_becomes_shift_insert_in_terminals -- --exact`

**C6** - os cinco outros `PasteMethod` ficam iguais num terminal (AC 7, door 2)
Proof: `cargo test -p fala --lib clipboard::tests::explicit_paste_method_is_kept -- --exact`

**C7** - a troca loga em `debug` com o nome do app e nunca com o texto (AC 9)
Proof: leitura de `clipboard.rs::paste`: o `debug!` fica antes do `match`, recebe só `app_name`, e nenhuma linha de log da função recebe `text`

**C8** - exemplo com `--chord auto`: Windows Terminal em foco imprime `chord=shift_insert result=ok` e o texto aparece no terminal; Bloco de Notas em foco imprime `chord=ctrl_v result=ok` (AC 8; manual)
Proof: `cargo run -p fala-inject --example paste -- --chord auto` com cada destino focado

**C9** - clippy e fmt limpos nos dois pacotes tocados, e os scripts do CI
Proof: `cargo clippy -p fala-inject -p fala --all-targets -- -D warnings`; `cargo fmt --all -- --check`; `scripts/check-no-tauri-in-crates.sh`; `scripts/check-brand.sh`

**C10** - o Code Map do `ARCHITECTURE.md` diz que o `inject` escolhe o acorde por app
Proof: `grep -n "acorde por app" ARCHITECTURE.md` acha a linha do `crates/inject`
