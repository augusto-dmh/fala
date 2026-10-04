# media-import verification

**Verdict**: PASS
**Profile**: light
**Diff range**: 008102f..d65b0cbc5103754d9613d0d14fe0ce8f3aadad0b
**Round**: 2 - scoped
**Verifier**: independent sub-agent (author != verifier)

A rodada 1 (em `9fe6ed2`) deu FAIL em C5, C12 e C15: nenhuma proof observava um `<out>.part` no
disco, então remover a limpeza passava verde. A correção `d65b0cb` só toca testes e artefatos
(`crates/media/tests/import.rs`, `apps/cli/tests/import.rs`, `checks.md`, `plan.md`);
`crates/media/src/lib.rs` e `apps/cli/src/import.rs` não mudaram (`git diff 9fe6ed2 HEAD --stat --
crates/media/src apps/cli/src` vazio). Escopo desta rodada: todas as proofs de novo em HEAD
`d65b0cb`, citações renovadas nos dois arquivos de teste, julgamento completo de C5, C12, C15 e
C20 e das notas não-PASS da rodada 1; o resto vem de `9fe6ed2`, marcado como tal.

Agora o `.part` está no disco nos momentos em que as asserções contam: C5 registra
`(part.exists(), out.exists())` em cada evento com `processed > 0` e exige `(true, false)` em
todos, com pelo menos um; C12 (proof nova) e C15 esperam `wait_for(&part)` antes do SIGTERM e do
`cancel()`. Checagem de discriminação: com as três linhas de limpeza removidas, C12 e C15 ficam
vermelhos exatamente em `!part.exists()`.

## Binding sources

carried from 9fe6ed2: nenhuma fonte marcada como binding no plano. Perfil `light`, então o passo 1
não roda, e a correção não mexeu na interface.

## Checks

Proofs verified at d65b0cb (todas rodadas de novo). Citações de `M` e `T` verified at d65b0cb
(arquivos tocados pela correção). Citações de `L` e `I` carried from 9fe6ed2 (arquivos
inalterados, as linhas continuam valendo).

