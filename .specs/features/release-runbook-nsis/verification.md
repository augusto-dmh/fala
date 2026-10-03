# release-runbook-nsis verification

**Verdict**: PASS
**Profile**: light
**Diff range**: origin/main..HEAD (HEAD 825d0fe; 3 commits: 65c376d plan/checks, 230c1c7 runbook, 825d0fe config). A rodada 1 rodou em a6a1713 e a rodada 2 em f22b702, antes dos autosquash.
**Round**: 3 - scoped
**Verifier**: independent sub-agent (author != verifier)

Escopo desta rodada: o diff do fix (`git diff f22b702 825d0fe`: só `docs/RELEASE.md`, 3 linhas trocadas) e os veredictos abertos da rodada 2 (G7, G8, G9). As provas rodaram de novo por inteiro no HEAD 825d0fe. Cada seção abaixo diz se foi `verified at 825d0fe` ou `carried from f22b702`.

PASS vale para os 13 checks C1..C13. G7, G8 e G9 estão resolvidos; G5 segue como `TODO(windows)`. Um gap menor novo (G10) e uma dependência de ordem de merge (G7, residual) ficam registrados; nenhum derruba um check.

## Binding sources

carried from f22b702 (verified at a6a1713). Profile `light`: o passo 1 roda só sob `ui`. A fonte da obrigação (ADR-0008) e o critério da fase 1 (`ROADMAP.md`) não mudaram desde a rodada 1; os checks C1-C6 e C9-C10 cobrem o `RELEASE.md` antes do primeiro `.exe`, o instalador sem assinatura com aviso no README e o updater desligado com chave privada fora do repo. Sem contradição.

## Checks

verified at 825d0fe.

