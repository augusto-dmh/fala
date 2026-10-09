# ui-tinta verification

**Verdict**: PASS
**Profile**: light
**Diff range**: eb0482c..c0ed440 (`origin/main...HEAD`, slice S1 = PR 1)
**Round**: 1 - full
**Verifier**: independent sub-agent (author != verifier); não escreveu o código, rodou cada prova em `c0ed440`

Escopo: os 14 checks de S1, C1..C14 (o cabeçalho de `checks.md` diz "13 checks in 1 slice", mas lista 14; ver O6) contra a Parte 3 de `fala-research/research/21-ui-direcao-e-diagnostico.md` (3.6, 3.7 e a linha do PR 1 na 3.10), lida como fonte. Não há `plan.md` (perfil light, "cada PR cabe numa frase").

## Binding sources

Perfil light: a comparação do passo 1 não é obrigatória. Mesmo assim li 3.6, 3.7 e 3.10 e comparei com os checks; nenhuma contradição.

| Source | Opened | Contradiction | Uncovered |
| --- | --- | --- | --- |
| 3.7 tabela de cor (15 linhas) | yes - `21-ui-direcao-e-diagnostico.md` linhas 244-262 | - | - |
| 3.7 tipografia, forma, movimento | yes - linhas 264-285 | - | - |
| 3.10 linha do PR 1 | yes - linha 314 | - | - |

Notas: (a) a 3.10 diz "Nenhum componente" para o PR 1; as trocas de uma classe em 9 componentes são a decisão D3/D4 delegada e confirmada em `checks.md`, não contradição. (b) A 3.7 lista `text` entre "nomes antigos que viram alias"; `text` é também token novo, então não ter alias para ele é coerente. (c) Movimento 3.7 "80-120 ms ease-out" = 100 ms `cubic-bezier(0, 0, 0, 1)`.

## Checks

