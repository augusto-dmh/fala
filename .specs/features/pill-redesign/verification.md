# pill-redesign verification

**Verdict**: PASS
**Profile**: ui
**Diff range**: 881e77c..1bd7f3c (`origin/main..HEAD`, um commit: `1bd7f3c feat(ui): show dictation as a small black pill with ten bars`)
**Round**: 2 - scoped (C7, C11, C14 e o que o diff da correção tocou, verified at 1bd7f3c; o resto carried from 5b40fa0, rodada 1 com veredito FAIL)
**Verifier**: independent sub-agent (author != verifier), sem contexto herdado; rodada 2 leu verify.md, checks.md, este relatório e `git diff 5b40fa0 1bd7f3c`

Rodada 2: os dois achados da rodada 1 estão fechados. A diferença entre as rodadas é só
`src/overlay/pill.test.tsx` (`git diff 5b40fa0 1bd7f3c --stat`: 1 arquivo, 24 inserções, 7
remoções); nenhum arquivo de produção mudou. As provas rodaram de novo em 1bd7f3c (`C1 ok`..`C14 ok`,
exit 0; lint, prettier, traduções e build exit 0). Os quatro mutantes da rodada 2 morreram, cada um
num check diferente. Detalhes em `## Round 2 (scoped: C7, C11, C14)`.

Cada seção abaixo diz de onde vem: `verified at 1bd7f3c` (refeito nesta rodada) ou
`carried from 5b40fa0` (rodada 1, não tocado pelo diff da correção). Como `pill.test.tsx` mudou, todas
as citações nesse arquivo foram atualizadas para as linhas de 1bd7f3c.

### Achados da rodada 1 (5b40fa0), mantidos como registro

Veredito da rodada 1: FAIL. Os 15 checks estavam provados no HEAD de então, com prova verde e
evidência localizada. O FAIL veio de dois achados que verify.md manda tratar como reprovação:

1. **Mutante sobrevivente (F5).** Com `holdToTalk={false}` fixo em `src/overlay/RecordingOverlay.tsx:300`
   (a pill nunca fica vermelha), o script imprimia `C1 ok`..`C14 ok` e saía 0. A asserção de C14
   em 5b40fa0, `src/overlay/pill.test.tsx:265` `assert.ok(/holdToTalk=\{[^}]*\}/.test(overlaySource))`,
   aceitava qualquer expressão na prop. Pela mesma leitura (não injetado na rodada 1, o teto de 5
   faltas já tinha sido atingido): trocar os ramos do ternário do `label` em
   `RecordingOverlay.tsx:303-307` também passaria, porque `pill.test.tsx:226-227` (em 5b40fa0) só
   conferia que as duas strings `t(...)` aparecem no fonte.
   **Fechado na rodada 2:** F5 morre em C14 (`pill.test.tsx:277`), a troca do ternário morre em C11
   (`pill.test.tsx:234-239`).
2. **Arranjo do estado processando sem check.** O esboço e o AC 7 dizem que processando é a cápsula
   com as 10 barras paradas e nada mais ("sem ícone, sem texto"; o AC 7 diz "sem spinner"). Em
   5b40fa0, C2 provava "10 filhos `<i>` e nada mais" só gravando (`pill.test.tsx:86`), e C7 provava
   processando com `text(html) == ""`, ausência de `<svg`/`<button` e as alturas das `<i>`
   (`:162-164`). Um `<span className="sspinner" />` ou um `<img>` dentro da `Pill` em modo
   processando passaria C7 e C8.
   **Fechado na rodada 2:** `onlyBars(html)` em C7 (`pill.test.tsx:171`, helper em `:43-48`) mata os
   dois mutantes.

## Binding sources

Carried from 5b40fa0. A linha do esboço foi reaberta na rodada 2 só para o elemento que a rodada 1
deixou descoberto (processando, "nada mais"); o diff da correção não tocou a interface.

