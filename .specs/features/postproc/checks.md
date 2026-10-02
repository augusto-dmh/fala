# postproc - checks

Profile: standard
Plan: `.specs/features/postproc/plan.md`

Perfil `standard` nesta feature (o projeto declara `light`): o teste de payload é a confirmação
da ADR-0004, e o `light` não pega um teste que passa sob a implementação errada. Decidido pelo
Lux em 2026-10-02.

35 checks in 5 slices · 6 one-way doors (C33-C35 e a door 6 por emenda aditiva de 2026-10-02, pedida pelo Lux) · 1 open (blocks go-live, não bloqueia o build)

Todo `cargo` com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`.
Provas unitárias e do servidor falso rodam no CI. As provas contra o keyring real são
`#[ignore]` (C17b, C22b, C29) e rodam à mão nesta máquina (gnome-keyring ativo), sempre com
provedores de teste (`teste_*`), nunca com chave real.

## Checks

### S1 - regras locais pt-BR · 3 files · 40 KB · ~10k

Prova: `cargo test -p fala-postproc --lib rules::tests::<nome>`.

**C1** - Com `Language::PtBr`, cada um dos 10 tokens `hã` `hãã` `ãh` `ahn` `hum` `humm` `hmm` `uh` `uhm` `éé` é removido como palavra inteira, sem caixa, com um `,` ou `.` final; `então, hã, eu acho` → `Então, eu acho` (AC 1)
Proof: `cargo test -p fala-postproc --lib rules::tests::removes_pt_br_fillers`

**C2** - Com `Language::En`, `uh` `uhm` `hmm` saem e os 7 tokens só pt-BR (`hã` `hãã` `ãh` `ahn` `hum` `humm` `éé`) ficam; `ahn ok` → `Ahn ok` (AC 2)
Proof: `cargo test -p fala-postproc --lib rules::tests::en_keeps_pt_only_fillers`

**C3** - 3 ou mais repetições seguidas (sem caixa) viram 1: `eu eu eu acho` → `Eu acho`, `Eu EU eu eu acho` → `Eu acho`; 2 repetições ficam: `o que que é` → `O que que é` (AC 3)
Proof: `cargo test -p fala-postproc --lib rules::tests::collapses_three_or_more_repeats`

**C4** - Com o dicionário `ChargeBee`, `Itaú`, `Fala Cloud Sync`: `a charge bee e o itau.` → `A ChargeBee e o Itaú.`; `charge-bee` → `ChargeBee`; `CHARGEBEE` → `ChargeBee`; `fala cloud sync` → `Fala Cloud Sync`; `falacloud sync` → `Fala Cloud Sync` (AC 4)
Proof: `cargo test -p fala-postproc --lib rules::tests::dictionary_replaces_spelling_variants`

**C5** - Uma sequência que cruza qualquer um dos 6 sinais `,` `.` `;` `:` `?` `!` não é juntada: `charge, bee` → `Charge, bee` e o mesmo para os outros 5 (AC 5)
Proof: `cargo test -p fala-postproc --lib rules::tests::dictionary_does_not_join_across_punctuation`

**C6** - Palavra que só contém o termo como prefixo ou sufixo fica igual: `chargebeex` → `Chargebeex`, `xchargebee` → `Xchargebee` (AC 6)
Proof: `cargo test -p fala-postproc --lib rules::tests::dictionary_ignores_partial_words`

**C7** - `  eu   acho ,  que sim .  ` → `Eu acho, que sim.`; espaço antes de cada um dos 6 sinais `,` `.` `;` `:` `?` `!` some; primeira letra maiúscula, inclusive acentuada (`é isso` → `É isso`) (AC 7)
Proof: `cargo test -p fala-postproc --lib rules::tests::normalizes_spacing_and_capitalizes`

**C8** - Entrada `""` e `"   "` → `final_text` `""`, `Editor::Rules`, 0 conexões no servidor falso mesmo com LLM ligado e chave presente (AC 8)
Proof: `cargo test -p fala-postproc --test fake_gemini empty_input_returns_empty_without_request`

### S2 - Gemini, payload e fallback · 4 files · 50 KB · ~13k

Prova: `cargo test -p fala-postproc --test fake_gemini <nome>`. O servidor falso é um
`std::net::TcpListener` em `127.0.0.1:0` que guarda request line, headers e corpo de cada
conexão e responde o que o teste mandar.

