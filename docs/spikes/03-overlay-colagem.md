# Spike 03 — Pill sem foco e colagem com restore

**Fase:** 0 · **Status:** pill e colagem medidas no Windows · **Risco:** design doc §7

## Objetivo

Medir, no Windows 11 e nos apps que o Augusto usa, duas apostas do design:

1. **A pill:** a janela de gravação (`Recording`, `always_on_top`, `focusable(false)`,
   `SetWindowPos(HWND_TOPMOST, SWP_NOACTIVATE)` em `overlay.rs`) aparece, fica no topo, não rouba
   o foco do app onde o texto vai entrar e some ao cancelar, mostrada 200 vezes.
2. **A colagem:** o caminho de `clipboard.rs` (salva o clipboard, escreve o texto, Ctrl+V,
   restaura) entrega o texto em Chrome, VS Code, Teams, Windows Terminal e Bloco de Notas, e
   devolve o clipboard original, com texto e com imagem. O design doc §7 lista o risco:
   "Colagem falha em Electron/Chromium → texto não entra no Slack/VS Code".

Tudo é medido sobre o app que já existe, sem código novo em `apps/desktop`: a instância em
execução aceita `fala.exe --toggle-transcription` e `fala.exe --cancel` pelo single-instance (que
não traz a janela principal para a frente nesses dois casos).

Este relatório mede; não recomenda.

## Como reproduzir

1. Build do app no Windows por `docs/dev/build-windows.md`; `fala.exe` no `PATH` ou passado por
   `-FalaExe`.
2. Configuração do app, em Settings: binding `transcribe` = `F9` em push-to-talk;
   `paste_method` = `CtrlV`; `clipboard_handling` = `DontModify`; `reliable_paste` = `false`;
   modelo de ASR já baixado e carregado (sem ele a gravação não abre). Rodar com
   `fala.exe --debug` para o log ajudar a explicar uma falha.
3. Os scripts rodam num PowerShell 7 de 64 bits (`pwsh`; o 5.1 lê o script sem BOM como ANSI
   e mostra os acentos trocados) com `-ExecutionPolicy Bypass`. O Fala precisa estar aberto
   antes: sem uma instância rodando, os scripts saem com 2 (um `fala.exe --toggle-transcription`
   abriria o app em vez de gravar).
   Pill — com o Bloco de Notas em foco (cursor piscando no corpo):

```powershell
pwsh -ExecutionPolicy Bypass -File .\scripts\spikes\03-overlay-focus.ps1 -Cycles 200 -SettleMs 500 -PollMs 10
```

4. Colagem — com os cinco apps abertos, cada um com o campo de texto pronto:
   - **Chrome:** uma aba em `data:text/html,<textarea>` (clique dentro da caixa);
   - **VS Code:** um arquivo novo sem salvar;
   - **Teams:** a caixa de mensagem de uma conversa consigo mesmo, **sem enviar**;
   - **Windows Terminal:** o prompt do PowerShell (não pressione Enter);
   - **Bloco de Notas:** o corpo de um documento novo.

```powershell
pwsh -STA -ExecutionPolicy Bypass -File .\scripts\spikes\03-paste-check.ps1 -Apps chrome,Code,Teams,WindowsTerminal,notepad -Repeats 3 -Kinds text,image
```

   Em cada combinação: foque o campo durante a contagem de 5 s, fale "teste um dois três" quando
   o terminal pedir, e volte ao terminal para responder `y`/`n`. O script confere sozinho se o
   clipboard voltou ao original (texto `FALA-SPIKE-<guid>` igual; bitmap 8×8 com o mesmo tamanho
   e os mesmos bytes). Uma combinação que falha (clipboard ocupado por outro app, `fala.exe` sem
   resposta em 10 s) vira uma linha `n`/`n` e a matriz segue. O app só restaura imagem quando o
   clipboard não tinha texto (`clipboard.rs`), por isso o kind `image` põe só a imagem.

5. Segunda rodada da colagem, como comparação: mudar `reliable_paste` = `true` em Settings e
   repetir o passo 4, registrando a tabela à parte.

A latência medida pelo script (`show_latency_ms_*`) vai do retorno do `fala.exe
--toggle-transcription` até `IsWindowVisible` e inclui o IPC do single-instance: é um teto para
o caminho da tecla. O orçamento do `ARCHITECTURE.md` para "tecla → pill visível e áudio
capturando" é ≤ 50 ms, máximo 100 ms; a validação com câmera a 240 fps é da fase 1.

## Evidência medida