Gate integral: `python3 scripts/ci/test_release_config.py -v` - 13 tests, OK, cada um listado individualmente. Cada `-k` abaixo foi rodado isolado (`Ran 1 test ... OK`, exit 0). Os 13 `def test_` existem (`rg -n "def test_" scripts/ci/test_release_config.py`, linhas 78 a 151); `git diff f22b702 825d0fe` não toca o arquivo de teste, então os `file:line` da rodada 2 continuam valendo; um seletor sem casamento sai com exit 5 em Python 3.13.3, então não vira verde falso. Os checks C1..C6 passam a ler seções por `release_sections()` reescrita (`:37-56`); ela foi exercitada também em memória, ver G6.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | `RELEASE.md` tem os 8 `##` na ordem | `-k test_release_md_sections_in_order` exit 0 | `scripts/ci/test_release_config.py:79` - `self.assertEqual(list(release_sections()), SECTIONS)` (lista em `:21-30`) | PASS |
| C2 | comando e caminho do NSIS na seção de build | `-k test_release_md_build_command` exit 0 | `scripts/ci/test_release_config.py:82-88` - `assert_contains_all("Construir o instalador", ["bun run tauri build --bundles nsis", r"target\release\bundle\nsis\Fala_<versão>_x64-setup.exe"])` | PASS |
| C3 | `TODO(windows)` em `Validar no Windows` | `-k test_release_md_marks_windows_only_steps` exit 0 | `scripts/ci/test_release_config.py:91` - `assertIn("TODO(windows)", release_sections()["Validar no Windows"])` | PASS |
| C4 | seção `Updater` com 5 termos | `-k test_release_md_updater_section` exit 0 | `scripts/ci/test_release_config.py:94-97` - needles `desligado`, `plugins.updater`, `createUpdaterArtifacts`, `ADR-0008`, `nunca no repositório` | PASS |
| C5 | seção de versão cita mantenedor, changelog e o commit | `-k test_release_md_version_section` exit 0 | `scripts/ci/test_release_config.py:100-102` - needles `mantenedor`, `CHANGELOG.md`, `chore(release): vX.Y.Z` | PASS |
| C6 | passos do SmartScreen no `RELEASE.md` e no README | `-k test_smartscreen_steps_in_release_md_and_readme` exit 0 | `scripts/ci/test_release_config.py:106` - `assert_contains_all("Sem assinatura: ...", steps + ["README.md"])`; `:109` - `assertIn(step, readme, ...)` | PASS |
| C7 | `bundle.targets` exato | `-k test_bundle_targets` exit 0 | `scripts/ci/test_release_config.py:114` - `assertEqual(conf()["bundle"]["targets"], ["nsis", "deb", "rpm", "appimage"])` | PASS |
| C8 | config do NSIS | `-k test_nsis_config` exit 0 | `scripts/ci/test_release_config.py:118-121` - `installMode == "currentUser"`, `languages == ["PortugueseBR", "English"]`, `assertIs(displayLanguageSelector, False)`, `template == "nsis/installer.nsi"` | PASS |
| C9 | bootstrapper explícito e sem campos de assinatura | `-k test_windows_unsigned_with_bootstrapper` exit 0 | `scripts/ci/test_release_config.py:125` - `assertEqual(windows.get("webviewInstallMode"), {"type": "downloadBootstrapper"})`; `:126-127` - `assertNotIn(field, windows, ...)` para `certificateThumbprint`, `signCommand`, `tsp` | PASS |
| C10 | updater desligado e o guarda rejeita adulteração | `-k test_updater_stays_off` exit 0; `-k test_guard_rejects_enabled_updater` exit 0 | `scripts/ci/test_release_config.py:130` - `assertEqual(updater_problems(conf()), [])` (lógica em `:59-69`); `:141` - `assertTrue(updater_problems(mutated), ...)` para pubkey, endpoint e `createUpdaterArtifacts` True (mutações em memória em `:132-140`) | PASS |
| C11 | passo no job `rust` sem `continue-on-error` | `-k test_ci_runs_release_config_test` exit 0 | `scripts/ci/test_release_config.py:157-158` - `assertEqual(len(hits), 1, ...)` e `assertNotIn("continue-on-error", hits[0])`; passo real em `.github/workflows/ci.yml:47-48` (`run: python3 scripts/ci/test_release_config.py`, dentro de `rust:` em `:17`) | PASS |
| C12 | diff só nas fronteiras; `"version"` intocada | as duas provas de shell literais do checks.md, ambas exit 0 | prova 1 (`test -z` sobre `git diff --name-only origin/main...HEAD` filtrado por `grep -vE` com a lista de fronteiras) exit 0; arquivos do diff: `.github/workflows/ci.yml`, `scripts/ci/test_release_config.py`, `docs/RELEASE.md`, `apps/desktop/tauri.conf.json`, `.specs/features/release-runbook-nsis/plan.md` e `checks.md`; prova 2 (`! git diff -U0` do `tauri.conf.json` filtrado por `grep -E` em `^[+-]\s*"version"`) exit 0, sem linha `"version"` no diff | PASS |
| C13 | identidade e `nsis` nos alvos | `-k test_identity_unchanged` exit 0 | `scripts/ci/test_release_config.py:145-147` - `productName == "Fala"`, `identifier == "br.com.augusto.fala"`, `assertIn("nsis", c["bundle"]["targets"])` | PASS |

Nível e amostragem (verified at 825d0fe):

- C1 prova existência e ordem de 8 seções, não o conteúdo de `Antes de começar`, `Publicar` e `Retirar uma versão`: lacuna declarada no checks.md, e é onde ficam G1, G4, G7 e G8.
- C2-C6 são prova de substring em seção; confirmam a presença da afirmação, não a verdade dela.
- C7-C9 e C13 leem o `tauri.conf.json` real; um valor errado derruba o teste.
- C10 prova a rejeição por mutação em memória dos 3 campos do set.
- C11 lê o `ci.yml` real por regex sobre o bloco do job `rust`; que o passo roda no CI é do PR (`gh pr checks`).
- Roda de qualquer cwd (`ROOT = Path(__file__).resolve().parents[2]`, `:15`) e só com stdlib; a rodada 1 confirmou a partir de `/tmp`, e o diff do fix não mexeu em `ROOT`.

## Swept existing

verified at 825d0fe. `concurrency: existing` - `.github/workflows/ci.yml:9-11` tem `group: ${{ github.workflow }}-${{ github.ref }}` e `cancel-in-progress: true`; o `ci.yml` não mudou no fix. A restrição citada existe. As demais linhas `Swept` são `n/a` ou apontam para checks.

