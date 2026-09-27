# spike-windows-prep — spikes 2 e 3 prontos para rodar no Windows

## Problem

Dois riscos do design doc §7 só se medem no Windows 11 e esta máquina é Ubuntu: "Hook
`WH_KEYBOARD_LL` para de receber eventos com WebView2 em foco → hotkey morre quando a janela do
Fala está aberta" e "Colagem falha em Electron/Chromium → texto não entra no Slack/VS Code". O
pitch soma a eles a pill: "janela `focusable:false` + `alwaysOnTop` mostrada 200 vezes". A
pesquisa 04 §2 traz o relato que motiva o spike 2, marcado `[PARCIAL]`: "hooks `WH_KEYBOARD_LL`
podem parar de receber eventos quando uma janela Chromium (WebView2/Electron) tem o foco".
Hoje não existe protocolo nem script: quando o Augusto sentar no Windows, teria de inventar a
medição na hora, e o resultado não seria reproduzível.

Dois fatos do código mudam o protocolo em relação ao pitch: o hook em uso no desktop é o do
crate `handy-keys` (`SetWindowsHookExW(WH_KEYBOARD_LL, …)` em `platform/windows/listener.rs`),
não o `rdev`, que está em `apps/desktop/Cargo.toml` mas nenhum `.rs` importa; e o binding
`cancel` só existe enquanto grava, então o único binding sempre registrado é `transcribe`. A
instância em execução já aceita `fala.exe --toggle-transcription` e `--cancel` (single-instance),
e o overlay é a janela de título `Recording` com `always_on_top(true)`, `focusable(false)` e
`SetWindowPos(HWND_TOPMOST, SWP_NOACTIVATE)`. Ou seja: os três spikes se medem sobre o app que
já existe, sem uma linha nova em `apps/desktop`.

Quando isto fechar, três scripts PowerShell em `scripts/spikes/` rodam cada medição e imprimem
uma tabela, e os relatórios `docs/spikes/02-hook-webview2.md` e `03-overlay-colagem.md` têm
objetivo e reprodução escritos, com a evidência marcada `TODO(windows)` até a rodada lá.

## Flow

Reusa o desktop inteiro como sujeito de teste: o log de `debug` do `handy-keys` (linha
`handy-keys event: binding=…, state=…` em `shortcut/fala_keys.rs`, antes de qualquer ação), a
interface single-instance (`--toggle-transcription`, `--cancel`), o overlay `Recording` e o
caminho de colagem de `clipboard.rs` (salva, escreve, Ctrl+V, restaura). Nada de Rust novo.

1. `scripts/spikes/02-hook-count.ps1` (new, door 1) - foca a janela do Fala, injeta N toques de F9 por `SendInput` (ou espera uma sessão manual de M minutos), depois conta no `fala.log` as linhas `handy-keys event: binding=transcribe` com `state=Pressed` e `state=Released` desde o início; imprime a tabela; sai 0/1/2
2. `scripts/spikes/03-overlay-focus.ps1` (new, door 1) - por 200 ciclos: guarda o HWND em foco, roda `fala.exe --toggle-transcription`, sonda `FindWindowW(null, "Recording")` a cada 10 ms até `IsWindowVisible`, lê `WS_EX_TOPMOST`, compara o foco, roda `fala.exe --cancel`, confirma que escondeu; imprime a tabela; sai 0/1/2
3. `scripts/spikes/03-paste-check.ps1` (new, door 1) - para cada app × tipo de clipboard × repetição: prepara o clipboard (texto com GUID ou bitmap gerado), dá 5 s para focar o app, liga e desliga a transcrição pelo `fala.exe` enquanto a pessoa fala a frase, espera a colagem, compara o clipboard com o original e pergunta se a frase apareceu; imprime a tabela; sai 0/1/2
4. `docs/spikes/02-hook-webview2.md` e `docs/spikes/03-overlay-colagem.md` (new, no door - formato do relatório 11 §3b) - Objetivo, Como reproduzir (build por `docs/dev/build-windows.md`, configuração do app, invocação exata de cada script), Evidência medida `TODO(windows)` com a tabela vazia que o script preenche
5. out: um PR `docs` com os três scripts e os dois relatórios; nenhum arquivo `.rs` muda; `scripts/check-brand.sh` continua verde (a string `handy-keys` está na allowlist)

## Impact