Pill medida em 2026-09-29 no Alienware 16 (Windows 11 Pro 25H2, build 10.0.26200.9457), com o
app de `bun run tauri build` em `da03312` rodando como `fala.exe --debug`, a configuração do
spike 02 (`transcribe` = `f9`, `push_to_talk`, `ctrl_v`, `dont_modify`, `reliable_paste` =
`false`) e o `parakeet-unified-en-0.6b` Q8_0 carregado. Bloco de Notas 11.2607.14.0 (o app da
Store). O script rodou no Windows PowerShell 5.1 (`powershell.exe`), com
`-FalaExe C:\dev\fala\target\release\fala.exe`; esta máquina não tem o PowerShell 7.

Pill (orçamento: ≤ 50 ms, máximo 100 ms, ao lado de `show_latency_ms_p50` e
`show_latency_ms_max`), rodada de 04:04:09Z a 04:07:00Z, com uma aba nova e vazia do Bloco de
Notas em foco antes e depois (exit 0):

| cycles | overlay_visible | topmost | focus_kept | hidden_after_cancel | show_latency_ms_p50 | show_latency_ms_max |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 200 | 200 | 200 | 200 | 200 | 0 | 23 |

Rodadas anteriores na mesma sessão, registradas porque diferem:

| Início (UTC) | Janela em foco antes | Resultado | Foco no fim |
| --- | --- | --- | --- |
| 03:57:19 | a janela principal do Fala (o Bloco de Notas não abriu: `Start-Process notepad -PassThru` falha com o Bloco de Notas da Store) | 200/200/200/200/200, p50 0 ms, max 29 ms; fora do protocolo, descartada | Fala |
| 04:01:02 | um documento já aberto no Bloco de Notas | 200/200/**198**/200/200, p50 0 ms, max 7 ms (exit 1) | a janela principal do Fala |

Na rodada de 04:01, o foco saiu do Bloco de Notas em 2 dos 200 ciclos e terminou na janela
principal do Fala; o script não registra em quais ciclos. A repetição de 04:04 não reproduziu.
As três rodadas correram enquanto a rodada 1 do spike 04 gravava no mesmo notebook.

`show_latency_ms_p50` = 0 significa que, na maioria dos ciclos, a pill já estava visível na
primeira sondagem depois do retorno do `fala.exe --toggle-transcription`.

Colagem medida em 2026-09-29 na mesma máquina, com o `03-paste-check.ps1` corrigido por #16 (sem
a correção, toda linha sai `n`/`n`: o `Read-Host` interpolava `$app?`, que o PowerShell lê como uma
variável `app?`). Apps: Chrome 154.0.8037.58 com `data:text/html,<textarea autofocus>`, VS Code 1.139.1 num
arquivo não salvo, Teams 26246.1604.5133.838 na conversa consigo mesmo (nada enviado), Windows
Terminal 1.24.11911.0 no prompt do Windows PowerShell, Bloco de Notas 11.2607.14.0. Modelo
`parakeet-tdt-0.6b-v3` Q8_0 com `selected_language` = `pt`.

A pessoa não falou nem respondeu: um condutor fora do repositório rodou o script (com #16 e mais
nada) e,
em cada combinação, focou a janela do app, clicou no campo, tocou a frase "teste um dois três"
com a voz Microsoft Daniel (pt-BR) no `Voicemeeter AUX Input`, roteado ao `Voicemeeter Out B2`
que o Fala usou como mic (`selected_microphone`), e respondeu `y`/`n` pela contagem de "tes" no
OCR do Windows antes e depois. `restored` é medido pelo próprio script.

Dois detalhes do comando acima valem para qualquer rodada: `-File` passa `-Apps chrome,Code,...`
como um único nome no PowerShell (a matriz vira uma linha só); o condutor usou
`powershell -STA -Command "& '.\scripts\spikes\03-paste-check.ps1' -Apps chrome,Code,Teams,WindowsTerminal,notepad
-Repeats 3 -Kinds text,image -SpeakSeconds 6"`. `-SpeakSeconds 6` dá tempo à primeira gravação
depois de abrir o app, que levou ~2.8 s para ficar pronta.

Colagem com `reliable_paste` = `false` (17:33:43Z-17:42:26Z), como o script imprimiu:

| app | kind | repeat | restored | pasted |
| --- | --- | ---: | --- | --- |
| chrome | text | 1 | y | y |
| chrome | text | 2 | y | y |
| chrome | text | 3 | y | y |
| chrome | image | 1 | y | y |
| chrome | image | 2 | y | y |
| chrome | image | 3 | y | y |
| Code | text | 1 | y | y |
| Code | text | 2 | y | y |
| Code | text | 3 | y | y |
| Code | image | 1 | y | y |
| Code | image | 2 | y | y |
| Code | image | 3 | y | y |
| Teams | text | 1 | y | y |
| Teams | text | 2 | y | y |
| Teams | text | 3 | y | n |
| Teams | image | 1 | y | n |
| Teams | image | 2 | y | n |
| Teams | image | 3 | y | n |
| WindowsTerminal | text | 1 | y | y |
| WindowsTerminal | text | 2 | y | y |
| WindowsTerminal | text | 3 | y | y |
| WindowsTerminal | image | 1 | y | y |
| WindowsTerminal | image | 2 | y | y |
| WindowsTerminal | image | 3 | y | y |
| notepad | text | 1 | y | y |
| notepad | text | 2 | y | y |
| notepad | text | 3 | y | n |
| notepad | image | 1 | y | n |
| notepad | image | 2 | y | y |
| notepad | image | 3 | y | y |
| total | - | 30 | 30 | 24 |

