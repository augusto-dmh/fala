# pill-drag — arrastar a pílula ao longo da borda e lembrar o lugar

## Problem

A pílula fica sempre no centro da borda escolhida (`overlay_position`), às vezes em cima do que o
usuário quer ver (a barra de um player, um botão de app). O Flow Bar do Wispr se arrasta e ensina
isso no onboarding ("Click and drag the Flow Bar to move it!", `research/notes/21-wispr.md` § 7);
o nub do Granola se arrasta, conta mais de 3 px como arrasto e salva a posição
(`research/notes/21-granola.md` § 3.1). Pitch: `fala-research/plans/features/pill-idle-hint.md`,
"fatia seguinte". Empilhada sobre `pill-menu`.

## Flow

1. Botão principal apertado na pílula em repouso ou de reunião e movido mais de 3 px: a página chama `start_pill_drag` uma vez (`createPillDrag`)
2. `start_pill_drag` (Windows) lê o monitor da janela e, numa thread, segue o cursor a ~60 Hz enquanto o botão está apertado: só o x muda, o centro vira fração da largura do monitor (`dragged_center`), vai para `PILL_CENTER` e a janela anda com `SetWindowPos` assíncrono, presa ao monitor (`pill_left`)
3. Solto o botão, a fração vai para `pill_center_x`; o clique que encerra o arrasto não abre nada
4. Toda colocação no Windows (`windows_overlay_bounds`) usa `PILL_CENTER`: ditado, reunião, aviso e repouso aparecem no lugar arrastado, em qualquer monitor, na borda de `overlay_position`
5. No boot, `set_pill_center(settings.pill_center_x)` antes do repouso; escolher a borda de novo em Configurações volta ao centro

## Impact

| Front | What changes |
| --- | --- |
| domain | `pill_center_x: Option<f64>` (padrão `None`, centro); `pill_left`, `dragged_center` em `overlay.rs` |
| behaviour | arrastar a pílula em repouso ou de reunião a desliza pela borda e o lugar fica; mudar a borda recentraliza |
| ui | `src/overlay/pillDrag.ts` (limiar e supressão do clique); nenhuma string nova |
| stored data | chave nova com `serde(default)`; nada a migrar |

## Criteria

1. WHEN the primary button is down on the idle or the meeting pill and the pointer moves more than 3 px THEN the pill SHALL follow the cursor horizontally only, inside its monitor, until the button comes up
2. The click that ends a drag SHALL not open anything; a click without a drag SHALL still open
3. The place SHALL be saved as a fraction of the monitor's width (`pill_center_x`) and every state SHALL be placed there on Windows, on the edge `overlay_position` chooses
4. `pill_center_x` SHALL default to `None` (centered), so a store from before the drag keeps the pill centered, and choosing the edge SHALL recenter it
5. The drag SHALL never take focus (`SWP_NOACTIVATE`) and SHALL live in `cfg(windows)` code in `apps/desktop` (ADR-0007)

## Out of scope

Arrastar para outra borda ou soltar em qualquer lugar da tela, arrastar no Linux e no macOS, a
dica de onboarding do Wispr, arrastar a pílula de ditado ou um aviso.

## Assumptions

| Assumption | Chosen default | Confirmed? |
| --- | --- | --- |
| Presa à borda escolhida, só no eixo x | sim (brief do orquestrador; não quebra `overlay_position`) | y — orquestrador |
| Lugar como fração da largura | sim: vale em qualquer monitor e resolução | n — executor |
| Quais pílulas arrastam | repouso e reunião (Wispr: a barra; Granola: o nub da reunião); ditado e avisos não | n — executor |
| Como voltar ao centro | escolher a borda de novo em Configurações | n — executor |
