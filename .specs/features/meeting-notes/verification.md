# meeting-notes verification

**Verdict**: PASS
**Profile**: light
**Diff range**: cec96f0..f019601 (HEAD)
**Round**: 3 - scoped
**Verifier**: independent sub-agent (author != verifier)

Os 31 checks estão provados em `f019601` com evidência localizada. O único gap da rodada 2 era a
terceira cláusula do C31 ("os ids seguintes continuam batendo com o payload"), que não tinha
asserção. O commit `f019601` acrescenta 11 linhas ao próprio teste do C31 e não mexe em mais nada.
Essas linhas constroem o `NotesPayload` a partir da mesma entrada com o marcador digitado e
assertam `payload["annotations"] == [{"id":"a1","text":"decidir data"},{"id":"a2","text":"Ana: contrato"}]`
(`crates/notes/src/render.rs:456-459`). As linhas 440-443 do mesmo teste assertam o Markdown
`decidir data ^a1` / `Ana: contrato ^a2`. Se o payload deixasse de passar por `annotation_lines`,
o marcador iria no texto de `a1` e a linha que só tem o marcador viraria `a2`. A asserção nova
falharia nos dois casos.

## Histórico

- **Rodada 1** (sobre `14503e5`, que virou `8be553c` depois do rebase): FAIL com três gaps. O título
  `en` do bloco humano era `## My notes`, e a linha da door 5 pede `## Notes`. O marcador
  `<!-- fala:ia -->` digitado numa anotação não saía da linha humana. O `content-type` do C11 não
  era asserido. A correção acrescentou o C30 e o C31 e deixou o C11 mais estrito.
- **Rodada 2** (sobre `8160d5f`, escopada): FAIL com um gap. Os três gaps da rodada 1 estavam
  resolvidos e 30 dos 31 checks estavam provados. O C31 afirmava que os ids "batem com o payload",
  mas a prova só observava o Markdown renderizado. Nenhum teste construía o `NotesPayload` com o
  marcador digitado.
- **Rodada 3** (sobre `f019601`, escopada): este relatório.

## Escopo da rodada 3

O fix (`git diff 8160d5f..f019601`) toca só `crates/notes/src/render.rs`, com 11 linhas inseridas
depois da linha 448, no fim de `marker_typed_in_annotations_is_removed`. Nenhum código de produção
mudou. As provas rodaram de novo por inteiro em `f019601`: uma invocação `--lib`, uma
`--test fake_claude` e as duas provas do C27. O C31 está `verified at f019601`, com citações
renovadas. Os outros checks dizem `carried from 8160d5f`, e as provas deles também rodaram de novo
e passaram. Em `render.rs`, toda citação da rodada 2 fica antes da linha 449 e não mudou de linha.
Conferi isso com `rg -n 'fn (annotations_block_precedes_generated|...|english_headings)' crates/notes/src/render.rs`
(linhas 252, 264, 287, 307, 329, 343, 360, 377, 410 e 427, iguais às da rodada 2).

## Binding sources

Perfil `light`: o passo 1 não é obrigatório. O fix só acrescenta um teste e não toca a interface.
Seção `carried from 8160d5f`.

| Source | Opened | Contradiction | Uncovered |
| --- | --- | --- | --- |
| ADR-0016 (proposta), `git show origin/docs/propose-adrs-0010-0016:docs/decisions/0016-llm-de-notas-de-reuniao-payload-enumerado.md` | yes (rodada 2, carried from 8160d5f) | none | - |
| design doc `docs/design/2026-10-fala-v1.md` §3.4 item 4 | yes (rodada 1, carried from 8160d5f) | none | - |

## Checks

