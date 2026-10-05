# desktop-keys - checks

Profile: standard
Plan: `.specs/features/desktop-keys/plan.md`

Perfil `standard` (o projeto declara `light`): a migração é porta de mão única sobre dado do
usuário. Decidido pelo painel, delegado pelo Augusto em 2026-10-02.

13 checks in 3 slices (C10-C13 por emenda aditiva depois da rodada 1 do Verifier) · 2 one-way doors · 1 open (blocks go-live, `TODO(windows)`)

Todo `cargo` com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`.
`cargo test -p fala` compila o desktop inteiro; roda uma vez por rodada de prova.

## Checks

### S1 - migração idempotente · 1 file · 75 KB · ~19k

Prova: `cargo test -p fala --lib settings::tests::<nome>`. `KeyVault` recebe um
`&dyn SecretStore`; os testes usam `MemoryStore` e stores que falham ou mentem.

**C1** - JSON com `openai: "sk-teste"` e `groq: ""`: depois de `to_disk`, o `MemoryStore` tem `openai` = `sk-teste`, o mapa para o JSON tem `openai: ""` e `groq: ""`, e o `AppSettings` em memória continua com `sk-teste` (AC 1)
Proof: `cargo test -p fala --lib settings::tests::migrates_plaintext_key_to_store`

**C2** - Com um store cujo `set` devolve `Unavailable`, e com um store cujo `get` devolve outro valor depois do `set`: o mapa para o JSON mantém `sk-teste` nos dois casos; o texto do `warn` (função que monta a mensagem) contém `openai` e não contém `sk-teste` (AC 2, door 1)
Proof: `cargo test -p fala --lib settings::tests::failed_store_keeps_plaintext`

**C3** - Provedor `Bad-Id` com `valor`: o mapa para o JSON mantém `valor` e o store não recebe nenhuma chamada (store contador com 0 chamadas para esse id) (AC 3)
Proof: `cargo test -p fala --lib settings::tests::invalid_provider_stays_in_json`

**C4** - Duas cargas seguidas (`hydrate` + `to_disk` duas vezes) sobre o mesmo JSON migrado: as duas devolvem `openai` = `sk-teste` e o store contador vê 1 `set` no total (AC 4)
Proof: `cargo test -p fala --lib settings::tests::second_load_does_not_rewrite`

### S2 - as chaves continuam visíveis · 1 file · ~5k

**C5** - JSON com `openai: ""` e store com `openai` = `sk-teste`: `hydrate` deixa `sk-teste` em memória; JSON com `openai: "nova"` e store com `velha` → `nova` em memória (AC 5, door 2)
Proof: `cargo test -p fala --lib settings::tests::hydrate_reads_store_and_json_wins`

**C6** - Trocar a chave em memória para `outra` e persistir → store = `outra`, JSON `""`; trocar para `""` e persistir → store sem a entrada (`get` = `None`), JSON `""` (AC 6)
Proof: `cargo test -p fala --lib settings::tests::write_updates_and_deletes_store`

**C7** - `store_value` (o corpo de `to_store_value`, com o vault injetado) de um `AppSettings` com duas chaves confirmadas produz um `serde_json::Value` cujo texto não contém nenhuma das duas, e mantém os outros campos iguais aos de `serde_json::to_value` (AC 7)
Proof: `cargo test -p fala --lib settings::tests::store_value_has_no_confirmed_key`

**C10** - Com um keyring cuja leitura falha: `hydrate` deixa `openai` vazio em memória sem pânico, e o `to_disk` seguinte não chama `delete` (0 deletes); um vault novo que nunca leu a chave, recebendo `openai: ""`, não apaga a `sk-teste` que está no store (AC 2, plano: "nada é apagado", door 1)
Proof: `cargo test -p fala --lib settings::tests::key_vault::unreadable_keyring_never_deletes`

**C11** - Três `hydrate` seguidos no mesmo vault leem o store 1 vez por provedor (`openai` e `groq`), e as três cargas mostram `sk-teste` (door 2, cache)
Proof: `cargo test -p fala --lib settings::tests::key_vault::hydrate_reads_each_provider_once`

**C12** - `set` aceito e releitura com erro: o JSON mantém `sk-teste` (AC 2, door 1: cofre travado)
Proof: `cargo test -p fala --lib settings::tests::key_vault::failed_read_back_keeps_plaintext`

**C13** - Chave só com espaços (`"   "`) fica no JSON e o store recebe 0 `set` (AC 2)
Proof: `cargo test -p fala --lib settings::tests::key_vault::blank_key_stays_in_json`

### S3 - montagem e fronteiras · ~2k

**C8** - Os pontos de escrita do JSON de settings passam pelo `to_store_value`: nenhum `store.set("settings", serde_json::to_value(` sobra em `settings.rs` (door 1)
Proof: `! grep -n 'store.set("settings", serde_json::to_value(' apps/desktop/src/settings.rs`

**C9** - O workspace compila com o desktop dependendo de `fala-secrets`, e `crates/` continua sem `tauri` (ADR-0002)
Proof: `cargo check --workspace && scripts/check-no-tauri-in-crates.sh`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| passos da migração (3) | `set` C1 · relê igual C1, C2 · `""` no JSON C1 | - |
| falhas da migração (3) | `set` falha C2 · releitura diferente C2 · id inválido C3 | - |
| origem da chave em memória (3) | só JSON C1 · só store C5 · os dois, JSON vence C5 | - |
| escrita de chave (2) | valor novo C6 · valor vazio apaga C6 | - |
| falhas do cofre, recontadas na rodada 1 (4) | `set` falha C2 · releitura diferente C2 · releitura com erro C12 · leitura no `hydrate` com erro C10 | - |
| guarda de apagar, recontada na rodada 1 (2) | valor vazio com chave conhecida apaga C6 · valor vazio sem chave conhecida não apaga C10 | - |
| chaves que o `ApiKey` recusa (1) | só espaços C13 | - |
| cache da door 2 (2) | não regrava C4 · não relê C11 | - |
| Landing doors (2) | 1 migração C1, C2, C4, C8 · 2 leitura com cache C4, C5 | - |
| startup config: store das chaves (2 assemblies) | app com `KeyringStore` C9 (compila; a execução é `TODO(windows)` e o smoke manual no Linux) · testes com `MemoryStore` C1-C7 | - |

- No check claims more than the single case its proof exercises

## Test policy

| Code | Required proofs | Coverage expectation |
| --- | --- | --- |
| Decide, sem fronteira (`KeyVault::hydrate`, `to_disk`) | uma na própria camada, com store injetado | uma asserção por linha da tabela de origem e de falha |
| Ponto de entrada que decide pouco (`get_settings`, `write_settings`) | nenhuma própria (precisam de `AppHandle`); o grep de C8 prova que passam pelo `to_store_value` | todos os `store.set("settings", ...)` |

Evidence:

- `apps/desktop/src/settings.rs`: os testes existentes (`debug_output_redacts_api_keys`, salvage) testam funções puras sem `AppHandle`; mesma forma aqui
- `fala-secrets::MemoryStore` já existe para consumidores (B)

## Swept

- validation: C3
- failure modes: C2
- idempotency: C4
- authorization: n/a - o keyring do SO autoriza pela sessão do usuário
- concurrency: n/a - o cache do `KeyVault` fica atrás de um `Mutex`; `get_settings`/`write_settings` já são chamados de várias threads hoje e a ordem entre eles não muda
- data lifecycle: C1, C6
- dependency failure: C2, C10, C12 (store indisponível na leitura deixa a chave vazia em memória e nunca apaga)
- state transitions: n/a - por chave, só "no JSON" → "no keyring", provado por C1 e C4
- observability: C2 (o `warn` nomeia o provedor e nunca a chave)

## Handoff

- `settings.rs` 75 KB + `Cargo.toml` do desktop 4 KB + `fala-secrets` 10 KB ≈ 90 KB / 4 ≈ 23k, abaixo de 150k - one builder
- Mechanism: one builder
- Branch: `feat/desktop-keys` em cima de `feat/postproc` (K depende de `fala-secrets`); PR empilhado depois do de B
