# media-import checks

Profile: light
Plan: `.specs/features/media-import/plan.md`

25 checks in 5 slices plus doors · 5 one-way doors · 0 open, of which 0 block

Comandos (todos com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`):
`cargo test -p fala-media` roda os testes sem ffmpeg; `cargo test -p fala-media -- --include-ignored`
roda também os `#[ignore = "precisa de ffmpeg e ffprobe no PATH"]`. O mesmo vale para
`-p fala-cli --test import`. Os arquivos de teste são gerados pelo próprio ffmpeg dentro do teste,
em `CARGO_TARGET_TMPDIR`.

## Checks

### S1 - arquivo local vira WAV mono 48 kHz · 4 files · 20 KB · ~5k

**C1** - mp3, mp4 (vídeo + áudio estéreo) e ogg/opus com um tom de 440 Hz de 1 s viram em `out` um WAV PCM 16 bits, 1 canal, 48 000 Hz, 1 s ± 60 ms, com 440 ± 10 Hz por cruzamentos de zero e pico ≥ 0,1 da escala cheia (AC 1)
Proof: `cargo test -p fala-media --test import -- --include-ignored --exact mp3_becomes_mono_48k_wav`
Proof: `cargo test -p fala-media --test import -- --include-ignored --exact mp4_with_video_becomes_mono_48k_wav`
Proof: `cargo test -p fala-media --test import -- --include-ignored --exact ogg_becomes_mono_48k_wav`

**C2** - O `Ok` traz `ImportedAudio` com `path` = `out`, `sample_rate` = 48 000, `channels` = 1 e `duration` = frames do cabeçalho / 48 000 (AC 2)
Proof: `cargo test -p fala-media --test import -- --include-ignored --exact imported_audio_matches_written_header`

**C3** - Com duas trilhas de áudio (440 Hz e 880 Hz), o WAV tem 440 ± 10 Hz (AC 3)
Proof: `cargo test -p fala-media --test import -- --include-ignored --exact first_audio_track_wins`

**C4** - Com `out` existente, `import` devolve `OutputExists(out)`, `out` fica byte a byte igual, e as ferramentas (arquivos `ffmpeg`/`ffprobe` vazios, não executáveis) não são iniciadas: o erro não é `Io` (AC 4)
Proof: `cargo test -p fala-media --test import -- --exact existing_output_is_left_untouched`

**C5** - Com uma origem WAV de 30 min a 8 kHz, em todo `on_progress` com `processed` > 0 `<out>.part` existe e `out` não; depois do `Ok`, `out` existe, `<out>.part` não e a duração é 1 800 s ± 60 ms (AC 5)
Proof: `cargo test -p fala-media --test import -- --include-ignored --exact output_appears_only_after_success`

### S2 - falhas tipadas · 3 files · 15 KB · ~4k

**C6** - `Tools::locate_in("")` devolve `ToolNotFound { tool: "ffmpeg" }` e a mensagem contém `ffmpeg` e `PATH` (AC 6)
Proof: `cargo test -p fala-media --test import -- --exact empty_path_reports_ffmpeg`

**C7** - Com um diretório no PATH que só tem `ffmpeg`, `locate_in` devolve `ToolNotFound { tool: "ffprobe" }` (AC 7)
Proof: `cargo test -p fala-media --test import -- --exact path_without_ffprobe_reports_ffprobe`

**C8** - Origem inexistente e origem que é um diretório devolvem `InputNotFound(origem)`, sem iniciar subprocesso (ferramentas vazias: o erro não é `Io`) e sem `out` nem `<out>.part` (AC 8)
Proof: `cargo test -p fala-media --test import -- --exact missing_input_is_input_not_found`

**C9** - Um mp4 só com vídeo devolve `NoAudioTrack(origem)`, sem `out` nem `<out>.part` (AC 9)
Proof: `cargo test -p fala-media --test import -- --include-ignored --exact video_only_is_no_audio_track`

**C10** - Um arquivo de texto devolve `Unreadable { path, detail }` com `detail` não vazio, sem `out` nem `<out>.part` (AC 10)
Proof: `cargo test -p fala-media --test import -- --include-ignored --exact text_file_is_unreadable`

**C11** - Um `.ffconcat` local com `duration 50000` devolve `TooLong { duration: 50 000 s, max: 43 200 s }`, sem `<out>.part` e sem nenhum evento de progresso (AC 11)
Proof: `cargo test -p fala-media --test import -- --include-ignored --exact too_long_is_rejected_before_ffmpeg`

**C12** - Com `out` num diretório que não existe, `import` devolve `Ffmpeg { code, detail }` com `code` ≠ `Some(0)`, `detail` não vazio e com no máximo 2 000 bytes, sem `out` nem `<out>.part`; e com o ffmpeg terminado por SIGTERM depois de o `<out>.part` existir (origem de 30 min), devolve `Ffmpeg` com `code` ≠ `Some(0)` e não deixa `out` nem `<out>.part` (AC 12)
Proof: `cargo test -p fala-media --test import -- --include-ignored --exact ffmpeg_failure_is_typed_with_stderr`
Proof: `cargo test -p fala-media --test import -- --include-ignored --exact ffmpeg_killed_after_writing_leaves_nothing`
Proof: `cargo test -p fala-media --lib -- --exact tests::stderr_tail_keeps_last_2000_bytes`

### S3 - progresso e cancelamento · 2 files · 15 KB · ~4k

**C13** - O primeiro evento é `Progress { processed: 0, total: Some(d) }` com `d` = duração do ffprobe (1 s ± 60 ms para o mp3 do teste) (AC 13)
Proof: `cargo test -p fala-media --test import -- --include-ignored --exact first_progress_is_zero_with_probed_total`

**C14** - `processed` nunca decresce entre eventos e o último vale a `duration` do `ImportedAudio` ± 60 ms; um bloco `out_time_us=1500000` seguido de `progress=continue` vira um evento de 1,5 s e `out_time_us=N/A` mantém o valor anterior (AC 14)
Proof: `cargo test -p fala-media --test import -- --include-ignored --exact progress_is_monotonic_and_ends_at_duration`
Proof: `cargo test -p fala-media --lib -- --exact tests::progress_block_parses_out_time_us`

**C15** - `cancel()` chamado por outra thread assim que `<out>.part` aparece no disco (origem de 30 min) faz `import` devolver `Cancelled` em ≤ 1 s depois do cancelamento, sem `out` nem `<out>.part` (AC 15)
Proof: `cargo test -p fala-media --test import -- --include-ignored --exact cancel_while_running_kills_and_cleans`

**C16** - Token já cancelado devolve `Cancelled` sem iniciar subprocesso (ferramentas vazias: o erro não é `Io`) e sem `out` nem `<out>.part` (AC 16)
Proof: `cargo test -p fala-media --test import -- --exact pre_cancelled_token_spawns_nothing`

### S4 - sem rede · 1 file · 5 KB · ~1k

**C17** - Uma playlist `.m3u8` local com o segmento em `http://127.0.0.1:<porta>/a.ts` devolve `Unreadable`, `NoAudioTrack` ou `Ffmpeg`, e o `TcpListener` em `<porta>` não tem conexão pendente (`accept` = `WouldBlock`) (AC 17)
Proof: `cargo test -p fala-media --test import -- --include-ignored --exact local_playlist_opens_no_connection`

### S5 - `fala-cli import` · 3 files · 15 KB · ~4k

**C18** - `fala-cli import tom.mp3 --out t.wav` sai com 0, escreve `t.wav` e o stdout é exatamente 3 linhas: cabeçalho `| out | sample_rate | channels | duration_s |`, separador e a linha com `48000`, `1` e a duração com 3 casas (AC 18)
Proof: `cargo test -p fala-cli --test import -- --include-ignored --exact import_prints_table_and_writes_wav`

**C19** - Sem `--out`, rodando em `<dir>`, o WAV sai em `<dir>/tom.fala.wav` (AC 19)
Proof: `cargo test -p fala-cli --test import -- --include-ignored --exact import_default_out_is_stem_fala_wav`

**C20** - O stderr tem ao menos 2 linhas com `import: ` (o evento inicial e o bloco final), a primeira terminando em `import: 0.0 s / 1.0 s (0 %)` e todas com `/ 1.0 s (`; o formato é `import: 0.5 s / 2.0 s (25 %)` com total conhecido e `import: 0.5 s / ? s (? %)` sem total (AC 20)
Proof: `cargo test -p fala-cli --test import -- --include-ignored --exact import_reports_progress_on_stderr`
Proof: `cargo test -p fala-cli --bin fala-cli -- --exact import::tests::progress_line_formats_known_and_unknown_total`

**C21** - Com `PATH` vazio e uma origem que existe, sai com 1 e o stderr contém `ffmpeg` e `PATH` (AC 21)
Proof: `cargo test -p fala-cli --test import -- --exact import_without_ffmpeg_exits_1`

**C22** - Origem inexistente sai com 2 e o stderr nomeia o arquivo, mesmo com `PATH` vazio (AC 22)
Proof: `cargo test -p fala-cli --test import -- --exact import_missing_file_exits_2`

**C23** - Origem só com vídeo sai com 2; `--out` existente sai com 2 mesmo com `PATH` vazio, e o arquivo existente fica igual (AC 23)
Proof: `cargo test -p fala-cli --test import -- --include-ignored --exact import_no_audio_exits_2`
Proof: `cargo test -p fala-cli --test import -- --exact import_existing_out_exits_2`

### Doors

**C24** - `crates/media` existe como pacote `fala-media` sem dependência de `tauri` e o Code Map do `ARCHITECTURE.md` tem a linha `crates/media` | `fala-media` (door 1)
Proof: `scripts/check-no-tauri-in-crates.sh`
Proof: `grep -c '^| \`crates/media\` | \`fala-media\` |' ARCHITECTURE.md` = 1

**C25** - A invocação do ffprobe e do ffmpeg é a literal da door 4: os argumentos montados contêm `-protocol_whitelist file`, a origem como `file:<absoluto>`, `-map 0:a:0`, `-ac 1`, `-ar 48000`, `-c:a pcm_s16le`, `-f wav`, `-progress pipe:1`, `-stats_period 0.5` e o destino `file:<out>.part` (doors 2 e 4)
Proof: `cargo test -p fala-media --lib -- --exact tests::ffmpeg_args_match_the_door`
Proof: `cargo test -p fala-media --lib -- --exact tests::ffprobe_args_match_the_door`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| formatos de origem (3) | mp3 C1 · mp4 com vídeo C1 · ogg/opus C1 | - |
| `fala_media::import` statuses (9) | `Ok` C1, C2 · `InputNotFound` C8 · `OutputExists` C4 · `Unreadable` C10 · `NoAudioTrack` C9 · `TooLong` C11 · `Ffmpeg` C12 · `Cancelled` C15, C16 · `Io` C4, C8, C16 (asserted absent when the tools are not executable) | - |
| `Tools::locate*` statuses (3) | `Ok` C1 · `ToolNotFound{ffmpeg}` C6 · `ToolNotFound{ffprobe}` C7 | - |
| `fala-cli import` exit codes (3) | `0` C18 · `1` C21 · `2` C22, C23 | - |
| `fala-cli import` flags (2) | `<arquivo>` C18 · `--out` C18, C19 | - |
| falhas que não deixam `out` nem `.part` (6) | `out`/`.part` após `InputNotFound` C8 · `NoAudioTrack` C9 · `Unreadable` C10 · `TooLong` C11 · `Ffmpeg` C12 · `Cancelled` C15, C16 | - |
| doors (5) | 1 C24 · 2 C1, C25 · 3 C6, C7 · 4 C25, C17 · 5 C2, C13, C15 | - |
| eventos de progresso (3) | inicial em 0 C13 · por bloco `progress=` C14 · último = duração C14 | - |

- Claims naming an exit code or a response shape: C18, C21, C22, C23 cross the binary boundary;
  the crate's claims (C1 to C17) cross its public API from an integration test, the same API
  `crates/meeting` will call
- C25 is the one check at unit level: the door is the literal argv, which no integration test
  can observe. C17 proves only what it claims (error and 0 connections): with ffmpeg 7.1.1 the hls
  demuxer already refuses `http` without the flag, so the whitelist itself is proven by C25 alone

## Swept

- validation: C8 (origem inexistente ou diretório), C4 (saída existente), C11 (teto de 12 h)
- failure modes: C10 (ilegível), C12 (ffmpeg falha), C5 (sem arquivo parcial no sucesso)
- idempotency: C4 - repetir a mesma importação não sobrescreve o resultado anterior
- authorization: n/a - biblioteca e comando locais; o acesso ao arquivo é o do usuário do SO
- concurrency: n/a - cada chamada usa seu próprio `<out>.part`; duas importações para o mesmo `out` são uso indevido e a segunda falha no rename ou em `OutputExists`, sem corromper a primeira
- data lifecycle: C15, C12 - nada parcial fica no disco; o WAV pertence a quem chamou (a retenção é da F3)
- dependency failure: C6, C7, C21 (ferramenta ausente), C12 (ferramenta falha), C17 (rede bloqueada)
- state transitions: C13, C15, C16 - iniciado → progresso → concluído ou cancelado; cancelar antes de iniciar não inicia nada
- observability: C20 (progresso no stderr), C12 (stderr do ffmpeg no erro); nenhum conteúdo de áudio em log

## Handoff

- S1-S5 + doors ≈ 20k (arquivos novos em `crates/media` e `apps/cli`, mais 2 KB do `ARCHITECTURE.md` e dos manifests), muito abaixo do budget de 150k - one builder
- **Boundary:** C1-C17, C24, C25 closed at `87c5df8`; C18-C23 closed in the `fala-cli import` commit
- **Settled mid-build:** after verification round 1 (FAIL: C5, C12 and C15 never observed a `.part` on disk, so they would pass with the cleanup removed), the panel strengthened C5, C12 and C15 to run against a 30 min source with the `.part` already written, added the SIGTERM proof to C12, and tightened C20 to the initial and final progress lines; every change makes a claim stricter, none weakens one. Removing the `.part` cleanup now fails C12 and C15. Confirmed? y — delegado pelo Augusto em 2026-10-02, decidido pelo painel
- **Abandoned:** `-ac 2` to build the stereo mp4 fixture: it upmixes the mono tone at -3 dB, so the source no longer carried the full tone; replaced by `pan=stereo|c0=c0|c1=c0` (fixture only, the C1 threshold is unchanged)
