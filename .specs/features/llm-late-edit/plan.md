# llm-late-edit — a resposta tardia do Gemini vira "Aplicar edição da IA"

Profile: light (o que o `AGENTS.md` declara)

## Problem

Desde #53, #54 e #55, soltar `ctrl+shift+space` formata o ditado por `fala_postproc::Postprocessor`
(`apps/desktop/src/llm_auto.rs`): regras sempre, Gemini quando o LLM está ligado, há chave `gemini`, o app não
está em `llm_disabled_apps` e o texto passa de 15 palavras. Se o Gemini passa de 2 s, o texto das regras é
colado e salvo; mas `llm_auto::format` descarta a `Formatted.late_edit`, então a resposta que chega depois se
perde. O item do histórico também é gravado sempre por `Store::add`: um ditado num gerenciador de senha da lista
fica com `sensitive = false` em `fala.sqlite`.

A ADR-0004 manda, quando a resposta passa de 2 s, inserir o texto sem o LLM e oferecer "aplicar edição". Falta
guardar a resposta tardia no item (`Store::apply_late_edit`, door 3), avisar a tela do histórico e trocar o
rótulo do botão de "Reaplicar edição da IA" para "Aplicar edição da IA", que serve tanto depois de desfazer
quanto depois de uma edição tardia; e marcar como sensível o que veio de um app da lista. Quem paga é quem dita
numa rede lenta: a formatação que o Gemini fez chega e ninguém a vê. A fonte não traz número de incidência.

## Flow

Reusa `fala_postproc::LateEdit` (prazo tardio de 10 s, door 6 da postproc), o `llm_auto` e o stop path de main
(`process_transcription_output` com `OutputMode::Auto`, `deliver_unless_cancelled`, `HistoryManager::save_entry`)
e `Store::redo` como "aplicar"; não muda o que é colado nem quando.

5. `deliver_unless_cancelled` (exists) - cola o texto das regras e salva por `HistoryManager::save_entry` (exists); o `NewEntry` ganha `sensitive` (new), verdadeiro quando o `AppContext` lido ao soltar a tecla está em `llm_disabled_apps` (`llm_auto::is_disabled_app`, new), e o `HistoryManager` grava por `Store::add_sensitive` nesse caso, senão `Store::add` (exist)
6. com `ProcessedTranscription.late_edit` (new, vindo de `llm_auto::format_with_late_edit`, new) e o item salvo → `spawn_blocking(llm_auto::finish_late_edit)` (new) → `LateEdit::wait` (exists) → `Ok(texto)` → `HistoryManager::apply_late_edit` (new) → `Store::apply_late_edit` (door 3) e `HistoryManager::announce_updated` (new), evento `history-update-payload` `Updated` (exists); `Err`, prazo vencido ou item apagado → `debug!` e nada muda
7. out: o item em `fala.sqlite` passa a `final = resposta`, `editor = llm`, `showing = raw`; a tela do histórico (exists) mostra o texto, e para `editor = llm` com `showing = raw` o botão "Aplicar edição da IA" faz `Store::redo` (exists) e copia o texto aplicado

## Impact

| Front | What changes |
| --- | --- |
| domain | termo existente: `showing = raw` com `editor = llm` significava "a pessoa desfez a edição"; passa a significar também "a edição da IA chegou tarde e ainda não foi aplicada". Quem ramifica: `historyModel.editAction` (o rótulo do botão muda de "Reaplicar" para "Aplicar"), `Store::set_showing` |
| behaviour | `sensitive` em `fala.sqlite` é gravado pela primeira vez (hoje sempre `false`) para ditados em apps de `llm_disabled_apps`; nada filtra por ele ainda (MCP é a ADR-0010, `proposed`) |
| behaviour | uma resposta do Gemini entre 2 s e 10 s depois do pedido deixa de ser descartada no caminho `transcribe`; o retry do histórico continua a descartá-la |
| crate | `fala-storage` ganha `Store::apply_late_edit` (door 3) e a variante `StorageError::EmptyEdit`; schema 1 não muda |
| stored data | nenhuma forma nova: `apply_late_edit` reescreve `final`, `edited_by` e `showing` de uma linha existente e o `.md` dela |