| Source | Opened | Contradiction | Uncovered |
| --- | --- | --- | --- |
| `fala-research/pitches/fase-1-ditado-windows.md:19` (esboço da pill) | yes (rodada 1) - linha lida: "cápsula preta ~84×30 px, 10 barras simétricas, vermelha em push-to-talk, processando com barras paradas e pulso, sem ícone, sem texto" | none | - |
| `fala-research/plans/fase-0-fechamento-auditoria.md:203` (linha E) | yes (rodada 1) - não tocar em `overlay.rs`, desenhar dentro da janela 256×50 existente; estado "Colar" sem gatilho | none - `git diff --stat origin/main..HEAD -- apps/ crates/` vazio; nenhum estado "Colar" em `src/overlay/` (grep `Colar` e `paste` sem resultado) | - |
| `fala-research/plans/fase-1-delta-e-semanas-3-4.md:36` (S4) | yes (rodada 1) - hoje: 256×50, 9 barras, botão X, rótulos `overlay.transcribing`/`processing`, `aria-label="cancel"` literal; muda: redesenho completo | none - o ramo mínimo perdeu as 9 barras, o X e os rótulos (C13); o `aria-label="cancel"` literal segue só no ramo streaming, fora do escopo e sem mudança | - |

Na rodada 1 (5b40fa0), a linha do esboço tinha em `Uncovered`: processando, "só as 10 barras, nada
mais" (arranjo), porque C7 não afirmava os filhos e um `<span>` spinner ou `<img>` passaria. Em
1bd7f3c o elemento está coberto por `src/overlay/pill.test.tsx:171` (`onlyBars(html)` em C7), e os
mutantes R3 e R4 provam que a asserção falha; por isso a célula agora é `-`.

### Comparação elemento a elemento com o esboço

Carried from 5b40fa0, com as linhas "vermelha em push-to-talk" e "sem ícone" refeitas em 1bd7f3c. A
coluna que a rodada 1 chamava de "Resultado" agora se chama "Cobertura" (mesmo conteúdo; o validador
lê qualquer coluna "result" ao lado de "check" como veredito de check).

| Elemento do esboço | Check | Código | Cobertura |
| --- | --- | --- | --- |
| cápsula | C1 (`border-radius: 15px` com altura 30) | `Pill.css:12` | coberto |
| preta | C5 (`.fpill` `#000`), C7 (processando sem `hold`) | `Pill.css:13`, `Pill.tsx:33` | coberto |
| ~84×30 px | C1 | `Pill.css:10-11` | coberto (exato 84×30, compatível com "~") |
| 10 barras | C2 (gravando e armando), C7 (processando) | `Pill.tsx:40-42`, `pillModel.ts:13` | coberto |
| simétricas | C3 (`h[i] == h[9-i]`), C7 (forma fixa espelhada) | `pillModel.ts:50-55`, `:19` | coberto |
| vermelha em push-to-talk | C5 (modelo e classe), C14 (fiação, por texto exato) | `pillModel.ts:36-38`, `RecordingOverlay.tsx:83`, `:300` | coberto; F5 morre em 1bd7f3c |
| processando com barras paradas | C7 (forma fixa), C8 (mesmo HTML com níveis diferentes) | `Pill.tsx:25-29`, `pillModel.ts:19` | coberto |
| processando com pulso | C8 (1,2 s, 1 -> 0,55), C9 (desliga com reduced-motion) | `Pill.css:30-47` | coberto |
| sem ícone | C1 (gravando: sem `svg`, `img`, `button`, `span`, `p`), C7 (processando: `onlyBars`, só 10 `<i>`) | `Pill.tsx:39-43` | coberto nos dois estados (rodada 1: processando parcial) |
| sem texto | C1 (gravando), C7 (processando), C11 (só `aria-label`) | `Pill.tsx:39` | coberto |

Contradições: nenhuma. `hold_or_toggle` vermelho (C5) não contradiz o esboço, que fala só de
push-to-talk e não decide esse modo; o plano registra a escolha em Assumptions e Out of scope
("Distinguir hold de tap"). Processando sempre preta também não contradiz: o esboço chama a cápsula
de preta e só a pinta de vermelho em push-to-talk.

### Cópia e arranjo por estado

Carried from 5b40fa0; linhas de gravando, armando e processando refeitas em 1bd7f3c.

