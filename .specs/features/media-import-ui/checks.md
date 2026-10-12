# media-import-ui checks

Profile: light
Plan: `.specs/features/media-import-ui/plan.md`

Prefixo de todo `cargo`: `CARGO_TARGET_DIR='C:\f\media-import-ui'`.

### S1 - crates

**C1** - WAV mono vira `sys.opus` com o tom e `mic.opus` em silêncio, mesma contagem; 3 canais recusado
Proof: `cargo test -p fala-retention`

**C2** - só `http(s)://` com host; espaço, controle, `file:`, `ftp:`, `javascript:` e `--opção` recusados antes de iniciar processo
Proof: `cargo test -p fala-media only_http_and_https_urls_pass invalid_url_starts_nothing`

**C3** - argumentos em vetor, URL única e última depois de `--`, sem `--exec`/`--batch-file`
Proof: `cargo test -p fala-media ytdlp_args_keep_the_url_as_one_last_argument`

**C4** - `yt-dlp` ausente dá `ToolNotFound { tool: "yt-dlp" }` com o nome na mensagem
Proof: `cargo test -p fala-media missing_yt_dlp_is_reported_by_name`

**C5** - URL de servidor local baixa, tem título, e importa para mono 48 kHz
Proof: `cargo test -p fala-media --test download -- --include-ignored url_downloads_and_imports`

**C6** - cancelar mata a árvore inteira em < 2 s: a conexão cai e a pasta pode ser apagada (falha com kill só do processo)
Proof: `cargo test -p fala-media --test download -- --include-ignored cancel_stops_the_whole_download`

### S2 - desktop e UI

**C7** - `system_only` não abre mic; `import` vira sessão terminada com o título do arquivo e a URL nas anotações; erros de mídia tipados
Proof: `cargo test -p fala --lib meeting`

**C8** - seletor tem "só o som do computador" e, gravando nesse modo, sem nível do mic; Importar mostra avanço e cancelar; só http(s) no campo; erros novos em pt e en
Proof: `bun src/components/meeting/import.test.tsx`; `bun src/components/meeting/meeting.test.tsx`; `bun run check:translations`

**C9** - manual, Windows: `.mp4` com espaço e acento no caminho; URL do YouTube; 2 min só do sistema com vídeo tocando
Proof: app de debug portátil
