# hotkey-inject-traits

## Problem

O atalho global e a colagem do Fala só existem dentro da casca Tauri. O hook de teclado (`handy-keys`, medido no spike 02: 300/300 e 60/60 key-down e key-up com a janela do Fala em foco) vive em `apps/desktop/src/shortcut/fala_keys.rs`, preso a `AppHandle`, e a colagem com restore (spike 03, `reliable_paste = false`: restore 30/30, Chrome, VS Code e Terminal 6/6) vive em `apps/desktop/src/clipboard.rs`, presa ao `tauri-plugin-clipboard-manager` e ao `EnigoState` do app. `crates/hotkey` é só um `//!` de 6 linhas, que ainda cita o `rdev` (sem uso, spike 02), e `crates/inject` só detecta o app em foco.

Quem paga são três consumidores que já estão na fila e não têm onde se apoiar: o adaptador Linux do atalho e o da colagem da fase 3 (roadmap 3.T1 `hotkey-portal`, 3.T2 `inject-portal`, 3.T4 `gnome-toggle-fallback`), o 1.W-clipboard-timing e o `fala-cli`, que não consegue testar atalho nem colagem sem WebView. E a obrigação da fase 1 do `ARCHITECTURE.md` (adaptadores de plataforma que, no Linux, retornam `Unsupported`; ADR-0007 "os traits de plataforma são definidos na fase 1 já pensando nos dois SOs") continua aberta para `hotkey` e `inject`. A fonte não traz número de custo além da fila do roadmap (decisão 10, opção (b): abrir a D depois do spike 02 `manual`, que foi medido em 2026-10-01).

Quando isto entra, quem precisa de atalho com press e release, ou de colar texto com o clipboard restaurado, chama `fala-hotkey` e `fala-inject` sem Tauri: no Windows recebe o mesmo hook e a mesma sequência de colagem que os spikes mediram; fora do Windows recebe `Unsupported` com o motivo. O app que a pessoa usa não muda nesta feature.

## Flow

Reusa o laço do `HotkeyManager` de `shortcut/fala_keys.rs` (thread dona do manager, canal de comandos, `try_recv` a cada 10 ms) e a ordem de `clipboard.rs::paste_via_clipboard` (salva, escreve, espera, Ctrl+V, espera, restaura), com as mesmas bibliotecas medidas (`handy-keys`, `arboard`, `enigo`); não inventa outro hook nem outra estratégia de colagem. O desktop segue chamando o código dele até a feature de troca (ver `Out of scope`), então as duas cópias convivem por um tempo.

Atalho:

1. quem consome (nesta feature, só testes e o exemplo do crate) cria um `std::sync::mpsc::channel` e chama `fala-hotkey` (exists, hoje vazio) `platform_hotkey(tx)` (door 1)
2. Windows: `fala-hotkey` sobe uma thread dona do `handy_keys::HotkeyManager::new_with_blocking()` (door 4) e devolve um `Box<dyn GlobalHotkey>`; `register(binding_id, accelerator)` interpreta o acelerador no formato da door 2 e guarda o par na tabela de bindings
3. evento do `handy-keys` (id, `Pressed`/`Released`) -> tabela de bindings de `fala-hotkey` -> `HotkeyEvent { binding_id, accelerator, state }` enviado em `tx`; id desconhecido não gera evento
4. fora do Windows (GNOME Wayland incluído): `platform_hotkey` devolve `HotkeyError::Unsupported(motivo)` (door 5)

Colagem:

5. quem consome chama `fala-inject` (exists) `platform_injector(PasteConfig)` (door 3) e depois `insert(text)`
6. sequência de colagem de `fala-inject`, neutra de plataforma: lê o texto do clipboard; só sem texto, lê a imagem; escreve `text`; espera `delay_before`; manda o acorde; espera `delay_after`; restaura o texto salvo, ou a imagem salva, ou limpa o clipboard; restaura mesmo quando o acorde falhou
7. Windows: o clipboard é o `arboard` e o acorde é o `enigo` com `Key::Control` + `Key::Other(0x56)`, segurando o modificador por `modifier_hold` (door 4), o mesmo par que `input.rs::send_paste_ctrl_v` usa hoje
8. fora do Windows: `platform_injector` devolve `InjectError::Unsupported(motivo)` (door 5)
9. out: `Ok(())` ou `InjectError` para quem chamou; o clipboard volta ao conteúdo de antes

