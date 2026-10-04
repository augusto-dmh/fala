# desktop-keys verification

**Verdict**: PASS
**Profile**: standard
**Diff range**: ec1a526..0c5e894
**Round**: 2 - scoped
**Verifier**: independent sub-agent (author != verifier)

Rodada 2 sobre 0c5e894. A rodada 1 (ab41a00) reprovou por F4 e F5 sobreviventes e cinco membros
sem prova. A correção é só de teste: `git diff ab41a00 0c5e894` toca `apps/desktop/src/settings.rs`
em dois hunks (`@@ -1941,0 +1942,21` e `@@ -2081,0 +2103,59`), ambos dentro de `mod tests`
(`settings.rs:1376`), e `checks.md` (emenda aditiva C10-C13, Coverage, Swept). O diff de produção
é vazio, então as linhas de produção citadas na rodada 1 continuam valendo; as dos testes antigos
andaram +21. As provas rodaram de novo por inteiro; F4 e F5 morrem agora, e as três superfícies
novas (releitura com erro, chave só com espaços, `store_value`) também foram levadas a falhar.

## Binding sources

Carried from ab41a00 (o diff de produção é vazio), exceto a última linha, verified at 0c5e894.

| Source | Opened | Contradiction | Uncovered |
| --- | --- | --- | --- |
| `docs/decisions/0008-distribuicao-byok-sem-backend-assinatura-e-updater-adiados.md` | yes (rodada 1) | none - a "Confirmação" (`Settings` sem campo de chave) segue pendente pelo Out of scope do plano; o JSON fica com `""` e a chave em memória | - |
| `docs/decisions/0002-tauri-2-com-workspace-sem-tauri-no-core.md` | yes (rodada 1); script re-rodado em 0c5e894 | none - `scripts/check-no-tauri-in-crates.sh` ok | - |
| `docs/decisions/0007-windows-primeiro-linux-gnome-depois-sem-macos.md` | yes (rodada 1) | none - nenhum `#[cfg(...)]` novo; o diff da rodada 2 também não adiciona | - |
| `AGENTS.md` | yes | none | - |
| `.specs/features/postproc/plan.md` door 1 e door 3 | yes (rodada 1) | none - `KeyringStore` em `apps/desktop/src/settings.rs:368`; service `br.com.augusto.fala` (`crates/secrets/src/lib.rs:12`) | - |
| decisão do orquestrador: "migração idempotente que nunca apaga do JSON antes de confirmar a gravação no keyring" | yes - verified at 0c5e894 | none - a releitura com `Err` (`apps/desktop/src/settings.rs:457`), descoberta na rodada 1, agora tem C12 em `:2151`, e F6 morreu ali | - |

## Checks

