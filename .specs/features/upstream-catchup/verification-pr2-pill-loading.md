# upstream-catchup PR 2 (a pill mostra que o modelo está carregando) verification

**Verdict**: PASS
**Profile**: light
**Diff range**: eb0482c..2a50294
**Round**: 1 - full
**Verifier**: independent sub-agent (author != verifier)

Everything ran at `2a50294` (detached worktree, clean `git status --porcelain` before and after).
The checks have no plan.md; their `## Intent` section served as the plan.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | `nextModelLoadStart`: started with no load returns `now`, started during a load keeps the original start, completed/failed return `null`, unloaded/selection_changed keep the current value | `bun src/overlay/pill.test.tsx` exit 0, printed `upstream-catchup PR2 C1 ok` | `src/overlay/pill.test.tsx:363` - `assert.equal(nextModelLoadStart(null, "loading_started", 1000), 1000)`; `:364` - `assert.equal(nextModelLoadStart(500, "loading_started", 1000), 500)`; `:365-366` - completed/failed `-> null`; `:367-368` - unloaded/selection_changed `-> 500` | PASS |
| C2 | `showsModelLoading`: no load is false in both modes; processing with a load is true at 0 ms; recording is false at 1999 ms and true at 2000 ms; `SLOW_LOAD_MS = 2000` | same run, printed `upstream-catchup PR2 C2 ok` | `src/overlay/pill.test.tsx:375` - `assert.equal(SLOW_LOAD_MS, 2000)`; `:378` - `showsModelLoading("processing", 1000, 1000)` is `true`; `:379-380` - `showsModelLoading("recording", 1000, 2999)` is `false` and `(…, 3000)` is `true` (1999 and 2000 ms elapsed) | PASS |
| C3 | `Pill` with `modelLoading` gets class `loading`, no text, exactly 10 bars; recording bars follow levels (18 px at level 1); processing bars fixed; no class without `modelLoading` | same run, printed `upstream-catchup PR2 C3 ok` | `src/overlay/pill.test.tsx:387` - `assert.ok(classes(recording).includes("loading"))`; `:388` - `assert.equal(text(recording), "")`; `:389` and `:399` - `onlyBars(...)` (asserts 10 `<i>` and nothing else); `:390` - `assert.deepEqual(bars(recording), Array(10).fill(18))`; `:400` - `assert.deepEqual(bars(processing), [5, 7, 9, 11, 13, 13, 11, 9, 7, 5])`; `:402-404` - no `loading` class without the prop | PASS |
| C4 | `Pill.css`: ring `box-shadow`, `::after` runs `fpill-loading-sweep` infinitely, `.fpill.processing.loading` has `animation: none`, reduced motion sets `animation: none` and `display: none` on the sweep | same run, printed `upstream-catchup PR2 C4 ok` | `src/overlay/pill.test.tsx:410-414` - `has(rule(css, ".fpill.loading"), "box-shadow: inset 0 0 0 1.5px rgba(255, 255, 255, 0.6)")`; `:417` - `/animation:\s*fpill-loading-sweep\b[^;]*\binfinite\b/.test(sweep)`; `:421-425` - `has(rule(css, ".fpill.processing.loading"), "animation: none")`; `:431-432` - `has(still, "animation: none")`, `has(still, "display: none")` inside the reduced-motion block | PASS |
| C5 | pt and en strings exist with the stated values; the overlay uses both keys in the pill `aria-label` | same run, printed `upstream-catchup PR2 C5 ok`; `bun run check:translations` exit 0 (`PT: All keys present`, 466 keys) | `src/overlay/pill.test.tsx:440-443` - `assert.equal(pt.loadingModel, "Carregando modelo…")`, `pt.recordingLoadingModel` `"Gravando, carregando modelo…"`, en `"Loading model…"`, `"Recording, loading model…"`; `:444-445` - `overlaySource.includes('t("overlay.loadingModel")')` and `('t("overlay.recordingLoadingModel")')`; wiring read in code: `src/overlay/RecordingOverlay.tsx:367` - `label={pillLabel}` and `src/overlay/Pill.tsx:53` - `aria-label={label}` | PASS (see finding 2) |
| C6 | overlay listens to `model-state-changed`, runs each event through `nextModelLoadStart`, passes `modelLoading` from `showsModelLoading`, live working label uses `overlay.loadingModel` while a load is in flight | same run, printed `upstream-catchup PR2 C6 ok` | `src/overlay/pill.test.tsx:452` - `/listen<ModelStateEvent>\(\s*"model-state-changed"/.test(overlaySource)`; `:455-457` - source has `nextModelLoadStart(`, `showsModelLoading(`, `modelLoading={pillLoading}`; `:459` - `workKind === "polishing" ? t("overlay.processing") : transcribingLabel`; `:465` - `const transcribingLabel = loadStart !== null ? t("overlay.loadingModel")` | PASS |
| C7 | `show-overlay` handler has no new `await`: exactly 2 | same run, printed `upstream-catchup PR2 C7 ok` | `src/overlay/pill.test.tsx:478` - `assert.equal((handler[1].match(/\bawait\b/g) ?? []).length, 2, handler[1])`; read in code: `src/overlay/RecordingOverlay.tsx:91` `await syncLanguageFromSettings()` and `:95` `await commands.getAppSettings()`, handler untouched by the diff | PASS |
| C8 | inherited pill checks stay green with no assertion edited; lint and build pass | same run: `C1 ok` … `C14 ok`, `shortcut-gestures C24 ok`, `session-limit C18 ok`, last line `pill: all assertions passed`; `bun run lint` exit 0; `bun run build` (`tsc && vite build`) exit 0 | `src/overlay/pill.test.tsx:482` - `console.log("pill: all assertions passed")`; `git diff eb0482c..2a50294 --numstat -- src/overlay/pill.test.tsx` = `134 2`, and the 2 removed lines are the header comment (`:3`) and the `./pillModel` import (`:10`), no assertion | PASS |

