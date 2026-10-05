# postproc verification

**Verdict**: PASS
**Profile**: standard
**Diff range**: ba3b6df3de76924f99a8ee2ccf01e9f1727694a7..971006c (`ba3b6df..971006c`, 3 commits: feat(secrets) 9dfde1a, feat(postproc) 511cc6a, feat(cli) 971006c)
**Round**: 3 - scoped
**Verifier**: independent sub-agent (author != verifier)

Escopo da rodada 3: o diff `8d906d8d..971006c` e os vereditos que não foram PASS na rodada 2. O
diff toca só blocos `#[cfg(test)]` de `apps/cli/src/key.rs` e `apps/cli/src/format.rs` (helpers
`key_bytes`/`format_bytes`, os testes novos `non_utf8_stdin_exits_2` e uma asserção nova em
`format::tests::refused_keyring_exits_1`) e o `checks.md` (C41 por emenda aditiva, linhas de
Coverage recontadas ou anotadas como superadas). Nenhum arquivo de produção mudou:
`git diff 8d906d8d 971006c -- crates/ apps/cli/src/main.rs | wc -l` = 0, e `plan.md`,
`ARCHITECTURE.md` e `smoke-keyring.sh` também não mudaram. `git diff c349263 971006c -- '*.toml'
Cargo.lock | wc -l` = 0.

Os 41 checks têm prova verde em `971006c` com asserção localizada. Os 3 gaps da rodada 2 estão
fechados (C41 e a asserção nova do C37), e os 5 mutantes desta rodada morreram, inclusive os dois
que a rodada 2 deixou sem mutante por causa do teto (metade "dicionário" do C38 e `--disable-app`
do C40). A linha de Test policy do CLI passa a ser atendida.

## Binding sources

carried from 8d906d8d (que por sua vez carregou de c349263). O fix não tocou a interface:
`plan.md` sem mudança em `8d906d8d..971006c`, e `Surface`, `Relations` e `Landing` iguais. O C41
só prova uma causa de "uso inválido" que o Surface já previa (`plan.md:79`, saída 2).

| Source | Opened | Contradiction | Uncovered |
| --- | --- | --- | --- |
| `docs/decisions/0004-pos-processamento-por-llm-na-nuvem-so-texto.md` | yes - rodada 1 (c349263); a leitura de "bruto" está em `plan.md:191` | none | - |
| `docs/decisions/0008-distribuicao-byok-sem-backend-assinatura-e-updater-adiados.md` | yes - rodada 1 (c349263) | none | - |
| `docs/decisions/0007-windows-primeiro-linux-gnome-depois-sem-macos.md` | yes - rodada 1 (c349263) | none | - |
| `docs/decisions/0002-tauri-2-com-workspace-sem-tauri-no-core.md` | yes - rodada 1 (c349263) | none | - |
| `AGENTS.md` | yes - carregado pelo `CLAUDE.md` nesta rodada | none | - |
| https://ai.google.dev/api/generate-content | yes - rodada 1 (c349263) | none | - |
| https://ai.google.dev/gemini-api/docs/api-key | yes - rodada 1 (c349263) | none | - |
| https://ai.google.dev/gemini-api/docs/models | yes - rodada 1 (c349263) | none | - |

Conferido de novo nesta rodada (verified at 971006c): os 3 commits de `ba3b6df..971006c` têm só o
trailer `Assisted-by: Claude Code` (sem `Co-Authored-By` nem `Signed-off-by`). O código novo desta
rodada está todo dentro de `mod tests`, então o `unwrap` em `format_bytes`/`key_bytes` é permitido.

## Checks

verified at 971006c. As provas rodaram na árvore real em `971006c` com
`CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`, uma invocação por crate:

- `cargo test -p fala-secrets -- --include-ignored`: exit 0. lib: 9 passed, incluindo as 2
  `#[ignore]` contra o gnome-keyring (`keyring_store_round_trips`, `keyring_entry_uses_fala_service`).
- `cargo test -p fala-postproc`: exit 0. lib: 8 passed; `tests/fake_gemini.rs`: 14 passed (10,57 s).
- `cargo test -p fala-cli --bin fala-cli`: exit 0. 29 passed, 15 deles desta feature (6 em
  `key::tests`, 9 em `format::tests`); os 2 novos são `key::tests::non_utf8_stdin_exits_2 ... ok`
  e `format::tests::non_utf8_stdin_exits_2 ... ok`.