Verified at 0c5e894. Proof run (uma invocação na árvore real):
`CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2 cargo test -p fala --lib settings::tests`
exit 0, 34 passed, 0 failed; os 11 testes `settings::tests::key_vault::*` aparecem um a um como `ok`.
Rodado de novo depois da última falha injetada (com `touch apps/desktop/src/settings.rs` na árvore
real): 34 passed, 11 `key_vault` ok.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | chave do JSON vai ao store, JSON fica `""`, memória mantém | `key_vault::migrates_plaintext_key_to_store ... ok` | `apps/desktop/src/settings.rs:1986` - `assert_eq!(stored(&*store, "openai").as_deref(), Some("sk-teste"))`; `:1987` - `disk.get("openai")` = `Some("")`; `:1988` groq `Some("")`; `:1989` - `memory.get("openai")` = `Some("sk-teste")` | PASS |
| C2 | `set` falha ou releitura diferente: JSON mantém o texto; o `warn` nomeia o provedor e não a chave | `key_vault::failed_store_keeps_plaintext ... ok` | `apps/desktop/src/settings.rs:1996` (UnavailableStore) e `:2000` (LyingStore) - `disk.get("openai")` = `Some("sk-teste")`; `:2003` - `assert!(warning.contains("openai"))`; `:2004` - `assert!(!warning.contains("sk-teste"))` | PASS |
| C3 | id inválido fica no JSON e o store não é chamado | `key_vault::invalid_provider_stays_in_json ... ok` | `apps/desktop/src/settings.rs:2014` - `disk.get("Bad-Id")` = `Some("valor")`; `:2015` - `assert_eq!(store.calls_for("Bad-Id"), 0)` | PASS |
| C4 | duas cargas: mesmas chaves, 1 `set` no total | `key_vault::second_load_does_not_rewrite ... ok` | `apps/desktop/src/settings.rs:2026` - `memory.get("openai")` = `Some("sk-teste")` nas duas voltas; `:2030` - `assert_eq!(store.sets.load(Ordering::SeqCst), 1)` | PASS |
| C5 | `hydrate` lê do store; JSON não vazio vence | `key_vault::hydrate_reads_store_and_json_wins ... ok` | `apps/desktop/src/settings.rs:2043` - `Some("sk-teste")`; `:2053` - `Some("nova")` | PASS |
| C6 | trocar grava no store; esvaziar apaga | `key_vault::write_updates_and_deletes_store ... ok` | `apps/desktop/src/settings.rs:2066` - `stored(..., "openai")` = `Some("outra")`; `:2071` - `stored(..., "openai")` = `None`; `:2067`, `:2072` JSON `Some("")` | PASS |
| C7 | `store_value` sem chave confirmada; demais campos iguais | `key_vault::store_value_has_no_confirmed_key ... ok` | `apps/desktop/src/settings.rs:2088` - `assert!(!text.contains("sk-proj-confirmada-1"))`; `:2089` - `assert!(!text.contains("gsk-confirmada-2"))`; `:2101` - `assert_eq!(plain, stripped)` | PASS |
| C8 | toda escrita de `settings` passa por `to_store_value` | `! grep -n 'store.set("settings", serde_json::to_value(' apps/desktop/src/settings.rs` exit 0 (nenhuma linha) | `grep -n 'store.set("settings"'` acha os mesmos 4 sites, todos `to_store_value`: `apps/desktop/src/settings.rs:1177`, `:1184`, `:1190`, `:1348`. A varredura de caminhos alternativos da rodada 1 é carried from ab41a00 (diff de produção vazio) | PASS |
| C9 | compila com `fala-secrets`; `crates/` sem `tauri` | `scripts/check-no-tauri-in-crates.sh` exit 0 (`ok: no tauri in crates/`); `cargo check -p fala` exit 0 | `apps/desktop/Cargo.toml:51` - `fala-secrets = { workspace = true }`. Mesma decisão de custo da rodada 1: `cargo check -p fala` no lugar de `--workspace`; o diff ab41a00..0c5e894 só toca `mod tests` de `apps/desktop` e `.specs/`, então nenhum outro membro do workspace mudou | PASS |
| C10 | keyring com leitura falhando: `hydrate` deixa vazio sem pânico e o `to_disk` seguinte não apaga; vault novo que nunca leu não apaga a chave do store | `key_vault::unreadable_keyring_never_deletes ... ok` | `apps/desktop/src/settings.rs:2110` - `memory.get("openai")` = `Some("")` depois de `hydrate` com `UnreadableStore` (`get` = `Err(Unavailable)`, `:1950-1952`); `:2113` - `assert_eq!(store.deletes.load(Ordering::SeqCst), 0)`; `:2122-2125` - `stored(&counting.inner, "openai")` = `Some("sk-teste")` depois de `fresh.to_disk(openai: "")` | PASS |
| C11 | três `hydrate` leem o store 1 vez por provedor e mostram `sk-teste` | `key_vault::hydrate_reads_each_provider_once ... ok` | `apps/desktop/src/settings.rs:2139` - `Some("sk-teste")` nas três voltas; `:2141` - `assert_eq!(store.calls_for("openai"), 1)`; `:2142` - `assert_eq!(store.calls_for("groq"), 1)` (`calls_for` conta `get`+`set`+`delete`; o `hydrate` só faz `get`) | PASS |
| C12 | `set` aceito e releitura com erro: o JSON mantém `sk-teste` | `key_vault::failed_read_back_keeps_plaintext ... ok` | `apps/desktop/src/settings.rs:2150` - `assert_eq!(store.sets.load(Ordering::SeqCst), 1)` (o `set` aconteceu); `:2151` - `disk.get("openai")` = `Some("sk-teste")` | PASS |
| C13 | chave só com espaços fica no JSON e o store recebe 0 `set` | `key_vault::blank_key_stays_in_json ... ok` | `apps/desktop/src/settings.rs:2159` - `disk.get("openai")` = `Some("   ")`; `:2160` - `assert_eq!(store.sets.load(Ordering::SeqCst), 0)` | PASS |

