# pill-redesign checks

Profile: ui
Plan: `.specs/features/pill-redesign/plan.md`

15 checks in 3 slices · 0 one-way doors · 0 open, of which 0 block

Os checks de 1 a 12 seguem a numeração dos ACs do plano. Os de 13 a 15 fecham a ligação no
`RecordingOverlay` e os portões do frontend.

O repositório não tem vitest nem testing-library, e o `package.json` está fora da fronteira. A
prova segue o padrão de `src/components/settings/history/clipboard.test.ts`: um script com
`node:assert` rodado pelo `bun`, que renderiza com `react-dom/server` e lê o `Pill.css` como
texto. A expressão de cada check é uma asserção desse script, rotulada com o id do check, e o
script imprime `<id> ok` por check e sai com código diferente de 0 na primeira falha.

`T` = `bun src/overlay/pill.test.tsx`.

## Checks

### S1 - gravando · 4 files · ~20 KB · ~5k

**C1** - Gravando, a pill é um único elemento `.fpill` sem nenhum texto (o `textContent` do HTML renderizado é vazio), sem `<svg>`, sem `<button>` e sem `<img>`; o `Pill.css` dá a `.fpill` `width: 84px`, `height: 30px` e `border-radius: 15px` (AC 1)
Proof: `T` imprime `C1 ok`

**C2** - Gravando, a `.fpill` tem exatamente 10 filhos `<i>` e nada mais; o `Pill.css` dá a `.fpill` `display: flex`, `align-items: center` e `justify-content: center`, e nenhuma regra de `.fpill` define `flex-wrap` nem `flex-direction: column` (AC 2)
Proof: `T` imprime `C2 ok`

**C3** - `pillBars` com os níveis `[1, 0.6, 0.3, 0.1, 0, ...11 zeros]` devolve 10 alturas com `h[i] == h[9 - i]` para todo `i`, as duas centrais (índices 4 e 5) com a altura de `levels[0]` (18 px), as das pontas (0 e 9) com a de `levels[4]` (3 px), e `levels[1..3]` nas barras 3/6, 2/7 e 1/8; os índices 5 a 15 dos níveis não mudam o resultado (AC 3)
Proof: `T` imprime `C3 ok`

**C4** - `pillBars` dá 3 px para nível 0 (e para níveis negativos ou ausentes), 18 px para nível 1,0 e para níveis acima de 1, e alturas estritamente crescentes para os níveis 0,1 < 0,3 < 0,6 (AC 4)
Proof: `T` imprime `C4 ok`

**C5** - `isHoldToTalk` devolve `true` para `push_to_talk` e `hold_or_toggle` e `false` para `toggle` e para `undefined`; a pill gravando com `holdToTalk` tem a classe `hold`, sem ela não tem; o `Pill.css` pinta `.fpill` de `#000` e `.fpill.hold` de `#e5322d` (AC 5)
Proof: `T` imprime `C5 ok`

**C6** - Gravando com `ready = false`, as 10 barras têm 3 px mesmo com níveis altos e a pill tem a classe `arming`; o `Pill.css` dá `opacity: 0.45` às barras de `.fpill.arming`; com `ready = true` a classe some e as barras seguem os níveis (AC 6)
Proof: `T` imprime `C6 ok`

### S2 - processando · 2 files · ~12 KB · ~3k

**C7** - Processando, a pill tem a classe `processing`, 10 barras, nenhum texto, `<svg>` ou `<button>`, nunca a classe `hold` (mesmo com `holdToTalk = true`), e as alturas são a forma fixa `[5, 7, 9, 11, 13, 13, 11, 9, 7, 5]`; `toPillMode` leva `transcribing` e `processing` a `processing` e `recording` a `recording` (AC 7)
Proof: `T` imprime `C7 ok`

**C8** - Processando, renderizar com níveis `[1, 1, ...]` e com níveis zerados dá o mesmo HTML; o `Pill.css` dá a `.fpill.processing` a animação `fpill-pulse` de `1.2s` em loop infinito, e o `@keyframes fpill-pulse` vai de `opacity: 1` a `opacity: 0.55` (AC 8)
Proof: `T` imprime `C8 ok`

**C9** - O `Pill.css` tem um bloco `@media (prefers-reduced-motion: reduce)` que dá `animation: none` e `opacity: 1` à `.fpill.processing` (AC 9)
Proof: `T` imprime `C9 ok`

### S3 - oculto e acessível · 4 files · ~30 KB · ~8k

**C10** - `toPillMode` devolve `null` para `streaming`, e `RecordingOverlay` devolve `null` antes de renderizar a pill quando `isVisible` é falso (o `hide-overlay` põe `isVisible = false`) (AC 10)
Proof: `T` imprime `C10 ok` (a parte de `toPillMode`); a parte do `RecordingOverlay` é leitura no Verifier: `if (!isVisible) return null;` antes do ramo da pill e o listener de `hide-overlay` com `setIsVisible(false)`

**C11** - A pill tem `role="status"` e `aria-label` igual ao rótulo recebido; `overlay.recording` existe em `pt` ("Gravando") e em `en` ("Recording"), e `RecordingOverlay` passa `t("overlay.recording")` gravando e `t("overlay.processing")` processando (AC 11)
Proof: `T` imprime `C11 ok` (role, aria-label e as duas chaves lidas dos JSON); `bun run check:translations` sai com 0