- `bash .specs/features/postproc/smoke-keyring.sh` (provedor `teste_cli`, valor `nao-e-chave`):
  exit 0, `ok: set, lookup, status e delete contra o Secret Service`. Depois dele,
  `secret-tool lookup service br.com.augusto.fala username teste_cli` sai com 1.
- C30 e C31: exit 0. C32 é carried from c349263, porque nenhuma entrada do `cargo deny` mudou
  (`git diff c349263 971006c -- '*.toml' Cargo.lock` vazio).

Cada teste nomeado aparece individualmente como `... ok` na saída.

Incidente de ambiente, registrado porque mudou o que foi rodado: a primeira invocação desta rodada
deu `maps_other_keyring_errors_to_store ... FAILED` (`left: InvalidProvider`) e
`normalizes_spacing_and_capitalizes ... FAILED` (`left: "Sim ; não"`), que são exatamente os
mutantes F5 e F1 da rodada 2, com a árvore real limpa (`rules.rs:15` com `';'`, `lib.rs:122` com
`_ => SecretError::Store`). O cargo não recompilou: os binários de teste em
`target/debug/deps/fala_secrets-e99a1c9994d9c7f8` e `fala_postproc-be4b040202ffac2d` tinham
mtime 20:49:59 e 20:50:11, a hora das falhas da rodada 2 no rascunho `verify-wt2`. Com o
`CARGO_TARGET_DIR` compartilhado, o mesmo crate em dois worktrees gera o mesmo artefato e o mesmo
fingerprint (caminhos relativos ao workspace), e as fontes da árvore real, mais antigas que o
binário mutante, deixam o artefato "fresh". Corrigi só atualizando o mtime das fontes
(`touch` nos `*.rs` de `crates/secrets`, `crates/postproc` e `apps/cli`; conteúdo e
`git status --porcelain` iguais), e a segunda invocação mostrou `Compiling fala-secrets`,
`Compiling fala-postproc` e `Compiling fala-cli` com caminho da árvore real e tudo verde. Os
resultados acima são os da segunda invocação. Depois das falhas desta rodada, repeti o `touch` e as
3 invocações para não deixar mutante no target compartilhado (ver Faults injected).

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | 10 fillers pt-BR removidos, sem caixa, com `,`/`.` | `rules::tests::removes_pt_br_fillers ... ok` | `crates/postproc/src/rules.rs:223` - `assert_eq!(pt("então, hã, eu acho"), "Então, eu acho")`; `rules.rs:224-242` tabela literal dos 10 (carried from 8d906d8d: arquivo sem mudança) | PASS |
| C2 | En remove só `uh` `uhm` `hmm` e mantém os 7 | `rules::tests::en_keeps_pt_only_fillers ... ok` | `rules.rs:249` - `assert_eq!(en("ahn ok"), "Ahn ok")`; `rules.rs:250-258` 3 removidos e 7 mantidos | PASS |
| C3 | 3 ou mais repetições viram 1; 2 ficam | `rules::tests::collapses_three_or_more_repeats ... ok` | `rules.rs:264-266` - `"eu eu eu acho"→"Eu acho"`, `"Eu EU eu eu acho"→"Eu acho"`, `"o que que é"→"O que que é"` | PASS |
| C4 | 5 variantes de grafia do dicionário | `rules::tests::dictionary_replaces_spelling_variants ... ok` | `rules.rs:271-275` - `assert_eq!(with_dict("a charge bee e o itau."), "A ChargeBee e o Itaú.")` e as outras 4 | PASS |
| C5 | não junta através dos 6 sinais | `rules::tests::dictionary_does_not_join_across_punctuation ... ok` | `rules.rs:280-285` - `for sign in [',', '.', ';', ':', '?', '!'] { assert_eq!(with_dict(&format!("charge{sign} bee")), format!("Charge{sign} bee")) }` | PASS |
| C6 | prefixo/sufixo não casa | `rules::tests::dictionary_ignores_partial_words ... ok` | `rules.rs:291-292` - `"chargebeex"→"Chargebeex"`, `"xchargebee"→"Xchargebee"` | PASS |
| C7 | espaços, espaço antes dos 6 sinais, maiúscula acentuada | `rules::tests::normalizes_spacing_and_capitalizes ... ok` | `rules.rs:297` - `assert_eq!(pt("  eu   acho ,  que sim .  "), "Eu acho, que sim.")`; `rules.rs:298-303` laço literal dos 6 sinais; `rules.rs:305` - `"é isso"→"É isso"` | PASS |
| C8 | vazio vira `""`, `Editor::Rules`, 0 conexões | `empty_input_returns_empty_without_request ... ok` | `crates/postproc/tests/fake_gemini.rs:167-170` - `final_text == ""`, `editor == Editor::Rules`, `server.requests().len() == 0` | PASS |
| C9 | 16 palavras: 1 request, resposta aparada, `Llm`; desligado ou sem chave: 0 | `long_text_uses_llm_response ... ok`, `llm_off_or_no_key_skips_request ... ok` | `fake_gemini.rs:178-180` - `requests().len()==1`, `final_text=="Texto formatado."`, `Editor::Llm`; `:197-200` - `SIXTEEN_RULES`, `Editor::Rules`, `requests().len()==0` | PASS |
| C10 | 15 palavras: 0 requests, `Rules` | `fifteen_words_skip_llm ... ok` | `fake_gemini.rs:207-213` - contagem 15, texto literal, `Editor::Rules`, `requests().len()==0` | PASS |
| C11 | `slack` desligado sem caixa; `Slackware` não | `disabled_app_skips_llm ... ok` | `fake_gemini.rs:225-229` - `Editor::Rules`/0 requests para `slack`; `Editor::Llm`/1 request para `Slackware` | PASS |
| C12 | corpo só com os 3 campos, prompt+dicionário, template user, `temperature: 0`, sem `hã` | `payload_has_only_text_app_and_dictionary ... ok` | `fake_gemini.rs:250-253` chaves `== ["contents","generationConfig","systemInstruction"]`; `:254-257` prompt + dicionário; `:266-269` `== "<app>Slack</app>\n<ditado>{SIXTEEN_RULES}</ditado>"`; `:270-273` `== json!({"temperature": 0})`; `:279` `!has_filler(body)` | PASS |
| C13 | request line sem `?`, chave só no header, troca de modelo | `key_only_in_header ... ok` | `fake_gemini.rs:303-310` - `"POST /v1beta/models/gemini-2.5-flash-lite:generateContent HTTP/1.1"` e o de `3.5`; `:312-321` - sem `?`, `header("x-goog-api-key")==Some(TEST_KEY)`, chave fora da linha e do corpo | PASS |
| C14 | 3 s vira `timeout` entre 1,9 e 2,5 s; 1,5 s é usado | `slow_response_falls_back_at_two_seconds ... ok`, `response_under_two_seconds_is_used ... ok` | `fake_gemini.rs:333-337` - `fallback==Some(Fallback::Timeout)`, `elapsed >= 1900ms && < 2500ms`; `:344-345` - `Editor::Llm`, `"Texto formatado."` | PASS |
| C15 | 6 falhas caem nas regras com motivo | `failures_fall_back_to_rules ... ok` | `fake_gemini.rs:356-379` tabela; `:391-393` - `Editor::Rules`, `SIXTEEN_RULES`, `fallback==Some(expected)`; `:395-398` `to_string()` de cada motivo | PASS |
| C16 | erro não ecoa ditado nem chave | `errors_do_not_echo_text_or_key ... ok` | `fake_gemini.rs:407` `Fallback::Http(400)`; `:415-416` - `!shown.contains("relatório")`, `!shown.contains(TEST_KEY)` | PASS |
| C17 | set/get e sobrescrita, em memória e no keyring real | `tests::memory_store_round_trips ... ok`, `tests::keyring_store_round_trips ... ok` | `crates/secrets/src/lib.rs:172,174` - `get == Some(key("primeira"))`, depois `Some(key("segunda"))`; `lib.rs:288-296` o mesmo no `KeyringStore` | PASS |
| C18 | delete deixa None; delete de ausente dá Ok | `tests::delete_then_get_is_none ... ok`, `tests::keyring_store_round_trips ... ok` | `lib.rs:181-184` - `delete==Ok(())`, `get==None`, delete repetido e de ausente `==Ok(())`; `lib.rs:297-299` no keyring real | PASS |
| C19 | 6 ids inválidos dão `InvalidProvider` nas 3 operações; 3 aceitos | `tests::rejects_invalid_provider_ids ... ok` | `lib.rs:192-202` - `Err(SecretError::InvalidProvider)` em get/set/delete; `:204` `store.is_empty()`; `:207-208` `Ok(())` para os 3 | PASS |
| C20 | chave vazia dá `EmptyKey` | `tests::api_key_rejects_empty ... ok` | `lib.rs:214-215` - `ApiKey::new("")` e `("  \t")` `== Err(SecretError::EmptyKey)` | PASS |
| C21 | 3 erros viram `Unavailable`; `NoEntry` vira None/Ok | `tests::maps_keyring_errors ... ok` | `lib.rs:223-234` - `NoStorageAccess`, `PlatformFailure`, `NoDefaultStore` `== SecretError::Unavailable`; `:235-236` `get_result(NoEntry)==Ok(None)`, `delete_result(NoEntry)==Ok(())` | PASS |
| C22 | Debug redigido; sem Display/Serialize; service/account | `tests::api_key_debug_is_redacted ... ok`, `tests::keyring_entry_uses_fala_service ... ok` | `lib.rs:275` - `== "ApiKey([REDACTED])"`; `:271` - `assert_not_impl_any!(ApiKey: fmt::Display, serde::Serialize)`; `:306-309` - `get_specifiers() == Some(("br.com.augusto.fala", "gemini"))` | PASS |
| C23 | `key set` grava, sai 0, não ecoa | `key::tests::set_stores_and_never_echoes ... ok` | `apps/cli/src/key.rs:172-178` - `out.code == 0`, `store.get("gemini") == Some(ApiKey::new("k-sentinela"))`, `!out.stdout.contains("k-sentinela")`, `!out.stderr.contains("k-sentinela")` | PASS |
| C24 | stdin vazio sai 2 com store vazio | `key::tests::set_empty_stdin_exits_2 ... ok` | `key.rs:184-188` - `for stdin in ["", "\n"]`: `out.code == 2`, `store.is_empty()` | PASS |
| C25 | status definida/ausente, delete, provedor inválido sai 2 | `key::tests::status_delete_and_invalid_provider ... ok` | `key.rs:196,200,206` - `(0,"ausente\n")`, `(0,"definida\n")`, `(0,"ausente\n")`; `:203-207` delete sai 0, também com ausente; `:209-213` `Gemini` sai 2 em set/status/delete e o store fica vazio | PASS |
| C26 | `format` imprime o texto final e `editor: regras`; com fallback ainda sai 0 | `format::tests::prints_final_text_and_editor ... ok`, `format::tests::fallback_still_exits_0 ... ok` | `apps/cli/src/format.rs:198-200` - `code==0`, `stdout=="Eu acho que a ChargeBee\n"`, `editor: regras`; `:211-217` - `code==0`, texto das regras, `editor: regras`, `fallback: rede` | PASS |
| C27 | `--llm` sem chave sai 2 com mensagem e 0 conexões | `format::tests::llm_without_key_exits_2 ... ok` | `format.rs:228-236` - `code==2`, stderr contém `fala-cli key set gemini`, `stdout==""`, `accept()` dá `WouldBlock` | PASS |
| C28 | keyring indisponível sai 1 em `key *` e `format --llm` | `key::tests::unavailable_keyring_exits_1 ... ok`, `format::tests::unavailable_keyring_exits_1 ... ok` | `key.rs:224-225` - `code==1`, `keyring indisponível` para set/status/delete; `format.rs:242-248` - `code==1`, `keyring indisponível`, `stdout==""` | PASS |
| C29 | o binário real grava no gnome-keyring | `bash .specs/features/postproc/smoke-keyring.sh` exit 0, saída `ok: set, lookup, status e delete contra o Secret Service` | `.specs/features/postproc/smoke-keyring.sh:17-18` - `[ "$stored" = "$value" ]` sobre `secret-tool lookup`; `:19,21` status `definida`/`ausente`; `:22-25` entrada removida | PASS |
| C30 | linha do Code Map | o `grep -nE` do C30 sobre `ARCHITECTURE.md` (texto exato em `checks.md`), exit 0 | `ARCHITECTURE.md:23` - a linha da tabela com `crates/secrets`, `fala-secrets` e `chaves de API no keyring do SO (ApiKey, SecretStore), ADR-0008` | PASS |
| C31 | sem tauri e sem cfg de SO | `scripts/check-no-tauri-in-crates.sh` e o `! grep -rnE` de `cfg(windows / target_os / unix)` do C31 sobre `crates/secrets crates/postproc`, exit 0, saída `ok: no tauri in crates/` | o grep não acha nada em `crates/secrets` nem em `crates/postproc` | PASS |
| C32 | `cargo deny check` | carried from c349263 (exit 0 lá); entradas do deny sem mudança em `c349263..971006c` | `Cargo.lock:3478-3479` - `name = "keyring"`, `version = "4.2.0"` (lock igual ao da rodada 1); saída da rodada 1: `advisories ok, bans ok, licenses ok, sources ok` | PASS |
| C33 | resposta em 3 s vira `LateEdit` com o texto, 1 conexão | `late_response_is_delivered_as_late_edit ... ok` | `fake_gemini.rs:429-435` - 1,9-2,5 s, `Editor::Rules`, `Some(Fallback::Timeout)`; `:437` `late.wait() == Ok("Texto tardio.")`; `:438` `requests().len()==1` | PASS |
| C34 | prazo tardio de 3 s: 4 falhas tardias | `late_failures_yield_fallback ... ok` | `fake_gemini.rs:444-452` tabela; `:460` `assert_eq!(late_edit.wait(), Err(expected))` | PASS |
| C35 | nenhuma `LateEdit` nos 4 casos | `no_late_edit_unless_timeout ... ok` | `fake_gemini.rs:474,478,485,490` - `out.late_edit.is_none()` | PASS |
| C36 | os outros 7 erros do keyring viram `Store`, também em get e delete | `tests::maps_other_keyring_errors_to_store ... ok` | `crates/secrets/src/lib.rs:248-256` - os 7 variantes literais; `:259` - `assert_eq!(map_keyring_error(error), SecretError::Store, "{shown}")`; `:261-268` - `get_result(Err(BadEncoding(..))) == Err(SecretError::Store)`, `delete_result(Err(TooLong(..))) == Err(SecretError::Store)` | PASS |
| C37 | `Store` faz `key set/status/delete` e `format --llm` saírem 1, com `keyring indisponível` e sem o valor do stdin | `key::tests::refused_operation_exits_1 ... ok`, `format::tests::refused_keyring_exits_1 ... ok` | `apps/cli/src/key.rs:245-247` - `out.code == 1`, `out.stderr.contains("keyring indisponível")`, `!out.stderr.contains("valor")` nos 3 (stdin de `set` é `"valor\n"`, `key.rs:240`); `apps/cli/src/format.rs:298-305` - `out.code == 1`, `out.stderr.contains("keyring indisponível")`, `assert!(!out.stderr.contains("relatório"), ...)` (novo, `format.rs:304`; o stdin é `SIXTEEN`, que contém `relatório`, `format.rs:133-134`), `out.stdout == ""` | PASS |
| C38 | `--dictionary` inexistente e `--lang es` saem 2 com stdout vazio | `format::tests::unreadable_dictionary_or_bad_language_exits_2 ... ok` | `format.rs:324-325` - `out.code == 2`, `out.stdout == ""` (dicionário); `format.rs:327-328` - idem para `--lang es` | PASS |
| C39 | `--llm` com chave e resposta `Texto do LLM.`: sai 0, stdout, `editor: llm`, sem `fallback:`, 1 request | `format::tests::llm_answer_prints_editor_llm ... ok` | `format.rs:339-343` - `code == 0`, `stdout == "Texto do LLM.\n"`, `stderr.contains("editor: llm")`, `!stderr.contains("fallback:")`, `seen.len() == 1` | PASS |
| C40 | flags chegam ao processamento: `--app`/`--model` na request, `--disable-app` corta, `--lang` muda os fillers | `format::tests::flags_reach_the_postprocessor ... ok` | `format.rs:365-370` - `seen.len()==1`, `seen[0].0 == "POST /v1beta/models/gemini-3.5-flash-lite:generateContent HTTP/1.1"`, `seen[0].1.contains("<app>Slack</app>")`; `:386-388` - `code==0`, `editor: regras`, `seen.len()==1` (0 requests novas); `:391` `pt.stdout == "Ok\n"`; `:393` `en.stdout == "Ahn ok\n"` | PASS |
| C41 | stdin não UTF-8: `key set gemini` sai 2 com store vazio; `format` sai 2, stdout vazio, stderr fala do stdin | `key::tests::non_utf8_stdin_exits_2 ... ok`, `format::tests::non_utf8_stdin_exits_2 ... ok` | `apps/cli/src/key.rs:232-234` - `key_bytes(&store, &["set", "gemini"], &[0xff, 0xfe, b'\n'])`, `assert_eq!(out.code, 2)`, `assert!(store.is_empty())`; `apps/cli/src/format.rs:310-313` - `format_bytes(.., &[], &[0xff, 0xfe, b'\n'])`, `assert_eq!(out.code, 2)`, `assert_eq!(out.stdout, "")`, `assert!(out.stderr.contains("stdin"))` | PASS |