## Coverage

Not recomputed: profile light gives up the `Coverage` recompute. Read against the code for sanity
only: the five `model-state-changed` event types in the checks are the five the backend emits
(`apps/desktop/src/managers/transcription.rs:431,494,508,524,727` and
`apps/desktop/src/commands/models.rs:136`), and all five have an assertion in C1.

## Swept existing

No `Swept` row resolves to "existing": every row cites a check (C1, C2, C5) or is `n/a`. Nothing
to re-read.

## Faults injected

None - profile light. No mutant was run, so nothing here shows the new assertions would catch a
wrong implementation beyond what reading them shows.

## Gate

- `bun src/overlay/pill.test.tsx` - 23 check lines ok (14 pill-redesign, 1 shortcut-gestures, 1 session-limit, 7 PR2), exit 0
- `bun run check:translations` - exit 0
- `bun run lint` - exit 0
- `bun run build` - exit 0
- `scripts/check-brand.sh` - `ok: no Handy branding outside the allowlist`; `git diff eb0482c..2a50294 | grep -i handy` - no hit

## Findings

1. **Deviation from the pill design, recorded as a decision (needs a human to know about it, does
   not fail).** The binding pill design (`.specs/features/pill-redesign/plan.md` AC 8, and the
   pitch line it quotes: "processando com barras paradas e pulso") puts a pulse on every
   processing pill. `src/overlay/Pill.css` `.fpill.processing.loading { animation: none; }`
   replaces that pulse with the sweep while a load is in flight, and C4 asserts it. The swap is a
   delegated decision written in the checks' `## Intent`, for a sub-state that design never drew.
   The four invariants the brief named all hold: the capsule stays black (red/amber tones
   unchanged, the ring is a 1.5 px inset), 10 bars (`pill.test.tsx:389,399`), no visible text
   (`:388,398`), no icon (`onlyBars` admits only `<i>` bars). Under reduced motion the pill is
   still and opaque, as AC 9 requires. If the pitch's pulse is meant to cover every processing
   wait, Augusto should confirm the swap.
2. **C5 test is weaker than its claim (level/precision gap).** `pill.test.tsx:444` passes as long as
   `t("overlay.loadingModel")` appears anywhere in the overlay. It also appears in the live label
   (`RecordingOverlay.tsx:223`), so this line cannot tell whether the pill's `aria-label` uses it.
   No test asserts `label={pillLabel}`. I read the wiring in the code and it is correct today
   (`RecordingOverlay.tsx:345-353,367`, `Pill.tsx:53`), but no test protects it.
3. **C6/C7 are structural (regex over source), as the checks say.** They prove the wiring and the
   number of awaits, not runtime behaviour. C7's regex depends on the handler's closing `      });`
   indentation. A reformat would turn the check red (a false alarm), not hide a regression.
4. **Divergence from upstream, approved in C1.** Upstream `417dc6a` restarts the 2 s timer on
   every `loading_started`. Here a repeated `loading_started` keeps the first start. Loads are
   serialized by the backend (`is_loading` guard, `transcription.rs:402-406`), so this only
   matters if an event is duplicated. The worst case is the recording pill showing the load a
   bit early. The other upstream rules carried over: the 2 s threshold applies only while
   recording (`pillModel.ts` `showsModelLoading`); only `loading_completed`/`loading_failed` end
   a load; after the key is released the pill and the live working label switch at once
   (`showsModelLoading("processing", …)` at 0 ms, `transcribingLabel` on `loadStart !== null`).
   Upstream's "a new session re-checks the slow state" needs no code here, because the flag is
   computed from `loadStart` and `now` on each render. Upstream's live-panel notice during
   recording was left out on purpose (Intent).
5. **Latency (C7), the timer and stuck states: no defect found.** The `show-overlay` handler is
   unchanged. `pillLoading` is computed synchronously during render, and the new listener is
   registered after the show listener, so nothing sits between the event and the pill. The timer
   effect (`RecordingOverlay.tsx:186-195`) clears its `setTimeout` when `loadStart` changes or the
   component unmounts. When the threshold has already passed, it sets `now` directly and
   schedules no timer. `hide-overlay` correctly leaves `loadStart` alone, because the load
   outlives the overlay. The loading state can only stick if a terminal event never arrives. The
   backend emits `loading_failed` on every error path after `loading_started`
   (`transcription.rs:518-519` comment and the call sites). A panic mid-load would skip it, and
   the overlay would then show loading until the next load finishes. Upstream has the same risk
   (residual, not introduced here). One pre-existing pattern was also inherited: the listener
   cleanup returned from the async setup is never called.
6. **Strings and brand: clean.** Both new strings go through i18next under `overlay.*` in `pt` and
   `en`. There are no new JSX literals (lint exit 0) and no "Handy" anywhere in the diff.
