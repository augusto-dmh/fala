# meeting-notes-polish checks

Profile: light
Plan: `.specs/features/meeting-notes-polish/plan.md`

Todas as provas Rust rodam com `CARGO_TARGET_DIR=C:\f\meeting-notes-polish`; nada de rede, chave
ou microfone.

## PR 1 - renomear "Pessoa N"

**C1** - Nome digitado troca a sugestão pendente daquela pessoa, vale sem sugestão, e vazio volta
a "Pessoa N" (a linha some); sessão inexistente é `NotFound`; pessoa 0 é recusada (AC 4)
Proof: `cargo test -p fala-storage --test meeting_people typed_name_replaces_the_suggestion_and_can_be_erased -- --exact`

**C2** - Nome digitado: espaços colapsados, corte em 80, vazio = `None` (AC 4)
Proof: `cargo test -p fala --lib meeting::people::tests::blank_name_gives_the_label_back -- --exact`

**C3** - Nome aplicado com `@` sai no payload como "Pessoa N" (AC 5, door 3)
Proof: `cargo test -p fala --lib meeting::pipeline::tests::typed_name_with_at_never_goes_out -- --exact`

**C4** - "Pessoa N" e o nome aplicado viram botão de renomear, "Eu" não; o nome vai ao comando;
numa sessão com notas, o nome trocado oferece gerar de novo; strings em pt e en (AC 4, AC 7)
Proof: `bun src/components/meeting/speakerRename.test.tsx` (RN-label, RN-wiring, RN-i18n) · `grep -n "setMeetingSpeakerName" src/bindings.ts`

## PR 2 - nota editável e confirmação antes de regenerar

**C5** - `meeting_note_edits` nasce na abertura; editar marca, gerar desmarca, a marca sobrevive a
reabrir; editar sem nota gerada é `NotFound` (AC 1, AC 2, door 1)
Proof: `cargo test -p fala-storage --test meeting_notes edit_marks_the_notes_and_a_generation_clears_the_mark -- --exact`

**C6** - Apagar a reunião apaga a marca (cascata) (AC 1)
Proof: `cargo test -p fala-storage --test meeting_notes mark_goes_with_the_meeting -- --exact`

**C7** - Regenerar nota editada com o mesmo template sem `replace_edited` devolve
`{"kind":"notes_edited"}`; com ele, com outro template ou sem edição, segue (AC 3, door 2)
Proof: `cargo test -p fala --lib meeting::pipeline::tests::edited_notes_are_regenerated_only_after_the_confirmation -- --exact`

**C8** - O editor mostra o texto sem marcador/ponteiro/âncora; sem mudança a nota volta igual;
linha gerada mudada perde marcador e fontes, as outras ficam (AC 6)
Proof: `bun src/components/meeting/notesPolish.test.tsx` (NP-editable, NP-merge)

**C9** - "Editar" na nota; a confirmação só com o mesmo template e `true` só nela; a página
repassa ao comando; strings em pt e en; `MeetingDetail.notes_edited` e `saveMeetingNotes` em
`src/bindings.ts` (AC 3, AC 7)
Proof: `bun src/components/meeting/notesPolish.test.tsx` (NP-confirm, NP-i18n) · `grep -n "notes_edited: boolean\|saveMeetingNotes" src/bindings.ts`

## PR 3 - template antes, aviso, consentimento, busca e notas depois do parar

**C10** - O template escolhido antes das notas fica na sessão; com notas, a escolha não troca o
template que as escreveu (AC 8)
Proof: `cargo test -p fala-storage --test meeting_notes template_is_picked_only_before_the_notes -- --exact`

**C11** - Parar → transcrever → notas só com o ajuste ligado, a chave, sessão não "só local",
sem notas, ≥ 3 min e ≥ 2.500 caracteres de transcrição (AC 9)
Proof: `cargo test -p fala --lib meeting::pipeline::tests::notes_input_from_session -- --exact`

**C12** - A pílula ganha espaço para o cartão nos primeiros 5 s de gravação ou com a call em
outra saída (AC 10)
Proof: `cargo test -p fala --lib meeting::tests::pill_card_for_the_call_or_the_first_seconds -- --exact`

**C13** - Busca sem acento e sem maiúsculas com o trecho marcado; "Copiar transcrição" como
"Nome: texto" por turno; chip de 5 s e "Copiar aviso" no painel e chip na pílula; o aviso não
promete áudio só local; template no rascunho e na gravação; estados "Escrevendo notas" /
"Aprimorando suas notas", erro com "Tentar de novo"; strings em pt e en (AC 8-11)
Proof: `bun src/components/meeting/meetingFlow.test.tsx` (FL-search, FL-copy, FL-consent, FL-flow, FL-i18n)