**C9** - Com LLM ligado, chave e app `Slack` fora da lista, uma saída de regras com 16 palavras gera exatamente 1 request e devolve a resposta `"  Texto formatado.\n"` como `Texto formatado.` com `Editor::Llm`; com LLM desligado ou sem chave, as mesmas 16 palavras geram 0 requests e `Editor::Rules` (AC 9)
Proof: `cargo test -p fala-postproc --test fake_gemini long_text_uses_llm_response`
Proof: `cargo test -p fala-postproc --test fake_gemini llm_off_or_no_key_skips_request`

**C10** - Saída de regras com exatamente 15 palavras → 0 requests, `final_text` = saída das regras, `Editor::Rules` (AC 10)
Proof: `cargo test -p fala-postproc --test fake_gemini fifteen_words_skip_llm`

**C11** - Lista desligada `["Slack"]` e app `slack` → 0 requests, `Editor::Rules`; app `Slackware` com a mesma lista → 1 request (AC 11)
Proof: `cargo test -p fala-postproc --test fake_gemini disabled_app_skips_llm`

**C12** - O corpo capturado tem só as chaves `systemInstruction`, `contents`, `generationConfig`; `systemInstruction.parts[0].text` == `SYSTEM_PROMPT` + `"\n\nDicionário pessoal:\n- ChargeBee\n- Itaú"` (sem o bloco quando o dicionário é vazio); `contents` tem 1 item, `role` `user`, 1 parte, texto == `"<app>Slack</app>\n<ditado>{saída das regras}</ditado>"` (`"<ditado>…</ditado>"` sem app); `generationConfig` == `{"temperature":0}`; o filler `hã` do bruto não aparece no corpo (AC 12)
Proof: `cargo test -p fala-postproc --test fake_gemini payload_has_only_text_app_and_dictionary`

**C13** - A request line é `POST /v1beta/models/gemini-2.5-flash-lite:generateContent HTTP/1.1`, sem `?`; o header `x-goog-api-key` traz a chave de teste; a chave não aparece na request line nem no corpo; `--model`/construtor com `gemini-3.5-flash-lite` muda só o segmento do modelo (AC 13, door 4)
Proof: `cargo test -p fala-postproc --test fake_gemini key_only_in_header`

**C14** - Servidor que responde em 3 s → `Editor::Rules`, fallback `timeout`, tempo de parede entre 1,9 s e 2,5 s; servidor que responde em 1,5 s → resposta usada, `Editor::Llm` (AC 14)
Proof: `cargo test -p fala-postproc --test fake_gemini slow_response_falls_back_at_two_seconds`
Proof: `cargo test -p fala-postproc --test fake_gemini response_under_two_seconds_is_used`

**C15** - Cada um dos 6 casos cai no texto das regras com `Editor::Rules` e o fallback dito: status `500` → `http 500`; `429` → `http 429`; conexão recusada → `rede`; `{"candidates":[]}` → `resposta inválida`; texto vazio em `parts` → `resposta inválida`; corpo que não é JSON → `resposta inválida` (AC 15)
Proof: `cargo test -p fala-postproc --test fake_gemini failures_fall_back_to_rules`

**C16** - Com um servidor `400` cujo corpo ecoa o ditado e a chave, nem o `Display` nem o `Debug` do fallback e do `PostprocError` contêm o texto ditado nem a chave (AC 16)
Proof: `cargo test -p fala-postproc --test fake_gemini errors_do_not_echo_text_or_key`

**C33** - Servidor que responde `Texto tardio.` em 3 s, com prazo tardio padrão: o `Postprocessor` volta entre 1,9 s e 2,5 s com `Editor::Rules`, fallback `timeout` e `late_edit` presente; `late_edit.wait()` devolve `Ok("Texto tardio.")`; o servidor viu exatamente 1 conexão (AC 29, door 6)
Proof: `cargo test -p fala-postproc --test fake_gemini late_response_is_delivered_as_late_edit`

**C34** - Com prazo tardio de 3 s (`with_late_deadline`): servidor que responde em 5 s → `wait()` devolve `Err(Fallback::Timeout)`; servidor que responde `500` em 2,5 s → `Err(Fallback::Http(500))`; servidor que fecha a conexão em 2,5 s sem responder → `Err(Fallback::Network)`; corpo `{"candidates":[]}` em 2,5 s → `Err(Fallback::InvalidResponse)` (AC 30)
Proof: `cargo test -p fala-postproc --test fake_gemini late_failures_yield_fallback`

**C35** - `late_edit` é `None` nos 4 casos: resposta em 1,5 s (`Editor::Llm`), 15 palavras (LLM pulado), app desligado, servidor `500` imediato (AC 31)
Proof: `cargo test -p fala-postproc --test fake_gemini no_late_edit_unless_timeout`

### S3 - chaves no keyring · 3 files · 15 KB · ~5k