Notas (não mudam veredito):

1. O comentário `/// Um Gemini falso ...` que a rodada 2 achou preso ao `type Seen` agora está na
   `fn answering_server` (`format.rs:254`).
2. Em `key set`, o stdin não UTF-8 cai em `map_err(|_| SecretError::EmptyKey)` (`key.rs:61-63`), e
   o `report` imprime `erro: stdin vazio; passe a chave na primeira linha do stdin`
   (`key.rs:90-95`). O código de saída é o que o C41 pede, mas a mensagem diz "vazio" para um stdin
   que não está vazio. O C41 não fixa o texto do stderr de `key set`, então isto é UX, não falha.
3. No `checks.md` o C41 aparece entre C37 e C38 (`checks.md:145`), fora da ordem numérica. É só
   cosmético.

## Coverage

Recalculada nesta rodada (verified at 971006c) para as linhas que a rodada 2 deixou com membro
sem prova e para as linhas cuja autoridade o fix tocou: o conjunto de saídas 2 do `format` e do
`key set` (ganhou o membro "stdin ilegível") e as flags do `format` (mutante novo em
`--disable-app`). As outras são carried from 8d906d8d, que já citava a origem de cada uma.

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| fillers pt-BR (10) | carried from c349263 | C1 | - |
| fillers com `Language::En` (10) | carried from c349263 | C2 | - |
| `Language` (2) | carried from c349263 | `PtBr` C1 · `En` C2 | - |
| repetição (3 bordas) | carried from c349263 | C3 | - |
| variantes do dicionário (5) | carried from c349263 | C4 | - |
| sinais de fechamento (6) | carried from 8d906d8d | os 6 em `rules.rs:280` (C5) e `rules.rs:298` (C7) | - |
| limiar de palavras (2) | carried from c349263 | 15 C10 · 16 C9 | - |
| condições do LLM (4) | carried from c349263 | C9, C10, C11 | - |
| campos e formas do corpo (3 + 2 + 2) | carried from c349263 | C12 | - |
| motivos de fallback (4) | carried from c349263 | C14, C15 | - |
| destino da resposta (3) e resultado da `LateEdit` (5) | carried from c349263 | C14, C33, C34 | - |
| casos sem `LateEdit` (4) | carried from c349263 | C35 | - |
| `SecretStore` ops (3) × impls (2) | carried from c349263 | C17, C18 | - |
| variantes de `SecretError` (4) | carried from 8d906d8d (`crates/secrets/src/lib.rs` sem mudança) | `InvalidProvider` C19 · `EmptyKey` C20, C24, C41 · `Unavailable` C21, C28 · `Store` C36, C37 | - |
| `keyring::Error` mapeados (11) | carried from 8d906d8d (`keyring` 4.2.0, `Cargo.lock` sem mudança) | 4 em C21 · 7 em C36 | - |
| `fala-cli key set` saída 2 (3 causas) | verified at 971006c - `execute` em `apps/cli/src/key.rs:58-65` e `report` em `key.rs:85-101`: `validate_provider` (`:59`), erro de leitura (`:61-63`), `ApiKey::new` vazio (`:64`) | provedor inválido C25 (`key.rs:209-211`) · stdin vazio C24 (`key.rs:187-188`) · stdin ilegível C41 (`key.rs:233-234`); G1 morre | - |
| `fala-cli key status/delete` saídas (3 cada) | verified at 971006c - `key.rs:68-78`; os dois não leem o stdin, então não há causa nova | 0 C25 · 1 C28, C37 · 2 C25 (`key.rs:212-213`) | - |
| `fala-cli key set` saídas 0 e 1 | verified at 971006c | 0 C23 · 1 C28 (`key.rs:224`), C37 (`key.rs:245`) | - |
| C37 "o stderr nunca traz o valor lido do stdin" (4 comandos) | verified at 971006c - o próprio texto do C37 (`checks.md:141`) | `key set`, `key status`, `key delete`: `key.rs:247` `!out.stderr.contains("valor")` · `format --llm`: `format.rs:304` `!out.stderr.contains("relatório")` com stdin `SIXTEEN`; G3 morre | - |
| `fala-cli format` saída 2 (4 causas) | verified at 971006c - `run` em `apps/cli/src/format.rs:49-52`, `:56-62`, `:74-79`, `:94-96`. O braço `Err(error) => 2` em `format.rs:85-88` não é alcançável: `SecretStore::get("gemini")` com id válido só devolve `Unavailable` ou `Store`, que o braço de `:81` pega | `--llm` sem chave C27 (`format.rs:228`) · dicionário ilegível C38 (`format.rs:324`; G4 morre) · idioma inválido C38 (`format.rs:327`) · stdin ilegível C41 (`format.rs:311`; G2 morre) | - |
| `fala-cli format` saídas 0 e 1 | verified at 971006c | 0 C26, C39 · 1 C28 (`format.rs:242`), C37 (`format.rs:298`) | - |
| linha `editor:` do `format` (2) | carried from 8d906d8d (produção sem mudança) | `regras` C26 (`format.rs:200`) · `llm` C39 (`format.rs:341`) | - |
| linha `fallback:` do `format` (4 motivos) | carried from c349263 | Display dos 4 em `fake_gemini.rs:395-398`; impressão com `rede` em C26 (`format.rs:217`); ausência em C39 (`format.rs:342`) | - |
| flags de `format` (7) | verified at 971006c - `FormatArgs` em `format.rs:15-38` | `--llm` C26, C39 · `--dictionary` C26 (`:194`), C38 (`:324`; G4 morre) · `--app` C40 (`:370`) · `--model` C40 (`:366-369`) · `--disable-app` C40 (`:387-388`; G5 morre) · `--lang` C38 (`:327`), C40 (`:391`, `:393`; F8 da rodada 2) · `--gemini-base-url` C26, C39 | - |
| Landing doors (6) | carried from c349263 | C12-C15, C22, C29-C35 | - |
| montagem do store (2 assemblies) | carried from c349263 (`apps/cli/src/main.rs` sem mudança) | `main.rs` com `KeyringStore` (C29) · testes do CLI com `MemoryStore`/`UnavailableStore`/`RefusingStore` | - |

