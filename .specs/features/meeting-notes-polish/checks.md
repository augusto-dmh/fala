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
