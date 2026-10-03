# audio-retention verification

**Verdict**: FAIL
**Profile**: light
**Diff range**: aa9b85c..da18647
**Round**: 2 - scoped
**Verifier**: independent sub-agent (author != verifier)

Os 18 checks passam, cada um com teste existente, executado em HEAD `da18647` e assertiva localizada. A correção fechou a lacuna principal da rodada 1: a porta 5 ("apagar o WAV só depois de validar") agora tem prova (C18), e quem mudar `fs::remove_file(wav)` para antes da validação ou dos renomeios quebra pelo menos um dos dois testes (ver C18). O ramo de decodificação de `validate_opus` passou a ser exercitado (C8). A causa da falha agora é conferida (C8). Pacotes de 20 ms e VBR ganharam assertivas (C1 e C3). A pasta recebe `fsync` antes do `unlink` (`encode.rs:74`, `:303-307`).

Fica uma lacuna da rodada 1 que a correção não tratou, e é ela que dá o FAIL. A ADR-0014 decide "aplicação VoIP", e a rodada 1 a listou junto de 20 ms e VBR. Nenhum check a afirma. O código a define (`crates/retention/src/encode.rs:278`, `Application::Voip`), e o crate `opus` 0.3 expõe `Encoder::get_application()`, então a prova cabe numa linha em `encode::tests::encoder_targets_24_kbps`. Como a linha ADR-0014 em Binding sources fica com `Uncovered` não vazio, o veredito é FAIL.

## Binding sources

Verified at da18647 para a ADR-0014 (a correção mexeu nas linhas que ela cobre). Carried from 14ca852 para o resto: a correção não tocou nas interfaces nem nos critérios que essas fontes decidem.

| Source | Opened | Contradiction | Uncovered |
| --- | --- | --- | --- |
| ADR-0014 (`docs/decisions/0014-audio-de-reuniao-retido-em-dois-opus-mono.md`, proposed) - verified at da18647 | yes - lida inteira de novo | none - dois Opus mono, 48 kHz, 24 kbps, `audio/<id>/{mic,sys}.opus`, `.part` renomeado só depois de validar; "a decodificação devolve exatamente o número de amostras do WAV" bate com C5, porque o corte pelo granule final (RFC 7845) dá exatamente N, e a redação nova de C5 separa granule (exato) de decodificação bruta (N a N + 959) | "aplicação VoIP" (Decisão): nenhum check a afirma; o código a define em `encode.rs:278` e `opus::Encoder::get_application()` existe (`opus` 0.3.1, `src/lib.rs:564`). A Confirmação "o WAV continua no disco quando a validação falha" agora está coberta por C18; "pacotes de 20 ms" por C1; "VBR" por C3 |
| ADR-0005 (accepted) - carried from 14ca852 | yes (rodada 1) | none | - |
| Decisão 5, `fala-research/plans/roadmap-proposta-2026-10-02.md` §4 - carried from 14ca852 | yes (rodada 1) | none | - |
| Pitch fase 2, F3 / D1 / D2 / rabbit holes "Opus estéreo" e "Disco" - verified at da18647 para "Disco", carried from 14ca852 para o resto | yes | none | - ("apagar o WAV só depois de o Opus ser validado (decodificado de volta, duração conferida)": C18 + C5 + C8) |

## Checks

