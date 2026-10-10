# ui-tinta verification

**Verdict**: PASS
**Profile**: light
**Diff range**: eb0482c..06dd171 (feature inteiro: S1 = PR 1 `eb0482c..6dff00f`, S2 = PR 2 `6dff00f..bc612ad`, S3 = PR 3 `bc612ad..06dd171`, `style/ui-fluent-controls...HEAD`)
**Round**: 2 - scoped (rodada 1 completa em `85fa1dc`, FAIL só em C25; a correção `85fa1dc..06dd171` muda `shell.test.tsx` (C25), `ModelStatusButton.tsx` (`type="button"`), as provas de C13 e C24 e D17 em `checks.md`, e este relatório. C25, C13 e C24 verified at `06dd171`; todas as provas re-executadas em `06dd171`; o resto das citações carried from `85fa1dc`, S1 de `c0ed440`, S2 de `f109ade`)
**Verifier**: independent sub-agent (author != verifier); sub-agente novo, não escreveu nenhum dos três PRs, rodou cada prova em `85fa1dc` (rodada 1) e de novo em `06dd171` (rodada 2)

Escopo: os 34 checks de `checks.md` (perfil light). Não há `plan.md`; a fonte é a Parte 3 de `fala-research/research/21-ui-direcao-e-diagnostico.md` (3.3, 3.4, 3.6, 3.9 e a linha do PR 3 na 3.10) mais as decisões do dono (sem cor de marca; Início = só a lista, sem estatísticas; janela 960x640 com mínimo 720x520; whats-new e update-checker escondidos com o updater desligado).

Veredito em uma linha: na rodada 1 (`85fa1dc`) a prova de C25 não discriminava duas cláusulas e o relatório deu FAIL; na rodada 2 (`06dd171`) a prova lê as classes no próprio elemento, os dois mutantes da rodada 1 e um terceiro novo são mortos, e os 34 checks estão provados.

## Binding sources

Perfil light: o passo 1 não é obrigatório. Li 3.3, 3.4, 3.6, 3.9 e 3.10 e comparei com C25..C34.

| Source | Opened | Contradiction | Uncovered |
| --- | --- | --- | --- |
| 3.3 arquitetura (destinos Início, Dicionário, Configurações; rodapé do rail com bolinha e Configurações; Modelos, Pós-processamento, Depuração e Sobre saem do primeiro nível) | yes - `21-ui-direcao-e-diagnostico.md` linhas 198-212 | - | - |
| 3.4 layout (960x640, mínimo 720x520; rail 220/48 px abaixo de 840 px; wordmark 20 px semibold com a pill; barra de 3 px e fundo sutil) | yes - linhas 214-219 | - | - |
| 3.6 linhas `Sidebar`, `App.tsx`, `footer`/`model-selector`, `whats-new`/`update-checker`, ícones lucide 20 px traço 1.5 | yes - linhas 226-242 | - | - |
| 3.10 linha do PR 3 | yes - linha 316 | - | - |

Notas (não mudam o veredito sob light): (a) a 3.6 diz que o seletor de modelo sai da casca e vai para Configurações › Avançado; D13 mantém o `ModelSelector` montado no rodapé do rail como variante `status` (sem dropdown) para não duplicar os listeners, o que cumpre a 3.3 ("bolinha de estado"). (b) A 3.4 decide também o cabeçalho de página (título 28/36 semibold, ações à direita), a coluna de leitura de no máximo 720 px e margens de 24 px (16 px na largura mínima); nenhum check cobre esses três. O código usa `max-w-3xl` e `px-6`/`max-[840px]:px-4` (`App.tsx:366`), mas com a raiz de 14 px do PR 1 o rem vale 14 px: `--container-3xl:48rem` dá 672 px e `px-6` dá 21 px, não 24 (ver O4). (c) A 3.3 ordena Configurações por uso; D11 adia, Geral e Avançado mantêm o conteúdo herdado.

## Checks

