# ui-onboarding checks

Profile: light
Plan: none - cada PR cabe numa frase; a especificação é a Parte 3 do relatório de UI (`fala-research/research/21-ui-direcao-e-diagnostico.md`): §3.9 e a linha 5 da §3.10 para o PR 1, a decisão D17 de `.specs/features/ui-tinta/checks.md` para o PR 2, a §3.11 (com §3.4 e §3.7) para o PR 3. Os blocos são um por PR, empilhados sobre `feat/ui-rail-home` (#71): S1 = PR 1, S2 = PR 2, S3 = PR 3.

## Intent

O primeiro uso do Fala ainda é o do Handy: um passo de permissões pensado para o macOS e uma lista de cartões de modelo Whisper que contradiz a ADR-0009. Com o PR 1, quem abre o Fala pela primeira vez passa por três passos (microfone com medidor, atalho com um teste de ditado de verdade, IA opcional) enquanto o Parakeet baixa sozinho, e termina em Início. Com o PR 2, o tray deixa de oferecer "Verificar atualizações", que não faz nada com o updater desligado. Com o PR 3, a direção de UI (abrir no conteúdo, cor só semântica, tokens numa fonte única) vira uma ADR proposta, com os valores num `docs/design/ui.md` que o teste de tokens mantém igual ao `theme.css`.

21 checks in 3 slices · 0 one-way doors · 0 open, of which 0 block

O repositório não tem vitest. A prova segue o padrão da pilha de UI: um script `node:assert` rodado pelo `bun`, que imprime `<id> ok` por check e sai com código diferente de 0 na primeira falha. `T4` = `bun src/components/onboarding/onboarding.test.tsx` (testa as funções puras de `onboardingModel.ts`, renderiza os passos com `react-dom/server` e lê `App.tsx`, os passos e os locales como texto; sem i18next inicializado, os rótulos aparecem como chaves). `T1` = `bun src/styles/tokens.test.ts`. As provas de Rust rodam sob o lock do target compartilhado (`CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=1`).

Decisões delegadas (Confirmed? y — delegado), uma linha cada:

- D1: o medidor do passo 1 usa `getUserMedia` + `AnalyserNode` no próprio WebView, aberto por um botão "Testar microfone", e escolhe o dispositivo pelo rótulo igual ao nome do mic escolhido (cai no padrão quando não acha). O backend só emite `mic-level` para a janela da pill e só durante uma gravação; um comando novo exigiria regenerar `src/bindings.ts` pelo `tauri dev`, que esta máquina não roda na rodada. O áudio fica no WebView, não sai da máquina (ADR-0009). Confirmed? y — delegado
- D2: sem medidor disponível (WebView sem `mediaDevices`, erro de permissão), o passo mostra o motivo numa linha `text-2` e continua passável: o teste real é o do passo 2. Confirmed? y — delegado
- D3: o Parakeet (`parakeet-tdt-0.6b-v3`) começa a baixar quando o assistente monta (passo 1) e é selecionado ao terminar, pelos mesmos `downloadModel`/`selectModel` do `modelStore`; com o modelo já baixado, só seleciona; já sendo o atual, nada. A prévia de depuração nunca baixa nem seleciona. Confirmed? y — delegado
- D4: `onboarding_completed` continua sendo gravado pelo backend ao selecionar o modelo (`set_active_model`); quem fecha o app no meio do passo 2 ou 3 depois do download volta direto para Início. Nenhum Rust novo para isso. Confirmed? y — delegado
- D5: o passo 2 chama `initializeEnigo` e `initializeShortcuts` ao montar (os dois já são idempotentes no backend), para o atalho funcionar antes de chegar em Início; o campo de teste é um `<textarea>` com foco automático, e a pill é a de verdade, porque o ditado cola no app focado, que é o próprio Fala. Confirmed? y — delegado
- D6: o atalho é desenhado a partir do `current_binding` de `transcribe` (`ctrl+shift+space` vira Ctrl, Shift, Space em `<kbd>`), não de um literal; trocar o atalho e o modo de ativação reaproveita `ShortcutInput` e `ShortcutActivationSetting`. Confirmed? y — delegado
- D7: o passo 3 reaproveita o `ApiKeyField` com o provedor `gemini` e grava pelo `updatePostProcessApiKey` (keyring, ADR-0008) ao sair do campo; "Pular" e "Concluir" terminam em Início do mesmo jeito, e não mexem em `llm_enabled`. Confirmed? y — delegado
- D8: quem já completou o onboarding e está com o mic negado no Windows reabre o assistente só com o passo do microfone (`steps={["microphone"]}`), que substitui o antigo passo de permissões; a checagem de acessibilidade do macOS sai do `App.tsx` (o banner `AccessibilityPermissions` continua em Início). Confirmed? y — delegado
- D9: `AccessibilityOnboarding.tsx` sai; `ModelCard.tsx` continua, porque Configurações › Avançado › Modelos o usa, e muda para `src/components/settings/models/`; as strings `onboarding.models`, `onboarding.modelCard`, `onboarding.recommended` e `onboarding.customModelDescription` ficam (o cartão as usa), as do assistente antigo saem. Confirmed? y — delegado
- D10: a prévia de depuração passa a abrir o assistente novo num passo escolhido (`microphone`, `shortcut`, `ai`), com os mesmos três botões de passo. Confirmed? y — delegado
- D11: o PR 2 cria `UPDATER_ENABLED = false` em `settings.rs`, espelho do `src/lib/updater.ts`, e o tray remove o item quando ele é `false`, pelo mesmo caminho do `FALA_DISABLE_UPDATER`; `update_checks_forced_disabled()` não muda de sentido. Confirmed? y — delegado
- D13: depois da rodada 1 do Verifier (FAIL em C3 e no caminho do usuário de volta), a sessão do medidor sai para `micLevelSession.ts` com as dependências do navegador injetáveis, para a prova de C3 ser por comportamento (parar com `getUserMedia` pendente e clique duplo); o assistente ganha `prepareModel` (padrão `true`), `false` no caminho do usuário de volta, que não baixa nem seleciona modelo e não mostra o rodapé do modelo (C20); um download ou seleção que falha sem o store gravar erro também vira `failed`, com "Tentar de novo". Confirmed? y — delegado
- D14: o PR 1 passa de 1.000 linhas fora dos locales (cerca de 720 são a remoção do assistente herdado e ~600 o script de prova); leva o rótulo `large-change` com a justificativa no corpo, como o #71. Confirmed? y — delegado
- D12: a ADR do PR 3 é a 0017 (nenhuma branch remota nem worktree tem 0017), `status: proposed`; o teste de tokens (`T1`) ganha um check que compara a tabela de cor do `docs/design/ui.md` com o `theme.css`, e é essa a Confirmação da ADR. Confirmed? y — delegado

## Checks

### S1 - PR 1, onboarding em três passos · 16 files · ~75 KB · ~20k

**C1** - `ONBOARDING_STEPS` é `["microphone", "shortcut", "ai"]`; `nextStep(steps, s)` dá `shortcut` depois de `microphone`, `ai` depois de `shortcut` e `null` depois de `ai`; `previousStep` dá `null` antes de `microphone`; com `steps = ["microphone"]`, `nextStep` depois de `microphone` é `null`
Proof: `T4` imprime `C1 ok`

**C2** - `micAccess(status)` é `"denied"` só com `supported: true` e `overall_access: "denied"`; `allowed`, `unknown` e `supported: false` dão `"ok"` (as 4 combinações); o passo do microfone renderizado com `"denied"` mostra o painel de como liberar (`onboarding.microphone.denied.title`, os três passos `onboarding.microphone.denied.step1..3` e o botão `onboarding.microphone.denied.openSettings`) e não mostra o seletor; com `"ok"` mostra o seletor de microfone e o medidor, e nenhum painel de acesso negado
Proof: `T4` imprime `C2 ok`

**C3** - O medidor: `levelFromTimeDomain` dá `0` para silêncio (todos os bytes `128`), `≥ 0.9` para uma onda quadrada de amplitude total e nunca passa de `1`; `pickInputDeviceId` devolve o `deviceId` cujo `label` é igual ao mic escolhido, `undefined` para `"Default"`, `null` ou nome sem par; renderizado, o medidor é um `role="meter"` com `aria-valuemin="0"`, `aria-valuemax="100"` e `aria-valuenow`; o componente pára todas as trilhas (`track.stop()`) e fecha o `AudioContext` ao desmontar
Proof: `T4` imprime `C3 ok`

**C4** - Download do Parakeet: `DICTATION_MODEL_ID` é `"parakeet-tdt-0.6b-v3"`; `modelAction` dá `"download"` para modelo não baixado e parado, `"select"` para baixado e não atual, `"none"` para o atual, para um download, verificação ou extração em curso e para modelo ausente do catálogo; o `Onboarding.tsx` chama `downloadModel(DICTATION_MODEL_ID)` e `selectModel(DICTATION_MODEL_ID)` e o efeito que os chama sai cedo com `preview`
Proof: `T4` imprime `C4 ok`

**C5** - O rodapé: `modelFooterState` dá `{ kind: "downloading", percent: 42 }` com progresso 42, `verifying`, `extracting`, `ready` (baixado e atual), `failed` (com `error` do store e sem download) e `pending` nos demais; o rodapé renderizado em `downloading` tem `role="progressbar"` com `aria-valuenow="42"` e o texto `onboarding.footer.downloading`; em `ready`, `onboarding.footer.ready` com a bolinha `bg-ok`
Proof: `T4` imprime `C5 ok`

**C6** - O passo do atalho desenha o `current_binding` de `transcribe` como teclas `<kbd>` (`ctrl+shift+space` vira três `<kbd>`: `Ctrl`, `Shift`, `Space`), tem um `<textarea>` com `aria-label="onboarding.shortcut.testLabel"`, e o arquivo usa `<ShortcutInput shortcutId="transcribe"`, `<ShortcutActivationSetting` e chama `commands.initializeEnigo()` e `commands.initializeShortcuts()`
Proof: `T4` imprime `C6 ok`

**C7** - O passo da IA renderiza a frase `onboarding.ai.privacy`, os botões `onboarding.ai.skip` e `onboarding.ai.finish`, e o arquivo usa `ApiKeyField` com `updatePostProcessApiKey("gemini"`; em pt, `onboarding.ai.privacy` contém "só o texto" e "nunca o áudio", e em en contém "only the text" e "never the audio"
Proof: `T4` imprime `C7 ok`

**C8** - O assistente renderizado mostra "passo N de 3" (`onboarding.progress`), o título do passo atual (`onboarding.<passo>.title`) e o rodapé do modelo; no passo `microphone` não há botão `onboarding.nav.back`, nos outros há; o último passo não tem `onboarding.nav.next` (termina pelos botões da IA)
Proof: `T4` imprime `C8 ok`

**C9** - `App.tsx` não cita `AccessibilityOnboarding`, `checkAccessibilityPermission`, `onModelSelected` nem um passo `"model"`; um usuário novo vê `<Onboarding` com os três passos; o usuário de volta com o mic negado no Windows vê `<Onboarding steps={["microphone"]}`; o fim do assistente leva a `"done"`, cujo destino inicial é `"home"`; `src/components/onboarding/AccessibilityOnboarding.tsx` e `src/components/onboarding/ModelCard.tsx` não existem, `src/components/settings/models/ModelCard.tsx` existe, e nenhum arquivo de `src/components/onboarding/` importa `ModelCard`
Proof: `T4` imprime `C9 ok`
Proof: `bunx tsc --noEmit` sai com 0

**C10** - A prévia de depuração: `OnboardingPreviewStep` é `"microphone" | "shortcut" | "ai"`; `OnboardingPreview` tem um botão por passo; o `App.tsx` renderiza `<Onboarding preview initialStep={onboardingPreview}`
Proof: `T4` imprime `C10 ok`

**C20** - O usuário de volta com o mic negado reabre o assistente com `steps={["microphone"]}` e `prepareModel={false}`: o efeito do modelo sai cedo com `!prepareModel`, o assistente renderizado não tem rodapé do modelo (`onboarding.footer.*`), nem contador de passos, nem `onboarding.nav.back`, e termina em `onboarding.nav.done`; o assistente completo mantém o rodapé
Proof: `T4` imprime `C20 ok`

**C21** - Um download ou seleção do assistente que falha sem o store gravar erro (`downloadModel`/`selectModel` devolvem `false`) leva o rodapé a `failed`: `footerError(null, true)` é `"failed"`, `footerError(e, _)` é `e`, `footerError(null, false)` é `null`; o `Onboarding.tsx` marca `setActionFailed(true)` quando a ação devolve `false`, passa `footerError(error, actionFailed)` ao rodapé, e "Tentar de novo" limpa a marca
Proof: `T4` imprime `C21 ok`

**C11** - Strings: `onboarding.progress`, `onboarding.nav.{back,next}`, `onboarding.footer.{pending,downloading,verifying,extracting,ready,failed}`, `onboarding.microphone.{title,description,test,stop,unavailable}`, `onboarding.microphone.denied.{title,step1,step2,step3,openSettings,waiting}`, `onboarding.shortcut.{title,instruction,testLabel,testPlaceholder}`, `onboarding.ai.{title,privacy,keyLabel,skip,finish}` e `settings.debug.onboardingPreview.{microphoneButton,shortcutButton,aiButton}` existem em pt e en; `onboarding.permissions`, `onboarding.subtitle`, `onboarding.existingModelsTitle`, `onboarding.downloadModelsTitle`, `onboarding.showAllModels`, `onboarding.showFewerModels`, `settings.debug.onboardingPreview.permissionsButton` e `modelsButton` não existem em nenhum dos dois
Proof: `T4` imprime `C11 ok`
Proof: `bun run check:translations` sai com 0

**C12** - Cor só semântica: nenhum arquivo de `src/components/onboarding/` cita `logo-primary`, `background-ui`, `mid-gray`, `emerald`, `bg-white/` nem um hex; o painel de acesso negado usa `warn`, o rodapé falho usa `danger`
Proof: `T4` imprime `C12 ok`

**C13** - Portões e provas anteriores continuam verdes: `bun run lint`, `bun run format:check`, `bun run check:translations`, `bunx tsc --noEmit`, `bun run build`, `scripts/check-brand.sh`, `T1`, `bun src/components/ui/controls.test.tsx`, `bun src/components/shell.test.tsx`, `bun src/overlay/pill.test.tsx` e `bun src/components/settings/history/historyModel.test.ts`
Proof: os onze comandos, cada um com exit 0

### S2 - PR 2, tray sem "Verificar atualizações" · 2 files · ~70 KB · ~18k

**C14** - `settings::UPDATER_ENABLED` é `false`, com um comentário que aponta `src/lib/updater.ts` e a ADR-0008
Proof: `cargo test -p fala --lib -- tray::tests::check_updates_item_hidden_while_updater_off` sai com 0

**C15** - `tray::check_updates_item_visible()` é `false` enquanto `UPDATER_ENABLED` é `false` (independente de `FALA_DISABLE_UPDATER`), e `build_menu` remove `check_updates_i` do menu quando ele é `false`, em vez de só testar `update_checks_forced_disabled()`
Proof: `cargo test -p fala --lib -- tray::tests::check_updates_item_hidden_while_updater_off` sai com 0
Proof: `grep -n "if !check_updates_item_visible()" apps/desktop/src/tray.rs` sai com 0

**C16** - O crate do desktop compila, os testes de `tray` passam e o PR 2 não toca `src/` nem `crates/`
Proof: `cargo check -p fala` sai com 0
Proof: `cargo test -p fala --lib -- tray` sai com 0
Proof: `git diff --exit-code feat/ui-onboarding...fix/tray-update-check -- src crates` sai com 0

### S3 - PR 3, ADR-0017 e `docs/design/ui.md` · 3 files · ~25 KB · ~7k

**C17** - `docs/decisions/0017-*.md` existe com `status: proposed` e `date: 2026-10-10`, as seções do template (`## Contexto e problema`, `## Opções consideradas`, `## Decisão`, `### Consequências`, `## Confirmação`), as três regras (abre no conteúdo; cor só semântica, sem cor de marca; tokens em `src/styles/theme.css` como fonte única) e cita `docs/design/ui.md`; nenhum arquivo de `docs/decisions/` passa a ter `status: accepted` neste PR além dos que já tinham
Proof: `bun src/styles/tokens.test.ts` imprime `C17 ok`

**C18** - `docs/design/ui.md` tem a tabela de cor com os 13 tokens com tema mais `rec` e `me`, e para cada token com tema o hex claro e o escuro são os mesmos de `--light-color-<t>`/`--dark-color-<t>` no `theme.css`; tem a rampa tipográfica (12/16, 14/20, 18/24, 20/28, 28/36), os raios 4 e 8 px, e o layout da janela (960×640, mínimo 720×520, rail 220/48 px)
Proof: `bun src/styles/tokens.test.ts` imprime `C18 ok`

**C19** - O PR 3 só muda `docs/` e o teste de tokens, e os portões continuam verdes
Proof: `git diff --name-only fix/tray-update-check...docs/adr-ui-identity` lista só `docs/decisions/0017-*.md`, `docs/design/ui.md` e `src/styles/tokens.test.ts`
Proof: `bun run format:check`, `bun run lint`, `scripts/check-brand.sh` e `T1` saem com 0

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| passos do assistente (3) | `microphone` C1, C2, C8 · `shortcut` C1, C6, C8 · `ai` C1, C7, C8 | - |
| estado de acesso ao mic (4) | denied C2 · allowed C2 · unknown C2 · unsupported C2 | - |
| ações do modelo (5 casos) | download C4 · select C4 · atual C4 · em curso C4 · ausente C4 | - |
| estados do rodapé (6) | `pending` C5 · `downloading` C5 · `verifying` C5 · `extracting` C5 · `ready` C5 · `failed` C5, C21 | - |
| entradas do assistente (3) | usuário novo C9 · de volta com mic negado C9, C20 · prévia C10 | - |
| saídas da sessão do medidor (5) | parar depois de ouvir C3 · parar com `getUserMedia` pendente C3 · segundo início antes do primeiro C3 · reinício com o mic trocado enquanto ouve C3 · início superado que falha C3 | - |
| itens da linha 5 da §3.10 (5) | microfone com medidor C2, C3 · atalho com teste real C6 · IA opcional C7 · download do Parakeet C4, C5 · cartões de modelo e passo do macOS fora C9 | - |
| regras da ADR (3) | abre no conteúdo C17 · cor só semântica C17 · tokens em fonte única C17, C18 | - |
| tokens com tema no `ui.md` (13) | C18, table-driven sobre os 13 | - |

## Swept

- validation: C7 - a chave é aparada e só gravada quando muda (comportamento do `LlmSettings` reaproveitado)
- failure modes: C2 (mic negado), C5 (`failed`), D2 (sem medidor)
- idempotency: C3 (clique duplo no teste), C4 - `modelAction` dá `none` com download em curso ou modelo atual; D5 (inits idempotentes)
- authorization: n/a - sem ação com permissão própria; o acesso ao mic é do SO (C2)
- concurrency: C4 - um download em curso não dispara outro
- data lifecycle: C3 - o stream do medidor é parado ao desmontar; D4 (`onboarding_completed`)
- dependency failure: C5 (`failed`), D2
- state transitions: C1, C8
- observability: n/a - sem log novo

## Out of scope

- Pill, Mica, barra de título própria, `crates/`, o feed por dia (#79).
- Captura de tela claro/escuro no Windows e o teste do medidor e do ditado no WebView2: fora desta máquina (Linux); "have to be taken on Windows before merge". Não entra no veredito.

## Handoff

- S1 = ~20k (App.tsx 13 KB, Onboarding 10 KB reescrito, 4 arquivos novos ~25 KB, locales ~2 KB de diff, OnboardingPreview 1 KB, o teste ~10 KB) - um builder
- S2 = ~18k (tray.rs ~30 KB lido, settings.rs só o trecho do updater) - acumulado ~38k
- S3 = ~7k (ADR, ui.md, o teste de tokens) - acumulado ~45k, sob o orçamento de 150k: um builder, sem pergunta