## Coverage

Recomputado a partir do código (`apps/desktop/src/settings.rs:380-476`, idêntico ao da rodada 1) e
do plano, não da tabela do `checks.md`. As linhas com membros sem prova na rodada 1 e as que a
emenda acrescentou estão verified at 0c5e894; as demais, carried from ab41a00.

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| passos da migração (3) - carried from ab41a00 | Landing door 1; `settings.rs:439-457` | `set` C1 (`:1986`) · relê igual C2 (`:2000`) · `""` no JSON C1 (`:1987`) | - |
| falhas da migração / falhas do cofre (5) - verified at 0c5e894 | braços de erro de `to_disk`/`store_confirmed` (`settings.rs:414`, `:452`, `:453`, `:456`, `:457`) e AC 2/3; inclui as linhas "falhas do cofre (4)" e "chaves que o `ApiKey` recusa (1)" da emenda | `set` falha C2 (`:1996`) · releitura diferente C2 (`:2000`) · releitura com `Err` C12 (`:2151`, F6 morta) · id inválido C3 (`:2014-2015`) · só espaços (`EmptyKey`, único caso que `ApiKey::new` recusa, `crates/secrets/src/lib.rs:22`) C13 (`:2159-2160`, F7 morta) | - |
| origem da chave em memória (4) - verified at 0c5e894 | `hydrate`, `settings.rs:384-402`; Assumption "cofre indisponível na leitura" (`plan.md:101`) | só JSON C1 (`:1989`) · só store C5 (`:2043`) · os dois, JSON vence C5 (`:2053`) · leitura com `Err` -> `""` em memória C10 (`:2110`) | - |
| escrita de chave / guarda de apagar (3) - verified at 0c5e894 | `to_disk`, `settings.rs:420-445`; Assumption "nada é apagado" | valor novo C6 (`:2066`) · vazio com chave conhecida apaga C6 (`:2071`) · vazio sem chave conhecida mantém C10 (`:2113` depois de leitura com erro, `:2122-2125` em vault novo; F4 morta) | - |
| cache da door 2 (2) - verified at 0c5e894 | `settings.rs:388-391` (leitura) e `:435` (escrita); plan Landing door 2 | não regrava C4 (`:2030`, F3 da rodada 1) · não relê C11 (`:2141-2142`, F5 morta) | - |
| Landing doors (2) - verified at 0c5e894 | plan Landing | 1 migração C1, C2, C4, C8, C12 · 2 cache em processo C4, C11 | - |
| startup config: store das chaves (2 assemblies) - carried from ab41a00 | `settings.rs:368` e os testes | app com `KeyringStore` em `apps/desktop/src/settings.rs:368`, compila (C9, re-rodado); execução real é `TODO(windows)` (open question 1) · testes com store injetado C1-C7, C10-C13 | - |
| escritas do JSON de settings (4) - verified at 0c5e894 (grep) | grep em `apps/desktop/src` | `settings.rs:1177`, `:1184`, `:1190`, `:1348` -> C8 | - |

## Test policy rows

Linha 1 re-julgada at 0c5e894; linha 2 carried from ab41a00 (o grep foi re-rodado e dá o mesmo).

| Row | Files it classifies | Required proof | Expectation met |
| --- | --- | --- | --- |
| Decide, sem fronteira (`KeyVault::hydrate`, `to_disk`) | `apps/desktop/src/settings.rs:380-458` | uma prova na própria camada com store injetado: C1-C7, C10-C13 | yes - tabela de origem: 4 linhas, 4 asserções (`:1989`, `:2043`, `:2053`, `:2110`); tabela de falha: 5 linhas, 5 asserções (`:1996`, `:2000`, `:2151`, `:2014`, `:2159`) |
| Ponto de entrada que decide pouco (`get_settings`, `write_settings`) | `apps/desktop/src/settings.rs:1136-1194`, `:1343-1349` | a dele é o grep de C8 | yes - os 4 `store.set("settings", ...)` passam por `to_store_value` (`:1177`, `:1184`, `:1190`, `:1348`) |

Swept relido at 0c5e894: a linha "dependency failure" agora diz C2, C10, C12, e as três estão lá
(C10 cobre a leitura com erro, C12 a releitura com erro, C2 o `set` com erro). As demais linhas
`existing` não mudaram (carried from ab41a00).

