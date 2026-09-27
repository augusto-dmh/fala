---
status: accepted
date: 2026-09-26
---
# Tauri 2 como casca; lógica em crates Rust sem dependência de Tauri

## Contexto e problema
O trabalho difícil (hook de teclado, captura de áudio, injeção de texto, ASR) é Rust nativo em qualquer framework. Precisamos testar esse core sem UI (spikes, benchmark, CLI), portar para Linux trocando só adaptadores, e manter a UI em tecnologia de iteração rápida. O Handy é um crate único acoplado ao Tauri.

## Opções consideradas
* Tauri 2, crate único como no Handy.
* Tauri 2, workspace: `crates/*` sem `tauri`, `apps/desktop` (Tauri) e `apps/cli`.
* Electron com addons nativos; Flutter; Slint/iced; Python.

## Decisão
Tauri 2 com workspace. Cinco dos sete projetos de referência usam Tauri; instaladores `.msi`/`.exe`/`.deb`/AppImage com um comando; binários de 15-100 MB contra 120-150 MB do Electron. O workspace é o que permite `fala-cli` e a fase 3 só com adaptadores.

### Consequências
* Bom: core testável por CLI; UI em React onde a iteração importa; plugins oficiais de tray e autostart.
* Ruim: reestruturar o Handy em workspace é a primeira tarefa da fase 1 e pode consumir dias; WebView2 no Windows e webkit2gtk no Linux como dependências.
* Obrigatório: nada em `crates/` importa `tauri`; toda comunicação com a UI passa por `core::Event`.

## Confirmação
Script em CI falha se algum `Cargo.toml` em `crates/` listar `tauri` como dependência; `cargo build -p fala-cli` compila sem WebView.
