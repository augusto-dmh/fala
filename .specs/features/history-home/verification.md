# history-home — verification

Máquina: Windows 11 (Alienware 16), 2026-10-09. Target `CARGO_TARGET_DIR=C:\f\hist`.
Verificado pelo próprio executor (sem painel Verifier separado). Árvore verificada: topo da pilha
`feat/history-home` (5 commits sobre `origin/main` `07d2aa3`); a prova manual rodou sobre arquivos
byte a byte iguais a esse topo (conferido com `cmp`).

## Verdict

**FAIL** — 12 de 13 checks provados; C11 tem só a metade do caminho provada (o clique no menu
nativo do tray não foi acionado; ver M3). Fecha quando alguém clicar em Tray → Microfone no Windows.

## Checks

| ID | Result | Evidence |
| --- | --- | --- |
| C1 | PASS | `save_records_paste_failure` ok; `actions.rs`: `paste_failed.store(true)` só no `Err(e)` de `utils::paste`; caminho de erro de transcrição grava `paste_failed: false` |
| C2 | PASS | `discarded_classification` ok (8 casos: cola falha com e sem item, final vazio e só espaços, mostrando bruto, entregue, bruto vazio, sem item) |
| C3 | PASS | `recover_restores_emptied_and_paste_failed_entries`, `recover_refuses_entries_that_are_not_discarded` ok |
| C4 | PASS | `search_finds_entries_without_accents_newest_first` ("REUNIAO" acha "reunião" e "reuniao", mais nova primeiro), `search_skips_dictations_without_rows_and_needs_the_store` ok; M2 |
| C5 | PASS | `migration_six_adds_paste_failed` ok (versão 6, NOT NULL, padrão `0`, linha antiga preservada) |
| C6 | PASS | `historyModel.test.ts` H1 ok; M0 |
| C7 | PASS | H2 (hoje/ontem/data, virada de meia-noite e de mês, `07:05`), H3 ok; M0 |
| C8 | PASS | H4 ok; M2 (aos 120 ms ainda a lista anterior, aos 620 ms o resultado; campo vazio volta às 5 linhas) |
| C9 | PASS | H5 ok; M1 |
| C10 | PASS | `tray::tests::microphone_item_ids_round_trip`, `microphone_auto_label_names_the_default`, `microphone_changes_rebuild_the_menu` ok; M3 (menu reconstruído) |
| C11 | UNPROVEN (parcial) | `apply_selected_microphone` provado por M3 via `set_selected_microphone` (mesma função que o clique do tray chama); o clique no item do menu nativo não foi feito |
| C12 | PASS | `bun run check:translations` ✓; `bun run lint` sem erro; H6 ok |
| C13 | PASS | ver Gates |

## Gates (topo da pilha)

- `cargo fmt --all -- --check`: ok
- `cargo clippy --workspace --all-targets -- -D warnings`: ok, 0 avisos
- `cargo test -p fala --lib`: 358 passed, 0 failed (main: 348; +10 novos: 7 em `managers`, 3 em `tray`)
- `cargo test --workspace`: 710 passed, 0 failed, 48 ignored (53 suítes, incluindo doc-tests)
- `bun src/components/settings/history/historyModel.test.ts`: C27-C30 e H1-H6 ok
- `bunx tsc --noEmit`, `bun run lint`, `bunx prettier --check .`, `bun run check:translations`: ok
- `scripts/check-brand.sh`: ok; `scripts/check-no-tauri-in-crates.sh`: ok
- Cada etapa da pilha passou `cargo test -p fala --lib` sozinha: 353, 355, 358; as duas etapas de front passaram `tsc`, `lint`, `prettier` e o teste do modelo

## Atualização 2026-10-10: main com #58 e a Home do #71

