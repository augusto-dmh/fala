# language-picker — pt-BR ou en no tray, um idioma por ditado

## Problem

Para trocar o idioma do ditado é preciso abrir a janela de configurações, achar o card do modelo
e rolar uma lista com dezenas de idiomas mais `auto` (`front:components/settings/LanguageSelector.tsx`,
dentro de `ModelSettingsCard`). Quem dita em pt-BR o dia inteiro e às vezes escreve uma mensagem
em inglês paga essa ida à janela a cada troca, ou deixa em `auto` e aceita que a detecção erre em
frases curtas. A instalação nova também nasce em `auto` (`settings.rs:565-567`, `:937`), embora o
pitch seja "Parakeet local transcreve pt-BR" e o no-go diga "sem suporte a mais de um idioma no
mesmo ditado". O delta registra a lacuna (W12) e decide o lugar do seletor: submenu no tray, não
na pill (D7, a pill de 84×30 px não tem texto).

A fonte não traz números de frequência de troca.

Quando isto entrar, o menu do tray em repouso tem um submenu "Idioma do ditado" com "Português
(Brasil)" e "Inglês", o atual marcado; um clique grava a escolha no settings e vale para o
próximo ditado. Instalação nova nasce em pt-BR.

## Flow

Reusa o campo `selected_language` do settings, o comando `change_selected_language_setting` e a
coerção `effective_language` (`managers/model.rs`), que já casa `pt-BR` com o `pt` do modelo; não
cria um segundo campo de idioma nem depende de `crates/core`.

1. `tray::compute_desired` (exists) - lê `selected_language` e põe no `MenuInputs` a escolha do tray (`pt-BR`, `en` ou nenhuma), de modo que uma troca reconstrói o menu
2. `tray::build_menu` (exists) - no menu em repouso, um `Submenu` "Idioma do ditado" com dois `CheckMenuItem` de ids `dictation_language:pt-BR` e `dictation_language:en` (door 2); o menu ocupado (gravando ou processando) não tem o submenu
3. clique -> `on_menu_event` em `lib.rs` (exists) - reconhece o id, aceita só as duas tags e chama `shortcut::change_selected_language_setting` (exists)
4. `change_selected_language_setting` (exists) - grava `selected_language`, emite `settings-changed` com `setting: "selected_language"` e pede `tray::update_tray_menu`, para o front e o tray refletirem a troca venha ela do tray ou da tela de configurações
5. out: o ditado seguinte lê `selected_language` em `managers/transcription.rs` (exists) e `effective_language` entrega ao motor o código do modelo (`pt` para `pt-BR`)
6. `settings::get_default_settings` e `default_selected_language` (exist) - instalação nova e store sem a chave nascem com `"pt-BR"` (door 1)

## Impact

| Front | What changes |
| --- | --- |
| domain | termo novo: escolha do tray - `pt-BR` quando `selected_language` tem base `pt`, `en` quando tem base `en`, nenhuma para `auto` ou outro idioma. Vive em `tray.rs` |
| domain | termo existente: o padrão de `selected_language` era `"auto"` e passa a `"pt-BR"`. Quem ramifica nisso hoje: `effective_language` (com `auto`, detecção ou fallback em inglês) e a escolha das palavras de preenchimento em `managers/transcription.rs:1695`; a tela `LanguageSelector` mostra "Portuguese" para `pt-BR` pela base do código |
| behaviour | `change_selected_language_setting` passa a emitir `settings-changed` e atualizar o tray; antes não fazia nenhum dos dois |
| stored data | nada a migrar. Stores existentes guardam `selected_language` explícito (`auto`, `pt`, outros) e continuam com ele; com `auto` ou outro idioma o submenu aparece sem marca até o primeiro clique (door 1) |
| ui | o menu do tray em repouso ganha um submenu entre o de modelo e "Descarregar modelo"; três chaves novas na seção `tray` de `pt` e `en`, lidas pelo `build.rs` |

## Relations

`None - no stored-data shape change` (o campo `selected_language` já existe; muda só o padrão,
door 1).

## Surface

`None - nothing consumed outside`. `change_selected_language_setting` mantém a assinatura; o
evento `settings-changed` já existe e é consumido só por este frontend.

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. padrão de `selected_language` em instalação nova e em store sem a chave | `"pt-BR"` em `default_selected_language()` e em `get_default_settings()`; o tray grava `"pt-BR"` ou `"en"`, as formas serializadas de `fala_core::Language` no contrato S0; stores existentes não são migrados | migrar `auto` e `pt` para `pt-BR` (porta 1 de F4 no delta): o store serializa todos os campos, então não há como distinguir "nunca mexeu" de "escolheu `auto`", o mesmo motivo da door 1 da 1.F1; gravar `"pt"` como a tela herdada: diverge da tag do contrato S0 que a F9 vai ler |
| 2. ids dos itens do tray | `dictation_language:pt-BR` e `dictation_language:en`, no mesmo esquema `prefixo:valor` de `model_select:<id>` | ids por índice (`dictation_language:0`): quebram se a ordem mudar e não carregam a tag que vai para o settings |

