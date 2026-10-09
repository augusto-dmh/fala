# AGENTS.md — Fala

Fala é um app desktop de ditado por voz e notas de reunião (Tauri 2 + Rust), fork não oficial do Handy.
Leia `ARCHITECTURE.md` antes de criar ou mover crates ou adicionar dependências.
As decisões estruturais estão em `docs/decisions/`. Para mudar uma, escreva uma ADR nova que substitua a antiga; nunca edite uma ADR aceita.
Produto, pitches e pesquisa ficam fora do repo, em `~/projects/fala-research/` no Linux e `C:\dev\fala-research` no Windows (não versionado aqui).

## Comandos

- `bun install` · `bun run tauri dev` · `bun run tauri build` (rodar na raiz do repo)
- `bun run build` gera `dist/`, que só a build de release (`tauri build`) embute; `cargo check`/`test` não precisam dele.
- `cargo check --workspace` · `cargo test -p <crate>` (prefira o crate afetado)
- `cargo clippy --workspace --all-targets -- -D warnings` · `cargo fmt --all`
- `cargo build` sem `-p` compila só `crates/*` e `apps/cli` (`default-members`), sem WebView
- `cargo run -p fala-cli -- <dictate|meeting|record|bench|key|format|history|import|mcp>` testa o pipeline sem UI (`transcribe` ainda é stub)
- `cargo deny check` · `scripts/check-no-tauri-in-crates.sh` · `scripts/check-brand.sh`
- Frontend: `bun run lint` · `bun run format:check` · `bun run check:translations`

## Invariantes

- Nada em `crates/` depende de `tauri`, nem de forma transitiva. A casca Tauri é `apps/desktop` (ADR-0002).
- O áudio de ditado nunca sai da máquina; só texto vai ao LLM (ADR-0009, ADR-0004).
- Nenhuma chave de API no código, em config versionada ou em log. As chaves vivem no keyring do SO (ADR-0008).
- A gravação de reunião só começa por ação explícita e mostra um indicador enquanto dura (ADR-0005).
- `#[cfg(windows)]`/`#[cfg(target_os = ...)]` só dentro dos crates de plataforma (`hotkey`, `audio`, `inject`) e de `apps/desktop` (ADR-0007).
- Nenhuma marca do Handy. `scripts/check-brand.sh` lista as únicas referências permitidas (ver `NOTICE.md`).

## Estilo

- Rust: sem `unwrap`/`expect` fora de testes. Em `crates/*` e `apps/cli` o clippy nega os dois (`[workspace.lints]`).
- Erros: `thiserror` nos crates, `anyhow` nos apps. `Result` em vez de panic.
- Log com `log`/`tracing`, nunca `println!` em código de app. Nada de conteúdo ditado em nível acima de `debug`.
- `apps/desktop` é código herdado do Handy (edition 2021, sem os lints do workspace, com alguns lints liberados no próprio `Cargo.toml`). Não reformate nem refatore o que você não precisa tocar; ao migrar código para `crates/`, remova o `allow` correspondente.
- UI: toda string visível passa por i18next (`src/i18n/locales/{pt,en}`); o ESLint barra literal em JSX. pt-BR é o idioma-fonte.
- `src/bindings.ts` é gerado pelo `tauri-specta` no `tauri dev`. Não edite à mão.

## Quirks de ambiente

- O projeto roda em duas máquinas: Ubuntu 25.04 (GNOME Wayland, 14 GB de RAM) e Windows 11 (Alienware 16, 32 GB), que é o alvo da fase 1. Confira em qual você está antes de seguir um handoff. No Linux, o que só dá para verificar no Windows fica marcado `TODO(windows)`. No Windows, esses itens são trabalho a fazer: o handoff da rodada Windows, em `fala-research`, lista a ordem.
- Linux precisa dos pacotes de sistema listados em `docs/dev/build-windows.md` (apêndice Linux) para compilar `apps/desktop` (webkit2gtk, Vulkan/glslc, OpenSSL, evdev, gtk-layer-shell).
- Windows: `VK_LOADER_LAYERS_DISABLE=~implicit~` é definido pelo app (opt-out: `FALA_KEEP_VULKAN_IMPLICIT_LAYERS=1`). Se aparecer erro de path-limit (`MSB3491`, `FTK1011`), use um `CARGO_TARGET_DIR` curto (`C:\f`).
- O modelo VAD (`apps/desktop/resources/models/silero_vad_v4.onnx`) é versionado. Os modelos de ASR são baixados no primeiro uso para a pasta de dados do app, nunca para o repo.
- Os modelos ainda vêm do CDN do upstream (`blob.handy.computer`); não adicione URLs novas para ele.
- O updater está desligado (`plugins.updater` vazio em `tauri.conf.json`). Não o religue sem `docs/RELEASE.md` (ADR-0008).
- macOS não é alvo. Não gaste tempo com código `target_os = "macos"` além de mantê-lo compilando onde já existe.

## Commits e PRs

Canônico: `CONTRIBUTING.md`. Aqui só o que o agente erraria sem ler.

- Nunca commite em `main`; trabalhe em `<type>/<slug>` (mesmo `type` do commit). `bunx lefthook install` uma vez por clone.
- Mensagem em inglês: `type(scope): description`, minúscula, imperativo, ≤ 72 chars; corpo explica o porquê. Tipos: build chore ci docs feat fix perf refactor revert style test. Escopo = crate ou `ui`/`i18n`.
- Um commit por obrigação provada do plano; testes no mesmo commit; fixups via `--amend`/`rebase -i` antes do PR.
- Não escreva `Co-Authored-By`, "Generated with" nem link de sessão: o trailer `Assisted-by: Claude Code` vem de `.claude/settings.json`. Nunca adicione `Signed-off-by`.
- Não cite caminhos de spec locais nem IDs internos de tarefa do skill; ADRs (`docs/decisions/NNNN`), issues e fases do `ROADMAP.md` podem ser citados.
- PR: `gh pr create --title "<mesma regra do commit>" --body-file <arquivo>` com Problem / Change / Verification / AI assistance preenchidos com fatos, em inglês, sem TODO e sem os comentários do template. Título de feat/fix na voz de quem usa.
- Verification lista comandos exatos e contagens; o que não foi testado, diz.
- Não faça merge, push forçado, nem mude versão ou `CHANGELOG.md`: release é humano. Remova o worktree quando a branch integrar.
- Criou, renomeou ou removeu um crate: atualize o Code Map do `ARCHITECTURE.md` no mesmo commit.
- Mudou algo que contradiz uma ADR: pare e proponha uma ADR nova. Conflito entre documentos: ADR > ARCHITECTURE > AGENTS > CONTRIBUTING, e o perdedor é corrigido no mesmo commit.

## Fluxo por feature

- Feature com mais de ~3 arquivos ou porta de uma via: skill `tlc-spec-lean` (plan → checks → build → verify), artefatos em `.specs/`.
- Se dá para descrever o diff numa frase, pule o plano e escreva só os checks.
- Uma feature termina com evidência (`verification.md` do Verifier), não com afirmação.

## tlc-spec-lean

profile: light
budget: 150k