## Coverage

carried from f22b702. Perfil `light`: o recompute do `Coverage` não é exigido e não foi feito; a tabela do checks.md foi lida, não recomputada. O fix não adicionou membro a nenhum set.

## Test policy rows

carried from f22b702. O checks.md não tem seção `Test policy`; vale a convenção do repo (`scripts/ci/test_*.py`, stdlib).

## Faults injected

Perfil `light`: a injeção de falhas não é exigida e não rodou, nem na rodada 1 nem nesta. carried from f22b702. A rodada 3 não mexeu em código. Na rodada 2, como substituto barato para a lógica nova de `release_sections()`, rodei em memória (sem tocar o repo) um `RELEASE.md` com `## Publicar` repetido (resultado: `AssertionError: docs/RELEASE.md repeats the section 'Publicar'`) e um com `## Fake` dentro de bloco de código (resultado: lista de seções igual a `SECTIONS`). Não é injeção de falha no sentido do verify.md.

## Gate

verified at 825d0fe.

`python3 scripts/ci/test_release_config.py -v` - 13 passed, 0 failed.
`bunx prettier --check docs/RELEASE.md apps/desktop/tauri.conf.json .github/workflows/ci.yml .specs/features/release-runbook-nsis/` - "All matched files use Prettier code style!", exit 0.
Schema do Tauri: `apps/desktop/tauri.conf.json` não mudou de conteúdo desde a rodada 1 (carried from f22b702, verified at a6a1713: 0 erros contra `schema.tauri.app/config/2`, controle negativo rejeitado).
Não rodei `cargo`, `clippy` nem `tauri build` (fronteira da trilha).

## Gaps

Estado dos gaps contra o novo texto (verified at 825d0fe):

- G1, G2, G3, G4, G6 - resolvidos na rodada 2 (carried from f22b702; as linhas tocadas não mudaram no fix desta rodada).
- G5 (bundler aceita alvos de outra plataforma) - mantido como `TODO(windows)` em `docs/RELEASE.md:69`, por decisão do orquestrador.
- G7 (scan de segredos inexistente) - **resolvido, com dependência de ordem**. `docs/RELEASE.md:12` agora cita o job `secrets` e `scripts/check-secrets.sh` e diz que vêm do PR #21. Conferi o PR #21 (`gh pr view 21`: estado OPEN, branch `ci/windows-tests-and-secret-scan`; `gh pr diff 21`): ele cria o job `secrets` em `.github/workflows/ci.yml` (com `fetch-depth: 0` e o passo "Secret scan (history)" rodando `scripts/check-secrets.sh`) e o arquivo `scripts/check-secrets.sh` (exit 0 limpo, 1 segredo, 2 gitleaks ausente). O que a linha afirma corresponde ao que o #21 entrega. Mas em `origin/main` de hoje o job e o script não existem (`ls scripts/check-secrets.sh` falha), então a linha é falsa na base se este PR entrar antes do #21. Julgo a referência como dependência declarada e aceitável, não como afirmação falsa, desde que o #21 entre primeiro; severidade **minor** residual. Correção de processo: mergear o #21 antes deste PR, ou registrar a dependência no corpo do PR.
- G8 (`--target`) - **resolvido**: `docs/RELEASE.md:79` diz `--target <sha completo>`; a frase em `:76` explica que a tag é criada na publicação no commit de `--target`.
- G9 (release imutável) - **resolvido**: `docs/RELEASE.md:95` agora avisa que o GitHub pode recusar mexer num release publicado se o repositório passar a usar releases imutáveis.

Gaps abertos (nenhum derruba um check):

10. **minor, redação** - `docs/RELEASE.md:95` termina com "nesse caso, a única saída é publicar a versão seguinte", logo depois de oferecer "ou apague-o". O manual do `gh release create` só diz que, com imutabilidade, tag e assets de release publicado não podem ser modificados ou apagados; não diz que o release em si não possa ser apagado, e não verifiquei. "Única saída" afirma mais do que o manual sustenta. Correção: "nesse caso, a tag e os assets ficam; publique a versão seguinte e avise" (sem "única").
