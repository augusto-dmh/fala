# shortcuts-dialog

## Problem

Cada ação aceita uma combinação só (`ShortcutBinding.current_binding`), e nada impede duas ações na mesma tecla no Windows (`fala_keys::validate_shortcut` só confere a sintaxe), nem Ctrl+C, Alt+Tab ou Win+H como atalho. Não há "restaurar padrão" para todos, nem atalho de reunião. Fatia B de `shortcuts-last-text`: PR 1 é o backend, PR 2 o diálogo.

## Flow (PR 1)

Reusa `register_shortcut`/`unregister_shortcut`, `handle_shortcut_event`, `ACTION_MAP`, `MeetingManager::start_from_tray`/`stop`, `TranscriptionCoordinator::is_busy` (exists).

1. `ShortcutBinding.alternate_binding` (`#[serde(default)]`, vazio = nenhuma); `meeting_toggle` entra nos padrões sem tecla e um store antigo o recebe pelo `add_missing_bindings`
2. new `shortcut::registrations(binding)`: a 1ª com o id da ação, a 2ª com `<id>#2`; todo laço que registra ou desregistra passa por ele (init dos dois backends, suspend/resume, troca de backend, pós-processamento)
3. `handler::handle_shortcut_event` reduz `<id>#2` a `<id>` (new `action_id`) antes de decidir: coordenador, `cancel`, `ACTION_MAP`
4. new `shortcut/conflict.rs::check(combo, action, slot, bindings)`: tecla sozinha → `LoneKey`; reservada → `Reserved`; já usada → `InUse { action, slot, take_over }`; contém ou está contida (só modificadores) → aviso `Overlap`
5. new comando `set_binding(id, slot, binding, take_over)`; `change_binding` vira `set_binding(primary, false)`. Com `take_over`, a outra ação perde aquela combinação (a 2ª sobe para 1ª) na mesma operação; recusado se deixaria Ditar ou Cancelar sem tecla
6. new comando `remove_binding(id, slot)`: Cancelar nunca; Ditar só se sobra a outra
7. new comando `reset_all_bindings`: recusa com ditado em curso ou reunião gravando/pausada; senão 1ª = padrão, 2ª = vazia em todas, registro no fim, desfeito se um registro falha
8. new `MeetingToggleAction`: sem reunião → `start_from_tray`; gravando ou pausada → `stop`; parando → nada

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. campo no store | `alternate_binding: String` | `bindings: Vec<String>`: migração do store e de toda leitura de `current_binding` (pill, onboarding) |
| 2. id da ação nova | `meeting_toggle` | `meeting_start_stop`: mais longo, sem ganho |

- Reversíveis: id interno `<id>#2` (só no registro), a lista de reservados, o conjunto de teclas que não podem ficar sozinhas.

## Criteria (PR 1)

1. WHEN um store sem `alternate_binding` nem `meeting_toggle` é carregado THEN SHALL carregar com 2ª vazia e `meeting_toggle` sem tecla
2. WHEN um evento chega com `<id>#2` THEN o handler SHALL decidir como `<id>`
3. `conflict::check` SHALL recusar: mesma combinação em outra ação ou no outro lugar (sem ordem, `ctrl` × `ctrl_left`), Ctrl+C e Win+H, letra sozinha, Esc sozinho fora de Cancelar; SHALL aceitar F9 sozinho, Esc sozinho em Cancelar, só modificadores; SHALL avisar sobreposição
4. WHEN `take_over` THEN a outra ação SHALL perder a combinação (a 2ª sobe); WHEN isso deixaria Ditar ou Cancelar sem tecla THEN SHALL recusar
5. `reset_all_bindings` SHALL recusar com ditado e com reunião gravando/pausada, e restaurar 1ª = padrão, 2ª = vazia
6. `meeting_toggle` SHALL iniciar pela bandeja sem reunião e parar com reunião gravando ou pausada

## Out of scope

Mouse, "Pressionar Enter", máscara da Win, mais de duas, atalho na bandeja (definição). A UI é a PR 2.
