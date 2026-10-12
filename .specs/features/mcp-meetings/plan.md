# mcp-meetings - reuniões no MCP local, só leitura, com interruptor próprio

Profile: light. Semana 2 do pitch da fase 4 (MCP local) sobre o `fala-mcp` do `mcp-local`.

## Problem

O MCP local entrega só ditados. Quem quer "os action items da reunião com a Ana" no Claude
ainda copia as notas à mão. A ADR-0010 já fixou as regras das reuniões: interruptor próprio,
desligado mesmo com o MCP ligado; sessão "só local" inexistente; nada de áudio nem caminho;
conteúdo marcado `content_origin: "third_party_speech"`.

## Slices (PRs empilhados, gate de 1000 linhas)

1. `feat/mcp-meetings`: `meetings = true` no `mcp.toml`; `list_meetings` e `get_meeting`.
2. `feat/mcp-meetings-2`: `get_transcript_segment` paginado e as resources `fala://meeting/{id}`.

`search_meetings` fica de fora: o storage não tem FTS de reunião (F5 `meeting-storage`), e uma
busca em memória no `fala-mcp` duplicaria a normalização do FTS5.

## Landing

| Door | Forma | Rejeitada |
| --- | --- | --- |
| consentimento | `meetings = true` (booleano TOML) ao lado de `enabled = true`; qualquer outra coisa desliga só as reuniões | chave aninhada `[meetings] enabled`: o painel de privacidade lê o TOML por linha |
| visibilidade | reunião existe para o MCP se gravou até o fim (`started_at` e `ended_at`) e não é "só local"; rascunho, gravação em curso e "só local" respondem como id inexistente | expor rascunhos (só pauta) ou a gravação em curso (o usuário ainda não encerrou) |
| campos | lista `{ id, title, started_at, ended_at, duration_s, source, participants, has_notes, transcribed }` e `content_origin` no topo; `get_meeting` troca `has_notes`/`transcribed` por `speakers` (só nomes aplicados), `annotations`, `notes`, `notes_edited`, `transcript_segments` e põe `content_origin` | serializar `MeetingRecord`: leva `audio_retained` e `local_only` |
| paginação | ordem pelo início, mais recente primeiro; `limit` 1..=50 (20); `cursor` opaco = chave `<início ms>:<id>` do último item; `since`/`until` RFC 3339 sobre o início | offset: desloca quando uma reunião entra ou sai entre páginas; data sem hora: o fuso fica ambíguo |
| erros | `meeting not found: <id>`; MCP desligado = o texto dos ditados; reuniões desligadas = `Fala MCP meetings are disabled: set meetings = true in mcp.toml in the Fala data folder` | - |
| URI (fatia 2) | `fala://meeting/{id}`, JSON igual ao `get_meeting` | `fala://reunioes/{id}` da pesquisa 17: destoa de `fala://dictation/` |

Apagar uma reunião (PR do `meeting-session-lifecycle`) apaga a linha; como o MCP lê a cada
chamada, ela some sem mudança aqui.
