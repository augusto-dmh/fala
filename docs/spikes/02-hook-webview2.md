# Spike 02 — Hook de teclado com a WebView2 em foco

**Fase:** 0 · **Status:** medido no Windows (modos `auto` e `manual`) · **Risco:** design doc §7

## Objetivo

Medir se o hook global de teclado do Fala continua recebendo key-down e key-up quando a janela
do próprio Fala (Tauri, WebView2) tem o foco. O design doc §7 lista o risco: "Hook
`WH_KEYBOARD_LL` para de receber eventos com WebView2 em foco → hotkey morre quando a janela do
Fala está aberta". A pesquisa 04 §2 traz o relato que motiva o spike, marcado `[PARCIAL]`: hooks
`WH_KEYBOARD_LL` podem parar de receber eventos quando uma janela Chromium tem o foco.

O que se mede é o hook que o produto usa hoje, dentro do processo que tem a WebView2:

- O hook é o do crate `handy-keys` (`SetWindowsHookExW(WH_KEYBOARD_LL, …)` em
  `platform/windows/listener.rs`), não o `rdev`. O `rdev` está em `apps/desktop/Cargo.toml`, mas
  nenhum `.rs` de `apps/desktop` o importa: é dependência sem uso (removê-la é outra mudança).
  O pitch e a pesquisa 04 falam em `rdev`; o nome certo é `handy-keys`.
- A contagem vem do log do próprio app: a linha `debug` de `shortcut/fala_keys.rs`,
  `handy-keys event: binding=transcribe, hotkey=…, state=Pressed|Released`, escrita antes de
  qualquer ação. Nenhuma linha nova em `apps/desktop`.
- Teclas injetadas por `SendInput` chegam ao `WH_KEYBOARD_LL` com a flag `LLKHF_INJECTED`; o
  `handy-keys` só trata de forma especial os eventos que ele mesmo marca em `dwExtraInfo`. Por
  isso há dois modos: `auto` (injeta F9, reproduzível) e `manual` (teclado físico, que é o caso
  real e não depende de o hook aceitar teclas injetadas).

Este relatório mede; não recomenda.

## Como reproduzir

1. Build do app no Windows por `docs/dev/build-windows.md` (`bun run tauri build` ou
   `bun run tauri dev`).
2. Configuração do app, em Settings:
   - binding `transcribe` = `F9`, modo push-to-talk;
   - `paste_method` = `CtrlV`, `clipboard_handling` = `DontModify`, `reliable_paste` = `false`
     (os mesmos do spike 03, para rodar os dois na mesma sessão).
3. Rodar o app com log de debug: `fala.exe --debug` (sobrescreve o nível de log só nesta
   execução). O log fica em `%LOCALAPPDATA%\br.com.augusto.fala\logs\fala.log`.
4. Abrir a janela principal do Fala (título `Fala`) e deixá-la em foco.
5. Rodar num PowerShell 7 de 64 bits (`pwsh`; o 5.1 também roda, mas lê o script sem BOM como
   ANSI e mostra os acentos trocados). `-ExecutionPolicy Bypass` evita a política padrão do
   Windows 11, que bloqueia scripts:

```powershell
# Automático: 300 toques de F9 injetados, 2 s entre eles, 50 ms segurando (~10 min)
pwsh -ExecutionPolicy Bypass -File .\scripts\spikes\02-hook-count.ps1 -Mode auto -Presses 300 -IntervalMs 2000 -HoldMs 50

# Manual: 10 min digitando na janela do Fala e tocando F9 a cada ~10 s; informe quantos toques deu
pwsh -ExecutionPolicy Bypass -File .\scripts\spikes\02-hook-count.ps1 -Mode manual -Minutes 10 -ExpectedPresses 60
```

