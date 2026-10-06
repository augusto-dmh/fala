# shortcut-gestures — segurar fala, dois toques travam, toque curto some

## Problem

O gesto de ditado herdado do Handy é o oposto do que o pitch da fase 1 pede. Hoje o modo padrão é
`HoldOrToggle` (`settings.rs:162-174`): um toque curto, abaixo de `hold_threshold_ms` = 300 ms
(`settings.rs:529-531`), *trava* a gravação (`transcription_coordinator.rs:437-459`) até o próximo
toque. Um esbarrão no atalho deixa o microfone aberto gravando até alguém perceber. O pitch quer o
contrário: "segurar = push-to-talk; double-tap em 0,5 s = hands-free; toque curto descartado; Esc
cancela" (`fala-research/pitches/fase-1-ditado-windows.md`, semanas 3-4; delta W1 e W3).

O atalho também está trocado. O pitch descreve o gesto do Wispr Flow que o Augusto usa hoje:
"Ctrl+Shift+Space, falo, solto, texto pronto". No Handy, `ctrl+shift+space` é o binding de
*pós-processamento* `transcribe_with_post_process` (`settings.rs:878-897`) e o ditado é
`ctrl+space` (`settings.rs:858-877`). Quem vem do Wispr erra o atalho no primeiro dia.

O custo é o critério de sucesso da fase 1: "duas semanas usando o Fala em vez do Wispr". O gesto
errado é a primeira coisa que a pessoa sente. A fonte não traz números de incidência; o delta só
registra a contradição (item 2 do §0).

Quando isto entrar, numa instalação nova, segurar `Ctrl+Shift+Space` grava enquanto a tecla está
baixa e processa ao soltar. Dois toques em até 0,5 s gravam sem segurar até o próximo toque. Um
toque curto isolado não deixa rastro: nada é transcrito, colado ou guardado no histórico, e não
toca som de fim. Esc cancela a gravação, inclusive a sem as mãos.

## Flow

Reusa a máquina pura `CoordinatorState` (`transcription_coordinator.rs`), com debounce, carência
de 50 ms contra o auto-repeat do X11 e fila de toque durante o processamento, e reusa o teardown de
`utils::cancel_current_operation` para o descarte. Não cria uma segunda máquina nem um segundo
caminho de cancelamento.

1. tecla do binding `transcribe` -> `shortcut::handler::handle_shortcut_event` (exists) - repassa press/release com `settings.shortcut_activation` e `hold_threshold_ms` ao `TranscriptionCoordinator` (exists)
2. `CoordinatorState::on_input` (exists) - no modo `push_to_talk_double_tap` (door 1): key-down em Idle emite `Start`; key-up passa pela carência de 50 ms e é classificado por `finish_hold` (exists) em segurar (`Stop`) ou toque (abre a janela de dois toques até key-down + 500 ms); segundo key-down na janela trava a sessão; key-down com a sessão travada emite `Stop`
3. laço do coordenador (exists) - `recv_timeout` passa a dormir até o prazo mais próximo (carência ou janela); janela vencida sem segundo toque emite o efeito novo `Discard`
4. `run_effect` (exists) - `Start`/`Stop` seguem para `TranscribeAction` (exists, sem mudança); `Discard` chama o teardown de cancelamento em `utils` (exists, dividido para não notificar o próprio coordenador)
5. out: com `Stop`, o pipeline herdado transcreve e cola; com `Discard`, gravação cancelada, overlay escondido, tray em Idle, sem transcrição, colagem, histórico ou som de fim
6. `settings::get_default_settings` (exists) - instalação nova nasce com `push_to_talk_double_tap` e `transcribe` = `ctrl+shift+space` (door 2)
7. tela Geral -> `ShortcutActivationSetting` (exists, front) - lista o modo novo com i18n pt/en e grava via `changeShortcutActivationSetting` (exists)

## Impact