Prova: uma invocação, `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2 cargo test -p fala-retention 2>&1`, em HEAD `da18647`, exit 0. Resultado: 9 passed no bloco `unittests src/lib.rs`, 12 passed em `tests/retention.rs`, 0 doc-tests. Cada nome citado abaixo aparece como `... ok` nessa saída, inclusive os três novos: `encode::tests::rejected_validation_keeps_wav`, `encode::tests::decoded_count_below_granule_fails` e `validate::rename_failure_after_validation_keeps_wav`. Todos estão em arquivos deste diff (`crates/retention/**`). Os testes rodaram de novo para todos os checks. Nos marcados "carried from 14ca852", o arquivo não mudou ou só mudaram as linhas, e o julgamento da assertiva vem da rodada 1.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | dois arquivos; 1º pacote `OpusHead` 1 canal 48 kHz; 2º `OpusTags`; todo pacote de áudio = 960 amostras - verified at da18647 | `encode::writes_two_mono_ogg_opus_files ... ok` | `crates/retention/tests/retention.rs:157-158` caminhos `mic.opus`/`sys.opus`; `:162` `assert!(head.starts_with(b"OpusHead"))`; `:163` `assert_eq!(head[9], 1, ..)`; `:164-167` taxa `48_000`; `:168` `assert!(all[1].starts_with(b"OpusTags"))`; `:169-174` `for packet in &all[2..] { assert_eq!(opus::packet::get_nb_samples(packet, RATE).unwrap(), 960, ..) }` | PASS |
| C2 | L → mic (RMS > 0,05), R → sys (RMS < 0,001) - verified at da18647 (só as linhas mudaram) | `encode::left_is_mic_right_is_system ... ok` | `crates/retention/tests/retention.rs:186` `assert!(mic > 0.05)`; `:187` `assert!(system < 0.001)` | PASS |
| C3 | encoder a 24 000 bit/s com VBR ligado; ≤ 40 000 B por arquivo de 10 s - verified at da18647 | `encode::tests::encoder_targets_24_kbps ... ok`, `encode::ten_seconds_fit_in_40_kb ... ok` | `crates/retention/src/encode.rs:337` `assert_eq!(encoder.get_bitrate().unwrap(), Bitrate::Bits(24_000))`; `:339` `assert!(encoder.get_vbr().unwrap())`, os dois sobre `new_encoder()`, o mesmo que a produção usa (`encode.rs:202`); `crates/retention/tests/retention.rs:197` `assert!(size <= 40_000, ..)` | PASS |
| C4 | 3 formatos → `UnsupportedWav` com o formato, sem arquivos - verified at da18647 (só as linhas mudaram) | `encode::rejects_other_formats ... ok` | `crates/retention/tests/retention.rs:207-212` `matches!(result, Err(UnsupportedWav { sample_rate, channels: ch, bits: b }) if ..)`; `:215` `assert!(names(&f.dir).is_empty())` | PASS |
| C5 | 480 007 quadros: `samples == 480 007`; `validate_opus(.., 480 007)` passa (granule exato; decodificação em [N, N + 959]) - verified at da18647 | `validate::sample_count_matches_wav ... ok` | `crates/retention/tests/retention.rs:229` `assert_eq!(retained.samples, 480_007)`; `:230-231` `validate_opus(&retained.mic / .system, 480_007).unwrap()`. Os limites da redação nova são as condições de `validate_opus`: `encode.rs:123` `by_granule != expected_samples` e `:129` `by_decoding < expected_samples \|\| by_decoding >= expected_samples + FRAME` | PASS |
| C6 | depois do sucesso o WAV some e a pasta é exatamente `{mic.opus, sys.opus}` - verified at da18647 (só as linhas mudaram) | `validate::wav_deleted_only_after_success ... ok` | `crates/retention/tests/retention.rs:239` `assert!(!f.wav.exists())`; `:240` `assert_eq!(names(&f.dir), both())` | PASS |
| C7 | pasta impossível de criar: erro, WAV do mesmo tamanho - verified at da18647 | `validate::failure_keeps_wav ... ok` | `crates/retention/tests/retention.rs:251` `assert!(result.is_err())`; `:252` `assert_eq!(fs::metadata(&f.wav).unwrap().len(), size)`. Falha em `encode.rs:62` (`create_dir_all`) | PASS |
| C8 | `ValidationFailed` para truncado, para contagem + 1 (com as duas contagens na causa) e para granule de 10 quadros com 1 decodificado (causa cita decodificadas e a esperada) - verified at da18647 | `validate::truncated_or_wrong_count_fails ... ok`, `encode::tests::decoded_count_below_granule_fails ... ok` | `crates/retention/tests/retention.rs:281-283` `matches!(result, Err(ValidationFailed { .. }))` (truncado); `:288-290` `Err(ValidationFailed { reason, .. })` com `reason.contains(&expected.to_string())` e `reason.contains(&retained.samples.to_string())`, e qualquer outro resultado cai no `panic!` de `:292`; `crates/retention/src/encode.rs:390-392` `Err(ValidationFailed { reason, .. })` com `reason.contains("decodificadas")` e `reason.contains(&expected.to_string())`, `panic!` em `:394`. O teste novo chega ao ramo `encode.rs:128-133`: o granule (`pre_skip + 10*960`, `:386`) passa em `:123`, e só um pacote é decodificado | PASS |
| C9 | `.part` com lixo: termina com `{mic.opus, sys.opus}` válidos - verified at da18647 (só as linhas mudaram) | `validate::rerun_overwrites_leftover_parts ... ok` | `crates/retention/tests/retention.rs:308` `assert_eq!(names(&f.dir), both())`; `:309-310` `validate_opus(..).unwrap()` | PASS |
| C10 | `default() == Keep`; `Keep` nunca vence - carried from 14ca852 (`policy.rs` intocado) | `policy::tests::keep_never_deletes ... ok` | `crates/retention/src/policy.rs:113` `assert_eq!(RetentionPolicy::default(), RetentionPolicy::Keep)`; `:120` `assert!(audio_due(Keep, &sessions, NOW).is_empty())` | PASS |
| C11 | 30 d e 31 d vencem; 30 d − 1 min não - carried from 14ca852 | `policy::tests::delete_after_days_boundary ... ok` | `crates/retention/src/policy.rs:129-132` `assert_eq!(audio_due(policy, &[almost, exactly, older], NOW), vec![exactly.id, older.id])` | PASS |
| C12 | `DeleteAfterTranscript` vence exatamente as confirmadas - carried from 14ca852 | `policy::tests::delete_after_transcript ... ok` | `crates/retention/src/policy.rs:140-147` `assert_eq!(audio_due(..), vec![confirmed_new.id, confirmed_old.id])` | PASS |
| C13 | `ended_at = None` nunca vence - carried from 14ca852 | `policy::tests::unfinished_session_never_due ... ok` | `crates/retention/src/policy.rs:163-166` `assert!(audio_due(policy, &[unfinished], later).is_empty())` | PASS |
| C14 | 0 e 3651 → `DaysOutOfRange`; 1 e 3650 aceitos - carried from 14ca852 | `policy::tests::days_bounds ... ok` | `crates/retention/src/policy.rs:173-176` `matches!(.., Err(DaysOutOfRange(d)) if d == days)`; `:179-182` `assert_eq!(delete_after_days(days).unwrap(), ..)` | PASS |
| C15 | formas serializadas vão e voltam - carried from 14ca852 | `policy::tests::serialized_forms ... ok` | `crates/retention/src/policy.rs:199` `assert_eq!(serde_json::to_string(&policy).unwrap(), json)`; `:200-203` `from_str` de volta | PASS |
| C16 | apaga só `root/<A>/` e devolve `true` - verified at da18647 (só as linhas mudaram) | `delete::removes_only_that_session ... ok` | `crates/retention/tests/retention.rs:330` `assert!(delete_session_audio(root, a).unwrap())`; `:331-333` `!session_audio_dir(root, a).exists()`, B e `solto.txt` existem | PASS |
| C17 | pasta inexistente: `Ok(false)` - verified at da18647 (só as linhas mudaram) | `delete::missing_dir_is_false ... ok` | `crates/retention/tests/retention.rs:340` `assert!(!delete_session_audio(tmp.path(), c).unwrap())` | PASS |
| C18 | WAV fica com o mesmo tamanho quando a validação recusa (sem `mic.opus`/`sys.opus` finais) e quando o renomeio falha depois da validação - verified at da18647 | `encode::tests::rejected_validation_keeps_wav ... ok`, `validate::rename_failure_after_validation_keeps_wav ... ok` | `crates/retention/src/encode.rs:369-372` `matches!(result, Err(ValidationFailed { .. }))`; `:373` `assert_eq!(fs::metadata(&wav).unwrap().len(), size)`; `:374-375` `!dir.join(MIC_FILE).exists()`, `!dir.join(SYSTEM_FILE).exists()`; `crates/retention/tests/retention.rs:264-267` `matches!(result, Err(RetentionError::Io { .. }))`; `:268` `assert_eq!(fs::metadata(&f.wav).unwrap().len(), size)` | PASS |

