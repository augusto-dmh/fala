# ci-windows-secrets checks

Profile: light
Plan: `.specs/features/ci-windows-secrets/plan.md`

13 checks do plano (AC 1-13) + 3 de derivação (a Surface e o Impact) em 2 slices · 2 one-way doors · 0 open

Provas de workflow rodam em `scripts/ci/test_ci_workflow.py` (unittest, lê o YAML com PyYAML; `-k <nome>` escolhe o teste); as de
script rodam nos testes de shell ao lado dele (`-k <nome>` escolhe o caso; sem `-k`, todos). A prova de que o CI de fato roda o job fica no PR
(`gh pr checks`), não nestes checks, porque depende da rede e do runner.

## Checks

### S1 - o Windows testa de verdade, antes do merge · 1 file · 5 KB · ~2k

**C1** - O job `windows` de `ci.yml` tem `runs-on: windows-latest` e nenhuma condição `if`, e o workflow dispara em `pull_request`, `push` em `main` e `workflow_dispatch` (AC 1)
Proof: `python3 scripts/ci/test_ci_workflow.py -k test_windows_runs_on_every_trigger`

**C2** - O job `windows` tem um passo `run: cargo test` exato, sem `-p`, `--workspace`, `--exclude` ou `--skip`, depois do passo de clippy (AC 2)
Proof: `python3 scripts/ci/test_ci_workflow.py -k test_windows_runs_plain_cargo_test_after_clippy`

**C3** - Nenhum passo do job `windows` tem `continue-on-error`, e o job não tem `continue-on-error` (AC 3)
Proof: `python3 scripts/ci/test_ci_workflow.py -k test_windows_failure_is_not_masked`

### S2 - um scanner que prova que enxerga · 7 files · 25 KB · ~8k

**C4** - O workflow tem o job `secrets` em `ubuntu-24.04`, disparado em `pull_request` e `push` em `main`, com `actions/checkout@v5` e `fetch-depth: 0`, e `permissions: contents: read` (AC 4)
Proof: `python3 scripts/ci/test_ci_workflow.py -k test_secrets_job_shape`

**C5** - `scripts/ci/install-gitleaks.sh <dir>` instala um `gitleaks` cujo `version` imprime `8.30.1`, e o script contém literalmente `GITLEAKS_VERSION=8.30.1` e `GITLEAKS_SHA256=551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb` (AC 5)
Proof: `scripts/ci/test-install-gitleaks.sh -k installs_pinned_version`
Proof: `scripts/ci/test-install-gitleaks.sh -k pins_literal`

**C6** - Com um tarball de conteúdo diferente do fixado, `install-gitleaks.sh` sai com 1 e deixa o diretório de destino sem nenhum arquivo (AC 6)
Proof: `scripts/ci/test-install-gitleaks.sh -k hash_mismatch_exits_1_installs_nothing`

**C7** - `scripts/ci/test-check-secrets.sh` sai com 0 quando `check-secrets.sh` sai com 1 num repo com chave Gemini sintética (`AIza` + 35 caracteres gerados em tempo de execução) e com 0 num repo sem ela (AC 7)
Proof: `scripts/ci/test-check-secrets.sh -k leaky_repo_exits_1_clean_repo_exits_0`

**C8** - No job `secrets`, o passo que roda `scripts/ci/test-check-secrets.sh` vem antes do que roda `scripts/check-secrets.sh`, e os dois são passos `run` sem `continue-on-error` (AC 8)
Proof: `python3 scripts/ci/test_ci_workflow.py -k test_secrets_selftest_runs_before_scan`

**C9** - Num repo com uma chave sintética, `check-secrets.sh` sai com 1, imprime o nome da regra, o arquivo e o número da linha, e a saída não contém o valor da chave (AC 9)
Proof: `scripts/ci/test-check-secrets.sh -k leak_output_names_rule_file_line_without_secret`

**C10** - Num repo limpo, `check-secrets.sh` imprime `ok: no secrets found` e sai com 0 (AC 10)
Proof: `scripts/ci/test-check-secrets.sh -k clean_repo_prints_ok`

**C11** - Sem `gitleaks` no `PATH`, `check-secrets.sh` sai com 2 e o stderr contém `scripts/ci/install-gitleaks.sh` (AC 11)
Proof: `scripts/ci/test-check-secrets.sh -k missing_gitleaks_exits_2`

