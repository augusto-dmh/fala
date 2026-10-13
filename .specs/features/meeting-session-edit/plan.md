# meeting-session-edit — lixeira de 30 dias (primeira leva, PRs 1-2)

Profile: light · fase 4 (`fase-4-editar-sessao`) · ADR-0005, 0006, 0010, 0014 · nenhuma contrariada

## Problem

Apagar uma reunião é definitivo e é a única edição que existe: um clique errado no "⋯" perde
transcrição, anotações e notas.

## Slices (PRs empilhados, pelo gate de 1000 linhas)

- **S1** (`feat/meeting-session-edit`): `meeting_trash` aditiva; mover apaga `audio/<id>/` na hora
  (travas do `delete`) e leva o `.md` para `Reuniões/.lixeira/` com `trashed_at`; o `reindex` lê as
  duas pastas; lista, busca e MCP não veem a lixeira; `Store::trashed_meetings`; purga de 30 dias
  na `meeting-upkeep`; comandos `trash_meeting`, `restore_meeting`, `list_trashed_meetings`.
- **S2** (`feat/meeting-session-edit-2`): menu "Mover para a lixeira"; confirmação só com áudio
  retido, senão toast [Desfazer] 5 s; link "Lixeira (N)"; tela com Restaurar / Apagar para sempre.

## Decisions

- O `.md` muda de pasta escrevendo o novo e apagando o velho (não `rename`): o mesmo caminho do
  espelho de sempre, e uma sincronização que achar cópia na outra pasta a apaga.
- `trashed_at` no frontmatter só na sessão da lixeira (chave opcional, como `import`); um `.md`
  em `.lixeira/` sem a chave entra com agora, e um em `Reuniões/` com a chave entra na lixeira.
- Mover de novo uma sessão já na lixeira não muda `trashed_at` (o prazo nunca recomeça por clique).
- A busca filtra na consulta (a linha de `meetings_fts` fica), para restaurar não reindexar.
- Mover para a lixeira emite `meeting-deleted`: a página tira a sessão como no apagar de hoje.