`T1` = `bun src/styles/tokens.test.ts`, exit 0, imprime `C1 ok` .. `C13 ok` individualmente (13 linhas). Linhas citadas de `src/styles/tokens.test.ts` (arquivo novo neste diff).

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | 13 pares claro/escuro com hex da 3.7; `rec` `#e5322d`; `me` = `ok` | `T1` exit 0, `C1 ok` | `src/styles/tokens.test.ts:70` - `assert.equal(rootBlock.get(\`--light-color-${token}\`), light, ...)` e `:75` (dark) sobre a tabela `PALETTE` (`:44-58`, conferida hex a hex contra a 3.7); `:77` - `assert.equal(rootBlock.get("--color-rec"), "#e5322d")`; `:78` - `"var(--color-ok)"`. Código: `src/styles/theme.css:11`, `:38`, `:65` | PASS |
| C2 | 4 blocos de tema apontam os 13 tokens para o modo certo | `T1`, `C2 ok` | `src/styles/tokens.test.ts:90` - `assert.equal(map.get(\`--color-${token}\`), \`var(--${mode}-color-${token})\`)` sobre os 4 blocos de `:82-87`. Código: `src/styles/theme.css:83` (media), `:127` (dark forçado) | PASS |
| C3 | 8 aliases em `:root`, nunca reapontados; nenhum hex rosa em `src/` | `T1`, `C3 ok` | `src/styles/tokens.test.ts:111` - `assert.equal(rootBlock.get(\`--color-${alias}\`), \`var(--color-${target})\`)`; `:117` - `map.get(...) === undefined` nos 3 outros blocos; `:126` - `assert.ok(!PINK.test(...))`. Código: `src/styles/theme.css:73-80`. Varredura extra minha: `grep -rniE "#(faa2ca ou da5893 ou f28cbb ou 382731 ou fad1ed)" src index.html` (todos os tipos de arquivo) sem acerto | PASS |
| C4 | contraste ≥ 4,5:1 de `text`/`text-2` sobre `surface-0/1/2` e `on-accent` sobre `accent`, 2 temas | `T1`, `C4 ok` | `src/styles/tokens.test.ts:146` - `assert.ok(ratio >= 4.5, ...)`; `:150` - `assert.ok(onAccent >= 4.5, ...)`. Calcula sobre a tabela do teste, que C1 amarra ao CSS por igualdade estrita | PASS |
| C5 | raiz 14/20 com `var(--font-sans)`; pilhas Segoe exatas; sem fonte embutida | `T1`, `C5 ok` | `src/styles/tokens.test.ts:158-160` - `assert.match(appRoot, /font-size: 14px;/)` etc.; `:162-165` - `--font-sans` exato; `:166-169` - `--font-display` exato; `:172-175` - sem `@font-face`/`.woff`. Código: `src/App.css:126`, `:39` | PASS |
| C6 | 11 degraus (5 novos + `xs`..`2xl`) com line-height | `T1`, `C6 ok` | `src/styles/tokens.test.ts:195` - `assert.equal(themeAt.get(\`--text-${step}\`), size)`; `:196-199` - `--line-height`, sobre os 11 de `:181-193`. Código: `src/App.css:48` em diante | PASS |
| C7 | `--radius-sm/md/lg` 4px, `xl` 8px; CSS gerado com `.rounded-lg{border-radius:var(--radius-lg)}` e `--radius-lg:4px` | `T1` `C7 ok`; `bun run build` exit 0 + os 2 `grep -o` em `dist/assets/main-uCppzzVB.css` exit 0 (acertos `--radius-lg:4px` e `.rounded-lg{border-radius:var(--radius-lg)}`) | `src/styles/tokens.test.ts:207` - `assert.equal(themeAt.get("--radius-lg"), "4px")`; `:205`, `:206`, `:208`. Código: `src/App.css:74-75` | PASS |
| C8 | foco duplo em `:focus-visible` (`@layer base`) e `@utility focus-ring`; `focus-outer`=`text`, `focus-inner`=`surface-1` | `T1`, `C8 ok` | `src/styles/tokens.test.ts:222-223` - `assert.ok(focusVisible.includes(decl))` / `focusUtility.includes(decl)` sobre as 3 declarações de `:212-216`; `:225-226`. Código: `src/App.css:83`, `:92`; `src/styles/theme.css:67`. No `dist`: `.focus-ring{outline:2px solid var(--color-focus-outer);outline-offset:1px;box-shadow:0 0 0 1px var(--color-focus-inner)}` | PASS |
| C9 | 100 ms, `--ease-fluent`, reduced-motion a `0.01ms !important` | `T1`, `C9 ok` | `src/styles/tokens.test.ts:230-231` - `assert.equal(themeAt.get("--default-transition-duration"), "100ms")`, `--ease-fluent`; `:235-236` - `assert.match(reduced, /transition-duration: 0\.01ms !important;/)`. Código: `src/App.css:78`, `:114` | PASS |
| C10 | janela em `surface-0`; wordmark `.logo-primary` com `fill: var(--color-text)` | `T1`, `C10 ok` | `src/styles/tokens.test.ts:240` - `assert.match(appRoot, /background-color: var\(--color-surface-0\);/)`; `:241` - `fill: var(--color-text)`; `:242-245` - `FalaTextLogo` usa `className="logo-primary"`. Código: `src/App.css:152`, `:211`; `src/components/icons/FalaTextLogo.tsx:31` | PASS |
| C11 | `--s-accent: var(--color-accent)`, sem nomes herdados; `Pill.css` intacto; teste da pill verde | `T1` `C11 ok`; `git diff --exit-code origin/main...HEAD -- src/overlay/Pill.css` exit 0; `bun src/overlay/pill.test.tsx` exit 0 ("pill: all assertions passed") | `src/styles/tokens.test.ts:249` - `assert.match(overlayCss, /--s-accent: var\(--color-accent\);/)`; `:250` - `assert.doesNotMatch(overlayCss, /logo-primary ou background-ui ou mid-gray/)`. Código: `src/overlay/RecordingOverlay.css:11` | PASS |
| C12 | Toaster: `bg-surface-1`, `text-text`, `border-border`, `rounded-xl`, descrição `text-text-2`, sem nomes herdados | `T1`, `C12 ok` | `src/styles/tokens.test.ts:263` - `assert.ok(toaster.includes(cls))` sobre os 4 de `:257-262`; `:265` - `assert.match(toaster, /description: "text-text-2"/)`; `:266-269` - `doesNotMatch` herdados. Código: `src/App.tsx:307`, `:309` | PASS |
| C13 | sem `text-white` com fundo de tinta sólido; `Badge` primário `bg-accent text-on-accent`; bolinha `peer-checked:after:bg-on-accent`; sidebar sem `/80`; i18n intocado | `T1` `C13 ok`; `git diff --exit-code origin/main...HEAD -- src/i18n` exit 0 | `src/styles/tokens.test.ts:276-279` - `assert.ok(!/\btext-white\b/.test(line), ...)` em toda linha `.tsx` com `bg-(logo-primary ou background-ui ou accent)` sólido; `:283-286` Badge; `:287-290` toggle; `:291` - `assert.doesNotMatch(Sidebar, /bg-logo-primary\/80/)`. Varredura extra minha: os 13 usos sólidos restantes de `bg-logo-primary` (`ModelCard.tsx:208,219,296,335,345`, `ModelStatusButton.tsx:35`, `ProgressBar.tsx:47,78`) são barras/pontos sem texto; o único `text-white` restante é `Button.tsx:38` sobre `bg-red-600` | PASS |
| C14 | seis portões com exit 0 | `bun run lint` 0 · `bunx prettier --check .` 0 ("All matched files use Prettier code style!") · `bun run check:translations` 0 · `bunx tsc --noEmit` 0 · `bun run build` 0 · `scripts/check-brand.sh` 0 ("ok: no Handy branding outside the allowlist") | `package.json:11` (`eslint src`), `:14` (`format:check`), `:20` (`check:translations`), `:8` (`build`) - os comandos que a prova nomeia. Ver observação O4 sobre a metade `cargo fmt` | PASS |