### C18 adversarial

Julgado pela assertiva, sem mexer no código. Ordem em HEAD: `encode_channels` (`encode.rs:68`) → `validate` ×2 (`:69-70`) → `rename` mic (`:72`) → `rename` sys (`:73`) → `sync_dir` (`:74`) → `remove_file(wav)` (`:75`).

- `remove_file` movido para qualquer ponto antes da validação (até antes de `encode_channels`; no Linux o `WavReader` já aberto em `:48` continua lendo depois do `unlink`): `rejected_validation_keeps_wav` chega ao validador injetado, recebe `Err` e a linha `:373` faz `fs::metadata(&wav).unwrap()` sobre um arquivo que não existe mais. O `unwrap` entra em pânico e o teste falha. `rename_failure_after_validation_keeps_wav` falha do mesmo jeito em `tests/retention.rs:268`.
- `remove_file` movido para entre a validação e os renomeios, ou para entre o rename de mic e o de sys: o validador injetado barra o primeiro teste antes disso, mas `rename_failure_after_validation_keeps_wav` passa pela validação real e falha no rename de `sys.opus` (`:73`, sobre uma pasta não vazia). O WAV já teria sumido, então `:268` entra em pânico.
- `validate(...)` removido das linhas `:69-70`: `retain_wav_with` devolveria `Ok` e o `matches!` de `encode.rs:369-372` falharia.
- Uma lacuna residual, que não muda o resultado de C18: o validador real é ligado ao `retain_wav` público só por `encode.rs:39`, `retain_wav_with(wav, session_dir, validate_opus)`. Um `retain_wav` público que passasse `|_, _| Ok(())` passaria em todos os 21 testes, porque nenhum teste faz `validate_opus` recusar dentro do `retain_wav` público. A ligação é uma linha sem lógica, conferida por leitura. Sob `light` não há injeção de falhas para matar esse mutante.
- O teste do rename só afirma a variante `Io { .. }`, sem conferir o caminho `sys.opus`. Por leitura, nenhum outro passo devolve `Io` naquele cenário: a pasta existe e os `.part` têm nomes livres.