O script lê o `fala.log` enquanto ele cresce: o app apaga o arquivo ao passar de 500 KB
(`RotationStrategy::KeepOne`) e, em `--debug`, cada toque de push-to-talk escreve várias linhas.
Contar só no fim perderia eventos numa rotação. O script mantém um handle aberto no arquivo e,
quando o app o recria, drena o antigo até o fim antes de abrir o novo (no Windows 11 a exclusão
POSIX do NTFS tira o nome e o handle segue lendo). Cada rotação aparece no stderr; se uma rodada tiver
rotação e contagem diferente, repita-a antes de concluir qualquer coisa.

No modo `auto`, o script para com exit 2 se o Windows não deixar a janela do Fala vir para a
frente; clique nela e rode de novo.

Cada toque de 50 ms abre e fecha uma gravação curta que o app descarta; a linha do evento sai
antes disso, então a contagem não depende do modelo.

## Evidência medida

Modo `auto` medido em 2026-09-29 no Alienware 16 (Windows 11 Pro 25H2, build 10.0.26200.9457;
WebView2 Runtime 153.0.4234.48, lido do registro do EdgeUpdate), com o app de
`bun run tauri build` em `da03312` rodando como `fala.exe --debug`. Configuração: binding
`transcribe` = `f9`, `shortcut_activation` = `push_to_talk` (em Settings: Geral → Comportamento
do atalho → Segurar), `paste_method` = `ctrl_v`, `clipboard_handling` = `dont_modify`,
`reliable_paste` = `false`, modelo `parakeet-unified-en-0.6b` Q8_0 carregado. O script rodou no
Windows PowerShell 5.1 (`powershell.exe`): esta máquina não tem o PowerShell 7.

Rodada de 03:46:21Z a 03:56:58Z, três rotações do `fala.log` no meio (exit 0):

| mode | presses_sent | pressed_logged | released_logged | first_gap_at | foreground_at_end |
| --- | ---: | ---: | ---: | --- | --- |
| auto | 300 | 300 | 300 | - | Fala |

Uma rodada anterior, de 03:13:41Z a 03:24:20Z, deu 299/299 com `first_gap_at` = 03:14:24.918Z e
`foreground_at_end` = `Windows Default Lock Screen` (exit 1). No segundo do buraco, um processo
`powershell.exe` foi aberto na mesma sessão para ler o progresso, e a tela estava bloqueada ao
fim (a sessão foi bloqueada durante a rodada; o desligamento de vídeo por inatividade está em
"nunca"). Pela regra acima (rotação e contagem diferente), a rodada foi repetida sem nenhum
outro processo aberto e com a tela mantida ligada (`SetThreadExecutionState`); a tabela é a da
repetição.

Modo `manual` medido em 2026-10-01 na mesma máquina, com o mesmo binário (`da03312`,
`fala.exe --debug`) e o mesmo binding e modo de atalho. O Augusto digitou na janela do Fala, que
ficou em foco os 10 min, e tocou F9 no teclado físico a cada ~10 s, com
`-Mode manual -Minutes 10 -ExpectedPresses 60`. A tabela é a que o script gravou em
`C:\fala-spikes\02-manual.md`, que terminou às 03:31Z:

| mode | presses_sent | pressed_logged | released_logged | first_gap_at | foreground_at_end |
| --- | ---: | ---: | ---: | --- | --- |
| manual | 60 | 60 | 60 | - | Fala |

`presses_sent` é o valor de `-ExpectedPresses`. A contagem no papel não foi anotada na hora.
Como conferência, o `fala.log` daquela noite tem as linhas `hotkey=f9, state=Pressed` de
03:21:34Z a 03:31:56Z: são 62 em 10 min 22 s, e o script conta só a própria janela de 10 min,
cujo início ele não grava. O arquivo começa numa rotação às 03:21:26Z, por isso há um
`Released` a mais, de um toque anterior a ela. As três contagens da tabela batem entre si,
então a regra de repetir a rodada não se aplica.

`first_gap_at` é o instante (UTC) do primeiro toque sem `Pressed` no log; `foreground_at_end`
confirma que a janela do Fala ainda tinha o foco no fim. Se as contagens ficarem abaixo de
`presses_sent`, o design doc §7 nomeia as alternativas (`win-hotkeys`, thread dedicada) para uma
decisão posterior; este spike só registra o número.
