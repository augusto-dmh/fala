# pill-menu checks

Profile: light
Plan: `.specs/features/pill-menu/plan.md`

5 checks in 1 slice · 0 one-way doors · 0 open

Cargo com `CARGO_TARGET_DIR=C:\f\pill-menu-drag`, na raiz do worktree. As provas estruturais leem
só código de produção (antes de `mod tests`) e falham quando o alvo some.

## Checks

**C1** - `PILL_MENU` tem os itens e separadores na ordem do AC 1; cada id que o menu acrescenta tem braço no handler da bandeja em `lib.rs` (`paste_last_transcript`, `PILL_HIDE_HOUR_ID` → `snooze_idle`, `PILL_SETTINGS_ID`, idioma e microfone por prefixo); `hide_pill_hour` sai das locales pt e en (AC 1, 3)
Proof: `cargo test -p fala --lib tray::tests::pill_menu_items_and_their_handlers`

**C2** - `open_pill_menu` só age com `Shown::Idle | Shown::IdleHover`, chama `remember_foreground` antes de `popup_menu` e `return_focus_from` depois, e fecha o cartão do hover se o cursor saiu da pílula durante o menu; `return_focus_from` só age se a pílula é a janela em primeiro plano (AC 2, 4)
Proof: `cargo test -p fala --lib overlay::tests::snooze_hides_only_the_idle_pill_for_an_hour`

**C3** - `IDLE_SNOOZE` é 1 h; `is_snoozed` é verdadeiro só antes do fim; o timer único (`SNOOZE_TIMER_RUNNING`) dorme até o fim, acordando ao menos a cada minuto (`snooze_wait`), e então chama `refresh_idle`; `current_rest` trata a pílula adiada como não querida, então a reunião mantém seu estado e o repouso esconde (`idle_refresh` → `Hide`); `snooze_idle` chama `refresh_idle` agora e depois do timer; o toggle chama `end_idle_snooze` antes de `apply_idle_setting` (AC 5)
Proof: `cargo test -p fala --lib overlay::tests::snooze_hides_only_the_idle_pill_for_an_hour`

**C4** - o ramo `idle` de `RecordingOverlay` impede o menu do webview e chama `openPillMenu`; nenhum outro estado tem `onContextMenu`; `App.tsx` ouve `open-settings` e vai a `settings`, na vista geral (AC 2, 3)
Proof: `bun src/overlay/pill.test.tsx` (linha `pill-menu C4 ok`)

**C5** - Portões: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p fala --lib`, `bun run lint`, `bun run format:check`, `bunx tsc --noEmit`, `bun run check:translations`, `scripts/check-brand.sh`; o resto de `pill.test.tsx` e `notice.test.tsx` segue verde
Proof: os comandos acima, exit 0

## Manual on Windows (fora da tabela)

M1 clique direito na pílula em repouso abre o menu no cursor; Esc fecha e o foco volta ao app
anterior · M2 "Colar o último ditado" cola no app que tinha o foco · M3 trocar o microfone pelo
submenu muda o de Configurações · M4 "Ocultar por 1 hora" some com a pílula; um ditado ainda mostra
a pílula de gravação e, ao acabar, nada fica na tela · M5 "Configurações" abre Configurações.

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| itens do menu (5) | colar · idioma · microfone · ocultar · configurações: C1 | comportamento nativo: M1-M5 |
| estados da janela (4) | hidden · idle · idle_hover · active: C2 | - |

Swept: concurrency C3 (um timer só, que segue o fim mais novo; `refresh_idle`
nunca troca estado ativo) · failure modes: menu que falha ao montar ou abrir loga `error!` e não
muda nada · o resto n/a.