Colagem com `reliable_paste` = `true` (comparação; 17:44:05Z-17:53:05Z), como o script imprimiu:

| app | kind | repeat | restored | pasted |
| --- | --- | ---: | --- | --- |
| chrome | text | 1 | y | y |
| chrome | text | 2 | y | y |
| chrome | text | 3 | y | n |
| chrome | image | 1 | y | y |
| chrome | image | 2 | n | y |
| chrome | image | 3 | n | y |
| Code | text | 1 | y | y |
| Code | text | 2 | y | n |
| Code | text | 3 | y | n |
| Code | image | 1 | y | n |
| Code | image | 2 | y | n |
| Code | image | 3 | y | n |
| Teams | text | 1 | y | n |
| Teams | text | 2 | y | n |
| Teams | text | 3 | y | n |
| Teams | image | 1 | y | n |
| Teams | image | 2 | y | n |
| Teams | image | 3 | y | n |
| WindowsTerminal | text | 1 | y | y |
| WindowsTerminal | text | 2 | y | y |
| WindowsTerminal | text | 3 | y | n |
| WindowsTerminal | image | 1 | y | y |
| WindowsTerminal | image | 2 | y | y |
| WindowsTerminal | image | 3 | y | y |
| notepad | text | 1 | y | n |
| notepad | text | 2 | y | n |
| notepad | text | 3 | y | y |
| notepad | image | 1 | y | n |
| notepad | image | 2 | y | y |
| notepad | image | 3 | y | y |
| total | - | 30 | 28 | 14 |

O OCR errou para os dois lados, então cada `n` foi conferido nas capturas e na barra de status
(Bloco de Notas: número de caracteres; VS Code: `Col`, com todas as colagens numa linha só).
Contagem conferida:

| app | kind | `false`: colou | `false`: restored | `true`: colou | `true`: restored |
| --- | --- | --- | --- | --- | --- |
| chrome | text | 3/3 | 3/3 | 2/2 e 1 inválida | 3/3 |
| chrome | image | 3/3 | 3/3 | 3/3 | 1/3 |
| Code | text | 3/3 | 3/3 | 3/3 | 3/3 |
| Code | image | 3/3 | 3/3 | 3/3 | 3/3 |
| Teams | text | 3/3 | 3/3 | inconclusivo | 3/3 |
| Teams | image | inconclusivo | 3/3 | inconclusivo | 3/3 |
| WindowsTerminal | text | 3/3 | 3/3 | 3/3 | 3/3 |
| WindowsTerminal | image | 3/3 | 3/3 | 3/3 | 3/3 |
| notepad | text | 3/3 | 3/3 | 2/3 | 3/3 |
| notepad | image | 2/3 | 3/3 | 2/3 | 3/3 |

- Chrome, texto 3, com `true`: inválida. O OCR do "antes" demorou ~5 s, o condutor perdeu a linha
  "recording is ready" do log e tocou a frase 5 s depois de a gravação fechar; não houve fala.
- Chrome, imagem 2 e 3, com `true`: o app registrou `[reliable-paste] clipboard changed externally;
  leaving it untouched`, e o script leu o clipboard diferente do original (`restored` = `n`).
- Teams: as três colagens de texto com `false` entraram (cada uma no ponto do clique, no meio da
  anterior). Depois disso a caixa não mudou mais, e um Ctrl+V feito direto pelo condutor, sem o
  Fala, também não entrou: o clique deixou de pôr o foco na caixa. Essas linhas não medem o Fala.
- Com `reliable_paste` = `true`, o log registra `clipboard read <n>ms after chord` (8 a 20 ms) em
  29 das 30 combinações, inclusive as do Teams e as duas do Bloco de Notas que não colaram: ler o
  clipboard não garante que o texto entrou.
- Bloco de Notas: com `false`, imagem 1 não colou (107 → 107 caracteres); com `true`, texto 2
  (152 → 152) e imagem 1 (167 → 167) não colaram. O app registrou "Text pasted successfully" em
  todas.

Durante as rodadas, o Voicemeeter tinha `Altofalantes (Realtek(R) Audio)` como A1 e AUX→B2 ligado:
com o JBL como A1 o motor do Voicemeeter parou (medidores em zero, B2 mudo). Depois das rodadas,
o roteamento, os volumes e os ajustes do app voltaram ao original.
