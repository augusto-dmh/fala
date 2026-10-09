# llm-auto (F7a: ligação, limiar e lista)

## Problem

O ditado do atalho `transcribe` cola o texto do ASR sem formatação nenhuma. O LLM do Handy só roda pelo segundo atalho `transcribe_with_post_process`, que vem desligado (`post_process_enabled = false`), não tem Gemini, não tem limiar de palavras nem timeout e não sabe em que app o texto entra (`actions.rs:139-363`). Enquanto isso, `fala-postproc` já faz tudo o que a ADR-0004 pede (regras locais sempre, Gemini acima de 15 palavras, lista de apps desligados, espera de 2 s) e o desktop não o usa: só o `fala-cli format` chega nele. Quem paga é o uso diário no Windows: o Augusto dita com `ctrl+space` no `transcribe` (log de 2026-10-09) e recebe o texto cru.

Há um segundo detalhe: o histórico só pergunta o app em foco depois da colagem (`actions.rs:828`), tarde demais para o LLM, e decide quem editou por `post_process_requested`, que é falso no `transcribe`. Um texto que o LLM formatasse ficaria gravado como `Editor::Rules`, e o "desfazer edição da IA" (#46) não apareceria.

Quando isto entra, todo ditado do `transcribe` passa pelo `Postprocessor`: as regras locais sempre e o Gemini quando a pessoa tem a chave, o LLM está ligado, o app não está na lista e o texto tem mais de 15 palavras. O segundo atalho fica sem tecla por padrão (D6), e uma seção "IA" nos settings gerais liga e desliga o LLM, guarda a chave do Gemini e edita a lista de apps.

## Flow

Reusa `Postprocessor`, `LlmConfig`, `Gemini` (`fala-postproc`, exists), `foreground_app` (`fala-inject`, exists), o `KEY_VAULT` que hidrata `post_process_api_keys` do keyring (#35, exists) e `history_dictations::dictation_for` (#46, exists).

1. a pessoa solta o atalho `transcribe`: `TranscribeAction::stop` pergunta `fala_inject::foreground_app()` ali, antes de qualquer trabalho assíncrono, e guarda o `AppContext` para o LLM e para o histórico
2. o ASR devolve o texto; a conversão OpenCC herdada roda como hoje
3. `apps/desktop` monta o `Postprocessor` a partir dos settings (door 1): `llm_enabled`, `llm_disabled_apps` e a chave `post_process_api_keys["gemini"]`, que o `KEY_VAULT` leu do keyring (door 2)
4. `fala-postproc` aplica as regras e, se as quatro condições valem, pergunta ao Gemini com o nome do app; espera no máximo 2 s (`INSERT_DEADLINE`) numa thread de `spawn_blocking`, com o cancelamento herdado ainda valendo
5. passou dos 2 s ou o Gemini falhou: fica o texto das regras e a `LateEdit` é descartada (F7b a usa)
6. out: o desktop cola o `final_text` e grava o histórico com o `editor` que o `Postprocessor` decidiu (door 4) e o app do passo 1
7. o atalho `transcribe_with_post_process`, quando a pessoa ligou e deu uma tecla a ele, segue o caminho OpenAI-compat herdado, sem mudança

## Impact

| Front | What changes |
| --- | --- |
| domain | `AppSettings` ganha `llm_enabled` e `llm_disabled_apps`; os `post_process_*` ficam como estão, usados só pelo segundo atalho |
| domain | `post_process_providers` ganha o provedor `gemini` (endpoint OpenAI-compat do Google), para a chave ter id `gemini` no keyring, o mesmo do `fala-cli key set gemini` |
| domain | `EntryTexts` ganha `llm_produced: bool`; o histórico deixa de deduzi-lo de `post_process_requested` |
| domain | `NewEntry.app` era "o app em foco quando o texto foi entregue"; passa a ser "o app em foco quando a pessoa soltou o atalho" - quem ramifica nele: o histórico grava o valor, a UI do histórico mostra |
| domain | o retry do histórico (`retry_history_entry_transcription`) com `post_process_requested = false` passa pelo `Postprocessor`, com o app guardado no item; um item sem dictation (a primeira transcrição falhou, então o app nunca foi gravado) passa só pelas regras |
| stored data | `settings_store.json`: dois campos novos com default por serde; schema 2 → 3 com a migração do binding (door 3) |
| stored data | keyring: entrada nova `br.com.augusto.fala` / `gemini`, só quando a pessoa digita a chave |

## Relations

| Entity | Relation | Field | Note |
| --- | --- | --- | --- |
| `AppSettings.llm_disabled_apps` | compara com | `AppContext.app_name` | os dois no formato da door 1 do `active-app` (nome do executável em minúsculas, sem `.exe`) |
| `post_process_api_keys["gemini"]` | espelha | keyring `gemini` | o `KEY_VAULT` já faz o espelho para todo id de `post_process_providers` |

## Surface

| Surface | Change |
| --- | --- |
| comando Tauri `change_llm_enabled_setting(enabled: bool)` | novo |
| comando Tauri `change_llm_disabled_apps_setting(apps: Vec<String>)` | novo; normaliza cada item pela door 1 e devolve a lista gravada |
| UI: grupo "IA" em settings gerais | novo: ligar/desligar, chave do Gemini, lista de apps |
| atalho `transcribe_with_post_process` | sem tecla por padrão (D6) |

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. settings novos | `"llm_enabled": true` e `"llm_disabled_apps": ["1password", "bitwarden", "keepass", "keepassxc"]` no objeto `settings`, com default por serde para stores antigos; os `post_process_*` não mudam de nome nem de valor | reaproveitar `post_process_enabled` como "LLM ligado": hoje ele liga o segundo atalho e mostra a página herdada, e mudar o sentido de um campo já gravado confunde quem ligou um provedor OpenAI; apagar os `post_process_*`: o segundo atalho continua existindo (D6) e precisa deles |
| 2. onde mora a chave do Gemini | provedor `gemini` em `post_process_providers` (`base_url` `https://generativelanguage.googleapis.com/v1beta/openai`, `supports_structured_output: true`), chave em `post_process_api_keys["gemini"]`, que o `KEY_VAULT` grava no keyring como account `gemini` | um campo `gemini_api_key` à parte: exigiria um segundo caminho de keyring no desktop; ler o keyring direto em `actions.rs`: duplicaria o `KEY_VAULT` e a sua migração |
| 3. o segundo atalho (D6) | fresh install: `transcribe_with_post_process` com `default_binding = ""` e `current_binding = ""`. Store com schema < 3: `default_binding` vira `""`; `current_binding` vira `""` só se é igual ao default antigo da plataforma (`ctrl+space` no Windows e Linux), senão fica o que a pessoa escolheu; `transcribe` nunca é tocado. Binding vazio não é registrado | remover o binding: contraria a D6 ("o código fica"); mover o default para outra tecla: o pitch tem um gesto só |
| 4. quem editou | `EntryTexts.llm_produced` vem do `Formatted.dictation.editor == Editor::Llm` no caminho novo e de `post_process && post_processed_text.is_some()` no herdado | manter a dedução por `post_process_requested`: grava `Rules` para texto do LLM no `transcribe` |

- O retry de um item do `transcribe` reformata pelo `Postprocessor` atual, então pode chamar o Gemini de novo. É reversível (outro retry) e não grava nada novo.

## Criteria

### S1: o ditado passa pelo `Postprocessor` (P1)

**Acceptance Criteria**

1. WHEN o `Postprocessor` do desktop devolve um ditado editado pelo LLM THEN `apps/desktop` SHALL colar o `final_text` dele e gravar no histórico `post_processed_text = Some(final_text)` e um dictation com `editor = Llm`
2. WHEN o texto depois das regras tem 15 palavras ou menos, ou o LLM está desligado, ou não há chave do Gemini THEN `apps/desktop` SHALL colar o texto das regras sem nenhuma request HTTP
3. WHILE o app em foco está em `llm_disabled_apps` (sem diferenciar caixa) THEN `apps/desktop` SHALL colar o texto das regras sem nenhuma request HTTP
4. WHEN o app em foco é desconhecido (`app_name = None`) e as outras condições valem THEN `apps/desktop` SHALL chamar o LLM (D4), com o corpo sem o elemento `<app>…</app>`
5. IF o Gemini responde depois de 2 s, com HTTP 500 ou com corpo inválido THEN `apps/desktop` SHALL colar o texto das regras e gravar `post_processed_text = Some(texto das regras)` com `editor = Rules` quando o texto mudou
6. The `apps/desktop` SHALL perguntar o app em foco uma vez por ditado, ao soltar o atalho, e usar o mesmo `AppContext` no LLM e no histórico
7. The chave do Gemini SHALL não aparecer em nenhuma linha de log do desktop
8. WHEN o retry de um item do `transcribe` não tem o app gravado (item sem dictation) THEN `apps/desktop` SHALL formatar só com as regras, sem chamar o LLM; com o app gravado, SHALL usar esse app (achado do Verifier: sem isso, um ditado no `keepassxc` cujo ASR falhou iria ao Gemini no retry)

**Independent test:** `cargo test -p fala llm_auto`

### S2: settings e o segundo atalho (P1)

**Acceptance Criteria**

9. WHEN um store sem `llm_enabled` e `llm_disabled_apps` é lido THEN `apps/desktop` SHALL usar `true` e a lista padrão da door 1
10. WHEN um store com schema 2 tem `transcribe_with_post_process` no default antigo THEN a migração SHALL deixá-lo com `current_binding = ""` e `default_binding = ""`, e o `transcribe` com o valor que tinha
11. WHEN um store com schema 2 tem `transcribe_with_post_process` numa tecla escolhida pela pessoa THEN a migração SHALL manter essa tecla e só zerar o `default_binding`
12. WHEN `change_llm_disabled_apps_setting` recebe `[" Chrome.exe ", "chrome", "", "C:\\Tools\\Slack.exe"]` THEN `apps/desktop` SHALL gravar `["chrome", "slack"]`
13. The registro de atalhos SHALL pular um binding com `current_binding` vazio, nas duas implementações (Tauri e `fala_keys`)
14. The `post_process_providers` padrão SHALL ter o provedor `gemini`, e um store antigo SHALL ganhá-lo na leitura

### S3: UI (P2)

15. The settings gerais SHALL mostrar o grupo "IA" com o interruptor, o campo da chave do Gemini e a lista de apps, com todas as strings em `pt` e `en`

**Independent test:** `bun run lint`, `bun run check:translations`; visual em `bun run tauri dev` (`TODO(windows)`, manual)

## Out of scope

| Excluded | Why |
| --- | --- |
| colar o bruto aos 2 s e "aplicar edição" com a `LateEdit` | F7b (`feat/llm-late-edit`) |
| dicionário pessoal no prompt | F8; o desktop passa `Dictionary::default()` |
| gravar o histórico antes do LLM (design doc §6) | muda a ordem colar/gravar que o `cancel-anywhere` fixou; entra com a F7b, que já reescreve esse trecho |
| latência soltar → texto com LLM ≤ 1,2 s e W13 | só no Windows, na tomada: `TODO(windows)` no `checks.md` |
| tirar a página herdada de pós-processamento | o segundo atalho continua (D6) |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| LLM ligado por padrão | `llm_enabled = true`; sem chave nada sai da máquina | ADR-0004 decide "Gemini automático"; digitar a chave é a ação explícita | y — delegado (2026-10-02), decidido pelo executor |
| lista padrão | os quatro gerenciadores de senha da door 1 | ADR-0004 fala em "apps sensíveis"; ditar em gerenciador de senha não tem caso de uso e a lista mostra o formato | y — delegado (2026-10-02), decidido pelo executor |
| quando perguntar o app | ao soltar o atalho, no `stop` | é a janela que recebe o Ctrl+V; a pill não rouba foco (spike 03); a conferência antes do Ctrl+V é do `inject-c` | y — delegado (2026-10-02), decidido pelo executor |
| idioma das regras | `language_from_setting(selected_language)`, o mesmo do histórico | um idioma por ditado já vem do F4 | y — delegado (2026-10-02), decidido pelo executor |

**Open questions:** none - all resolved or logged above.

## Observable

- `debug`: `LLM não usado: <motivo>` (já existe no crate) e `post-processing: editor=<Llm|Rules|None>` no desktop. Nenhum texto ditado nem chave.

## Sources

- `fala-research/plans/fase-1-delta-e-semanas-3-4.md` § F7, W5, W6, W8 e §3 D4, D5, D6
- `fala-research/decisions-log.md`, 2026-10-09 (D4, D5, D6)
- `docs/decisions/0004-*` (só texto, app e dicionário; automático; desligado em apps sensíveis) e `0012-*`
