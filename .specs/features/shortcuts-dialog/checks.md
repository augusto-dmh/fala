# shortcuts-dialog checks

Profile: light · Plan: `plan.md` · `cargo` com `CARGO_TARGET_DIR=C:\f\shortcuts-dialog`.

## S1 - segunda combinação, restaurar padrão, atalho de reunião (PR 1)

**C1** - store antigo carrega com 2ª vazia e ganha `meeting_toggle` sem tecla (AC 1)
Proof: `cargo test -p fala --lib old_store_loads_without_second_keys`
**C2** - a 2ª registra como `<id>#2` (Cancelar nunca) e o handler decide como `<id>` (AC 2)
Proof: `cargo test -p fala --lib second_combination`
**C3** - a combinação vai para o lugar pedido; 2ª sem 1ª vira 1ª; Cancelar e Ditar ficam com uma (AC 4)
Proof: `cargo test -p fala --lib shortcut::tests`
**C4** - `reset_all_bindings` recusa com ditado e com reunião gravando/pausada; restaura 1ª = padrão, 2ª vazia (AC 5)
Proof: `cargo test -p fala --lib restoring_defaults`
**C5** - `meeting_toggle`: sem reunião inicia pela bandeja, gravando ou pausada para (AC 6)
Proof: `cargo test -p fala --lib meeting_shortcut`
**C6** - workspace verde: `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, `bunx tsc --noEmit`, `bun run lint`
