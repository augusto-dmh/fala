# mcp-meetings checks

Plan: `.specs/features/mcp-meetings/plan.md`. Todos em `apps/cli/tests/mcp.rs`
(`cargo test -p fala-cli --test mcp`).

## Fatia 1

- **C1** reuniões desligadas sem `meetings`, com `meetings = false` ou `"yes"`; com o MCP
  desligado e `meetings = true`, o texto é o do MCP desligado; `tools/list` lista tudo mesmo
  assim: `meetings_are_off_until_their_own_key_is_true`
- **C2** a lista tem só as gravadas até o fim e não "só local", pelo início, mais recente
  primeiro, com as chaves exatas; `limit`/`cursor`/`since`/`until`; argumento inválido = `-32602`;
  cursor no fim = vazio; uma reunião nova entre duas páginas não desloca a seguinte:
  `list_meetings_hides_local_only_and_drafts_newest_first`
- **C3** `get_meeting` com as chaves exatas, notas, anotações, só o nome aplicado (a sugestão
  não), sem texto da transcrição; "só local", rascunho, em curso, inexistente e malformado dão
  `meeting not found: <id>`: `get_meeting_returns_notes_without_the_transcript`
- **C4** nenhuma linha do stdout tem o caminho da pasta, `audio`, `.opus`, `.wav`, `local_only`
  ou texto da sessão "só local"; o stderr no nível padrão não tem texto da transcrição:
  `meeting_responses_never_carry_audio_or_paths`
- **C5** uma reunião apagada no meio da sessão some da lista e do `get_meeting`:
  `deleted_meeting_disappears_mid_session`
- **C6** falha de banco = `storage error`, sem detalhe no stdout:
  `meeting_storage_failure_is_opaque_error`
- **C7** sem `fala.sqlite`, lista vazia e o arquivo não é criado: `meetings_without_db_list_empty`
- **C8** sem rede: `cargo tree -p fala-mcp -e normal` sem `hyper`, `reqwest`, `rustls`,
  `native-tls`, `oauth2` ou `tauri`
