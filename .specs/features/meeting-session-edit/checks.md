# meeting-session-edit checks (lixeira)

Profile: light · Windows 11, `CARGO_TARGET_DIR=C:\f\meeting-session-edit`

## S1 — núcleo (PR 1)

- **C1** mover grava `meeting_trash`, leva o `.md` para `.lixeira/` com `trashed_at` e tira da
  busca; restaurar devolve (com `(2)` se o nome foi tomado). `cargo test -p fala-storage --test meeting_trash`
- **C2** apagar o banco e `reindex_meetings` traz a mesma lista, lixeira, `trashed_at` e busca;
  `.lixeira/` sem `trashed_at` entra com agora. Mesmo teste.
- **C3** `trash_due` vence aos 30 dias (29 d 23 h fica). Mesmo teste.
- **C4** mover apaga `audio/<id>/` e some da lista; a purga apaga só as vencidas e não uma
  restaurada. `cargo test -p fala --lib meeting::lifecycle`
- **C5** o MCP não vê a lixeira. `cargo test -p fala-cli --test mcp trashed`
- **C6** comandos e `bindings.ts` regenerado. `fala.exe --list-devices`

## S2 — UI (PR 2)

- **C7** confirmação só com áudio; mover mostra o toast [Desfazer] 5 s. `bun src/components/meeting/trash.test.tsx` (T1, T2)
- **C8** tela: título, data, "apagada há N dias", Restaurar e Apagar para sempre, rodapé dos
  30 dias, "Nada na lixeira", link "Lixeira (N)". Mesmo arquivo (T3, T4)
- **C9** manual: mover, desfazer, restaurar e apagar para sempre no app de debug portátil