Level: carried from 8d906d8d. C41 chama o `run` do subcomando com um leitor de bytes injetado,
o mesmo nível de C23-C28 e C37-C40. Não há level gap.

Sobre o artefato: a emenda do `checks.md` agora marca as linhas antigas
(`erros de SecretError ... (3)`, `erros do keyring mapeados ... (4)`, `fala-cli format statuses ...
(3)`) como superadas pelas recontadas. Isso resolve a observação da rodada 2.

## Test policy rows

verified at 971006c para a linha 3, que não foi atendida na rodada 2 e classifica os arquivos que o
fix tocou. As linhas 1, 2 e 4 são carried from 8d906d8d: os arquivos que elas classificam
(`crates/postproc/src/*`, `crates/secrets/src/lib.rs`) não mudaram em `8d906d8d..971006c`, e as
provas delas rodaram de novo verdes nesta rodada.

| Row | Files it classifies | Required proof | Expectation met |
| --- | --- | --- | --- |
| Decide, alcançado por uma fronteira (HTTP) | `crates/postproc/src/lib.rs`, `crates/postproc/src/gemini.rs` | fronteira `fake_gemini.rs` (C8-C16, C33-C35), própria camada `gemini.rs:152-161` | yes - carried from 8d906d8d |
| Decide, sem fronteira (regras, validação de id, mapeamento de erro) | `crates/postproc/src/rules.rs`, `crates/secrets/src/lib.rs` | própria camada: `rules::tests::*`, `tests::rejects_invalid_provider_ids`, `tests::maps_keyring_errors`, `tests::maps_other_keyring_errors_to_store` | yes - carried from 8d906d8d (uma asserção por membro: `lib.rs:223-236`, `:259`; `rules.rs:280`, `:298`) |
| Ponto de entrada que decide pouco (subcomandos do CLI) | `apps/cli/src/key.rs`, `apps/cli/src/format.rs` | handler com store e fluxos injetados: `key::tests::*`, `format::tests::*` | yes - entrada aceita: C23, C25, C26, C39, C40. Cada rejeição: provedor inválido C25, stdin vazio C24, `--llm` sem chave C27, dicionário C38, idioma C38. Cada caminho de erro: `Unavailable` C28, `Store` C37, stdin ilegível em `key set` (`key.rs:61-63`) e em `format` (`format.rs:94-96`) C41. O braço `format.rs:85-88` é inalcançável (ver Coverage). G1-G5 morrem |
| Instrumentação (`KeyringStore` repassando ao `keyring`) | `crates/secrets/src/lib.rs:78-99` | C17b e C29 contra o cofre real | yes - rodaram de novo nesta rodada contra o gnome-keyring (`--include-ignored` e o smoke) |

