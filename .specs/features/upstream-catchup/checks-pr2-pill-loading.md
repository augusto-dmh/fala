# upstream-catchup PR 2 (a pill mostra que o modelo está carregando) checks

Profile: light
Plan: none - o diff cabe numa frase (AGENTS.md, "Fluxo por feature")

## Intent

A gravação começa enquanto o modelo carrega em segundo plano (~3,5 s no Windows, ADR-0009, e de
novo depois do descarregamento por ociosidade). Hoje, se a pessoa solta a tecla antes da carga
acabar, a pill fica "processando" a carga inteira e parece travada. O upstream resolveu isso no
overlay dele (upstream #2227 e o commit seguinte `417dc6a`). Com este PR, a pill do Fala ganha um
estado "carregando modelo": depois de soltar a tecla ele aparece na hora; durante a gravação, só
quando a carga passa de 2 s (carga quente nunca pisca). Só `loading_completed` e `loading_failed`
encerram a carga, porque um `unloaded` do watcher de ociosidade pode chegar no meio.

Decisões (Confirmed? y — delegado):

- A pill continua sem texto (binding do pitch, `pill-redesign` AC 1 e 7). O estado é visual: um
  anel claro fino dentro da cápsula e um brilho que atravessa a cápsula, sobre qualquer cor
  (preta, vermelha, âmbar). Processando + carregando troca o pulso pelo brilho. Com
  `prefers-reduced-motion`, fica só o anel.
- Durante a gravação as barras continuam seguindo o mic: a captura está acontecendo, e esconder as
  barras faria a pessoa achar que não está sendo ouvida.
- O texto vai no `aria-label` por i18next: `overlay.loadingModel` e
  `overlay.recordingLoadingModel` (pt-BR fonte, en).
- No overlay "Ao vivo", o rótulo de trabalho troca "Transcrevendo..." por "Carregando modelo…"
  (parte do #2227). O aviso no painel ao vivo durante a gravação (parte do `417dc6a`) não vem: o
  overlay ao vivo fica como está (`pill-redesign`, fora de escopo).
- Nenhuma espera nova no handler de `show-overlay`: o estado vem só do evento
  `model-state-changed`, então a pill não atrasa (meta tecla → pill ≤ 50 ms).

8 checks in 1 slice · 0 one-way doors · 0 open, of which 0 block

## Checks

### S1 - estado de carga na pill · 6 files · 60 KB · ~15k

**C1** - `nextModelLoadStart(current, event, now)`: `loading_started` sem carga em voo devolve
`now`; `loading_started` com carga em voo mantém o início original; `loading_completed` e
`loading_failed` devolvem `null`; `unloaded` e `selection_changed` mantêm o valor atual
(upstream #2227 + `417dc6a`)
Proof: `bun src/overlay/pill.test.tsx` (linha `upstream-catchup PR2 C1 ok`)

**C2** - `showsModelLoading(mode, start, now)`: sem carga em voo é `false` nos dois modos;
processando com carga em voo é `true` com 0 ms decorridos; gravando é `false` com 1999 ms e
`true` com 2000 ms (`SLOW_LOAD_MS = 2000`)
Proof: `bun src/overlay/pill.test.tsx` (linha `upstream-catchup PR2 C2 ok`)

**C3** - A `Pill` com `modelLoading` ganha a classe `loading`, continua sem texto e com
exatamente 10 barras; gravando, as barras seguem os níveis (18 px com nível 1); processando, as
barras são `[5, 7, 9, 11, 13, 13, 11, 9, 7, 5]`; sem `modelLoading` não há classe `loading`
Proof: `bun src/overlay/pill.test.tsx` (linha `upstream-catchup PR2 C3 ok`)

**C4** - `Pill.css`: `.fpill.loading` tem `box-shadow: inset 0 0 0 1.5px rgba(255, 255, 255, 0.6)`;
`.fpill.loading::after` anima `fpill-loading-sweep` em loop infinito; `.fpill.processing.loading`
tem `animation: none` (sem o pulso); sob `prefers-reduced-motion: reduce`,
`.fpill.loading::after` tem `animation: none` e `display: none`
Proof: `bun src/overlay/pill.test.tsx` (linha `upstream-catchup PR2 C4 ok`)

**C5** - As strings existem nos dois idiomas: pt `overlay.loadingModel` = "Carregando modelo…",
`overlay.recordingLoadingModel` = "Gravando, carregando modelo…"; en "Loading model…" e
"Recording, loading model…"; o overlay usa as duas chaves no `aria-label` da pill
Proof: `bun src/overlay/pill.test.tsx` (linha `upstream-catchup PR2 C5 ok`)
Proof: `bun run check:translations`

**C6** - O overlay escuta `model-state-changed`, passa cada evento por `nextModelLoadStart`, passa
`modelLoading` para a `Pill` a partir de `showsModelLoading`, e o rótulo de trabalho do overlay
ao vivo usa `overlay.loadingModel` quando há carga em voo
Proof: `bun src/overlay/pill.test.tsx` (linha `upstream-catchup PR2 C6 ok`)

**C7** - O handler de `show-overlay` não ganhou nenhum `await` novo: continua com exatamente 2
(`syncLanguageFromSettings` e `getAppSettings`)
Proof: `bun src/overlay/pill.test.tsx` (linha `upstream-catchup PR2 C7 ok`)

**C8** - Os checks herdados da pill continuam verdes sem asserção editada, e o frontend passa no
lint e no build
Proof: `bun src/overlay/pill.test.tsx` (última linha `pill: all assertions passed`)
Proof: `bun run lint && bun run build`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| eventos de `model-state-changed` (5) | `loading_started` C1 · `loading_completed` C1 · `loading_failed` C1 · `unloaded` C1 · `selection_changed` C1 | - |
| modos da pill × carga (4) | gravando sem carga C2, C3 · gravando com carga C2, C3 · processando sem carga C2, C3 · processando com carga C2, C3 | - |
| limiar de 2 s (2 bordas) | 1999 ms C2 · 2000 ms C2 | - |
| idiomas (2) | pt C5 · en C5 | - |
| superfícies que mostram a carga (2) | pill C3, C4, C6 · overlay ao vivo C6 | - |

- C6 e C7 são estruturais (lêem o fonte do overlay, como os checks herdados da pill), porque o
  overlay depende de eventos do Tauri que o teste não sobe.

## Swept

- validation: n/a - nenhuma entrada de usuário
- failure modes: C1 (`loading_failed` encerra o estado)
- idempotency: C1 (`loading_started` repetido mantém o início)
- authorization: n/a - nada de acesso novo
- concurrency: C1 (`unloaded` do watcher no meio da carga não encerra o estado)
- data lifecycle: n/a - nada persistido
- dependency failure: C1
- state transitions: C2
- observability: C5 (`aria-label` para leitor de tela)

## Handoff

- S1 = ~15k (6 arquivos, 60 KB / 4), sob o orçamento de 150k - one builder
