# release-runbook-nsis checks

Profile: light
Plan: `.specs/features/release-runbook-nsis/plan.md`

12 checks do plano (AC 1-12) + 1 de derivação (a Landing) em 2 slices · 2 one-way doors · 0 open

Todas as provas de conteúdo e de configuração rodam em `scripts/ci/test_release_config.py` (unittest, só stdlib; `-k <nome>` escolhe o
teste). As funções que checam a configuração recebem o JSON como argumento, para que os checks de falha (C10) as exerçam com uma
configuração adulterada em memória. A prova de que o `tauri-build` aceita o JSON e de que o passo roda no CI fica no PR (`gh pr checks`,
job `rust`), não nestes checks.

## Checks

### S1 - o roteiro de release existe e é fiel à ADR-0008 · 2 files · 12 KB · ~3k

**C1** - `docs/RELEASE.md` tem exatamente estes `##`, nesta ordem: `Antes de começar`, `Versão e changelog`, `Construir o instalador`, `Sem assinatura: o aviso do SmartScreen`, `Validar no Windows`, `Publicar`, `Updater`, `Retirar uma versão` (AC 1)
Proof: `python3 scripts/ci/test_release_config.py -k test_release_md_sections_in_order`

**C2** - A seção `Construir o instalador` contém o comando `bun run tauri build --bundles nsis` e o caminho `target\release\bundle\nsis\Fala_<versão>_x64-setup.exe` (AC 2)
Proof: `python3 scripts/ci/test_release_config.py -k test_release_md_build_command`

**C3** - A seção `Validar no Windows` contém `TODO(windows)` pelo menos uma vez (AC 3)
Proof: `python3 scripts/ci/test_release_config.py -k test_release_md_marks_windows_only_steps`

**C4** - A seção `Updater` contém `desligado`, `plugins.updater`, `createUpdaterArtifacts`, `ADR-0008` e a frase `nunca no repositório` (AC 4)
Proof: `python3 scripts/ci/test_release_config.py -k test_release_md_updater_section`

**C5** - A seção `Versão e changelog` contém `mantenedor`, `CHANGELOG.md` e `chore(release): vX.Y.Z` (AC 5)
Proof: `python3 scripts/ci/test_release_config.py -k test_release_md_version_section`

**C6** - A seção `Sem assinatura` do `RELEASE.md` contém `Mais informações`, `Executar assim mesmo` e `README.md`, e o `README.md` contém `Mais informações` e `Executar assim mesmo` (AC 6)
Proof: `python3 scripts/ci/test_release_config.py -k test_smartscreen_steps_in_release_md_and_readme`

### S2 - o instalador NSIS é explícito, sem assinatura, e a configuração é travada · 3 files · 10 KB · ~3k

**C7** - `bundle.targets` de `tauri.conf.json` é exatamente `["nsis", "deb", "rpm", "appimage"]` (AC 7)
Proof: `python3 scripts/ci/test_release_config.py -k test_bundle_targets`

**C8** - `bundle.windows.nsis` tem `installMode` `"currentUser"`, `languages` `["PortugueseBR", "English"]`, `displayLanguageSelector` `false` e `template` `"nsis/installer.nsi"` (AC 8, Landing porta 1)
Proof: `python3 scripts/ci/test_release_config.py -k test_nsis_config`

**C9** - `bundle.windows.webviewInstallMode` é `{"type": "downloadBootstrapper"}` e `bundle.windows` não tem `certificateThumbprint`, `signCommand` nem `tsp` (AC 9)
Proof: `python3 scripts/ci/test_release_config.py -k test_windows_unsigned_with_bootstrapper`

**C10** - A configuração atual tem `plugins.updater.pubkey` `""`, `plugins.updater.endpoints` `[]` e `bundle.createUpdaterArtifacts` `false`; e a função que checa isso rejeita uma cópia em memória com a `pubkey` preenchida, uma com um endpoint e uma com `createUpdaterArtifacts` `true` (AC 10)
Proof: `python3 scripts/ci/test_release_config.py -k test_updater_stays_off`
Proof: `python3 scripts/ci/test_release_config.py -k test_guard_rejects_enabled_updater`

**C11** - O job `rust` de `ci.yml` tem um passo `run: python3 scripts/ci/test_release_config.py` sem `continue-on-error` (AC 11)
Proof: `python3 scripts/ci/test_release_config.py -k test_ci_runs_release_config_test`

