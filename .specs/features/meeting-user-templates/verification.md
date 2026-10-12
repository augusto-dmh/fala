# meeting-user-templates verification

Checks: `.specs/features/meeting-user-templates/checks.md` · Windows 11 (Alienware) ·
`CARGO_TARGET_DIR=C:\f\meeting-user-templates` · one PR, `feat/meeting-user-templates`.

Verifier: the same agent that built the feature, rerunning the proof commands after the commit,
plus a `code-review` pass at level high. No independent verifier pane ran.

**Verdict: PASS for C1–C6.** The round trip through Notepad and a real notes call is under "Manual".

| Check | Proof | Result |
|---|---|---|
| C1–C4 | `cargo test -p fala-notes` | 34 + 10 passed (4 new in `template::tests`) |
| C5 | `cargo test -p fala --lib meeting` | 39 passed; bindings carry both commands and `source` |
| C6 | `bun src/components/meeting/userTemplates.test.tsx` | UT-groups, UT-search, UT-settings, UT-i18n ok |

Also clean: workspace clippy and tests (pre-push), tsc, lint, format, translations, UI tests.

## Manual (not run: needs a real call and the maintainer's Anthropic key)

Duplicate "Reunião geral", rename a section of `geral-copia.json` in Notepad, come back to Fala,
generate notes with "Reunião geral (cópia)" and see the section; break the JSON and see it in red.
