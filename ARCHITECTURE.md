# Arquitetura do Fala

Codemap vivo. Muda no mesmo commit que cria, move ou remove um crate. O porquê das escolhas fica em `docs/decisions/`; o desenho completo da v1, congelado, em `docs/design/2026-10-fala-v1.md`.

## Visão geral

O Fala captura áudio (microfone e, na fase 2, áudio do sistema), segmenta com VAD e transcreve localmente (Parakeet). Opcionalmente formata o texto com um LLM e insere o resultado no app ativo. Tudo roda na máquina do usuário e não há backend. As chamadas de rede opcionais são três, todas desligáveis: texto do ditado para o LLM, áudio de reunião para o ASR em nuvem, transcrição para o LLM de notas.

```
hotkey ─▶ audio (mic + pré-buffer) ─▶ VAD ─▶ asr ─▶ postproc ─▶ inject ─▶ app ativo
                                                        └─▶ storage (histórico)
```

## Code Map

| Caminho | Pacote | O que é |
|---|---|---|
| `crates/core` | `fala-core` | tipos (`Event`, `Settings`, `Utterance`, `Session`), config, dicionário pessoal, erros |
| `crates/hotkey` | `fala-hotkey` | atalho global com press/release. Windows: hook de teclado. Linux: portal GlobalShortcuts |
| `crates/audio` | `fala-audio` | captura (cpal, loopback WASAPI, PipeWire), VAD Silero, resample, gravador de reunião |
| `crates/asr` | `fala-asr` | trait `Transcriber`. Parakeet local; backends de nuvem para reunião |
| `crates/postproc` | `fala-postproc` | trait `Formatter`. Regras pt-BR mais o LLM opcional |
| `crates/inject` | `fala-inject` | inserção no app ativo (clipboard + Ctrl+V com restore, SendInput) |
| `crates/storage` | `fala-storage` | SQLite (FTS5) mais o espelho Markdown |
| `crates/meeting` | `fala-meeting` | sessão de reunião: máquina de estados (início só por ação explícita, indicador obrigatório, pausa, suspensão, teto), `SessionId` (ULID) e modos |
| `apps/desktop` | `fala` (lib `fala_app_lib`) | casca Tauri 2: tray, pill, janelas, comandos e eventos |
| `apps/cli` | `fala-cli` | `dictate`, `record`, `transcribe`, `bench`, para spikes, benchmark e uso headless |
| `src/` | — | frontend React + TypeScript + Tailwind (Vite, Bun), servido pelo `apps/desktop` |

**Estado no dia 1:** os crates de `crates/` existem só como `//!`. Toda a lógica ainda vive em `apps/desktop/src`, herdada do Handy: `managers/` (audio, model, transcription, history), `audio_toolkit/` (captura, VAD, resample), `shortcut/`, `clipboard.rs` e `paste_tx/` (inserção), `overlay.rs` e `tray.rs`. A fase 1 move essa lógica para os crates, e os managers viram fachadas finas sobre eles. O crate `meeting` nasceu antes da fase 2 só com a sessão (2.F1); calendário, detecção, importação e notas entram nele na fase 2.

## Invariantes

- **Nada em `crates/` depende de `tauri`**, nem de forma transitiva. A UI só conversa com o core via `core::Event`. Verificado por `scripts/check-no-tauri-in-crates.sh` no CI.
- **O áudio de ditado nunca é enviado pela rede.** O tipo que o representa não implementa serialização para os clientes HTTP.
- **Nenhuma chave de API** em código, em config versionada ou em log. As chaves ficam no keyring do SO.
- **A gravação de reunião só começa por ação explícita** e tem indicador visível enquanto dura.
- **O código específico de plataforma** fica em `hotkey`, `audio`, `inject` e `apps/desktop`. `cargo check --workspace` passa no Linux desde a fase 1 (adaptadores ainda sem implementação retornam `Unsupported`).
- **`apps/cli` compila sem WebView:** `cargo build -p fala-cli` não puxa `tauri`.

## Cross-cutting

- **Erros:** `thiserror` nos crates, `anyhow` nos apps; sem `unwrap`/`expect` fora de testes (lint do workspace).
- **Config:** um `Settings` em `core`, persistido por `tauri-plugin-store` no desktop e por TOML na CLI.
- **Eventos:** `core::Event` (enum) do backend para a UI; comandos e tipos TS gerados por `tauri-specta` em `src/bindings.ts`.
- **i18n:** pt-BR é a fonte, en é o segundo idioma (`src/i18n/locales`); o tray lê as mesmas strings via `build.rs`.
- **Logs:** locais, com níveis; nada remoto. O trace de latência por etapa (`FALA_TRACE=1`) entra na fase 1.
- **Dados do usuário:** pasta de dados do app (`%APPDATA%` no Windows, `~/.local/share` no Linux) com `fala.sqlite`, `audio/` e `models/`.

## Orçamento de latência e recursos

Copiado do design doc §5 como referência. Será medido pelo log de etapas (`FALA_TRACE=1`, fase 1) a cada milestone. Estourar o máximo entra na bug bar da fase (ver `ROADMAP.md`).

| Evento | Meta | Máximo | Fonte |
|---|---|---|---|
| Tecla → pill visível e áudio capturando | ≤ 50 ms | 100 ms | Nielsen "instantâneo"; Swink |
| Barras da pill | ≤ 1 frame de atraso, sem stutter | — | — |
| Soltar → texto no campo (frase < 10 s, ASR local, sem LLM) | ≤ 700 ms | 1,0 s | limite de fluxo de pensamento |
| Idem com LLM | ≤ 1,2 s | 2,0 s; acima disso insere o bruto | — |
| Ditado longo (> 30 s) | parciais a cada ≤ 1 s | — | evolução |
| Reunião: progresso da transcrição | visível em ≤ 1 s, cancelável | 10 s sem progresso | Nielsen |
| Idle na bandeja com modelo carregado | < 1 % CPU, < 300 MB RAM | — | contraste com 800 MB do Wispr |
| Início a frio até hotkey funcionar | ≤ 3 s | 5 s | — |
| Pré-buffer de áudio | 300 ms contínuos | — | nunca perder a primeira sílaba |