## Faults injected

Verified at 0c5e894. Árvore real: `git status --porcelain` igual antes e depois (só
`?? .specs/features/desktop-keys/verification.md`, este relatório). Rascunho em
`scratchpad/verify-k2` (`git worktree add --detach ... HEAD`), mesmo `CARGO_TARGET_DIR`; cada falha
foi aplicada, testada com `cargo test -p fala --lib settings::tests::key_vault` e desfeita com
`git checkout`. Depois da última, `touch apps/desktop/src/settings.rs` na árvore real e as provas
rodaram lá de novo (34 passed). Rascunho removido com `git worktree remove --force` + `prune`.
Nenhum keyring real tocado: todos os testes usam stores injetados.

| Mutation | Location | Killed |
| --- | --- | --- |
| F4 (re-injetada): apaga do keyring com valor vazio mesmo sem o cache conhecer a chave (`if !previous.is_empty()` -> `if true`) | `apps/desktop/src/settings.rs:423` | yes - C10 `:2113` (`left: 1`, `right: 0`); 10/11 passaram |
| F5 (re-injetada): `hydrate` ignora o cache e vai ao cofre sempre (remove o `if let Some(cached)`) | `apps/desktop/src/settings.rs:388-391` | yes - C11 `:2141` (`left: 3`, `right: 1`); 10/11 passaram |
| F6 (nova superfície): releitura com `Err` aceita como confirmação (`Err(error) => Err(error.to_string())` -> `Err(_) => Ok(())`) | `apps/desktop/src/settings.rs:457` | yes - C12 `:2151` (`left: Some("")`, `right: Some("sk-teste")`); 10/11 passaram |
| F7 (nova superfície): chave só com espaços tratada como apagada (`if value.is_empty() {` -> `if value.trim().is_empty() { value.clear();`) | `apps/desktop/src/settings.rs:421` | yes - C13 `:2159` (`left: Some("")`, `right: Some("   ")`); 10/11 passaram |
| F8 (C7): `store_value` pula o vault e grava as chaves como estão (`on_disk.post_process_api_keys = vault.to_disk(...)` -> `let _ = vault;`) | `apps/desktop/src/settings.rs:474` | yes - C7 `:2088` (`!text.contains("sk-proj-confirmada-1")`); 10/11 passaram |

Cada falha foi morta por exatamente um teste, o do check que a cobre. Não injetada (teto de 5):
uma falha que leve C6 a falhar; C6 continua com a morte legível pelas asserções `:2066`/`:2071`
sobre o conteúdo do store. F1-F3 da rodada 1 ficam carried from ab41a00 (o código que elas mutam
não mudou e os testes que as mataram, C2/C5/C4, continuam lá com as mesmas asserções).

## Gate

Verified at 0c5e894: `cargo test -p fala --lib settings::tests` - 34 passed, 0 failed (11
`key_vault`, C1-C7 e C10-C13); C8 grep - 0 linhas; `scripts/check-no-tauri-in-crates.sh` - ok;
`cargo check -p fala` - ok. `cargo check --workspace` não foi rodado pelo Verifier (custo de RAM; o
diff da rodada 2 só toca `mod tests` de `apps/desktop`).

## Ranked gaps

Nenhum que pese no veredito.

Observações (não pesam no veredito):

- C10 em `:2110` prova que a leitura com erro deixa `""` e não entra em pânico, mas o `warn!` dessa
  leitura (`settings.rs:396`), que a Assumption de `plan.md:101` também promete, não é observado
  por nenhum teste; é o mesmo padrão dos outros `warn!` da feature (só o texto de
  `key_kept_warning` é testado, em C2).
- Carried from ab41a00: `serde_json::to_value(&on_disk).unwrap()` em `settings.rs:475` (o `AGENTS.md`
  pede `Result` fora de testes); uma leitura que falha fica em cache como `""` até reiniciar o
  processo (`settings.rs:401`, agora fixado por C10) e uma migração que falha tenta o cofre a cada
  `get_settings`; o `Mutex` do cache fica preso durante as chamadas ao cofre (`settings.rs:381`,
  `:410`). Vale olhar no smoke manual com o Secret Service bloqueado.
- A execução real com `KeyringStore` segue `TODO(windows)` (open question 1 do `checks.md`, que
  bloqueia o go-live, não este gate).
