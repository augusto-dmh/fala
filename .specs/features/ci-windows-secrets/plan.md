# ci-windows-secrets — CI Windows que testa de verdade e scanner de segredos

## Problem

O job `windows` de `.github/workflows/ci.yml` só roda `cargo clippy`: nenhum teste de `crates/*` ou
`apps/cli` jamais executa no Windows, que é o alvo da fase 1 (ADR-0007). Além disso o job só roda
em push para `main` e em dispatch (`if: github.event_name != 'pull_request'`), então uma regressão
Windows só aparece depois do merge. As trilhas A, B e C desta rodada escrevem lógica em `crates/*`
sem acesso a uma máquina Windows; hoje nada a exercita lá.

A ADR-0008 diz literalmente "CI roda scanner de segredos" e "um teste de que nenhum arquivo
versionado contém padrão de chave". O repo é público e o produto vai guardar chaves de Gemini,
ElevenLabs e Claude dos usuários; `grep -ril "gitleaks|trufflehog"` no repo não acha nada: o
scanner não existe e a obrigação está descumprida. O comentário atual do job, "Windows minutes
cost double on private repos", está obsoleto: o repo é público (`gh repo view`: `isPrivate: false`).

Evidência que a fonte não dá: não há incidente de vazamento nem tempo de CI medido; o motivo é a
obrigação da ADR e o fato de o primeiro `.exe` depender dela (`docs/RELEASE.md`, trilha H).

Quando isto fechar: todo PR mostra o resultado de `cargo test` no Windows e um job `secrets` que
varre o histórico inteiro e prova, a cada execução, que sabe reconhecer uma chave.

## Flow

Reusa o job `windows` que já existe (toolchain, Vulkan SDK, rust-cache) em vez de criar um segundo
job Windows, e o padrão `scripts/check-*.sh` + `scripts/ci/` já usado por `check-brand.sh` e
`stage-transcribe-libs.sh`.

1. PR, push em `main` ou dispatch -> job `windows` (exists) - remove o `if`, mantém clippy, roda `cargo test` (sem `-p`: `default-members` = `crates/*` + `apps/cli`)
2. PR ou push em `main` -> job `secrets` (new, door 1) - `actions/checkout@v5` com `fetch-depth: 0`
3. `scripts/ci/install-gitleaks.sh` (new, door 1) - baixa o tarball fixado, confere o sha256, instala o binário num diretório do `PATH` do job
4. `scripts/ci/test-check-secrets.sh` (new, no door - placement per conventions) - monta dois repos git temporários (com e sem chave sintética) e exige exit 1 e exit 0 de `check-secrets.sh`
5. `scripts/check-secrets.sh` (new, door 2) - `gitleaks git` sobre o histórico com `.github/gitleaks.toml`, saída redigida; exit 0 limpo, 1 vazamento, 2 `gitleaks` ausente

## Impact

| Front | What changes |
| --- | --- |
| CI | o check `windows` passa a aparecer em todo PR (hoje só em push/dispatch); PRs esperam a build nativa Windows (CMake + MSVC + Vulkan SDK), amenizada pelo `swatinem/rust-cache`; sem medida prévia do tempo frio |
| CI | novo check `secrets` em todo PR e push em `main`; se existir branch protection com checks obrigatórios, os dois nomes só entram nela por ação humana (configuração do repo, fora daqui) |
| CI | o comentário "Windows minutes cost double on private repos" sai; o repo é público |
| build | nenhuma mudança em `Cargo.toml`, `Cargo.lock`, `deny.toml`; a dependência nova é um binário de CI, não um crate |
| domain | nenhum termo muda de significado |
| stored data | nada a migrar. Baseline medido: `gitleaks 8.30.1 git` com regras default sobre as 1.328 commits de todas as refs locais deste clone achou 0 vazamentos, então não há baseline nem allowlist |

## Relations

`None - no stored-data shape change`

## Surface

Três comandos novos, consumidos pelo CI e por quem roda localmente. Não há rota HTTP.

