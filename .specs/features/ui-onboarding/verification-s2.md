# ui-onboarding verification (S2, PR 2)

**Verdict**: PASS
**Profile**: light
**Diff range**: f6c6c11..0194e4b
**Round**: 1 - full
**Verifier**: independent sub-agent (author != verifier)

Escopo: só o bloco S2 (C14–C16) e a decisão D11, no commit 0194e4b (`fix/tray-update-check`). Provas rodadas na worktree real em `docs/adr-ui-identity` (2923c42), cuja árvore `apps/` é idêntica a 0194e4b (`git diff --stat 0194e4b HEAD -- apps` vazio).

## Binding sources

Não roda sob `light`. D17 de `.specs/features/ui-tinta/checks.md` e a ADR-0008 (linha 16: "Updater desligado até o primeiro release") foram lidas só para conferir que o PR não religa o updater: `tauri.conf.json` e `lib.rs` não estão no diff, e o PR só esconde o item.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C14 | `settings::UPDATER_ENABLED` é `false`, com comentário que aponta `src/lib/updater.ts` e a ADR-0008 | `cargo test -p fala --lib -- tray` exit 0, `tray::tests::check_updates_item_hidden_while_updater_off ... ok` | `apps/desktop/src/settings.rs:1381` - `pub const UPDATER_ENABLED: bool = false;`, comentário em `settings.rs:1378-1380` cita "ADR-0008, docs/RELEASE.md" e "`UPDATER_ENABLED` in `src/lib/updater.ts`"; `apps/desktop/src/tray.rs:807-808` - `let updater_enabled = settings::UPDATER_ENABLED; assert!(!updater_enabled);` | PASS |
| C15 | `check_updates_item_visible()` é `false` com `UPDATER_ENABLED` `false`, independente de `FALA_DISABLE_UPDATER`, e `build_menu` remove o item por ele | mesmo `cargo test` exit 0; `grep -n "if !check_updates_item_visible()" apps/desktop/src/tray.rs` exit 0 (`638:`) | `apps/desktop/src/tray.rs:486` - `settings::UPDATER_ENABLED && !settings::update_checks_forced_disabled()` (o `&&` curto-circuita: com a constante `false`, o env não importa); `tray.rs:809` - `assert!(!check_updates_item_visible());`; `tray.rs:638-639` - `if !check_updates_item_visible() { menu.remove(&check_updates_i)?; }`, depois dos dois ramos (busy `tray.rs:562`, idle `tray.rs:625`) | PASS |
| C16 | o crate compila, os testes de `tray` passam, o PR não toca `src/` nem `crates/` | `cargo test -p fala --lib -- tray` exit 0, 11 passed, 0 failed (o build de teste compila o crate; `cargo check -p fala` separado não foi rodado); `git diff --exit-code f6c6c11..0194e4b -- src crates` exit 0; `git diff --exit-code feat/ui-onboarding...fix/tray-update-check -- src crates` exit 0 (branches = f6c6c11 e 0194e4b) | saída do runner: `test result: ok. 11 passed; 0 failed`; `git diff --stat f6c6c11..0194e4b` lista só `apps/desktop/src/settings.rs` e `apps/desktop/src/tray.rs` | PASS |

## Review

- Os dois layouts: `build_menu` monta `menu` num `if inputs.busy { ... } else { ... }`, os dois com `&check_updates_i` (`tray.rs:562` e `tray.rs:625`), e a remoção em `tray.rs:638-639` roda depois do `if`, sobre o menu de qualquer ramo. O aviso de Secure Input é inserido depois, em posições fixas 2 e 3, que ficam acima do item; nada o reinsere.
- `update_checks_forced_disabled()` (`settings.rs:1386-1390`) não mudou: ainda lê só `FALA_DISABLE_UPDATER`. Os outros chamadores, `commands::is_update_checks_locked` (`commands/mod.rs:27-28`), o toggle de Debug (`shortcut/mod.rs:817`), `update_checks_effectively_enabled` (`settings.rs:1396-1398`), o handler `"check_updates"` (`lib.rs:296-301`) e `trigger_update_check` (`lib.rs:385-390`), não estão no diff e mantêm o sentido (D11 cumprida).
- AGENTS.md: o diff não adiciona `#[cfg]`, nem `unwrap`/`expect` fora de teste, e não mexe no updater (`plugins.updater` em `tauri.conf.json` e o `tauri_plugin_updater` em `lib.rs:891` estão fora do diff). `cargo fmt --all -- --check` exit 0.
- Discriminação (raciocínio, sem injeção, porque `light`): trocar `UPDATER_ENABLED` para `true` falha o teste em `tray.rs:808` (`assert!(!updater_enabled)`). Voltar a função à fórmula antiga (`!update_checks_forced_disabled()`) também falha em `tray.rs:809`, porque o env não está definido no teste.

## Findings

Nenhum bloqueante.

1. (observação) O teste não passa por `build_menu`, que precisa de `AppHandle`. Se alguém tirar o `!` de `tray.rs:638` ou voltar a testar `update_checks_forced_disabled()` ali, o `cargo test` continua verde. Esse caso só é pego pela prova `grep`, que C15 já exige. A prova é estrutural, não de comportamento, mas é a que o check declara.
2. (cosmético) O doc comment de `build_menu` (`tray.rs:489-492`) ainda cita só o env `FALA_DISABLE_UPDATER` como dependência fixa do processo. Agora a constante `UPDATER_ENABLED` também é. O comentário de `check_updates_item_visible` (`tray.rs:483-484`) já cita as duas.
3. (cosmético) O handler `"check_updates"` em `lib.rs:296-301` não tem mais como ser chamado enquanto o item estiver escondido. Ele não faz mal e volta a servir quando a constante virar `true`.

## Faults injected

Não roda sob `light`. A discriminação está na seção Review, só por raciocínio.

## Coverage

Não roda sob `light`.

## Gate

`CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=1 cargo test -p fala --lib -- tray` - 11 passed, 0 failed (inclui `tray::tests::check_updates_item_hidden_while_updater_off`)

`git status --porcelain` da árvore real antes e depois: só `?? .specs/features/ui-onboarding/verification-s1.md` e `?? .specs/features/ui-onboarding/verification-s3.md`, mais este relatório.