Provas: `T1` = `bun src/styles/tokens.test.ts` (exit 0, `C1 ok`..`C13 ok`); `T2` = `bun src/components/ui/controls.test.tsx` (exit 0, `C15 ok`..`C24 ok`); `T3` = `bun src/components/shell.test.tsx` (exit 0, `C25 ok`..`C32 ok`, arquivo novo neste PR). Citações de `T3` são de `src/components/shell.test.tsx`. Nos regexes citados, " ou " está no lugar do `|`. O PR 3 não toca `controls.test.tsx`, `src/App.css`, `src/styles/theme.css` nem `src/overlay/`; em `tokens.test.ts` só muda o fim do bloco C13 (`:291-296`), então as citações de S1 e S2 continuam valendo.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | 13 pares claro/escuro com hex da 3.7; `rec`; `me` | `T1` em `85fa1dc`, `C1 ok` | carried from `c0ed440`: `src/styles/tokens.test.ts:70`, `:75`, `:77` `"#e5322d"`, `:78` `"var(--color-ok)"` | PASS |
| C2 | 4 blocos de tema apontam os 13 tokens | `T1`, `C2 ok` | `src/styles/tokens.test.ts:90` - `assert.equal(map.get(\`--color-${token}\`), \`var(--${mode}-color-${token})\`)` | PASS |
| C3 | 8 aliases; nenhum hex rosa em `src/` | `T1`, `C3 ok` | `src/styles/tokens.test.ts:111`, `:117`, `:126` - `assert.ok(!PINK.test(...))` sobre todo `src/` (inclui os 7 arquivos novos do PR 3) | PASS |
| C4 | contraste maior ou igual a 4,5:1 | `T1`, `C4 ok` | `src/styles/tokens.test.ts:146` - `assert.ok(ratio >= 4.5, ...)`; `:150` | PASS |
| C5 | raiz 14/20, pilhas Segoe, sem fonte embutida | `T1`, `C5 ok` | `src/styles/tokens.test.ts:158-175` | PASS |
| C6 | rampa de 11 degraus | `T1`, `C6 ok` | `src/styles/tokens.test.ts:195-199` | PASS |
| C7 | raios 4/4/4/8; CSS gerado | `T1` `C7 ok`; `bun run build` exit 0 e os dois `grep -o` em `dist/assets/main-BCfaEfX6.css` com acerto, exit 0 | `src/styles/tokens.test.ts:207` - `assert.equal(themeAt.get("--radius-lg"), "4px")` | PASS |
| C8 | foco duplo do Fluent | `T1`, `C8 ok` | `src/styles/tokens.test.ts:222-223` | PASS |
| C9 | 100 ms, `--ease-fluent`, reduced-motion | `T1`, `C9 ok` | `src/styles/tokens.test.ts:230-231`, `:235-236` | PASS |
| C10 | janela `surface-0`; wordmark em `text` | `T1`, `C10 ok` | `src/styles/tokens.test.ts:240-245` | PASS |
| C11 | acento Live; `Pill.css` intacto; pill verde | `T1` `C11 ok`; `git diff --exit-code origin/main...HEAD -- src/overlay/Pill.css` exit 0; `bun src/overlay/pill.test.tsx` exit 0 ("pill: all assertions passed") | `src/styles/tokens.test.ts:249-250` | PASS |
| C12 | Toaster só com tokens novos | `T1`, `C12 ok` (o PR 3 mexe em `App.tsx`, mas não no bloco do Toaster) | `src/styles/tokens.test.ts:263`, `:265-269` | PASS |
| C13 | sem `text-white` sobre tinta; Badge; toggle; item ativo da navegação; PR 1 sem string | `T1` `C13 ok`; `git diff --exit-code origin/main...style/ui-ink-tokens -- src/i18n` exit 0 em `06dd171` (a prova de `checks.md` agora fixa essa ponta; F3 resolvida) | `src/styles/tokens.test.ts:273-282` (varre todo `.tsx`), `:283-290`, `:293-295` - `const navFile = sources.find((p) => /components\/(Sidebar ou Rail)\.tsx$/.test(p)); assert.ok(navFile, ...); assert.doesNotMatch(readFileSync(navFile, "utf8"), /bg-logo-primary\/80/)`. D16 julgada abaixo (legítima); falta 6 prova que lê `Rail.tsx` | PASS |
| C14 | seis portões | `bun run lint` 0 · `bunx prettier --check .` 0 · `bun run check:translations` 0 · `bunx tsc --noEmit` 0 · `bun run build` 0 · `scripts/check-brand.sh` 0, todos em `85fa1dc` | `package.json` scripts; metade `cargo fmt` de `format:check` não rodada (cargo proibido nesta sessão; O6) | PASS |
| C15 | `SettingsGroup` Fluent | `T2`, `C15 ok` | carried from `f109ade`: `src/components/ui/controls.test.tsx:52`, `:54`, `:56-57`, `:58-61`, `:69`, `:76` | PASS |
| C16 | `SettingContainer` nos 2x2 modos | `T2`, `C16 ok` | `controls.test.tsx:82-83` (laço), `:95-99`, `:100-104`, `:105`, `:106-109`, `:113` | PASS |
| C17 | 56/68 px, ícone 20 px, cartão agrupado ou sozinho | `T2`, `C17 ok` | `controls.test.tsx:124`, `:130-131`, `:143-147`, `:148`, `:154-155` | PASS |
| C18 | `Button` 7 variantes, 3 tamanhos | `T2`, `C18 ok` | `controls.test.tsx:163-172`, `:179`, `:181-185`, `:188`, `:189-198`, `:205` | PASS |
| C19 | `ToggleSwitch` Fluent | `T2`, `C19 ok` | `controls.test.tsx:222-233`, `:235` | PASS |
| C20 | `Input` | `T2`, `C20 ok` | `controls.test.tsx:242`, `:253`, `:255`, `:258-260` | PASS |
| C21 | `selectStyles` | `T2`, `C21 ok` | `controls.test.tsx:270-281` | PASS |
| C22 | `Dropdown` | `T2`, `C22 ok` | `controls.test.tsx:298`, `:300` (prova no nível do fonte; O2 de S2) | PASS |
| C23 | `Slider` | `T2`, `C23 ok` | `controls.test.tsx:326`, `:328`, `:329` | PASS |
| C24 | 8 arquivos sem nomes herdados; PR 2 não toca telas nem strings; chamadores compilam; portões verdes | `T2` `C24 ok`; `git diff --exit-code style/ui-ink-tokens...style/ui-fluent-controls -- src/components/settings src/i18n` exit 0 em `06dd171` (a prova de `checks.md` agora fixa essas pontas; F3 resolvida); `tsc`, lint, prettier, traduções, build, brand, `T1` e pill exit 0 | `controls.test.tsx:334-339` sobre `ROW_FILES` (`:33-42`) | PASS |
| C25 | rail: `home`, `dictionary` no topo, `settings` após `mt-auto`; `<nav aria-label="rail.navigation">` `w-[220px]`/`max-[840px]:w-[48px]`; rótulos `max-[840px]:hidden`; wordmark "Fala" `text-subtitle font-semibold text-text` com a pill (`fill-black`/`fill-white`); ícones 20 px traço 1.5 | `T3` em `06dd171`, `C25 ok` | verified at `06dd171`: `:49-52` - `assert.deepEqual(RAIL_ITEMS.map((item) => item.id), ["home", "dictionary"])`; `:54` `<nav` com `aria-label="rail.navigation"`; `:55-57` larguras; `:62` - `assert.ok(home > 0 && home < dict && dict < footer && footer < settings)`; `:64-74` - `const wordmark = html.match(/<span class="([^"]*)">Fala<\/span>/)` e `assert.ok(wordmarkClasses.includes(cls), ...)` para `text-subtitle`, `font-semibold`, `text-text`, `max-[840px]:hidden` no próprio span; `:75` pill; laço `:76-90` por destino: `:82-84` - `assert.ok(classes(label[0]).includes("max-[840px]:hidden"), ...)` no span do rótulo; `:88` - `assert.match(icon, /^<svg[^>]*width="20"/)` e `:89` - `assert.match(icon, /stroke-width="1.5"/)` no primeiro `<svg>` depois do tag do próprio botão. Código: `Rail.tsx:15-18`, `:75-76`, `:96-97`, `:101`, `:115`. Rodada 1 (`85fa1dc`): FAIL, as classes eram lidas no HTML inteiro e as faltas 7 e 9 não eram mortas; em `06dd171` as duas e a falta 10 são mortas | PASS |
| C26 | só o destino ativo tem `aria-current="page"`, `bg-text/5`, `before:w-[3px]`, `before:bg-accent`; nenhum cita `logo-primary` | `T3`, `C26 ok` | `:96-116`, laço 3x3 sobre o tag de abertura de cada botão: `:102-106` - `assert.equal(tag.includes('aria-current="page"'), isActive, ...)`; `:107-113` - `assert.equal(tagClasses.includes(cls), isActive, ...)`; `:114` - `assert.ok(!tag.includes("logo-primary"))`. `classes()` separa por espaço, então `hover:bg-text/5` do inativo não conta como `bg-text/5`. Código: `Rail.tsx:66`, `:71` | PASS |
| C27 | abre em Início; `HomePage` com `<HistorySettings />`, `DictionaryPage`; sem `Sidebar`/`Footer`/`SECTIONS_CONFIG`; arquivos removidos | `T3`, `C27 ok` | `:120` - `assert.match(appTsx, /useState<RailDestination>\("home"\)/)`; `:121-122`; `:123` - `assert.doesNotMatch(appTsx, /Sidebar ou Footer ou SECTIONS_CONFIG/)`; `:124-128` - `!existsSync(...)` para `Sidebar.tsx` e `footer`; `:129`. Código: `App.tsx:51`, `:369-370`; `HomePage.tsx` | PASS |
| C28 | `visibleAdvancedPages` nas 4 combinações e `null`; `resolveSettingsView` para as 6 visões e os 2 casos inalcançáveis; `SettingsPage` com tablist, tabs, `aria-selected`, 3 abas e as 5 telas | `T3`, `C28 ok` | `:135-147` - `assert.deepEqual(visibleAdvancedPages({ post_process_enabled: post, debug_mode: debug }), pages, ...)` sobre as 4; `:148` `null` dá `["models"]`; `:151-167` - `assert.deepEqual(resolveSettingsView(view, settings), { current, tab, subPage }, view)` sobre 8 casos (6 visões alcançáveis e `postprocessing`/`debug` inalcançáveis caindo para `advanced`); `:169-171` papéis ARIA; `:172-178` chaves das 3 abas; `:179-186` as 5 telas por `current`. Código: `settingsNav.ts:31-60`, `SettingsPage.tsx:90`, `:100-101`, `:115-166` | PASS |
| C29 | `DictionaryPage` com `<CustomWords grouped />`; `AdvancedSettings` sem `CustomWords` | `T3`, `C29 ok` | `:191` - `assert.match(read("./DictionaryPage.tsx"), /<CustomWords grouped \/>/)`; `:192-195` - `assert.doesNotMatch(read("./settings/advanced/AdvancedSettings.tsx"), /CustomWords/)`. Código: `DictionaryPage.tsx:17` | PASS |
| C30 | rodapé com `<ModelSelector variant="status" onOpen={onOpenModels} />`; ramo `status` chama `onOpen?.()`, sem `ModelDropdown`, `compact`; botão compacto sem seta e texto `max-[840px]:sr-only`; `onOpenModels` vai a Configurações › Modelos | `T3`, `C30 ok` | `:203-209` - fatia do ramo `status` com `includes("onOpen?.()")`, `!includes("ModelDropdown")`, `includes("compact")`; `:211` - `assert.match(statusButton, /\{!compact && \(/)`; `:212` `max-\[840px\]:sr-only`; `:214-217` linha do `Rail`; `:218-221` - `assert.match(appTsx, /setDestination\("settings"\);\s*setSettingsView\("models"\);/)`. Código: `ModelSelector.tsx:253-264`, `ModelStatusButton.tsx:65`, `:70`, `Rail.tsx:117`, `App.tsx:361-362` | PASS |
| C31 | `UPDATER_ENABLED` é `false` e condiciona `WhatsNewGate`, `ShowWhatsNewOnUpdate`, `WhatsNewPreview`, `UpdateChecksToggle` | `T3`, `C31 ok` | `:227` - `assert.equal(UPDATER_ENABLED, false)` (valor importado, não texto); `:228-231`, `:232-235`, `:237`, `:238` - `/\{UPDATER_ENABLED && \(\s*<.../` nos três arquivos. Código: `lib/updater.ts:4`, `App.tsx:348`, `AboutSettings.tsx:47`, `DebugSettings.tsx:36`, `:46` | PASS |
| C32 | strings novas em pt e en; sem bloco `sidebar` | `T3` `C32 ok`; `bun run check:translations` exit 0 ("PT: All keys present") | `:244-256` (11 valores pt), `:257-262` (4 valores en), `:267-270` - `assert.equal(get(pt, key), value, ...)`; `:271-279` - as 4 descrições são `string` nos dois; `:280-281` - `assert.equal(pt.sidebar, undefined)`, `en.sidebar`. Os en `rail.navigation`, `tabs.general`/`about`, `advancedPages.*` não são comparados por valor (O5) | PASS |
| C33 | janela 960x640, mínimo 720x520; crate compila | `grep -n "inner_size(960.0, 640.0)"` e `grep -n "min_inner_size(720.0, 520.0)"` em `apps/desktop/src/lib.rs`, exit 0 (linhas 960 e 961). `cargo check -p fala`: rodado pelo autor sob o lock de build (`CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=1`), com `lib.rs` tocado antes para não reaproveitar artefato de outro worktree: `Checking fala v0.1.0 (.../apps/desktop)`, `Finished`, exit 0 | `apps/desktop/src/lib.rs:960` - `.inner_size(960.0, 640.0)`; `:961` - `.min_inner_size(720.0, 520.0)`. `git diff --numstat bc612ad HEAD -- apps/desktop/src/lib.rs` = `2 2`, e é o único `.rs` do PR 3: exatamente as duas linhas de tamanho | PASS |
| C34 | dez portões e provas anteriores verdes no PR 3 | `bun run lint` 0 · `bunx prettier --check .` 0 · `bun run check:translations` 0 · `bunx tsc --noEmit` 0 · `bun run build` 0 · `scripts/check-brand.sh` 0 ("no Handy branding outside the allowlist") · `T1` 0 · `T2` 0 · `bun src/overlay/pill.test.tsx` 0 · `bun src/components/settings/history/historyModel.test.ts` 0 ("history: all assertions passed"; os `error: ipc` impressos são o caso de falha simulado do próprio teste) | `tokens.test.ts:293-295` (C13 lendo o rail, D16); lint inclui `i18next/no-literal-string` | PASS |