## Impact

| Front | What changes |
| --- | --- |
| domain | new term: `GlobalHotkey` / `HotkeyEvent` / `KeyState` / `HotkeyError` - o contrato do atalho global com press e release, lives in `fala-hotkey` |
| domain | new term: `Injector` / `PasteConfig` / `PasteChord` - a inserção de texto por clipboard + acorde com restore, lives in `fala-inject` ao lado de `foreground_app` |
| domain | existing term: `InjectError::Unsupported` era "detecção do app em foco indisponível" (texto do `Display`); passa a valer para detecção e inserção, com `Display` `indisponível nesta plataforma: {0}` e o motivo carregando o resto - quem ramifica hoje: ninguém; o único leitor é o `log::debug!` de `foreground_app` (`crates/inject/src/foreground.rs`), e `apps/desktop/src/actions.rs:828` chama `foreground_app()`, que não expõe o erro |
| domain | existing term: `InjectError` ganha `Clipboard(String)` e `Keystroke(String)` - quem faz `match` exaustivo nele hoje: só os testes de `foreground.rs` |
| docs | `crates/hotkey/src/lib.rs` diz "hook `WH_KEYBOARD_LL` via `rdev`"; passa a dizer `handy-keys` (spike 02). `crates/inject/src/lib.rs` diz "falha de inserção deixa o texto no clipboard"; passa a dizer o que o código faz (ver Assumptions). O parágrafo "Estado" do `ARCHITECTURE.md` diz que `hotkey` é só `//!` |
| code | a lógica do manager de `shortcut/fala_keys.rs` e a sequência de `clipboard.rs::paste_via_clipboard` passam a existir em dois lugares até a troca do desktop; `apps/desktop` não muda |
| dependencies | `Cargo.lock` ganha só arestas novas (`fala-hotkey` -> `handy-keys`, `log`, `thiserror`; `fala-inject` -> `arboard`, `enigo`); nenhum pacote novo nem versão nova, porque o desktop já trava `handy-keys 0.3.4`, `arboard 3.6.1` e `enigo 0.6.1` |
| stored data | nothing to migrate - o formato de acelerador da door 2 é o que `settings.bindings[*].current_binding` já grava |

## Relations

None - no stored-data shape change

## Surface