- Nothing else in this change is hard to reverse: o submenu e as chaves de i18n são lidos só por este app, e o `settings-changed` extra é idempotente no front.

## Criteria

### S1: o submenu do tray (P1)

**Acceptance Criteria**

1. The tray SHALL map `selected_language` to its choice: base `pt` (`pt`, `pt-BR`) to `pt-BR`, base `en` (`en`, `en-US`) to `en`, and anything else (`auto`, `es`, empty) to no choice
2. The tray menu inputs SHALL include that choice, so two snapshots that differ only in it compare unequal and the menu is rebuilt
3. WHILE the tray is idle the menu SHALL show a "Idioma do ditado" submenu with two check items, `dictation_language:pt-BR` and `dictation_language:en`, the current choice checked; WHILE it is busy the submenu SHALL be absent
4. WHEN a tray item id `dictation_language:<tag>` is clicked THEN the desktop SHALL accept only `pt-BR` and `en` as `<tag>` and SHALL ignore any other id
5. WHEN an accepted language item is clicked THEN the desktop SHALL write `<tag>` to `selected_language` through `change_selected_language_setting`
6. WHEN `change_selected_language_setting` runs THEN it SHALL emit `settings-changed` with `setting` = `"selected_language"` and request a tray menu update
7. The tray SHALL label the submenu "Idioma do ditado" / "Dictation language" and the items "Português (Brasil)" / "Portuguese (Brazil)" and "Inglês" / "English", in pt and en, from the `tray` section of the locale files

**Independent test:** `cargo test -p fala --lib tray`; no `tauri dev` da sessão Windows, abrir o tray, trocar para Inglês, ditar uma frase em inglês.

### S2: padrão e efeito no ditado (P1)

**Acceptance Criteria**

8. The settings SHALL default `selected_language` to `"pt-BR"`, both in `get_default_settings()` and when the stored object lacks the key
9. WHEN a stored settings object holds `"auto"`, `"pt"` or `"es"` THEN loading with migrations SHALL keep that value
10. WHEN the intent is `"pt-BR"` and the model advertises `pt` THEN `effective_language` SHALL return `"pt"`

**Independent test:** `cargo test -p fala --lib settings` e `cargo test -p fala --lib managers::model`.

## Out of scope

| Excluded | Why |
| --- | --- |
| Reduzir a tela de configurações a pt-BR/en e tirar o `auto` de lá | o catálogo herdado ainda tem Whisper e outros modelos multilíngues até a F9 cortar o catálogo (D8); a tela segue servindo a eles. O tray oferece só os dois |
| Atalho de teclado para alternar o idioma | D7 (c) do delta: só se o uso diário mostrar troca frequente |
| Seletor na pill | D7: a pill não tem texto |
| Persistir como `fala_core::Language` | o tipo do S0 não está no `main`; o valor gravado já é a forma serializada dele (door 1) e a F9 faz a ligação |
| Registrar a divergência do design doc em `decisions-log.md`/ADR | pedido do delta ao orquestrador, fora do repo e desta feature |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Rótulos dos idiomas | traduzidos para o idioma da interface ("Inglês" em pt, "English" em en), não autônimos | o tray inteiro segue o idioma da interface e o seletor herdado também usa nomes traduzidos | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| Submenu no menu ocupado | ausente | a troca no meio de um ditado valeria para o ditado em voo (o idioma é lido na transcrição), contra "um idioma por ditado"; o submenu de modelo faz o mesmo | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| Store existente com `auto` (máquina do Augusto) | sem migração; o submenu aparece sem marca e um clique resolve | ver door 1; vira item do checklist `TODO(windows)` | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| Posição do submenu | logo depois do submenu de modelo | os dois decidem como o próximo ditado é transcrito | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| Rótulo do submenu | fixo ("Idioma do ditado"), com a marca nos itens | mostrar o idioma no rótulo duplicaria a marca; o de modelo mostra o nome porque a lista pode ser longa | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |

**Open questions:** none - all resolved or logged above.

## Observable

| Surface | Decision | Landing |
| --- | --- | --- |
| tray · submenu "Idioma do ditado" | itens, ordem e textos | AC 3, AC 7 |
| tray · submenu | estado vazio (store com `auto` ou outro idioma) | AC 1 (nenhum item marcado) |
| tray · submenu | carregando | n/a - o menu é montado de um snapshot síncrono do settings |
| tray · submenu | erro ao gravar | existing - `write_settings` herdado não devolve erro; o tray é reconstruído do settings lido, então mostra o que ficou gravado |
| tray · submenu | durante um ditado | AC 3 (ausente no menu ocupado) |
| tray · submenu | ação destrutiva | n/a - trocar o idioma não apaga nada e se desfaz com outro clique |
| tela de configurações · idioma | reflete a troca feita no tray | AC 6 (`settings-changed` recarrega o store do front) |

## Sources

- `fala-research/pitches/fase-1-ditado-windows.md` - "seletor pt-BR/en" nas semanas 3-4; no-go de mais de um idioma por ditado
- `fala-research/plans/fase-1-delta-e-semanas-3-4.md` - item W12, feature F4 e decisão D7 (tray, não pill)
