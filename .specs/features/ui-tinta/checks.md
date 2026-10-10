# ui-tinta checks

Profile: light
Plan: none - cada PR cabe numa frase; a especificação é a Parte 3 do relatório de UI (`fala-research/research/21-ui-direcao-e-diagnostico.md`, 3.1 a 3.10, direção "Tinta" escolhida pelo dono em 2026-10-09). Os blocos abaixo são um por PR, empilhados: S1 = PR 1, S2 = PR 2, S3 = PR 3.

## Intent

O Fala ainda tem a cara do Handy: a paleta rosa, a sidebar de configurações, o cartão de settings com título em caixa-alta e "?" com tooltip, o footer com seletor de modelo e a janela 680×570. Com estes três PRs, o app passa a usar tokens de tinta (neutros e semânticas, sem cor de marca), linhas de configuração no padrão do app Configurações do Windows 11, e abre em Início (os ditados) com um rail de três destinos.

14 checks in 1 slice · 0 one-way doors · 0 open, of which 0 block

O repositório não tem vitest. A prova segue o padrão de `src/overlay/pill.test.tsx`: um script `node:assert` rodado pelo `bun`, que lê o CSS e o TSX como texto, imprime `<id> ok` por check e sai com código diferente de 0 na primeira falha. Os portões do frontend são comandos do `package.json` e do CI.

`T1` = `bun src/styles/tokens.test.ts`.

Decisões delegadas (Confirmed? y — delegado), uma linha cada:

- D1: alias `background` → `surface-1` (todo `bg-background` herdado é cartão, menu, tooltip ou diálogo); a janela usa `surface-0` explicitamente no `App.css`. Confirmed? y — delegado
- D2: alias `mid-gray` → `text-2` (texto secundário a 6,7:1 em vez de 3,8:1; `border-mid-gray/20` resulta perto de `border`). Confirmed? y — delegado
- D3: `logo-primary` e `background-ui` → `accent` (tinta). Onde a tinta encontra `text-white` fixo ou texto herdado (botão primário, bolinha do toggle, `Badge` primário, botões do onboarding e do `UpdateChecker`, item ativo da sidebar), o PR 1 troca só essa classe por `on-accent` ou por uma tinta a 10 %, para nenhum texto ficar tinta sobre tinta; sem layout e sem string. Confirmed? y — delegado
- D4: literais rosa em `AudioPlayer` e nos ícones sem uso (`CancelIcon`, `MicrophoneIcon`, `TranscriptionIcon`) viram `var(--color-accent)` e `currentColor`. Confirmed? y — delegado
- D5: a rampa redefine `text-xs`/`sm`/`base`/`lg`/`xl`/`2xl` do Tailwind sobre os degraus do Windows 11 (12, 14, 14, 18, 20, 28) e cria `text-caption`, `text-body`, `text-body-lg`, `text-subtitle`, `text-title`; `rounded-sm`/`md`/`lg` = 4 px e `rounded-xl` = 8 px. Confirmed? y — delegado
- D6: `color-scheme` fica no `App.css` (só janela principal); o overlay é janela transparente e não o recebe. Confirmed? y — delegado

## Checks

### S1 - PR 1, tokens de tinta · 14 files · ~60 KB · ~15k

**C1** - `src/styles/theme.css` define, para cada um dos 13 tokens com tema da tabela 3.7 (`surface-0`, `surface-1`, `surface-2`, `border`, `text`, `text-2`, `text-3`, `accent`, `on-accent`, `warn`, `ok`, `them`, `danger`), o par `--light-color-<t>`/`--dark-color-<t>` com o hex exato da tabela (ex.: `surface-0` `#f3f3f3`/`#202020`, `accent` `#1a1a1a`/`#f5f5f5`); `--color-rec` é `#e5322d` nos dois temas e `--color-me` é `var(--color-ok)`
Proof: `T1` imprime `C1 ok`

**C2** - Os quatro blocos de tema (`:root` padrão, `@media (prefers-color-scheme: dark)`, `:root[data-theme="light"]`, `:root[data-theme="dark"]`) apontam cada um dos 13 `--color-<t>` para `var(--<modo>-color-<t>)` do modo certo
Proof: `T1` imprime `C2 ok`

**C3** - Os nomes herdados são aliases dos tokens novos, definidos uma vez em `:root` e nunca reapontados nos blocos de tema: `background`→`surface-1`, `background-ui`→`accent`, `logo-primary`→`accent`, `logo-stroke`→`text`, `text-stroke`→`surface-1`, `mid-gray`→`text-2`, `warning`→`warn`, `error`→`danger`; nenhum arquivo de `src/` fora dos testes contém os hex rosa herdados (`#faa2ca`, `#da5893`, `#f28cbb`, `#382731`, `#fad1ed`)
Proof: `T1` imprime `C3 ok`

**C4** - Contraste WCAG ≥ 4,5:1 de `text` e `text-2` sobre `surface-0`, `surface-1` e `surface-2`, e de `on-accent` sobre `accent`, nos dois temas
Proof: `T1` imprime `C4 ok`

**C5** - O `:root` tipográfico do `App.css` tem `font-size: 14px`, `line-height: 20px` e `font-family: var(--font-sans)`; o `@theme` define `--font-sans` = `"Segoe UI Variable Text", "Segoe UI Variable", "Segoe UI", system-ui, sans-serif` e `--font-display` = `"Segoe UI Variable Display", "Segoe UI Variable", "Segoe UI", system-ui, sans-serif`; nenhum arquivo de `src/` tem `@font-face`, `fonts.googleapis` ou `.woff`/`.woff2`
Proof: `T1` imprime `C5 ok`

