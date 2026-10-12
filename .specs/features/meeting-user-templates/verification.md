# meeting-user-templates verification

Checks: `.specs/features/meeting-user-templates/checks.md` · Windows 11 (Alienware) ·
`CARGO_TARGET_DIR=C:\f\meeting-user-templates` · one PR, `feat/meeting-user-templates`.

Verifier: the same agent that built the feature, rerunning the proof commands after the commit,
plus a `code-review` pass at level high. No independent verifier pane ran.

**Verdict: PASS for C1–C6.** The round trip through Notepad and a real notes call is under "Manual".

| Check | Proof | Result |
|---|---|---|
| C1–C4 | `cargo test -p fala-notes` | 35 + 10 passed (5 new in `template::tests`) |
| C5 | `cargo test -p fala --lib meeting` | 39 passed; bindings carry both commands and `source` |
| C6 | `bun src/components/meeting/userTemplates.test.tsx` | UT-groups, UT-search, UT-settings, UT-i18n ok |

Also clean: `cargo clippy --workspace --all-targets -- -D warnings`, `bunx tsc --noEmit`,
`bun run lint`, `bun run format:check`, `bun run check:translations`, and the meeting UI tests
whose fixtures changed (`meeting`, `meetingFlow`, `notesPolish`).

## Manual (not run: needs a real call and the maintainer's Anthropic key)

1. Meetings → "Templates de notas" → "Duplicar para editar" on "Reunião geral": the folder opens
   with `geral-copia.json` and `LEIAME.txt`. Rename a section in Notepad, save, come back to Fala:
   "Reunião geral (cópia)" is in the list and in the "Seus templates" group of both pickers.
2. Generate notes for a transcribed meeting with the copy: the renamed section is in the notes.
3. Break the JSON (delete a brace), come back: "geral-copia.json: …" shows in red; the others stay.