Provas rodadas em `f019601`. Cada teste nomeado aparece individualmente como `ok` na saída (ver
Gate). O teste do C31 está no diff `cec96f0..f019601`, em `crates/notes/src/render.rs:427`.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | chaves do payload exatamente a lista enumerada | `--lib` `payload::tests::payload_keys_are_exactly_the_enumerated_list ... ok` | carried from 8160d5f: `crates/notes/src/payload.rs:168` - `assert_eq!(keys(&json), set(&["title","started_at","language","dictionary","template","annotations","transcript"]))`; `payload.rs:181`, `:188`, `:193`, `:198` sub-objetos | PASS |
| C2 | `channel` nunca sai | `--lib` `payload::tests::payload_never_carries_channel ... ok` | carried from 8160d5f: `crates/notes/src/payload.rs:218` - `assert!(!all_keys.contains(&"channel"))`; `payload.rs:220` para `mic`/`system`/`local_only` | PASS |
| C3 | anotações com id, linhas em branco puladas | `--lib` `payload::tests::annotations_get_ids_skipping_blank_lines ... ok` | carried from 8160d5f: `crates/notes/src/payload.rs:230` - `assert_eq!(json["annotations"], json!([{"id":"a1","text":"decidir data"},{"id":"a2","text":"Ana: contrato"}]))` | PASS |
| C4 | rótulos de falante × idioma (6) | `--lib` `payload::tests::speaker_labels_follow_language ... ok` | carried from 8160d5f: `crates/notes/src/payload.rs:251` - `assert_eq!(json["transcript"][0]["speaker"], expected)` sobre a tabela de 6 casos | PASS |
| C5 | `Segment` lê a forma da 2.F4 mais `id` | `--lib` `input::tests::segment_deserializes_from_meeting_asr_shape ... ok` | carried from 8160d5f: `crates/notes/src/input.rs:153` - `assert_eq!(system, Segment { id: 3, channel: Channel::System, speaker: Speaker::Person(1), ... })`; `input.rs:168-169` - `mic.channel == Channel::Mic`, `mic.speaker == Speaker::Me` | PASS |
| C6 | id duplicado -> `InvalidInput`, LLM não chamado | `--lib` `tests::duplicate_segment_ids_are_rejected ... ok` | carried from 8160d5f: `crates/notes/src/lib.rs:162` - `assert!(matches!(generate_notes(&input, &NeverCalled), Err(NotesError::InvalidInput(_))))` | PASS |
| C7 | `t1<t0`, data e hora inválidas -> `InvalidInput` | `--lib` `input::tests::invalid_times_and_dates_are_rejected ... ok` | carried from 8160d5f: `crates/notes/src/input.rs:186` - `matches!(backwards.validate(), Err(NotesError::InvalidInput(_)))`; `input.rs:194` datas; `input.rs:202` horas; `input.rs:181` - `assert_eq!(input().validate(), Ok(()))` | PASS |
| C8 | sessão vazia -> `EmptySession` sem LLM | `--lib` `tests::empty_session_never_calls_llm ... ok` | carried from 8160d5f: `crates/notes/src/lib.rs:173` - `assert_eq!(generate_notes(&input, &NeverCalled), Err(NotesError::EmptySession))` | PASS |
| C9 | `geral` e `um-a-um` pelo schema | `--lib` `template::tests::builtin_templates_parse ... ok` | carried from 8160d5f: `crates/notes/src/template.rs:69` - `assert_eq!(ids, ["geral", "um-a-um"])`; `template.rs:75` títulos | PASS |
| C10 | template inválido -> `InvalidTemplate` | `--lib` `template::tests::template_rejects_unknown_field_wrong_schema_and_no_sections ... ok` | carried from 8160d5f: `crates/notes/src/template.rs:108` - `matches!(Template::from_json(&json), Err(NotesError::InvalidTemplate(_)))` | PASS |
| C11 | 1 POST com chave, versão, `content-type`, corpo exato, `system`, mensagem = payload | `--test fake_claude` `request_carries_key_version_and_exact_body ... ok` | carried from 8160d5f: `crates/notes/tests/fake_claude.rs:234` - `assert_eq!(requests.len(), 1)`; `:236` `"POST /v1/messages HTTP/1.1"`; `:237` `x-api-key == TEST_KEY`; `:238` `anthropic-version == "2023-06-01"`; `:239` - `assert_eq!(request.header("content-type"), Some("application/json"))`; `:242-245` chaves de topo; `:246` sem `tools`; `:247` `system == SYSTEM_PROMPT`; `:249-250` uma mensagem `user`; `:252` - `assert_eq!(messages[0]["content"], expected.as_str())` | PASS |
| C12 | `claude-sonnet-5` por padrão, trocável | `--test fake_claude` `default_model_is_sonnet_5 ... ok` | carried from 8160d5f: `crates/notes/tests/fake_claude.rs:281` - `assert_eq!(models, [json!("claude-sonnet-5"), json!("claude-opus-5")])` | PASS |
| C13 | "só local": 0 conexões, 0 chaves, só anotações | `--test fake_claude` `local_only_never_connects_nor_reads_key ... ok` | carried from 8160d5f: `crates/notes/tests/fake_claude.rs:291` - `connections() == 0`; `:293` - `keys_source.calls() == 0`; `:294` `!notes.generated`; `:295` - `assert_eq!(notes.markdown, "## Anotações\n\ndecidir data ^a1\n\nAna: contrato ^a2\n")`; `:299` sem `GENERATED_MARKER` | PASS |
| C14 | desmarcar "só local" chega ao servidor | `--test fake_claude` `unmarking_local_only_reaches_server ... ok` | carried from 8160d5f: `crates/notes/tests/fake_claude.rs:310` - `connections() == 0`; `:312` - `generate_notes(...).unwrap().generated`; `:313` - `assert_eq!(server.requests().len(), 1)` | PASS |
| C15 | chave ausente -> `MissingKey`, 0 conexões | `--test fake_claude` `missing_key_never_connects ... ok` | carried from 8160d5f: `crates/notes/tests/fake_claude.rs:320` - `assert_eq!(generate_notes(...), Err(NotesError::MissingKey))`; `:324` - `connections() == 0` | PASS |
| C16 | 401/429/500 -> `Http(status)` sem corpo nem chave | `--test fake_claude` `http_error_maps_status_without_body ... ok` | carried from 8160d5f: `crates/notes/tests/fake_claude.rs:338` - `assert_eq!(error, NotesError::Http(status))` para `[401, 429, 500]` (`:330`); `:340` sem `SENTINELA`; `:341` sem `TEST_KEY` | PASS |
| C17 | lento -> `Timeout`; porta fechada -> `Network` | `--test fake_claude` `slow_server_times_out ... ok`, `refused_connection_is_network ... ok` | carried from 8160d5f: `crates/notes/tests/fake_claude.rs:356` - `assert_eq!(generate_notes(&input(), &claude), Err(NotesError::Timeout))`; `:366` - `Err(NotesError::Network)` | PASS |
| C18 | `refusal`/`max_tokens`/ilegível -> `Refused`/`Truncated`/`InvalidResponse` | `--test fake_claude` `refusal_truncation_and_garbage_map_to_errors ... ok` | carried from 8160d5f: `crates/notes/tests/fake_claude.rs:390` - `assert_eq!(generate_notes(...), Err(expected))` sobre `:372` `Refused`, `:375` `Truncated`, `:379` e `:381` `InvalidResponse` | PASS |
| C19 | `## Anotações` com ` ^a1`/` ^a2` antes de `## Notas · Reunião geral` | `--lib` `render::tests::annotations_block_precedes_generated ... ok` | carried from 8160d5f (linha inalterada, antes da inserção em 449): `crates/notes/src/render.rs:255` - `assert!(markdown.starts_with("## Anotações\n"))`; `render.rs:260` - `assert!(annotations < first && first < second && second < generated)` | PASS |
| C20 | linha gerada termina no marcador; humana nunca o tem | `--lib` `render::tests::generated_lines_carry_marker_human_lines_do_not ... ok` | carried from 8160d5f (linha inalterada): `crates/notes/src/render.rs:277` - `assert!(line.ends_with(" <!-- fala:ia -->"))`; `render.rs:282` - `assert!(!line.contains(GENERATED_MARKER))` | PASS |
| C21 | `[[#^s12&#124;01:05]]` antes de `[[#^a2&#124;a2]]`; `1:02:03` | `--lib` `render::tests::pointers_render_with_timestamp_and_order ... ok` | carried from 8160d5f (linha inalterada): `crates/notes/src/render.rs:297` - `assert_eq!(line, "- Ana cuida do contrato. [[#^s12&#124;01:05]] [[#^a2&#124;a2]] <!-- fala:ia -->")`; `render.rs:301` - `assert_eq!(timestamp(3_723_000), "1:02:03")` | PASS |
| C22 | ids desconhecidos descartados, `dropped_sources == 3` | `--lib` `render::tests::unknown_ids_are_dropped_and_counted ... ok` | carried from 8160d5f (linha inalterada): `crates/notes/src/render.rs:312` - `assert_eq!(notes.dropped_sources, 3)`; `render.rs:319` linha mantida; `render.rs:324` ids ausentes | PASS |
| C23 | linha sem fonte mantida, `unsourced_lines == 1` | `--lib` `render::tests::unsourced_lines_are_kept_and_counted ... ok` | carried from 8160d5f (linha inalterada): `crates/notes/src/render.rs:331` - `assert_eq!(notes.unsourced_lines, 1)`; `render.rs:338` - `assert_eq!(line, "- Clima bom na reunião. <!-- fala:ia -->")` | PASS |
| C24 | ordem do template, sem seção estranha ou vazia | `--lib` `render::tests::sections_follow_template_order ... ok` | carried from 8160d5f (linha inalterada): `crates/notes/src/render.rs:353` - `assert!(resumo < passos)`; `render.rs:354` sem `### Extra`; `render.rs:356` sem `### Decisões` | PASS |
| C25 | texto gerado em uma linha, marcador uma vez | `--lib` `render::tests::generated_text_is_one_line_without_marker ... ok` | carried from 8160d5f (linha inalterada): `crates/notes/src/render.rs:369` - `assert_eq!(generated, ["- linha um linha dois [[#^s12&#124;01:05]] <!-- fala:ia -->"])`; `render.rs:373` - `matches(GENERATED_MARKER).count() == 1` | PASS |
| C26 | `render_transcript` com âncora; todo ponteiro tem âncora | `--lib` `render::tests::transcript_anchors_match_pointers ... ok` | carried from 8160d5f (linha inalterada): `crates/notes/src/render.rs:386` - `assert_eq!(render_transcript(...), "- **[01:05] Pessoa 1:** oi ^s12\n")`; `render.rs:400` âncora por ponteiro; `render.rs:406` - `assert_eq!(cited, 3)` | PASS |
| C27 | sem `tauri` em `crates/`; linha no Code Map | `scripts/check-no-tauri-in-crates.sh` exit 0 (`ok: no tauri in crates/`); `grep -c ...` imprimiu `1` | carried from 8160d5f, provas re-rodadas em f019601: `ARCHITECTURE.md:25` - `&#124; \`crates/notes\` &#124; \`fala-notes\` &#124; notas de reunião: ...`; `scripts/check-no-tauri-in-crates.sh:12-14` itera `crates/*/Cargo.toml` | PASS |
| C28 | o `NotesLlm` recebe exatamente o payload construído | `--lib` `tests::llm_receives_the_built_payload ... ok` | carried from 8160d5f: `crates/notes/src/lib.rs:186` - `assert_eq!(*llm.0.borrow(), vec![expected])` | PASS |
| C29 | `Debug` da chave redigido; chave em branco recusada | `--lib` `key::tests::api_key_debug_is_redacted ... ok` | carried from 8160d5f: `crates/notes/src/key.rs:49` - `assert_eq!(debug, "ApiKey([REDACTED])")`; `key.rs:51` - `assert_eq!(ApiKey::new("  "), Err(NotesError::MissingKey))` | PASS |
| C30 | `en`: `## Notes` e `## AI notes · <template>`, também em "só local" | `--lib` `render::tests::english_headings_follow_door_5 ... ok` | carried from 8160d5f (linha inalterada): `crates/notes/src/render.rs:422` - `assert_eq!(headings, ["## Notes", "## AI notes · Reunião geral"])`; `render.rs:423` - `assert_eq!(local_only(&input).markdown.lines().next(), Some("## Notes"))` | PASS |
| C31 | marcador digitado sai da linha humana; linha só com o marcador sem id; ids seguintes batem com o payload | `--lib` `render::tests::marker_typed_in_annotations_is_removed ... ok` | verified at f019601: entrada `render.rs:429-430` - `format!("decidir data {GENERATED_MARKER}\n{GENERATED_MARKER}\nAna: contrato")`; `crates/notes/src/render.rs:440-443` - `assert_eq!(human...filter(non-empty), [&"decidir data ^a1", &"Ana: contrato ^a2"])` (linha só com o marcador sem id); `render.rs:445` - `assert!(!line.contains(GENERATED_MARKER))` (marcador sai da linha humana); `render.rs:449-455` - `NotesPayload::build(&input)...to_json()` da mesma `input`; `crates/notes/src/render.rs:456-459` - `assert_eq!(payload["annotations"], serde_json::json!([{"id":"a1","text":"decidir data"},{"id":"a2","text":"Ana: contrato"}]))` (ids e textos do payload batem com `^a1`/`^a2` do Markdown); `render.rs:447` `[[#^a2&#124;a2]]`; `render.rs:448` `dropped_sources == 0` | PASS |