**C12** - `check-secrets.sh` sobre o histórico do `HEAD` deste repositório sai com 0, e `.github/gitleaks.toml` tem `useDefault = true` e nenhuma tabela `allowlist` nem `[[rules]]` (AC 12, Landing porta 2)
Proof: `scripts/check-secrets.sh`
Proof: `python3 scripts/ci/test_ci_workflow.py -k test_default_rules_and_no_allowlist`

**C13** - `ci.yml` não contém nenhuma referência `secrets.X` além de `secrets.GITHUB_TOKEN` (AC 13)
Proof: `python3 scripts/ci/test_ci_workflow.py -k test_only_github_token_secret`

**C14** - Em plataforma que não é Linux x64 (`uname` devolvendo `Darwin`), `install-gitleaks.sh` sai com 2 sem baixar nada (Surface, exit 2)
Proof: `scripts/ci/test-install-gitleaks.sh -k unsupported_platform_exits_2`

**C15** - Se `check-secrets.sh` não detecta a chave sintética (scanner substituído por um stub que sai com 0), `test-check-secrets.sh` sai com 1 (Surface, exit 1; é o que impede o self-test de passar com um scanner cego)
Proof: `scripts/ci/test-check-secrets.sh -k blind_scanner_fails_selftest`

**C16** - O diff `origin/main...HEAD` não toca `Cargo.toml`, `Cargo.lock`, `deny.toml` nem nada fora de `.github/`, `scripts/` e `.specs/features/ci-windows-secrets/` (Impact: build; fronteiras da trilha)
Proof: `python3 scripts/ci/test_ci_workflow.py -k test_diff_stays_inside_track`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| `scripts/check-secrets.sh` exit codes (3) | 0 C10 · 1 C9 · 2 C11 | - |
| `scripts/ci/install-gitleaks.sh` exit codes (3) | 0 C5 · 1 C6 · 2 C14 | - |
| `scripts/ci/test-check-secrets.sh` exit codes (2) | 0 C7 · 1 C15 | - |
| `check-secrets.sh` rodado sobre histórico real (1) | HEAD deste repo C12 | - |
| triggers do workflow (3) | `pull_request` C1 · `push` main C1 · `workflow_dispatch` C1 | - |
| triggers do job `secrets` (2) | `pull_request` C4 · `push` main C4 | - |
| formatos de chave provados no CI (1) | Gemini `AIza…` C7 | - |
| Landing portas (2) | 1 dependência gitleaks C5 + C6 · 2 config sem allowlist C12 | - |
| startup config: pino do gitleaks (1 lugar) | `install-gitleaks.sh` C5 | - |
| secrets referenciados no workflow (1) | `GITHUB_TOKEN` C13 | - |

- Claims que nomeiam um exit code ou uma saída: C6, C9, C10, C11, C14, C15 - cada um roda o script de verdade pela fronteira (processo, exit code, stdout/stderr)
- Nenhum outro check afirma mais que o caso único que sua prova exercita
- Só o formato Gemini é provado no CI. ElevenLabs e Claude foram detectados na medição de pesquisa (fora do repo, não é um check) - registrado como lacuna conhecida, não como garantia

## Swept

- validation: C6, C14
- failure modes: C6, C11, C15
- idempotency: n/a - os scripts não gravam estado fora do diretório de destino e podem rodar de novo; `install-gitleaks.sh` sobrescreve o binário
- authorization: C4 (`permissions: contents: read`), C13
- concurrency: existing - o `concurrency` do workflow (`cancel-in-progress`) já cobre execuções simultâneas; os scripts usam diretórios temporários próprios
- data lifecycle: n/a - nada persiste; o binário vive só no runner efêmero
- dependency failure: C6 (download corrompido ou adulterado), C11 (binário ausente)
- state transitions: n/a - sem máquina de estados
- observability: C9 (a saída do scan nomeia regra, arquivo e linha e não vaza o segredo)

## Out of scope

Herdado do plano (hook local, `cargo test --workspace`, branch protection, SARIF, cache do binário, consertar testes que falharem no Windows).

## Handoff

- S1 = ~2k (1 file, `ci.yml` 5 KB); S2 entra em 7 files (~25 KB) em scripts e `.github/`: total ~10k, abaixo do budget de 150k - one builder
- Mechanism: one builder (cabe; sem pergunta). Confirmed? y — delegado pelo Augusto em 2026-10-02, decidido pelo painel
