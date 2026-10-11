# pill-idle — pílula em repouso, com o atalho no hover

## Problem

Fora do ditado e da reunião, a janela da pílula (`recording_overlay`) fica escondida. Quem fecha a
janela principal não vê sinal de que o Fala está rodando nem qual é o atalho; o ícone da bandeja vai
para o overflow no Windows 11. O Flow Bar do Wispr fica visível em repouso e mostra o atalho no
hover (`fala-research/research/20-ui-wispr-granola.md` § 1, § 3). Pitch:
`fala-research/plans/features/pill-idle-hint.md`.

## Flow

1. Avançado → App: toggle novo (só Windows, só com `overlay_style != none`) chama `change_pill_idle_visible_setting`, que grava `pill_idle_visible` e chama `overlay::apply_idle_setting`
2. `apply_idle_setting` (boot, toggle, `change_overlay_style_setting`) atualiza o cache `IDLE_VISIBLE`, liga o poll de tela cheia e chama `refresh_idle`, que só mexe numa janela escondida ou em repouso (`idle_refresh`)
3. `hide_recording_overlay`: a reunião continua primeiro; depois do fade de 300 ms, `rest_after_hide` decide entre `idle` e esconder, se nenhuma sessão nova apareceu (`OVERLAY_SHOW_GENERATION`)
4. Tela cheia (`cfg(windows)`, `overlay.rs`): `SHQueryUserNotificationState` a cada 2 s só com o repouso ligado; mudou, `refresh_idle`
5. Front: `show-overlay` `idle` desenha `IdlePill`; o hover chama `set_idle_pill_hover`, que cresce a janela sem foco, e mostra "Ditar" + as teclas de `transcribe`; o clique chama `open_home_from_pill` (`open-home` → Início). A página chama `pill_overlay_ready` ao montar

## Impact

| Front | What changes |
| --- | --- |
| domain | `pill_idle_visible` (padrão `PILL_IDLE_VISIBLE_DEFAULT = false`); `rest_after_hide`, `idle_refresh` em `overlay.rs` |
| behaviour | com o toggle ligado, uma cápsula fina fica na tela entre as sessões; ditado, reunião e avisos a substituem e ela volta; some com app em tela cheia |
| ui | estados `idle` (janela 64×20) e `idle_hover` (300×64); chaves `settings.advanced.idlePill.*` e `overlay.idle.*` |
| stored data | chave nova com default; nada a migrar |

Relations, Surface, Landing: `None` — comandos e eventos internos, campo novo com `serde(default)`.

## Criteria

1. `pill_idle_visible` SHALL default to `false`, from one constant
2. WHEN the setting is on, `overlay_style` is not `none`, the platform is Windows, no meeting records and no full-screen app is in front THEN after a hide the window SHALL show `idle`
3. WHEN a meeting records THEN a hide SHALL show the meeting state (ADR-0005, existing)
4. WHEN any condition of AC 2 fails THEN a hide SHALL hide the window
5. WHEN the setting or the full-screen state changes THEN a hidden or idle window SHALL follow AC 2/4, and a recording, processing, meeting or notice state SHALL never be replaced
6. `idle` SHALL draw no bars, no red, no dot, no time, and SHALL not animate (ADR-0005)
7. The full-screen poll SHALL run only while the idle pill is wanted, every 2 s, in `cfg(windows)` code in `apps/desktop` (ADR-0007)
8. Hover SHALL grow the window without focus and show "Ditar", the `transcribe` keys as `<kbd>`, and "toque duplo para travar" only for `push_to_talk_double_tap`
9. A click SHALL open the main window on Início
10. The toggle SHALL sit under the pill style in Avançado → App, only on Windows with a pill style on; strings via i18next (pt, en)

## Out of scope

Arrastar e memorizar a posição, menu de clique direito ("Ocultar por 1 hora"), clique na pílula de
reunião (fatias seguintes do pitch); repouso no Linux e macOS (o backend só o liga no Windows).

## Assumptions

| Assumption | Chosen default | Confirmed? |
| --- | --- | --- |
| Padrão do setting | desligado (recomendação do pitch, aceita pelo orquestrador) | n — decisão do Augusto, no PR |
| Repouso com `overlay_style = none` | não aparece; o toggle some, como a posição | n — executor |
| Janela em repouso | 64×20 lógicos, cápsula 40×8 (Wispr: 40×8): pouca área transparente bloqueia cliques | n — executor |
