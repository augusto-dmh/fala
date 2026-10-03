# audio-retention verification

**Verdict**: FAIL
**Profile**: light
**Diff range**: aa9b85c..14ca852959655164b27e9ca1409dc160b79717c5
**Round**: 1 - full
**Verifier**: independent sub-agent (author != verifier)

Os 17 checks passam um a um, com teste existente, executado em HEAD e assertiva localizada. O FAIL vem da cobertura da porta 5 (apagar o WAV só depois de validar): a ADR-0014, na seção Confirmação, pede a prova de que "o WAV continua no disco quando a validação falha", e nenhum teste a faz. A linha de Coverage do `checks.md` atribui a porta 5 a C6 e C7, mas nenhum dos dois prova a ordem. Por leitura, o código está certo: em `crates/retention/src/encode.rs:60-65`, `remove_file(wav)` só é alcançado depois de `validate_opus` ×2 e `rename` ×2 retornarem `Ok`. O que falta é a prova. Uma implementação que apagasse o WAV antes de validar, ou que pulasse a validação, passaria nos 18 testes. C6 só olha o estado final do caminho feliz, e C7 falha em `create_dir_all` (`encode.rs:53`), antes de qualquer codificação.

## Binding sources

| Source | Opened | Contradiction | Uncovered |
| --- | --- | --- | --- |
| ADR-0014 (`docs/decisions/0014-audio-de-reuniao-retido-em-dois-opus-mono.md`, proposed, deste diff) | yes - lida inteira | none - dois Opus mono, 48 kHz, 24 kbps, `audio/<id>/{mic,sys}.opus`, libopus via `opus` 0.3 + `audiopus_sys` 0.2 `static`, `.part` renomeado só depois de validar: os checks C1, C2, C3, C5, C6 e C9 batem com isso | Confirmação "o WAV continua no disco quando a validação falha": nenhum teste faz `retain_wav` falhar na validação (ou no rename) e confere o WAV. C7 falha antes de codificar e C8 testa `validate_opus` isolado. Decisão "pacotes de 20 ms, aplicação VoIP, VBR": nenhum check afirma isso (C3 afirma só bitrate e taxa) |
| ADR-0005 (`docs/decisions/0005-...-localmente.md`, accepted) | yes - lida inteira | none - dois canais, "WAV durante, Opus depois", retenção configurável com padrão de manter (C10). O "~13 MB/h por canal" fica coberto com folga por C3 (≤ 40 000 B por 10 s ≈ 14,4 MB/h no teto, ~11 MB/h no alvo) | - |
| Decisão 5, `fala-research/plans/roadmap-proposta-2026-10-02.md` §4 | yes - seção 4 lida | none - "(a) dois Opus mono de 24 kbps com libopus compilado do fonte". O libopus foi compilado do fonte por CMake nesta máquina (`target/debug/build/audiopus_sys-57aa8fdf9a22b2ee/output`: "`pkg_config` could not find `Opus`" → "Linking Opus as static lib: .../out"). A ADR-0014 escreve "quando o sistema não o tem": numa máquina com libopus estático visível pelo pkg-config, o build ligaria o do sistema. É uma nuance que a própria ADR registra, não uma contradição | - |
| Pitch fase 2, F3 / D1 / D2 / rabbit holes "Opus estéreo" e "Disco" (`fala-research/pitches/fase-2-reuniao-videos.md`) | yes - F3, D1, D2 e rabbit holes lidos | none - F3 põe o encoder em `crates/audio`; a ADR-0014 o move para `fala-retention` e prevalece (ADR > pitch), registrado nas Assumptions do plano. Recuperação de WAV órfão, agendamento do job e "apagar sessão" inteira estão em Out of scope do plano, com motivo | "Disco": "apagar o WAV só depois de o Opus ser validado (decodificado de volta, duração conferida)", a mesma lacuna de prova da linha da ADR-0014 |

## Checks

