# audio-retention verification

**Verdict**: PASS
**Profile**: light
**Diff range**: aa9b85c..ad684d6
**Round**: 3 - scoped
**Verifier**: independent sub-agent (author != verifier)

Os 18 checks passam. Todas as provas rodaram de novo numa só invocação em HEAD `ad684d6`, e cada assertiva foi localizada. A correção da rodada 3 é uma linha de teste (`crates/retention/src/encode.rs:340`) mais a redação de C3 em `checks.md`. Ela fecha a única lacuna que bloqueava na rodada 2: a ADR-0014 decide "aplicação VoIP", e agora C3 a afirma sobre o mesmo `new_encoder()` que a produção usa. A linha da ADR-0014 em Binding sources fica com `Uncovered` vazio, e nenhuma linha de Coverage tem membro sem prova. A lacuna baixa da rodada 2, a ligação `retain_wav` → `validate_opus`, continua registrada em Findings e não bloqueia: não é check nem membro de nenhum conjunto de cobertura.

## Binding sources

Verified at ad684d6 para a ADR-0014, porque a correção mexeu na decisão que ela fixa. As demais fontes vêm carried from da18647: a correção não tocou nas interfaces nem nos critérios que elas decidem.

| Source | Opened | Contradiction | Uncovered |
| --- | --- | --- | --- |
| ADR-0014 (`docs/decisions/0014-audio-de-reuniao-retido-em-dois-opus-mono.md`, proposed) - verified at ad684d6 | yes - Decisão (`:18`) e Confirmação relidas | none - "pacotes de 20 ms, aplicação VoIP, VBR com alvo de 24 kbps" (`:18`) bate com C1 (20 ms) e C3 (24 kbps, VBR, VoIP, este em `crates/retention/src/encode.rs:340`); o resto vem da rodada 2 | - |
| ADR-0005 (accepted) - carried from da18647 | yes (rodada 1) | none | - |
| Decisão 5, `fala-research/plans/roadmap-proposta-2026-10-02.md` §4 - carried from da18647 | yes (rodada 1) | none | - |
| Pitch fase 2, F3 / D1 / D2 / rabbit holes "Opus estéreo" e "Disco" - carried from da18647 | yes (rodadas 1 e 2) | none | - |

## Checks

