# release-runbook-nsis — runbook de release e instalador NSIS sem assinatura

## Problem

A ADR-0008 diz que `docs/RELEASE.md` precisa existir antes do primeiro `.exe`, e o ROADMAP fecha a fase 1 só com "`docs/RELEASE.md` escrito e primeiro `.exe` distribuído". O arquivo não existe (`git ls-files docs | grep -i release` não acha nada). `AGENTS.md` e `NOTICE.md` já o citam como pré-condição, então hoje duas regras apontam para um documento ausente.

A seção `bundle.windows.nsis` de `apps/desktop/tauri.conf.json` só aponta o template herdado do Handy. Todo o resto é default implícito do Tauri: modo de instalação, idiomas do instalador, instalação do WebView2. O modo de instalação é difícil de reverter: depois que um colega instala em um modo, trocar para o outro deixa duas instalações em registros diferentes. `bundle.targets` é `"all"`, que no Windows também gera um `.msi` que ninguém pediu. Nada garante que a configuração siga sem assinatura e com o updater desligado, que é o que a ADR-0008 decide: só um parágrafo no `AGENTS.md`.

O aviso do SmartScreen já está no README (seção "Instalação (Windows 11)"), como a ADR manda.

Evidência que a fonte não dá: não há instalador testado, nem máquina Windows nesta rodada, nem relato de colega travado na instalação. Tudo que só dá para saber rodando o `.exe` fica `TODO(windows)`.

Quando isto fechar: quem for fazer o primeiro release segue um roteiro único, a configuração do instalador está explícita e travada por um teste no CI, e cada passo que só o Windows valida está marcado.

## Flow

Reusa o padrão `scripts/ci/test_*.py` aberto pela trilha G e o job `rust` do CI. Nenhum módulo novo de produto.

1. Mantenedor decide lançar -> lê `docs/RELEASE.md` (new, door 2) - pré-condições, versão e changelog (humano), build, SmartScreen, validação `TODO(windows)`, publicação, updater, retirada
2. Mantenedor roda `bun run tauri build --bundles nsis` no Windows -> `tauri.conf.json` seção `bundle` (exists, door 1) - NSIS explícito, sem assinatura; `target\release\bundle\nsis\Fala_<versão>_x64-setup.exe`
3. PR com mudança em `tauri.conf.json` ou `docs/RELEASE.md` -> job `rust` -> `scripts/ci/test_release_config.py` (new, no door - placement per conventions) - falha se a configuração sair do que a ADR-0008 decide
4. Colega baixa o `.exe` -> SmartScreen avisa -> README (exists, não muda) - passos "Mais informações" e "Executar assim mesmo"

## Impact

| Front | What changes |
| --- | --- |
| build | só `bundle.targets` e `bundle.windows` do `tauri.conf.json`; chaves `nsis`/`windows` são ignoradas fora do Windows; as seções `deb`, `rpm`, `appimage` e `macOS` não mudam. O `tauri-build` valida este arquivo na compilação de `apps/desktop`, então o job `rust` do CI (clippy do workspace) é quem prova que o JSON é aceito |
| build | `targets` deixa de ser `"all"`: no Windows `tauri build` passa a gerar só NSIS (sem o `.msi`, que exige WiX e não é pedido); no Linux continua gerando `deb`, `rpm` e `appimage`, para a verificação da fase 3 |
| CI | um passo novo, de segundos, no job `rust` (Python 3 e PyYAML dispensados: só `json` e `re` da stdlib) |
| domain | nenhum termo muda de significado |
| stored data | nada a migrar. Nenhum instalador foi distribuído ainda, então a escolha do modo de instalação não afeta instalação existente; depois do primeiro `.exe` ela afeta |
| updater | nada muda: `plugins.updater` segue vazio e `createUpdaterArtifacts` segue `false`; o teste passa a travar os dois |

## Relations

`None - no stored-data shape change`

## Surface

Um comando novo e um documento. Não há rota HTTP.