## Relations

None - no stored-data shape change: `fala.sqlite` continua no schema 1; `history.db` fica na `user_version` 5.

## Surface

| Route | In | Out | Status |
| --- | --- | --- | --- |
| `Store::apply_late_edit(id, text)` (API pública de `fala-storage`, consumida pelo desktop e disponível à CLI) | `id`, `text` | `DictationRecord` com `editor = llm`, `showing = raw` | `Ok`, `NotFound`, `EmptyEdit` (texto em branco), `Mirror`, `Database` · sem status HTTP (biblioteca; 200-599 n/a) |

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. `Store::apply_late_edit(&self, id: &str, text: &str) -> Result<DictationRecord, StorageError>` em `fala-storage` | `UPDATE dictations SET final = ?, edited_by = 'llm', showing = 'raw' WHERE id = ?`, reescreve o `.md` do item; id desconhecido devolve `NotFound` sem mudar nada; texto vazio ou em branco devolve `EmptyEdit` sem mudar nada | `delete` + `add`: muda o id e quebra o vínculo `dictation_id` de `history.db`; guardar o texto tardio só em `history.db`: `fala-cli history`, o espelho `.md` e o MCP nunca o veriam, e o "aplicar" não teria `Store::redo` para reusar |

- Nothing else in this change is hard to reverse (onde o `spawn_blocking` mora, o nome dos helpers e o texto do rótulo mudam num commit)

## Criteria

### S1: o histórico sabe o que é sensível (P1)

**Acceptance Criteria**

1. WHEN a dictation is saved and its `app_name` is in `llm_disabled_apps` (compared without case) THEN the dictation SHALL be added with `Store::add_sensitive` (`sensitive = true`); otherwise with `Store::add` (`sensitive = false`)
2. The history screen SHALL offer "Desfazer edição da IA" only for entries whose dictation has `editor = llm` and `showing = final` (existing behaviour, kept)

**Independent test:** `cargo test -p fala --lib managers::history_dictations::tests::disabled_app_marks_sensitive`; `bun src/components/settings/history/historyModel.test.ts`.

### S2: a resposta que chega tarde vira "Aplicar edição da IA" (P1)

Quando o Gemini passa de 2 s, o texto das regras já foi colado; a resposta que chegar até 10 s fica guardada no
item e a pessoa escolhe aplicá-la.

**Acceptance Criteria**

3. WHEN `Store::apply_late_edit(id, text)` is called for an existing dictation with non-empty `text` THEN the store SHALL set `final_text = text`, `editor = Llm` and `showing = Raw`, keep `raw`, `app`, `created_at` and `sensitive`, rewrite the item's `.md`, and return the updated record
4. IF `Store::apply_late_edit` is called with an unknown id THEN the store SHALL return `NotFound` and change nothing; IF `text` is empty or blank THEN the store SHALL return an error and change nothing
5. WHEN the Gemini answer arrives after the paste and before the 10 s late deadline for a saved dictation THEN the desktop SHALL call `apply_late_edit` with it and emit `history-update-payload` `Updated` with the refreshed entry, leaving the pasted text and `history.db` untouched
6. IF the late answer is an error, or arrives after the late deadline, or the entry was deleted meanwhile THEN the desktop SHALL change nothing and log at `debug` only
7. WHERE an entry's dictation has `editor = llm` and `showing = raw` the history screen SHALL offer "Aplicar edição da IA" (en "Apply AI edit"), which runs the existing redo command and copies the applied text with the existing toast "Texto editado copiado"
8. WHEN the user undoes an applied late edit THEN the entry SHALL go back to `showing = raw` and offer "Aplicar edição da IA" again (the existing undo path)
9. The front SHALL give every new visible string a key in both `src/i18n/locales/pt` and `src/i18n/locales/en`, and the replaced key `settings.history.redoAiEdit` SHALL be gone from both

