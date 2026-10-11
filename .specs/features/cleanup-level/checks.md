# cleanup-level checks

Profile: light
Plan: `.specs/features/cleanup-level/plan.md`

11 checks in 2 slices · 2 one-way doors · 0 open, of which 0 block

Comandos de cargo com `CARGO_TARGET_DIR=C:\f\cleanup-level-cards`, rodados na raiz do worktree.

## Checks

### S1 - setting e uso · 4 files

**C1** - `get_default_settings().cleanup_level` e `from_value::<AppSettings>(json!({})).cleanup_level` valem `CleanupLevel::Light`, e um store de schema 2 sem a chave carrega `Light` depois das migrações (AC 1)
Proof: `cargo test -p fala --lib settings::tests::cleanup_level_defaults_to_light`

**C2** - Um store com `"cleanup_level": "extreme"` (e com `42`) falha o parse estrito; `salvage_settings` devolve `Light` e mantém `selected_model` e `onboarding_completed` guardados (AC 2; o `warn` só com a chave é o do `salvage_settings` herdado)
Proof: `cargo test -p fala --lib settings::tests::invalid_cleanup_level_is_salvaged_to_light`

**C3** - Para cada um dos quatro literais, o store carrega o valor, ele serializa de volta com o mesmo literal, e `fala_postproc::CleanupLevel::from(v).as_str()` é o literal (AC 3, door 1, tabela com 4)
Proof: `cargo test -p fala --lib settings::tests::stored_cleanup_levels_round_trip`

**C4** - Com o servidor falso do Gemini, para cada nível o `systemInstruction` contém `fala_postproc::CleanupLevel::<nível>.instruction()` e não contém a dos outros três; com `get_default_settings()` contém a de `Light` (AC 4)
Proof: `cargo test -p fala --lib llm_auto::tests::cleanup_level_reaches_the_llm_prompt`

**C5** - `change_cleanup_level_setting` atribui `settings.cleanup_level = level` e chama `write_settings`, está em `collect_commands!` e `bindings.ts` tem `changeCleanupLevelSetting(level: CleanupLevel)` e `CleanupLevel = "verbatim" | "light" | "medium" | "heavy"` (AC 5, door 2)
Proof: `awk '/^pub fn change_cleanup_level_setting/,/^}/' apps/desktop/src/shortcut/mod.rs | grep -q 'settings.cleanup_level = level'`
Proof: `awk '/^pub fn change_cleanup_level_setting/,/^}/' apps/desktop/src/shortcut/mod.rs | grep -q 'write_settings'`
Proof: `grep -q 'shortcut::change_cleanup_level_setting,' apps/desktop/src/lib.rs`
Proof: `grep -q 'export type CleanupLevel = "verbatim" | "light" | "medium" | "heavy"' src/bindings.ts`
Proof: `grep -q 'async changeCleanupLevelSetting(level: CleanupLevel)' src/bindings.ts`

### S2 - os cards · 5 files

**C6** - `CleanupLevelCards` renderiza um `role="radiogroup"` com quatro `role="radio"` na ordem `verbatim, light, medium, heavy`, cada um com rótulo, descrição e o exemplo de `settings.llm.cleanup`; o bruto aparece uma vez, fora dos cards (AC 6)
Proof: `bun src/components/settings/cleanupLevel.test.tsx` (linha `C6 ok`)

**C7** - Com `value="medium"`, só o radio `medium` tem `aria-checked="true"`, `tabindex="0"` e `border-accent`; os outros têm `tabindex="-1"` (AC 7)
Proof: `bun src/components/settings/cleanupLevel.test.tsx` (linha `C7 ok`)

**C8** - `levelForKey` devolve o próximo/anterior com volta para ArrowRight/ArrowDown/ArrowLeft/ArrowUp, o primeiro/último para Home/End, e `null` para outras teclas; cada radio liga `onClick` a `onChange(level)` e `onKeyDown` ao handler, que escolhe o nível e move o foco (AC 8, tabela + fonte)
Proof: `bun src/components/settings/cleanupLevel.test.tsx` (linha `C8 ok`)

**C9** - Com `dimmed`, o grupo tem `opacity-60` e mostra `settings.llm.cleanup.inactive`; sem `dimmed`, nenhum dos dois; os radios não ficam `disabled` (AC 9)
Proof: `bun src/components/settings/cleanupLevel.test.tsx` (linha `C9 ok`)

**C10** - O grupo mostra `settings.llm.cleanup.footer`, e o pt diz "O original nunca se perde: Desfazer edição da IA, no Início." e "vale quando a IA formata o ditado" no `inactive`; `LlmSettings.tsx` monta os cards logo depois do `ToggleSwitch`, sem outro elemento entre os dois, com `dimmed={!enabled || !apiKey}` (AC 9, AC 10)
Proof: `bun src/components/settings/cleanupLevel.test.tsx` (linha `C10 ok`)

**C11** - As chaves de `settings.llm.cleanup` existem em pt e en com a mesma estrutura, os exemplos pt são os da definição, e a paridade geral passa (AC 11)
Proof: `bun src/components/settings/cleanupLevel.test.tsx` (linha `C11 ok`)
Proof: `bun run check:translations`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| origens do padrão (3) | `get_default_settings` C1 · chave ausente C1 · store antigo migrado C1 | - |
| valores guardados (4 + 2 inválidos) | 4 literais C3 · string desconhecida C2 · tipo errado C2 | - |
| caminhos que pedem ao Gemini (3) | ditado C4 · edição tardia e "Aplicar edição da IA": mesmo `Postprocessor`, `ask` captura `self.cleanup` (estrutural, `crates/postproc/src/lib.rs`) | runtime da edição tardia por nível |
| cards (4) | C6 ordem · C7 selecionado · C8 teclado · C9 esmaecido | - |
| locales (2) | pt C11 · en C11 | - |

- Manual no Windows (Augusto, com chave): trocar para "Forte", ditar > 15 palavras no Bloco de Notas, ver o texto mais curto; voltar para "Leve". Fica fora da tabela, como checklist no PR; não muda o veredito.

## Swept

- validation: C2 (valor inválido no store); o comando recebe o enum tipado
- failure modes: existing - `settingsStore` reverte a UI se o comando falhar
- idempotency: escolher o card já marcado grava o mesmo valor
- authorization: n/a - setting local
- concurrency: o nível é lido quando o ditado monta o formatter; a troca vale para o próximo
- data lifecycle: C1 (stores antigos não são reescritos além do campo novo)
- dependency failure: n/a - o nível só muda o texto do prompt
- state transitions: C9 (IA ligada/desligada, com/sem chave)
- observability: o `warn` do salvage nomeia só a chave

## Handoff

- S1 + S2 bem abaixo do budget; one builder
- **Settled mid-build:** os cards não ficam `disabled` durante a gravação do setting (o foco se perderia no meio da navegação por setas); o `bindings.ts` regenerado também reordenava três comandos de histórico já existentes, e o commit leva só os três trechos desta feature
- **Abandoned:** nada
- **Boundary:** C1-C11 fechados em `feat/cleanup-level-cards`; o efeito de cada nível num ditado real fica para o manual no Windows com chave
