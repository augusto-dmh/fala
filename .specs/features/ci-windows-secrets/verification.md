# ci-windows-secrets verification

**Verdict**: PASS
**Profile**: light
**Diff range**: origin/main..56d16c625bf40e5a6c811545c72b5866063897aa (3 commits: eb43014, 81debfe, 56d16c6)
**Round**: 1 - full
**Verifier**: independent sub-agent (author != verifier)

Perfil `light` (o mesmo de `checks.md`): este relatório não executa injeção de falhas nem recomputa o Coverage; as seções correspondentes dizem isso. Todas as provas foram rodadas por mim no HEAD, com `gitleaks 8.30.1` instalado por `scripts/ci/install-gitleaks.sh` num diretório do scratchpad (download real de github.com, hash conferido).

## Binding sources

Profile `light`: o passo 1 (`ui`) não roda. O plano não marca fonte vinculante de UI. A ADR-0008 (fonte da obrigação) foi aberta e lida: "CI roda scanner de segredos" e "um teste de que nenhum arquivo versionado contém padrão de chave".

| Source | Opened | Contradiction | Uncovered |
| --- | --- | --- | --- |
| ADR-0008 (Obrigatório + Confirmação) | yes - `docs/decisions/0008-...md:21,24` | none | - |

## Checks

Proofs run (uma invocação por alvo, cada teste aparece individualmente):
- `python3 scripts/ci/test_ci_workflow.py -v` - 8 tests, OK, cada um listado `... ok`; mais cada `-k` do checks.md rodado isolado: `Ran 1 test ... OK` nos 8.
- `scripts/ci/test-install-gitleaks.sh` - `ok installs_pinned_version`, `ok pins_literal`, `ok hash_mismatch_exits_1_installs_nothing`, `ok unsupported_platform_exits_2`, exit 0.
- `scripts/ci/test-check-secrets.sh` - `ok leaky_repo_exits_1_clean_repo_exits_0`, `ok leak_output_names_rule_file_line_without_secret`, `ok clean_repo_prints_ok`, `ok missing_gitleaks_exits_2`, `ok blind_scanner_fails_selftest`, exit 0.
- `scripts/check-secrets.sh` - `1335 commits scanned ... no leaks found`, `ok: no secrets found`, exit 0.

