# Spike 03 — Pill sem foco e colagem com restore

**Fase:** 0 · **Status:** protocolo e scripts prontos; medição no Windows pendente · **Risco:** design doc §7

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

`TODO(windows)`: rodar os dois scripts na máquina Windows e colar aqui as tabelas que eles
imprimem, com data e versão do Windows e de cada app.

Pill (orçamento: ≤ 50 ms, máximo 100 ms, ao lado de `show_latency_ms_p50` e
`show_latency_ms_max`):

| cycles | overlay_visible | topmost | focus_kept | hidden_after_cancel | show_latency_ms_p50 | show_latency_ms_max |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| | | | | | | |

Colagem com `reliable_paste` = `false`:

| app | kind | repeat | restored | pasted |
| --- | --- | ---: | --- | --- |
| | | | | |

Colagem com `reliable_paste` = `true` (comparação):

| app | kind | repeat | restored | pasted |
| --- | --- | ---: | --- | --- |
| | | | | |
