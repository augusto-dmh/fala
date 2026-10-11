# pill-idle verification

Windows 11 (Alienware), `CARGO_TARGET_DIR=C:\f\pill-idle-hint`, 2026-10-11.

Verdict: **PASS** for C1-C10. The Windows manual checks M1-M5 are **Not tested** (they need the PC
free; script in the PR body), and do not change the verdict (L-004).

| Check | Proof | Result |
| --- | --- | --- |
| C1 | `cargo test -p fala --lib settings::tests::pill_idle_defaults_off` | ok |
| C2 | `overlay::tests::rest_after_hide_table` (12 rest cases + 12 `idle_wanted` cases) | ok |
| C3 | `overlay::tests::idle_dimensions` | ok |
| C4 | `overlay::tests::meeting_overlay_states`, `overlay::tests::idle_never_replaces_an_active_state` | ok |
| C5 | `overlay::tests::fullscreen_poll_is_gated`, `overlay::tests::fullscreen_states` (Windows); `cfg(windows)` proven by the Linux CI build, where the `windows` crate does not exist | ok |
| C6-C9 | `bun src/overlay/pill.test.tsx` (`pill-idle C6` .. `C9 ok`) | ok |
| C10 | `bun run lint`, `bunx tsc --noEmit`, `bun run check:translations`, `bun run format:check`, `bun run build`, `cargo clippy -p fala --all-targets -- -D warnings`, `scripts/check-brand.sh`, `bun src/overlay/notice.test.tsx` | all exit 0 |

`cargo test -p fala --lib -- overlay settings::tests::pill_idle`: 18 passed, 0 failed.

## Settled mid-build

- C5 lost its source-text check of `cfg(windows)`: the Linux CI build fails without it, which is a
  stronger proof. The three notification states moved into `is_fullscreen_state`, tested on Windows.
- `src/bindings.ts` regenerated with the portable debug build (`fala.exe --list-devices`); the
  generator also reorders three existing history/stats commands to match `lib.rs`.