**C6** - O `@theme` define a rampa 12/16, 14/20, 18/24, 20/28, 28/36 como `--text-caption`, `--text-body`, `--text-body-lg`, `--text-subtitle`, `--text-title` (com `--line-height`) e mapeia `xs` 12/16, `sm` 14/20, `base` 14/20, `lg` 18/24, `xl` 20/28, `2xl` 28/36
Proof: `T1` imprime `C6 ok`

**C7** - O `@theme` define `--radius-sm`, `--radius-md` e `--radius-lg` = `4px` e `--radius-xl` = `8px`, e o CSS gerado tem `.rounded-lg{border-radius:var(--radius-lg)}` com `--radius-lg:4px`
Proof: `T1` imprime `C7 ok`
Proof: `bun run build && grep -o -- "--radius-lg:4px" dist/assets/main-*.css && grep -o ".rounded-lg{border-radius:var(--radius-lg)}" dist/assets/main-*.css` sai com 0

**C8** - Foco duplo do Fluent: `:focus-visible` em `@layer base` e a utilidade `@utility focus-ring` têm `outline: 2px solid var(--color-focus-outer)`, `outline-offset: 1px` e `box-shadow: 0 0 0 1px var(--color-focus-inner)`; `--color-focus-outer` = `var(--color-text)` e `--color-focus-inner` = `var(--color-surface-1)`
Proof: `T1` imprime `C8 ok`

**C9** - Movimento: `--default-transition-duration: 100ms` com `--ease-fluent: cubic-bezier(0, 0, 0, 1)`; um bloco `@media (prefers-reduced-motion: reduce)` leva `transition-duration` e `animation-duration` a `0.01ms !important`
Proof: `T1` imprime `C9 ok`

**C10** - A janela pinta `background-color: var(--color-surface-0)`; o wordmark (`FalaTextLogo`, classe `logo-primary`) é preenchido com `var(--color-text)`
Proof: `T1` imprime `C10 ok`

**C11** - O painel Live (`src/overlay/RecordingOverlay.css`) tem `--s-accent: var(--color-accent)` e não cita `logo-primary`, `background-ui` nem `mid-gray`; `src/overlay/Pill.css` não muda e o teste da pill continua verde
Proof: `T1` imprime `C11 ok`
Proof: `git diff --exit-code origin/main...HEAD -- src/overlay/Pill.css` sai com 0
Proof: `bun src/overlay/pill.test.tsx` sai com 0

**C12** - O `Toaster` do `App.tsx` usa só tokens novos: o toast tem `bg-surface-1`, `text-text`, `border-border` e `rounded-xl` (8 px), a descrição é `text-text-2`, e o bloco não cita `logo-primary`, `background-ui`, `mid-gray` nem `bg-background`
Proof: `T1` imprime `C12 ok`

**C13** - Nenhuma linha de `.tsx` em `src/` põe `text-white` junto de um fundo sólido `bg-logo-primary`, `bg-background-ui` ou `bg-accent`; o `Badge` primário é `bg-accent text-on-accent`; a bolinha do toggle ligado é `peer-checked:after:bg-on-accent`; o item ativo da sidebar não é mais `bg-logo-primary/80`; o PR 1 não muda nenhuma string (`git diff origin/main...HEAD -- src/i18n` vazio)
Proof: `T1` imprime `C13 ok`
Proof: `git diff --exit-code origin/main...HEAD -- src/i18n` sai com 0

**C14** - Portões do frontend: `bun run lint`, `bun run format:check`, `bun run check:translations`, `bunx tsc --noEmit`, `bun run build` e `scripts/check-brand.sh` saem com 0
Proof: os seis comandos, cada um com exit 0

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| tokens com tema da 3.7 (13) | C1 e C2, table-driven sobre os 13 | - |
| tokens sem tema (2) | `rec` C1 · `me` C1 | - |
| blocos de tema (4) | `:root` C2 · `prefers-color-scheme: dark` C2 · `data-theme="light"` C2 · `data-theme="dark"` C2 | - |
| aliases herdados (8) | `background` C3 · `background-ui` C3 · `logo-primary` C3 · `logo-stroke` C3 · `text-stroke` C3 · `mid-gray` C3 · `warning` C3 · `error` C3 | - |
| degraus da rampa (11) | C6, table-driven sobre os 11 | - |
| raios (4) | `sm` C7 · `md` C7 · `lg` C7 · `xl` C7 | - |
| itens do PR 1 na 3.10 (8) | tokens C1 · aliases C3 · pilha Segoe C5 · rampa C6 · raiz 14 px C5 · `--radius-lg` C7 · acento Live C11 · wordmark C10 · Toaster C12 | - |
| janelas que importam `theme.css` (2) | principal C10 · overlay C11 | - |

## Swept

- validation: n/a - nenhuma entrada do usuário muda
- failure modes: C4 (contraste) e C13 (tinta sobre tinta)
- idempotency: n/a - CSS estático
- authorization: n/a - sem ação
- concurrency: n/a - sem estado
- data lifecycle: n/a - nenhum dado persistido muda; a escolha de tema continua em `AppSettings`
- dependency failure: C5 - sem Segoe UI Variable (Linux), a pilha cai em `system-ui`
- state transitions: C2 - claro, escuro, forçado claro, forçado escuro
- observability: n/a - sem log

## Out of scope

- PR 4 (feed por dia), PR 5 (onboarding), pill (`src/overlay/Pill.*`), Mica, barra de título própria, `crates/`, ADR da 3.11.
- Captura de tela claro/escuro no Windows: `TODO(windows)`, fora desta máquina (Linux); não entra no veredito.

## Handoff

- S1 = ~15k (theme.css 6 KB, App.css 6 KB, App.tsx 13 KB, RecordingOverlay.css 14 KB, 9 componentes com troca de uma classe ~25 KB) - um builder, sob o orçamento de 150k
