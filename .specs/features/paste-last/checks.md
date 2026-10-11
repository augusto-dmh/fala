# paste-last checks

Profile: light
Plan: `.specs/features/paste-last/plan.md`

11 checks in 2 slices · 2 one-way doors · 0 open, of which 0 block

## Checks

Prefixo de todo `cargo` abaixo (PowerShell, Windows): `$env:CARGO_TARGET_DIR='C:\f\shortcuts-last-text'`.

### S1 - atalhos e escolha do texto

**C1** - um store antigo ganha `paste_last` e `copy_last` sem tecla, mantém as outras teclas, e uma tecla escolhida sobrevive ao próximo merge (AC 1, door 1, door 2)
Proof: `cargo test -p fala --lib settings::tests::old_store_gets_last_dictation_bindings_without_a_key -- --exact`

**C2** - `to_paste`: ocupado → nada, com ou sem histórico; sem histórico → nada; ocioso com item → o texto dele (AC 2, 3)
Proof: `cargo test -p fala --lib last_dictation::tests::paste_is_ignored_while_busy_and_without_history -- --exact`

**C3** - `text_of`: final mostrado, bruto depois de desfazer, bruto quando o final ficou vazio, `post_processed_text`/`transcription_text` sem `dictation`, nada para branco (AC 4)
Proof: `cargo test -p fala --lib last_dictation::tests::text_is_what_the_history_shows -- --exact`

**C4** - `ACTION_MAP` tem `paste_last` (cola no `stop`) e `copy_last` (copia no `start`) (AC 5)
Proof: `grep -n '"paste_last"\|"copy_last"' apps/desktop/src/actions.rs` acha as duas entradas; leitura de `PasteLastAction`/`CopyLastAction`

**C5** - `paste_last` cola por `crate::utils::paste` na thread principal e emite `paste-error` na falha, como o ditado; nenhum log recebe o texto (AC 6)
Proof: leitura de `last_dictation.rs::paste_last`

**C6** - o crate `fala` inteiro verde, clippy e fmt limpos
Proof: `cargo test -p fala --lib`; `cargo clippy -p fala --all-targets -- -D warnings`; `cargo fmt --all -- --check`; `scripts/check-brand.sh`

### S2 - bandeja e Geral

**C7** - menu ocioso: `paste_last_transcript` antes de `copy_last_transcript`; ocupado: só copiar (AC 7)
Proof: `cargo test -p fala --lib tray::tests::paste_last_is_offered_only_while_idle -- --exact`

**C8** - Geral renderiza as duas linhas; nomes e descrições em pt e en (AC 8)
Proof: `grep -n 'shortcutId="paste_last"\|shortcutId="copy_last"' src/components/settings/general/GeneralSettings.tsx`; `bun run check:translations`

**C9** - chip sem tecla mostra `settings.general.shortcut.unset` nos dois componentes de atalho (AC 9)
Proof: `grep -n 'shortcut.unset' src/components/settings/*ShortcutInput.tsx` acha as duas

**C10** - frontend limpo
Proof: `bun run lint`; `bunx prettier --check .`; `bun run build`

**C11** - Windows, manual: atribuir `Alt+Shift+Z` a "Colar o último ditado", ditar no Bloco de Notas, clicar noutro campo, apertar o atalho → o texto cola sem Alt/Shift misturados; repetir no Windows Terminal (cola com Shift+Insert); pela bandeja, "Colar o último ditado" cola no app que estava em foco antes de abrir o menu; durante um ditado o atalho não faz nada (AC 6, 7; manual)
Proof: roteiro no PR; não rodado pelo agente (exige teclado e tela)