Os seletores existem (`rg -n`): os 8 `def test_*` em `scripts/ci/test_ci_workflow.py` (linhas 35, 45, 57, 63, 76, 86, 94, 103) e as 9 funções de caso em `scripts/ci/test-install-gitleaks.sh` (12, 18, 24, 34) e `scripts/ci/test-check-secrets.sh` (35, 42, 51, 57, 66). Todos os arquivos são novos neste diff (nenhuma prova resolve para teste intocado).

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | `windows` em `windows-latest`, sem `if`; triggers pull_request/push main/workflow_dispatch | `test_ci_workflow.py -k test_windows_runs_on_every_trigger` exit 0 | `scripts/ci/test_ci_workflow.py:38` - `assertEqual(job["runs-on"], "windows-latest")`; `:39` - `assertNotIn("if", job)`; `:41-43` - `assertIn("pull_request", on)`, `assertEqual(on["push"]["branches"], ["main"])`, `assertIn("workflow_dispatch", on)` | PASS |
| C2 | passo `cargo test` exato, sem flags, depois do clippy | `-k test_windows_runs_plain_cargo_test_after_clippy` exit 0 | `test_ci_workflow.py:47-48` - `assertIn("cargo test", runs)`, `assertEqual(runs.count("cargo test"), 1)`; `:51` - `assertLess(clippy[0], runs.index("cargo test"))` | PASS |
| C3 | nenhum `continue-on-error` no job nem nos passos | `-k test_windows_failure_is_not_masked` exit 0 | `test_ci_workflow.py:59` - `assertNotIn("continue-on-error", job)`; `:61` - `assertNotIn("continue-on-error", step, step)` | PASS |
| C4 | job `secrets` em `ubuntu-24.04`, triggers, checkout@v5 `fetch-depth: 0`, `permissions: contents: read` | `-k test_secrets_job_shape` exit 0 | `test_ci_workflow.py:66` - `assertEqual(job["runs-on"], "ubuntu-24.04")`; `:68` - `assertEqual(job["permissions"], {"contents": "read"})`; `:73-74` - `checkout["uses"] == "actions/checkout@v5"`, `checkout["with"]["fetch-depth"] == 0` | PASS |
| C5 | instala gitleaks que imprime `8.30.1`; literais de versão e sha256 | `test-install-gitleaks.sh -k installs_pinned_version` e `-k pins_literal` (rodados também no lote) exit 0; download real | `test-install-gitleaks.sh:15` - `[ "$("$dest/gitleaks" version)" = "8.30.1" ]`; `:19-21` - `grep -qx 'GITLEAKS_VERSION=8.30.1'` e `grep -qx 'GITLEAKS_SHA256=551f6f...470eb'` | PASS |
| C6 | tarball diferente: exit 1 e destino vazio | `-k hash_mismatch_exits_1_installs_nothing` exit 0 | `test-install-gitleaks.sh:30` - `[ "$status" -eq 1 ]`; `:31` - `[ -z "$(ls -A "$dest")" ]` (destino pré-criado vazio, `:26`) | PASS |
| C7 | self-test sai 0 quando check-secrets dá 1 (leaky, chave Gemini gerada em runtime) e 0 (clean) | `test-check-secrets.sh -k leaky_repo_exits_1_clean_repo_exits_0` exit 0 | `test-check-secrets.sh:14` - chave `"AIza$(python3 ... range(35))"` (em runtime); `:37` - `[ "$status" -eq 1 ]`; `:39` - `[ "$status" -eq 0 ]` | PASS |
| C8 | self-test antes do scan; ambos `run` sem `continue-on-error` | `-k test_secrets_selftest_runs_before_scan` exit 0 | `test_ci_workflow.py:82` - `assertLess(selftest[0], scan[0])`; `:84` - `assertNotIn("continue-on-error", steps[i])`; `:80-81` - exatamente 1 de cada | PASS |
| C9 | leak: exit 1, regra, arquivo, linha, sem segredo | `-k leak_output_names_rule_file_line_without_secret` exit 0 | `test-check-secrets.sh:44` - `[ "$status" -eq 1 ]`; `:45` - `grep -Eq 'RuleID: +[a-z0-9-]+'`; `:46` - `grep -Eq 'File: +gemini\.env'`; `:47` - `grep -Eq 'Line: +1$'`; `:48` - `if grep -qF "$key" <<<"$output"; then fail` | PASS |
| C10 | repo limpo: `ok: no secrets found`, exit 0 | `-k clean_repo_prints_ok` exit 0 | `test-check-secrets.sh:53` - `[ "$status" -eq 0 ]`; `:54` - `grep -qx 'ok: no secrets found'` | PASS |
| C11 | sem gitleaks: exit 2 e stderr com o comando de instalação | `-k missing_gitleaks_exits_2` exit 0 | `test-check-secrets.sh:62` - `[ "$status" -eq 2 ]`; `:63` - `grep -q 'scripts/ci/install-gitleaks.sh'` (stdout+stderr juntos, `:61` `2>&1`; ver gap G4) | PASS |
| C12 | scan do histórico do HEAD sai 0; config só `useDefault`, sem allowlist/`[[rules]]` | `scripts/check-secrets.sh` exit 0 (1335 commits); `-k test_default_rules_and_no_allowlist` exit 0 | `test_ci_workflow.py:96` - `assertIs(cfg["extend"]["useDefault"], True)`; `:97` - `assertEqual(set(cfg), {"extend"}, ...)`; execução real: `no leaks found`, `ok: no secrets found`, exit 0 | PASS |
| C13 | só `secrets.GITHUB_TOKEN` no ci.yml | `-k test_only_github_token_secret` exit 0 | `scripts/ci/test_ci_workflow.py:90` - `assertEqual(found, {"GITHUB_TOKEN"})`; `rg -n 'secrets\.' .github/workflows/ci.yml` acha a única expressão em `.github/workflows/ci.yml:88` (`GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}`, job pré-existente); as outras linhas que casam são nomes de script | PASS |
| C14 | `uname`=Darwin: exit 2 sem baixar nada | `test-install-gitleaks.sh -k unsupported_platform_exits_2` exit 0 | `test-install-gitleaks.sh:41` - `[ "$status" -eq 2 ]`; `:42` - `[ -z "$(ls -A "$dest")" ]`; `:40` - base URL aponta para caminho inexistente | PASS |
| C15 | scanner cego (stub exit 0): self-test sai 1 | `test-check-secrets.sh -k blind_scanner_fails_selftest` exit 0 | `test-check-secrets.sh:67` - stub `exit 0`; `:70` - `CHECK_SECRETS="$work/blind.sh" "$0" -k leaky_repo_...`; `:71` - `[ "$status" -eq 1 ]` | PASS |
| C16 | diff fica em `.github/`, `scripts/`, `.specs/features/ci-windows-secrets/` | `-k test_diff_stays_inside_track` exit 0 | `test_ci_workflow.py:108` - `assertTrue(out, "empty diff ...")`; `:110` - `assertEqual(outside, [])`; `git diff --stat` lista 9 arquivos, todos dentro | PASS |

Nota C13: o regex só olha dentro de `${{ }}`, então nomes como `check-secrets.sh` não contam; reproduzi o conjunto fora do teste e deu `{'GITHUB_TOKEN'}`.

## Coverage

Perfil `light`: o recompute da junção Coverage não é exigido nem foi feito; o que segue é só a checagem de nível e amostragem pedida.

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| (não recomputado - perfil `light`) | - | - | - |

Nível e amostragem (julgamento):
- Exit codes de `check-secrets.sh`, `install-gitleaks.sh` e `test-check-secrets.sh` são provados pela fronteira (processo, exit code, stdout/stderr), no nível certo.
- C5 baixa da rede de verdade (github.com) e confere `version`; o hash mismatch (C6) é offline com `file://`. Boa amostragem. Dependência de rede nomeada no cabeçalho do teste.
- O self-test só cobre o formato Gemini (lacuna declarada em `checks.md`, linha 85): ElevenLabs e Claude não têm prova no CI. Aceito como declarado.
- C14 prova a plataforma com um shim de `uname`; só o ramo `uname -s`, não o `uname -m` (ver G5).

