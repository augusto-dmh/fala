# inject-paste-proof

## Problem

O `Injector` do `crates/inject` (#57) cola por clipboard + acorde e não tem como saber se a cola chegou: escreve o texto, manda o Ctrl+V, espera 60 ms e restaura. Se o foco estava num lugar sem campo, o ditado some sem aviso; e se o destino demora mais de 60 ms para ler, ele cola o clipboard antigo, já restaurado. Ainda: o texto ditado entra no histórico do Win+V e, com a sincronização ligada, na nuvem da Microsoft, contra o pitch de privacidade do Fala; e a restauração sobrescreve às cegas uma cópia que a pessoa tenha feito nesses 60 ms. É o problema nº 1 de inserção (`fala-research/research/16` §3.4).

O Wispr Flow resolve os três no Windows com delayed rendering (`research/18` §3.4, §7 itens 1 a 3), e o Granola marca o texto como fora do histórico (`research/19` §9.4). Quando isto entra, `Injector::insert` devolve `Ok` só quando algum app leu o texto do clipboard depois do acorde, devolve `InjectError::PasteNotRead` quando ninguém leu dentro do prazo, nunca deixa o ditado no histórico do Win+V nem na nuvem, e só restaura o clipboard anterior se ninguém escreveu nele desde a cola.

## Flow

Reusa `paste_with_restore`, os traits `Clipboard`/`Keyboard` e o `WindowsInjector` (exists, #57); a sequência continua neutra de plataforma e testada com fakes.

1. quem cola chama `Injector::insert(text)` (exists)
2. a sequência salva o clipboard como hoje (texto; imagem só sem texto)
3. `Clipboard::offer_text(text)` (new): no Windows, uma janela message-only numa thread própria (door 3) abre o clipboard, esvazia, grava `CF_UNICODETEXT` com dado nulo (delayed rendering) e os três formatos de exclusão com `DWORD 0`, fecha, e anota o `GetClipboardSequenceNumber`
4. espera `delay_before`, manda o acorde (`PasteChord`, agora com `ShiftInsert`) por `SendInput` via `enigo`
5. se o acorde saiu: `Clipboard::wait_read(read_timeout)` (new) espera o `WM_RENDERFORMAT`; no render a janela grava o texto e reanota o número de sequência
6. espera `delay_after`; `Clipboard::unchanged_since_offer()` (new) compara o número de sequência atual com o anotado: igual restaura (texto, imagem ou limpa); diferente não restaura e registra em `debug`
7. `Clipboard::end_offer()` (new) solta a `DelayedOffer` (`paste/windows/offer.rs`), cujo `Drop` fecha a janela e junta a thread; se ela ainda é dona com o formato não renderizado (restauração falhou), `WM_RENDERALLFORMATS` grava o texto para o clipboard não ficar com formato vazio
8. out: `Ok(())`, `Keystroke` (acorde falhou), `PasteNotRead` (ninguém leu em `read_timeout`) ou `Clipboard` (oferta falhou, sem acorde)

## Impact

| Front | What changes |
| --- | --- |
| domain | new term: `InjectError::PasteNotRead` - nenhum app leu o texto oferecido dentro de `read_timeout`; o chamador ainda tem o texto - quem faz `match` exaustivo hoje: ninguém (o desktop só usa `foreground_app`) |
| domain | new term: `PasteChord::ShiftInsert` (o enum já é `#[non_exhaustive]`) |
| domain | existing term: `PasteConfig` ganha `read_timeout: Duration` (padrão 1500 ms); `delay_after` passa a ser a espera entre a leitura (ou o prazo) e a restauração |
| domain | existing term: o trait interno `Clipboard` troca a escrita do ditado por `offer_text`/`wait_read`/`unchanged_since_offer`/`end_offer`; `write_text` fica só para restaurar |
| domain | `lib.rs`: a doc da inserção descreve a prova de leitura e a exclusão do histórico |
| stored data | nothing to migrate |

## Relations

None - no stored-data shape change

## Surface

None - nada é consumido fora do workspace; o desktop ainda cola por `apps/desktop/src/clipboard.rs` (a troca é a feature seguinte). O exemplo `paste` é ferramenta de verificação manual.

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. prazo de leitura | `PasteConfig.read_timeout`, padrão `Duration::from_millis(1500)`; depois dele, `PasteNotRead` e a restauração (se o clipboard não mudou) | 500 ms: um Electron sob carga (VS Code, Teams) pode passar disso e a cola boa viraria aviso falso; 3 s: depois do prazo o clipboard volta ao anterior, então um destino que lê tarde cola o conteúdo antigo, e 3 s com o clipboard tomado é tempo demais para a pessoa que copia logo em seguida. 1,5 s fica uma ordem de grandeza acima do que um app lê no Ctrl+V |
| 2. lido mas não colado | conta como `Ok`. A prova é "um app chamou `GetClipboardData` no nosso texto", não "o texto apareceu". Não há como saber sem UIA (ler o campo), que o Wispr usa à parte e que é outra feature. Casos conhecidos: um monitor de clipboard que ignore `ExcludeClipboardContentFromMonitorProcessing` lê antes do destino e mascara a falha | exigir que quem leu seja o processo em primeiro plano: o Windows Terminal e apps UWP leem por processo auxiliar, e o falso `PasteNotRead` seria pior que o falso `Ok`; UIA `TextPattern` para conferir o campo: lê o conteúdo do documento, custo e privacidade de outra feature |
| 3. vida da janela oculta | uma thread + janela message-only (`HWND_MESSAGE`) por cola, criadas em `offer_text` e encerradas em `end_offer`; o estado do render fica num `thread_local!` da thread da janela | janela viva o tempo todo: thread ociosa por toda a vida do app, `Drop` com encerramento e estado entre colas; o custo por cola (registrar classe uma vez, criar janela, juntar thread) fica em ~1 ms contra os 60 ms de espera que já existem |
| 4. formatos de exclusão | junto com o texto, na mesma abertura do clipboard: `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory` e `CanUploadToCloudClipboard`, os três com um `DWORD 0` (mesma forma do `arboard`); a restauração usa `exclude_from_history` + `exclude_from_cloud` do `arboard`, porque o conteúdo restaurado já passou pelo histórico quando foi copiado | só `ExcludeClipboardContentFromMonitorProcessing` (Granola): os dois outros custam duas linhas e cobrem leitores que só olham um deles |
| 5. dependências | `windows-sys` 0.61 ganha as features `Win32_System_DataExchange`, `Win32_System_Memory`, `Win32_System_LibraryLoader` e `Win32_Graphics_Gdi`; nenhuma crate nova | `clipboard-win` direto (já vem pelo `arboard`): não expõe delayed rendering nem a janela dona |

- A restauração depois do prazo (e não só depois da leitura) vem do brief; o risco que ela traz é o da door 1

## Criteria

### S1: sequência com prova de leitura (P1)

**Acceptance Criteria**

1. WHEN o acorde sai e o texto oferecido é lido dentro de `read_timeout` THEN a sequência SHALL devolver `Ok(())` e restaurar o conteúdo salvo (texto, imagem ou limpar), na ordem salvar, oferecer, `delay_before`, acorde, esperar leitura, `delay_after`, conferir, restaurar, encerrar a oferta
2. WHEN o acorde sai e ninguém lê dentro de `read_timeout` THEN a sequência SHALL devolver `Err(InjectError::PasteNotRead)` e restaurar o conteúdo salvo do mesmo jeito
3. IF o número de sequência do clipboard mudou desde a oferta (ou desde o render) THEN a sequência SHALL não restaurar, registrar em `debug` e devolver o mesmo resultado de 1 ou 2
4. IF o acorde falha THEN a sequência SHALL não esperar leitura, restaurar como em 1 (se o clipboard não mudou) e devolver `Err(InjectError::Keystroke(_))`
5. IF a oferta falha THEN a sequência SHALL devolver `Err(InjectError::Clipboard(_))` sem mandar o acorde
6. The sequência SHALL encerrar a oferta em todo caminho que a abriu, depois da restauração
7. The `PasteConfig::default()` SHALL ser `CtrlV`, 60 ms, 60 ms, 100 ms e `read_timeout` 1500 ms
8. The texto ditado e o conteúdo salvo SHALL nunca ir ao log, em nenhum dos caminhos acima

**Independent test:** `cargo test -p fala-inject paste`

### S2: adaptador Windows (P1)

**Acceptance Criteria**

9. WHERE o alvo é Windows, WHEN `insert` cola no Bloco de Notas, no Windows Terminal e no VS Code THEN `insert` SHALL devolver `Ok` e o texto SHALL aparecer no destino (manual, 3 colas por app)
10. WHERE o alvo é Windows, WHEN `insert` cola numa janela sem campo de texto THEN `insert` SHALL devolver `Err(PasteNotRead)` depois de ~`read_timeout` e o clipboard SHALL voltar ao conteúdo de antes (manual)
11. WHERE o alvo é Windows, WHEN uma cola termina THEN o histórico do Win+V SHALL não mostrar o texto colado (manual)
12. The `PasteChord::ShiftInsert` SHALL mandar Shift + `VK_INSERT` (com a flag de tecla estendida que o `enigo` põe no Insert)
13. The código `cfg(windows)` SHALL passar no `cargo clippy --workspace --all-targets -- -D warnings` nesta máquina Windows

**Independent test:** `cargo run -p fala-inject --example paste`

## Out of scope

| Excluded | Why |
| --- | --- |
| desktop colando pelo `Injector`, com o `PasteNotRead` virando aviso e "colar o último" | só o crate já pede três PRs empilhados de ~400 linhas; a troca no desktop é a feature seguinte |
| tecla de cola por app | outra feature; aqui só a capacidade (`PasteChord`) |
| CF_HTML e outros formatos ricos | o ditado é texto puro |
| conferir o campo por UIA, refocar o campo do início do ditado | outras ideias do relatório 18, com custo próprio |
| restaurar formatos além de texto e imagem | o contrato do #57 é texto e imagem |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| prazo de leitura | door 1, 1500 ms | brief: "~1,5 s; configurável" | y — delegado |
| lido mas não colado | door 2, conta como `Ok`, documentado | indetectável sem UIA | y — delegado |
| vida da janela oculta | door 3, por cola | menos estado, sem thread ociosa | y — delegado |
| o render (`SetClipboardData` dentro de `WM_RENDERFORMAT`) pode mudar o número de sequência | reanotar o número depois do render | sem isso, toda cola lida pareceria "clipboard mudou" e nunca restauraria; a prova manual mostra qual dos dois acontece | y — delegado |
| prova manual sem pessoa na frente | o exemplo foca o destino por `AppActivate` (PowerShell) e a evidência é a saída do exemplo mais um print da tela | o agente roda sozinho nesta máquina | y — delegado |

**Open questions:** none - all resolved or logged above.

## Observable

- log `debug` "clipboard mudou desde a cola; não restaurado" (sem conteúdo)
- `InjectError::PasteNotRead` para o chamador mostrar o aviso (feature seguinte, no desktop)

## Sources

- `fala-research/research/18-wispr-flow-binario-1.6.1034.md` §3.4 e §7 itens 1 a 3
- `fala-research/research/19-granola-binario-7.637.md` §9.4
- `.specs/features/hotkey-inject-traits/` (o `Injector` e o contrato de restauração)
- Microsoft Learn: "Clipboard Operations" (delayed rendering, `WM_RENDERFORMAT`, `WM_RENDERALLFORMATS`) e "Clipboard Formats" (cloud clipboard and clipboard history formats)
