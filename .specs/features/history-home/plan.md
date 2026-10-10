# history-home — histórico como tela inicial, com busca, "descartado · Recuperar" e microfone no tray

## Problem

A janela do Fala abre em "Geral" (`App.tsx`, `useState<SidebarSection>("general")`): a primeira
coisa que a pessoa vê é uma tela de configurações, e o histórico é a segunda aba da barra lateral.
O que ela procura ao abrir a janela é quase sempre um ditado: o que foi colado há pouco, o texto que
não entrou no app certo, uma frase de ontem. A lista atual mostra data e hora por extenso, o texto
inteiro e um player em cada item, sem agrupamento por dia e sem busca, embora a busca sem acento
exista em `fala-storage` (`Store::search`) e no `fala-cli history search` (inventário 2026-10-09 § 1).

Dois casos somem hoje sem aviso: o ditado cujo texto final ficou vazio (as regras tiraram tudo, como
um "hum" isolado) e o ditado cuja cola falhou (`utils::paste` devolveu erro). Os dois gravam linha
no `history.db`, mas a primeira aparece em itálico como texto vazio e a segunda é igual a um ditado
entregue. Por fim, trocar de microfone exige abrir a janela, ir a Geral e achar o seletor; o tray já
tem submenus de modelo e idioma, não de microfone (pesquisa 20 § 3 itens 2, 3, 8 e § 5; síntese
2026-10-09 § 5 itens 2 e 4).

A fonte não traz números de frequência de colas que falham nem de ditados esvaziados.

Quando isto entrar, a janela abre no histórico agrupado por dia ("Hoje", "Ontem", data), com a hora
numa coluna, o texto em até duas linhas e o app de destino; um campo no topo busca no `fala.sqlite`;
um ditado esvaziado ou com cola falha aparece cinza como "Este ditado foi descartado." com
"Recuperar", que copia o texto e devolve a linha ao normal; e o menu do tray em repouso tem
"Microfone ›" com "Automático (<padrão>)" e os dispositivos de entrada, o ativo marcado.

## Flow

Reusa `HistoryManager` (linha do `history.db` + item do `fala.sqlite`), `Store::search`/`undo`, o
`AudioRecordingManager::update_selected_device` que o seletor de Geral usa e o tray por snapshot
(`MenuInputs`). Nada de pipeline novo; `actions.rs` só repassa o resultado da cola.

1. `actions.rs` (exists) — o fecho da cola grava num `AtomicBool` se `utils::paste` falhou; `save_history` passa `paste_failed` no `NewEntry` (o caminho de erro de transcrição passa `false`)
2. `HistoryManager::save_entry_with` (exists) — grava `paste_failed` na coluna nova (door 1)
3. `HistoryManager` ao montar cada `HistoryEntry` (página, busca, retry, undo, recuperar) — `discarded = is_discarded(paste_failed, dictation)`: cola falhou, ou o item mostra texto vazio e o bruto tem texto
4. comando `recover_history_entry(id)` (new, door 2) — zera `paste_failed`; se o item mostra o final vazio e o bruto tem texto, `Store::undo` (passa a mostrar o bruto); devolve a entrada; erro se não estava descartada
5. comando `history_search(query)` (new, door 2) — `Store::search(query, 100)` e, para cada item, a linha do `history.db` com aquele `dictation_id`; itens sem linha (CLI) ficam de fora
6. front `HistorySettings` (exists) — vira a home: campo de busca (debounce 200 ms; vazio volta à página normal), grupos por dia (`groupByDay` em `historyModel.ts`), linha compacta com hora, texto em duas linhas, app; clique expande texto inteiro e player; linha descartada cinza com "Recuperar"
7. `App.tsx`/`Sidebar.tsx` (exist) — seção inicial `history`, primeira na barra lateral
8. `tray.rs` (exists) — `MenuInputs` ganha a lista de entradas, o padrão do sistema e a escolha; submenu "Microfone" no menu em repouso com ids `microphone_auto` e `microphone:<nome>` (door 3); a lista vem de um cache em `TrayState`, preenchido numa thread (`refresh_microphones`) no setup e quando o ponteiro entra no ícone ou clica (Windows)
9. `lib.rs` `on_menu_event` (exists) — reconhece os ids e, numa thread, chama `commands::audio::apply_selected_microphone` (extraído de `set_selected_microphone`), emite `settings-changed` com `selected_microphone` e atualiza o tray

## Impact

| Front | What changes |
| --- | --- |
| domain | termo novo "descartado": linha cuja cola falhou ou cujo item mostra texto vazio com bruto não vazio. Linha sem texto do ASR continua "a transcrição falhou" com re-transcrever |
| behaviour | cancelar com Esc continua sem gravar nada (door 4) |
| stored data | `history.db` migração 6: `paste_failed BOOLEAN NOT NULL DEFAULT 0`; linhas antigas nascem `0` |
| ui | home = histórico; chaves novas em `settings.history.*` e `tray.*` (pt fonte, en) |
| tray | submenu de microfone no menu em repouso, entre idioma e "Descarregar modelo" |

