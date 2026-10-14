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

## S3 — apagar trecho, núcleo (PR 3)

- **C10** `zero_range`: mesma contagem, < -90 dBFS a 40 ms das bordas, pacotes fora da janela iguais, `.part` recusado não substitui. `cargo test -p fala-retention`
- **C11** falas que cruzam saem nos dois canais, marcador, renumeração, ponteiros, trechos que se cruzam viram um; transcrição nova e `reindex` mantêm o apagado. `cargo test -p fala-storage --test meeting_erasures`, `-p fala-notes`

## S4 — apagar trecho, app e UI (PR 4)

- **C12** o comando recusa gravando, transcrevendo, gerando notas, na lixeira e acima de 500
  falas; zera o WAV no lugar ou os Opus, depois o texto. `cargo test -p fala --lib meeting::erase`
- **C13** seleção primeira/última fala nos dois canais, contagem, confirmação com horários (com e
  sem áudio), faixa "A transcrição mudou". `bun src/components/meeting/erase.test.tsx`
- **C14** manual: reunião real, apagar 30 s, tocar antes e depois, regenerar notas
## S5 — retomar, núcleo (PR 5)

- **C15** só sessão encerrada, gravada e não importada abre tomada; a tomada começa no `recorded_ms`, fecha somando; a transcrição dela entra depois, com "Pessoa N" depois da maior, nomes mantidos, marcador "[gravação retomada às HH:MM]"; `reindex` traz as tomadas. `cargo test -p fala-storage --test meeting_takes`
- **C16** um trecho toca da tomada dele, cortado no início da seguinte; sem o áudio de todas, sem áudio. `cargo test -p fala --lib meeting::clip`

## S6 — retomar, app e UI (PR 6)

- **C17** só sessão encerrada, gravada, não importada, fora da lixeira e sem tomada aberta retoma; a tomada seguinte conta. `cargo test -p fala --lib meeting::tests::only_a_finished`
- **C18** tomada interrompida vira órfã e é recuperada como tomada. `cargo test -p fala --lib meeting::recovery`
- **C19** "Retomar" na barra (com aviso de terceiros), marcador na transcrição, faixa "A transcrição mudou". `bun src/components/meeting/resume.test.tsx`
- **C20** manual: parar, retomar, parar, tocar um trecho de cada tomada, regenerar notas