### D16: a adaptação de C13 é legítima

A asserção antiga era `assert.doesNotMatch(read("../components/Sidebar.tsx"), /bg-logo-primary\/80/)`; a nova (`tokens.test.ts:293-295`) procura `Sidebar.tsx` ou `Rail.tsx` entre os fontes, falha alto se nenhum existir (`assert.ok(navFile, ...)`) e aplica o mesmo `doesNotMatch`. A força é a mesma, não menor: a sidebar em `bc612ad` já usava `bg-logo-primary/10` (`git show bc612ad:src/components/Sidebar.tsx`, linha 108), então a asserção antiga também só provava a ausência do `/80`. A falta 6 (pôr `bg-logo-primary/80` no item ativo do `Rail.tsx`) faz `T1` sair 1 em C13, então a asserção lê de fato o rail. O que a 3.4 pede do item ativo (barra de 3 px e fundo sutil, sem preenchimento colorido) é provado com mais força em C26 (`shell.test.tsx:87-94`, por botão e por estado). A varredura geral de `text-white` sobre tinta (`:273-282`) continua inalterada.

## Coverage

Perfil light: recompute não obrigatório; refiz os conjuntos de S3 a partir do código e da 3.3/3.6. S1 carried from `c0ed440`, S2 carried from `f109ade` (0 membro sem prova).

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| destinos do rail (3) | 3.3 (fase 1) e `RailDestination` em `Rail.tsx:6` | `home`, `dictionary`, `settings`: C25 ordem (`shell.test.tsx:62`), C26 laço 3x3 | - |
| visões de Configurações (6) | `SettingsTab` e `AdvancedPage` em `settingsNav.ts:4-8` | 6 casos alcançáveis em `shell.test.tsx:152-157` | - |
| `post_process_enabled` x `debug_mode` (4) | `NavSettings` em `settingsNav.ts` | `shell.test.tsx:135-147`, mais `null` em `:148` | - |
| páginas que saem do primeiro nível (4) | 3.3 "Sai do primeiro nível" | Modelos, Pós-processamento, Depuração: C28 (subpáginas); Sobre: C28 (aba) | - |
| superfícies do updater na UI (4) | `grep -rn "WhatsNew ou UpdateCheck"` em `src/` | as 4 de C31; `UpdateChecker` some com o footer (C27). Fora do conjunto: o item "Verificar atualizações" do tray (O1) | - |
| chaves i18n novas (15 folhas por idioma) | diff de `src/i18n` | 11 valores pt e 4 en por igualdade, 4 descrições por tipo, demais en por `check:translations` | - |

