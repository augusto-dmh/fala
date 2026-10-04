# active-app

## Problem

O Fala não sabe em que app o texto ditado vai entrar. Em `apps/desktop/src` não há detecção de janela em primeiro plano (grep por `GetForegroundWindow`, `active_app`, `foreground` sem resultado; `fala-research/plans/fase-1-delta-e-semanas-3-4.md` §0 item 3), e `crates/inject` é só um `//!`. Sem isso, o `AppContext` do S0 fica sempre vazio: o prompt do LLM não recebe o nome do app que a ADR-0004 prevê, a lista de apps onde o LLM fica desligado não tem com o que comparar, e o histórico de C grava `app` vazio. Quem paga é o uso diário no Windows (fase 1): o LLM formata igual um e-mail no Outlook e um comando no terminal, e não há como desligá-lo no app sensível.

No GNOME Wayland (o notebook de trabalho) a situação é outra: um app comum não tem como saber o app em foco sem extensão. O relatório `13a` mediu `org.gnome.Shell.Introspect.GetWindows` e `GetRunningApplications` com `AccessDenied`, `Shell.Eval` desligado e `_NET_ACTIVE_WINDOW = 0x0` com app Wayland em foco.

Quando isto entra, quem monta o ditado chama uma função de `fala-inject` e recebe o `AppContext` com o nome do executável em foco no Windows (`chrome`, `code`, `ms-teams`) ou `app_name: None` onde a plataforma não permite saber, sem erro fatal.

## Flow

Reusa o `AppContext` de `fala-core` (exists) e o `windows-sys` 0.61 que já está no `Cargo.lock`; não cria tipo novo de contexto.

1. quem monta o ditado (o desktop, na ligação de F7) chama `fala-inject` (exists, hoje vazio) - `foreground_app()`
2. Windows: `fala-inject` pergunta ao sistema a janela em primeiro plano, o processo dono dela e o caminho do executável desse processo (door 2), sem ler o título da janela
3. `fala-inject` reduz o caminho ao nome do app pela regra da door 1 e devolve `AppContext { app_name: Some(..) }`
4. fora do Windows (GNOME Wayland incluído): `fala-inject` devolve `InjectError::Unsupported` em `try_foreground_app()`, e `foreground_app()` converte em `AppContext::default()` com um log `debug` do motivo (door 4)
5. out: o `AppContext` vai ao `Dictation` (`fala-core`, exists), que B lê para o prompt e a lista de desligados e C grava no histórico

## Impact

| Front | What changes |
| --- | --- |
| domain | new term: `InjectError` - erros de `fala-inject` (`Unsupported`, `NoForegroundWindow`, `Os`), lives in `fala-inject` |
| domain | existing term: `AppContext.app_name` era "nome do app, se conhecido", sem formato; passa a ser o nome do executável em minúsculas sem `.exe` (door 1) - quem ramifica nele: ninguém no código de hoje; B compara a lista `--disable-app` com ele e C grava o valor, ambos em branches não integradas |
| domain | `crates/inject/src/lib.rs` doc comment fala só da inserção; ganha a detecção do app em foco |
| stored data | nothing to migrate - nada persiste `app_name` ainda |

## Relations

None - no stored-data shape change

## Surface