| Front | What changes |
| --- | --- |
| domain | novo termo: `spike script` - PowerShell descartável em `scripts/spikes/NN-<nome>.ps1`, um por medição, que imprime uma tabela Markdown e sai 0/1/2; o relatório de mesmo número o cita |
| domain | correção de termo: o pitch e a pesquisa 04 chamam o hook de "`rdev`"; no código o hook é o do `handy-keys`, e `rdev` é dependência sem uso em `apps/desktop` (fato a registrar no relatório 02; remover a dependência é PR à parte, fora deste escopo) |
| build | nada: nenhum `Cargo.toml`, nenhum `.rs`; o CI roda `check-brand.sh` sobre os scripts, que só citam `handy-keys` |
| stored data | nada |

## Relations

`None - no stored-data shape change`.

## Surface

Três comandos PowerShell novos; nenhuma mudança em `fala-cli` nem no desktop.

| Route | In | Out | Status |
| --- | --- | --- | --- |
| `scripts/spikes/02-hook-count.ps1` | `-Mode auto` (default) ou `manual`; `-Presses 300`; `-IntervalMs 2000`; `-HoldMs 50`; `-Minutes 10` (manual); `-ExpectedPresses N` (manual); `-LogDir` (default `$env:LOCALAPPDATA\br.com.augusto.fala\logs`); `-WindowTitle Fala` | stdout: tabela Markdown `mode · presses_sent · pressed_logged · released_logged · first_gap_at · foreground_at_end`; stderr: instruções e avisos | exit `0` contagens iguais ao esperado · `1` contagem diferente · `2` log ou janela não encontrados; sem status HTTP (comando local; 200-599 n/a) |
| `scripts/spikes/03-overlay-focus.ps1` | `-Cycles 200`; `-FalaExe` (default `fala.exe` no `PATH`); `-SettleMs 500`; `-PollMs 10` | stdout: tabela Markdown `cycles · overlay_visible · topmost · focus_kept · hidden_after_cancel · show_latency_ms_p50 · show_latency_ms_max`; stderr: progresso a cada 20 ciclos | exit `0` quatro contagens iguais a `cycles` · `1` alguma contagem menor · `2` `fala.exe` não roda ou overlay nunca apareceu; sem status HTTP (200-599 n/a) |
| `scripts/spikes/03-paste-check.ps1` | `-Apps` (default `chrome, Code, Teams, WindowsTerminal, notepad`); `-Repeats 3`; `-Kinds text,image`; `-FalaExe`; `-Phrase 'teste um dois três'`; `-SpeakSeconds 4`; `-PasteWaitMs 3000` | stdout: tabela Markdown com uma linha por `app · kind · repeat · restored · pasted` e uma linha `total`; stderr: contagem regressiva e perguntas | exit `0` todo `restored` e `pasted` = `y` · `1` alguma linha `n` · `2` `fala.exe` não roda ou clipboard inacessível; sem status HTTP (200-599 n/a) |

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. onde vive código de spike que não é Rust | `scripts/spikes/NN-<nome>.ps1`, com `#Requires -Version 5.1`, bloco `param(...)`, `Set-StrictMode -Version Latest`, tabela Markdown no stdout e exit 0/1/2; o relatório `docs/spikes/NN-*.md` cita a invocação exata | `fala-research/`: o relatório do repo citaria reprodução que não está versionada com o código que mede. `apps/cli` em Rust: as três medições precisam de Win32 (`FindWindowW`, `SendInput`, clipboard) e `apps/cli` não pode ter `cfg(windows)` (ADR-0007) |
| 2. como o hook é medido dentro do processo com WebView2 | binding `transcribe` = `F9`, modo push-to-talk, app rodando com `--debug`; contagem das linhas `handy-keys event: binding=transcribe, …, state=Pressed` e `state=Released` no `fala.log` (formato de `shortcut/fala_keys.rs`), com a janela do Fala em foco | contador `rdev` em `fala-cli`: mede outro hook, em outro processo, sem WebView2 em foco no processo do hook. `debug!` por tecla no desktop: lógica nova em `apps/desktop`, vetada na fase 0 pelo handoff |

- Nothing else in this change is hard to reverse: parâmetros, textos e o formato das tabelas mudam num commit; ninguém fora do repo os consome.

## Criteria

### S1: contagem do hook com a WebView2 em foco (P1)

O script prova quantos press/release o hook entregou enquanto a janela do Fala tinha o foco.

**Acceptance Criteria**