None - nothing consumed outside: APIs de biblioteca dentro do workspace. Os exemplos `cargo run -p fala-hotkey --example hotkey_listen` e `cargo run -p fala-inject --example paste` são ferramentas de verificação manual `TODO(windows)`, não interface de produto.

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. API pública de `fala-hotkey` | `pub trait GlobalHotkey: Send { fn register(&mut self, binding_id: &str, accelerator: &str) -> Result<(), HotkeyError>; fn unregister(&mut self, binding_id: &str) -> Result<(), HotkeyError>; }`; `pub fn platform_hotkey(events: std::sync::mpsc::Sender<HotkeyEvent>) -> Result<Box<dyn GlobalHotkey>, HotkeyError>`; `pub struct HotkeyEvent { pub binding_id: String, pub accelerator: String, pub state: KeyState }`; `pub enum KeyState { Pressed, Released }`; `pub enum HotkeyError { Unsupported(&'static str), InvalidAccelerator { accelerator: String, reason: String }, AlreadyRegistered(String), Backend(String) }`; soltar o `Box` para a thread e remove o hook | funções livres sem trait (como a door 3 do `active-app`): a fase 3 escolhe em tempo de execução entre portal, atalho custom do GNOME e evdev (`13` §2.4), o que pede objeto de trait, ao contrário do `foreground_app`, que tem uma implementação por build; callback `Fn(HotkeyEvent)`: roda código do consumidor na thread do adaptador e torna o teste dependente de sincronizar closures, enquanto o canal deixa o consumidor escolher a thread |
| 2. formato do acelerador | a string do `handy-keys`, `mod+mod+tecla` ou só modificadores, gravada em minúsculas e lida sem diferenciar maiúsculas (`ctrl+shift+space`, `f9`, `ctrl+space`), interpretada por `handy_keys::Hotkey::from_str`; é o formato que `settings.bindings[*].current_binding` já persiste | struct neutra `Accelerator { modifiers, key }`: exige um segundo parser e converter os settings salvos; o adaptador portal da fase 3 converte a string para o trigger do XDG dentro dele |
| 3. API pública de inserção em `fala-inject` | `pub trait Injector: Send { fn insert(&mut self, text: &str) -> Result<(), InjectError>; }`; `pub fn platform_injector(config: PasteConfig) -> Result<Box<dyn Injector>, InjectError>`; `pub struct PasteConfig { pub chord: PasteChord, pub delay_before: Duration, pub delay_after: Duration, pub modifier_hold: Duration }` com `Default` = `CtrlV`, 60 ms, 60 ms, 100 ms; `#[non_exhaustive] pub enum PasteChord { CtrlV, CtrlShiftV }`; `InjectError` ganha `Clipboard(String)` e `Keystroke(String)` e o `Display` de `Unsupported` vira `indisponível nesta plataforma: {0}` | o `Injector` ler `PasteMethod`, `ClipboardHandling`, `auto_submit` e `append_trailing_space`: são settings de produto do `AppSettings` do desktop, e `Direct`, `ExternalScript` e `None` não são a estratégia medida; `ShiftInsert` no enum: nenhum alvo da fase 1 usa, e o enum é `non_exhaustive` para crescer sem quebrar; uma variante nova `InsertUnsupported`: duplica o motivo que `Unsupported(&'static str)` já carrega |
| 4. dependências de plataforma | `crates/hotkey/Cargo.toml`: `thiserror.workspace = true`, `log = "0.4"` e `[target.'cfg(windows)'.dependencies] handy-keys = "0.3.4"`; `crates/inject/Cargo.toml`: `[target.'cfg(windows)'.dependencies] arboard = "3.6.1"` (features padrão, com `image-data`) e `enigo = "0.6.1"`, ao lado do `windows-sys` que já está lá | Win32 direto pelo `windows-sys` (`OpenClipboard`/`SetClipboardData` e `SendInput`): reimplementa o salvar e restaurar texto e imagem que o spike 03 mediu pelo `arboard` e pelo `enigo` (restore 30/30), e um caminho não medido pediria a matriz de novo no Windows; `handy-keys` em todos os alvos: traz o `evdev` para o build Linux e antecipa a escolha Linux (capacidade de keylogger, grupo `input`, `13` §2.4) que é da fase 3; `rdev`: declarado no desktop sem uso, e o spike 02 mediu o `handy-keys` |
| 5. Linux sem atalho e sem colagem | todo alvo que não é Windows devolve `HotkeyError::Unsupported(&'static str)` de `platform_hotkey` e `InjectError::Unsupported(&'static str)` de `platform_injector`, com o motivo (portal da fase 3) | embrulhar já o evdev do `handy-keys` e a cadeia `wtype`/`dotool`/`ydotool`/`xdotool` + `enigo` do `clipboard.rs`: no GNOME Wayland o Ctrl+V sintético não chega a app Wayland (`13` §3.1), o evdev exige o grupo `input`, e os spikes L1 e L2 do portal não rodaram; a escolha é da fase 3 (ADR-0007) |

- Nothing else in this change is hard to reverse

## Criteria

### S1: contrato do atalho, neutro e Unsupported no Linux (P1)

Quem pede o atalho recebe um adaptador ou o motivo de não haver um, e os eventos chegam com o binding certo e na ordem.

**Acceptance Criteria**

1. WHILE o alvo não é Windows (GNOME Wayland incluído), WHEN `platform_hotkey(tx)` é chamada THEN `fala-hotkey` SHALL devolver `Err(HotkeyError::Unsupported(motivo))` com `motivo` não vazio, sem panic e sem enviar nada em `tx`
2. WHEN o binding `transcribe` com acelerador `ctrl+shift+space` está registrado sob o id de backend 7 e chegam os eventos de backend (7, pressionado) e (7, solto) THEN a tabela de bindings de `fala-hotkey` SHALL enviar `HotkeyEvent { binding_id: "transcribe", accelerator: "ctrl+shift+space", state: KeyState::Pressed }` e depois o mesmo evento com `KeyState::Released`, nessa ordem, e nada mais
3. IF chega um evento de backend com um id que nunca foi registrado, ou que foi desregistrado antes do evento THEN a tabela de bindings SHALL não enviar nenhum `HotkeyEvent`
4. IF `register` é chamada com um `binding_id` já registrado THEN `fala-hotkey` SHALL devolver `Err(HotkeyError::AlreadyRegistered(binding_id))` e manter o acelerador anterior desse binding
5. WHEN `unregister` é chamada com um `binding_id` que não está registrado THEN `fala-hotkey` SHALL devolver `Ok(())`, como o `do_unregister` do desktop faz hoje

