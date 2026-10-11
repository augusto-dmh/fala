# Release do Fala

Roteiro para gerar e distribuir o instalador do Windows. A [ADR-0008](decisions/0008-distribuicao-byok-sem-backend-assinatura-e-updater-adiados.md) exige este arquivo antes do primeiro `.exe`. Release é decisão e ação do mantenedor: nenhum passo daqui roda sozinho, e nenhum agente de código versiona, empurra tag ou publica.

O instalador sai **sem assinatura de código** (ADR-0008) e **com atualização automática** assinada com minisign ([ADR-0018](decisions/0018-updater-ligado-feed-no-github-release-chave-fora-do-ci.md)). A chave do updater é só do mantenedor: entra no build como variável da sessão e nunca vai para o repositório, workflow, secret do Actions ou log.

TODO(windows): este roteiro foi escrito sem acesso a uma máquina Windows. Nenhum passo de build, instalação ou desinstalação foi executado; os que só ela prova estão marcados `TODO(windows)`. Revisar na primeira execução real.

## Antes de começar

- A barra de bugs "Antes de dar o instalador a um colega" do [`ROADMAP.md`](../ROADMAP.md) está zerada: o caminho principal funciona, nenhum texto ditado se perde, nenhuma chave aparece em log, instalação e desinstalação são limpas e "desfazer edição da IA" funciona.
- O CI de `main` está verde, incluindo o job `secrets` (o scan de segredos do histórico que a ADR-0008 exige, introduzido no PR #21). Antes do primeiro `.exe`, rode também `scripts/check-secrets.sh` na `main` local e confira que sai com 0.
- A lista de licenças de terceiros está em [`NOTICE.md`](../NOTICE.md). Hoje ela é um `TODO` pendente: gerar com `cargo deny list` e anexar antes de distribuir o primeiro `.exe`.
- A máquina de build é um Windows 11 com os pré-requisitos de [`docs/dev/build-windows.md`](dev/build-windows.md) (Rust, Bun, MSVC, CMake, Vulkan SDK).
- `git status` limpo, na `main` atualizada.

## Versão e changelog

Versão e `CHANGELOG.md` são do mantenedor; nenhum PR de feature os altera ([`CONTRIBUTING.md`](../CONTRIBUTING.md)).

1. Escolha a versão pelo [SemVer](https://semver.org/lang/pt-BR/). O nome do instalador vem de `version` em `apps/desktop/tauri.conf.json`; mantenha `apps/desktop/Cargo.toml` e `package.json` na mesma versão.
2. Gere a entrada do changelog a partir dos commits de `main` com `git cliff` (configuração em `cliff.toml`) e revise o texto.
3. Faça o commit `chore(release): vX.Y.Z` numa branch `chore/release-<x-y-z>` (kebab-case, por exemplo `chore/release-0-1-0`), abra o PR e espere o squash, como qualquer outro PR.
4. Só construa o instalador depois que esse commit estiver em `main`.

## Construir o instalador

No PowerShell, na raiz do repositório, numa sessão aberta só para isso. A chave privada do updater e a senha dela vêm do gerenciador de senhas (ADR-0018) e só existem nesta sessão:

```powershell
# Só se aparecer erro de path-limit (MSB3491, FTK1011); veja docs/dev/build-windows.md
$env:CARGO_TARGET_DIR = "C:\f"

$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $HOME\.tauri\fala-updater.key -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = [Net.NetworkCredential]::new('', (Read-Host -AsSecureString 'Senha da chave do updater')).Password

bun install
bun run tauri build --bundles nsis

Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY, Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

`--bundles nsis` gera só o instalador NSIS, mesmo que alguém altere `bundle.targets`. O resultado fica em `target\release\bundle\nsis\Fala_<versão>_x64-setup.exe` (com `CARGO_TARGET_DIR` curto, em `C:\f\release\bundle\nsis\`), com a assinatura do updater ao lado: `Fala_<versão>_x64-setup.exe.sig`. Com `createUpdaterArtifacts` ligado, o build falha sem `TAURI_SIGNING_PRIVATE_KEY`; nunca contorne isso desligando a opção. A senha é lida com `Read-Host -AsSecureString`, então não fica no histórico do PowerShell.

O instalador usa `installMode` `currentUser`: instala só para o usuário atual, sem janela de administrador, para o colega instalar sozinho. Trocar de modo depois do primeiro `.exe` distribuído deixa duas instalações no mesmo computador; se um dia for preciso, escreva uma ADR nova.

Calcule o hash do arquivo para o corpo do release:

```powershell
Get-FileHash target\release\bundle\nsis\Fala_<versão>_x64-setup.exe -Algorithm SHA256
```

Sem assinatura de código, o SHA-256 é a única forma de o colega conferir o que baixou à mão. O updater confere sozinho a assinatura minisign.

## Sem assinatura: o aviso do SmartScreen

O instalador não tem assinatura de código ([ADR-0008](decisions/0008-distribuicao-byok-sem-backend-assinatura-e-updater-adiados.md)): o certificado custa, e desde 2024 nem o certificado EV dispensa o SmartScreen. Ao executar o `.exe`, o Windows mostra "O Windows protegeu o computador". Quem instala precisa clicar em **Mais informações** e depois em **Executar assim mesmo**.

Os mesmos passos estão na seção "Instalação (Windows 11)" do [`README.md`](../README.md); mantenha os dois textos iguais. Avise o colega antes de mandar o link, para o aviso não parecer um problema do instalador, e mande junto o SHA-256.

Não configure `certificateThumbprint`, `signCommand` nem `tsp` em `bundle.windows` de `tauri.conf.json` sem uma ADR que substitua a 0008. O teste `scripts/ci/test_release_config.py` falha se algum aparecer.

## Validar no Windows

Faça numa máquina Windows 11 limpa, sem Visual Studio nem Rust, que não seja a de build. Cada item é `TODO(windows)` até alguém marcá-lo de fato.

- TODO(windows): o SmartScreen mostra o aviso e os dois cliques do README levam à instalação.
- TODO(windows): o instalador abre em português num Windows em pt-BR, em inglês num Windows em inglês e em português nos demais idiomas (um idioma fora de `languages` cai no primeiro da lista); confirmar a escolha de `languages`.
- TODO(windows): instalar sem administrador funciona; anotar a pasta de instalação e os atalhos criados.
- TODO(windows): o WebView2 é usado se já existe e baixado pelo bootstrapper se não existe.
- TODO(windows): o app abre, fica na bandeja e, na primeira execução, baixa o modelo de voz.
- TODO(windows): instalar a versão nova por cima da antiga preserva configurações e histórico (é o caminho de quem tem uma versão sem updater).
- TODO(windows): **atualizar a partir da versão anterior** pelo updater, com o release publicado como prerelease (veja "Publicar", passo 4). Numa máquina com a versão anterior instalada, feche o Fala pela bandeja e abra-o pelo PowerShell com `$env:FALA_UPDATE_MANIFEST_URL = "https://github.com/augusto-dmh/fala/releases/download/vX.Y.Z/latest.json"` definido, chamando o `fala.exe` da pasta de instalação anotada acima. Em Configurações › Sobre › Atualizações, "Verificar agora" mostra a versão nova. Comece um ditado e clique em "Reiniciar para atualizar": o app recusa e diz por quê. Termine o ditado e clique de novo: o instalador roda em modo passivo, o Fala reabre na versão nova com configurações e histórico intactos e mostra o "Novidades" uma vez. Repita com "Sair" na bandeja no lugar do clique: o update instala e o Fala não reabre.
- TODO(windows): "Depois" esconde a versão oferecida até sair uma maior; desligar "Procurar atualizações" para as checagens.
- TODO(windows): a desinstalação remove o programa, o atalho e a entrada em Aplicativos instalados; registrar o que sobra em dados do usuário e decidir se isso é o desejado.
- TODO(windows): `tauri build` aceita `bundle.targets` com `deb`, `rpm` e `appimage` no Windows, ignorando-os. Se recusar, separar a lista por plataforma.
- TODO(windows): nenhuma chave de API em arquivo de log da sessão de teste.

Um item que falha entra na barra de bugs e bloqueia a publicação.

## Publicar

1. Monte as notas: a entrada do `CHANGELOG.md` desta versão em `notas.md`. É o mesmo texto do corpo do release, do `latest.json` e do "Novidades" (ADR-0018).
2. Gere o manifesto do updater. O script recusa um `.exe` de outra versão, um `.sig` ausente e uma assinatura que não confere com a `pubkey` de `tauri.conf.json`:

   ```powershell
   python scripts/release/gen-latest-json.py --version X.Y.Z --exe target\release\bundle\nsis\Fala_X.Y.Z_x64-setup.exe --notes-file notas.md --output latest.json
   ```

3. Crie o release no GitHub como rascunho, anexando o `.exe`, o `.sig` e o `latest.json`. O GitHub cria a tag `vX.Y.Z` na publicação, no commit dado em `--target`: use o commit `chore(release): vX.Y.Z` de `main`. No corpo (`corpo.md`) vão as notas, o SHA-256 e a frase "O Windows vai mostrar o aviso do SmartScreen: veja o passo a passo no README".

   ```powershell
   gh release create vX.Y.Z --draft --target <sha completo> --title "Fala X.Y.Z" --notes-file corpo.md target\release\bundle\nsis\Fala_X.Y.Z_x64-setup.exe target\release\bundle\nsis\Fala_X.Y.Z_x64-setup.exe.sig latest.json
   ```

   Num rascunho, as URLs do manifesto não resolvem: nenhuma instalação recebe a oferta antes da hora.
4. Publique como prerelease para validar: `gh release edit vX.Y.Z --draft=false --prerelease`. O alias `releases/latest`, de onde as instalações leem o `latest.json`, ignora prereleases; só quem aponta `FALA_UPDATE_MANIFEST_URL` para a tag (seção anterior) vê a versão.
5. Com a seção anterior marcada, promova: `gh release edit vX.Y.Z --prerelease=false --latest`. Cada instalação recebe a versão na checagem seguinte (até ~6 h). O repositório é público: o `.exe` anexado também é.

Um release candidate (`vX.Y.Z-rc.N`) para no passo 4: fica como prerelease e nunca chega ao `latest`. Não há canal beta no app.

## Updater

O updater está **ligado** ([ADR-0018](decisions/0018-updater-ligado-feed-no-github-release-chave-fora-do-ci.md), que substitui a ADR-0008 só neste ponto). Em `tauri.conf.json`: `createUpdaterArtifacts` `true`, `plugins.updater.endpoints` igual a `["https://github.com/augusto-dmh/fala/releases/latest/download/latest.json"]` e a chave pública em `plugins.updater.pubkey`. O teste `scripts/ci/test_release_config.py` falha se qualquer um dos três mudar, e continua falhando se `certificateThumbprint`, `signCommand` ou `tsp` aparecerem.

**A chave.** O mantenedor gera o par minisign uma vez com `scripts/release/new-updater-key.ps1`, que pede uma senha, grava a chave privada em `$HOME\.tauri\fala-updater.key` e escreve só a pública em `tauri.conf.json`. A chave privada e a senha vão para o gerenciador de senhas, mais uma cópia offline guardada em outro lugar, e **nunca no repositório**, nem em workflow, nem em secret do Actions, nem em log. Perder a chave privada é perder a capacidade de atualizar todas as instalações existentes.

**Rotação.** O app aceita uma segunda chave pública, `UPDATER_PUBKEY_NEXT` em `apps/desktop/src/app_update.rs`, tentada quando a assinatura não confere com a principal. Para trocar a chave: (1) gere o par novo, ponha a pública nova em `UPDATER_PUBKEY_NEXT` e publique um release assinado com a chave **antiga**; (2) quando as instalações estiverem nele, troque `plugins.updater.pubkey` pela nova, esvazie `UPDATER_PUBKEY_NEXT` e assine dali em diante com a nova.

**No app.** A checagem vem ligada: no início e a cada ~6 h com jitter, o Fala baixa o `latest.json` do GitHub (que vê o IP; nenhum dado do usuário vai junto) e, se houver versão maior, baixa e confere o instalador em segundo plano. A instalação só acontece por clique em "Reiniciar para atualizar" (bandeja e Configurações › Sobre) ou ao sair do app, e nunca durante ditado, gravação de reunião ou pós-processamento. "Depois" dispensa aquela versão. Não há downgrade. O toggle "Procurar atualizações" fica em Sobre; `FALA_DISABLE_UPDATER` força a checagem desligada e `FALA_UPDATE_MANIFEST_URL` aponta para outro manifesto (validação de prerelease).

A primeira versão com updater é instalada à mão; quem tem uma versão anterior a ela atualiza instalando por cima.

## Retirar uma versão

1. Volte o release para rascunho (`gh release edit vX.Y.Z --draft`). O `releases/latest` volta a apontar para o release anterior, e quem ainda não atualizou deixa de receber a versão ruim. Avise quem instalou. Por isso as releases imutáveis do GitHub ficam desligadas (ADR-0018): com elas, o GitHub recusa mexer num release publicado.
2. Não há rollback nem downgrade: quem já atualizou fica na versão ruim até a seguinte. Não reutilize a tag nem o número: corrija e publique `X.Y.Z+1`, que chega a todos pelo updater.
3. Se a versão ruim quebrar o próprio updater, quem a instalou só sai dela instalando a nova por cima; diga isso no aviso.