A pilha foi dividida. A parte Rust segue contra `main` (que agora tem #58, "apply a late gemini
answer") em três PRs empilhados; o front virou a "later PR" do #71 (`feat/ui-rail-home`).

- Sem push forçado: cada branch Rust partiu da ref antiga no origin e recebeu `git merge` de
  `origin/main` (ou da branch de baixo), com a mesma resolução da pilha rebaseada (árvore do merge
  igual, conferida com `git diff`). Resolução do #58: `apply_late_edit_with` também devolve a
  entrada com `discarded` classificado; o teste do `llm_auto` ganha `paste_failed: false`; o teste
  `search_skips_dictations_without_rows_and_needs_the_store` passa o 4º argumento `sensitive` de
  `add_dictation`.
- Contagens sobre o main novo (main: 353 em `cargo test -p fala --lib`):
  - `feat/history-home-discarded` (#76): 358 passed, 0 failed
  - `feat/history-home-search` (#77): 360 passed, 0 failed
  - `feat/history-home-tray-mic` (#78): 363 passed, 0 failed; `cargo fmt --all -- --check` ok;
    `cargo clippy --workspace --all-targets -- -D warnings` ok; `cargo test --workspace` 718 passed,
    0 failed, 48 ignored (53 suítes); `scripts/check-brand.sh` e `check-no-tauri-in-crates.sh` ok
- Front (`feat/history-home-feed`, sobre `feat/ui-rail-home`): o #71 tira Sidebar e Footer e abre
  no Início (`HomePage.tsx`), que já mostra `HistorySettings`. A rota própria do commit "open the
  app on the history" saiu; a tela agrupada entra no lugar da lista herdada, dentro do Início. H1
  passou a provar `useState<RailDestination>("home")` e `<HistorySettings />` no `HomePage.tsx`.
  O `feat/ui-rail-home` está sobre `07d2aa3` (sem #58), então o front usa a versão anterior ao
  rebase (chave `redoAiEdit`); a troca para `applyAiEdit` vem quando o #71 for rebaseado no main.
  Os comandos `history_search` e `recover_history_entry` só existem com #76-#78 no main.
- Prova manual (M0-M3) não foi refeita sobre o main novo nem dentro do Início do #71.
- Portões do front em `feat/history-home-feed` (teste do modelo, `shell.test.tsx`, `tsc`, `lint`,
  `prettier`, traduções): ainda não rodados; a execução foi interrompida por falta de memória na
  máquina antes de começar.

## Prova manual (build de debug portable, Windows)

Como foi montada, sem tocar no app instalado (PID dele intacto do começo ao fim):

- O `tauri-plugin-single-instance` usa o `identifier` como chave; um debug com o mesmo identifier
  encaminharia para o app instalado. O binário de prova foi compilado com
  `TAURI_CONFIG={"identifier":"br.com.augusto.fala.histproof"}` (só no build, sem mudar o código),
  com `portable` ao lado do exe (dados em `C:\f\hist\debug\Data`), vite em `:1420` e
  `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333`.
- Store semeado com 5 ditados sintéticos (escritos pelo executor; nenhum conteúdo pessoal) direto no
  `history.db`; o backfill do app copiou para o `fala.sqlite` no start. Um deles com `"hum"` → final
  vazio; um com `paste_failed = 1`. Atalho de ditado vazio e LLM desligado no store de prova.
- A tela foi lida e operada pelo protocolo DevTools (DOM + `Runtime.evaluate` + captura de tela).
  Nenhum mouse ou teclado foi usado.

| ID | Result | O que se viu |
| --- | --- | --- |
| M0 | PASS | ao abrir: seção ativa "Histórico", primeira da barra; campo "Buscar nos ditados"; grupos "Hoje" (3), "Ontem" (1), "6 de outubro de 2026" (1); hora numa coluna (`00:15`, `18:00`...); linhas normais com `line-clamp-2`; as duas descartadas cinza com "Este ditado foi descartado. Recuperar" (captura conferida) |
| M1 | PASS | clique em "Recuperar" na linha de cola falha → a linha volta ao normal com o texto, a área de transferência recebe o texto da linha, e não sobra nenhum "Recuperar"; no `history.db` todas as linhas ficam com `paste_failed = 0`. O ditado esvaziado ("hum") foi recuperado pelo comando `recover_history_entry` (devolveu `showing: "raw"`, `discarded: false`) e a linha passou a mostrar "hum" pelo evento `history-update`. A área de transferência do usuário foi salva antes e restaurada depois |
| M2 | PASS | "relatorio" → 1 linha (a da cola falha); "REUNIAO amanha" → "Reunião de planejamento…"; "cafe" → "Lembrar de comprar café." em "Ontem"; "inexistente" → "Nenhum ditado encontrado."; campo vazio → 5 linhas de novo |
| M3 | PARCIAL | `get_available_microphones` listou 11 entradas (Default + 10). `set_selected_microphone("Microfone (Fifine Microphone)")` gravou a escolha e o log mostra `tray apply: icon=unchanged menu=rebuilt`; de volta para `default`, outro `menu=rebuilt`. No start, 2 `tray apply` (inicial + lista de microfones preenchida pela thread). **Não feito:** abrir o menu do tray e clicar num item: exige mouse/teclado reais na barra de tarefas, que a sessão não tem |

Checklist para fechar C11 (Windows, à mão): botão direito no ícone do Fala → Microfone → escolher
um dispositivo → reabrir o menu e ver o ✓ nele; Geral → Microfone mostra o mesmo; voltar para
"Automático (<padrão>)".

## Findings

1. O cancelamento com Esc continua sem deixar linha (porta 4 do plano); um ditado cancelado não
   pode ser recuperado. Mudar isso é outra feature.
2. A lista de microfones do tray só se refaz no start e quando o ponteiro passa no ícone (Windows);
   no Linux/macOS só no start. Registrado no plano (Out of scope).
3. A etapa 5 (tela) tem 487 linhas alteradas, acima do alvo de 400: é a reescrita de um componente
   (`HistorySettings.tsx`, 293+/160−) mais as asserções de tela do teste.