## Faults injected

Perfil light: não obrigatório; o briefing pediu. Mutação direta na árvore com cópia do arquivo no scratchpad, restaurada com `cp` logo depois e conferida com `cmp`; `git status --porcelain` vazio antes da primeira e depois da última restauração, e `T1`/`T3` verdes de novo. As faltas 7 a 9 passam do teto de cinco do procedimento de propósito: depois de ver que C25 lê classes no HTML inteiro, testei se cada cláusula de C25 discrimina.

| Mutation | Location | Killed |
| --- | --- | --- |
| 1. destino inicial `"home"` para `"settings"` | `src/App.tsx:51` | yes - `T3` exit 1 em C27 (`:120`) |
| 2. `debug` visível sem `debug_mode` (`return true`) | `src/components/settingsNav.ts:36` | yes - `AssertionError: post=false debug=false`, `actual: ["models", "debug"]` (C28) |
| 3. barra de 3 px removida (`before:w-[3px]`) | `src/components/Rail.tsx:71` | yes - `AssertionError: home/home before:w-[3px]` (C26) |
| 4. `<CustomWords .../>` de volta em Avançado › Transcrição | `src/components/settings/advanced/AdvancedSettings.tsx:52` | yes - `doesNotMatch /CustomWords/` (C29) |
| 5. `UPDATER_ENABLED = true` | `src/lib/updater.ts:4` | yes - `actual: true, expected: false` (C31) |
| 6. item ativo do rail com `bg-logo-primary/80` | `src/components/Rail.tsx:71` | yes - `T1` exit 1 em C13 (`:295`), prova da adaptação D16 |
| 7. rótulo do botão sem `max-[840px]:hidden` (rótulo fica visível com o rail recolhido) | `src/components/Rail.tsx:76` | yes em `06dd171` - `AssertionError: label home hides when collapsed` (C25, `shell.test.tsx:82-84`); em `85fa1dc` não era morto (`T3` exit 0) |
| 8. ícones `size={24} strokeWidth={2}` | `src/components/Rail.tsx:75` | yes - `doesNotMatch stroke-width="1.5"` (C25, `:69`) |
| 9. wordmark sem `font-semibold` e em `text-text-2` | `src/components/Rail.tsx:101` | yes em `06dd171` - `AssertionError: wordmark font-semibold` (C25, `shell.test.tsx:73`); em `85fa1dc` não era morto (`T3` exit 0) |
| 10. (rodada 2) ícone real `size={24} strokeWidth={2}` e um `<svg width="20" stroke-width="1.5" />` de isca depois do rótulo, no mesmo botão | `src/components/Rail.tsx:75-76` | yes - `AssertionError: icon home 20 px` (C25, `shell.test.tsx:88`): a asserção lê o primeiro `<svg>` do próprio botão |