## Coverage

Perfil `light`: o recálculo não é obrigatório. A linha das cláusulas do C31, que reprovou a rodada
2, foi recalculada (`verified at f019601`). As outras linhas estão `carried from 8160d5f`.

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| cláusulas do C31 (3) | `checks.md` C31 (verified at f019601) | marcador sai da linha humana `render.rs:445` · linha só com o marcador sem id `render.rs:440-443` · ids batem com o payload `render.rs:456-459` | - |
| títulos do Markdown × idioma, door 5 (4) | `plan.md` Landing door 5 (carried from 8160d5f) | `## Anotações` C19 (`render.rs:255`) · `## Notas · ` C19 (`render.rs:260`) · `## Notes` C30 (`render.rs:422-423`) · `## AI notes · ` C30 (`render.rs:422`) | - |
| `POST /v1/messages` statuses (7) | `plan.md` Surface (carried from 8160d5f) | 200+`end_turn` C11 · `max_tokens` C18 · `refusal` C18 · ilegível C18 · 4xx/5xx C16 · timeout C17 · recusada C17 | - |
| headers da Surface (3) | `plan.md` Surface (carried from 8160d5f) | `fake_claude.rs:237` · `:238` · `:239` (C11) | - |
| rótulo de falante × idioma (6) | `payload.rs:114-122` (carried from 8160d5f) | C4 tabela com os 6 | - |