**C12** - O `Pill.css` não usa `var(` nem `prefers-color-scheme` nem `[data-theme`: as cores da pill são literais (`#000`, `#e5322d`, barras `#fff`) e valem nos dois temas (AC 12)
Proof: `T` imprime `C12 ok`

**C13** - No ramo mínimo, `RecordingOverlay` renderiza `<Pill>` dentro de `.ov-stage` e não renderiza mais `.scard compact`, o botão `.sx`, o spinner `.sspinner` nem o `.swork-label`; o ramo `streaming` continua igual ao de `origin/main` (Out of scope)
Proof: `T` imprime `C13 ok` (lê o fonte do `RecordingOverlay.tsx` e confere o ramo mínimo por texto); `git diff origin/main -- src/overlay/RecordingOverlay.tsx` mostra o ramo `if (state === "streaming")` sem mudança (Verifier)

**C14** - `RecordingOverlay` lê `shortcut_activation` em `getAppSettings()` no `show-overlay` e passa `isHoldToTalk(...)` à pill
Proof: `T` imprime `C14 ok` (texto do fonte); leitura no Verifier

**C15** - Os portões do frontend passam: `bun run lint` (inclusive a regra i18next contra literal em JSX), `bunx prettier --check src/overlay src/i18n`, `bun run check:translations` e `bun run build` (tsc + vite) saem com 0
Proof: os quatro comandos, cada um com exit 0

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| pill states (3) | gravando C1 · processando C7 · oculto C10 | - |
| overlay events into the pill (4) | `recording` C7 · `transcribing` C7 · `processing` C7 · `streaming` (não é pill) C10 | - |
| `shortcut_activation` values (4) | `push_to_talk` C5 · `hold_or_toggle` C5 · `toggle` C5 · ausente C5 | - |
| bar inputs (4) | silêncio C4 · nível 1,0 C4 · acima de 1 C4 · crescente C4 | - |
| capture readiness (2) | `ready = false` C6 · `ready = true` C6 | - |
| motion preference (2) | padrão C8 · `reduce` C9 | - |
| locales (2) | `pt` C11 · `en` C11 | - |
| binding sketch elements (8) | cápsula C1 · preta C5 · ~84×30 C1 · 10 barras C2 · simétricas C3 · vermelha em PTT C5 · processando parado + pulso C7, C8 · sem ícone, sem texto C1, C7 | - |
| removed from the minimal overlay (4) | botão X C13 · spinner C13 · rótulo C13 · `.scard compact` C13 | - |

- Claims about the rendered pill run the real component through `react-dom/server`; claims about CSS read the shipped `Pill.css`.
- C10 (parte do `RecordingOverlay`), C13 e C14 provam a ligação por texto do fonte, não por execução: sem `tauri dev` (fora da fronteira de RAM) e sem mock de módulo no runner, o listener não roda em teste. Gap de nível declarado.
- No other check claims more than the single case its proof exercises.

## Test policy

| Code | Required proofs | Coverage expectation |
| --- | --- | --- |
| `src/overlay/pillModel.ts` (decide: 4 eventos -> modo, 3 ativações -> vermelho, níveis -> alturas com clamp e espelho) | uma no próprio nível | um caso afirmado por linha de cada tabela de decisão |
| `src/overlay/Pill.tsx` (decide: modo × hold × ready -> classes e alturas) | render estático por combinação que muda a saída | gravando hold, gravando sem hold, armando, processando com hold |
| ligação em `RecordingOverlay.tsx` (encaminha eventos e settings à pill, sem decisão própria) | nenhuma própria | coberta pela leitura do fonte (C13, C14) e pelo build (C15) |

Evidence:

- `src/overlay/pillModel.ts`: três funções, 9 pontos de decisão (4 eventos, 3 ativações + ausente, clamp inferior e superior) -> decides
- `src/overlay/Pill.tsx`: 3 condições (modo, hold, ready) -> decides
- closest analogue: `src/components/settings/history/clipboard.test.ts`, script `node:assert` sobre uma função pura, rodado por `bun`

Cost: 1 script de prova sobre 2 arquivos. Sem estas linhas, as tabelas do `pillModel.ts` só seriam provadas pelo build, que não afirma valor nenhum.

## Swept

- validation: C4 (níveis fora de 0-1, ausentes), C5 (ativação ausente)
- failure modes: n/a - a pill não faz I/O; falha de `getAppSettings()` mantém o último valor (comportamento existente do `show-overlay`)
- idempotency: C8 (processando ignora níveis: mesmo HTML)
- authorization: n/a - janela local sem usuários
- concurrency: n/a - eventos chegam pela mesma thread do webview; a ordem `show-overlay`/`recording-ready` já é tratada pelo código existente
- data lifecycle: n/a - nada persistido
- dependency failure: n/a - nenhum serviço; `getAppSettings()` já é chamado hoje e tem `catch`
- state transitions: C1, C6, C7, C10 (armando -> gravando -> processando -> oculto)
- observability: n/a - a pill não loga; nenhum texto ditado passa por ela

## Handoff

- S1–S3 tocam `src/overlay/{pillModel.ts,Pill.tsx,Pill.css,pill.test.tsx,RecordingOverlay.tsx}` e `src/i18n/locales/{pt,en}/translation.json`; existente ~70 KB (RecordingOverlay.tsx 12 KB + css 14 KB + dois JSON ~45 KB) + novo ~15 KB = ~85 KB / 4 = ~21k tokens, abaixo do budget de 150k - one builder
- Mechanism: one builder (cabe no budget, sem pergunta)