## Gaps

Nenhum em `06dd171`. Gap da rodada 1, resolvido (texto original abaixo):

1. **C25 - prova não discriminava** (resolvido em `06dd171`) (mutantes 7 e 9 sobreviveram) - `src/components/shell.test.tsx:64-66` e `:68`. As classes do wordmark e o `max-[840px]:hidden` dos rótulos são buscados no HTML inteiro do rail, onde outros elementos as têm. Correção sugerida, só no teste: ler o `<span ...>Fala</span>` e conferir as três classes nele; para cada `rail.<id>`, conferir `max-[840px]:hidden` no `<span>` do rótulo dentro do botão. O código já cumpre (`Rail.tsx:76`, `:101`); nenhum arquivo de produção precisa mudar.

## Re-verification (round 2, `85fa1dc..06dd171`)

O autor emendou o commit do PR 3. `git diff --stat 85fa1dc 06dd171`: `checks.md` (5 linhas: D17 e as provas de C13 e C24), `verification.md`, `ModelStatusButton.tsx` (1 linha, `type="button"`) e `shell.test.tsx` (bloco C25). Escopo pelo diff: C25 (prova nova), C13 e C24 (provas reescritas), C30 (lê `ModelStatusButton.tsx`) e O1/O3.

- `bun src/components/shell.test.tsx` em `06dd171`: `C25 ok`..`C32 ok`, exit 0. C30 continua verde: a linha nova não toca as asserções de `shell.test.tsx:211-213`.
- Faltas re-injetadas: 7 (rótulo sem `max-[840px]:hidden`) e 9 (wordmark sem semibold, em `text-2`) agora fazem `T3` sair 1; falta nova 10 na superfície criada pela correção (ícone real a 24 px com um `<svg>` isca de 20 px no mesmo botão) também. Restauradas com `cp` e `cmp`; `git status --porcelain` vazio; `T3` verde de novo.
- C13 e C24: as provas fixadas de `checks.md` rodadas literalmente, exit 0.
- Todas as outras provas re-executadas em `06dd171` com o mesmo resultado (seção Gate). Nenhum outro arquivo de produção mudou; as citações de C26..C34 continuam valendo, salvo as linhas de `shell.test.tsx`, que deslocaram +20 depois do bloco C25 (as asserções citadas são as mesmas).
- `cargo check -p fala` (C33): rodado pelo autor sob o lock de build (`CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=1`), com `lib.rs` tocado antes para não reaproveitar artefato de outro worktree: `Checking fala v0.1.0 (.../apps/desktop)`, `Finished`, exit 0. O PR 3 não mudou nenhum `.rs` na rodada 2.