Prova: `cargo test -p fala-secrets <nome>`; as `#[ignore]` com `-- --ignored`.

**C17** - `set` e depois `get` do mesmo provedor devolve a mesma chave; um segundo `set` sobrescreve (`get` devolve a última), no `MemoryStore` e no `KeyringStore` contra o Secret Service desta máquina (AC 17)
Proof: `cargo test -p fala-secrets --lib tests::memory_store_round_trips`
Proof: `cargo test -p fala-secrets --lib tests::keyring_store_round_trips -- --ignored`

**C18** - Depois de `delete`, `get` devolve `None`; `delete` de um provedor sem chave devolve `Ok(())` (AC 18)
Proof: `cargo test -p fala-secrets --lib tests::delete_then_get_is_none`
Proof: `cargo test -p fala-secrets --lib tests::keyring_store_round_trips -- --ignored`

**C19** - Os ids `""`, `Gemini`, `a-b`, `a b`, `ç`, 33 × `a` dão `InvalidProvider` em `get`, `set` e `delete`, e o `MemoryStore` continua vazio; `gemini`, `bedrock_mantle`, 32 × `a` são aceitos (AC 19)
Proof: `cargo test -p fala-secrets --lib tests::rejects_invalid_provider_ids`

**C20** - `ApiKey::new("")` e `ApiKey::new("  \t")` dão `EmptyKey`; logo nenhum `set` recebe chave vazia (AC 20)
Proof: `cargo test -p fala-secrets --lib tests::api_key_rejects_empty`

**C21** - Os erros do `keyring` `NoStorageAccess`, `PlatformFailure` e `NoDefaultStore` viram `SecretError::Unavailable`; `NoEntry` vira `Ok(None)` em `get` e `Ok(())` em `delete` (AC 21)
Proof: `cargo test -p fala-secrets --lib tests::maps_keyring_errors`

**C22** - `format!("{:?}", ApiKey::new("sk-teste")?)` == `ApiKey([REDACTED])`; `ApiKey` não implementa `Display` nem `Serialize` (asserção de compilação com `static_assertions::assert_not_impl_any!`); a entrada do `KeyringStore` usa service `br.com.augusto.fala` e account = id do provedor (AC 22, door 3)
Proof: `cargo test -p fala-secrets --lib tests::api_key_debug_is_redacted`
Proof: `cargo test -p fala-secrets --lib tests::keyring_entry_uses_fala_service -- --ignored`

### S4 - fala-cli key e format · 4 files · 25 KB · ~7k

Prova: `cargo test -p fala-cli --bin fala-cli <módulo>::tests::<nome>`; os testes injetam o
store e os fluxos de stdin/stdout/stderr.

**C23** - `key set gemini` com stdin `k-sentinela\n` → exit 0, `get` devolve `k-sentinela`, e nem stdout nem stderr contêm `k-sentinela` (AC 23)
Proof: `cargo test -p fala-cli --bin fala-cli key::tests::set_stores_and_never_echoes`

**C24** - `key set gemini` com stdin `""` e com `"\n"` → exit 2 e store vazio (AC 24)
Proof: `cargo test -p fala-cli --bin fala-cli key::tests::set_empty_stdin_exits_2`

**C25** - `key status gemini` imprime `definida\n` com chave e `ausente\n` sem, exit 0 nos dois; `key delete gemini` → exit 0 e depois `ausente`, também quando já estava ausente; `key set Gemini` → exit 2 (AC 25, AC 18, AC 19)
Proof: `cargo test -p fala-cli --bin fala-cli key::tests::status_delete_and_invalid_provider`

**C26** - `format --dictionary <arq com ChargeBee>` com stdin `hã eu eu eu acho que a charge bee` → stdout `Eu acho que a ChargeBee\n`, stderr com `editor: regras`, exit 0; `format --llm` com chave e servidor recusando conexão, 16 palavras → exit 0, stdout = texto das regras, stderr com `fallback: rede` (AC 26)
Proof: `cargo test -p fala-cli --bin fala-cli format::tests::prints_final_text_and_editor`
Proof: `cargo test -p fala-cli --bin fala-cli format::tests::fallback_still_exits_0`

**C27** - `format --llm` sem chave `gemini` → exit 2, stderr contém `fala-cli key set gemini`, 0 conexões no servidor falso (AC 27)
Proof: `cargo test -p fala-cli --bin fala-cli format::tests::llm_without_key_exits_2`

**C28** - Com um store que devolve `Unavailable`: `key set`, `key status`, `key delete` e `format --llm` saem com 1 e o stderr contém `keyring indisponível` (AC 28)
Proof: `cargo test -p fala-cli --bin fala-cli key::tests::unavailable_keyring_exits_1`
Proof: `cargo test -p fala-cli --bin fala-cli format::tests::unavailable_keyring_exits_1`