**Independent test:** `cargo test -p fala-hotkey` no Linux

### S2: atalho no Windows sobre o `handy-keys` (P1)

No Windows, o adaptador entrega press e release do hook que o spike 02 mediu.

**Acceptance Criteria**

6. WHERE o alvo é Windows, IF `register` recebe `""`, `ctrl+banana` ou `ctrl+a+b` THEN `fala-hotkey` SHALL devolver `Err(HotkeyError::InvalidAccelerator { accelerator, .. })` com o acelerador recebido, e para `ctrl+shift+space`, `f9` e `ctrl+shift` (só modificadores, que o desktop já aceita) o parser SHALL aceitar
7. WHERE o alvo é Windows, WHEN o `handy-keys` entrega `HotkeyState::Pressed` ou `HotkeyState::Released` THEN o adaptador SHALL traduzir para `KeyState::Pressed` ou `KeyState::Released`, respectivamente
8. WHERE o alvo é Windows, WHEN o exemplo `hotkey_listen` registra `f9` e a pessoa toca F9 60 vezes no teclado físico com a janela do exemplo em foco THEN o exemplo SHALL contar 60 `Pressed` e 60 `Released` e sair com 0 (`TODO(windows)`: manual, não roda no Linux)

**Independent test:** job `windows` do CI no PR para 6 e 7; `cargo run -p fala-hotkey --example hotkey_listen -- f9 60` no Windows para 8

### S3: sequência de colagem com restore, neutra e Unsupported no Linux (P1)

Colar não perde o que estava no clipboard, nem quando o acorde falha.

**Acceptance Criteria**

9. WHILE o alvo não é Windows, WHEN `platform_injector(PasteConfig::default())` é chamada THEN `fala-inject` SHALL devolver `Err(InjectError::Unsupported(motivo))` com `motivo` não vazio, sem panic
10. The `PasteConfig::default()` SHALL ser `chord: PasteChord::CtrlV`, `delay_before: 60 ms`, `delay_after: 60 ms`, `modifier_hold: 100 ms`, os padrões do desktop (`settings.rs` `default_paste_delay_ms`/`default_paste_delay_after_ms`; `clipboard.rs` `send_paste_ctrl_v(enigo, 100)`)
11. WHEN o clipboard tem o texto `antes` e a sequência de colagem roda com `ditado` e um acorde que dá certo THEN `fala-inject` SHALL fazer, nesta ordem, ler o texto, escrever `ditado`, esperar `delay_before`, mandar o acorde configurado, esperar `delay_after` e escrever `antes`, sem ler imagem, e devolver `Ok(())`
12. IF o clipboard não tem texto, ou tem texto vazio, e tem uma imagem THEN a sequência SHALL ler a imagem antes de escrever `ditado` e, depois do acorde, escrever de volta a mesma imagem (mesmas dimensões e bytes) em vez de limpar
13. IF o clipboard não tem texto nem imagem, ou as duas leituras falham THEN a sequência SHALL seguir com a colagem e, depois do acorde, limpar o clipboard
14. IF o acorde falha THEN a sequência SHALL restaurar o conteúdo salvo do mesmo jeito que em 11, 12 e 13 e devolver `Err(InjectError::Keystroke(_))`
15. IF escrever `ditado` no clipboard falha THEN a sequência SHALL devolver `Err(InjectError::Clipboard(_))` sem mandar o acorde
16. The `fala-inject` SHALL não passar o texto colado nem o conteúdo salvo do clipboard a nenhuma macro de log, em nenhum nível

**Independent test:** `cargo test -p fala-inject` no Linux, com clipboard e teclado falsos

### S4: colagem no Windows sobre `arboard` e `enigo` (P1)

No Windows, o adaptador cola pelo mesmo caminho que o spike 03 mediu.