| Estado | Cópia visível | Arranjo decidido | Check |
| --- | --- | --- | --- |
| gravando | nenhuma; `role="status"` + `aria-label` "Gravando"/"Recording" | uma região: a cápsula; dentro dela uma linha de 10 barras centrada nos dois eixos, nada mais | C1, C2 (`pill.test.tsx:93`), C11 |
| armando (antes do `recording-ready`) | nenhuma; mesmo `aria-label` de gravando | igual a gravando, barras em 3 px com opacidade 0,45 | C6, e C2 agora também afirma só 10 `<i>` armando (`pill.test.tsx:94`) |
| processando (`transcribing` e `processing`) | nenhuma; `aria-label` "Processando..."/"Processing..." | a cápsula preta com as 10 barras na forma fixa, nada mais | C7 (`pill.test.tsx:171` `onlyBars`), C8, C11 |
| oculto | nada renderizado | `RecordingOverlay` devolve `null` (`RecordingOverlay.tsx:166`) | C10 |

### Fora de alcance, enumerado por tela

Carried from 5b40fa0. Os checks não trazem cláusula genérica de "fidelidade visual não provada";
também não enumeram o que deixam de fora, então a lista vai aqui. Nada disto é decidido pelo esboço:

- gravando: `gap: 2px` entre barras, largura 3 px e raio 1,5 px de cada barra (`Pill.css:9`, `:22-23`),
  transição de cor de 120 ms e de altura de 60 ms (`:15`, `:25`), tom exato do vermelho (`#e5322d` é
  escolha do plano; o esboço diz só "vermelha"), curva `pow(v, 0.7)` entre 3 e 18 px
  (`pillModel.ts:42`), posição da cápsula na janela 256×50 (centro horizontal, rente à borda pelo
  `.ov-stage` existente, `RecordingOverlay.css:106-116`), fade de 200 ms do `.ov-fade`.
- armando: nada além do que gravando já lista.
- processando: curva `ease-in-out` do pulso (`Pill.css:31`), a forma `[5, 7, 9, 11, 13, ...]` (escolha do plano).
- oculto: nada.
- tipografia: n/a em todas as telas, não há texto visível.

## Checks