### S5 - montagem, fronteiras e dependências · 3 files · 10 KB · ~3k

**C29** - O binário real grava no gnome-keyring: `printf 'nao-e-chave' | fala-cli key set teste_cli` exit 0; `secret-tool lookup service br.com.augusto.fala username teste_cli` imprime `nao-e-chave`; `key status teste_cli` imprime `definida`; depois de `key delete teste_cli`, `ausente` (door 1, door 2, door 3)
Proof: `bash .specs/features/postproc/smoke-keyring.sh`

**C30** - `ARCHITECTURE.md` tem a linha do Code Map de `crates/secrets` / `fala-secrets` (door 1)
Proof: `grep -nE '^\| .crates/secrets. \| .fala-secrets. \|' ARCHITECTURE.md`

**C31** - Nada em `crates/secrets` nem em `crates/postproc` depende de `tauri` nem usa `#[cfg(windows)]`/`#[cfg(target_os` (ADR-0002, ADR-0007, door 2)
Proof: `scripts/check-no-tauri-in-crates.sh && ! grep -rnE 'cfg\((windows|target_os|unix)' crates/secrets crates/postproc`

**C32** - `cargo deny check` passa com `keyring` e `ureq` na árvore (licenças, fontes, advisories) (door 2, door 4)
Proof: `cargo deny check`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| fillers pt-BR (10) | C1, table-driven sobre os 10 | - |
| fillers com `Language::En` (10) | removidos: `uh` C2 · `uhm` C2 · `hmm` C2 · mantidos: `hã` C2 · `hãã` C2 · `ãh` C2 · `ahn` C2 · `hum` C2 · `humm` C2 · `éé` C2 | - |
| `Language` (2) | `PtBr` C1 · `En` C2 | - |
| repetição (3 bordas) | 2 mantém C3 · 3 colapsa C3 · 4 com caixa mista colapsa C3 | - |
| variantes de grafia do dicionário (5) | espaço C4 · hífen C4 · caixa C4 · acento C4 · n-grama de 3 palavras C4 | - |
| sinais que fecham n-grama e perdem espaço antes (6) | `,` C5, C7 · `.` C5, C7 · `;` C5, C7 · `:` C5, C7 · `?` C5, C7 · `!` C5, C7 | - |
| limiar de palavras (2 bordas) | 15 C10 · 16 C9 | - |
| condições do LLM (4) | LLM ligado C9 · chave presente C9 · app fora da lista C11 · > 15 palavras C10 | - |
| campos do corpo do request (3) | `systemInstruction` C12 · `contents` C12 · `generationConfig` C12 | - |
| `systemInstruction` (2 formas) | com dicionário C12 · dicionário vazio C12 | - |
| parte `user` (2 formas) | com app C12 · sem app C12 | - |
| motivos de fallback (4) | `timeout` C14 · `http <código>` C15 (500, 429) · `rede` C15 · `resposta inválida` C15 (3 casos) | - |
| borda do timeout (2) | 1,5 s usa C14 · 3 s cai C14 | - |
| destino da resposta do Gemini (3) | antes de 2 s vira o final C14 · entre 2 s e o prazo tardio vira `LateEdit` C33 · depois do prazo tardio vira `Err(Timeout)` C34 | - |
| resultado da `LateEdit` (5) | texto C33 · `timeout` C34 · `http <código>` C34 · `rede` C34 · `resposta inválida` C34 | - |
| casos sem `LateEdit` (4) | LLM usado C35 · LLM pulado por palavras C35 · app desligado C35 · fallback não-timeout C35 | - |
| `SecretStore` operações (3) | `get` C17 · `set` C17 · `delete` C18 | - |
| implementações de `SecretStore` (2) | `MemoryStore` C17, C18, C19 · `KeyringStore` C17, C18, C22, C29 | - |
| erros de `SecretError` (3) | `InvalidProvider` C19 · `EmptyKey` C20 · `Unavailable` C21, C28 | - |
| erros do `keyring` mapeados (4) | `NoStorageAccess` C21 · `PlatformFailure` C21 · `NoDefaultStore` C21 · `NoEntry` C21 | - |
| `fala-cli key set` statuses (3) | 0 C23 · 1 C28 · 2 C24, C25 | - |
| `fala-cli key status` statuses (3) | 0 C25 · 1 C28 · 2 C25 (mesmo validador de `set`, provado em `status_delete_and_invalid_provider`) | - |
| `fala-cli key delete` statuses (3) | 0 C25 · 1 C28 · 2 C25 | - |
| `fala-cli format` statuses (3) | 0 C26 · 1 C28 · 2 C27 | - |
| Landing doors (6) | 1 crate `fala-secrets` C30, C29 · 2 `keyring` C29, C31, C32 · 3 nome da entrada C22, C29 · 4 contrato Gemini C12, C13, C15, C32 · 5 trait `Formatter` C1, C9 · 6 prazo duplo C33, C34, C35 | - |
| startup config: store das chaves (2 assemblies) | `main.rs` com `KeyringStore` C29 · testes do CLI com `MemoryStore` C23 | - |

