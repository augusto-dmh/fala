# history-home — checks

Target: `$env:CARGO_TARGET_DIR='C:\f\hist'`. Cada check nomeia a prova; o Verifier roda a prova e
registra a saída em `verification.md`.

| ID | AC | Check | Proof |
| --- | --- | --- | --- |
| C1 | 1 | a falha da cola chega ao `NewEntry` e à coluna | `cargo test -p fala --lib managers::history_dictations::tests::save_records_paste_failure` e leitura de `actions.rs` (diff: `paste_failed` atribuído só no `Err` de `utils::paste`) |
| C2 | 2 | classificação "descartada" cobre cola falha, final vazio com bruto, entregue, bruto vazio, sem item | `cargo test -p fala --lib managers::history_dictations::tests::discarded_classification` |
| C3 | 3 | recuperar zera a falha, mostra o bruto do item esvaziado e recusa o resto sem mudar nada | `cargo test -p fala --lib managers::history_dictations::tests::recover_` (2 testes) |
| C4 | 4 | busca acha sem acento, mais recentes primeiro, com item; pula item sem linha; erro sem store | `cargo test -p fala --lib managers::history_dictations::tests::search_` (2 testes) |
| C5 | 5 | migração 6 | `cargo test -p fala --lib managers::history::tests::migration_six_adds_paste_failed` |
| C6 | 6 | seção inicial e ordem da barra | `bun src/components/settings/history/historyModel.test.ts` (H1: `App.tsx` usa `"history"`, primeira chave de `SECTIONS_CONFIG` é `history`) |
| C7 | 7 | grupos por dia e rótulos | `historyModel.test.ts` H2 (`groupByDay` com hoje, ontem, outro dia, virada local da meia-noite) e H3 (`line-clamp-2` na tela) |
| C8 | 8 | debounce e volta à lista | `historyModel.test.ts` H4 (`SEARCH_DEBOUNCE_MS === 200`, `searchQuery("  ")` é `null`) + prova manual M2 |
| C9 | 9 | linha descartada e Recuperar | `historyModel.test.ts` H5 (`rowKind` e `recoverEntry` com ok, erro, cópia falha) + prova manual M1 |
| C10 | 10 | submenu do tray e marca | `cargo test -p fala --lib tray::tests::microphone_` (escolha, ids, rebuild) + prova manual M3 |
| C11 | 11 | clique troca o dispositivo | `tray::tests::microphone_item_ids_round_trip` + prova manual M3 (`fala.log` mostra o dispositivo novo) |
| C12 | 12 | traduções | `bun run check:translations`; `bun run lint`; `historyModel.test.ts` H6 (chaves novas em pt e en) |
| C13 | — | portões | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p fala --lib`, `bun run format:check`, `scripts/check-brand.sh` |

## Prova manual (Windows, build de debug portable)

Registrada em `verification.md`, sem texto de ditado:

- M0: abrir o app → a primeira tela é o histórico.
- M1: um ditado de "hum" (regras esvaziam) aparece cinza com "Recuperar"; clicar copia e normaliza a linha. Se não for possível provocar, dizer.
- M2: buscar uma palavra de um ditado feito na sessão acha a linha; apagar o campo volta à lista.
- M3: tray → Microfone → outro dispositivo: ✓ muda e o log registra o dispositivo; voltar para Automático.

Checks que não rodarem ficam como Unproven e o veredito fica FAIL até rodarem (lição L-004).