**C12** - O diff `origin/main...HEAD` só toca `.github/`, `scripts/`, `docs/RELEASE.md`, `apps/desktop/tauri.conf.json` e `.specs/features/release-runbook-nsis/`, e a linha `"version"` de `tauri.conf.json` não aparece no diff (AC 12)
Proof: `test -z "$(git diff --name-only origin/main...HEAD | grep -vE '^(\.github/|scripts/|docs/RELEASE\.md$|apps/desktop/tauri\.conf\.json$|\.specs/features/release-runbook-nsis/)')"`
Proof: `! git diff -U0 origin/main...HEAD -- apps/desktop/tauri.conf.json | grep -E '^[+-][[:space:]]*"version"'`

**C13** - `productName` é `Fala`, `identifier` é `br.com.augusto.fala` e `bundle.targets` contém `nsis` (Landing porta 2)
Proof: `python3 scripts/ci/test_release_config.py -k test_identity_unchanged`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| seções do `RELEASE.md` (8) | Antes de começar C1 · Versão e changelog C1 + C5 · Construir o instalador C1 + C2 · Sem assinatura C1 + C6 · Validar no Windows C1 + C3 · Publicar C1 · Updater C1 + C4 · Retirar uma versão C1 | - |
| `scripts/ci/test_release_config.py` exit codes (2) | 0 C1-C11, C13 · 1 C10 (adulteração em memória) | - |
| alvos de bundle (4) | `nsis` C7 · `deb` C7 · `rpm` C7 · `appimage` C7 | - |
| campos de assinatura proibidos em `bundle.windows` (3) | `certificateThumbprint` C9 · `signCommand` C9 · `tsp` C9 | - |
| campos que mantêm o updater desligado (3) | `pubkey` C10 · `endpoints` C10 · `createUpdaterArtifacts` C10 | - |
| Landing portas (2) | 1 `installMode` C8 · 2 identidade e artefato C13 | - |
| triggers do passo no CI (1) | job `rust` C11 | - |

- Claims que nomeiam um valor: C2, C4, C5, C6, C7, C8, C9, C10, C13 - cada um lê o arquivo real do repo (ou, em C10, a cópia adulterada) pela fronteira de arquivo
- Nenhum outro check afirma mais que o caso único que sua prova exercita
- C1 prova existência e ordem das seções `Antes de começar`, `Publicar` e `Retirar uma versão`, não o conteúdo delas: registrado como lacuna, o conteúdo é revisão humana do diff
- O que só o Windows prova (o `.exe` instala, desinstala limpo, mostra o aviso do SmartScreen, o `tauri build` aceita `targets` sem `.msi`) não é check: está em `TODO(windows)` no `RELEASE.md`

## Swept

- validation: C7, C8, C9, C10 (a configuração é conferida campo a campo)
- failure modes: C10 (a configuração adulterada é rejeitada); arquivo ausente falha o teste com o caminho, pelo `read_text` do `unittest`
- idempotency: n/a - o teste só lê arquivos; o `RELEASE.md` descreve passos que o mantenedor repete a cada release
- authorization: C9 (nenhuma assinatura configurada), C10 (updater desligado); nenhum segredo no workflow
- concurrency: existing - o `concurrency` do workflow `ci` (`cancel-in-progress`) já cobre execuções simultâneas
- data lifecycle: n/a - nada persiste; o instalador só existe quando o mantenedor o constrói
- dependency failure: n/a - sem rede nem serviço externo no teste; o `tauri-build` que valida o JSON roda no job `rust` do PR
- state transitions: n/a - sem máquina de estados
- observability: C10 (a asserção nomeia o campo e o valor encontrado, via mensagem do `assertEqual`)

## Out of scope

Herdado do plano (gerar o `.exe`, workflow de release, ligar o updater, lista de licenças no `NOTICE.md`, versão e changelog, assinatura, `build-windows.md`, README).

## Handoff

- S1 = ~3k (`RELEASE.md` novo, ~8 KB, e o teste); S2 = ~3k (`tauri.conf.json` 2,5 KB, `ci.yml` 6 KB, o teste ~6 KB): total ~6k, abaixo do budget de 150k - one builder
- Mechanism: one builder (cabe; sem pergunta). Confirmed? y — delegado pelo Augusto em 2026-10-02, decidido pelo painel