### Swept `existing`

- `concurrency: existing`: `.github/workflows/ci.yml:9-11` - `group: ${{ github.workflow }}-${{ github.ref }}` e `cancel-in-progress: true`. Existe e cobre execuções simultâneas por ref. Nota: um push em `main` seguido de outro cancela o scan do primeiro (o histórico do segundo contém o primeiro, então nada se perde). Os scripts usam `mktemp -d` próprios (`test-check-secrets.sh:8`, `install-gitleaks.sh:22`). Confirmado.

## Test policy rows

`checks.md` não carrega linhas `Test policy`: sem seção a julgar.

## Faults injected

Perfil `light`: injeção de falhas não roda (nem é exigida). Nenhum mutante foi injetado; a árvore real não foi mutada.

## Gaps (todos com prova verde; nenhum contradiz um check)

| # | Severidade | Gap | Evidência |
| --- | --- | --- | --- |
| G1 | medium (não verificável aqui) | O `cargo test` do job `windows` roda o crate `apps/cli` (e `transcribe-cpp`) com backends dinâmicos (`dynamic-backends`, DLL separada; `scripts/ci/stage-transcribe-libs.sh:4-9`). Se o executável de teste no Windows não achar a DLL, o job fica vermelho por ambiente, não por defeito de código. Nenhum check prova que o teste Windows roda; o plano declara isso (prova fica em `gh pr checks`). Passo `test` sob PowerShell: `cargo test` simples é válido em pwsh e o wrapper do Actions propaga `$LASTEXITCODE`, igual ao passo clippy existente. | `.github/workflows/ci.yml:148-150`; `apps/cli/Cargo.toml:20-21` |
| G2 | low | `check-secrets.sh` chamado por nome sem barra (`cd scripts && bash check-secrets.sh`) sai com 1 e a mensagem `cd: check-secrets.sh: Não é um diretório`: `${BASH_SOURCE[0]%/*}` não remove nada sem `/`. Sai com 1 ("segredo encontrado") por erro de caminho, não por achado; o CI chama `scripts/check-secrets.sh`, então não dispara lá. Reproduzi. | `scripts/check-secrets.sh:14` - `script_dir=$(cd "${BASH_SOURCE[0]%/*}" && pwd)` |
| G3 | low | `test_ci_workflow.py` (incluindo `test_diff_stays_inside_track`) e `test-install-gitleaks.sh` não são chamados por nenhum job (`rg` em `.github`, `lefthook.yml`, `CONTRIBUTING.md`, `AGENTS.md` só acha `test-check-secrets.sh` e `check-secrets.sh`). Eles são provas de verificação locais; uma regressão no workflow (por exemplo remover `fetch-depth: 0`) não faz o CI falhar. `test_diff_stays_inside_track` também quebra depois do merge (`origin/main...HEAD` vazio dispara `assertTrue(out, ...)` na linha 108). | `.github/workflows/ci.yml:169,172`; `test_ci_workflow.py:108` |
| G4 | low | `missing_gitleaks_exits_2` junta stdout e stderr (`2>&1`), então não prova que a dica de instalação vai para stderr, como diz AC 11 e a Surface. O script a escreve em stderr (`check-secrets.sh:9`, `>&2`); só o teste não distingue. | `test-check-secrets.sh:61`; `check-secrets.sh:9` |
| G5 | low | C14 só exerce `uname -s`; o ramo `uname -m != x86_64` (Linux arm64) não tem prova. | `install-gitleaks.sh:17`; `test-install-gitleaks.sh:37` |
| G6 | info | Retirado: eu supus que o workflow não usava `secrets.GITHUB_TOKEN`; usa, em `.github/workflows/ci.yml:88`, e o teste está correto. O assert por igualdade falharia se o job que usa o token fosse removido (acoplamento a um job alheio), mas é só isso. | `test_ci_workflow.py:88-90` |
| G7 | info | Entropia do self-test: a chave sintética é aleatória; em 200 000 amostras a entropia de Shannon mínima foi 4,12 (nenhuma abaixo de 4); flake desprezível. | simulação local |
| G8 | info | Obrigação da ADR-0008 ("CI roda scanner de segredos" + teste de que nenhum arquivo versionado contém padrão de chave): cumprida por `secrets` (scan do histórico completo, `fetch-depth: 0`, self-test antes) em todo PR/push. O job windows depende de CI real (tempo frio de build sem medida, declarado no Impact). Saída do scan não vaza o segredo (`--redact`, C9). |  |

## Gate

`python3 scripts/ci/test_ci_workflow.py -v` - 8 passed, 0 failed · `scripts/ci/test-install-gitleaks.sh` - 4 passed, 0 failed · `scripts/ci/test-check-secrets.sh` - 5 passed, 0 failed · `scripts/check-secrets.sh` - 0 findings em 1335 commits. Total 17 provas, 0 falhas.
