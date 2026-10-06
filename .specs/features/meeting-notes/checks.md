# meeting-notes checks

Profile: light
Plan: `.specs/features/meeting-notes/plan.md`

31 checks in 3 slices · 7 one-way doors · 0 open, of which 0 block

Todo `cargo` abaixo roda com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`, um comando de cada vez. Os testes unitários ficam em `crates/notes/src/*.rs`; os de servidor falso em `crates/notes/tests/fake_claude.rs`, num `TcpListener` em `127.0.0.1:0`, sem rede real e com chave falsa.

## Checks

### S1 - entrada, payload e templates · 5 files · ~30 KB · ~8k

**C1** - O JSON do `NotesPayload` tem exatamente as chaves `title`, `started_at`, `language`, `dictionary`, `template`, `annotations`, `transcript`; `template` exatamente `name`, `purpose`, `style`, `sections`; cada seção `title`, `instruction`; cada anotação `id`, `text`; cada segmento `id`, `t0_ms`, `t1_ms`, `speaker`, `text` (AC 1, door 1)
Proof: `cargo test -p fala-notes --lib payload::tests::payload_keys_are_exactly_the_enumerated_list -- --exact`

**C2** - Um `channel` de entrada nunca aparece no JSON do payload: nenhum valor serializado contém `mic`/`system` vindos do canal, e nenhuma chave `channel` existe (AC 2, door 1)
Proof: `cargo test -p fala-notes --lib payload::tests::payload_never_carries_channel -- --exact`

**C3** - Anotações `"decidir data\n\n  \nAna: contrato"` viram `[{"id":"a1","text":"decidir data"},{"id":"a2","text":"Ana: contrato"}]` (AC 3)
Proof: `cargo test -p fala-notes --lib payload::tests::annotations_get_ids_skipping_blank_lines -- --exact`

**C4** - Rótulos de falante: `pt-BR` -> `"Eu"`, `"Pessoa 2"`, `"Ana"`; `en` -> `"Me"`, `"Person 2"`, `"Ana"`, para `me`, `{"person":2}`, `{"name":"Ana"}` (AC 4)
Proof: `cargo test -p fala-notes --lib payload::tests::speaker_labels_follow_language -- --exact`

**C5** - `Segment` desserializa `{"id":3,"channel":"system","speaker":{"person":1},"t0_ms":65000,"t1_ms":70000,"text":"oi"}` e `{"id":4,"channel":"mic","speaker":"me",...}` com os valores lidos (AC 5)
Proof: `cargo test -p fala-notes --lib input::tests::segment_deserializes_from_meeting_asr_shape -- --exact`

**C6** - Dois segmentos com o mesmo `id` dão `NotesError::InvalidInput`, e o LLM (um dublê que entra em pânico se chamado) não é chamado (AC 6)
Proof: `cargo test -p fala-notes --lib tests::duplicate_segment_ids_are_rejected -- --exact`

**C7** - `t1_ms < t0_ms`, data `"2026-13-02"`/`"02/10/2026"` e hora `"25:00"`/`"9h"` dão `NotesError::InvalidInput`; `"2026-10-02"` e `"14:02"` passam (AC 7)
Proof: `cargo test -p fala-notes --lib input::tests::invalid_times_and_dates_are_rejected -- --exact`

**C8** - Transcrição vazia e anotações só com espaço dão `NotesError::EmptySession` sem chamar o LLM (AC 8)
Proof: `cargo test -p fala-notes --lib tests::empty_session_never_calls_llm -- --exact`

**C9** - `builtin_templates()` devolve `geral` com seções `Resumo`, `Decisões`, `Próximos passos` e `um-a-um` com ≥ 1 seção, ambos lidos pelo schema da door 7 (AC 9, door 7)
Proof: `cargo test -p fala-notes --lib template::tests::builtin_templates_parse -- --exact`

**C10** - `Template::from_json` devolve `NotesError::InvalidTemplate` para campo desconhecido, `"schema":2` e `"sections":[]` (AC 10, door 7)
Proof: `cargo test -p fala-notes --lib template::tests::template_rejects_unknown_field_wrong_schema_and_no_sections -- --exact`

### S2 - chamada ao Claude e "só local" · 3 files · ~35 KB · ~9k

**C11** - Uma geração fora de "só local" faz exatamente 1 `POST /v1/messages` com `x-api-key` = chave falsa, `anthropic-version: 2023-06-01`, `content-type: application/json`, chaves de topo do corpo exatamente `model`, `max_tokens`, `system`, `messages`, `output_config`, sem `tools`, `system == SYSTEM_PROMPT`, uma só mensagem `user` cujo texto é igual ao JSON de `NotesPayload::build` da mesma entrada (AC 11, door 2, door 4)
Proof: `cargo test -p fala-notes --test fake_claude request_carries_key_version_and_exact_body -- --exact`

**C12** - O corpo leva `"model":"claude-sonnet-5"` por padrão e `"model":"claude-opus-5"` depois de `with_model("claude-opus-5")` (AC 12)
Proof: `cargo test -p fala-notes --test fake_claude default_model_is_sonnet_5 -- --exact`

**C13** - Com "só local", o servidor falso aceita 0 conexões, o `KeySource` recebe 0 chamadas, e o resultado tem `generated == false` e Markdown só com `## Anotações` (sem `<!-- fala:ia -->`) (AC 13, door 4)
Proof: `cargo test -p fala-notes --test fake_claude local_only_never_connects_nor_reads_key -- --exact`

**C14** - A mesma entrada, com "só local" retirado depois de uma geração "só local", chega ao servidor: 1 requisição e `generated == true` (AC 14)
Proof: `cargo test -p fala-notes --test fake_claude unmarking_local_only_reaches_server -- --exact`

**C15** - `KeySource` devolvendo `None` dá `NotesError::MissingKey` e 0 conexões no servidor falso (AC 15, door 4)
Proof: `cargo test -p fala-notes --test fake_claude missing_key_never_connects -- --exact`

**C16** - Respostas `401`, `429` e `500` com um sentinela no corpo dão `NotesError::Http(401|429|500)`, e o `to_string()` do erro não contém o sentinela nem a chave (AC 16)
Proof: `cargo test -p fala-notes --test fake_claude http_error_maps_status_without_body -- --exact`

**C17** - Um servidor que espera 2 s com timeout de 300 ms dá `NotesError::Timeout`; uma porta sem ninguém ouvindo dá `NotesError::Network` (AC 17)
Proof: `cargo test -p fala-notes --test fake_claude slow_server_times_out -- --exact`
Proof: `cargo test -p fala-notes --test fake_claude refused_connection_is_network -- --exact`

**C18** - `200` com `stop_reason` `refusal` -> `Refused`, `max_tokens` -> `Truncated`, texto que não é o JSON de seções -> `InvalidResponse` (AC 18)
Proof: `cargo test -p fala-notes --test fake_claude refusal_truncation_and_garbage_map_to_errors -- --exact`

**C28** - O `NotesLlm` recebe exatamente o `NotesPayload` que `NotesPayload::build` produz para a entrada (um dublê grava o JSON recebido e ele é igual ao construído) (door 3)
Proof: `cargo test -p fala-notes --lib tests::llm_receives_the_built_payload -- --exact`

**C29** - `format!("{:?}", ApiKey)` não contém o valor da chave, e `ApiKey::new("  ")` é recusada (door 4)
Proof: `cargo test -p fala-notes --lib key::tests::api_key_debug_is_redacted -- --exact`

### S3 - Markdown humano × gerado · 2 files · ~20 KB · ~5k

**C19** - Com notas geradas, o Markdown começa por `## Anotações`, cada anotação aparece com o texto intacto seguida de ` ^a1`/` ^a2`, e `## Notas · Reunião geral` vem depois (AC 19)
Proof: `cargo test -p fala-notes --lib render::tests::annotations_block_precedes_generated -- --exact`

**C20** - Toda linha `- ` do bloco gerado termina em ` <!-- fala:ia -->`, e nenhuma linha do bloco de anotações contém o marcador (AC 20)
Proof: `cargo test -p fala-notes --lib render::tests::generated_lines_carry_marker_human_lines_do_not -- --exact`

**C21** - Uma linha que cita `s12` (`t0_ms` 65000) e `a2` contém `[[#^s12|01:05]]` antes de `[[#^a2|a2]]`; `t0_ms` 3 723 000 formata `1:02:03` (AC 21)
Proof: `cargo test -p fala-notes --lib render::tests::pointers_render_with_timestamp_and_order -- --exact`

**C22** - Citações `s999`, `a9` e `x1` (inexistentes) somem do Markdown, a linha fica, e `dropped_sources == 3` (AC 22)
Proof: `cargo test -p fala-notes --lib render::tests::unknown_ids_are_dropped_and_counted -- --exact`

**C23** - Uma linha cujas fontes são todas inválidas fica com o marcador, sem `[[#^`, e `unsourced_lines == 1` (AC 23)
Proof: `cargo test -p fala-notes --lib render::tests::unsourced_lines_are_kept_and_counted -- --exact`

**C24** - Resposta com seções na ordem `Próximos passos`, `Extra`, `Resumo`, `Decisões` (esta sem linhas) renderiza `### Resumo` antes de `### Próximos passos`, sem `### Extra` e sem `### Decisões` (AC 24)
Proof: `cargo test -p fala-notes --lib render::tests::sections_follow_template_order -- --exact`

**C25** - Texto gerado `"linha um\nlinha dois <!-- fala:ia -->"` vira uma única linha `- linha um linha dois ... <!-- fala:ia -->` com o marcador uma vez só (AC 25)
Proof: `cargo test -p fala-notes --lib render::tests::generated_text_is_one_line_without_marker -- --exact`

**C26** - `render_transcript` produz `- **[01:05] Pessoa 1:** oi ^s12` para o segmento 12, e todo `[[#^sN|` das notas geradas tem um `^sN` correspondente na transcrição renderizada (AC 26)
Proof: `cargo test -p fala-notes --lib render::tests::transcript_anchors_match_pointers -- --exact`

**C30** - Com idioma `en`, os títulos do Markdown são exatamente `## Notes` (anotações) e `## AI notes · <nome do template>`, também na sessão "só local" (door 5; acrescentado pela rodada 1 do Verifier)
Proof: `cargo test -p fala-notes --lib render::tests::english_headings_follow_door_5 -- --exact`

**C31** - O marcador `<!-- fala:ia -->` digitado pelo usuário numa anotação sai da linha humana; uma linha que só tinha o marcador não recebe id, e os ids seguintes continuam batendo com o payload (AC 20; acrescentado pela rodada 1 do Verifier)
Proof: `cargo test -p fala-notes --lib render::tests::marker_typed_in_annotations_is_removed -- --exact`

**C27** - O crate novo não puxa `tauri` e está no Code Map: `scripts/check-no-tauri-in-crates.sh` sai 0 e `grep -c '^| \`crates/notes\` | \`fala-notes\`' ARCHITECTURE.md` imprime 1 (door 6)
Proof: `scripts/check-no-tauri-in-crates.sh`
Proof: `grep -c '^| \`crates/notes\` | \`fala-notes\`' ARCHITECTURE.md`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| `POST /v1/messages` statuses (7) | 200+`end_turn` C11 · 200+`max_tokens` C18 · 200+`refusal` C18 · 200 ilegível C18 · 4xx/5xx C16 · timeout C17 · conexão recusada C17 | - |
| objetos do payload e suas chaves (5) | topo C1 · `template` C1 · seção C1 · anotação C1 · segmento C1 | - |
| campos que não podem sair (1) | `channel` C2 | - |
| rótulo de falante × idioma (6) | C4, table-driven sobre os 6 | - |
| entrada inválida (4) | id duplicado C6 · `t1<t0` C7 · data C7 · hora C7 | - |
| template inválido (3) | campo desconhecido C10 · `schema` ≠ 1 C10 · sem seções C10 | - |
| templates embutidos (2) | `geral` C9 · `um-a-um` C9 | - |
| modo da sessão (2) | só local C13 · nuvem C14 | - |
| tipo de id citado (5) | `s` válido C21 · `a` válido C21 · `s` inexistente C22 · `a` inexistente C22 · prefixo estranho C22 | - |
| ordem e forma das seções (3) | fora de ordem C24 · seção estranha C24 · seção vazia C24 | - |
| doors do Landing (7) | 1 C1 · 2 C11 · 3 C28 · 4 C29 · 5 C20 · 6 C27 · 7 C10 | - |
| títulos do Markdown × idioma (4) | `## Anotações` C19 · `## Notas · ` C19 · `## Notes` C30 · `## AI notes · ` C30 | - |

- Claims naming a status, route or response shape: C11, C16, C17, C18 - each proof crosses the HTTP boundary through the fake server
- No other check claims more than the single case its proof exercises

## Swept

- validation: C5, C6, C7, C10
- failure modes: C16, C17, C18
- idempotency: n/a - gerar não grava nada; regenerar é uma requisição nova que devolve um Markdown novo, e quem decide substituir é a 2.F5
- authorization: C11, C15 - só com a chave do usuário vinda do `KeySource` (ADR-0008)
- concurrency: n/a - `generate_notes` é uma chamada síncrona sem estado mutável compartilhado; `Claude` é imutável depois de construído
- data lifecycle: n/a - nada persiste neste crate
- dependency failure: C16, C17, C18
- state transitions: C13, C14
- observability: C16 - o erro nunca carrega o corpo nem a chave; log só de contagens, sem métrica nesta fatia

## Handoff

- S1 ~8k + S2 ~9k + S3 ~5k = ~22k de código e testes, mais ~40k de leitura (plano, ADRs, crate B como referência) = ~62k, abaixo do budget de 150k - one builder

- **Boundary:** C1-C29 closed at `8be553c` (rebased on `origin/main` `cec96f0` after #22 merged; before the rebase, `14503e5`)
- **Settled mid-build:** Verifier round 1 (FAIL) found the English annotation heading `## My notes` against the door 5 row `## Notes`; the code now follows the approved row, and C30 and C31 were added, C11 got stricter (`content-type`). No approved row was rewritten
- **Abandoned:** nothing