## Swept existing

carried from c349263. O fix não tocou `crates/postproc/src/lib.rs`: concurrency (`Formatter: Send +
Sync`, `thread::spawn` + `mpsc`) e observability (um `log::debug!` só com o motivo) continuam onde
a rodada 1 os achou.

## Faults injected

verified at 971006c. Worktree de rascunho
`git worktree add --detach <scratchpad>/verify-wt3 HEAD` (HEAD `971006c`), mesmo
`CARGO_TARGET_DIR`, uma falha por vez aplicada por script com substituição exata (uma ocorrência),
`cargo test -p fala-cli --bin fala-cli <teste>` e `git checkout -- .` entre elas. O
`git status --porcelain` da árvore real foi o mesmo antes e depois (`?? .specs/features/desktop-keys/`
e `?? .specs/features/postproc/verification.md`). Escopo: as 3 lacunas da rodada 2 (G1, G2, G3) e,
dentro do teto de 5, a metade "dicionário" do C38 (G4) e o `--disable-app` do C40 (G5).

| Mutation | Location | Killed |
| --- | --- | --- |
| G1: erro de leitura do stdin em `key set` vira `SecretError::Unavailable` em vez de `EmptyKey` (o caso que a rodada 2 apontou) | `apps/cli/src/key.rs:63` | yes - `key::tests::non_utf8_stdin_exits_2` FAILED em `key.rs:233` (`left: 1`, `right: 2`; C41) |
| G2: `format` ignora o erro de leitura do stdin (`let _ = stdin.read_to_string(&mut text);` no lugar do `if let Err .. return 2`) | `apps/cli/src/format.rs:94-97` | yes - `format::tests::non_utf8_stdin_exits_2` FAILED na asserção `out.code == 2` (`format.rs:311` do HEAD; linha 308 no mutante, que tem 3 linhas a menos), `left: 0`, `right: 2` (C41) |
| G3: o erro de keyring do `format --llm` lê o stdin e o ecoa (`"erro: keyring indisponível ({error}): {seen}"`) | `apps/cli/src/format.rs:81-83` | yes - `format::tests::refused_keyring_exits_1` FAILED na asserção nova `!out.stderr.contains("relatório")` (`format.rs:304` do HEAD; linha 306 no mutante, 2 linhas a mais) (C37) |
| G4: dicionário ilegível vira dicionário vazio (`Err(_) => Dictionary::default()`) em vez de sair 2 | `apps/cli/src/format.rs:56-63` | yes - `format::tests::unreadable_dictionary_or_bad_language_exits_2` FAILED na asserção do dicionário (`format.rs:324` do HEAD; 317 no mutante, 7 linhas a menos), `left: 0`, `right: 2` (C38) |
| G5: `--disable-app` ignorado (`disabled_apps: Vec::new()`) | `apps/cli/src/format.rs:101` | yes - `format::tests::flags_reach_the_postprocessor` FAILED em `format.rs:387` (`out.stderr.contains("editor: regras")`; C40) |

