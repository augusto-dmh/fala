# terminal-aware-paste

## Problem

O desktop cola todo ditado com o mesmo acorde, o `paste_method` das configurações (`apps/desktop/src/clipboard.rs::paste`, padrão `CtrlV`). Num terminal, Ctrl+V não é colar: o conhost e o PuTTY mandam `^V` ao programa, o Alacritty e o WezTerm não o ligam por padrão, e um TUI (Claude Code, vim) que roda no Windows Terminal recebe a tecla antes do terminal e faz outra coisa com ela. O ditado some ou vira lixo no prompt, sem aviso, no destino que mais recebe ditado: Windows Terminal e Cursor somam 196 dos 762 ditados do Augusto no Wispr (`research/18` §4.4), e o Wispr resolve isso com Shift+Insert ligado por flag (`research/18` §3.4 item 6, §3.6, §7 item 5). Trocar o `paste_method` global para `ShiftInsert` não serve: em apps web e Electron, Shift+Insert ora cola, ora não faz nada.

Quando isto entra, uma colagem num terminal sai com Shift+Insert e nas outras janelas continua com o acorde configurado; quem configurou outro acorde de propósito não é sobreposto. O estilo "prompt" do pós-processamento já existe (`fala-postproc::Destination::Prompt`, #93); esta feature é só o acorde.

## Flow

Reusa `foreground_app()` e `PasteChord` (exists, `fala-inject`), `PasteMethod` e `clipboard::paste` (exists, desktop).

1. `clipboard::paste(text)` (exists) lê `settings.paste_method` como hoje
2. new: chama `fala_inject::foreground_app()` no momento da cola (não o `AppContext` capturado ao soltar a tecla, porque o foco pode ter mudado nesse meio-tempo) e `effective_paste_method(paste_method, &app)` (new, desktop)
3. new: `fala_inject::is_terminal(app_name)` responde pela lista de terminais (door 1); `PasteChord::for_app(&AppContext)` (new) é a mesma regra para quem usar o `Injector` do crate
4. `effective_paste_method`: `CtrlV` num terminal vira `ShiftInsert`; qualquer outro `paste_method` fica como está (door 2); app desconhecido fica como está
5. o resto de `paste` segue igual: caminho confiável (`paste_tx`) ou legado, auto-submit e `clipboard_handling`, agora com o acorde efetivo
6. out: a linha `info` "Using paste method" já existente mostra o acorde efetivo; um `debug` registra a troca com o nome do app (nunca o texto)

## Impact

| Front | What changes |
| --- | --- |
| domain | new term: `fala_inject::is_terminal(app_name) -> bool` e `PasteChord::for_app(&AppContext) -> PasteChord` - o acorde que cola naquele app; vive em `crates/inject/src/terminal.rs` |
| domain | `clipboard::paste` passa a colar com o acorde efetivo, não com o configurado; `paste_tx::try_reliable_paste` e `paste_via_clipboard` recebem o efetivo sem mudar |
| domain | `crates/inject/src/lib.rs` e o Code Map do `ARCHITECTURE.md` passam a dizer que o `inject` também escolhe o acorde por app |
| stored data | nothing to migrate - `paste_method` continua com o mesmo significado: o acorde fora de terminais |

## Relations

None - no stored-data shape change

## Surface

None - nada é consumido fora do workspace. O exemplo `paste` ganha `--chord auto` (escolhe pelo app em foco) como ferramenta da prova manual.

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. onde mora a lista de terminais | `crates/inject/src/terminal.rs`, `const TERMINALS: &[&str]` com nomes no formato do `app_name_from_exe_path` (minúsculas, sem `.exe`): os 9 do Wispr (`windowsterminal`, `cmd`, `powershell`, `pwsh`, `warp`, `alacritty`, `hyper`, `kitty`, `wezterm`) mais `wt`, `openconsole`, `conhost`, `wezterm-gui`, `tabby`, `mintty`, `ghostty`, `conemu`, `conemu64`, `putty`, e os do Linux que o `postproc` já lista (`gnome-terminal-server`, `kgx`, `ptyxis`, `konsole`, `xterm`, `foot`) | reaproveitar `fala_postproc::destination::PROMPT`: é outra pergunta (estilo do texto; inclui `claude`, `cursor`, `chatgpt`, que colam com Ctrl+V) e o `postproc` não pode depender do `inject` (crate de plataforma) nem o contrário (ADR-0007, pipeline `postproc ─▶ inject`); uma lista em `fala-core`: o core guarda tipos e config, não catálogo de apps, e a doc do `destination.rs` já diz que a detecção de terminal é do `inject` |
| 2. quando a regra vale | só quando `paste_method == CtrlV` (o padrão); `Direct`, `None`, `ShiftInsert`, `CtrlShiftV` e `ExternalScript` são escolha explícita e ficam | sobrepor sempre: quem escolheu `Direct` por causa de um app que não lê clipboard perderia isso no terminal; setting novo "acorde em terminais": mais uma configuração para explicar, sem pedido; o Wispr faz por flag sem UI |
| 3. o acorde do terminal | `ShiftInsert` | `CtrlShiftV`: o Windows Terminal e o GNOME Terminal aceitam, mas conhost, cmd, PuTTY e ConEmu não; Shift+Insert cola em todos os da lista (é o que o Wispr liga, `shift-insert` ON) |

- O momento da consulta ao app em foco (na cola, não ao soltar a tecla) é reversível: custa uma chamada Win32 e o `AppContext` do ditado continua sendo o de `actions.rs`

## Criteria

### S1: acorde por app no crate (P1)

**Acceptance Criteria**

1. WHEN `is_terminal` recebe `windowsterminal`, `WindowsTerminal`, `pwsh`, `cmd`, `conhost`, `wezterm-gui`, `alacritty` ou `kitty` THEN `fala-inject` SHALL devolver `true`
2. WHEN `is_terminal` recebe `chrome`, `code`, `cursor`, `notepad`, `claude`, `` ou `  ` THEN `fala-inject` SHALL devolver `false`
3. WHEN `PasteChord::for_app` recebe um `AppContext` com `app_name` de terminal THEN `fala-inject` SHALL devolver `ShiftInsert`; com `app_name` de outro app ou `None`, `CtrlV`
4. The `TERMINALS` SHALL estar toda em minúsculas e sem `.exe`, o formato de `app_name_from_exe_path`, de modo que a comparação sem caixa é redundante e não uma dependência

**Independent test:** `cargo test -p fala-inject terminal`

### S2: o desktop cola com o acorde efetivo (P1)

**Acceptance Criteria**

5. WHEN `effective_paste_method(CtrlV, app)` recebe um app de terminal THEN o desktop SHALL devolver `ShiftInsert`
6. WHEN `effective_paste_method(CtrlV, app)` recebe outro app ou app desconhecido THEN o desktop SHALL devolver `CtrlV`
7. WHEN `effective_paste_method(m, app)` recebe `m` em {`Direct`, `None`, `ShiftInsert`, `CtrlShiftV`, `ExternalScript`} e um terminal THEN o desktop SHALL devolver `m`
8. WHERE o alvo é Windows, WHEN um texto é colado pelo exemplo com `--chord auto` com o Windows Terminal em foco THEN o acorde SHALL ser `shift_insert` e o texto SHALL aparecer no terminal; com o Bloco de Notas em foco, `ctrl_v` (manual)
9. The troca de acorde SHALL ir ao log em `debug` com o nome do app e sem o texto

**Independent test:** `cargo test -p fala --lib clipboard`; `cargo run -p fala-inject --example paste -- --chord auto`

## Out of scope

| Excluded | Why |
| --- | --- |
| desktop colando pelo `Injector` do crate (`PasteNotRead` → aviso, "colar o último") | feature própria, já listada no `lib.rs` do `inject`; aqui só a escolha do acorde |
| `AppContext.kind` (Terminal, AiChat, Editor) no `fala-core` | o `postproc` já resolve o estilo por nome; um enum no core só se um terceiro consumidor aparecer |
| detectar o CLI de agente dentro do terminal (claude, codex) pelo título da janela | lê o título, que a ADR-0004 mantém fora do `AppContext`; e o estilo "prompt" já vale para o terminal inteiro |
| setting para desligar a regra | sem pedido; o `paste_method` explícito já é o opt-out (door 2) |
| Linux | o `foreground_app` devolve `None` lá até a fase 3, então a regra nunca dispara; a lista já traz os terminais do GNOME para esse dia |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| a prova manual sem pessoa ditando | exemplo `paste --chord auto`, destino focado por `AppActivate` como na `inject-paste-proof` | a dictação ao vivo exige o teclado do Augusto; o acorde é o mesmo que o desktop manda | y — delegado |
| o desktop consulta o app em foco de novo na cola | sim | o foco pode mudar entre soltar a tecla e colar (~0,5 s); a consulta custa microssegundos | y — delegado |

**Open questions:** none.

## Observable

- `info` "Using paste method: ShiftInsert" na cola em terminal (linha já existente)
- `debug` "terminal em foco (<app>): colando com Shift+Insert"

## Sources

- `fala-research/research/18-wispr-flow-binario-1.6.1034.md` §3.4 item 6, §3.6, §4.4, §7 item 5
- `fala-research/plans/status/sintese-2026-10-09.md` §1 item 7
- `.specs/features/inject-paste-proof/` (o `PasteChord::ShiftInsert` e o exemplo)
- `.specs/features/active-app/` (o formato de `app_name`)
