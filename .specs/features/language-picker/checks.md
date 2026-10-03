# language-picker checks

Profile: light
Plan: `.specs/features/language-picker/plan.md`

10 checks in 2 slices · 2 one-way doors · 0 open, of which 0 block

Comandos de cargo com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`,
rodados na raiz do worktree.

## Checks

### S1 - o submenu do tray · 4 files · 75 KB · ~19k (só os trechos tocados de `lib.rs` e `shortcut/mod.rs`)

**C1** - `tray_language_choice` devolve `Some("pt-BR")` para `"pt"`, `"pt-BR"`; `Some("en")` para `"en"`, `"en-US"`; e `None` para `"auto"`, `"es"` e `""` (AC 1, tabela com 7 entradas)
Proof: `cargo test -p fala --lib tray::tests::tray_language_choice_maps_the_stored_intent`

**C2** - Dois `MenuInputs` iguais exceto pela escolha de idioma (`Some("pt-BR")` contra `Some("en")`, e `Some("en")` contra `None`) comparam diferentes (AC 2)
Proof: `cargo test -p fala --lib tray::tests::language_choice_change_rebuilds_the_menu`

**C3** - `language_item_id(tag)` gera `dictation_language:pt-BR` e `dictation_language:en`, e `parse_language_item` devolve a tag para esses dois ids e `None` para `dictation_language:es`, `dictation_language:`, `dictation_language:PT-BR` e `model_select:pt-BR` (AC 3 ids, AC 4, door 2)
Proof: `cargo test -p fala --lib tray::tests::language_item_ids_round_trip_only_for_known_tags`

**C4** - Em `build_menu`, o submenu de idioma é montado só no ramo em repouso: o trecho do ramo ocupado não menciona `language_submenu` e o ramo em repouso o inclui em `Menu::with_items`, com um `CheckMenuItem` por tag marcado por `inputs.language_choice` (AC 3)
Proof: `! awk '/let menu = if inputs.busy/,/} else {/' apps/desktop/src/tray.rs | grep -q language_submenu`
Proof: `awk '/} else {/,/^    };/' apps/desktop/src/tray.rs | grep -q '&language_submenu'`
Proof: `grep -q 'Some(\*tag) == inputs.language_choice' apps/desktop/src/tray.rs`

**C5** - O handler de menu do tray em `lib.rs` passa a tag de `parse_language_item` para `shortcut::change_selected_language_setting`, e só para ela (AC 4, AC 5)
Proof: `grep -A4 'parse_language_item(id)' apps/desktop/src/lib.rs | grep -q 'change_selected_language_setting'`

**C6** - `change_selected_language_setting` emite `"settings-changed"` com `"setting": "selected_language"` e chama `crate::tray::update_tray_menu` (AC 6)
Proof: `awk '/^pub fn change_selected_language_setting/,/^}/' apps/desktop/src/shortcut/mod.rs | grep -q '"setting": "selected_language"'`
Proof: `awk '/^pub fn change_selected_language_setting/,/^}/' apps/desktop/src/shortcut/mod.rs | grep -q 'tray::update_tray_menu'`

**C7** - A seção `tray` tem `dictationLanguage` = "Idioma do ditado" / "Dictation language", `languagePtBr` = "Português (Brasil)" / "Portuguese (Brazil)" e `languageEn` = "Inglês" / "English" em pt / en, a paridade de chaves passa e o `build.rs` gera os campos (o crate compila com `strings.dictation_language`) (AC 7)
Proof: `python3 -c 'import json,sys; g=lambda l: json.load(open(f"src/i18n/locales/{l}/translation.json"))["tray"]; p,e=g("pt"),g("en"); sys.exit(0 if (p["dictationLanguage"],p["languagePtBr"],p["languageEn"])==("Idioma do ditado","Português (Brasil)","Inglês") and (e["dictationLanguage"],e["languagePtBr"],e["languageEn"])==("Dictation language","Portuguese (Brazil)","English") else 1)'`
Proof: `bun run check:translations`
Proof: `cargo test -p fala --lib tray::tests::tray_language_choice_maps_the_stored_intent`

### S2 - padrão e efeito no ditado · 2 files · 170 KB · ~3k (só os testes e a função de padrão)

**C8** - `get_default_settings().selected_language` e `serde_json::from_value::<AppSettings>(json!({})).selected_language` valem `"pt-BR"` (AC 8, os dois lugares do padrão)
Proof: `cargo test -p fala --lib settings::tests::default_selected_language_is_pt_br`

**C9** - Um store com `settings_schema_version: 2` e `selected_language` em `"auto"`, `"pt"` ou `"es"` desserializa e, depois de `apply_settings_migrations`, mantém exatamente o valor guardado (AC 9, tabela com 3)
Proof: `cargo test -p fala --lib settings::tests::stored_selected_language_loads_unchanged`

**C10** - `effective_language("pt-BR", ["en", "pt"], true)` devolve `"pt"`, e com `["en", "pt"]` e detecção desligada também `"pt"` (AC 10)
Proof: `cargo test -p fala --lib managers::model::tests::test_effective_language_resolves_pt_br_intent_to_model_pt`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| intenção guardada -> escolha do tray (7) | `pt` C1 · `pt-BR` C1 · `en` C1 · `en-US` C1 · `auto` C1 · `es` C1 · vazio C1, table-driven | - |
| ids do tray (6) | `dictation_language:pt-BR` C3 · `dictation_language:en` C3 · tag desconhecida C3 · tag vazia C3 · caixa diferente C3 · prefixo de outro item C3 | - |
| estados do menu (2) | em repouso, com submenu C4 · ocupado, sem submenu C4 | - |
| entradas que gravam o idioma (2) | tray C5 · tela de configurações (comando existente) C6 | - |
| efeitos da gravação (2) | `settings-changed` C6 · tray atualizado C6 | - |
| locales (2) | pt C7 · en C7 | - |
| startup config: origem do padrão (2 lugares) | `get_default_settings` C8 · `#[serde(default)]` com a chave ausente C8 | - |
| valores guardados que não migram (3) | `auto` C9 · `pt` C9 · `es` C9, table-driven | - |
| door 1 (padrão `pt-BR`) (3) | padrão C8 · sem migração C9 · chega ao motor como `pt` C10 | - |
| door 2 (ids) (1) | C3 | - |

- C4, C5 e C6 são estruturais: `build_menu` e o handler do tray precisam de um `AppHandle`; a parte que decide (escolha, ids) está nas funções puras de C1 e C3
- Nenhum check fica só no Windows; o visual do tray no Windows entra no checklist `TODO(windows)` da sessão, sem check próprio

## Swept

- validation: C3 (só as duas tags entram no settings pelo tray)
- failure modes: existing - `write_settings` herdado; o menu é reconstruído do settings relido, C2
- idempotency: C2 (clicar no idioma já marcado grava o mesmo valor e não muda o snapshot do menu)
- authorization: n/a - menu local da própria app
- concurrency: existing - o tray aplica snapshots com sequência e coalescência (`tray.rs`, cabeçalho do módulo); a troca vale para o ditado seguinte e o submenu some no menu ocupado, C4
- data lifecycle: C9 (stores existentes não são reescritos)
- dependency failure: n/a - nenhuma dependência externa; modelo sem `pt` cai na coerção herdada de `effective_language`
- state transitions: C4 (repouso/ocupado)
- observability: n/a - nenhum log novo; a troca aparece no `settings-changed` já existente

## Handoff

- S1 = ~19k, S2 = ~3k, total ~22k, tudo em `apps/desktop/src` e nas locales, abaixo do budget de 150k - one builder