- Claims naming a status, route or request shape: C12, C13, C15 cruzam o HTTP pelo servidor falso; C23-C28 chamam o handler do subcomando; C29 roda o binário
- No other check claims more than the single case its proof exercises

## Test policy

O repo diz onde ficam os testes (`#[cfg(test)] mod tests` no módulo, `apps/cli/tests/` para o
binário) e não diz que nível prova o quê. Linhas por forma do código:

| Code | Required proofs | Coverage expectation |
| --- | --- | --- |
| Decide, alcançado por uma fronteira (HTTP) | uma na fronteira (servidor falso) **e** uma na própria camada quando a decisão não depende da rede | uma asserção por linha da tabela de decisão |
| Decide, sem fronteira (regras, validação de id, mapeamento de erro) | uma na própria camada | uma asserção por membro do conjunto |
| Ponto de entrada que decide pouco (subcomandos do CLI) | uma no handler, com store e fluxos injetados | entrada aceita, cada rejeição, cada caminho de erro |
| Instrumentação (`KeyringStore` repassando ao `keyring`) | nenhuma própria além do mapeamento de erro | coberta por C17b e C29 contra o cofre real |

Evidence:

- `crates/postproc/src/rules.rs` (novo): fillers por idioma (2 listas), repetição (1 borda), n-grama 1-3 com 6 fechamentos, normalização → decide, ~12 pontos
- `crates/postproc/src/lib.rs` `Postprocessor` (novo): 4 condições do LLM + 4 motivos de fallback → decide, alcançado pelo HTTP
- `crates/postproc/src/gemini.rs` (novo): monta o corpo com 2 × 2 formas, lê a resposta com 3 formas inválidas → decide, alcançado pelo HTTP
- `crates/secrets/src/lib.rs` (novo): validação de id e mapeamento de 4 erros → decide; o resto repassa ao `keyring`
- analogia no repo: `apps/desktop/src/llm_client.rs` testa erro e URL com `serve_one_response` (servidor TCP local), mesma forma do servidor falso aqui

Cost: 32 checks, ~40 testes em 6 arquivos. Sem estas linhas, a tabela de fallback seria provada
por um único caminho de timeout. Linhas não vão para as diretrizes do repo (sem pergunta: o
Augusto delegou; ficam só nesta feature).

## Swept

- validation: C19, C20, C24, C25
- failure modes: C14, C15, C26, C33, C34
- idempotency: C17 (segundo `set` sobrescreve), C18 (`delete` de ausente é `Ok`)
- authorization: n/a - o keyring do SO autoriza pela sessão do usuário; o CLI é local e não tem outro chamador
- concurrency: C33, C34 - a request tardia roda numa thread e entrega por canal; fora isso B não guarda estado mutável compartilhado (o `Postprocessor` é imutável depois de construído; `Formatter: Send + Sync` é imposto pelo compilador) e a ordem dos ditados é do pipeline (trilha A)
- data lifecycle: C18; a migração das chaves existentes é a trilha K
- dependency failure: C14, C15, C21, C28
- state transitions: n/a - não há máquina de estado; o `Editor` é escolhido por chamada (C8-C11)
- observability: C16 (erro sem texto nem chave), C26 (`editor:` e `fallback:` no stderr); log do texto só em `debug`

## Handoff

- S1-S5 ≈ 10k + 13k + 5k + 7k + 3k = 38k de arquivos tocados (`wc -c` estimado: regras e servidor falso ~90 KB, segredos ~15 KB, CLI ~25 KB, `main.rs` 2,5 KB, `Cargo.toml` 3 KB) mais ~30k de leitura (`text.rs` 30 KB, docs do `ureq`/`keyring`) = ~70k, abaixo de 150k - one builder
- Mechanism: one builder
- PR: se o diff passar de ~1.000 linhas, dois PRs empilhados no fim (`feat(secrets)` com S3 + `key`, depois `feat(postproc)` com S1, S2, `format`); decidido pelo painel, delegado pelo Augusto em 2026-10-02