| Front | What changes |
| --- | --- |
| domain | termo novo: `push_to_talk_double_tap` - modo de ativação: segurar = PTT, dois toques em 500 ms = travado, toque isolado = descarte. Vive em `settings::ShortcutActivation` e no tipo TS gerado |
| domain | termo novo: `Discard` - efeito do coordenador que encerra a gravação sem processar, por fim de janela de dois toques. Só `run_effect` o consome |
| domain | termo existente: `ShortcutActivation::default()` era `HoldOrToggle` e passa a ser `PushToTalkDoubleTap`. Quem ramifica nisso hoje: `get_default_settings` e o `#[serde(default)]` de `AppSettings` (store sem a chave). `HoldOrToggle`, `PushToTalk` e `Toggle` continuam com a semântica de antes |
| domain | binding padrão: `transcribe` vai de `ctrl+space` para `ctrl+shift+space` e `transcribe_with_post_process` faz o caminho inverso, em Windows e Linux (no macOS, `option+...` com a mesma troca). O binding de pós-processamento só é registrado com `post_process_enabled = true` (`shortcut/tauri_impl.rs:27`, `shortcut/fala_keys.rs:437`), que vem `false` |
| stored data | nada a migrar. Stores existentes guardam `shortcut_activation` e `bindings` explícitos (o store serializa tudo) e continuam com o que têm. Só instalação nova, ou store sem a chave, recebe o modo e o atalho novos (door 1, door 2) |
| ui | o seletor "Comportamento do atalho" ganha uma quarta opção, a primeira da lista, e o fallback de exibição passa a ser o modo novo |
| generated | `src/bindings.ts` ganha o membro `"push_to_talk_double_tap"` no tipo `ShortcutActivation`, regenerado pelo `tauri-specta` (não editado à mão) |

## Relations

`None - no stored-data shape change` (o campo `shortcut_activation` já existe; ganha um valor
possível, registrado no door 1).

## Surface

`None - nothing consumed outside`. O comando `change_shortcut_activation_setting` mantém a
assinatura; o valor novo do enum é o door 1. O store e o tipo TS são consumidos só por este repo.

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. valor novo do enum persistido em `settings_store.json` e no tipo TS gerado | `ShortcutActivation::PushToTalkDoubleTap`, serializado `"push_to_talk_double_tap"` (`#[serde(rename_all = "snake_case")]`), marcado `#[default]`; `Toggle`, `PushToTalk` e `HoldOrToggle` ficam como estão | reescrever `HoldOrToggle` com a semântica nova (D2 b do delta): muda o comportamento de quem escolheu esse modo de propósito, e o teste herdado `shortcut_activation_migration_respects_explicit_new_key` fixa que um `"hold_or_toggle"` explícito sobrevive. Migrar stores `"hold_or_toggle"` para o modo novo: o store serializa todos os campos, então não há como distinguir "nunca mexeu" de "escolheu", e o mesmo teste herdado proíbe |
| 2. atalhos padrão de instalação nova | `transcribe.default_binding = current_binding = "ctrl+shift+space"` e `transcribe_with_post_process` = `"ctrl+space"` em Windows e Linux; `option+shift+space`/`option+space` no macOS; `alt+shift+space`/`alt+space` no resto. Stores existentes não são migrados | deixar o pós-processamento sem tecla (D6 b do delta): `change_binding` rejeita binding vazio ("every shortcut should have a value", `shortcut/mod.rs:118-121`) e `validate_shortcut` também (`shortcut/tauri_impl.rs:44-46`), então "sem tecla" exige caminho novo no registro e na UI. Remover o binding (D6 a): apaga código herdado que a F7 ainda vai decidir |

- Nothing else in this change is hard to reverse: a janela de 500 ms é uma constante em memória, o efeito `Discard` é interno ao desktop, a divisão de `cancel_current_operation` é refatoração local, e as chaves de i18n novas são lidas só por este frontend.

## Criteria

### S1: o gesto no coordenador (P1)

Com o modo `push_to_talk_double_tap`, a sequência de press/release e o relógio decidem, sem teclado
real, entre gravar segurando, travar com dois toques e descartar o toque isolado.