Nível e amostragem: carried from 14ca852. Os checks de arquivo rodam pela API pública sobre WAVs e Opus reais. Os dois testes unitários novos ficam em `encode::tests` porque `retain_wav_with` e `OpusStream` são privados, e esse é o nível certo para uma costura de injeção.

## Coverage

Não recomputado (perfil `light`). As linhas abaixo re-julgam, por leitura, as lacunas da rodada 1 contra o `checks.md` atualizado. Verified at da18647.

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| falhas antes de apagar o WAV (3) | `encode.rs:62,69-70,72-73`, ADR-0014 Confirmação | pasta impossível C7 · validação recusada C18 · renomeio falho C18 | - |
| falhas da validação (3 ramos) | `encode.rs:96/120` (Ogg/EOS), `:123` (granule), `:129` (decodificação) | Ogg/EOS: C8 truncado · granule: C8 +1 · decodificação abaixo: C8 `decoded_count_below_granule_fails` | - (o lado de cima do ramo `:129`, ≥ N + 960, não é exercitado; C8 não o alega) |
| Landing door 3 + ADR-0014 Decisão, traços do encoder | plano Landing 3, ADR-0014 Decisão | mono/48 kHz C1 · 20 ms C1 · 24 kbps C3 · VBR C3 · granule final C5 | aplicação VoIP (ADR-0014 Decisão; `encode.rs:278`): nenhum check |

## Swept

Verified at da18647 para as linhas que a correção tocou. O resto vem de 14ca852.

- failure modes (C7, C8, C18): presente; C18 prova a ordem.
- state transitions ("`.part` -> validado -> renomeado -> WAV apagado"): a ordem está no código (`encode.rs:68-75`) e agora tem prova em C18.
- fsync da pasta: `sync_dir(session_dir)` em `encode.rs:74`, depois dos dois `rename` e antes do `remove_file`. É melhor esforço (`:303-307`, erros ignorados) e nenhum check o cobre. Ele fecha a lacuna 5 da rodada 1 por leitura. Não dá para provar com teste sem injetar um crash.
- validation, idempotency, data lifecycle, dependency failure, `n/a`s: carried from 14ca852.

## Gate

- `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2 cargo test -p fala-retention 2>&1` em `da18647`: 21 passed (9 unit + 12 integration), 0 failed, 0 doc-tests; exit 0
- `cargo deny`, `check-no-tauri-in-crates.sh`: carried from 14ca852 (a correção não mexe em `Cargo.toml` nem em dependências)

## Ranked gaps

1. "Aplicação VoIP" (ADR-0014, Decisão) sem check: o código a define em `crates/retention/src/encode.rs:278`, mas nada a afirma. Correção de uma linha: `assert_eq!(encoder.get_application().unwrap(), Application::Voip)` em `encode::tests::encoder_targets_24_kbps` e a redação de C3 estendida.
2. Baixo, não bloqueia: a ligação `retain_wav` → `validate_opus` (`encode.rs:39`) não tem teste que a faça falhar pelo caminho público. Ver C18 adversarial.

## Round 1

Rodada 1 - full, em `14ca852`, veredito FAIL. Os 17 checks de então passaram, e o FAIL veio de cobertura:

1. Porta 5 e ADR-0014 Confirmação ("o WAV continua no disco quando a validação falha") sem prova: **fechado** por C18 (dois testes novos).
2. O ramo de decodificação de `validate_opus` nunca falhava, e o AC 5 dizia "exatamente": **fechado** por C8 (`decoded_count_below_granule_fails`) e pela redação nova de C5, que separa granule exato de decodificação em [N, N + 959].
3. C8 não conferia a causa: **fechado** (assertivas sobre `reason`).
4. 20 ms, VBR e aplicação VoIP sem check: 20 ms **fechado** (C1), VBR **fechado** (C3), VoIP **aberto** (gap 1 acima).
5. Sem `fsync` da pasta antes do `unlink` do WAV: **fechado** por leitura (`encode.rs:74`).