| Route | In | Out | Status |
| --- | --- | --- | --- |
| `scripts/check-secrets.sh` | nenhum argumento; `gitleaks` no `PATH`; lê o repo git do diretório de trabalho e `.github/gitleaks.toml` | stdout: `ok: no secrets found`, ou os findings do gitleaks (regra, arquivo, linha, segredo redigido); stderr: erros de uso | exit `0` limpo · `1` segredo encontrado · `2` `gitleaks` ausente; não há status HTTP (comando local, códigos 200-599 n/a) |
| `scripts/ci/install-gitleaks.sh` | `[dir]` opcional, default `$HOME/.local/bin`; rede para github.com | stdout: caminho do binário instalado; stderr: diagnósticos | exit `0` instalado · `1` sha256 diferente do fixado ou download falhou · `2` plataforma não suportada (só linux x64); não há status HTTP (comando local, códigos 200-599 n/a) |
| `scripts/ci/test-check-secrets.sh` | nenhum argumento; `gitleaks` no `PATH` | stdout: uma linha `ok` por caso; stderr: o que falhou | exit `0` os dois casos se comportam · `1` algum caso não se comporta; não há status HTTP (comando local, códigos 200-599 n/a) |

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| Dependência nova de CI: o binário gitleaks | em `scripts/ci/install-gitleaks.sh`: `GITLEAKS_VERSION=8.30.1` e `GITLEAKS_SHA256=551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb` (tarball `gitleaks_8.30.1_linux_x64.tar.gz`); o job só chama o script e põe o diretório no `$GITHUB_PATH` | `gitleaks/gitleaks-action@v2`: código de terceiro com `GITHUB_TOKEN` e tag mutável, sem verificação de hash; script de regex próprio: não varre o histórico e só pega o que escrevemos |
| Precedente de configuração do scanner | `.github/gitleaks.toml` com `[extend]` `useDefault = true` e nenhuma entrada de allowlist; qualquer allowlist futura leva comentário com o motivo | `--baseline-path`: o histórico está limpo, e um baseline esconderia um vazamento futuro com o mesmo formato; regras próprias para Gemini/ElevenLabs/Claude: o teste com chaves sintéticas dos três formatos já foi detectado pelas regras default |

- Nada mais nesta mudança é difícil de reverter

## Criteria

### S1: o Windows testa de verdade, antes do merge (P1)

O PR mostra, no check `windows`, o resultado de `cargo test` dos crates e da CLI.

**Acceptance Criteria**

1. WHEN um `pull_request` é aberto ou atualizado, um push chega em `main` ou `workflow_dispatch` é acionado THEN o workflow `ci` SHALL iniciar o job `windows` em `windows-latest`, sem condição `if` que exclua `pull_request`
2. WHEN o passo de clippy do job `windows` termina THEN o job SHALL executar `cargo test` sem `-p` e sem `--workspace`, que cobre exatamente os `default-members` (`crates/*` e `apps/cli`)
3. IF `cargo test` sai com código diferente de zero no job `windows` THEN o job SHALL falhar, sem `continue-on-error`, `--exclude` ou `--skip` que escondam o teste que falhou

**Independent test:** abrir o PR e ler no log do job `windows` a linha `Running` de cada binário de teste de `fala-core`, `fala-cli` etc. e as contagens `test result:`.

### S2: um scanner que prova que enxerga (P1)

Todo PR e todo push em `main` varrem o histórico inteiro atrás de chaves, e o job prova antes que o scanner reconhece uma.

**Acceptance Criteria**

4. WHEN um `pull_request` é aberto ou atualizado, ou um push chega em `main` THEN o workflow `ci` SHALL iniciar o job `secrets` em `ubuntu-24.04`, com checkout `fetch-depth: 0`
5. WHEN `scripts/ci/install-gitleaks.sh` roda THEN ele SHALL instalar o gitleaks 8.30.1 a partir do tarball `gitleaks_8.30.1_linux_x64.tar.gz` cujo sha256 é `551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb`
6. IF o sha256 do tarball baixado difere do fixado THEN `scripts/ci/install-gitleaks.sh` SHALL sair com código 1 sem extrair nem instalar nenhum arquivo
7. WHEN `scripts/ci/test-check-secrets.sh` roda THEN ele SHALL montar em diretórios temporários um repo git com uma chave no formato Gemini (`AIza` mais 35 caracteres) gerada em tempo de execução e um repo sem ela, e SHALL exigir que `scripts/check-secrets.sh` saia com 1 no primeiro e com 0 no segundo
8. WHEN o job `secrets` roda THEN ele SHALL executar `scripts/ci/test-check-secrets.sh` antes de `scripts/check-secrets.sh` sobre o repo, e uma falha do teste SHALL falhar o job
9. IF `scripts/check-secrets.sh` encontra um segredo THEN ele SHALL sair com código 1 e imprimir regra, arquivo e linha do finding com o valor redigido (`--redact`), nunca o segredo em claro
10. WHEN `scripts/check-secrets.sh` não encontra nenhum segredo THEN ele SHALL imprimir `ok: no secrets found` e sair com código 0
11. IF `gitleaks` não está no `PATH` THEN `scripts/check-secrets.sh` SHALL sair com código 2 e imprimir em stderr o comando `scripts/ci/install-gitleaks.sh`
12. WHEN `scripts/check-secrets.sh` roda sobre o histórico de `HEAD` deste PR THEN ele SHALL sair com código 0, sem entrada de allowlist em `.github/gitleaks.toml`
13. The workflow `.github/workflows/ci.yml` SHALL NOT referenciar nenhum `secrets.*` além de `secrets.GITHUB_TOKEN`

**Independent test:** `scripts/ci/install-gitleaks.sh /tmp/x && PATH=/tmp/x:$PATH scripts/ci/test-check-secrets.sh && scripts/check-secrets.sh` localmente, mais o log do job `secrets` no PR.

## Out of scope