## Findings

- F3 - **resolvida em `06dd171`** (as duas provas de `checks.md` fixam as pontas do PR 1 e do PR 2; ambas exit 0). Texto da rodada 1 (precisão dos checks, não bloqueia): as provas de C13 (`git diff --exit-code origin/main...HEAD -- src/i18n`) e de C24 (`style/ui-ink-tokens...HEAD -- src/components/settings src/i18n`) são relativas a `HEAD` e saem 1 em `85fa1dc`, porque o PR 3 muda essas pastas por desenho. As afirmações são sobre o PR 1 e o PR 2 e continuam provadas com as pontas fixadas (`origin/main...style/ui-ink-tokens` e `style/ui-ink-tokens...style/ui-fluent-controls`, ambos exit 0). Sugestão: reescrever as duas provas em `checks.md` com essas pontas. O mesmo vale para C11 (`Pill.css`), que hoje passa só porque nenhum PR toca o arquivo.
- O1 - **registrada como D17 em `06dd171`** (follow-up no corpo do PR; esconder o item é mudança em `apps/desktop` além das duas linhas de tamanho do PR 3; sem mudança de código, decisão delegada aceitável). Texto da rodada 1 (comportamento perdido, baixa): o tray ainda mostra "Verificar atualizações" quando `update_checks_enabled` (padrão `true`, `apps/desktop/src/settings.rs:692`, `tray.rs:364`, `:527-531`), e o clique emite `check-for-updates` (`lib.rs:296-300`). O único ouvinte era o `UpdateChecker` do footer (`src/components/update-checker/UpdateChecker.tsx:67`), que saiu com o PR 3; e o `UpdateChecksToggle` que desligaria o item está escondido (C31). O item agora abre a janela e não faz nada. D14 e a 3.6 não enumeram o tray; fica para quem religar o updater (ou esconder o item no `tray.rs`).
- O2 (comportamento herdado preservado, conferido): os listeners do `ModelSelector` (estado do modelo e seleção automática ao fim do download, `ModelSelector.tsx:58-150`) rodam antes do ramo `status` (`:253`) e o componente é montado uma única vez (só `Rail.tsx:117` o importa); o atalho Ctrl+Shift+D continua em `App.tsx:106-129`, sempre montado, e a Depuração aparece em Avançado › Mais opções quando liga (C28), com volta para Avançado se desligar estando nela; a prévia do onboarding chega ao `DebugSettings` por `SettingsPage.tsx:163-164` e `App.tsx:375`; a versão do app, que estava no footer, continua em Sobre (`AboutSettings.tsx`).
- O3 (acessibilidade, baixa): recolhido, cada botão do rail mantém nome acessível (`aria-label` e `title`, `Rail.tsx:67-68`) e o estado do modelo mantém o texto em `sr-only`; o `nav` tem rótulo. Pontos menores: as abas de Configurações não têm `aria-controls`/`tabpanel` nem navegação por setas (padrão WAI-ARIA de tabs); o `ModelStatusButton` herdado não tinha `type="button"` (**resolvido em `06dd171`**, `ModelStatusButton.tsx:56`) e seu `title` é o literal em inglês `Model status: ...` (`ModelStatusButton.tsx:58`), agora visível no rodapé do rail.
- O4 (arranjo da 3.4 sem check, baixa sob light): cabeçalho de página 28/36 com ações à direita (`PageHeader.tsx`), coluna de leitura de 720 px e margens de 24/16 px não têm check. Com a raiz de 14 px (C5), `max-w-3xl` vale 672 px e `px-6` vale 21 px.
- O5 (precisão de C32, baixa): só 4 valores en são comparados; os demais en são cobertos apenas por `check:translations` (existência da chave).
- O6 (prova parcial, por restrição do briefing): `format:check` = `prettier --check . && cargo fmt --all -- --check`; rodei só a metade prettier. O PR 3 muda apenas dois literais numéricos em `lib.rs`, sem mudar a forma das linhas. `cargo check -p fala` (C33): run by the orchestrator, result pending/attached.
- AGENTS.md: nenhum literal novo em JSX (o wordmark "Fala" vem da constante `WORDMARK`, `Rail.tsx:26`, nome do produto, não traduzível; lint verde); nenhum `eslint-disable`, `cfg(` ou "Handy" adicionado no diff de `src/` e `apps/`; `check-brand.sh` verde; o diff não reformata código alheio (no `ModelStatusButton` o `<svg>` só ganhou o `{!compact && (` em volta e, na rodada 2, o botão ganhou `type="button"`). Nota cosmética: D17 aparece antes de D16 em `checks.md`; a única mudança em Rust são as duas linhas de tamanho; commit `feat(ui): open on dictations with a rail instead of settings` (60 caracteres) com corpo em inglês, `Assisted-by: Claude Code`, sem `Co-Authored-By`; `checks.md` traz D16 para a mudança no teste do PR 1.
- Fora do veredito, como `checks.md` declara: captura de tela claro/escuro no Windows (`TODO(windows)`).

