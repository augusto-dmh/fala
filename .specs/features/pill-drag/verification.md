# pill-drag verification

Windows 11 (Alienware), `CARGO_TARGET_DIR=C:\f\pill-menu-drag`, 2026-10-12. Stacked on
`pill-menu`.

Verdict: **PASS** for C1-C5. The Windows manual checks M1-M5 are **Not tested** with a person at
the PC (script in `checks.md`); they do not change the verdict.

| Check | Proof | Result |
| --- | --- | --- |
| C1 | `cargo test -p fala --lib overlay::tests::dragged_pill_slides_along_its_edge` (Windows) | ok |
| C2 | `cargo test -p fala --lib overlay::tests::pill_center_setting_is_applied_and_reset` | ok |
| C3 | `cargo test -p fala --lib settings::tests::pill_idle_defaults_off` | ok |
| C4 | `bun src/overlay/pill.test.tsx` (`pill-drag C4 ok`) | ok |
| C5 | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p fala --lib` (524 passed, 0 failed, 1 ignored), `bun run lint`, `bun run format:check`, `bunx tsc --noEmit`, `bun run check:translations`, `scripts/check-brand.sh`, `bun src/overlay/notice.test.tsx` | all exit 0 |

## Settled mid-build

- Review: a flick released before the drag thread's first step used to save the unmoved place;
  now only a drag that moved the window saves, and the live center changes only when it moves.
- Review: on Linux the backend never drags, so the page only wires the drag on Windows; a small
  slip of the hand no longer swallows a click on the meeting pill there.
- `windows_overlay_bounds` takes the center as an eighth argument; clippy's
  `too_many_arguments` is allowed on that one function.
- `src/bindings.ts` regenerated with the portable debug build: `startPillDrag` and
  `AppSettings.pill_center_x` added.
- Known limit: the drag saves with the same read-modify-write of the settings as every other
  setting command; a setting changed in the same instant the button comes up can be lost.