1. WHEN `02-hook-count.ps1 -Mode auto -Presses 300 -IntervalMs 2000 -HoldMs 50` roda com o Fala em `--debug`, binding `transcribe` = `F9` e a janela do Fala focada THEN o script SHALL ativar a janela `-WindowTitle`, anotar o instante inicial, injetar 300 pares key-down/key-up de F9 por `SendInput` com 50 ms entre down e up e 2 000 ms entre pares, e depois contar no `fala.log` mais recente de `-LogDir` as linhas com `handy-keys event: binding=transcribe` e `state=Pressed` (e `state=Released`) posteriores ao instante inicial
2. WHEN `-Mode manual -Minutes 10 -ExpectedPresses N` roda THEN o script SHALL escrever no stderr a instrução ("digite na janela do Fala e toque F9 a cada ~10 s"), esperar 10 minutos e contar como no AC 1, comparando com N
3. WHEN a contagem termina THEN o script SHALL imprimir no stdout uma tabela Markdown com `mode`, `presses_sent`, `pressed_logged`, `released_logged`, `first_gap_at` (instante do primeiro toque sem par no log, ou `-`) e `foreground_at_end` (título da janela em foco ao final)
4. IF `pressed_logged` ≠ `presses_sent` ou `released_logged` ≠ `presses_sent` THEN o script SHALL sair com 1
5. IF `-LogDir` não contém `fala.log` ou a janela `-WindowTitle` não existe THEN o script SHALL sair com 2 nomeando o que faltou, antes de injetar qualquer tecla

**Independent test:** no Windows, com o app aberto e focado, `-Presses 20 -IntervalMs 500` termina em ~10 s com `20 · 20 · 20` e exit 0. `TODO(windows)`.

### S2: overlay sem foco, 200 vezes (P1)

O script prova que a pill aparece, fica no topo, não rouba o foco e some.

**Acceptance Criteria**

6. WHEN `03-overlay-focus.ps1 -Cycles 200` roda com o Fala aberto e outra janela em foco THEN o script SHALL, por ciclo, guardar o HWND em foco, executar `fala.exe --toggle-transcription`, sondar a cada `-PollMs` até `-SettleMs` se a janela de título `Recording` está visível (`IsWindowVisible`), ler se tem `WS_EX_TOPMOST`, comparar o HWND em foco com o guardado, executar `fala.exe --cancel` e confirmar que a janela deixou de estar visível em até `-SettleMs`
7. WHEN os ciclos terminam THEN o script SHALL imprimir uma tabela Markdown com `cycles`, `overlay_visible`, `topmost`, `focus_kept`, `hidden_after_cancel`, `show_latency_ms_p50` e `show_latency_ms_max` (tempo entre o `--toggle-transcription` retornar e a janela ficar visível)
8. IF qualquer uma das quatro contagens é menor que `cycles` THEN o script SHALL sair com 1
9. IF `fala.exe` não executa ou a janela `Recording` nunca fica visível nos 3 primeiros ciclos THEN o script SHALL sair com 2 nomeando a causa

**Independent test:** `-Cycles 5` com o Bloco de Notas em foco imprime `5 · 5 · 5 · 5 · 5` e o cursor continua piscando no Bloco de Notas. `TODO(windows)`.

### S3: colagem com restore em cinco apps (P1)

O script conduz a matriz app × clipboard × repetição e registra o que a pessoa viu e o que o clipboard mostrou.

**Acceptance Criteria**

10. WHEN `03-paste-check.ps1` roda THEN o script SHALL, para cada combinação de `-Apps` × `-Kinds` × `-Repeats`, colocar no clipboard um texto `FALA-SPIKE-<guid>` (kind `text`) ou um bitmap 8×8 gerado (kind `image`, sem texto junto), escrever no stderr "foque `<app>` num campo de texto" com contagem regressiva de 5 s, executar `fala.exe --toggle-transcription`, esperar `-SpeakSeconds`, executar `fala.exe --toggle-transcription` de novo, esperar `-PasteWaitMs`, ler o clipboard e comparar com o original (texto igual; imagem com mesmas dimensões e mesmos bytes), e perguntar no stderr se `-Phrase` apareceu no app (`y`/`n`)
11. WHEN a matriz termina THEN o script SHALL imprimir uma tabela Markdown com uma linha por `app · kind · repeat · restored · pasted` e uma linha `total` com as contagens de `y`
12. IF alguma linha tem `restored` = `n` ou `pasted` = `n` THEN o script SHALL sair com 1
13. IF `fala.exe` não executa ou o clipboard não pode ser lido THEN o script SHALL sair com 2 antes da primeira combinação

**Independent test:** `-Apps notepad -Repeats 1` com o Bloco de Notas aberto: uma linha por kind, exit 0 se a frase apareceu e o clipboard voltou. `TODO(windows)`.

### S4: relatórios com objetivo e reprodução (P1)

Os dois relatórios existem no formato do kit, com a evidência marcada como pendente.

**Acceptance Criteria**

