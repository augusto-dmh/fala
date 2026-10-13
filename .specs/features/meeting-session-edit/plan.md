# meeting-session-edit — lixeira de 30 dias (PRs 1-2)

Profile: light · fase 4 · ADR-0005, 0006, 0010, 0014 · nenhuma contrariada

Apagar uma reunião era definitivo: um clique errado perdia transcrição, anotações e notas.

- **S1** (`feat/meeting-session-edit`): `meeting_trash` aditiva; mover apaga o áudio na hora e
  leva o `.md` para `Reuniões/.lixeira/` com `trashed_at`; `reindex` lê as duas pastas; lista,
  busca e MCP não veem a lixeira; purga de 30 dias na `meeting-upkeep`; três comandos.
- **S2** (`feat/meeting-session-edit-2`): "Mover para a lixeira" no "⋯", confirmação só com
  áudio, toast [Desfazer] 5 s, link "Lixeira (N)" e a tela.

Decisions: o `.md` muda de pasta escrevendo o novo e apagando a cópia velha só se for da sessão;
mover de novo não muda `trashed_at`; a busca filtra na consulta; mover emite `meeting-deleted`.
