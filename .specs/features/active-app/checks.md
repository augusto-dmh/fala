# active-app checks

Profile: light
Plan: `.specs/features/active-app/plan.md`

9 checks in 2 slices · 4 one-way doors · 0 open, of which 0 block

## Checks

Prefixo de todo `cargo` abaixo: `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`. Os testes ficam ao lado do código em `crates/inject/src/foreground.rs`. O prefixo da prova Windows cruzada é `RUSTC_BOOTSTRAP=1` e `-Zbuild-std=std,panic_abort` (ver Assumptions do plano).

### S1 - identidade do app · 3 files · 6 KB · ~2k

**C1** - `app_name_from_exe_path` devolve `Some("chrome")` para `C:\Program Files\Google\Chrome\Application\chrome.exe`, `Some("code")` para `C:\Users\a\AppData\Local\Programs\Microsoft VS Code\Code.exe`, `Some("ms-teams")` para `C:\Program Files\WindowsApps\MSTeams_x64\ms-teams.EXE`, `Some("foo.bar")` para `C:\Tools\Foo.Bar.exe`, `Some("notepad")` para `notepad` e `Some("gnome-text-editor")` para `/usr/bin/gnome-text-editor`, caso a caso (AC 1, door 1)
Proof: `cargo test -p fala-inject foreground::tests::exe_path_becomes_lowercase_name -- --exact`

**C2** - `app_name_from_exe_path` devolve `None` para `""`, `C:\dir\`, `.exe` e `C:\x\.EXE`, caso a caso (AC 2, door 1)
Proof: `cargo test -p fala-inject foreground::tests::exe_path_without_name_is_none -- --exact`

**C3** - nenhum arquivo de `crates/inject` contém `GetWindowText` (AC 3)
Proof: `! grep -rn "GetWindowText" crates/inject`

### S2 - detecção por plataforma · 5 files · 10 KB · ~3k

**C4** - fora do Windows, `try_foreground_app()` devolve `Err(InjectError::Unsupported(motivo))` e `motivo` não é vazio (AC 4, door 4)
Proof: `cargo test -p fala-inject foreground::tests::non_windows_detection_is_unsupported -- --exact`

**C5** - fora do Windows, `foreground_app()` devolve `AppContext { app_name: None }` (AC 5, door 3)
Proof: `cargo test -p fala-inject foreground::tests::non_windows_context_is_unknown -- --exact`

**C6** - a conversão de resultado de `foreground_app()` leva `Err(Unsupported("x"))`, `Err(NoForegroundWindow)` e `Err(Os { call: "OpenProcess", code: 5 })` a `AppContext::default()`, e `Ok(AppContext { app_name: Some("code") })` a ele mesmo (AC 6)
Proof: `cargo test -p fala-inject foreground::tests::every_error_becomes_unknown_app -- --exact`

**C7** - no Windows, com o Chrome em primeiro plano 3 s depois do comando, o exemplo imprime o `AppContext` e sai com 0 só se `app_name == Some("chrome")` (AC 7; `TODO(windows)`: manual, não roda no Linux)
Proof: `cargo run -p fala-inject --example foreground_app -- chrome`

**C8** - `scripts/check-no-tauri-in-crates.sh` sai com 0; a árvore de dependências normais de `fala-inject` para `x86_64-unknown-linux-gnu` não contém `windows-sys` nem `tauri`; a mesma árvore para `x86_64-pc-windows-msvc` contém `windows-sys v0.61` (AC 8, door 2)
Proof: `scripts/check-no-tauri-in-crates.sh && ! cargo tree -p fala-inject -e normal --target x86_64-unknown-linux-gnu | grep -E 'windows-sys|tauri' && cargo tree -p fala-inject -e normal --target x86_64-pc-windows-msvc | grep -q 'windows-sys v0.61'`

**C9** - `cargo clippy` de `fala-inject` com `--all-targets -- -D warnings` para `x86_64-pc-windows-msvc` sai com 0, compilando o código `cfg(windows)` e o exemplo (AC 9)
Proof: `RUSTC_BOOTSTRAP=1 cargo clippy -Zbuild-std=std,panic_abort --target x86_64-pc-windows-msvc -p fala-inject --all-targets -- -D warnings`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| caminhos válidos do AC 1 (6) | C1, table-driven over all 6 | - |
| caminhos sem nome do AC 2 (4) | C2, table-driven over all 4 | - |
| variantes de `InjectError` (3) | `Unsupported` C6 · `NoForegroundWindow` C6 · `Os` C6 | - |
| plataformas (2) | não-Windows C4 · Windows C7 (prova só roda no Windows, `TODO(windows)`) | - |
| árvore de dependências por alvo (2) | linux-gnu C8 · windows-msvc C8 | - |
| one-way doors do plano (4) | identidade C1 · dependências C8 · API pública C5 · Linux sem detecção C4 | - |

- Nenhum check reivindica comportamento do código Windows em execução além do C7; o C9 prova só que ele compila e passa no lint
- O C4 e o C5 são `#[cfg(not(windows))]`: no job windows do CI eles não compilam, e nada os substitui lá

## Swept

- validation: C1, C2
- failure modes: C4, C6
- idempotency: n/a - leitura sem efeito colateral; chamar duas vezes só consulta o sistema duas vezes
- authorization: n/a - nenhum chamador externo; processo elevado em foco cai em `Os` (C6) e vira app desconhecido
- concurrency: n/a - função sem estado; cada chamada consulta o sistema na hora
- data lifecycle: n/a - nada persiste
- dependency failure: C6 (falha de uma chamada Win32 vira `Os` e app desconhecido)
- state transitions: n/a - não há estado
- observability: n/a - o motivo fica no `InjectError` (C4); o log `debug` de `foreground_app()` é extra, sem obrigação

## Handoff

- S1 + S2 = ~5k (lib.rs, foreground.rs, Cargo.toml, exemplo, Cargo.lock), um crate só, bem abaixo do budget de 150k - one builder

- **Boundary:** C1-C3 closed at `ef53e87`; C4-C6, C8 and C9 closed by the commit that adds this line; C7 open until the manual Windows run (`TODO(windows)`)
- **Settled mid-build:** none; the Windows cross-check target moved from `-gnu` to `-msvc` before the checks were written (the `-gnu` std build needs `x86_64-w64-mingw32-dlltool`, absent here)
- **Abandoned:** none