14. The system SHALL ter `docs/spikes/02-hook-webview2.md` e `docs/spikes/03-overlay-colagem.md` com os títulos `## Objetivo`, `## Como reproduzir` e `## Evidência medida`, e SHALL NOT conter título com "Recomenda"
15. The section `## Como reproduzir` de cada relatório SHALL listar: a build por `docs/dev/build-windows.md`, a configuração do app (`--debug`; binding `transcribe` = `F9` em push-to-talk; `paste_method` = `CtrlV`, `clipboard_handling` = `DontModify`, `reliable_paste` = `false`), e a invocação exata de cada script com os valores dos ACs 1, 6 e 10
16. The section `## Evidência medida` de cada relatório SHALL conter a linha `TODO(windows)` e a tabela vazia com as colunas exatas que o script correspondente imprime (ACs 3, 7 e 11)
17. The report `02-hook-webview2.md` SHALL registrar que o hook medido é o do `handy-keys` (`WH_KEYBOARD_LL` em `platform/windows/listener.rs`), que `rdev` é dependência sem uso em `apps/desktop`, e que teclas injetadas por `SendInput` chegam ao hook com `LLKHF_INJECTED`, motivo de o modo `manual` existir
18. The report `03-overlay-colagem.md` SHALL citar o orçamento "tecla → pill visível ≤ 50 ms, máximo 100 ms" do `ARCHITECTURE.md` ao lado de `show_latency_ms_*`, e SHALL pedir uma segunda rodada de colagem com `reliable_paste` = `true`, como comparação, sem recomendar
19. The reports SHALL listar, em `## Como reproduzir`, o campo de texto de cada app (Chrome: `data:text/html,<textarea>`; VS Code: arquivo novo; Teams: caixa de mensagem sem enviar; Windows Terminal: prompt; Bloco de Notas: corpo)

**Independent test:** `grep -c '^## ' docs/spikes/0{2,3}-*.md` ≥ 3 cada; `grep -ci recomenda` = 0; `grep -c 'TODO(windows)'` ≥ 1 cada; `grep -c 'handy-keys' docs/spikes/02-hook-webview2.md` ≥ 1.

### S5: higiene do PR (P1)

Nada de Rust, nada de marca, scripts com cabeçalho uniforme.

**Acceptance Criteria**

20. The PR SHALL NOT alterar nenhum arquivo `.rs`, `Cargo.toml` ou `Cargo.lock`
21. The scripts SHALL cada um começar com `#Requires -Version 5.1`, ter um bloco `param(` e `Set-StrictMode -Version Latest`, e SHALL NOT conter a palavra `handy` fora da string `handy-keys` (para `scripts/check-brand.sh` passar)
22. WHEN `scripts/check-brand.sh` roda THEN o script SHALL sair com 0

**Independent test:** `git diff --stat main -- '*.rs' 'Cargo.*'` vazio; `head -1 scripts/spikes/*.ps1`; `scripts/check-brand.sh`.

## Out of scope

Product capabilities only. Process and harness rules live in AGENTS.md or as Observable `n/a`.

| Excluded | Why |
| --- | --- |
| Rodar as medições | só no Windows; a evidência fica `TODO(windows)` até o Augusto ou a sessão de build sentar lá (handoff) |
| Contador de teclas em `fala-cli` (rdev ou handy-keys) | mede outro processo e não o risco (hook dentro do processo com WebView2); sem valor para a decisão |
| `fala-cli paste` com `arboard` + `enigo` | duplicaria `clipboard.rs` para medir um caminho que o produto não usa; o app real cola melhor que um clone |
| Remover a dependência `rdev` sem uso | PR à parte (`chore(desktop)`), quando alguém confirmar no Windows que nada a carrega por `cfg` |
| Slack, Outlook, navegadores além do Chrome | a lista do pitch são os cinco apps; outros entram na fase 1 conforme uso |
| Restore de clipboard com imagem e texto ao mesmo tempo | o desktop só restaura imagem quando não há texto (`clipboard.rs`); o kind `image` do script reflete isso |
| Trade study cpal loopback vs `wasapi` e Parakeet vs Nemotron | tarefa 4 do handoff, só quando as medições existirem |
| Alternativas ao hook (`win-hotkeys`, thread dedicada) | o design §7 as reserva para quando o spike falhar; o spike mede |

## Assumptions