## Coverage

Perfil light: recompute não obrigatório; refiz o join dos conjuntos por ser barato.

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| tokens com tema (13) | tabela da 3.7 | `PALETTE` em `tokens.test.ts:44-58` tem os 13, hex iguais à 3.7; C1 e C2 iteram sobre eles | - |
| tokens sem tema (2) | 3.7 (`rec`, `me`) | `tokens.test.ts:77`, `:78` | - |
| blocos de tema (4) | `theme.css` | `tokens.test.ts:82-87` | - |
| aliases (8) | `checks.md` C3 e `theme.css:73-80` | `tokens.test.ts:100-109` | - |
| degraus da rampa (11) | 3.7 + D5 | `tokens.test.ts:181-193` | - |
| raios (4) | D5 | `tokens.test.ts:205-208` | - |

## Faults injected

Perfil light: não obrigatório; injetei 5 por serem baratos. Mutação direta na árvore com cópia de segurança em scratchpad, restaurada com `cp` após cada uma; `git status --porcelain` vazio antes e depois de cada restauração.

| Mutation | Location | Killed |
| --- | --- | --- |
| `--dark-color-accent` `#f5f5f5` -> `#f5f5f4` | `src/styles/theme.css:34` | yes - `T1` exit 1, `AssertionError: accent dark` (C1) |
| `data-theme="dark"` `--color-text-2` -> `var(--light-color-text-2)` | `src/styles/theme.css:133` | yes - `C1 ok` e então `AssertionError: data-theme="dark": --color-text-2` (C2) |
| `--radius-lg: 4px` -> `6px` | `src/App.css:74` | yes - `T1` para em C7 (`actual "6px"`); e após `bun run build` o `grep -o -- "--radius-lg:4px"` sai 1; rebuild após restaurar volta a sair 0 |
| Button primário `text-on-accent` -> `text-white` | `src/components/ui/Button.tsx:27` | yes - `AssertionError: text-white on ink in .../Button.tsx` (C13) |
| `--s-accent: var(--color-accent)` -> `var(--color-logo-primary)` | `src/overlay/RecordingOverlay.css:11` | yes - `C10 ok` e então falha em C11 |

