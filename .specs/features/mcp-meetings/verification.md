# mcp-meetings verification - fatia 1

Branch `feat/mcp-meetings`, Windows 11, `CARGO_TARGET_DIR=C:\f\mcp-meetings`.

| Check | Evidência |
| --- | --- |
| C1 | `meetings_are_off_until_their_own_key_is_true` ok |
| C2 | `list_meetings_hides_local_only_and_drafts_newest_first` ok: ordem pelo início (a 2 foi criada antes da 1 e gravou depois), gravação em curso de fora, cursor (início, id) estável com uma reunião nova entre as páginas |
| C3 | `get_meeting_returns_notes_without_the_transcript` ok |
| C4 | `meeting_responses_never_carry_audio_or_paths` ok |
| C5 | `deleted_meeting_disappears_mid_session` ok |
| C6 | `meeting_storage_failure_is_opaque_error` ok |
| C7 | `meetings_without_db_list_empty` ok |
| C8 | `cargo tree -p fala-mcp -e normal`: 0 linhas com hyper, reqwest, rustls, native-tls, oauth2 ou tauri |

`cargo test -p fala-cli --test mcp`: 43 passed. `cargo clippy -p fala-cli -p fala-mcp --all-targets
-- -D warnings` e `cargo fmt --all --check` limpos. Frontend (só texto de locale): `bun run lint`,
`bun run format:check`, `bunx tsc --noEmit`, `bun run check:translations` limpos;
`bun src/components/settings/about/privacy.test.tsx` C1-C10 ok.

Revisão `code-review high` do diff: 10 achados. Corrigidos: apagar entre a lista e os
participantes derrubava a página; ordem pela criação em vez do início; cursor por offset; gravação
em curso visível; docs citando a fatia 2; código morto. Mantidos: `transcript_segments` carrega os
trechos para contar e a lista filtra em memória (pedem API nova no `fala-storage`; o histórico é
pequeno); `source()` repete o mapeamento do storage de propósito, como lista fechada da saída.

Não testado: cliente MCP real (`claude mcp add`), para não mexer na config do Augusto, e reuniões
reais.
