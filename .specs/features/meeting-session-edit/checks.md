# meeting-session-edit checks (lixeira)

Profile: light
Plan: `.specs/features/meeting-session-edit/plan.md`

Windows 11 (Alienware), `CARGO_TARGET_DIR=C:\f\meeting-session-edit`.

## S1 — núcleo (PR 1)

**C1** - Mover grava `meeting_trash`, leva o `.md` para `Reuniões/.lixeira/` com `trashed_at`,
tira a sessão da busca; mover de novo mantém `trashed_at`; restaurar devolve o `.md` (com `(2)`
se o nome foi tomado) e a busca
Proof: `cargo test -p fala-storage --test meeting_trash`

**C2** - Round-trip da ADR-0006: apagar o banco e `reindex_meetings` traz a mesma lista, a mesma
lixeira com o mesmo `trashed_at` e a mesma busca; `.lixeira/` sem `trashed_at` entra com agora
Proof: `cargo test -p fala-storage --test meeting_trash reindex`

**C3** - Purga: `trash_due` devolve quem passou de 30 dias (29 d 23 h fica); apagar para sempre
tira o `.md` de `.lixeira/`
Proof: `cargo test -p fala-storage --test meeting_trash due`

**C4** - Desktop: mover apaga `audio/<id>/`, zera `audio_retained` e some da lista; a purga apaga
exatamente as vencidas; "Nada gravado" continua definitivo (é o `delete`)
Proof: `cargo test -p fala --lib meeting::lifecycle`

**C5** - MCP: sessão na lixeira some de `list_meetings`, `get_meeting`, `get_transcript_segment`
e das resources
Proof: `cargo test -p fala-cli --test mcp trashed`

**C6** - Comandos registrados e `bindings.ts` regenerado
Proof: `fala.exe --list-devices` regenera; `bunx tsc --noEmit`