**Acceptance Criteria**

1. WHILE `shortcut_activation` is `push_to_talk_double_tap`, WHEN the dictation key goes down with the coordinator idle THEN the coordinator SHALL emit `Start` on that key-down, without waiting for a second tap
2. WHEN the key comes up at least `hold_threshold_ms` (300 ms by default) after key-down THEN the coordinator SHALL emit `Stop` when the 50 ms release grace elapses
3. WHEN the key comes up less than 300 ms after key-down THEN the coordinator SHALL emit nothing and keep recording, with its next wake-up deadline at key-down + 500 ms
4. WHEN a second key-down arrives at most 500 ms after the first key-down THEN the coordinator SHALL lock the recording (hands-free) and emit nothing
5. WHILE the recording is locked, WHEN the key comes up THEN the coordinator SHALL emit nothing and defer no release
6. WHILE the recording is locked, WHEN the key goes down again THEN the coordinator SHALL emit `Stop`
7. IF the 500 ms window after a short tap ends with no second key-down THEN the coordinator SHALL emit `Discard` and return to idle, never `Stop`
8. IF the second key-down arrives more than 500 ms after the first key-down and before the window deadline fired THEN the coordinator SHALL emit `Discard` instead of locking
9. WHILE the window after a short tap is open, WHEN a key-up arrives THEN the coordinator SHALL emit nothing
10. WHILE the pipeline is processing a previous dictation, WHEN a short tap (press and release under 300 ms) arrives THEN the coordinator SHALL start nothing when the pipeline drains
11. WHEN the key is held for at least 300 ms under X11 auto-repeat (synthetic release/press pairs 5 ms apart) THEN the coordinator SHALL emit exactly one `Start` and one `Stop`
12. WHEN a cancel arrives during a locked recording or during the open window THEN the coordinator SHALL return to idle and the next key-down SHALL emit `Start`
13. WHILE both a release grace and a tap window are pending the coordinator SHALL wake at the earlier of the two deadlines

**Independent test:** `cargo test -p fala --lib transcription_coordinator` com relógio sintético.

### S2: o descarte não deixa rastro (P1)

`Discard` encerra a gravação pelo mesmo teardown do cancelamento, sem processar.

**Acceptance Criteria**

14. WHEN the coordinator emits `Discard` THEN the desktop SHALL cancel the recording, hide the overlay and set the tray to idle through the teardown shared with `cancel_current_operation`, with no transcription, paste, history entry or stop sound
15. WHEN the desktop executes `Discard` THEN it SHALL NOT send a `Cancel` command back to the coordinator (the coordinator already moved to idle)

**Independent test:** no Windows, um toque curto em `Ctrl+Shift+Space` com o som de feedback ligado: pill aparece e some, nada é colado, histórico sem entrada nova.

### S3: padrões de instalação nova (P1)

**Acceptance Criteria**

16. The settings SHALL default `shortcut_activation` to `push_to_talk_double_tap`, both in `get_default_settings()` and when the stored object lacks the key
17. The settings SHALL serialize `ShortcutActivation::PushToTalkDoubleTap` as `"push_to_talk_double_tap"` and parse it back to the same variant
18. The default settings on Windows and Linux SHALL bind `transcribe` to `ctrl+shift+space` and `transcribe_with_post_process` to `ctrl+space`, in both `default_binding` and `current_binding`
19. WHEN a stored settings object holds `"hold_or_toggle"`, `"push_to_talk"` or `"toggle"` THEN loading with migrations SHALL keep that value

**Independent test:** `cargo test -p fala --lib settings`.

### S4: o seletor do modo (P2)

**Acceptance Criteria**

20. The "Comportamento do atalho" dropdown SHALL list `push_to_talk_double_tap` as its first option, labelled "Segurar ou dois toques" in pt and "Hold or double-tap" in en, each with a description key present in both locales
21. WHEN no `shortcut_activation` setting has loaded THEN the dropdown SHALL show `push_to_talk_double_tap` as selected

