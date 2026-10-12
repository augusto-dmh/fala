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

## Fatia 2

- **C9** `get_transcript_segment` pagina pelo `seq` (`from_segment`/`max_segments`, 200, até
  500), traz `transcribed_at` para o cliente notar uma retranscrição entre páginas, com o
  nome aplicado de `person_N`, `next_from_segment` nulo no fim e teto de 30.000 caracteres por
  página (um trecho sempre sai); argumento inválido = `-32602`; "só local" = não encontrada:
  `transcript_pages_by_segment_and_by_size`
- **C10** `resources/list` traz até 20 `fala://meeting/<id>` depois dos ditados, pelo início, e
  uma falha nas reuniões não esconde os ditados; `resources/read` devolve o mesmo JSON do
  `get_meeting`; "só local" e rascunho = não encontrada; com reuniões desligadas, nem o template
  nem a lista têm reunião e a leitura recusa (`templates_list_has_dictation_and_meeting_templates`):
  `meeting_resources_list_and_read_like_get_meeting`, `meetings_are_off_until_their_own_key_is_true`
- **C11** C4 e C6 valem também para a transcrição e as resources