Provas re-rodadas em 1bd7f3c numa única invocação de `bun src/overlay/pill.test.tsx` (exit 0, cada
check impresso individualmente, `C1 ok`..`C14 ok`, depois `pill: all assertions passed`). Linhas
citadas em `pill.test.tsx` são as de 1bd7f3c. C2, C7, C11 e C14 verified at 1bd7f3c (asserções novas
ou alteradas); os demais têm só a citação atualizada, julgamento carried from 5b40fa0.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | cápsula 84×30, raio 15, sem texto, ícone ou botão | `bun src/overlay/pill.test.tsx` exit 0, imprime `C1 ok` | `src/overlay/pill.test.tsx:77` `assert.equal(text(html), "")`; `:78-79` sem `<svg`/`<button`/`<img`/`<span`/`<p`; `:84-86` `width: 84px`, `height: 30px`, `border-radius: 15px` | PASS |
| C2 | exatamente 10 `<i>`, linha única centrada | idem, `C2 ok` | `src/overlay/pill.test.tsx:93-94` `onlyBars(...)` gravando e armando, que afirma `inner == parts.join("")` e `parts.length == 10` (`:46-47`); `:96-98` flex/center; `:99-100` sem `flex-wrap`/coluna | PASS (verified at 1bd7f3c) |
| C3 | espelho a partir de `levels[0..4]`, `levels[0]` no centro | idem, `C3 ok` | `src/overlay/pill.test.tsx:109` `h[i] == h[9 - i]`; `:111-117` centro 18, pontas 3, 1/8 2/7 3/6; `:119` cauda 5..15 não muda | PASS |
| C4 | 3 px no silêncio, 18 px no teto, crescente | idem, `C4 ok` | `src/overlay/pill.test.tsx:126-130` 0, -0.5, `[]`, 1, 2.5; `:131-134` crescente estrito | PASS |
| C5 | vermelha em `push_to_talk`/`hold_or_toggle`, preta em `toggle`/ausente | idem, `C5 ok` | `src/overlay/pill.test.tsx:140-143` `isHoldToTalk`; `:144-145` classe `hold`; `:146-147` `#000` e `#e5322d` | PASS |
| C6 | armando: 3 px, classe `arming`, opacidade 0,45 | idem, `C6 ok` | `src/overlay/pill.test.tsx:154-156`; `:158-159` com `ready` some e segue níveis | PASS |
| C7 | processando: classe, sem `hold`, sem texto/svg/button, forma fixa, só as 10 barras; `toPillMode` | idem, `C7 ok` | `src/overlay/pill.test.tsx:167-170`; `:171` `onlyBars(html)` (filhos da raiz == exatamente 10 `<i>`, nada mais); `:172` `deepEqual(bars(html), [5, 7, 9, 11, 13, 13, 11, 9, 7, 5])`; `:173-175` | PASS (verified at 1bd7f3c) |
| C8 | ignora níveis; pulso 1,2 s infinito de 1 a 0,55 | idem, `C8 ok` | `src/overlay/pill.test.tsx:181-184` mesmo HTML; `:187` regex `fpill-pulse 1.2s ... infinite`; `:192-193` | PASS |
| C9 | reduced-motion desliga o pulso | idem, `C9 ok` | `src/overlay/pill.test.tsx:204-205` `animation: none`, `opacity: 1` dentro do `@media` | PASS |
| C10 | streaming não é pill; oculto devolve `null` | idem, `C10 ok`; leitura (rodada 1) | `src/overlay/pill.test.tsx:211` `toPillMode("streaming") == null`; `:214-220`; leitura: `src/overlay/RecordingOverlay.tsx:166` `if (!isVisible) return null;` antes do ramo da pill, `:98-99` `listen("hide-overlay", ...)` com `setIsVisible(false)` | PASS |
| C11 | `role="status"`, `aria-label`, chaves pt/en, `recording` gravando e `processing` processando | idem, `C11 ok`; `bun run check:translations` exit 0 | `src/overlay/pill.test.tsx:228-229`; `:232-233` "Gravando"/"Recording"; `:234-239` regex `pillMode === "recording" ? t("overlay.recording") : t("overlay.processing")` contra o fonte; fonte em `src/overlay/RecordingOverlay.tsx:303-307` | PASS (verified at 1bd7f3c) |
| C12 | cores literais, sem tema | idem, `C12 ok` | `src/overlay/pill.test.tsx:245-247` sem `var(`, `prefers-color-scheme`, `[data-theme`; `:248` barras `#fff` | PASS |
| C13 | ramo mínimo é só `<Pill>` em `.ov-stage`; streaming igual ao `origin/main` | idem, `C13 ok`; diff (rodada 1) | `src/overlay/pill.test.tsx:257-269`; leitura: `src/overlay/RecordingOverlay.tsx:293-309`; ramo streaming idêntico ao de `origin/main` (rodada 1; `RecordingOverlay.tsx` não mudou entre 5b40fa0 e 1bd7f3c) | PASS |
| C14 | lê `shortcut_activation` no `show-overlay` e passa `isHoldToTalk(...)` à pill | idem, `C14 ok`; leitura | `src/overlay/pill.test.tsx:277` `overlaySource.includes("holdToTalk={holdToTalk}")`; `:278-283` `includes("setHoldToTalk(isHoldToTalk(settings.data.shortcut_activation))")`; leitura: `src/overlay/RecordingOverlay.tsx:83` dentro do listener `show-overlay` (`:62`), `:300` `holdToTalk={holdToTalk}` | PASS (verified at 1bd7f3c) |
| C15 | lint, prettier, traduções, build | re-rodado em 1bd7f3c: `bun run lint` exit 0; `bunx prettier --check src/overlay src/i18n` exit 0 ("All matched files use Prettier code style!"); `bun run check:translations` exit 0 ("All 1 languages have complete translations!"); `bun run build` exit 0 ("built in 14.92s") | códigos de saída lidos de `$?` com a saída redirecionada para arquivo; o arquivo alterado (`src/overlay/pill.test.tsx`) está dentro do alvo do prettier e do eslint | PASS (verified at 1bd7f3c) |

## Coverage

