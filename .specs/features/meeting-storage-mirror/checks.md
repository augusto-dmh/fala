# meeting-storage-mirror checks

Profile: light · Windows 11, `CARGO_TARGET_DIR=C:\f\meeting-storage-mirror` · "manual" = app aberto

## S1 — o espelho (PR 1)

**C1** - O `.md` de uma sessão tem o frontmatter do plano (sem caminho de disco), o cabeçalho, as
notas ou o bloco de anotações e a transcrição com `^sN` e o bloco `fala:segments`
Proof: `cargo test -p fala-storage --test meeting_mirror every_change_rewrites_the_md`

**C2** - Toda mudança da sessão (rascunho, início, fim, áudio, anotações, trechos, participantes,
nome de falante, notas, nota editada, título) reescreve o `.md`
Proof: `cargo test -p fala-storage --test meeting_mirror every_change_rewrites_the_md`

**C3** - Ordem da ADR-0006: o banco recusa → nenhum `.md`; o espelho falha → a mudança fica no banco
sem erro, e `sync_meeting_mirrors` escreve depois o que faltou
Proof: `cargo test -p fala-storage --test meeting_mirror the_md_follows_the_commit`

**C4** - Trocar o título renomeia o arquivo; títulos iguais no mesmo dia viram `(2)`, `(3)`; o nome
não leva caracteres proibidos no Windows nem nos links do Obsidian
Proof: `cargo test -p fala-storage --test meeting_mirror title_changes_rename_and_same_names_get_a_number`
e `--lib meeting_mirror::tests::file_stem_is_safe_for_windows_and_obsidian`

**C5** - Apagar a sessão apaga o `.md`; as outras ficam; se o `.md` não sai, a sessão fica e volta
erro (um `.md` órfão a ressuscitaria no `reindex`)
Proof: `cargo test -p fala-storage --test meeting_mirror deleting_a_meeting_deletes_its_md`

**C6** - O corpo é o documento do "Copiar Markdown" (`fala_notes::note_document`), com e sem notas
Proof: `cargo test -p fala-storage --test meeting_mirror body_is_the_copied_note_document`

**C7** - Na abertura, o desktop escreve o `.md` das sessões gravadas antes do espelho
Proof: leitura de `MeetingManager::spawn_upkeep` + manual (app de debug portátil: abrir com um
`fala.sqlite` que já tem reuniões e ver `notas/Reuniões/`)

**C8** - manual: abrir `Reuniões/` no Obsidian e clicar num ponteiro `01:05` da nota pula para o
trecho
Proof: manual (Windows, Obsidian; precisa de nota gerada)
