# meeting-notes-polish verification

Checks: `.specs/features/meeting-notes-polish/checks.md` · Windows 11 (Alienware) ·
`CARGO_TARGET_DIR=C:\f\meeting-notes-polish` · three stacked PRs: #126
(`feat/meeting-notes-polish`), #127 (`feat/meeting-notes-polish-2`), #129
(`feat/meeting-notes-polish-3`).

Verifier: the same agent that built the feature, rerunning the proof commands after the commits;
each PR also went through a `code-review` pass at level high, with the confirmed findings fixed
before the push. No independent verifier pane ran this round.

**Verdict: PASS for C1–C13.** What needs a real call or the maintainer's keys is under "Manual".

| Check | Proof | Result |
|---|---|---|
| C1 | `cargo test -p fala-storage --test meeting_people typed_name_replaces_the_suggestion_and_can_be_erased -- --exact` | passed (storage 62/62 on #126) |
| C2 | `cargo test -p fala --lib meeting::people::tests::blank_name_gives_the_label_back -- --exact` | passed (meeting 30/30 on #126) |
| C3 | `cargo test -p fala --lib meeting::pipeline::tests::typed_name_with_at_never_goes_out -- --exact` | passed |
| C4 | `bun src/components/meeting/speakerRename.test.tsx` | RN-label, RN-wiring, RN-i18n ok |
| C5 | `cargo test -p fala-storage --test meeting_notes edit_marks_the_notes_and_a_generation_clears_the_mark -- --exact` | passed |
| C6 | `cargo test -p fala-storage --test meeting_notes mark_goes_with_the_meeting -- --exact` | passed |
| C7 | `cargo test -p fala --lib meeting::pipeline::tests::edited_notes_are_regenerated_only_after_the_confirmation -- --exact` | passed (meeting 31/31 on #127) |
| C8 | `bun src/components/meeting/notesPolish.test.tsx` (NP-editable, NP-merge) | ok |
| C9 | same (NP-confirm, NP-i18n); `grep -n "notes_edited: boolean\|saveMeetingNotes" src/bindings.ts` | ok; both present |
| C10 | `cargo test -p fala-storage --test meeting_notes template_is_picked_only_before_the_notes -- --exact` | passed (storage 65/65 on #129) |
| C11 | `cargo test -p fala --lib meeting::pipeline::tests::notes_input_from_session -- --exact` | passed (meeting 32/32 on #129) |
| C12 | `cargo test -p fala --lib meeting::tests::pill_card_for_the_call_or_the_first_seconds -- --exact` | passed |
| C13 | `bun src/components/meeting/meetingFlow.test.tsx` | FL-search, FL-copy, FL-consent, FL-flow, FL-i18n ok |

Every PR: `cargo clippy --workspace --all-targets -- -D warnings` clean and `cargo test --workspace`
green in the pre-push hook (972 passed on #127's tree, 974 on #129's; 0 failed; 53 ignored),
`bunx tsc --noEmit`, `bun run lint`, `bun run format:check`, `bun run check:translations` clean.
The hook ran the tests with `RUST_TEST_THREADS=1`: in parallel runs under machine load,
`llm_auto::tests::a_slow_block_keeps_its_rules_until_the_late_answer` (a 2.9 s wall-clock
assertion outside this feature) failed in 3 of 4 hook runs.

## Manual (not run: needs a real call and the maintainer's ElevenLabs and Anthropic keys)

1. Open a transcribed meeting, click "Pessoa 1", type a name, Enter: every turn of that person
   shows it; clear the field: "Pessoa 1" again. In a meeting with notes, the rename offers
   "Gerar de novo".
2. Generate notes, click "Editar", change one AI line and add one, "Concluir": the changed and new
   lines are black without the magnifier, the others gray with sources. "Gerar notas" with the same
   template asks first; another template does not.
3. Pick "1:1" in a draft, record 3+ minutes with both keys, stop: "Transcrevendo…", then
   "Escrevendo notas…" (or "Aprimorando suas notas…" with typed notes), and the notes use 1:1.
4. At each start, the 5 s consent chip on the page and over the pill; "Copiar aviso" pastes the
   text into the call chat.
5. Search "reuniao" in a transcript with "Reunião": the line shows, with the word marked.