Carried from 5b40fa0: o diff da correção não toca nenhuma autoridade (`overlay.rs`, `bindings.ts`,
esboço, `pillModel.ts`, `Pill.tsx`, `Pill.css`, locales). Refeita em 1bd7f3c: a linha "elementos do
esboço", que era a única com membro sem prova, e todas as citações de `pill.test.tsx`.

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| payloads de `show-overlay` (4) | `apps/desktop/src/overlay.rs:614-630` emite `recording`, `streaming`, `transcribing`, `processing` | `recording` C7 (`pill.test.tsx:175`) · `transcribing` C7 (`:173`) · `processing` C7 (`:174`) · `streaming` C10 (`:211`) | - |
| `ShortcutActivation` (3 + ausente) | `src/bindings.ts:1117-1130`: `toggle`, `push_to_talk`, `hold_or_toggle` | `push_to_talk` `pill.test.tsx:140` · `hold_or_toggle` `:141` · `toggle` `:142` · ausente `:143` | - |
| estados da pill (4) | esboço + plano: gravando, armando, oculto, processando | gravando C1/C2 · armando C6/C2 · processando C7/C8 · oculto C10 (leitura `RecordingOverlay.tsx:166`) | - |
| elementos do esboço (8) | `pitches/fase-1-ditado-windows.md:19` | cápsula C1 · preta C5 · ~84×30 C1 · 10 barras C2/C7 · simétricas C3 · vermelha em PTT C5 + C14 (`pill.test.tsx:277`, mata F5) · parado + pulso C7/C8 · sem ícone, sem texto C1 (gravando) e C7 (processando, `pill.test.tsx:171`) | - |
| entradas de altura (4) | `pillModel.ts:41-44` (clamp 0..1, `?? 0`) | silêncio/negativo/ausente `pill.test.tsx:126-128` · 1,0 `:129` · acima de 1 `:130` · crescente `:131-134` | - |
| prontidão da captura (2) | `Pill.tsx:25-29`, `:34` | `ready = false` `pill.test.tsx:153-155` · `ready = true` `:157-159` | - |
| preferência de movimento (2) | `Pill.css:30-47` | padrão C8 `pill.test.tsx:187` · `reduce` C9 `:204-205` | - |
| locales (2) | `ls src/i18n/locales` = `en`, `pt` | `pt` `pill.test.tsx:232` · `en` `:233` | - |
| temas (2) | plano AC 12; tema do overlay vem de `data-theme` e `prefers-color-scheme` (`RecordingOverlay.css:62-89`) | claro e escuro: C12, ausência dos dois ganchos no `Pill.css` (`pill.test.tsx:245-247`) | - |
| removidos do overlay mínimo (4) | `git diff origin/main -- src/overlay/RecordingOverlay.tsx` (linhas `-` do ramo mínimo) | X `cancelBtn` · spinner `workingRow` · rótulo `swork-label` · `scard compact`: C13 `pill.test.tsx:262-269` | - |
| ramos do rótulo (2) | `RecordingOverlay.tsx:303-307` (ternário sobre `pillMode`) | gravando -> `overlay.recording` e processando -> `overlay.processing`: C11 `pill.test.tsx:234-239` (mata a troca dos ramos) | - |

## Test policy rows

Carried from 5b40fa0; a terceira linha re-julgada em 1bd7f3c porque classifica `RecordingOverlay.tsx`,
cuja prova por texto foi a que a correção fortaleceu.

| Row | Files it classifies | Required proof | Expectation met |
| --- | --- | --- | --- |
| decide: 4 eventos -> modo, ativações -> vermelho, níveis -> alturas | `src/overlay/pillModel.ts` | uma no próprio nível: C3, C4, C5, C7, C10 chamam as funções direto | yes - um caso por linha de cada tabela (4 eventos `pill.test.tsx:173-175`, `:211`; 3 ativações + ausente `:140-143`; clamp inferior e superior `:126-130`) |
| decide: modo × hold × ready -> classes e alturas | `src/overlay/Pill.tsx` | render estático por combinação | yes - gravando hold `pill.test.tsx:144`, gravando sem hold `:145`, armando `:153-156`, processando com hold `:165-172` |
| encaminha eventos e settings, sem decisão própria | `src/overlay/RecordingOverlay.tsx` | nenhuma própria; leitura do fonte (C13, C14) e build (C15) | yes - leitura feita em `RecordingOverlay.tsx:62-101`, `:166`, `:237-309`; build exit 0 em 1bd7f3c; as asserções de texto agora fixam a expressão exata (C11 `pill.test.tsx:234-239`, C14 `:277-283`) |

