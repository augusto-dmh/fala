# Release do Fala

Roteiro para gerar e distribuir o instalador do Windows. A [ADR-0008](decisions/0008-distribuicao-byok-sem-backend-assinatura-e-updater-adiados.md) exige este arquivo antes do primeiro `.exe`. Release é decisão e ação do mantenedor: nenhum passo daqui roda sozinho, e nenhum agente de código versiona, empurra tag ou publica.

O instalador sai **sem assinatura de código** e **sem atualização automática**. Os dois são decisões da ADR-0008, não pendências esquecidas.

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

No PowerShell, na raiz do repositório:

```powershell
# Só se aparecer erro de path-limit (MSB3491, FTK1011); veja docs/dev/build-windows.md
$env:CARGO_TARGET_DIR = "C:\f"

bun install
bun run tauri build --bundles nsis
```

`--bundles nsis` gera só o instalador NSIS, mesmo que alguém altere `bundle.targets`. O resultado fica em `target\release\bundle\nsis\Fala_<versão>_x64-setup.exe` (com `CARGO_TARGET_DIR` curto, em `C:\f\release\bundle\nsis\`).

O instalador usa `installMode` `currentUser`: instala só para o usuário atual, sem janela de administrador, para o colega instalar sozinho. Trocar de modo depois do primeiro `.exe` distribuído deixa duas instalações no mesmo computador; se um dia for preciso, escreva uma ADR nova.

Calcule o hash do arquivo para o corpo do release:

```powershell
Get-FileHash target\release\bundle\nsis\Fala_<versão>_x64-setup.exe -Algorithm SHA256
```

Sem assinatura, o SHA-256 é a única forma de o colega conferir o que baixou.

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
- TODO(windows): instalar a versão nova por cima da antiga preserva configurações e histórico (o README manda atualizar assim, no lugar do update automático).
- TODO(windows): a desinstalação remove o programa, o atalho e a entrada em Aplicativos instalados; registrar o que sobra em dados do usuário e decidir se isso é o desejado.
- TODO(windows): `tauri build` aceita `bundle.targets` com `deb`, `rpm` e `appimage` no Windows, ignorando-os. Se recusar, separar a lista por plataforma.
- TODO(windows): nenhuma chave de API em arquivo de log da sessão de teste.

Um item que falha entra na barra de bugs e bloqueia a publicação.

## Publicar

1. Crie o release no GitHub como rascunho, anexando o `.exe`. O GitHub cria a tag `vX.Y.Z` na publicação, no commit dado em `--target`: use o commit `chore(release): vX.Y.Z` de `main`.

   ```powershell
   gh release create vX.Y.Z --draft --target <sha completo> --title "Fala X.Y.Z" --notes-file notas.md target\release\bundle\nsis\Fala_<versão>_x64-setup.exe
   ```

2. No corpo (`notas.md`): a entrada do `CHANGELOG.md`, o SHA-256, e a frase "O Windows vai mostrar o aviso do SmartScreen: veja o passo a passo no README".
3. Publique o rascunho só depois de a seção anterior estar marcada: `gh release edit vX.Y.Z --draft=false`. O repositório é público: o `.exe` anexado também é.

## Updater

O updater está **desligado** e deve continuar assim até uma decisão registrada ([ADR-0008](decisions/0008-distribuicao-byok-sem-backend-assinatura-e-updater-adiados.md)). Hoje isso significa `plugins.updater` com `pubkey` vazia e `endpoints` vazio, e `createUpdaterArtifacts` `false` em `tauri.conf.json`. O teste `scripts/ci/test_release_config.py` falha se qualquer um dos três mudar.

Para ligar, escreva antes uma ADR nova que substitua a 0008 nesse ponto. O Tauri usa uma chave minisign: gere o par uma vez, guarde a chave privada no gerenciador de senhas do mantenedor e **nunca no repositório**, nem em workflow, nem em log. Perder a chave privada é perder a capacidade de atualizar todas as instalações existentes.

Sem updater, quem já instalou só recebe a versão nova instalando-a por cima.

## Retirar uma versão

1. Volte o release para rascunho (`gh release edit vX.Y.Z --draft`) ou apague-o, e avise quem instalou. Se o repositório passar a usar releases imutáveis, o GitHub pode recusar mexer num release publicado: nesse caso, a única saída é publicar a versão seguinte.
2. Não reutilize a tag nem o número: corrija e publique `X.Y.Z+1`.
3. Sem updater, quem instalou a versão ruim fica nela até instalar a nova por cima; diga isso no aviso.
