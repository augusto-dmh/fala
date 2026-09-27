# Build no Windows

Pré-requisitos e problemas conhecidos para compilar o Fala no Windows 11 (alvo da fase 1). Podado do `BUILD.md` do upstream; as partes de macOS e empacotamento Linux ficaram de fora.

TODO(windows): este guia ainda não foi executado numa máquina Windows limpa. Revisar na primeira build.

## Pré-requisitos

- [Rust](https://rustup.rs/). A versão fica fixada em `rust-toolchain.toml`, e o `rustup` a instala sozinho na primeira build.
- [Bun](https://bun.sh/)
- [Pré-requisitos do Tauri](https://tauri.app/start/prerequisites/) (WebView2 já vem no Windows 11)
- Microsoft C++ Build Tools: Visual Studio 2022 com "Desenvolvimento para desktop com C++", ou o Build Tools 2022
- [CMake](https://cmake.org/download/) no `PATH`:

  ```powershell
  winget install Kitware.CMake
  ```

- [Vulkan SDK](https://vulkan.lunarg.com/sdk/home), da LunarG. É necessário para o backend Vulkan do `transcribe-cpp`, porque o `vulkan-shaders-gen` precisa dos headers e do `glslc`:

  ```powershell
  winget install KhronosGroup.VulkanSDK
  ```

  Abra um terminal novo depois, para que `VULKAN_SDK` esteja definido.

## Build

```powershell
bun install
bun run tauri dev      # desenvolvimento
bun run tauri build    # instalador NSIS em target\release\bundle\nsis\
```

Para compilar só o binário, sem instalador: `bun run tauri build --no-bundle`.

O instalador sai **sem assinatura** (ADR-0008). Ao executá-lo, o SmartScreen mostra um aviso; o README explica como passar por ele.

## Problemas conhecidos

### Erros de path-limit (`MSB3491` / `FTK1011` / `MSB6003`)

A build nativa pode falhar no meio do `transcribe-cpp-sys` com mensagens como:

```
error MSB3491: Could not write lines to file "...VCTargetsPath.tlog\VCTargetsPath.lastbuildstate".
Path: ... exceeds the OS max path limit. The fully qualified file name must be less than 260 characters.
```

A causa é o limite de 260 caracteres do Windows (`MAX_PATH`), estourado pela árvore CMake aninhada do gerador de shaders Vulkan, somada ao `target\...\build\<crate>-<hash>\out\...` do Cargo. Desde o `transcribe-cpp` 0.1.3, a build compila através de uma junction NTFS curta em `%LOCALAPPDATA%\tcs`, sem precisar de admin. Ativar "long paths" no registro **não** resolve, porque o `tracker.exe` do MSBuild ignora a flag.

Se o erro persistir (junction bloqueada por política, ou checkout muito fundo), use um diretório de build curto:

```powershell
$env:CARGO_TARGET_DIR = "C:\f"
```

Para fixar a variável para todos os terminais: `[Environment]::SetEnvironmentVariable('CARGO_TARGET_DIR', 'C:\f', 'User')`. Isso redireciona a build de todos os projetos Rust, não só a do Fala.

### Camadas Vulkan implícitas

O app define `VK_LOADER_LAYERS_DISABLE=~implicit~` ao iniciar, para evitar que overlays de terceiros (gravadores de tela, drivers) quebrem o backend Vulkan. Para depurar com essas camadas ligadas, defina `FALA_KEEP_VULKAN_IMPLICIT_LAYERS=1`.

## Apêndice: Linux (só verificação até a fase 3)

No Ubuntu 25.04, estes pacotes bastam para `cargo check --workspace` e `bun run tauri build`:

```bash
sudo apt install build-essential clang libclang-dev libevdev-dev libasound2-dev pkg-config \
  libssl-dev libvulkan-dev vulkan-tools glslc spirv-headers glslang-tools libgtk-3-dev \
  libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libgtk-layer-shell0 \
  libgtk-layer-shell-dev libopenblas0 patchelf cmake
```