A terceira linha segue cumprida como foi aprovada (sem prova própria, gap de nível declarado em
checks.md). As asserções por texto ficaram exatas: matam F5 e a troca do ternário, mas reprovam também
uma reescrita equivalente (por exemplo `pillMode !== "recording" ? ... : ...`). É fragilidade a favor
da segurança, não um achado.

### Swept existing, relido no código

Carried from 5b40fa0 (`RecordingOverlay.tsx` não mudou entre as rodadas).

- failure modes / dependency failure: `getAppSettings()` dentro de `try/catch` em `RecordingOverlay.tsx:77-87`; se falhar ou vier `status != "ok"`, `holdToTalk` fica com o valor anterior. Confere.
- concurrency: o reset síncrono antes do I/O (`RecordingOverlay.tsx:64-72`) trata a ordem `show-overlay`/`recording-ready`. Confere.

## Faults injected

F1..F4 carried from 5b40fa0 (os arquivos mutados não mudaram e as asserções que os mataram seguem em
1bd7f3c). F5 e R2..R4 verified at 1bd7f3c: feitas na árvore real, uma por vez, cada uma revertida com
`git checkout -- <arquivo>` (como o brief pediu, em vez do worktree de verify.md). Baseline
`git status --porcelain` em 1bd7f3c: só `?? .specs/features/pill-redesign/verification.md`; depois da
última reversão, igual.

| Mutation | Location | Check que morreu | Killed |
| --- | --- | --- | --- |
| F1: espelho invertido, `levels[0]` nas pontas (`levels[i < 5 ? i : 9 - i]`) | `src/overlay/pillModel.ts:52-54` | C3 (rodada 1) | yes |
| F2: `isHoldToTalk` sem `hold_or_toggle` | `src/overlay/pillModel.ts:38` | C5 (rodada 1) | yes |
| F3: classe `hold` também em processando (sem `recording &&`) | `src/overlay/Pill.tsx:33` | C7 (rodada 1) | yes |
| F4: pulso de `1.2s` para `2s` | `src/overlay/Pill.css:31` | C8 (rodada 1) | yes |
| F5: `holdToTalk={false}` fixo (a pill nunca fica vermelha) | `src/overlay/RecordingOverlay.tsx:300` | C14 em 1bd7f3c: `C13 ok`, depois `AssertionError: false == true` na linha 274+ (bloco C14), exit 1 | yes - em 1bd7f3c (na rodada 1, em 5b40fa0, o mesmo mutante passava; ver achado 1) |
| R2: ramos do ternário do `label` trocados (`? t("overlay.processing") : t("overlay.recording")`) | `src/overlay/RecordingOverlay.tsx:305-306` | C11: `C10 ok`, depois `AssertionError: o rótulo precisa ser overlay.recording gravando e overlay.processing processando`, exit 1 | yes |
| R3: `{!recording && <span className="sspinner" />}` depois das barras | `src/overlay/Pill.tsx:42` | C7: `C6 ok`, depois `AssertionError` em `onlyBars` (`pill.test.tsx:46`) com o HTML terminando em `<span class="sspinner"></span></div>`, exit 1 | yes |
| R4: `{!recording && <img alt="" src="spin.svg" />}` depois das barras | `src/overlay/Pill.tsx:42` | C7: `C6 ok`, depois `AssertionError` em `onlyBars` (`pill.test.tsx:46`) com o HTML terminando em `<img alt="" src="spin.svg"/></div>`, exit 1 | yes |

## Gate

Em 1bd7f3c: `bun src/overlay/pill.test.tsx` - 14 checks impressos (`C1 ok`..`C14 ok`), 0 falhas,
exit 0. `bun run lint`, `bunx prettier --check src/overlay src/i18n`, `bun run check:translations`,
`bun run build` - os quatro exit 0 (C15).

