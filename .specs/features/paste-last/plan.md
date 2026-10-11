# paste-last

## Problem

Quando a cola de um ditado falha, ou quando a pessoa quer o mesmo texto noutro campo, o único caminho é abrir o Início e copiar o item. A bandeja tem "Copiar última transcrição", sem atalho e sem "colar". O Wispr tem "Paste last transcript" e "Copy last transcript" como atalhos e no menu da bandeja, e o Augusto os usa (`research/18` §1 #5, §4.4; `research/20` §1). Esta é a fatia A de `shortcuts-last-text`: dois atalhos novos, sem tecla por padrão, e "Colar o último ditado" na bandeja. A fatia B (diálogo único de atalhos) fica para depois.

A bandeja copia hoje `post_processed_text` ou o bruto do `history.db`; um ditado formatado pelas regras (#92) ou com a IA desfeita no Início copia outro texto que não o que o Início mostra e copia.

## Flow

Reusa `ShortcutAction`/`ACTION_MAP`, `handle_shortcut_event`, `TranscriptionCoordinator::is_busy`, `HistoryManager::get_latest_completed_entry`, `utils::paste` (exists).

1. `settings::get_default_settings` ganha `paste_last` e `copy_last` com `default_binding` e `current_binding` vazios; um store antigo os recebe no merge de bindings que o `load_or_create_app_settings` já faz (new: `add_missing_bindings`, extraído para teste)
2. `init_shortcuts`/`resume_all_shortcuts` (exists) pulam binding vazio; com tecla, o evento chega ao `ACTION_MAP`
3. new `last_dictation::text_of(entry)`: o texto que o item mostra no Início (`dictation.final_text` ou, depois de desfazer, `raw_text`); se esse estiver vazio e o bruto não (o pipeline esvaziou, item descartado-recuperável), o bruto; sem `dictation`, `post_processed_text` ou `transcription_text`; texto em branco → nada
4. `get_latest_completed_entry` passa a anexar o `dictation` do `fala.sqlite` (como a lista paginada)
5. `copy_last` (atalho, no press) e "Copiar" da bandeja: `last_dictation::copy_last` escreve `text_of` no clipboard
6. `paste_last` (atalho, no release): `to_paste(busy, latest)` devolve nada durante um ditado ou sem histórico; senão, no Windows espera a pessoa soltar Ctrl/Shift/Alt/Win (até 1 s), e `utils::paste` na thread principal, o mesmo caminho do ditado (acorde por app do #95, colagem confiável se ligada, auto-submit, `clipboard_handling`); erro → `paste-error`, como no ditado
7. "Colar o último ditado" na bandeja (layout ocioso): no Windows, o app em primeiro plano é lembrado quando o ponteiro entra no ícone e devolvido ao foco antes de colar, porque o menu da bandeja toma o foco
8. out: `info` "pasted/copied the last dictation" sem o texto; `debug` quando ignorado

## Impact

| Front | What changes |
| --- | --- |
| stored data | `bindings` ganha `paste_last` e `copy_last` vazios; nada a migrar além do merge existente |
| domain | new `apps/desktop/src/last_dictation.rs`: `text_of`, `to_paste`, `copy_last`, `paste_last`, e no Windows `wait_for_modifiers_released`, `remember_foreground`/`restore_foreground` |
| domain | `tray::copy_last_transcript` e `last_transcript_text` saem; o item da bandeja chama `last_dictation::copy_last` |
| UI | Geral ganha duas linhas `ShortcutInput`; a chip de um atalho sem tecla mostra "Sem atalho" em vez de vazia (vale também para o de pós-processamento) |
| UI | bandeja: "Colar o último ditado" acima de "Copiar o último ditado" (rótulo pt renomeado para o par) |

## Relations

None - no stored-data shape change

## Surface

None - `src/bindings.ts` não muda (nenhum comando novo).

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. ids dos bindings | `paste_last`, `copy_last` | `paste_last_transcript`: mais longo sem ganho; o id vive no store e na fatia B |
| 2. tecla padrão | nenhuma (`""`) | `alt+shift+z`/`alt+shift+x` do Wispr: colide com quem usa o Wispr ao lado e com atalhos de apps; a definição escolhe vazio |

- Reversíveis: colar no release e copiar no press; o prazo de 1 s para soltar os modificadores; colar pelo `clipboard::paste` do desktop (não pelo `Injector` do crate, que o desktop ainda não usa: troca própria, `crates/inject/src/lib.rs`)

## Criteria

### S1: atalhos e escolha do texto (P1)

1. WHEN um store schema 3 sem `paste_last`/`copy_last` é carregado THEN o desktop SHALL acrescentá-los com `current_binding` e `default_binding` vazios, sem mexer nos outros
2. WHEN `to_paste` recebe `busy = true` THEN SHALL devolver nada, mesmo com histórico
3. WHEN `to_paste` recebe nenhum item THEN SHALL devolver nada
4. WHEN o item tem `dictation` mostrando o final THEN `text_of` SHALL devolver o final; mostrando o bruto, o bruto; final vazio com bruto, o bruto; sem `dictation`, o pós-processado ou o `transcription_text`; texto só com espaços, nada
5. The `ACTION_MAP` SHALL ter `paste_last` e `copy_last`
6. WHEN o atalho `paste_last` é solto THEN o desktop SHALL colar pelo `utils::paste` (manual Windows: Bloco de Notas e Windows Terminal)

### S2: bandeja e Geral (P1)

7. The menu ocioso da bandeja SHALL ter `paste_last_transcript` antes de `copy_last_transcript`; o ocupado SHALL ter só o de copiar
8. The Geral SHALL renderizar `ShortcutInput` para `paste_last` e `copy_last`, com nome e descrição em pt e en
9. WHEN um binding não tem tecla THEN a chip SHALL mostrar `settings.general.shortcut.unset`

## Out of scope

| Excluded | Why |
| --- | --- |
| [Copiar] no aviso de "ninguém leu a cola" da pill | o desktop não tem esse aviso: a falha de cola é um toast `paste-error` na janela principal e o `PasteNotRead` do crate não está ligado ao desktop (troca própria). Os dois atalhos já dão o caminho de volta ao texto |
| diálogo único de atalhos, várias combinações por ação | fatia B |
| atalho de reunião, máscara da tecla Win | fatias B e C |
| feedback visual do `copy_last` | sem pedido; o Wispr também copia em silêncio pelo atalho |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| o menu da bandeja tira o foco do app no Windows | lembrar o foreground no `Enter` do ícone e devolvê-lo antes de colar | o muda faz `SetForegroundWindow` na janela dona do menu | n — prova manual no PR |
| colar com Alt+Shift ainda presos vira Ctrl+Alt+Shift+V | esperar soltar (GetAsyncKeyState, 1 s) | o ditado não tem o problema porque cola depois da transcrição | n — prova manual no PR |

**Open questions:** none.

## Observable

- `info` "Pasted the last dictation" / "Copied the last dictation" (sem texto)
- `debug` "paste_last ignored: dictation in progress" / "no dictation to paste"