Prova: uma invocação, `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2 cargo test -p fala-retention`, em HEAD `14ca852`, saída 0. Cada nome abaixo aparece como `... ok` nessa saída: 7 do bloco `unittests src/lib.rs` e 11 do bloco `tests/retention.rs`. Todos os testes nomeados estão em arquivos novos deste diff (`crates/retention/**`).

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | dois arquivos; 1º pacote `OpusHead` com 1 canal e 48 kHz; 2º `OpusTags` | `encode::writes_two_mono_ogg_opus_files ... ok` | `crates/retention/tests/retention.rs:162` - `assert!(head.starts_with(b"OpusHead"))`; `:163` - `assert_eq!(head[9], 1, ..)`; `:164-167` - `u32::from_le_bytes([head[12..16]]) == 48_000`; `:168` - `assert!(all[1].starts_with(b"OpusTags"))`; `:157-158` caminhos `mic.opus`/`sys.opus` | PASS |
| C2 | L → mic (RMS > 0,05), R → sys (RMS < 0,001) | `encode::left_is_mic_right_is_system ... ok` | `crates/retention/tests/retention.rs:179` - `assert!(mic > 0.05)`; `:180` - `assert!(system < 0.001)`. O tom vai em `c == 0` (`:68`), e o decode do teste (`:95-106`) não depende de `validate_opus`. Uma troca L/R faria as duas assertivas falharem | PASS |
| C3 | encoder a 24 000 bit/s; ≤ 40 000 B por arquivo de 10 s | `encode::tests::encoder_targets_24_kbps ... ok`, `encode::ten_seconds_fit_in_40_kb ... ok` | `crates/retention/src/encode.rs:318` - `assert_eq!(encoder.get_bitrate().unwrap(), Bitrate::Bits(24_000))` sobre `new_encoder()`, a mesma função que a produção usa (`encode.rs:192`); `crates/retention/tests/retention.rs:190` - `assert!(size <= 40_000, ..)` | PASS |
| C4 | mono 48k, estéreo 44,1k e estéreo 24 bits dão `UnsupportedWav` com o formato, sem arquivos | `encode::rejects_other_formats ... ok` | `crates/retention/tests/retention.rs:200-205` - `matches!(result, Err(UnsupportedWav { sample_rate, channels: ch, bits: b }) if sample_rate == rate && ch == channels && b == bits)`; `:208` - `assert!(names(&f.dir).is_empty())` | PASS |
| C5 | 480 007 quadros: `samples == 480 007` e `validate_opus(.., 480 007)` passa nos dois | `validate::sample_count_matches_wav ... ok` | `crates/retention/tests/retention.rs:222` - `assert_eq!(retained.samples, 480_007)`; `:223-224` - `validate_opus(&retained.mic, 480_007).unwrap()` / `system` | PASS |
| C6 | depois do sucesso o WAV some e a pasta é exatamente `{mic.opus, sys.opus}` | `validate::wav_deleted_only_after_success ... ok` | `crates/retention/tests/retention.rs:232` - `assert!(!f.wav.exists())`; `:233` - `assert_eq!(names(&f.dir), both())` | PASS |
| C7 | pasta da sessão impossível de criar: erro, WAV com o mesmo tamanho | `validate::failure_keeps_wav ... ok` | `crates/retention/tests/retention.rs:244` - `assert!(result.is_err())`; `:245` - `assert_eq!(fs::metadata(&f.wav).unwrap().len(), size)` | PASS |
| C8 | `ValidationFailed` para arquivo truncado na metade e para contagem + 1 | `validate::truncated_or_wrong_count_fails ... ok` | `crates/retention/tests/retention.rs:258-261` - `matches!(result, Err(RetentionError::ValidationFailed { .. }))` (truncado); `:263-267` - idem para `retained.samples + 1` | PASS |
| C9 | `.part` com lixo de uma tentativa anterior: termina com `{mic.opus, sys.opus}` válidos | `validate::rerun_overwrites_leftover_parts ... ok` | `crates/retention/tests/retention.rs:282` - `assert_eq!(names(&f.dir), both())`; `:283-284` - `validate_opus(..).unwrap()` | PASS |
| C10 | `default() == Keep`; `Keep` nunca vence (0, 1 e 10 000 dias, com e sem transcrição) | `policy::tests::keep_never_deletes ... ok` | `crates/retention/src/policy.rs:113` - `assert_eq!(RetentionPolicy::default(), RetentionPolicy::Keep)`; `:120` - `assert!(audio_due(RetentionPolicy::Keep, &sessions, NOW).is_empty())` | PASS |
| C11 | 30 dias: exatamente 30 e 31 vencem; 30 d − 1 min não | `policy::tests::delete_after_days_boundary ... ok` | `crates/retention/src/policy.rs:129-132` - `assert_eq!(audio_due(policy, &[almost, exactly, older], NOW), vec![exactly.id, older.id])`, com `almost = 30*DAY_MS - MINUTE_MS` (`:126`); código `:87` `>=` | PASS |
| C12 | `DeleteAfterTranscript` vence exatamente as confirmadas, de qualquer idade | `policy::tests::delete_after_transcript ... ok` | `crates/retention/src/policy.rs:140-147` - `assert_eq!(audio_due(DeleteAfterTranscript, &[confirmed_new, pending_old, confirmed_old], NOW), vec![confirmed_new.id, confirmed_old.id])` | PASS |
| C13 | `ended_at = None` nunca vence, com as 3 políticas, 10 000 dias depois | `policy::tests::unfinished_session_never_due ... ok` | `crates/retention/src/policy.rs:163-166` - `assert!(audio_due(policy, &[unfinished], later).is_empty())` dentro de `for policy in [Keep, delete_after_days(1), DeleteAfterTranscript]` | PASS |
| C14 | 0 e 3651 dão `DaysOutOfRange`; 1 e 3650 são aceitos | `policy::tests::days_bounds ... ok` | `crates/retention/src/policy.rs:173-176` - `matches!(delete_after_days(days), Err(DaysOutOfRange(d)) if d == days)` para `[0, 3651]`; `:179-182` - `assert_eq!(delete_after_days(days).unwrap(), DeleteAfterDays(..))` para `[1, 3650]` | PASS |
| C15 | `"keep"`, `{"delete_after_days":30}`, `"delete_after_transcript"` vão e voltam | `policy::tests::serialized_forms ... ok` | `crates/retention/src/policy.rs:199` - `assert_eq!(serde_json::to_string(&policy).unwrap(), json)`; `:200-203` - `assert_eq!(serde_json::from_str::<RetentionPolicy>(json).unwrap(), policy)` | PASS |
| C16 | apaga só `root/<A>/`, mantém `root/<B>/mic.opus` e `root/solto.txt`, devolve `true` | `delete::removes_only_that_session ... ok` | `crates/retention/tests/retention.rs:304` - `assert!(delete_session_audio(root, a).unwrap())`; `:305-307` - `!session_audio_dir(root, a).exists()`, `session_audio_dir(root, b).join("mic.opus").exists()`, `root.join("solto.txt").exists()` | PASS |
| C17 | pasta inexistente: `Ok(false)` | `delete::missing_dir_is_false ... ok` | `crates/retention/tests/retention.rs:314` - `assert!(!delete_session_audio(tmp.path(), c).unwrap())` | PASS |