## Relations

`history.db.transcription_history.paste_failed` (novo, sem índice) — lido só pelo `HistoryManager`.

## Surface

Comandos Tauri novos `history_search(query: string) -> HistoryEntry[]` e
`recover_history_entry(id: number) -> HistoryEntry`; `HistoryEntry` ganha `paste_failed` e
`discarded`. Consumidos só por este front (`src/bindings.ts` gerado).

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. onde fica "a cola falhou" | coluna `paste_failed` no `history.db` (migração 6, padrão 0) | campo no `fala.sqlite`: exige mudar `crates/storage`, fora do escopo desta feature; deduzir pelo log: não há ligação linha↔log |
| 2. comandos | `history_search(query)` com teto fixo de 100; `recover_history_entry(id)` devolve a entrada | paginar a busca: o FTS já ordena por data e 100 cobre a tela; recuperar no front com `undo`: não zera `paste_failed` |
| 3. ids do tray | `microphone_auto` e `microphone:<nome do dispositivo>`, gravando `None`/nome em `selected_microphone` como o seletor de Geral | id por índice: muda quando um dispositivo entra ou sai |
| 4. cancelamento | Esc continua sem deixar rastro (`deliver_unless_cancelled`), então não há linha "descartada" por cancelamento | gravar o bruto no cancel: muda o contrato da 1.F2 ("cancel não deixa rastro") e mexe no `actions.rs` de outro painel |

- Rota inicial `history` é reversível (um `useState`).

## Criteria

1. WHEN a dictation's paste fails THEN its history row SHALL be saved with `paste_failed = true`; a delivered or empty one with `false`
2. An entry SHALL be `discarded` iff `paste_failed`, or its dictation shows empty text while its raw text is not empty
3. `recover_history_entry` SHALL clear `paste_failed`, switch an emptied dictation to its raw text, return the entry with `discarded = false`, and SHALL fail without change on an entry that is not discarded or does not exist
4. `history_search(q)` SHALL return, newest first, the history entries whose dictation matches `q` in `fala-storage` (accent-insensitive), each with its dictation; dictations without a history row SHALL be skipped; an unavailable store SHALL be an error
5. Migration 6 SHALL add `paste_failed` NOT NULL default 0 and keep existing rows
6. The main window SHALL open on the history section, listed first in the sidebar
7. The history screen SHALL group entries by local day, labelled "Hoje"/"Ontem"/date, with the time in its own column and the text clamped to two lines
8. The search field SHALL call `history_search` 200 ms after the last keystroke and return to the paged list when emptied
9. A discarded entry SHALL render grey "Este ditado foi descartado." with "Recuperar"; clicking it calls `recover_history_entry`, copies the shown text and replaces the row
10. WHILE idle the tray menu SHALL show "Microfone" with "Automático (<padrão>)" and each input device, checking "Automático" when `selected_microphone` is unset and the named device otherwise; WHILE busy it SHALL be absent
11. WHEN a microphone item is clicked THEN the desktop SHALL store the choice like the Geral selector, switch the device, emit `settings-changed` and rebuild the tray; unknown ids are ignored
12. Every new string SHALL come from i18next (`pt` source, `en`), tray strings from the `tray` section

## Out of scope

| Excluded | Why |
| --- | --- |
| Linha descartada para Esc | door 4 |
| Menu ⋮, play no hover, "Extract audio" | pesquisa 20 § 3 item 2 pede só busca, agrupamento e Recuperar; o player continua ao expandir |
| Rodar a lista de microfones sem evento do SO | sem hotplug; a lista se refaz ao passar o mouse no ícone |
| Busca no `history.db` (linhas sem item) | linhas sem texto não têm o que achar |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Linha sem texto do ASR | segue "a transcrição falhou" + re-transcrever | não há bruto para recuperar e erro × vazio não se distinguem no `history.db` | y — brief do orquestrador ("Recuperar quando o texto bruto existir") |
| Recuperar | copia o texto e devolve a linha ao normal | igual ao "desfazer edição da IA", que também copia | y — decidido pelo executor |
| Ordem do submenu | após "Idioma do ditado" | os três decidem como o próximo ditado é captado/transcrito | y — decidido pelo executor |
| Hora | `HH:mm` local via `Intl` no idioma da UI | | y — decidido pelo executor |

**Open questions:** none.

## Sources

- `fala-research/research/20-ui-wispr-granola.md` § 3 itens 2, 3, 8 e § 5; capturas `wispr/08`, `10`, `12`, `13`
- `fala-research/plans/status/sintese-2026-10-09.md` § 5 itens 2 e 4; `inventario-2026-10-09.md` § 1
