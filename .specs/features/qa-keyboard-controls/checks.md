# qa-keyboard-controls — checks

Achados Q33, Q34 e Q35 do QA de 2026-10-13: o `Dropdown` não fecha com Esc, não anda com as
setas e não tem `aria-*`; o ↻ e os toggles não têm nome acessível. Diff de uma frase; sem plano.

| ID | Achado | Check | Proof |
| --- | --- | --- | --- |
| K1 | Q35 | ↑/↓ andam pelas opções habilitadas e dão a volta; Home/End vão às pontas | `bun src/components/ui/keyboardControls.test.tsx` K1 |
| K2 | Q35 | numa linha de configuração, o botão se chama pelo título da linha e pelo valor (`aria-labelledby`); `aria-haspopup="listbox"`, `aria-expanded`; lista `listbox`, opções `option` com `aria-selected`; Enter/Espaço e as setas abrem na opção atual; Esc fecha só a lista e devolve o foco ao botão; Tab fecha, do botão ou da lista | `keyboardControls.test.tsx` K2 |
| K3 | Q33 | o `ResetButton` sem rótulo próprio diz "Restaurar o padrão" (nome e dica), em pt e en | `keyboardControls.test.tsx` K3 |
| K4 | Q34 | o checkbox do `ToggleSwitch` é `role="switch"` com `aria-label` do título | `keyboardControls.test.tsx` K4 |
| K5 | — | portões | `bun run lint`, `bunx prettier --check .`, `bunx tsc --noEmit`, `bun run check:translations`, os testes de `src/components/{ui,settings,meeting,onboarding}` e `shell.test.tsx` |