Prova: uma invocação, `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2 cargo test -p fala-retention 2>&1`, em HEAD `ad684d6`, exit 0. Resultado: 9 passed no bloco `unittests src/lib.rs`, 12 passed em `tests/retention.rs`, 0 doc-tests. Todos os nomes citados abaixo aparecem como `... ok` nessa saída. A correção só inseriu uma linha em `encode.rs:340`, então as citações de `encode.rs` depois dela andaram uma linha (C8 e C18). `tests/retention.rs` e `policy.rs` não mudaram. Nos checks marcados "carried from da18647", o teste rodou de novo e o julgamento da assertiva vem da rodada 2.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | dois arquivos; 1º pacote `OpusHead` 1 canal 48 kHz; 2º `OpusTags`; todo pacote de áudio = 960 amostras - carried from da18647 | `encode::writes_two_mono_ogg_opus_files ... ok` | `crates/retention/tests/retention.rs:162` `assert!(head.starts_with(b"OpusHead"))`; `:163` `assert_eq!(head[9], 1, ..)`; `:168` `OpusTags`; `:169-174` `get_nb_samples(packet, RATE) == 960` | PASS |
| C2 | L → mic (RMS > 0,05), R → sys (RMS < 0,001) - carried from da18647 | `encode::left_is_mic_right_is_system ... ok` | `crates/retention/tests/retention.rs:186` `assert!(mic > 0.05)`; `:187` `assert!(system < 0.001)` | PASS |
| C3 | encoder a 24 000 bit/s, VBR ligado e aplicação VoIP; ≤ 40 000 B por arquivo de 10 s - verified at ad684d6 | `encode::tests::encoder_targets_24_kbps ... ok`, `encode::ten_seconds_fit_in_40_kb ... ok` | `crates/retention/src/encode.rs:337` `assert_eq!(encoder.get_bitrate().unwrap(), Bitrate::Bits(24_000))`; `:339` `assert!(encoder.get_vbr().unwrap())`; `:340` `assert_eq!(encoder.get_application().unwrap(), Application::Voip)`, os três sobre `new_encoder()` (`:336`), o mesmo da produção (`:202`); `crates/retention/tests/retention.rs:197` `assert!(size <= 40_000, ..)` | PASS |
| C4 | 3 formatos → `UnsupportedWav` com o formato, sem arquivos - carried from da18647 | `encode::rejects_other_formats ... ok` | `crates/retention/tests/retention.rs:207-212` `matches!(result, Err(UnsupportedWav { .. }) if ..)`; `:215` `assert!(names(&f.dir).is_empty())` | PASS |
| C5 | 480 007 quadros: `samples == 480 007`; `validate_opus(.., 480 007)` passa (granule exato; decodificação em [N, N + 959]) - carried from da18647 | `validate::sample_count_matches_wav ... ok` | `crates/retention/tests/retention.rs:229` `assert_eq!(retained.samples, 480_007)`; `:230-231` `validate_opus(..).unwrap()`; limites em `crates/retention/src/encode.rs:123` e `:129` | PASS |
| C6 | depois do sucesso o WAV some e a pasta é exatamente `{mic.opus, sys.opus}` - carried from da18647 | `validate::wav_deleted_only_after_success ... ok` | `crates/retention/tests/retention.rs:239` `assert!(!f.wav.exists())`; `:240` `assert_eq!(names(&f.dir), both())` | PASS |
| C7 | pasta impossível de criar: erro, WAV do mesmo tamanho - carried from da18647 | `validate::failure_keeps_wav ... ok` | `crates/retention/tests/retention.rs:251` `assert!(result.is_err())`; `:252` `assert_eq!(fs::metadata(&f.wav).unwrap().len(), size)` | PASS |
| C8 | `ValidationFailed` para truncado, para contagem + 1 (duas contagens na causa) e para granule de 10 quadros com 1 decodificado - verified at ad684d6 (só as linhas de `encode.rs` mudaram) | `validate::truncated_or_wrong_count_fails ... ok`, `encode::tests::decoded_count_below_granule_fails ... ok` | `crates/retention/tests/retention.rs:281-283` (truncado), `:288-290` causa com as duas contagens, `panic!` em `:292`; `crates/retention/src/encode.rs:387` granule `pre_skip + 10*960`; `:391-393` `Err(ValidationFailed { reason, .. })` com `reason.contains("decodificadas")` e a esperada; `panic!` em `:395` | PASS |
| C9 | `.part` com lixo: termina com `{mic.opus, sys.opus}` válidos - carried from da18647 | `validate::rerun_overwrites_leftover_parts ... ok` | `crates/retention/tests/retention.rs:308` `assert_eq!(names(&f.dir), both())`; `:309-310` `validate_opus(..).unwrap()` | PASS |
| C10 | `default() == Keep`; `Keep` nunca vence - carried from da18647 | `policy::tests::keep_never_deletes ... ok` | `crates/retention/src/policy.rs:113`; `:120` | PASS |
| C11 | 30 d e 31 d vencem; 30 d − 1 min não - carried from da18647 | `policy::tests::delete_after_days_boundary ... ok` | `crates/retention/src/policy.rs:129-132` | PASS |
| C12 | `DeleteAfterTranscript` vence exatamente as confirmadas - carried from da18647 | `policy::tests::delete_after_transcript ... ok` | `crates/retention/src/policy.rs:140-147` | PASS |
| C13 | `ended_at = None` nunca vence - carried from da18647 | `policy::tests::unfinished_session_never_due ... ok` | `crates/retention/src/policy.rs:163-166` | PASS |
| C14 | 0 e 3651 → `DaysOutOfRange`; 1 e 3650 aceitos - carried from da18647 | `policy::tests::days_bounds ... ok` | `crates/retention/src/policy.rs:173-176`; `:179-182` | PASS |
| C15 | formas serializadas vão e voltam - carried from da18647 | `policy::tests::serialized_forms ... ok` | `crates/retention/src/policy.rs:199`; `:200-203` | PASS |
| C16 | apaga só `root/<A>/` e devolve `true` - carried from da18647 | `delete::removes_only_that_session ... ok` | `crates/retention/tests/retention.rs:330`; `:331-333` | PASS |
| C17 | pasta inexistente: `Ok(false)` - carried from da18647 | `delete::missing_dir_is_false ... ok` | `crates/retention/tests/retention.rs:340` | PASS |
| C18 | WAV fica com o mesmo tamanho quando a validação recusa e quando o renomeio falha depois da validação - verified at ad684d6 (só as linhas de `encode.rs` mudaram) | `encode::tests::rejected_validation_keeps_wav ... ok`, `validate::rename_failure_after_validation_keeps_wav ... ok` | `crates/retention/src/encode.rs:370-373` `matches!(result, Err(ValidationFailed { .. }))`; `:374` `assert_eq!(fs::metadata(&wav).unwrap().len(), size)`; `:375-376` sem `mic.opus`/`sys.opus`; `crates/retention/tests/retention.rs:264-267` `Err(RetentionError::Io { .. })`; `:268` mesmo tamanho | PASS |

### C3, re-julgado

A assertiva nova discrimina, e isso foi conferido por leitura sem mexer no código. `Encoder::get_application` (`opus` 0.3.1, `src/lib.rs:564-567`) lê `OPUS_GET_APPLICATION` do libopus e o converte por `Application::from_raw` (`src/lib.rs:39-47`), que distingue Voip, Audio e LowDelay e devolve erro para qualquer outro valor. `new_encoder` (`crates/retention/src/encode.rs:276-283`) cria o encoder com `Application::Voip` (`:278`) e depois só chama `set_bitrate`, sem nada que redefina a aplicação. Se `:278` passasse a usar `Audio` ou `LowDelay`, `:340` falharia. O teste roda sobre `new_encoder()`, a mesma função que a produção chama em `:202`, então o que ele afirma é o encoder real.

