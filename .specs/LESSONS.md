# LESSONS - auto-maintained by scripts/lessons.py

> Machine-owned. Do NOT hand-edit. Changes are overwritten on the next `lessons.py` write.
> Canonical state lives in `.specs/lessons.json`. Edit lessons only via the script.
> promote_threshold=2 distinct features · window_days=45 · quarantine_threshold=2

## Confirmed (load these at Plan/Checks)

Corroborated across multiple features. Safe to apply as guidance.

### L-002 - A grep proof that counts or negates matches must exclude the test module and fail when the target is absent, or it passes with production broken
- signal: `spec_precision_gap` · recurrence: 2 feature(s) · scope: `proofs` · harmful: 0
- features: cancel-anywhere, shortcut-gestures
- evidence: cancel-anywhere C8 (verification.md finding 2) (proofs) (+1 more)
- last seen: 2026-10-04T19:55:48Z

## Candidates (under observation - do NOT load as guidance yet)

Seen once or not yet corroborated. Tracked, not trusted.

### L-001 - For a check on a document, make the proof assert the claimed values and structure, not only that a matching line or row count exists
- signal: `spec_precision_gap` · recurrence: 1 feature(s) · scope: `docs` · harmful: 0
- features: cli-bench
- evidence: C32, C33 - .specs/features/cli-bench/verification.md Gaps R8, R9 (docs)
- last seen: 2026-09-30T02:28:17Z

### L-003 - A check derived from a criterion that says 'the reason' must assert the reason text, and a check may never be narrower than its criterion
- signal: `spec_precision_gap` · recurrence: 1 feature(s) · scope: `checks` · harmful: 0
- features: mcp-local
- evidence: mcp-local rounds 1-3 (C26) (checks)
- last seen: 2026-10-04T19:55:48Z

### L-004 - Decide in the plan how a manual check that only runs on another machine is recorded: an Unproven row that keeps the verdict FAIL, or a checklist outside the table; never leave it to the Verifier
- signal: `spec_deviation` · recurrence: 1 feature(s) · scope: `verify` · harmful: 0
- features: shortcut-gestures
- evidence: shortcut-gestures C17 vs active-app C7 vs cancel-anywhere C9 (verify)
- last seen: 2026-10-04T19:55:48Z

### L-005 - Tests must not depend on OS network timing: a freshly closed port is retried for about 2 s on Windows; use port 0 for a refused connection
- signal: `gate_fail` · recurrence: 1 feature(s) · scope: `tests` · harmful: 0
- features: postproc
- evidence: Windows CI on main after #34 and #33 (fixed by #41, #42) (tests)
- last seen: 2026-10-04T19:55:48Z

### L-006 - A diff proof scoped to the feature must compare from the merge-base (origin/main...HEAD), never two-dot, or a moving main makes it fail on commits the branch never touched
- signal: `spec_precision_gap` · recurrence: 1 feature(s) · scope: `proofs` · harmful: 0
- features: hotkey-inject-traits
- evidence: C22 - .specs/features/hotkey-inject-traits/verification.md round 1 finding 1 (proofs)
- last seen: 2026-10-09T19:33:29Z

## Quarantined (failed when applied - ignore)

A confirmed lesson that recurred alongside failure. Kept for the maintainer to review.

_none_