Nível e amostragem: os checks de arquivo (C1-C9, C16, C17) rodam como teste de integração pela API pública (`tests/retention.rs`), sobre WAVs reais gerados com `hound` e Opus reais do libopus, sem mocks. Esse é o nível certo para uma biblioteca cuja superfície são arquivos em disco. C3 (bitrate) e C10-C15 são unitários sobre funções puras ou sobre o mesmo `new_encoder()` da produção. Amostragem: 3 formatos recusados de um espaço maior (float de 32 bits, 8 bits, 96 kHz não testados; o código recusa qualquer um que não seja `48000/2/16/Int`, `encode.rs:41-44`). As bordas de dias são exatas (0, 1, 3650, 3651; 30 d − 1 min, 30 d, 31 d).

Respostas às perguntas adversariais do brief:

- (a) Não há caminho, por leitura, em que o WAV seja apagado sem os dois `.part` validados e renomeados: `remove_file` (`encode.rs:65`) vem depois de `?` em `:60`, `:61`, `:63` e `:64`, e os dois `.part` passam por `sync_all` antes da validação (`encode.rs:262`). O que falta é a prova, como dito no topo. Lacuna menor: a pasta da sessão não recebe `fsync` depois dos dois `rename` e antes do `unlink` do WAV. Um crash nessa janela pode deixar os dados íntegros com nome `.part` e o WAV já apagado; nada se perde, mas `retain_wav` não teria como retomar.
- (b) Granule: a contagem esperada + 1 falha pelo granule exato (`encode.rs:113`). Truncamento: `PacketReader` dá erro de Ogg (`:86`) ou fica sem pacote EOS (`:110`), e os dois casos viram `ValidationFailed`. A checagem por decodificação (`encode.rs:118-123`) existe, mas nenhum teste a faz falhar: os dois casos de C8 param antes, no granule ou no Ogg/EOS. Além disso, ela aceita de N a N + 959 amostras. O AC 5 do plano ("decodificar ... SHALL devolver exatamente esse número") só vale no sentido da RFC 7845, com o corte pelo granule final; a contagem bruta decodificada não é exata.
- (c) L vai para mic e R para sys: `encode_channels` lê os pares intercalados `l`, `r` (`encode.rs:150-155`) e escreve `left` em `mic` e `right` em `system` (`:163-164`). C2 pegaria a troca.
- (d) Sem off-by-one: `now - ended >= days * DAY_MS` (`policy.rs:87`), com `saturating_sub` (um fim no futuro não vence) e sem overflow (3650 × 86 400 000 cabe em `u64`).
- (e) `delete_session_audio` não sai da pasta: o caminho é `audio_root.join(id.to_string())` (`lib.rs:58-59`), e `SessionId` só se escreve como 26 caracteres Crockford (`crates/meeting/src/id.rs:45-54`), sem `/` nem `..`. `std::fs::remove_dir_all` não segue symlinks.
- (f) Nenhum check alega mais do que o próprio teste prova. Quem alega mais é a linha "Landing doors" do Coverage do `checks.md`, que dá a porta 5 como provada por C6 e C7, e a linha `state transitions` do Swept ("C6 - `.part` -> renomeado -> WAV apagado, nessa ordem"): C6 não observa a ordem.
- (g) Nenhuma contradição entre os checks e a ADR-0014 ou a decisão 5. A única lacuna em relação a elas é a de cobertura listada em Binding sources.

