# pill-menu verification

Windows 11 (Alienware), `CARGO_TARGET_DIR=C:\f\pill-menu-drag`, 2026-10-12.

Verdict: **PASS** for C1-C5. The Windows manual checks M1-M5 are **Not tested** with a person at
the PC (script in `checks.md`); they do not change the verdict.

| Check | Proof | Result |
| --- | --- | --- |
| C1 | `cargo test -p fala --lib tray::tests::pill_menu_items_and_their_handlers` | ok |
| C2, C3 | `cargo test -p fala --lib overlay::tests::snooze_hides_only_the_idle_pill_for_an_hour` | ok |
| C4 | `bun src/overlay/pill.test.tsx` (`pill-menu C4 ok`) | ok |
| C5 | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p fala --lib` (522 passed, 0 failed, 1 ignored), `bun run lint`, `bun run format:check`, `bunx tsc --noEmit`, `bun run check:translations`, `scripts/check-brand.sh`, `bun src/overlay/notice.test.tsx` | all exit 0 |

The build log shows the tray strings now carry 20 fields (`hidePillHour` added).

## Settled mid-build

- Review: the snooze timer first slept one hour on the monotonic clock while the end was kept on
  the wall clock, so a clock change could strand the pill hidden. It is now one timer that sleeps
  toward the end and wakes at least every minute (`snooze_wait`); a second snooze reuses it.
- Review: "Configurações" opens the general view, as the rail does, not the last sub-view.
- `src/bindings.ts` regenerated with the portable debug build (`fala.exe --list-devices`): only
  `openPillMenu` was added.