## Round 2 (scoped: C7, C11, C14)

Verified at 1bd7f3c. Escopo: os dois achados da rodada 1 (C7; C11 e C14) e o diff
`git diff 5b40fa0 1bd7f3c`, que toca só `src/overlay/pill.test.tsx` (+24 -7).

**O que o diff faz.** Cria o helper `onlyBars` (`src/overlay/pill.test.tsx:43-48`): tira a `<div>`
raiz, exige que o interior seja exatamente a concatenação dos `<i></i>` (`:46`) e que sejam 10
(`:47`). Usa o helper em C2 gravando e armando (`:93-94`) e em C7 processando (`:171`). Troca a
asserção de C11 por uma regex sobre o ternário completo (`:234-239`) e as de C14 por igualdade de
texto exata (`:277`, `:278-283`).

**Asserções que liquidam cada achado.**

- C7: `src/overlay/pill.test.tsx:171` `onlyBars(html);` com `html = render({ mode: "processing", holdToTalk: true, levels: loud })` (`:165`). Afirma, via `:46` `assert.equal(inner, parts.join(""), html)` e `:47` `assert.equal(parts.length, 10, html)`, que a raiz processando tem só as 10 `<i>`. Mata R3 (`<span className="sspinner" />`) e R4 (`<img>`).
- C11: `src/overlay/pill.test.tsx:234-239` `assert.ok(/pillMode === "recording"\s*\?\s*t\("overlay\.recording"\)\s*:\s*t\("overlay\.processing"\)/.test(overlaySource), ...)`. Fixa qual chave vai em qual ramo. Mata R2.
- C14: `src/overlay/pill.test.tsx:277` `assert.ok(overlaySource.includes("holdToTalk={holdToTalk}"))` e `:278-283` `assert.ok(overlaySource.includes("setHoldToTalk(isHoldToTalk(settings.data.shortcut_activation))"), ...)`. Fixa a prop e a origem do estado. Mata F5.

**Superfície nova feita falhar.** Cada asserção nova morreu ao menos uma vez: `:277` (F5), `:234-239`
(R2), `:46` via `:171` (R3, R4). `onlyBars` em C2 (`:93-94`) reusa a mesma asserção `:46-47`,
morta por R3/R4. Não injetei mutante só para C2, porque a asserção é a mesma.

**Comandos e saídas (em 1bd7f3c).**

- `bun src/overlay/pill.test.tsx` - exit 0, `C1 ok`..`C14 ok`, `pill: all assertions passed`.
- `bun run lint` - exit 0 (`$ eslint src`, sem diagnóstico).
- `bunx prettier --check src/overlay src/i18n` - exit 0; `bun run check:translations` - exit 0; `bun run build` - exit 0.
- Mutantes F5, R2, R3, R4: exit 1 cada um, check que morreu na tabela de Faults.
- `git status --porcelain` final: `?? .specs/features/pill-redesign/verification.md`, igual ao baseline.

**Resíduo, não achado.** C11 e C14 seguem como prova por texto do fonte (gap de nível declarado em
checks.md: sem `tauri dev` e sem mock de módulo, o listener não roda em teste). Uma regressão que
mantenha as strings exatas e mude o comportamento por outro caminho (por exemplo um segundo
`setHoldToTalk(false)` depois) passaria. Isso fica fora do que um check por texto alcança e não
contradiz o plano aprovado.

## Ranked gaps

Nenhum aberto em 1bd7f3c. Registro da rodada 1 (5b40fa0), os dois fechados na rodada 2:

1. Mutante sobrevivente F5 - C14 - em 5b40fa0 `src/overlay/pill.test.tsx:265` aceitava `holdToTalk={false}`; mesmo padrão em C11 (`:226-227`) para o rótulo. Fechado: `pill.test.tsx:277` (C14) e `:234-239` (C11) em 1bd7f3c.
2. Arranjo do processando sem check - C7 - em 5b40fa0 `src/overlay/pill.test.tsx:162-164` não afirmava que os filhos são só as 10 `<i>`. Fechado: `pill.test.tsx:171` em 1bd7f3c.