## Coverage

Não recomputado (perfil `light`). As linhas abaixo são lacunas notadas durante a leitura, não um recálculo do join.

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| Landing doors (5), porta 5 "apagar o WAV" | plano (Landing 5), ADR-0014 (Obrigatório e Confirmação), pitch rabbit hole "Disco" | sucesso → WAV apagado: C6 · falha antes de codificar → WAV mantido: C7 | WAV mantido quando a validação ou o rename falham dentro de `retain_wav`: nenhum teste. Apagar antes de validar, ou não validar, passa em C6 e C7 |
| Landing door 3, forma literal (6 traços) | plano (Landing 3), ADR-0014 Decisão | mono/48 kHz: C1 · 24 kbps: C3 · granule final = pre-skip + amostras: C5 (via `validate_opus`, `encode.rs:113`) | pacotes de 20 ms, VBR e aplicação VoIP: nenhum check |
| falhas da validação (3 ramos no código) | `encode.rs:86,110,113,119` | Ogg/EOS: C8 (truncado) · granule: C8 (+1) | ramo da contagem por decodificação (`encode.rs:119`): nenhum teste o faz falhar |

## Swept

Relido contra o código (linhas que resolvem para algo existente):

- validation (C4, C14): presente, `encode.rs:41-51` e `policy.rs:17-23`.
- failure modes (C7, C8): presente; ver a lacuna da porta 5.
- idempotency (C9): `File::create` trunca o `.part` (`encode.rs:196`); presente.
- data lifecycle: presente.
- dependency failure: erros do libopus passam por `map_err(opus_error)?` (`encode.rs:193,241,268-271`), sem pânico; presente.
- state transitions (".part -> renomeado -> WAV apagado, nessa ordem"): a ordem está no código (`encode.rs:59-65`), mas a atribuição a C6 não prova a ordem (ver acima).
- authorization, concurrency, observability: `n/a` aprovado; o crate de fato não depende de `log` (`crates/retention/Cargo.toml`).

## Gate

- `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2 cargo test -p fala-retention`: 18 passed (7 unit + 11 integration), 0 failed, 0 doc-tests; exit 0
- `cargo deny check licenses bans sources`: `bans ok, licenses ok, sources ok`, exit 0
- `bash scripts/check-no-tauri-in-crates.sh`: `ok: no tauri in crates/`, exit 0
- Build do libopus: compilado do fonte por CMake e ligado estático (`audiopus_sys` 0.2.2, a única versão no `Cargo.lock`)

## Ranked gaps

1. Porta 5 e ADR-0014 Confirmação sem prova: nenhum teste faz `retain_wav` falhar depois da codificação (na validação ou no rename) e confere que o WAV ficou. Apagar antes de validar, ou não validar, passa em todos os 18 testes. Afeta a linha de Coverage de C6/C7 e a linha `state transitions` do Swept. `crates/retention/src/encode.rs:60-65`, sem evidência de teste. Correção sugerida: um teste com `sys.opus` pré-criado como diretório (o `rename` falha depois da validação; WAV mantido com o mesmo tamanho) e um teste de unidade com um ponto de falha na validação (por exemplo, a contagem esperada adulterada), conferindo o WAV.
2. O ramo da contagem por decodificação em `validate_opus` nunca é feito falhar, e tolera até 959 amostras a mais (`encode.rs:118-123`). O AC 5 diz "exatamente", o que vale só pelo granule. É uma lacuna de precisão no AC e no C5.
3. O AC 8 pede `ValidationFailed` "com as duas contagens ou a causa"; C8 afirma só a variante (`{ .. }`, `tests/retention.rs:259,265`). É uma lacuna de precisão no check.
4. Pacotes de 20 ms, VBR e aplicação VoIP (ADR-0014, porta 3) sem check.
5. Sem `fsync` da pasta da sessão entre os `rename` e o `unlink` do WAV (`encode.rs:63-65`): numa queda nessa janela a recuperação fica só no `.part`. Baixo.