## Swept

`carried from 8160d5f`. Nenhuma linha `Swept` de `checks.md` resolve para `existing`. O fix só
acrescenta asserções a um teste.

## Observations (não decidem o veredito)

- Carried from 8160d5f: `annotation_lines` troca o marcador por `" "` e apara as pontas, mas não
  colapsa o espaço de dentro (`crates/notes/src/input.rs:97`). Então `"a <!-- fala:ia --> b"` vira
  `"a   b"`, no bloco humano e no payload. O teste cobre só o marcador no fim da linha e a linha que
  só tem o marcador.
- Carried from 8160d5f: a tabela de Coverage de `checks.md` ainda liga a door 5 só ao C20
  (`checks.md:127`). Quem fecha os títulos é a linha `checks.md:128`.
- Perfil `light`: não houve injeção de falha. Pela leitura da asserção nova, ela falharia se o
  payload deixasse de filtrar o marcador ou numerasse a linha que só tem o marcador. Não executei
  esse mutante.
- Clippy e `cargo fmt` não rodaram nesta verificação (ficam fora das provas dos checks).

## Gate

`CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2 cargo test -p fala-notes --lib` - 24 passed, 0 failed (os 22 testes nomeados em `--lib`, entre eles `render::tests::marker_typed_in_annotations_is_removed`, mais `render::tests::local_only_renders_only_annotations` e `render::tests::rejects_text_that_is_not_the_sections_json`)

`CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2 cargo test -p fala-notes --test fake_claude` - 9 passed, 0 failed (os 9 nomeados)

`scripts/check-no-tauri-in-crates.sh` - exit 0 (`ok: no tauri in crates/`); `grep -c '^| \`crates/notes\` | \`fala-notes\`' ARCHITECTURE.md` - `1`

Total: 33 passed, 0 failed. 31/31 checks com evidência localizada.
