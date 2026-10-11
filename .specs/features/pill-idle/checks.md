# pill-idle checks

Profile: light
Plan: `.specs/features/pill-idle/plan.md`

10 checks in 2 slices · 0 one-way doors · 0 open

Cargo com `CARGO_TARGET_DIR=C:\f\pill-idle-hint`, na raiz do worktree. As provas estruturais leem só
código de produção (antes de `mod tests`) e falham quando o alvo some (L-002).

## Checks

### S1 - repouso

**C1** - `PILL_IDLE_VISIBLE_DEFAULT` é `false` e é a única fonte do default: `get_default_settings()` e um store sem a chave dão `pill_idle_visible = false` (AC 1)
Proof: `cargo test -p fala --lib settings::tests::pill_idle_defaults_off`

**C2** - `rest_after_hide`: tabela completa de `idle_wanted` × reunião (`None`, `Recording`, `Paused`) × tela cheia: reunião -> `Meeting(estado)` sempre; sem reunião, `Idle` só com `idle_wanted` e sem tela cheia; o resto `Hidden`. `idle_wanted` é verdadeiro só com o setting ligado, `overlay_style != None` e plataforma suportada (AC 2, 3, 4; 12 casos + 8 de `idle_wanted`)
Proof: `cargo test -p fala --lib overlay::tests::rest_after_hide_table`

**C3** - `idle` mede 64×20 e `idle_hover` 300×64; os outros estados não mudam (AC 8)
Proof: `cargo test -p fala --lib overlay::tests::idle_dimensions`

**C4** - `hide_recording_overlay` checa a reunião antes de emitir `hide-overlay` (existente) e o fim agendado decide por `rest_after_hide` dentro da checagem de geração; `refresh_idle` só age quando `SHOWN` é hidden ou idle (AC 2, 5)
Proof: `cargo test -p fala --lib overlay::tests::meeting_overlay_states` e `overlay::tests::idle_never_replaces_an_active_state`

**C5** - O poll de tela cheia está em `#[cfg(target_os = "windows")]` dentro de `overlay.rs`, usa `SHQueryUserNotificationState` com os três estados, dorme 2 s e sai quando `IDLE_VISIBLE` desliga (AC 7)
Proof: `cargo test -p fala --lib overlay::tests::fullscreen_poll_is_gated` e, no Windows, `overlay::tests::fullscreen_states` (os três estados contam, `QUNS_NOT_PRESENT`, `QUNS_ACCEPTS_NOTIFICATIONS`, `QUNS_QUIET_TIME` e `QUNS_APP` não); o `cfg(windows)` é provado pelo build Linux do CI

### S2 - hover e clique

**C6** - `IdlePill` em repouso: só a cápsula, sem `<i>`, sem `.dot`, sem `.hold`/`.limit`, sem texto; o CSS de `.fpill.idle` não tem `animation` nem cor vermelha (AC 6)
Proof: `bun src/overlay/pill.test.tsx` (linha `C6 ok`)

**C7** - `IdlePill` em hover: "Ditar" traduzido e um `<kbd>` por tecla do binding recebido (`ctrl+meta` -> `Ctrl`, `Meta`); a dica de toque duplo só com `push_to_talk_double_tap` (AC 8)
Proof: `bun src/overlay/pill.test.tsx` (linha `C7 ok`)

**C8** - `RecordingOverlay` trata `idle`: `onMouseEnter`/`onMouseLeave` chamam `setIdlePillHover(true/false)` e o clique chama `openHomeFromPill`; `App.tsx` ouve `open-home` e vai para `home`; `open_home_from_pill` chama `show_main_window` e emite `open-home` (AC 8, 9)
Proof: `bun src/overlay/pill.test.tsx` (linha `C8 ok`)

**C9** - `ShowIdlePill` fica em `AdvancedSettings` logo depois de `ShowOverlay`, some fora do Windows e com `overlay_style === "none"`; as chaves `settings.advanced.idlePill.*` e `overlay.idle.*` existem em pt e en (AC 10)
Proof: `bun src/overlay/pill.test.tsx` (linha `C9 ok`), `bun run check:translations`

**C10** - Portões: `bun run lint`, `bunx tsc --noEmit`, `bun run check:translations`, `bun run format:check`, `cargo fmt --all -- --check`, `cargo clippy -p fala --all-targets -- -D warnings`, `scripts/check-brand.sh`; `bun src/overlay/notice.test.tsx` e o resto de `pill.test.tsx` seguem verdes
Proof: os comandos acima, exit 0

## Manual on Windows (fora da tabela, L-004)

Roteiro M1-M5 no corpo do PR (hover, ditado, reunião, F11, clique, CPU/RAM); vai ao
`verification.md` como evidência ou "Not tested" e não muda o veredito.

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| eixos de `rest_after_hide` (3) | `idle_wanted` · reunião ×3 · tela cheia: C2 | - |
| condições de `idle_wanted` (3) | setting · `overlay_style` · plataforma: C2 | - |
| estados de tela cheia (3) | C5 | comportamento real: M3 |

Swept: concurrency C4 (geração; `refresh_idle` nunca troca estado ativo) · state transitions C2, C4 ·
failure modes: consulta de tela cheia que falha conta como "não" e loga em `debug!` · o resto n/a.
