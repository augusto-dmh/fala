---
status: accepted
date: 2026-09-26
---
# Forkar o Handy como base do ditado

## Contexto e problema
Precisamos de ditado push-to-talk no Windows 11 com ASR local, pill flutuante, tray, autostart, colagem com restore do clipboard e instalador. Escrever tudo do zero custa semanas antes do primeiro ditado útil. O Handy (github.com/cjpais/Handy, MIT, Tauri 2, v0.9.7 em 2026-09-18, ~32k estrelas) já resolveu o Windows e tem pipeline VAD + Parakeet via `transcribe-rs`. Sua marca (nome, logo, ícone) é declarada não-open-source pelo autor.

## Opções consideradas
* Forkar o Handy e renomear.
* Começar do zero em Tauri 2 com os mesmos crates (`cpal`, `rdev`, `enigo`, `arboard`, `transcribe-rs`).
* Forkar Whispering/Epicenter ou anarlog/Hyprnote.

## Decisão
Forkar o Handy. Do zero custaria 2-3 semanas a mais reimplementando settings, histórico e gestão de modelos; Whispering tem menos Windows resolvido; anarlog é um monorepo comercial renomeado duas vezes em um ano.

### Consequências
* Bom: Windows funcional no dia 1; PRs abertos de GNOME Wayland no upstream (issue #1555) reaproveitáveis na fase 3.
* Ruim: herdamos React/Bun/Tailwind e um crate único que precisa virar workspace; precisamos acompanhar o upstream seletivamente.
* Obrigatório: manter o aviso de copyright do CJ Pais no `LICENSE`, adicionar `NOTICE.md`, trocar toda marca, ícone e identificador (`br.com.augusto.fala`), remover sponsor, Nix, traduções e `AI_POLICY.md`.

## Confirmação
`grep -ri handy` no repo retorna só `LICENSE`, `NOTICE.md` e esta ADR.
