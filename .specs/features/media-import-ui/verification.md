# media-import-ui verification

Windows 11 (Alienware), ffmpeg 9.0.2 e yt-dlp 2026.08.19 no PATH, `CARGO_TARGET_DIR=C:\f\media-import-ui`.

| Check | Evidência |
| --- | --- |
| C1 | `cargo test -p fala-retention`: 12 + 15 ok |
| C2-C4 | `cargo test -p fala-media`: 8 unit + 2 + 5 integração ok, 17 ignorados (precisam das ferramentas) |
| C5-C6 | `cargo test -p fala-media --test download -- --include-ignored`: 4 ok; com `child.kill()` no lugar do kill da árvore, C6 falha ("being used by another process") |
| C7 | `cargo test -p fala --lib meeting`: 32 ok; `cargo test -p fala --lib`: 494 ok, 1 ignorado |
| C8 | `bun src/components/meeting/import.test.tsx`: 5 checks ok; `meeting.test.tsx` ok; `bun run check:translations` ok |
| C9 | parcial, abaixo |

Também: `cargo clippy --workspace --all-targets -- -D warnings` limpo; `bun run lint`, `bun run format:check`, `bunx tsc --noEmit` limpos.

Manual, sem a UI:

- `download` + `import` contra `https://www.youtube.com/watch?v=jNQXAC9IVRw`: título "Me at the zoo", 9 eventos de progresso, WAV mono 48 kHz de 19,0 s.
- `fala-cli import "…\Vídeos de aula\Aula de física – 1.mp4"`: 3,000 s, mono 48 kHz.
- `yt-dlp.exe` morto só no pai deixou o filho rodando (visto no `Win32_Process`); o `taskkill /T` resolve.

Not tested:

- A página no app (clicar Importar, arrastar arquivo, ver o avanço, cancelar) e "Gravar só o som do computador" por 2 min com vídeo tocando: precisa de alguém na máquina.
- Transcrição e notas de uma importação: a transcrição do app é a ElevenLabs (chave paga do Augusto); não há modelo Parakeet ONNX local nesta máquina para o caminho local.
- `ffmpeg_killed_after_writing_leaves_nothing` usa `pkill` e só roda no Linux.
