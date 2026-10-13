# pill-menu — menu de clique direito na pílula em repouso

## Problem

A pílula em repouso (#102) só abre Início no clique. Quem quer trocar o microfone, colar o último
ditado ou tirar a pílula da frente por um tempo precisa achar a bandeja, que o Windows 11 esconde
no overflow. O Flow Bar do Wispr tem um menu de clique direito com Microphone, Languages, Paste
last transcript, Hide for 1 hour e Settings (`fala-research/research/notes/21-wispr.md` § 8.A).
Pitch: `fala-research/plans/features/pill-idle-hint.md`, "fatia seguinte".

## Flow

1. Clique direito na pílula em repouso: a página impede o menu do webview e chama `open_pill_menu`
2. `open_pill_menu` (async, fora da thread principal) só age com a pílula em `idle`/`idle_hover`, guarda o app em primeiro plano (`remember_foreground`) e monta o menu com `tray::build_pill_menu`
3. Na thread principal, `popup_menu` abre o menu nativo no cursor e bloqueia até fechar; depois, se a pílula ficou em primeiro plano, `return_focus_from` devolve o foco ao app do usuário
4. O handler de menu da bandeja (`lib.rs`) ouve todo menu e roda o item: "Colar o último ditado", idioma e microfone com os ids da bandeja; `pill_hide_hour` chama `overlay::snooze_idle`; `pill_settings` abre a janela em Configurações (`open-settings`)
5. `snooze_idle` grava o fim (agora + 1 h) em `IDLE_SNOOZED_UNTIL` e chama `refresh_idle`; `current_rest` trata a pílula adiada como não querida; um timer de 1 h chama `refresh_idle` de novo. Ligar o toggle em Configurações encerra o adiamento

## Impact

| Front | What changes |
| --- | --- |
| behaviour | menu nativo na pílula em repouso; "Ocultar por 1 hora" esconde só a pílula em repouso (ditado, avisos e reunião continuam) |
| ui | chave `tray.hidePillHour` (pt, en); evento `open-settings` no `App.tsx` |
| code | submenus de idioma e microfone da bandeja viram funções compartilhadas |
| stored data | nada: o adiamento vive na memória e some ao reiniciar |

## Criteria

1. A right click on the idle pill SHALL open a native menu with, in order: Colar o último ditado, separator, Idioma do ditado ▸, Microfone ▸, separator, Ocultar por 1 hora, Configurações
2. The menu SHALL open only over the idle pill, never over a dictation, notice or meeting state
3. The items SHALL reuse the tray's ids and handler; Configurações SHALL open the main window on Configurações
4. After the menu closes, the focus SHALL go back to the app that had it, unless a click moved it elsewhere
5. "Ocultar por 1 hora" SHALL hide the idle pill for 60 min and bring it back after; dictation, notices and the meeting indicator SHALL still show (ADR-0005); turning the toggle on SHALL end it

## Out of scope

Arrastar a pílula (fatia empilhada seguinte), Histórico e Formatação do menu do Wispr, menu na
pílula de ditado ou de reunião, persistir o adiamento entre execuções.

## Assumptions

| Assumption | Chosen default | Confirmed? |
| --- | --- | --- |
| Menu nativo (muda) em vez de HTML | nativo: submenus, teclado e fechar ao clicar fora de graça, mesmo handler da bandeja | n — executor |
| Menu só na pílula em repouso | sim: no ditado o menu tomaria o foco do app que recebe a cola | n — executor |
| Adiamento em memória | sim: reiniciar mostra a pílula de novo | n — executor |
| Idioma no menu | sim (o do Wispr tem Languages; a função já é compartilhada) | n — executor |