**Acceptance Criteria**

17. WHERE o alvo é Windows, WHEN o exemplo `paste` roda com `teste um dois três` e o Bloco de Notas em foco, com o clipboard segurando primeiro um texto e depois uma imagem 8×8 THEN o texto SHALL aparecer no Bloco de Notas e o exemplo SHALL sair com 0 só se o clipboard voltou ao texto e à imagem originais (`TODO(windows)`: manual, não roda no Linux)

**Independent test:** `cargo run -p fala-inject --example paste -- "teste um dois três"` no Windows

### S5: plataforma, dependências e documentos (P1)

O código de plataforma fica nos dois crates, o Linux compila sem nada de Windows e o desktop fica como está.

**Acceptance Criteria**

18. The `fala-hotkey` e o `fala-inject` SHALL depender de `handy-keys`, `arboard` e `enigo` só no alvo Windows: a árvore de dependências normais dos dois para `x86_64-unknown-linux-gnu` não contém `handy-keys`, `arboard`, `enigo`, `windows`, `windows-sys` nem `tauri`, e a de `x86_64-pc-windows-msvc` contém `handy-keys v0.3.4` (em `fala-hotkey`) e `arboard v3.6.1` e `enigo v0.6.1` (em `fala-inject`)
19. The código `cfg(windows)` de `fala-hotkey` e `fala-inject` SHALL passar no `cargo clippy --all-targets -- -D warnings` para `x86_64-pc-windows-msvc` e no `cargo test` do job `windows` do CI no PR
20. The workspace SHALL seguir passando, no Linux, `cargo clippy --workspace --all-targets -- -D warnings` e `cargo test --workspace` (job `rust` do CI no PR), com os adaptadores de `hotkey` e `inject` devolvendo `Unsupported`
21. The diff da feature SHALL não tocar `apps/desktop/` nem `src/`, e SHALL adicionar `cfg(windows)`, `cfg(not(windows))` ou `cfg(target_os = ...)` só dentro de `crates/hotkey` e `crates/inject`
22. The doc comment de `crates/hotkey/src/lib.rs` SHALL citar `handy-keys` e não citar `rdev`, o de `crates/inject/src/lib.rs` SHALL descrever o restore depois de acorde falho, e o parágrafo "Estado" do `ARCHITECTURE.md` SHALL dizer que `hotkey` e `inject` têm os traits e os adaptadores Windows e que o desktop ainda usa `shortcut/` e `clipboard.rs`

**Independent test:** `cargo tree`, cross-clippy e `git diff` no Linux; jobs `rust` e `windows` do CI no PR

## Out of scope

