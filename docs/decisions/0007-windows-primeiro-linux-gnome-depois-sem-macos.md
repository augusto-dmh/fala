---
status: accepted
date: 2026-09-26
---
# Ordem de plataformas: Windows 11 primeiro, Ubuntu GNOME Wayland na fase 3, macOS fora

## Contexto e problema
O computador pessoal é Windows 11; o de trabalho é Ubuntu 25.04 com GNOME 48 em Wayland. No Windows, hotkey (hook `WH_KEYBOARD_LL`), overlay sem foco, colagem e loopback WASAPI têm caminho conhecido. No GNOME Wayland não existem hotkey com key-up, injeção de texto nem janela sempre-no-topo fora dos portais (`GlobalShortcuts`, `RemoteDesktop`) e do PipeWire, e o portal pode recusar apps não-Flatpak. Ninguém usa macOS.

## Opções consideradas
* Windows primeiro, Linux na fase 3.
* Linux primeiro.
* Os dois em paralelo.

## Decisão
Windows primeiro. A fase 3 implementa só os adaptadores Linux dos crates `hotkey`, `audio`, `inject` e o feedback por tray/som em vez de overlay. macOS não é alvo e não recebe tempo.

### Consequências
* Bom: primeiro uso diário em semanas; risco de Wayland isolado numa fase.
* Ruim: o notebook do trabalho fica com o Wispr até a fase 3.
* Obrigatório: os traits de plataforma são definidos na fase 1 já pensando nos dois SOs; nenhum `#[cfg(windows)]` fora dos crates de plataforma.

## Confirmação
`cargo check --workspace` passa em Linux desde a fase 1 (adaptadores Linux podem ser stubs que retornam `Unsupported`).