**Independent test:** `bun run check:translations`, `bun run lint`, `bunx tsc --noEmit`; visual no `tauri dev` da sessão Windows.

## Out of scope

| Excluded | Why |
| --- | --- |
| Esc cancelar durante a transcrição e o LLM | é a 1.F2 `cancel-anywhere` (W2); aqui o Esc só cancela enquanto grava, como hoje |
| Som de fim condicionado a haver fala num PTT longo e mudo | o toque curto isolado já não toca som (S2); um PTT ≥ 300 ms sem fala segue o caminho herdado |
| Migrar stores existentes para o modo e o atalho novos | sem como distinguir "padrão nunca tocado" de "escolha" (door 1); o Augusto troca uma vez na tela |
| Dois toques durante o processamento travarem a próxima gravação | com o pipeline ocupado o modo novo se comporta como push-to-talk (AC 10); o processamento dura ~1 s |
| Limite de sessão no hands-free (aviso aos 19 min) | é a 1.F3 `session-limit` |
| Hook físico e modo manual do spike 02 | sessão Windows |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Toque curto com fala | descarta sempre, sem consultar o VAD | um toque < 300 ms mais a janela guarda no máximo ~500 ms de áudio, que não é um ditado; o VAD como juiz (D2 do delta) dependeria de `vad_enabled = true` e de reordenar o som de fim herdado (`actions.rs:669`) para depois de `stop_recording` | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| De onde contam os 0,5 s | do primeiro key-down ao segundo key-down | é a leitura literal de "double-tap em 0,5 s" e mantém um único prazo; contar do key-up daria até 800 ms | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| A gravação travada é a mesma que o primeiro toque abriu | sim, continua a sessão iniciada no key-down | o orçamento de 100 ms até a pill não permite esperar o segundo toque para abrir o microfone (D2 do delta) | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| Destino do binding de pós-processamento | troca com o ditado (`ctrl+space`), em vez de ficar sem tecla | ver door 2; com `post_process_enabled = false` ele nem é registrado, então nada colide no padrão | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| Store existente do Augusto no Windows | sem migração; escolher "Segurar ou dois toques" e regravar o atalho uma vez | ver door 1; vira item do checklist `TODO(windows)` | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| `hold_threshold_ms` vale para o modo novo | sim, o mesmo campo (padrão 300, ajustável na aba Debug) | evita um segundo limiar; a janela de 500 ms fica constante | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |

**Open questions:** none - all resolved or logged above.

## Observable

| Surface | Decision | Landing |
| --- | --- | --- |
| gesto no atalho (entrada física) | segurar, soltar, dois toques, toque isolado, Esc | AC 1-12 |
| gesto no atalho | o que a pessoa vê e ouve no toque isolado | AC 14 (pill some, sem som de fim, nada colado) |
| gesto no atalho | atalho padrão | AC 18 |
| tela Geral · "Comportamento do atalho" | estado vazio (setting ainda não carregou) | AC 21 |
| tela Geral · "Comportamento do atalho" | carregando / salvando | existing - `disabled={isUpdating("shortcut_activation")}` no `Dropdown` |
| tela Geral · "Comportamento do atalho" | erro ao salvar | existing - `useSettings`/`settingsStore` herdados tratam o `Result` do comando |
| tela Geral · "Comportamento do atalho" | ordem e textos das opções | AC 20 |
| tela Geral · "Comportamento do atalho" | ação destrutiva | n/a - trocar o modo não apaga nada e se desfaz na mesma tela |
| pill (overlay) | estados novos | n/a - a pill mostra gravando/processando como hoje; o descarte usa o esconder do cancelamento |

## Sources

- `fala-research/pitches/fase-1-ditado-windows.md` - o gesto (Ctrl+Shift+Space, double-tap em 0,5 s, toque curto descartado, Esc cancela)
- `fala-research/plans/fase-1-delta-e-semanas-3-4.md` - itens S1, W1, W3, feature F1 e decisões D2 e D6
