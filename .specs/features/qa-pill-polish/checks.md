# qa-pill-polish — checks

Achados Q45 (itens do menu), Q46 e Q47 do QA de 2026-10-13. O "Nova reunião" no menu e no hover
(Q44) ficou fora desta rodada; o anel de contagem do aviso (Q47) também. O casing do menu entrou
no PR de strings. Diff de uma frase; sem plano.

| ID | Achado | Check | Proof |
| --- | --- | --- | --- |
| P1 | Q47 | o aviso de mudo diz qual microfone gravou: o backend lê o nome do microfone aberto antes do stop e o manda com o aviso; a pergunta vem antes do nome ("Microphone muted? No sound from <mic>."), então a reticência corta só o nome; sem nome, a frase genérica | `bun src/overlay/pillPolish.test.ts` P1, `notice.test.tsx` |
| P2 | Q46 | a pílula em repouso tem preenchimento cinza (`rgba(64,64,64,.85)`, contorno 0.7) e fica 10 px acima da borda, dentro da janela de 20 px; aberta no hover mantém os 6 px | `pillPolish.test.ts` P2, `pill.test.tsx` (pill-idle C6) |
| P3 | Q45 | o menu do clique direito ganha "Copy last dictation" (id do tray, mesmo handler) | `cargo test -p fala --lib tray::tests::pill_menu_items_and_their_handlers` |
| P4 | — | portões | `bun run lint`, `bunx prettier --check .`, `bunx tsc --noEmit`, `bun run check:translations`, os testes de `src/overlay/`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p fala --lib tray` |
