# cleanup-level — quanto a IA mexe no ditado, em quatro cards

## Problem

O `CleanupLevel` (`verbatim`, `light`, `medium`, `heavy`) existe em `fala-postproc` desde o #93,
com uma instrução própria no `systemInstruction` e `Postprocessor::with_cleanup_level`, mas o
desktop nunca o escolhe: `llm_auto::formatter_at` monta o `Postprocessor` sem nível, então todo
ditado sai em `light`. Só o `fala-cli bench format --level` alcança os outros. A queixa recorrente
contra o Wispr é a reescrita indesejada, e o Wispr explica os níveis com o mesmo exemplo
antes/depois em cada card. Hoje a tela da IA tem só o toggle, a chave e os apps sem IA.

Quando isto entrar, Configurações → Geral → IA tem, logo abaixo do toggle, quatro cards
(Literal, Leve, Média, Forte) com o mesmo ditado de exemplo transformado em cada um; o escolhido
vale para o próximo ditado formatado pelo Gemini, inclusive a edição tardia.

## Flow

1. `settings::CleanupLevel` (new) - enum do desktop com `serde` em minúsculas e `specta::Type`, mapeado para `fala_postproc::CleanupLevel` por `From`; o crate não ganha `specta`
2. `AppSettings::cleanup_level` (new) - `#[serde(default)]` = `light`; um valor desconhecido faz o parse estrito falhar e o `salvage_settings` herdado mantém o padrão com `warn` só da chave (door 1)
3. `shortcut::change_cleanup_level_setting` (new) - grava o campo, no padrão de `change_llm_enabled_setting`; registrado em `lib.rs` e exportado para `bindings.ts` (door 2)
4. `llm_auto::formatter_at` (exists) - passa `.with_cleanup_level(settings.cleanup_level.into())`; a edição tardia e o "Aplicar edição da IA" saem do mesmo `Postprocessor` (`ask` captura o nível)
5. `settingsStore.ts` (exists) - `cleanup_level` → `commands.changeCleanupLevelSetting`
6. `CleanupLevelCards.tsx` + `cleanupLevelModel.ts` (new) - `radiogroup` de quatro cards com setas, texto estático do i18n; esmaecidos quando a IA está desligada ou sem chave
7. `LlmSettings.tsx` (exists) - os cards logo abaixo do toggle

## Impact

| Front | What changes |
| --- | --- |
| domain | termo novo no desktop: nível de limpeza, o mesmo vocabulário do `fala-postproc` |
| behaviour | o `systemInstruction` passa a levar a instrução do nível escolhido; com o padrão, o pedido é idêntico ao de hoje |
| stored data | campo novo `cleanup_level` no store; stores antigos carregam `light` sem migração |
| ui | grupo de cards abaixo do toggle da IA; chaves novas em `settings.llm.cleanup` nos dois locales |

## Relations

`None - additive field with default; no existing field changes shape`.

## Surface

`change_cleanup_level_setting(level: CleanupLevel)` e o tipo `CleanupLevel` em `bindings.ts`,
consumidos só por este frontend.

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. literal guardado | `"cleanup_level": "verbatim" \| "light" \| "medium" \| "heavy"`, os mesmos de `fala_postproc::CleanupLevel::as_str` e do `--level` do bench; ausente ou desconhecido → `light` | nomes da UI (`literal`, `leve`): amarram o store ao idioma; um número: ilegível no store e no TOML futuro do CLI |
| 2. comando | `change_cleanup_level_setting` com argumento `level` do tipo `CleanupLevel` | `String` com parse no comando: o specta já entrega o union e o erro de tipo fica no compilador do TS |

- Nothing else is hard to reverse: os textos dos cards são estáticos e lidos só aqui.

## Criteria

### S1: setting e uso (P1)

1. The settings SHALL default `cleanup_level` to `light`, both in `get_default_settings()` and when the stored object lacks the key
2. WHEN the stored `cleanup_level` is not one of the four literals THEN loading SHALL keep every other valid field and set `cleanup_level` to `light`, logging a warning that names only the key
3. WHEN the stored `cleanup_level` is one of the four literals THEN loading SHALL keep it, and each maps to the `fala_postproc::CleanupLevel` of the same name
4. WHEN a dictation is formatted by the Gemini THEN the `systemInstruction` SHALL contain the instruction of the stored level, and with the default it SHALL contain `CleanupLevel::Light.instruction()`
5. WHEN `change_cleanup_level_setting` runs THEN it SHALL write the level to the settings

**Independent test:** `cargo test -p fala --lib llm_auto` e `cargo test -p fala --lib settings`.

### S2: os cards (P1)

6. The IA group SHALL show, right below the AI toggle, a `radiogroup` with four radios in this order: Literal, Leve, Média, Forte (`verbatim`, `light`, `medium`, `heavy`), each with a one-line description and the same example dictation transformed, all from i18n
7. The radio of the stored level SHALL be `aria-checked="true"` and the only one in the tab order, with an `accent` border
8. WHEN ArrowRight/ArrowDown or ArrowLeft/ArrowUp is pressed on a card THEN the next/previous level SHALL be chosen, wrapping around; Home/End choose the first/last
9. WHILE the AI is off or has no key the cards SHALL be dimmed but readable and show "vale quando a IA formata o ditado"; they stay clickable
10. The group SHALL show the footer "O original nunca se perde: Desfazer edição da IA, no Início."
11. The keys SHALL exist in pt and en with the same structure

**Independent test:** `bun src/components/settings/cleanupLevel.test.tsx`.

## Out of scope

| Excluded | Why |
| --- | --- |
| Nível por app | fase 4 (`fase-4-estilo-por-app.md`) |
| `FillerWordRemoval`, `Rules`, texto das instruções do prompt, toggle da IA | definição da feature; o toggle decide se o texto sai da máquina |
| Nível no TOML do `fala-cli` | o `--level` do bench basta |
| `settings-changed` no comando | `change_llm_enabled_setting` também não emite; só a tela escreve o campo |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Cards esmaecidos aceitam clique | sim | escolher o nível antes de colar a chave é legítimo; o esmaecido só avisa que não vale agora | y — delegado pelo Augusto, decidido pelo executor |
| Layout | grade de 2 colunas (1 em janela estreita) | quatro exemplos lado a lado não cabem em `max-w-3xl` | y — delegado, decidido pelo executor |
| Exemplo en | tradução da ideia do pt, escrita à mão | definição | y |

**Open questions:** none.

## Observable

| Surface | Decision | Landing |
| --- | --- | --- |
| Geral → IA · cards | ordem, textos, exemplo | AC 6, AC 11 |
| cards | selecionado | AC 7 |
| cards | IA desligada ou sem chave | AC 9 |
| cards | carregando | n/a - lê o settings já carregado; a gravação é otimista e os cards não ficam desabilitados, para o foco não se perder no meio da navegação por setas |
| cards | erro ao gravar | existing - o `settingsStore` reverte e loga, como nos outros settings |
| cards | ação destrutiva | n/a - outro clique desfaz; o original fica no histórico (AC 10) |

## Sources

- `fala-research/plans/features/cleanup-level-cards.md` (definição, vale como pitch)
- `fala-research/research/20` § 1 e § 3 item 4; `research/18` § 1 #13 e § 4.4; `research/16` § 3.5
