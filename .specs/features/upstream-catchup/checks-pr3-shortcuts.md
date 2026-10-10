# upstream-catchup PR 3 (atalhos: Esc preso e validação do backend Tauri) checks

Profile: light
Plan: none - cada tema cabe numa frase (AGENTS.md, "Fluxo por feature")

## Intent

Duas correções de atalho do upstream. Primeiro (upstream #2190): o PR #26 do Fala passou a armar o
Esc pela mudança do estado busy do coordinator, mas `fala_keys` e `tauri_impl` ainda faziam cada
register e unregister num `tauri::async_runtime::spawn` independente. Numa janela busy curta
(gravação descartada, cancelamento logo após começar) as duas tasks saem quase juntas, sem ordem
garantida; se o unregister roda primeiro, o Esc fica capturado no sistema todo até sair do app (no
Windows, quebra o Esc em todos os apps). Com este PR, o pedido é gravado de forma síncrona e cada
task aplica o pedido mais recente, então a ordem deixa de importar. Segundo (upstream #2158):
`validate_shortcut` do backend Tauri passa a usar o parser de acelerador, então um atalho com
modificador de lado gravado pelo `fala_keys` (`ctrl_right+space`) é recusado e volta ao padrão em
vez de falhar no registro.

Decisões (Confirmed? y — delegado):

- A reconciliação mora num tipo `CancelKey` (pedido em `AtomicBool`, estado real em `Mutex<bool>`)
  com um `reconcile(apply)` que recebe o backend como closure, para o teste exercitar a ordem
  trocada sem `AppHandle`. O upstream usa duas statics soltas; a regra é a mesma.
- Um pass cujo backend falha não muda o estado gravado, então o próximo pass tenta de novo.
- O estado guarda o binding registrado (`Mutex<Option<ShortcutBinding>>`), e o unregister recebe
  esse mesmo binding, não o que está nas settings. Assim, um atalho de cancelar editado no meio do
  ditado ainda solta o Esc capturado. O upstream desregistra o binding atual das settings; lá, o
  unregister falha, o estado fica "registrado" e o Esc morre até reiniciar (achado da rodada 1 do
  Verifier, que o código anterior do Fala não tinha).
- O `#[cfg(target_os = "linux")]` (Esc desligado no Linux, como antes) fica em `apps/desktop`,
  que a ADR-0007 permite.
- Os comentários do #2158 citam `fala_keys`, nunca a marca do upstream.

10 checks in 1 slice · 0 one-way doors · 0 open, of which 0 block

Comandos de cargo com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`, na
raiz do worktree.

## Checks

### S1 - atalhos · 3 files · 110 KB · ~28k

**C1** - Com pedidos `true` e depois `false` gravados em ordem e os dois passes rodando depois
(o do stop primeiro), a tecla termina não registrada e o backend não recebe nenhuma chamada
(upstream #2190)
Proof: `cargo test -p fala --lib -- shortcut::tests::cancel_key_ends_unregistered_when_passes_run_out_of_order`

**C2** - Cada pass aplica o pedido mais recente uma vez: `true` registra, um segundo pass com o
mesmo pedido não chama o backend, `false` desregistra; chamadas = `[true, false]`
Proof: `cargo test -p fala --lib -- shortcut::tests::cancel_key_follows_the_latest_request`

**C3** - Um registro que falha deixa o estado em não registrado e o pass seguinte registra;
chamadas ao backend que funcionou = `[true]`
Proof: `cargo test -p fala --lib -- shortcut::tests::cancel_key_retries_after_a_failed_registration`

**C4** - Nenhum backend tem mais `register_cancel_shortcut`/`unregister_cancel_shortcut` próprios;
as duas funções existem só em `shortcut/mod.rs`, e lá não há nenhum `async_runtime::spawn` além do
de `schedule_cancel_reconcile`
Proof: `! grep -n 'fn register_cancel_shortcut\|fn unregister_cancel_shortcut' apps/desktop/src/shortcut/fala_keys.rs apps/desktop/src/shortcut/tauri_impl.rs && test "$(grep -c 'async_runtime::spawn(' apps/desktop/src/shortcut/mod.rs)" -eq 1 && grep -B3 'async_runtime::spawn(' apps/desktop/src/shortcut/mod.rs | grep -q 'fn schedule_cancel_reconcile'`

**C5** - `validate_shortcut` recusa `option_left+space` e `ctrl_right+space` e aceita
`option+space`, `option+shift+space`, `ctrl+space`, `ctrl+shift+space`, `alt+space` e `escape`
(upstream #2158)
Proof: `cargo test -p fala --lib -- shortcut::tauri_impl::tests::rejects_side_specific_modifiers_the_parser_cannot_register shortcut::tauri_impl::tests::accepts_the_default_shortcuts`

**C6** - Os testes de atalho e do coordinator que já existiam continuam verdes sem asserção
editada (inclui a decisão de armar do PR #26)
Proof: `cargo test -p fala --lib -- shortcut:: transcription_coordinator::tests::cancel_key_changes_only_when_busy_flips`

**C7** - Nenhuma marca do upstream entra no código; os comentários do #2158 dizem `fala_keys`
Proof: `scripts/check-brand.sh && grep -q 'the fala_keys recorder saves' apps/desktop/src/shortcut/tauri_impl.rs`

**C9** - O unregister recebe o binding que o register devolveu: registrado com `escape` e as
settings trocadas para `ctrl+q`, o stop desregistra `["escape"]` e a tecla termina solta
Proof: `cargo test -p fala --lib -- shortcut::tests::cancel_key_unregisters_the_binding_it_registered`

**C10** - Um unregister que falha deixa o estado registrado e o pass seguinte desregistra;
chamadas ao backend que funcionou = `[true, false]`
Proof: `cargo test -p fala --lib -- shortcut::tests::cancel_key_retries_after_a_failed_unregistration`

**C8** - TODO(windows): no build do Windows, 20 gravações descartadas logo após começar (toque
curto) seguidas de Esc no Bloco de Notas; o Esc chega ao Bloco de Notas nas 20 (a tecla não ficou
capturada). Não dá para conferir no Linux, onde o Esc dinâmico é desligado
Proof: `TODO(windows)` manual - contar quantos Esc chegam ao Bloco de Notas depois de cada toque curto (esperado 20/20)

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| ordens dos passes de um ditado curto (2) | stop antes do start C1 · start antes do stop C2 | - |
| resultado do backend (4) | register Ok C2 · register Err C3 · unregister Ok C2, C9 · unregister Err C10 | - |
| binding desregistrado (2) | o que foi registrado C9 · o das settings atuais nunca C9 | - |
| backends que tinham cancel próprio (2) | `fala_keys` C4 · `tauri_impl` C4 | - |
| atalhos do #2158 (8) | recusados `option_left+space` C5 · `ctrl_right+space` C5 · aceitos `option+space` C5 · `option+shift+space` C5 · `ctrl+space` C5 · `ctrl+shift+space` C5 · `alt+space` C5 · `escape` C5 | - |
| plataformas do Esc dinâmico (2) | Windows C8 (manual) · Linux desligado, sem mudança C4 | - |

- C4 e C7 são estruturais; C8 é a única prova do hook real e fica para o Windows.

## Swept

- validation: C5
- failure modes: C3, C10
- idempotency: C2 (pass repetido não chama o backend)
- authorization: n/a - nada de acesso novo
- concurrency: C1 (ordem dos passes), C4 (um só lugar agenda)
- data lifecycle: n/a - nada persistido
- dependency failure: C3 (backend recusa o registro)
- state transitions: C1, C2
- observability: C3 (o erro do backend é logado com o sentido da operação)

## Handoff

- S1 = ~28k (3 arquivos, 110 KB / 4), sob o orçamento de 150k - one builder