Os 5 mutantes morreram, e cada um derrubou uma asserção diferente. As facetas `--app` e `--model`
do C40 (`format.rs:366-370`) seguem sem mutante próprio por causa do teto de 5. As duas comparam
valores literais (request line inteira e `<app>Slack</app>`), então isto é um limite desta rodada,
não um mutante sobrevivente.

Limpeza do target compartilhado: removi o rascunho com `git worktree remove --force`
(`git worktree list` não o mostra mais). Como o incidente do início desta rodada mostrou que um
binário mutante pode ficar "fresh" para a árvore real, repeti o `touch` nas fontes de
`crates/secrets`, `crates/postproc` e `apps/cli` e rodei de novo, na árvore real,
`cargo test -p fala-cli --bin fala-cli` (29 passed, com `Compiling fala-secrets`,
`Compiling fala-postproc` e `Compiling fala-cli` da árvore real), `cargo test -p fala-secrets`
(7 passed, 2 ignored) e `cargo test -p fala-postproc --lib` (8 passed). O `git status --porcelain`
continuou igual.

## Gate

- `cargo test -p fala-secrets -- --include-ignored`: 9 passed, 0 failed
- `cargo test -p fala-postproc`: 22 passed (8 lib + 14 `fake_gemini`), 0 failed
- `cargo test -p fala-cli --bin fala-cli`: 29 passed (15 desta feature), 0 failed
- `bash .specs/features/postproc/smoke-keyring.sh`: exit 0. C30 e C31: exit 0. C32 carried from
  c349263 (entradas do deny sem mudança)
- Faults: 5 injetadas, 5 mortas, 0 sobreviventes
- Gaps da rodada 2: os 3 fechados (stdin ilegível em `key set`: C41/G1; em `format`: C41/G2;
  "nunca o valor do stdin" no `format --llm`: `format.rs:304`/G3)
- `python3 /home/augusto/.claude/skills/tlc-spec-lean/scripts/validate_verification.py postproc`:
  exit 0 (0 errors, 0 warnings)