| Route | In | Out | Status |
| --- | --- | --- | --- |
| `python3 scripts/ci/test_release_config.py [-k <teste>]` | nenhum argumento; roda da raiz do repo; lê `apps/desktop/tauri.conf.json`, `docs/RELEASE.md`, `README.md` | `unittest` padrão: um `ok` por teste em stderr e o resumo; a asserção que falha nomeia o campo e o valor encontrado | exit `0` tudo confere · `1` algum teste falha; não há status HTTP (comando local, códigos 200-599 n/a) |
| `docs/RELEASE.md` | lido por quem faz o release | seções fixas, em pt-BR; comandos copiáveis; cada passo que só o Windows valida leva `TODO(windows)` | não há status HTTP (documento, códigos 200-599 n/a) |

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| Modo de instalação do NSIS: depois do primeiro `.exe` distribuído, mudar de modo deixa duas instalações (HKCU e HKLM) | em `bundle.windows.nsis`: `"installMode": "currentUser"` | `perMachine`: pede UAC e administrador, e o colega no notebook corporativo sem admin não instala sozinho (critério de fechamento da fase 1); `both`: pergunta a cada instalação e dobra os caminhos de upgrade para testar |
| Nome do artefato e identidade do instalador: o README já promete `Fala_<versão>_x64-setup.exe` e o `identifier` `br.com.augusto.fala` já existe | `productName` `Fala` e `identifier` não mudam; o `.exe` sai de `bundle.targets` contendo `"nsis"` | renomear o artefato: quebraria o README e o link de quem já baixou; `perMachine`/MSI com UpgradeCode próprio: segunda identidade para manter |

- O resto da configuração do instalador (idiomas, WebView2, compressão) é reversível a cada release e fica no diff

## Criteria

### S1: o roteiro de release existe e é fiel à ADR-0008 (P1)

Quem for fazer o primeiro release abre um arquivo e sabe o que fazer, em que ordem, e o que só o Windows prova.

**Acceptance Criteria**

1. WHEN alguém abre `docs/RELEASE.md` THEN o arquivo SHALL ter, nesta ordem, as seções `## Antes de começar`, `## Versão e changelog`, `## Construir o instalador`, `## Sem assinatura: o aviso do SmartScreen`, `## Validar no Windows`, `## Publicar`, `## Updater` e `## Retirar uma versão`
2. The seção `## Construir o instalador` SHALL conter o comando literal `bun run tauri build --bundles nsis` e o caminho de saída `target\release\bundle\nsis\Fala_<versão>_x64-setup.exe`
3. WHERE um passo só pode ser verificado numa máquina Windows, `docs/RELEASE.md` SHALL marcá-lo com `TODO(windows)`, e a seção `## Validar no Windows` SHALL ter pelo menos um
4. The seção `## Updater` SHALL dizer que o updater está desligado, citar `plugins.updater` e `createUpdaterArtifacts`, citar a ADR-0008 e proibir a chave privada do updater no repositório
5. The seção `## Versão e changelog` SHALL dizer que versão e `CHANGELOG.md` são do mantenedor e citar o commit `chore(release): vX.Y.Z`
6. The seção `## Sem assinatura: o aviso do SmartScreen` SHALL descrever os passos "Mais informações" e "Executar assim mesmo" e SHALL apontar para o README, e o README SHALL continuar com os dois passos

**Independent test:** `python3 scripts/ci/test_release_config.py` e a leitura do `docs/RELEASE.md` renderizado.

### S2: o instalador NSIS é explícito, sem assinatura, e a configuração é travada (P1)

**Acceptance Criteria**

7. The `bundle.targets` de `apps/desktop/tauri.conf.json` SHALL ser a lista `["nsis", "deb", "rpm", "appimage"]`
8. The `bundle.windows.nsis` SHALL ter `installMode` `"currentUser"`, `languages` `["PortugueseBR", "English"]`, `displayLanguageSelector` `false` e manter `template` `"nsis/installer.nsi"`
9. The `bundle.windows` SHALL ter `webviewInstallMode` `{"type": "downloadBootstrapper"}` e SHALL NOT ter `certificateThumbprint`, `signCommand` nem `tsp`
10. IF `plugins.updater.pubkey` ou `plugins.updater.endpoints` deixam de estar vazios, ou `bundle.createUpdaterArtifacts` deixa de ser `false`, THEN `scripts/ci/test_release_config.py` SHALL falhar
11. WHEN o job `rust` do workflow `ci` roda THEN ele SHALL executar `python3 scripts/ci/test_release_config.py` num passo `run` sem `continue-on-error`
12. The mudança SHALL NOT tocar `Cargo.toml`, `Cargo.lock`, `deny.toml`, `CHANGELOG.md`, nem a versão em `tauri.conf.json`, e SHALL ficar dentro de `.github/`, `scripts/`, `docs/RELEASE.md`, `apps/desktop/tauri.conf.json` e `.specs/features/release-runbook-nsis/`

