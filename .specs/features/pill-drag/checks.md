# pill-drag checks

Profile: light
Plan: `.specs/features/pill-drag/plan.md`

5 checks in 1 slice · 0 one-way doors · 0 open

Cargo com `CARGO_TARGET_DIR=C:\f\pill-menu-drag`, na raiz do worktree. Os testes de geometria são
`cfg(windows)`, como os de `windows_overlay_bounds`; o build Linux do CI prova o `cfg`.

## Checks

**C1** - `pill_left` com o centro em 0,5 dá a fórmula antiga de centralizar; em outras frações põe o centro ali e prende a janela ao monitor; `dragged_center` soma o deslocamento do cursor e prende em 0..1; `windows_overlay_bounds` muda só o x com o centro (AC 1, 3)
Proof: `cargo test -p fala --lib overlay::tests::dragged_pill_slides_along_its_edge` (Windows)

**C2** - o arrasto segue enquanto o botão principal está apertado (`GetAsyncKeyState`), salva `pill_center_x` só depois de soltar e só se a janela andou (um toque rápido não salva nada); `place_windows_overlay` lê `pill_center()`; o boot chama `set_pill_center` antes de `apply_idle_setting`; `change_overlay_position_setting` zera o centro; `set_pill_center` ignora um valor não finito (AC 3, 4)
Proof: `cargo test -p fala --lib overlay::tests::pill_center_setting_is_applied_and_reset`

**C3** - `pill_center_x` vale `None` no padrão e num store sem a chave (AC 4)
Proof: `cargo test -p fala --lib settings::tests::pill_idle_defaults_off`

**C4** - `createPillDrag`: limiar de 3 px; clique parado passa; arrasto chama `start_pill_drag` uma vez e engole o clique do fim; botão direito ou movimento sem botão não arrasta; os handlers estão só no ramo `idle` e no de reunião e só no Windows, para um deslize da mão não engolir o clique no Linux (AC 1, 2)
Proof: `bun src/overlay/pill.test.tsx` (linha `pill-drag C4 ok`)

**C5** - Portões: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p fala --lib`, `bun run lint`, `bun run format:check`, `bunx tsc --noEmit`, `bun run check:translations`, `scripts/check-brand.sh`
Proof: os comandos acima, exit 0

## Manual on Windows (fora da tabela)

M1 arrastar a pílula em repouso: desliza pela borda de baixo sem subir, sem tirar o foco do app
em uso; soltar não abre Início · M2 ditar: a pílula de gravação aparece no lugar arrastado ·
M3 reiniciar o app: a pílula volta ao lugar · M4 trocar a borda em Configurações: volta ao centro
· M5 dois monitores de DPI diferentes: o lugar vale nos dois.

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| pílulas que arrastam (2) | repouso · reunião: C4 | arrasto nativo: M1 |
| caminhos do centro (3) | boot · arrasto · troca de borda: C2 | M3, M4 |

Swept: concurrency C2 (um arrasto por vez, `PILL_DRAGGING`; o hover que redimensiona no meio usa
o centro vivo) · failure modes: monitor, janela ou cursor ilegíveis não arrastam e não salvam ·
o resto n/a.