**Independent test:** `cargo test -p fala-storage --test store apply_late_edit`; `cargo test -p fala --lib llm_auto::tests::late_answer` com o Gemini falso de `llm_auto.rs` respondendo depois de 2 s; `bun src/components/settings/history/historyModel.test.ts`.

## Out of scope

| Excluded | Why |
| --- | --- |
| Gravar o bruto antes do LLM (design doc §6) | contradiz a regra "cancelado não deixa histórico" da cancel-anywhere; a resposta tardia já é coberta pela door 1 |
| Aplicar a resposta tardia no retry do histórico | o retry é do áudio guardado, fora do gesto; ligá-lo à edição tardia é outra feature |
| Dicionário pessoal no prompt, latência com LLM, página herdada de pós-processamento | fora desde a `llm-auto` de main (F8, `TODO(windows)`, D6) |
| MCP, sync e destinos filtrarem `sensitive` | ADR-0010 e 0011 `proposed`; esta feature só grava a marca |
| Um aviso visual de "edição da IA disponível" além do botão | o botão já muda; um badge é decisão de design que a fase 1 não pede |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| 1. Rótulo "Aplicar edição da IA" substitui "Reaplicar edição da IA" nos dois casos (`showing = raw` após desfazer e após edição tardia) | um rótulo só | não há marca no item que distinga os dois estados, e "aplicar" vale para ambos | y — delegado (Augusto, 2026-10-09) |
| 2. Ditado com WAV que falhou ao salvar | não grava em nenhum banco, como hoje; a `LateEdit` é descartada | sem item não há onde aplicar | y — delegado (Augusto, 2026-10-09) |
| 3. `sensitive` também no atalho herdado | a marca segue o app, não o atalho: um ditado num app da lista é sensível pelos dois atalhos | o que torna o texto sensível é o app onde ele entrou | y — delegado (Augusto, 2026-10-09), decidido pelo executor |
| 4. Depois de uma edição tardia o item mostra o bruto do ASR (`showing = raw`) até a pessoa aplicar | consequência da door 1 | o `history.db` continua com o texto das regras em `post_processed_text`, e o tray copia o que foi colado | y — delegado (Augusto, 2026-10-09), decidido pelo executor |

**Open questions:** none - all resolved or logged above.

## Observable

| Surface | Decision | Landing |
| --- | --- | --- |
| screen histórico | botão de aplicar a edição tardia | AC 7, AC 8 |
| screen histórico | erro ao aplicar | existing - toast `settings.history.editToggleError` da history-undo |
| screen histórico | texto visível novo | AC 9 |
| screen histórico | carregando, vazio, não autorizado, ação destrutiva | n/a - nenhum estado novo; app local de um usuário; aplicar e desfazer são reversíveis |
| evento `history-update-payload` | forma | existing - `Updated { entry }` da history-undo; AC 5 |
| API `Store::apply_late_edit` | forma do erro | AC 4 (`NotFound`, `EmptyEdit`; `Mirror`/`Database` herdados) |
| documento `.md` em `notas/Ditados/` | estrutura após a edição tardia | existing - o mesmo `mirror::write`, com `final`, `edited_by` e `showing` novos |
| log | o que aparece | AC 6 (só `debug`, sem o texto) |

## Sources

- `docs/decisions/0004-pos-processamento-por-llm-na-nuvem-so-texto.md` - o prazo de 2 s e o "aplicar edição"
- `.specs/features/llm-auto/plan.md` (main) - Out of scope que entrega esta parte à F7b
- `.specs/features/postproc/plan.md` (door 6, `LateEdit`) e `.specs/features/history-undo/plan.md` (`showing`, undo/redo)