Defaults that are not already a numbered criterion. Drop a row once it is.

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Título da janela principal do Fala no Windows | `Fala` (`productName` de `tauri.conf.json`); parametrizado em `-WindowTitle` | conferido na primeira rodada; se diferir, o parâmetro resolve sem mudar o script | n |
| Caminho de `fala.exe` e do log | `fala.exe` no `PATH` (ou `target\release\fala.exe`) e `$env:LOCALAPPDATA\br.com.augusto.fala\logs\fala.log` (`TargetKind::LogDir` com `file_name: "fala"`) | é o padrão do `tauri-plugin-log`; parametrizado em `-FalaExe` e `-LogDir` | n |
| Toques curtos em push-to-talk | cada toque de 50 ms abre e fecha uma gravação que o app descarta como curta; o log do evento sai antes disso | é o custo de medir sem código novo no desktop; 300 toques em 10 min | n |
| Teclas injetadas contam para o hook | `SendInput` entrega ao `WH_KEYBOARD_LL` com `LLKHF_INJECTED`; o `handy-keys` só trata de forma especial a própria máscara de menu (`dwExtraInfo` marcado) | leitura de `platform/windows/listener.rs`; o modo `manual` cobre teclado físico | n |
| Teams | o "novo Teams" (WebView2), o que está instalado na máquina | é o app real de reunião do Augusto | n |
| Detecção da colagem | resposta da pessoa (`y`/`n`) e não leitura do app | ler o conteúdo de Chrome/Teams/Terminal por automação custa mais que o spike vale | n |
| Latência da pill | medida do retorno do `--toggle-transcription` até `IsWindowVisible`; inclui o IPC single-instance, então é um teto para o caminho da hotkey | a primeira validação com câmera a 240 fps é da fase 1 (design doc) | n |
| Validação de sintaxe dos scripts | nenhuma nesta máquina (`pwsh` não instalado); o Verifier confere estrutura (AC 21) e o Windows confere execução | instalar `pwsh` no Linux só para isso não vale | n |

**Open questions:** none - all resolved or logged above.

## Observable

Worksheet, not the review. `n/a` needs its reason. One row may group the same decision
across several routes.

| Surface | Decision | Landing |
| --- | --- | --- |
| command `02-hook-count.ps1` | output format | AC 3 |
| command `02-hook-count.ps1` | flags and defaults | AC 1, AC 2 |
| command `02-hook-count.ps1` | exit codes and halfway failure | AC 4, AC 5 - valida antes de injetar; depois disso só conta |
| command `03-overlay-focus.ps1` | output format | AC 7 |
| command `03-overlay-focus.ps1` | flags and defaults | AC 6 |
| command `03-overlay-focus.ps1` | exit codes and halfway failure | AC 8, AC 9 - aborta nos 3 primeiros ciclos; depois conta falhas e segue |
| command `03-paste-check.ps1` | output format | AC 11 |
| command `03-paste-check.ps1` | flags and defaults | AC 10 |
| command `03-paste-check.ps1` | exit codes and halfway failure | AC 12, AC 13 - uma combinação que falha vira linha `n`, nunca aborta a matriz |
| all three scripts | verbosity | AC 3, 7, 11 - stdout só a tabela; instruções e progresso no stderr |
| document `docs/spikes/02-*.md`, `03-*.md` | structure | AC 14 |
| document `docs/spikes/02-*.md`, `03-*.md` | depth | AC 15, AC 16, AC 19 |
| document `docs/spikes/02-*.md`, `03-*.md` | what the reader does next | AC 17, AC 18 - rodar no Windows e preencher a tabela; comparar com o orçamento, sem recomendar |
| document `docs/spikes/02-*.md`, `03-*.md` | tone | existing - formato do relatório 11 §3b: objetivo, como reproduzir, evidência medida, sem recomendação |
| collection `scripts/spikes/` | naming and ordering | door 1 - `NN-<nome>.ps1`, NN = número do relatório |
| screen | n/a - o overlay e a janela de settings já existem; nada de UI nova (pitch: "Nenhuma UI") |
| API or webhook | n/a - nada sai pela rede; os scripts falam só com o `fala.exe` local |

## Sources

- `~/projects/fala-research/pitches/fase-0-spikes.md` - spikes 2 e 3: o que medir, os cinco apps, as 200 exibições
- `docs/design/2026-10-fala-v1.md` §7 - as duas linhas de risco (hook com WebView2; colagem em Chromium) e a pill
- `~/projects/fala-research/research/04-integracao-plataforma.md` §2 e §3 - o relato `[PARCIAL]` do hook e as opções de injeção
- `ARCHITECTURE.md` - orçamento "tecla → pill visível ≤ 50 ms, máximo 100 ms"; ADR-0007 sobre `cfg`
- `apps/desktop/src/shortcut/fala_keys.rs`, `lib.rs` (single-instance), `overlay.rs`, `clipboard.rs` - o comportamento existente que os scripts observam
