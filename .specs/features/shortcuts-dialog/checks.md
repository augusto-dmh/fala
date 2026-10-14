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

## S2 - conflitos (PR 2)

**C7** - tabela do `conflict.rs`: em uso (sem ordem, `ctrl` × `ctrl_left`, outro lugar da mesma ação), Ctrl+C e Win+H reservados, letra sozinha recusada, F9 aceito, Esc só no Cancelar, só modificadores aceitos, sobreposição vira aviso (AC 3)
Proof: `cargo test -p fala --lib conflict`
**C8** - `take_over` tira da outra ação (a 2ª sobe); recusado se Ditar ou Cancelar ficaria sem tecla (AC 4)
Proof: `cargo test -p fala --lib a_combination_in_use_is_refused_or_taken_over`

## S3 - o diálogo (PR 3)

**C9** - Geral mostra uma linha "Atalhos do teclado" com as teclas de Ditar e [Editar atalhos]; as quatro linhas antigas saem
Proof: leitura de `GeneralSettings.tsx`/`KeyboardShortcuts.tsx`
**C10** - cinco linhas na ordem (Cancelar escondida no Linux); "+" só com uma combinação; 🗑 nunca na única de Ditar nem no Cancelar; Esc encerra a gravação, menos no Cancelar
Proof: `bun src/components/settings/shortcuts.test.tsx` (rows, buttons)
**C11** - mensagens na linha: em uso com [Usar aqui] só quando dá, reservada, tecla sozinha, sobreposição como aviso; recusas de restaurar em pt e en
Proof: mesmo arquivo (messages, restore)
**C12** - frontend limpo: `bunx tsc --noEmit`, `bun run lint`, `bun run format:check`, `bun run check:translations`
**C13** - Windows, manual: Ctrl+Shift+Espaço como 2ª de Colar → recusa citando Ditar (sem [Usar aqui]); com F9 como 2ª de Ditar, de novo → [Usar aqui] → Ditar fica com F9; Win+H → recusa; Alt+Shift+M em "Gravar ou parar reunião" → começa e para uma reunião sem abrir a janela; [Restaurar padrão] com reunião gravando → recusa; sem reunião → padrões de volta e o ditado funciona
Proof: roteiro no PR; não rodado pelo agente