## Gate

Tudo em `06dd171` (rodada 2; a rodada 1 em `85fa1dc` deu o mesmo resultado), árvore limpa antes e depois (`git status --porcelain` vazio; só este relatório muda):

- `bun src/components/shell.test.tsx` - 8 checks impressos (`C25 ok`..`C32 ok`), exit 0
- `bun src/components/ui/controls.test.tsx` - 10 checks (`C15 ok`..`C24 ok`), exit 0
- `bun src/styles/tokens.test.ts` - 13 checks (`C1 ok`..`C13 ok`), exit 0
- `bun src/overlay/pill.test.tsx` - "pill: all assertions passed", exit 0
- `bun src/components/settings/history/historyModel.test.ts` - "history: all assertions passed", exit 0
- `bun run lint` 0 · `bunx prettier --check .` 0 · `bun run check:translations` 0 · `bunx tsc --noEmit` 0 · `bun run build` 0 · os 2 greps de C7 em `dist/assets/main-DO8UcZ4a.css` 0 · `scripts/check-brand.sh` 0
- C33: os 2 greps em `apps/desktop/src/lib.rs` 0; `cargo check -p fala` exit 0 (autor, sob o lock)
- `git diff --exit-code origin/main...HEAD -- src/overlay/Pill.css` 0 · `origin/main...style/ui-ink-tokens -- src/i18n` 0 · `style/ui-ink-tokens...style/ui-fluent-controls -- src/components/settings src/i18n` 0 (as provas de `checks.md` usam agora essas pontas)
