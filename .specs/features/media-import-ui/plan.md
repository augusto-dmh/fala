# media-import-ui — importar arquivo ou URL e gravar só o som do computador

Profile: light · Fase 2, F7 · critério: "um vídeo importado por URL"

## Problem

`fala-media` converte arquivo local (#39) e `SessionMode` já tem `system_only` e `import`, mas
nada disso aparece na UI. Aulas e vídeos ficam sem transcrição; o Granola e o Wispr não importam.

## Flow

1. Página Reuniões → "Importar arquivo ou link": diálogo de arquivo, ou URL `http(s)://`.
2. `import_meeting(source)` → `MeetingManager::import`: valida a URL, acha `ffmpeg`/`ffprobe`
   (e `yt-dlp` para URL) no PATH, uma importação por vez, id novo.
3. URL: `fala_media::download` (`yt-dlp`, argumentos em vetor, URL depois de `--`) baixa só o
   áudio em `audio/<id>/download/`.
4. `fala_media::import` → `audio/<id>/recording.wav` (mono 48 kHz).
5. Linha `meetings` no modo `import`: título do arquivo (ou da página), início e fim na
   importação, duração da origem, URL nas anotações.
6. `retain`: `fala-retention` aceita WAV mono → `sys.opus` + `mic.opus` em silêncio (ADR-0014).
7. Com a chave da ElevenLabs no cofre, a transcrição segue sozinha, como depois de parar uma
   gravação; o ASR já pula o mic em `import` e `system_only`.
8. Falha ou cancelamento: nenhuma linha; a pasta da sessão é apagada.

"Só o som do computador": `MeetingMode::SystemOnly` → `SessionMode::SystemOnly`; o gravador não
abre o mic (o `MeetingRecorder` escreve zeros nesse canal); mesmo indicador e mesmas regras de
início explícito (ADR-0005).

## Doors

1. Windows: `yt-dlp.exe` é bootloader PyInstaller; matar só o filho deixa o download rodando.
   `ProcessHooks { prepare, stop }` injetados pelo desktop (`taskkill /T /F`, `CREATE_NO_WINDOW`
   sob `cfg(windows)` em `apps/desktop`); `crates/media` segue sem `cfg` (ADR-0007).
2. Ferramentas só do PATH (D3, opção c); nenhum download de binário, nenhuma URL nova de CDN.

## Slices

- PR 1 (`feat/media-import-ui`): retenção de WAV mono; `fala_media::download`.
- PR 2 (`feat/media-import-ui-2`, base no PR 1): desktop e UI.