| Excluded | Why |
| --- | --- |
| ligar o desktop aos crates (o laço de `fala_keys.rs` passa a usar `fala-hotkey`, `paste_via_clipboard` passa a usar `fala-inject`) e remover os `allow` de `apps/desktop/Cargo.toml` que esse código exigia | o corte recomendado (pergunta 1): a troca só se prova no Windows, re-rodando os spikes 02 `manual` e 03; vai numa feature própria, que fecha a metade "allow removidos" da obrigação da fase 1 |
| mover `paste_tx/` (colagem "confiável" por recibo) | o spike 03 mediu pior com `reliable_paste = true` (colou 14 contra 24 no OCR); fica no desktop, desligado por padrão |
| `Direct`, `ExternalScript`, `ShiftInsert`, `auto_submit`, espaço final e `CopyToClipboard` | settings de produto do desktop, fora da estratégia medida |
| captura de tecla para a tela de settings (`KeyboardListener`, `start_fala_keys_recording`) e o fallback para `tauri-plugin-global-shortcut` | UI e casca Tauri; ficam no desktop |
| adaptadores Linux reais (portal GlobalShortcuts, RemoteDesktop + Clipboard, atalho custom do GNOME, evdev) | fase 3 (3.T1, 3.T2, 3.T4); dependem dos spikes L1 e L2 |
| conferir, antes do Ctrl+V, que o foco não mudou | comportamento novo, que o desktop não tem; pede a janela capturada no press e muda a assinatura de `insert` |
| restaurar o clipboard só depois de o app consumir a colagem | é o 1.W-clipboard-timing do roadmap, que depende desta feature |
| tirar o `rdev` sem uso de `apps/desktop/Cargo.toml` | outra mudança (spike 02); esta feature não toca o desktop |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| o corte da feature | só os crates: traits e adaptadores Windows em `crates/hotkey` e `crates/inject`, `apps/desktop` sem mudança; a troca do desktop e a remoção dos `allow` ficam para uma feature própria | menor corte verificável no Linux (pergunta 1 do handback, respondida (a)) | y — delegado (Augusto, 2026-10-09) |
| como ficam no relatório as linhas que só rodam no Windows (8 e 17) | cada uma vira um check `Unproven` com `TODO(windows)`, que mantém o veredito FAIL até a rodada manual no Windows; nada fica fora da tabela | L-004 (candidata): decidir no plano, não deixar ao Verifier; mesmo tratamento do C7 do `active-app` | y — delegado (Augusto, 2026-10-09) |
| prova do código Windows a partir do Linux | `RUSTC_BOOTSTRAP=1 cargo clippy -Zbuild-std=std,panic_abort --target x86_64-pc-windows-msvc -p <crate> --all-targets -- -D warnings`, como no `active-app`; se o build-std não compilar `handy-keys` (crate `windows` 0.58), `arboard` ou `enigo` sem linkar, a prova Windows passa a ser só o job `windows` do CI no PR, e isso fica escrito no `checks.md` | o clippy não linka; o job `windows` roda em `pull_request` e cobre `clippy --workspace` e `cargo test` dos crates | y — delegado (Augusto, 2026-10-09) |
| onde rodam os testes de 6 e 7 | testes `#[cfg(windows)]` no crate, executados pelo `cargo test` do job `windows` do CI; a prova é o log do job no PR | o parser e a tradução de estado são do `handy-keys`, que só entra no build Windows (door 4) | y — delegado (Augusto, 2026-10-09) |
| o que fazer com o clipboard quando o acorde falha | restaurar o conteúdo salvo e devolver `Keystroke`, como `paste_via_clipboard` faz hoje; o doc comment de `crates/inject/src/lib.rs` é corrigido (AC 22) | não reprojetar a colagem (spikes 02 e 03); quem chama tem o texto e o histórico para oferecer "Colar" nas semanas 5-6 (pergunta 2 do handback, respondida (a)) | y — delegado (Augusto, 2026-10-09) |
| encerramento do adaptador de atalho | soltar o `Box<dyn GlobalHotkey>` manda `Shutdown` à thread e faz `join`, como o `Drop` de `FalaKeysState` | mesmo comportamento do desktop; verificado no Windows só pela rodada manual de 8 | y — delegado (Augusto, 2026-10-09) |
| modo bloqueante | `HotkeyManager::new_with_blocking()`: o acelerador registrado não chega ao app em foco, como no desktop | é o modo que o spike 02 mediu; trocar muda o que a pessoa vê no app em foco | y — delegado (Augusto, 2026-10-09) |
| tabela de bindings testável no Linux | a tabela (binding ↔ id de backend, roteamento de evento) é código neutro, com o id do backend como `u32` (`HotkeyId::as_u32`) e o estado como `KeyState`; só a tradução dos tipos do `handy-keys` é `cfg(windows)` | o `HotkeyId` do `handy-keys` não tem construtor público, e os critérios 2 a 5 precisam rodar no Linux | y — delegado (Augusto, 2026-10-09) |

**Open questions:** none - all resolved or logged above. As duas perguntas do handback (o corte e a falha do acorde) foram respondidas com a opção (a) e estão nas linhas acima.

## Observable

None - no user-facing surface: o app que a pessoa usa não muda nesta feature (AC 21), e os exemplos são ferramentas de verificação manual.

## Sources

- `fala-research/plans/roadmap-proposta-2026-10-02.md` linha 1.D e decisão 10 - o escopo (traits neutros, Windows sobre `handy-keys`, `Unsupported` no Linux) e o pré-requisito do spike 02 `manual`
- `docs/spikes/02-hook-webview2.md` e `docs/spikes/03-overlay-colagem.md` - o hook e a colagem que os adaptadores Windows embrulham
- `docs/decisions/0007-*` - `cfg` de plataforma só nos crates de plataforma; `cargo check --workspace` no Linux com `Unsupported`