O resto da análise adversarial (C18, ordem `encode_channels` → `validate` ×2 → `rename` ×2 → `sync_dir` → `remove_file`) vem carried from da18647. As citações andaram uma linha só depois de `encode.rs:340`, e o código de produção (`:39-75`) não mudou.

## Coverage

Não foi recomputado (perfil `light`). Abaixo, a linha que a correção tocou é re-julgada por leitura contra o `checks.md` atual (verified at ad684d6). As outras vêm carried from da18647.

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| falhas antes de apagar o WAV (3) - carried from da18647 | `encode.rs:62,69-70,72-73`, ADR-0014 Confirmação | pasta impossível C7 · validação recusada C18 · renomeio falho C18 | - |
| falhas da validação (3 ramos) - carried from da18647 | `encode.rs:96/120` (Ogg/EOS), `:123` (granule), `:129` (decodificação) | Ogg/EOS: C8 truncado · granule: C8 +1 · decodificação abaixo: C8 `decoded_count_below_granule_fails` | - |
| Landing door 3 + ADR-0014 Decisão, traços do encoder - verified at ad684d6 | plano Landing 3, ADR-0014 Decisão (`:18`) | mono/48 kHz C1 · 20 ms C1 · 24 kbps C3 · VBR C3 · aplicação VoIP C3 (`encode.rs:340`) · granule final C5 | - |

O lado de cima do ramo `encode.rs:129` (decodificação ≥ N + 960) não é exercitado. Isso não é lacuna, porque C8 não o alega (carried from da18647).

## Findings

1. Baixo, não bloqueia (carried from da18647, ainda aberto): o `retain_wav` público liga o validador real numa única linha, `crates/retention/src/encode.rs:39` `retain_wav_with(wav, session_dir, validate_opus)`. Nenhum teste faz `validate_opus` recusar pelo caminho público. Um `retain_wav` que passasse `|_, _| Ok(())` ainda passaria nos 21 testes. Pelas regras de `verify.md` isso não bloqueia: não é check, não é membro de nenhum conjunto de Coverage do `checks.md` (o membro "validação recusada" está provado por C18 na costura `retain_wav_with`), e o perfil `light` não injeta falhas. A ligação foi conferida por leitura. Para fechar, seria preciso um teste público que entregue a `retain_wav` um WAV cujo Opus o validador real recuse, e o código hoje não oferece um jeito barato de produzir esse WAV.
2. Informativo (carried from da18647): `rename_failure_after_validation_keeps_wav` afirma só a variante `Io { .. }`, sem conferir o caminho. Por leitura, nenhum outro passo devolve `Io` nesse cenário.

## Swept

Carried from da18647. A correção é uma assertiva de teste e não muda modos de falha, transições de estado, validação, idempotência, ciclo de vida dos dados nem falha de dependência.

## Gate

- `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2 cargo test -p fala-retention 2>&1` em `ad684d6`: 21 passed (9 unit + 12 integration), 0 failed, 0 doc-tests; exit 0
- `cargo deny`, `check-no-tauri-in-crates.sh`, clippy: carried from da18647 (a correção não mexe em `Cargo.toml` nem em dependências; a linha nova usa `Application`, já importado em `encode.rs:6`)

## Round 2

Rodada 2 - scoped, em `da18647`, veredito FAIL. Os 18 checks passaram, e o FAIL veio de uma linha de Binding sources:

1. "Aplicação VoIP" (ADR-0014, Decisão) sem check: **fechado** nesta rodada por C3 (`encode.rs:340`).
2. Ligação `retain_wav` → `validate_opus` sem teste que a faça falhar pelo caminho público: baixo, não bloqueante, **aberto** (Findings 1).

## Round 1

Rodada 1 - full, em `14ca852`, veredito FAIL. Os 17 checks de então passaram, e o FAIL veio de cobertura:

1. Porta 5 e ADR-0014 Confirmação ("o WAV continua no disco quando a validação falha") sem prova: **fechado** na rodada 2 por C18.
2. Ramo de decodificação de `validate_opus` nunca falhava: **fechado** na rodada 2 por C8 e pela redação nova de C5.
3. C8 não conferia a causa: **fechado** na rodada 2.
4. 20 ms, VBR e aplicação VoIP sem check: 20 ms e VBR **fechados** na rodada 2 (C1, C3), VoIP **fechado** na rodada 3 (C3).
5. Sem `fsync` da pasta antes do `unlink` do WAV: **fechado** por leitura na rodada 2 (`encode.rs:74`).
