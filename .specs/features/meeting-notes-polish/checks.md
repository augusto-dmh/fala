# meeting-notes-polish checks

Profile: light
Plan: `.specs/features/meeting-notes-polish/plan.md`

PR A: 9 checks · 3 doors · 0 open. Todas as provas Rust rodam com
`CARGO_TARGET_DIR=C:\f\meeting-notes-polish`; nada de rede, chave ou microfone.

## PR A - renomear, nota editável, confirmar antes de regenerar

**C1** - `meeting_note_edits` nasce na abertura; editar marca, gerar desmarca, a marca sobrevive a
reabrir; editar sem nota gerada é `NotFound` (AC 1, AC 2, door 1)
Proof: `cargo test -p fala-storage --test meeting_notes edit_marks_the_notes_and_a_generation_clears_the_mark -- --exact`

**C2** - Apagar a reunião apaga a marca (cascata) (AC 1)
Proof: `cargo test -p fala-storage --test meeting_notes mark_goes_with_the_meeting -- --exact`

**C3** - Nome digitado troca a sugestão pendente daquela pessoa, vale sem sugestão, e vazio volta
a "Pessoa N" (a linha some); sessão inexistente é `NotFound`; pessoa 0 é recusada (AC 4)
Proof: `cargo test -p fala-storage --test meeting_people typed_name_replaces_the_suggestion_and_can_be_erased -- --exact`

**C4** - Regenerar nota editada sem `replace_edited` devolve `{"kind":"notes_edited"}`; com ele,
ou sem edição, segue (AC 3, door 2)
Proof: `cargo test -p fala --lib meeting::pipeline::tests::edited_notes_are_regenerated_only_after_the_confirmation -- --exact`

**C5** - Nome aplicado com `@` sai no payload como "Pessoa N" (AC 5, door 3)
Proof: `cargo test -p fala --lib meeting::pipeline::tests::typed_name_with_at_never_goes_out -- --exact`

**C6** - Nome digitado: espaços colapsados, corte em 80, vazio = `None` (AC 4)
Proof: `cargo test -p fala --lib meeting::people::tests::blank_name_gives_the_label_back -- --exact`

**C7** - O editor mostra o texto sem marcador/ponteiro/âncora; sem mudança a nota volta igual;
linha gerada mudada perde marcador e fontes, as outras ficam (AC 6)
Proof: `bun src/components/meeting/notesPolish.test.tsx` (NP-editable, NP-merge)

**C8** - "Editar" na nota; confirmação só depois do clique e `true` só nela; a página repassa ao
comando; "Pessoa N" e nome aplicado viram botão de renomear, "Eu" não (AC 4, AC 7)
Proof: `bun src/components/meeting/notesPolish.test.tsx` (NP-confirm, NP-rename)

**C9** - Strings em pt e en; `MeetingDetail.notes_edited` e os comandos novos em `src/bindings.ts`
(AC 7)
Proof: `bun src/components/meeting/notesPolish.test.tsx` (NP-i18n) · `bun run check:translations` · `grep -n "notes_edited: boolean\|setMeetingSpeakerName\|saveMeetingNotes" src/bindings.ts`