Comandos (todos com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`):
A = `cargo test -p fala-media -- --include-ignored` exit 0 (lib 5 ok, tests/import.rs 20 ok);
B = `cargo test -p fala-cli --test import -- --include-ignored` exit 0 (7 ok);
U = `cargo test -p fala-cli --bin fala-cli -- --exact import::tests::progress_line_formats_known_and_unknown_total` exit 0 (1 ok, 14 filtered).
Arquivos: `M` = `crates/media/tests/import.rs`, `L` = `crates/media/src/lib.rs`, `T` = `apps/cli/tests/import.rs`, `I` = `apps/cli/src/import.rs`.
ffmpeg do sistema: 7.1.1-1ubuntu1.3.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | mp3, mp4 (vídeo + estéreo), ogg/opus viram WAV s16 mono 48 kHz, 1 s ± 60 ms, 440 ± 10 Hz, pico ≥ 0,1 | A: `mp3_becomes_mono_48k_wav ... ok`, `mp4_with_video_becomes_mono_48k_wav ... ok`, `ogg_becomes_mono_48k_wav ... ok` | `crates/media/tests/import.rs:184` - `assert_eq!(wav.spec.channels, 1)`; `:185` `assert_eq!(wav.spec.sample_rate, 48_000)`; `:186` `assert_eq!(wav.spec.bits_per_sample, 16)`; `:189` `near(wav.duration(), Duration::from_secs(1))` (TOLERANCE 60 ms em `:22`); `:193` `wav.peak() >= 0.1`; `:195` `(hz - expected_hz).abs() <= 10.0`; chamado com 440.0 em `:215`, `:224`, `:233`. Julgamento carried from 9fe6ed2 | PASS |
| C2 | `ImportedAudio` com path = out, 48 000, 1, duration = frames / 48 000 | A: `imported_audio_matches_written_header ... ok` | `crates/media/tests/import.rs:246` - `assert_eq!(audio.path, out)`; `:247` `assert_eq!(audio.sample_rate, 48_000)`; `:248` `assert_eq!(audio.channels, 1)`; `:249-252` `assert_eq!(audio.duration, Duration::from_nanos(u64::from(frames) * 1_000_000_000 / 48_000))`. Julgamento carried from 9fe6ed2 | PASS |
| C3 | duas trilhas (440/880) -> WAV com 440 ± 10 Hz | A: `first_audio_track_wins ... ok` | `crates/media/tests/import.rs:280` - `assert_mono_48k_tone(&out, 440.0)` -> `:195`; fixture com dois `-map` em `:269`, `:271`. Julgamento carried from 9fe6ed2 | PASS |
| C4 | `out` existente -> `OutputExists(out)`, bytes intactos, ferramentas falsas não iniciadas | A: `existing_output_is_left_untouched ... ok` | `crates/media/tests/import.rs:298` - `Err(MediaError::OutputExists(path)) => assert_eq!(path, out)` (ferramentas falsas em `:291`; spawn daria `Io`); `:301` `assert_eq!(fs::read(&out).unwrap(), b"conteudo anterior")`. Julgamento carried from 9fe6ed2 | PASS |
| C5 | origem WAV de 30 min a 8 kHz: em todo `on_progress` com `processed` > 0 `.part` existe e `out` não; após `Ok`, `out` existe, `.part` não, duração 1 800 s ± 60 ms | A: `output_appears_only_after_success ... ok` | verified at d65b0cb. `crates/media/tests/import.rs:314-316` - `if p.processed > Duration::ZERO { mid_run.push((part.exists(), out.exists())) }`; `:319` `assert!(!mid_run.is_empty())`; `:320-323` `mid_run.iter().all(...)` com o predicado `*seen == (true, false)`; `:324` `out.exists()`; `:325` `!part.exists()`; `:326-330` `near(audio.duration, Duration::from_secs(1800))`; fonte `long_wav` em `:114-125` (`sine=...:duration=1800:sample_rate=8000`). O `.part` é observado existindo no disco durante a conversão, então escrever direto em `out` ou renomear cedo falha `:320`; medido aqui: conversão de ~1,1-1,35 s, `.part` visível a partir de ~0,15-0,2 s, então há eventos `processed > 0` com o arquivo no disco | PASS |
| C6 | `locate_in("")` -> `ToolNotFound{ffmpeg}`, mensagem com `ffmpeg` e `PATH` | A: `empty_path_reports_ffmpeg ... ok` | `crates/media/tests/import.rs:336` - `matches!(error, MediaError::ToolNotFound { tool: "ffmpeg" })`; `:338` `message.contains("ffmpeg")`; `:339` `message.contains("PATH")`. Julgamento carried from 9fe6ed2 | PASS |
| C7 | PATH só com `ffmpeg` -> `ToolNotFound{ffprobe}` | A: `path_without_ffprobe_reports_ffprobe ... ok` | `crates/media/tests/import.rs:351-352` - `matches!(error, MediaError::ToolNotFound { tool: "ffprobe" })`. Julgamento carried from 9fe6ed2 | PASS |
| C8 | origem inexistente e diretório -> `InputNotFound(origem)`, sem spawn, sem `out`/`.part` | A: `missing_input_is_input_not_found ... ok` | `crates/media/tests/import.rs:371` - `Err(MediaError::InputNotFound(path)) => assert_eq!(path, input)` (laço em `:362`, ferramentas falsas em `:360`); `:377` `!out.exists()`; `:378` `!part_of(&out).exists()`. Julgamento carried from 9fe6ed2 | PASS |
| C9 | mp4 só vídeo -> `NoAudioTrack(origem)`, sem `out`/`.part` | A: `video_only_is_no_audio_track ... ok` | `crates/media/tests/import.rs:396` - `Err(MediaError::NoAudioTrack(path)) => assert_eq!(path, input)`; `:399` `!out.exists()`; `:400` `!part_of(&out).exists()`. Julgamento carried from 9fe6ed2 | PASS |
| C10 | texto -> `Unreadable{path, detail}` com detail não vazio, sem `out`/`.part` | A: `text_file_is_unreadable ... ok` | `crates/media/tests/import.rs:419` - `assert_eq!(path, input)`; `:420` `assert!(!detail.is_empty())`; `:424-425` `!out.exists()`, `!part_of(&out).exists()`. Julgamento carried from 9fe6ed2 | PASS |
| C11 | `.ffconcat` de 50 000 s -> `TooLong{50 000 s, 43 200 s}`, sem `.part`, 0 eventos | A: `too_long_is_rejected_before_ffmpeg ... ok` | `crates/media/tests/import.rs:451` - `assert_eq!(duration, Duration::from_secs(50_000))`; `:452` `assert_eq!(max, Duration::from_secs(43_200))`; `:456` `assert_eq!(events, 0)`; `:458` `!part_of(&out).exists()`. Julgamento carried from 9fe6ed2 | PASS |
| C12 | `out` em diretório inexistente -> `Ffmpeg{code != Some(0), detail não vazio, ≤ 2 000 B}`, sem `out`/`.part`; SIGTERM no ffmpeg com o `.part` já no disco (30 min) -> `Ffmpeg` com `code != Some(0)`, sem `out`/`.part` | A: `ffmpeg_failure_is_typed_with_stderr ... ok`, `ffmpeg_killed_after_writing_leaves_nothing ... ok`, `tests::stderr_tail_keeps_last_2000_bytes ... ok` | verified at d65b0cb. `crates/media/tests/import.rs:476` - `assert_ne!(code, Some(0))`; `:477` `!detail.is_empty()`; `:478` `detail.len() <= 2_000`; `:482-483` `!out.exists()`, `!part_of(&out).exists()`. Proof nova: `:575` `wait_for(&part)` antes do `pkill -TERM -f <part>` em `:576-581`; `:591` `assert!(killer.join().unwrap().success(), "pkill não achou o ffmpeg")` (o ffmpeg estava vivo quando o sinal saiu); `:593` `Err(MediaError::Ffmpeg { code, .. }) => assert_ne!(code, Some(0))`; `:596` `!out.exists()`; `:597` `!part.exists()`. Cauda: `crates/media/src/lib.rs:362` `assert_eq!(tail.len(), 2_000)`, `:363` `tail.ends_with("fim")` (carried from 9fe6ed2). Discriminação: sem a limpeza, `:597` falha; ignorando o status do ffmpeg, `:593` falha (ver Faults injected) | PASS |
| C13 | primeiro evento `Progress{0, Some(d)}`, d = 1 s ± 60 ms | A: `first_progress_is_zero_with_probed_total ... ok` | `crates/media/tests/import.rs:501` - `assert_eq!(first.processed, Duration::ZERO)`; `:503` `near(total, Duration::from_secs(1))`. Julgamento carried from 9fe6ed2 | PASS |
| C14 | `processed` monotônico, último = duration ± 60 ms; `out_time_us=1500000` + `progress=continue` -> 1,5 s; `N/A` mantém | A: `progress_is_monotonic_and_ends_at_duration ... ok`, `tests::progress_block_parses_out_time_us ... ok` | `crates/media/tests/import.rs:521-524` - `events.windows(2).all(...)` com o predicado `w[0].processed <= w[1].processed`; `:526-530` `near(last, audio.duration)`; `crates/media/src/lib.rs:337-338` `feed("progress=continue") == Some(Duration::from_millis(1500))`; `:340-343` `N/A` seguido de `progress=end` mantém 1,5 s (carried from 9fe6ed2) | PASS |
| C15 | `cancel()` de outra thread assim que o `.part` aparece (30 min) -> `Cancelled` em ≤ 1 s depois do cancelamento, sem `out`/`.part` | A: `cancel_while_running_kills_and_cleans ... ok` | verified at d65b0cb. `crates/media/tests/import.rs:546-547` - `let seen = wait_for(&part); canceller.cancel();` (o `.part` existe antes do cancelamento); `:554-555` `matches!(result, Err(MediaError::Cancelled))`; `:558` `elapsed = returned.duration_since(cancelled_at)` (medido a partir de antes do `cancel()`, então mais estrito que o check); `:559` `elapsed <= Duration::from_secs(1)`; `:560` `!out.exists()`; `:561` `!part.exists()`. Discriminação: sem a limpeza, `:561` falha. Medido aqui: `.part` em ~0,15-0,2 s, conversão termina em ~1,1-1,35 s, então o cancelamento cai com o ffmpeg rodando | PASS |
| C16 | token já cancelado -> `Cancelled`, sem spawn, sem `out`/`.part` | A: `pre_cancelled_token_spawns_nothing ... ok` | `crates/media/tests/import.rs:609-610` - `matches!(result, Err(MediaError::Cancelled))` (ferramentas falsas em `:608`); `:613-614` `!out.exists()`, `!part_of(&out).exists()`. Julgamento carried from 9fe6ed2 | PASS |
| C17 | `.m3u8` local com segmento `http://127.0.0.1:<porta>` -> erro tipado e 0 conexões | A: `local_playlist_opens_no_connection ... ok` | `crates/media/tests/import.rs:639-646` - `matches!(result, Err(...))` com as alternativas `MediaError::Unreadable { .. }`, `MediaError::NoAudioTrack(_)`, `MediaError::Ffmpeg { .. }`; `:649-650` `matches!(&pending, Err(e) if e.kind() == ErrorKind::WouldBlock)`. A nota da rodada 1 (C17 não discrimina a whitelist no ffmpeg 7.1.1) agora está registrada em `checks.md` (Coverage, nota de nível); julgamento carried from 9fe6ed2 | PASS |
| C18 | `import tom.mp3 --out t.wav` exit 0, WAV escrito, stdout = 3 linhas com 48000, 1 e 3 casas | B: `import_prints_table_and_writes_wav ... ok` | `apps/cli/tests/import.rs:78` - `assert_eq!(o.status.code(), Some(0))`; `:80` `(spec.sample_rate, spec.channels) == (48_000, 1)`; `:83` `assert_eq!(lines.len(), 3)`; `:84` cabeçalho literal; `:91-92` `cells[1] == "48000"`, `cells[2] == "1"`; `:94` `assert_eq!(decimals, Some(3))`. Julgamento carried from 9fe6ed2 | PASS |
| C19 | sem `--out`, em `<dir>`, sai `<dir>/tom.fala.wav` | B: `import_default_out_is_stem_fala_wav ... ok` | `apps/cli/tests/import.rs:107` - `assert_eq!(o.status.code(), Some(0))`; `:109` `wav_spec(&dir.join("tom.fala.wav"))` == `(48_000, 1)`; `:110` `!media.join("tom.fala.wav").exists()`. Julgamento carried from 9fe6ed2 | PASS |
| C20 | stderr com ≥ 2 linhas `import: `, a primeira terminando em `import: 0.0 s / 1.0 s (0 %)`, todas com `/ 1.0 s (`; formato com e sem total | B: `import_reports_progress_on_stderr ... ok`; U: `import::tests::progress_line_formats_known_and_unknown_total ... ok` | verified at d65b0cb. `apps/cli/tests/import.rs:122` - `progress = err.lines().filter(...)` com o predicado `l.contains("import: ")`; `:123` `assert!(progress.len() >= 2)`; `:124-128` `progress[0].ends_with("import: 0.0 s / 1.0 s (0 %)")`; `:129` `progress.iter().all(...)` com o predicado `l.contains("/ 1.0 s (")`; `:130` `!stdout(&o).contains("import: ")`; `apps/cli/src/import.rs:95-100` `== "import: 0.5 s / 2.0 s (25 %)"`, `:102-107` `== "import: 0.5 s / ? s (? %)"` (carried from 9fe6ed2). Ver gap 2 sobre "uma linha por evento" | PASS |
| C21 | PATH vazio + origem existente -> exit 1, stderr com `ffmpeg` e `PATH` | B: `import_without_ffmpeg_exits_1 ... ok` | `apps/cli/tests/import.rs:138` - `assert_eq!(o.status.code(), Some(1))`; `:140` `err.contains("ffmpeg")`; `:141` `err.contains("PATH")`. Julgamento carried from 9fe6ed2 | PASS |
| C22 | origem inexistente -> exit 2 nomeando o arquivo, mesmo com PATH vazio | B: `import_missing_file_exits_2 ... ok` | `apps/cli/tests/import.rs:149` - `assert_eq!(o.status.code(), Some(2))`; `:150` `stderr(&o).contains("nao-existe.mp3")`. Julgamento carried from 9fe6ed2 | PASS |
| C23 | só vídeo -> exit 2; `--out` existente -> exit 2 com PATH vazio, arquivo igual | B: `import_no_audio_exits_2 ... ok`, `import_existing_out_exits_2 ... ok` | `apps/cli/tests/import.rs:169` - `assert_eq!(o.status.code(), Some(2))`; `:179` `assert_eq!(o.status.code(), Some(2))`; `:180` `assert_eq!(fs::read(dir.join("t.wav")).unwrap(), b"conteudo anterior")`. Julgamento carried from 9fe6ed2 | PASS |
| C24 | `fala-media` sem `tauri`; Code Map com a linha `crates/media` / `fala-media` | `scripts/check-no-tauri-in-crates.sh` exit 0 (`ok: no tauri in crates/`); grep do Code Map (o comando literal de `checks.md` C24) -> `1`, exit 0 | `ARCHITECTURE.md:25` - linha do Code Map com as células `crates/media`, `fala-media` e "importação de arquivo de áudio ou vídeo pelo ..."; `crates/media/Cargo.toml:10-13` deps só `hound`, `log`, `thiserror`. Arquivos inalterados pela correção; carried from 9fe6ed2, comandos rodados de novo | PASS |
| C25 | argv do ffprobe/ffmpeg é a literal da door 4 | A: `tests::ffmpeg_args_match_the_door ... ok`, `tests::ffprobe_args_match_the_door ... ok` | `crates/media/src/lib.rs:294` - `has_pair(&args, "-protocol_whitelist", "file")`; `:295` `has_pair(&args, "-i", "file:/media/aula com espaço.mp4")`; `:296-302` `-map 0:a:0`, `-ac 1`, `-ar 48000`, `-c:a pcm_s16le`, `-f wav`, `-progress pipe:1`, `-stats_period 0.5`; `:303-306` destino `file:/out/aula.fala.wav.part`; `:310` `whitelist < input_flag`; `:316` whitelist no ffprobe; `:323-326` `args.last() == Some("file:/media/aula.mp4")`. Carried from 9fe6ed2 | PASS |

**Os checks que mudaram de texto ficaram mais estritos** (comparado com
`git show 9fe6ed2:.specs/features/media-import/checks.md`), verified at d65b0cb:

- C5: antes era "no primeiro `on_progress` `out` não existe". Agora pede `(.part existe, out não)` em
  todo evento com `processed > 0`, além da duração de 1 800 s. O evento 0 deixou de ser amostrado,
  mas ele sai antes de qualquer evento com `processed > 0` e `out` só surge no rename final, então
  a afirmação nova cobre a antiga e acrescenta o `.part` presente. A troca da origem mp3 por WAV
  não perde formato, porque C1 cobre o mp3.
- C12: a cláusula antiga ficou igual e ganhou uma segunda, com o `.part` no disco. Só acrescenta.
- C15: antes era "dentro do primeiro `on_progress`, ≤ 1 s". Agora o cancelamento só vem depois de
  o `.part` existir, e o relógio começa antes do `cancel()` (`:546-558`). As duas mudanças apertam
  a afirmação.
- C20: antes era "ao menos uma linha". Agora pede ≥ 2 linhas, a primeira fixa em `0.0 s / 1.0 s (0 %)`
  e todas com o total. Só acrescenta.
- Plan Flow 1: `ensure_input` passou a `check_paths`, que agora existe (`crates/media/src/lib.rs:94`)
  e é chamado pela CLI antes de `Tools::locate` (`apps/cli/src/import.rs:29`). A contagem
  "25 checks in 5 slices plus doors" bate com C1-C25.

Nível: carried from 9fe6ed2 (C18-C23 na fronteira do binário, C1-C17 na API pública, C25 unitário
justificado). As proofs novas ou alteradas continuam no mesmo nível. Nenhum gap de nível.

`Swept`: carried from 9fe6ed2. Nenhuma linha resolve para "existing"; as linhas `n/a` são política
aprovada.

## Coverage

Perfil `light`: a tabela do autor foi lida. A linha das falhas sem `.part` foi re-julgada contra
as proofs novas (verified at d65b0cb); as outras são carried from 9fe6ed2.

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| formatos de origem (3) | lida do autor (light), carried from 9fe6ed2 | mp3 C1 · mp4 com vídeo C1 · ogg/opus C1 | - |
| `fala_media::import` statuses (9) | lida do autor (light), carried from 9fe6ed2 | `Ok` C1, C2, C5 · `InputNotFound` C8 · `OutputExists` C4 · `Unreadable` C10 · `NoAudioTrack` C9 · `TooLong` C11 · `Ffmpeg` C12 · `Cancelled` C15, C16 · `Io` ausente em C4, C8, C16 | - |
| `Tools::locate*` statuses (3) | lida do autor (light), carried from 9fe6ed2 | `Ok` C1 · `ToolNotFound{ffmpeg}` C6 · `ToolNotFound{ffprobe}` C7 | - |
| `fala-cli import` exit codes (3) | lida do autor (light), carried from 9fe6ed2 | `0` C18 · `1` C21 · `2` C22, C23 | - |
| `fala-cli import` flags (2) | lida do autor (light), carried from 9fe6ed2 | `<arquivo>` C18 · `--out` C18, C19 | - |
| falhas que não deixam `out` nem `.part` (6) | verified at d65b0cb: proofs novas lidas e discriminadas por mutante | `InputNotFound` C8 · `NoAudioTrack` C9 · `Unreadable` C10 · `TooLong` C11 (param antes do ffmpeg) · `Ffmpeg` C12 com `.part` no disco (`crates/media/tests/import.rs:575`, `:597`) · `Cancelled` C15 com `.part` no disco (`:546`, `:561`), C16 | - |
| doors (5) | lida do autor (light), carried from 9fe6ed2 | 1 C24 · 2 C1, C25 · 3 C6, C7 · 4 C25 (C17 não discrimina a whitelist, como o próprio `checks.md` agora diz) · 5 C2, C13, C15 | - |
| eventos de progresso (3) | lida do autor (light), carried from 9fe6ed2 | inicial em 0 C13, C20 · por bloco `progress=` C14 · último = duração C14 | - |

## Faults injected

O perfil `light` não roda injeção de faults. O brief da rodada 2 pediu uma checagem de
discriminação para C12 e C15, e fiz mais um mutante barato sobre a regra "renomear só com status 0",
que a rodada 1 achou sem asserção que a discriminasse. Ambos em `git worktree add --detach
<scratchpad>/mut HEAD`, verified at d65b0cb, com o mesmo `CARGO_TARGET_DIR`. Depois veio
`git worktree remove --force`. O `git status --porcelain` do worktree real ficou idêntico ao baseline
(`?? .specs/features/media-import/verification.md`, `diff` vazio).

| Mutation | Location | Killed |
| --- | --- | --- |
| remover `if converted.is_err() { let _ = fs::remove_file(&part); }` | `crates/media/src/lib.rs:142-144` | yes - `cancel_while_running_kills_and_cleans` FAILED em `crates/media/tests/import.rs:561` (`!part.exists()`), `ffmpeg_killed_after_writing_leaves_nothing` FAILED em `:597` (`!part.exists()`); `ffmpeg_failure_is_typed_with_stderr` e `output_appears_only_after_success` seguem verdes, como esperado (diretório inexistente e caminho de sucesso) |
| `if !status.success()` -> `if false && !status.success()` (renomeia com qualquer status) | `crates/media/src/lib.rs:236` | yes - `ffmpeg_killed_after_writing_leaves_nothing` FAILED em `:594` (`esperava Ffmpeg, veio Ok(ImportedAudio { ..., duration: 1800s })`) |

Medição à parte, sem mutar código: rodei o ffmpeg do sistema com a argv da door 4 sobre um WAV
idêntico de 30 min, 3 vezes, em diretório de scratchpad. O `.part` aparece em 0,15-0,2 s e o
processo sai em 1,09-1,35 s com status 0. Com `pkill -TERM -f <part>` assim que o `.part` aparece,
o pkill acha o processo (rc 0), mas o ffmpeg 7.1.1 termina a conversão inteira (`.part` com
172 800 078 B, o tamanho cheio) e sai com 255. Ver gap 1.

## Gate

verified at d65b0cb:

- `cargo test -p fala-media -- --include-ignored`: 25 passed (5 lib + 20 integração), 0 failed, 0 ignored.
- `cargo test -p fala-cli --test import -- --include-ignored`: 7 passed, 0 failed.
- `cargo test -p fala-cli --bin fala-cli -- --exact import::tests::progress_line_formats_known_and_unknown_total`: 1 passed, 0 failed, 14 filtered out.
- `scripts/check-no-tauri-in-crates.sh`: exit 0. `grep -c` do Code Map: 1.
- Total: 33 testes passaram e 0 falharam. Os 25 checks têm evidência localizada e os 25 deram PASS.
  Os dois mutantes foram mortos.

## Ranked gaps

Nenhum bloqueia. Três observações de precisão:

1. **C12: o SIGTERM não interrompe a escrita.** No ffmpeg 7.1.1, o ffmpeg que recebe SIGTERM ainda
   termina a conversão (`.part` cheio) e só depois sai com 255. Medido aqui, e confirmado pelo
   `Ok(... duration: 1800s)` do segundo mutante. A proof garante o que o check diz: o sinal chega
   com o ffmpeg vivo e o `.part` no disco (`crates/media/tests/import.rs:575`, `:591`), o status
   ≠ 0 vira `Ffmpeg` e a limpeza é exigida (`:597`). Ela só não exercita um `.part` truncado. A
   janela entre o `.part` aparecer e o processo sair é de ~0,9 s nesta máquina. Numa máquina muito
   mais rápida o `pkill` pode chegar tarde, e aí `:591` falha alto (flake visível, não falso verde).
2. **AC 20 vs C20: "uma linha por evento".** O AC 20 pede uma linha de stderr por evento de
   progresso. C20 agora exige ≥ 2 linhas, a inicial exata e o total em todas
   (`apps/cli/tests/import.rs:123-129`), mas não compara o número de linhas com o número de
   eventos. É mais estrito que na rodada 1 e prova o que o check afirma.
3. **Tabela de Coverage do `checks.md` em desacordo com a própria nota.** A linha "doors" ainda
   lista `4 C25, C17`, e a nota logo abaixo diz que a whitelist é provada só por C25. É
   inconsistência de texto, sem efeito em nenhuma proof.