| Excluded | Why |
| --- | --- |
| Hook local de gitleaks no `lefthook.yml` | a obrigação da ADR-0008 é o CI; o hook é conveniência e pode vir depois sem mudar o job |
| Rodar os testes do `apps/desktop` no Windows (`cargo test --workspace`) | exige linkar a WebView e o clippy já o compila; fica `TODO(windows)` até a primeira máquina Windows real |
| Tornar `windows` e `secrets` checks obrigatórios (branch protection) | configuração do repositório, ação humana |
| GitHub secret scanning e push protection nativos, upload SARIF | configuração do repositório, não código; não substitui o scan com o self-test |
| Cache do binário gitleaks, scan em runner Windows | o download é de ~6 MB e o scan leva ~6 s; não compensa a complexidade |
| Consertar testes que falharem no Windows por defeito em `crates/*` ou `apps/cli` | fora das fronteiras desta trilha; vira relatório ao Augusto, não `--exclude` |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Escopo do `cargo test` no Windows | `cargo test` sem flags (`default-members`), não `--workspace` | cobre o pedido da trilha (`crates/*` e `apps/cli`), pega crates novos sem editar o workflow e não linka o desktop | y — aprovado pelo Augusto via Lux em 2026-10-02 |
| Teste Windows que falha por defeito de código | o job fica vermelho e o defeito vai no relatório final; não mascaro com `--skip` | esconder a falha derrota o objetivo; consertar código está fora das fronteiras desta trilha | y — aprovado pelo Augusto via Lux em 2026-10-02 |
| Regras do scanner | só as regras default do gitleaks 8.30.1, sobre `gitleaks git` (histórico) | histórico limpo (0 achados); chaves sintéticas dos formatos Gemini, ElevenLabs e Claude foram detectadas pelas regras default | y — aprovado pelo Augusto via Lux em 2026-10-02 |
| Local do arquivo de config | `.github/gitleaks.toml` | `.github/` está dentro das fronteiras desta trilha; a raiz do repo não está | y — aprovado pelo Augusto via Lux em 2026-10-02 |
| Chave do self-test | montada em tempo de execução (prefixo `AIza` concatenado com 35 caracteres gerados), nunca literal no repo | um literal com cara de chave seria um falso positivo do próprio scanner | y — aprovado pelo Augusto via Lux em 2026-10-02 |
| Atualizar o gitleaks | PR deliberado que troca `GITLEAKS_VERSION` e `GITLEAKS_SHA256` juntos | sem bot de atualização; versão e hash revisáveis no diff | y — aprovado pelo Augusto via Lux em 2026-10-02 |
| Plataforma do scanner | só `ubuntu-24.04` (`linux_x64`); o script recusa outra com exit 2 | o histórico git é o mesmo em qualquer runner; um job Windows a mais custaria tempo sem pegar nada novo | y — aprovado pelo Augusto via Lux em 2026-10-02 |

**Open questions:** none - all resolved or logged above.

## Observable

| Surface | Decision | Landing |
| --- | --- | --- |
| command `scripts/check-secrets.sh` | output format and verbosity | AC 9, AC 10 |
| command `scripts/check-secrets.sh` | flags and defaults | n/a - sem flags; a config é fixa em `.github/gitleaks.toml` (AC 12) |
| command `scripts/check-secrets.sh` | exit codes | AC 9, AC 10, AC 11 |
| command `scripts/check-secrets.sh` | what it prints when it fails halfway | AC 11 (gitleaks ausente); falha interna do gitleaks propaga o exit do binário sob `set -e` |
| command `scripts/ci/install-gitleaks.sh` | output format and verbosity | AC 5 (imprime o caminho instalado) |
| command `scripts/ci/install-gitleaks.sh` | flags and defaults | AC 5 (`[dir]` opcional, default `$HOME/.local/bin`) |
| command `scripts/ci/install-gitleaks.sh` | exit codes | AC 5, AC 6 |
| command `scripts/ci/install-gitleaks.sh` | what it prints when it fails halfway | AC 6 (nada instalado quando o hash difere); download interrompido sai com 1 sem deixar binário em `[dir]` |
| command `scripts/ci/test-check-secrets.sh` | output format, flags, exit codes | AC 7 |
| command `scripts/ci/test-check-secrets.sh` | what it prints when it fails halfway | AC 8 (falha do teste falha o job antes do scan real) |
| CI job `windows` | output, exit codes and failing halfway | AC 2, AC 3 |
| CI job `secrets` | output, exit codes and failing halfway | AC 4, AC 8, AC 9 |

## Sources

- `docs/decisions/0008-distribuicao-byok-sem-backend-assinatura-e-updater-adiados.md` - "CI roda scanner de segredos" e o teste de que nenhum arquivo versionado contém padrão de chave
- `docs/decisions/0007-windows-primeiro-linux-gnome-depois-sem-macos.md` - Windows 11 é o alvo; nenhum `cfg(windows)` fora dos crates de plataforma
