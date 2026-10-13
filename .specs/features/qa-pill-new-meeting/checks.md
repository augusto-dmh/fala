# qa-pill-new-meeting — checks

Achado Q44 do QA de 2026-10-13 e o item "Nova reunião" do Q45: um botão "Nova reunião" no hover
da pílula em repouso e o mesmo item no menu do clique direito. Ação explícita (ADR-0005): a
gravação começa só com o clique, pelo mesmo caminho do item do tray, com o consentimento antes
quando ainda falta.

| ID | Check | Proof |
| --- | --- | --- |
| M1 | o botão aparece só no hover, cinza e centrado; o clique nele não abre a janela principal nem começa um arrasto; com o botão, o cartão é um grupo (não um botão com outro dentro) | `bun src/overlay/pillMeeting.test.tsx` M1 |
| M2 | `start_meeting_from_pill` só vale com a pílula em repouso, chama `start_from_tray` numa thread e abre Reuniões se a reunião não começou; a janela do hover tem 96 px de altura; `bindings.ts` regerado | `pillMeeting.test.tsx` M2, `cargo test -p fala --lib overlay` |
| M3 | o menu do clique direito tem "Gravar reunião" com o id `meeting_start` do tray | `pillMeeting.test.tsx` M3, `cargo test -p fala --lib tray` |
| M4 | portões | `bun run lint`, `bunx prettier --check .`, `bunx tsc --noEmit`, `bun run check:translations`, os testes de `src/overlay/`, `cargo fmt --all -- --check`, clippy e testes do workspace |