None - nothing consumed outside: funções de biblioteca dentro do workspace. O exemplo `cargo run -p fala-inject --example foreground_app` é ferramenta de verificação manual, não interface de produto.

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. identidade do app | `app_name = Some("chrome")`: o nome do arquivo do executável, depois do último `\` ou `/`, sem o sufixo `.exe` (sem diferenciar maiúsculas), em minúsculas; vazio vira `None` | nome amigável da versão do arquivo (`FileDescription` = "Google Chrome"): muda com o idioma e a versão do app e exige outra API; título da janela: carrega conteúdo do documento, e a ADR-0004 manda só o nome do app ao LLM |
| 2. dependências novas em `fala-inject` | `thiserror.workspace = true`, `log = "0.4"` (mesma forma das irmãs `fala-postproc` e `fala-audio`) e `[target.'cfg(windows)'.dependencies] windows-sys = { version = "0.61", features = ["Win32_Foundation", "Win32_System_Threading", "Win32_UI_WindowsAndMessaging"] }` | crate `windows` 0.61 (o do desktop): wrapper maior, sem ganho para quatro funções livres; crates de "janela ativa" do crates.io: leem o título da janela e trazem dependências próprias |
| 3. API pública | `pub fn foreground_app() -> AppContext`, `pub fn try_foreground_app() -> Result<AppContext, InjectError>`, `pub fn app_name_from_exe_path(path: &str) -> Option<String>` | trait `ForegroundApp` com um struct por plataforma: só existe uma implementação por build e o consumidor recebe o `AppContext` por valor, então um teste de F7 passa um `AppContext` falso sem trait; só `foreground_app() -> Result<..>` (D9): obriga cada chamador a reescrever o fallback que a D4 já fixou como "app desconhecido" |
| 4. Linux sem detecção | todo alvo que não é Windows devolve `InjectError::Unsupported(&'static str)` com o motivo | AT-SPI: respondeu na sonda do `13a` (1 app GTK em foco), mas Chromium e Electron não foram medidos, puxa `zbus` e dá leitura da árvore de acessibilidade de todos os apps, e o Linux é a fase 3 (ADR-0007); `_NET_ACTIVE_WINDOW` do XWayland: `0x0` com app Wayland em foco |

- Nothing else in this change is hard to reverse

## Criteria

### S1: identidade do app (P1)

Um caminho de executável vira sempre o mesmo nome curto.

**Acceptance Criteria**

1. WHEN `app_name_from_exe_path` recebe `C:\Program Files\Google\Chrome\Application\chrome.exe`, `C:\Users\a\AppData\Local\Programs\Microsoft VS Code\Code.exe`, `C:\Program Files\WindowsApps\MSTeams_x64\ms-teams.EXE`, `C:\Tools\Foo.Bar.exe`, `notepad` ou `/usr/bin/gnome-text-editor` THEN `fala-inject` SHALL devolver `Some("chrome")`, `Some("code")`, `Some("ms-teams")`, `Some("foo.bar")`, `Some("notepad")` e `Some("gnome-text-editor")`, respectivamente
2. IF `app_name_from_exe_path` recebe `""`, `C:\dir\`, `.exe` ou `C:\x\.EXE` THEN `fala-inject` SHALL devolver `None`
3. The `fala-inject` SHALL não chamar nenhuma API de título de janela (`GetWindowText*`), de modo que o título nunca chega ao `AppContext`

**Independent test:** `cargo test -p fala-inject foreground`

### S2: detecção por plataforma (P1)

Quem pede o app em foco sempre recebe um `AppContext`, e o motivo da falta fica disponível.

**Acceptance Criteria**

4. WHILE o alvo não é Windows (GNOME Wayland incluído), WHEN `try_foreground_app()` é chamada THEN `fala-inject` SHALL devolver `Err(InjectError::Unsupported(motivo))` com `motivo` não vazio
5. WHILE o alvo não é Windows, WHEN `foreground_app()` é chamada THEN `fala-inject` SHALL devolver `AppContext { app_name: None }` sem panic
6. IF a detecção devolve `Unsupported`, `NoForegroundWindow` ou `Os { .. }` THEN `foreground_app()` SHALL devolver `AppContext::default()` para cada um dos três
7. WHERE o alvo é Windows, WHEN `try_foreground_app()` é chamada com uma janela em primeiro plano THEN `fala-inject` SHALL devolver `Ok(AppContext { app_name })` com `app_name` igual a `app_name_from_exe_path` do executável do processo dono da janela (`TODO(windows)`: verificação manual)
8. The `fala-inject` SHALL depender de `windows-sys` só no alvo Windows: a árvore de dependências normais para `x86_64-unknown-linux-gnu` não contém `windows-sys` nem `tauri`
9. The código `cfg(windows)` de `fala-inject` SHALL passar no `cargo clippy --all-targets -- -D warnings` para `x86_64-pc-windows-msvc`, o alvo do job windows do CI

**Independent test:** `cargo test -p fala-inject` no Linux; `cargo run -p fala-inject --example foreground_app` no Windows

## Out of scope

| Excluded | Why |
| --- | --- |
| detecção em sessão X11 no Linux | o alvo Linux é GNOME Wayland (ADR-0007); X11 não é usado no notebook |
| detecção no GNOME Wayland por AT-SPI ou extensão | fase 3; o `13a` dá a evidência para o pitch dela |
| resolver o app real atrás do `ApplicationFrameHost.exe` (apps UWP como Calculadora e Configurações) | app de sistema; aparece como `applicationframehost`, aceitável para a lista de desligados da fase 1 |
| conferir, antes do Ctrl+V, que o foco não mudou | é do `Injector`, que nasce com a migração do `clipboard.rs`; usa a mesma chamada de sistema depois |
| nome amigável do app na UI | a UI da lista de desligados é de F7 |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| verificação do código Windows no Linux | `cargo clippy` com `RUSTC_BOOTSTRAP=1 -Zbuild-std` para `x86_64-pc-windows-msvc`, usando o `rust-src` que o `rust-toolchain.toml` já instala; sem instalar target (o `-gnu` foi tentado e falha por falta do `x86_64-w64-mingw32-dlltool`) | o job windows do CI só roda depois do merge; o clippy pega erro de tipo, de assinatura e de lint antes, sem baixar nada; não linka nem executa | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| processo elevado em foco | `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)`, que o Windows concede entre níveis de integridade; se falhar, `Os { call: "OpenProcess", code }` e `app_name: None` | o mesmo caminho de erro dos outros casos; sem privilégio extra | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |
| nível do log do motivo | `debug` | D4: falha de detecção vira log `debug`; o nome do app não é conteúdo ditado, mas não precisa subir de nível | y — delegado pelo Augusto em 2026-10-02, decidido pelo executor |

**Open questions:** none - all resolved or logged above.

## Observable

None - no user-facing surface

## Sources

- `fala-research/plans/fase-1-delta-e-semanas-3-4.md` § F6 e D9 - a feature, a recomendação de identidade e o lugar em `crates/inject`
- `fala-research/research/13a-spikes-linux-l4-l7.md` §5 - o que um app comum consegue ver do foco no GNOME 48 Wayland
- `docs/decisions/0004-*` e `0007-*` - só o nome do app ao LLM; `cfg` de plataforma só nos crates de plataforma
