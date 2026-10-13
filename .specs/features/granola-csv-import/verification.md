# granola-csv-import verification

Windows 11 (Alienware), `CARGO_TARGET_DIR=C:\f\granola-csv-import`, CSVs sintéticos (o export real
não está na máquina).

| Check | Evidência |
| --- | --- |
| C1 | `cargo test -p fala-storage granola`: 7 unit ok; `--test granola` `a_zip_named_csv_is_read_by_its_content` ok |
| C2 | `cargo test -p fala-storage --test granola`: 3 ok; `--lib meeting_imports`: 2 ok; `cargo test -p fala-storage`: 94 ok |
| C3 | `cargo test -p fala --lib meeting`: 50 ok (2 do `granola`) |
| C4 | `bun src/components/meeting/granola.test.tsx`: 3 checks ok; `import`, `meeting`, `meetingFlow`, `notesPolish` ok; `bun run check:translations` ok |
| C5 | não testado (abaixo) |

Também: `cargo clippy --workspace --all-targets -- -D warnings` limpo; `cargo fmt --all --check`,
`bun run lint`, `bun run format:check`, `bunx tsc --noEmit` limpos. `src/bindings.ts` regerado com o
`fala.exe --list-devices` do app portátil.

Revisão `code-review` (high) sobre a pilha: 10 achados; corrigidos 9 (reimportar desfazia edições,
trabalho em curso, botão Transcrever, datas com offset colado, título em pt no dado, falante falso
"Próximos passos:", teto do `.zip` descomprimido, marcador em bloco de código e tabela, N+1 na
lista, guarda do slot). Fica: o arquivo é lido duas vezes (prévia e importação).

Not tested:

- C5, com o CSV real do Augusto: importar, abrir 3 reuniões antigas, buscar um termo de uma
  transcrição importada, importar de novo e ver as mesmas contagens (atualizadas, não novas).
  Conferir o formato de `document_created` e das linhas da transcrição (`Me:`/`Them:`).
- O diálogo no app (clicar, prévia, progresso, cancelar): precisa de alguém na máquina.
- Teto de 100 MB do `.zip` descomprimido: sem teste (pediria 100 MB de dados).
- Depois de perder o banco e rodar `reindex`, `meeting_imports` está vazia e reimportar duplica.