## Findings

Nenhuma bloqueante. Observações:

- O1 (escopo, informativa): `src/overlay/RecordingOverlay.css` também troca `--s-font` (pilha Apple/Roboto -> pilha Segoe) e a mistura de `--s-accent-soft` (16/20 % -> 10/16 %). Cabem em "pilha Segoe UI Variable" e "acento do painel Live" da 3.10, mas nenhum check as cobre.
- O2 (escopo, informativa): no Toaster, o título passa de `font-medium` para `font-semibold` e o `actionButton` é restilizado (`text-caption`, `rounded-md`, `bg-surface-2`, `hover:bg-text/10`). Cabe em "Toaster com tokens novos"; C12 não cobre esses dois campos.
- O3 (discriminação do teste, baixa): C9 corta `App.css` do `@media (prefers-reduced-motion: reduce)` até o fim do arquivo (`tokens.test.ts:234`), então uma declaração `0.01ms !important` fora do bloco também passaria. C4 calcula o contraste sobre a tabela do teste, não sobre o CSS; isso é sólido só porque C1 amarra os dois por igualdade.
- O4 (prova parcial, por restrição do briefing): `bun run format:check` = `prettier --check . && cargo fmt --all -- --check`. Rodei só a metade prettier (exit 0); `cargo` estava proibido nesta sessão. O diff não toca nenhum `.rs` (`git diff --stat origin/main...HEAD` lista só `src/` e `.specs/`).
- O5 (herdado, não é regressão): o `@theme inline` emite `--color-surface-0:var(--color-surface-0)` (e o mesmo para os outros tokens) em `@layer theme` no CSS gerado. É o mesmo padrão que `--color-text` já tinha em `main`, e o `:root` sem camada de `theme.css` sobrescreve, então os utilitários resolvem certo.
- AGENTS.md: nenhum literal novo em JSX (o diff de `.tsx` só troca classes e um `color` padrão de ícone); `src/i18n` intocado; `check-brand.sh` verde; nada fora de `src/` e `.specs/`; nenhum código não relacionado foi reformatado; ícones `CancelIcon`/`MicrophoneIcon`/`TranscriptionIcon` confirmados sem uso (`grep` fora de `src/components/icons/` sem acerto), como D4 afirma; commit `style(ui): ...` com trailer `Assisted-by: Claude Code` e sem `Co-Authored-By`.
- O6 (artefato, cosmética): `checks.md` declara "13 checks in 1 slice", mas define C1..C14 (14). Nenhum check ficou sem prova; a contagem do cabeçalho está errada.
- Fora do veredito, como `checks.md` declara: captura de tela claro/escuro no Windows (`TODO(windows)`).

## Gate

- `bun src/styles/tokens.test.ts` - 13 checks impressos (`C1 ok`..`C13 ok`), exit 0
- `bun src/overlay/pill.test.tsx` - "pill: all assertions passed", exit 0
- `git diff --exit-code origin/main...HEAD -- src/overlay/Pill.css` exit 0 · `-- src/i18n` exit 0
- `bun run lint` 0 · `bunx prettier --check .` 0 · `bun run check:translations` 0 · `bunx tsc --noEmit` 0 · `bun run build` 0 · os 2 greps em `dist/assets/main-*.css` 0 · `scripts/check-brand.sh` 0