**Independent test:** `python3 scripts/ci/test_release_config.py`; `bun run build`/`cargo check` do desktop não rodam aqui (fronteira da trilha): o job `rust` do PR prova que o `tauri-build` aceita o JSON.

## Out of scope

| Excluded | Why |
| --- | --- |
| Gerar ou rodar o `.exe`; validar SmartScreen, instalação e desinstalação limpas | exige máquina Windows; vira `TODO(windows)` no `RELEASE.md` |
| Workflow de release no GitHub Actions, tag, `gh release create` | release é humano (`AGENTS.md`); o roteiro descreve, não automatiza |
| Ligar o updater, gerar a chave minisign | decisão aberta nº 5 do design doc; exige `docs/RELEASE.md` e, para ligar, ADR nova |
| Lista de licenças de terceiros no `NOTICE.md` (`cargo deny list`) | arquivo fora das fronteiras; o `RELEASE.md` a lista como pré-condição |
| Mudar versão, `CHANGELOG.md` | release é humano |
| Assinatura de código | ADR-0008 a adia |
| Editar `docs/dev/build-windows.md` (que diz `bun run tauri build` sem `--bundles`) | fora das fronteiras; com `targets` agora sem `.msi` os dois comandos produzem o mesmo no Windows |
| Editar o README | o aviso do SmartScreen já está nele; só o teste o confere |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Modo de instalação | `currentUser` | sem UAC nem administrador: o colega instala sozinho; é o default do Tauri, agora explícito | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| Idiomas do instalador | `["PortugueseBR", "English"]`, sem seletor | pt-BR é o idioma-fonte do produto; o NSIS escolhe o idioma do SO e cai no primeiro da lista | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| Alvos de bundle | `["nsis", "deb", "rpm", "appimage"]` em vez de `"all"` | sem `.msi` no Windows; Linux mantém o que a fase 3 verifica. Se o Tauri recusar alvo de outra plataforma, o CI não pega: `TODO(windows)` | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| WebView2 | `downloadBootstrapper`, o default, explícito | o Windows 11 já traz o runtime; o bootstrapper cobre o resto sem inflar o instalador | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| Comando de build do release | `bun run tauri build --bundles nsis` | só NSIS, mesmo que alguém reponha `targets` | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| Integridade do `.exe` | o `RELEASE.md` manda publicar o SHA-256 (`Get-FileHash`) no corpo do release | sem assinatura, o hash é a única forma de o colega conferir o que baixou | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| Onde mora o teste | `scripts/ci/test_release_config.py`, stdlib, no job `rust` | `scripts/` está nas fronteiras; sem dependência nova | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| Desktop não é compilado localmente | o job `rust` do PR compila o desktop (clippy do workspace) | fronteira da trilha: sem compilar o desktop localmente | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |

**Open questions:** none - all resolved or logged above.

## Observable

| Surface | Decision | Landing |
| --- | --- | --- |
| command `scripts/ci/test_release_config.py` | output format and verbosity | AC 10 (a asserção nomeia campo e valor) |
| command `scripts/ci/test_release_config.py` | flags and defaults | n/a - só o `-k` do `unittest`; sem flags próprias |
| command `scripts/ci/test_release_config.py` | exit codes | AC 10, AC 11 |
| command `scripts/ci/test_release_config.py` | what it prints when it fails halfway | AC 10 (o `unittest` imprime o teste e a asserção; um arquivo ausente falha o teste com o caminho) |
| document `docs/RELEASE.md` | structure and content | AC 1-6 |
| document `docs/RELEASE.md` | what it marks as unverified | AC 3 |
| CI job `rust` | output, exit codes and failing halfway | AC 11 |

## Sources

- `docs/decisions/0008-distribuicao-byok-sem-backend-assinatura-e-updater-adiados.md` - `docs/RELEASE.md` antes do primeiro `.exe`; instalador sem assinatura com aviso do SmartScreen no README; updater desligado, chave nunca no repo
- `ROADMAP.md` - critério de fechamento da fase 1: um colega instalou sozinho, `docs/RELEASE.md` escrito e primeiro `.exe` distribuído
