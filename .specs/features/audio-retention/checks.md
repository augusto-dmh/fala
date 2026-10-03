# audio-retention checks

Profile: light
Plan: `.specs/features/audio-retention/plan.md`

17 checks in 4 slices · 5 one-way doors · 0 open, of which 0 block

Todas as provas rodam com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2` (regra da rodada). Os WAVs de teste são gerados pelos próprios testes numa pasta temporária; nada de microfone nem de rede.

## Checks

### S1 - WAV vira dois Opus mono · ~4 files · ~25 KB · ~7k

**C1** - De um WAV estéreo 48 kHz i16 de 10 s, `retain_wav` deixa `mic.opus` e `sys.opus` na pasta da sessão; em cada um o 1º pacote Ogg começa com `OpusHead`, tem 1 canal e 48 000 Hz, e o 2º começa com `OpusTags` (AC 1)
Proof: `cargo test -p fala-retention --test retention encode::writes_two_mono_ogg_opus_files -- --exact`

**C2** - Com tom de 440 Hz em L e silêncio digital em R, o RMS decodificado de `mic.opus` é > 0,05 e o de `sys.opus` é < 0,001 (AC 2)
Proof: `cargo test -p fala-retention --test retention encode::left_is_mic_right_is_system -- --exact`

**C3** - O encoder configurado reporta 24 000 bit/s, e cada arquivo de 10 s de tom com ruído tem no máximo 40 000 bytes (AC 3)
Proof: `cargo test -p fala-retention --lib encode::tests::encoder_targets_24_kbps -- --exact`
Proof: `cargo test -p fala-retention --test retention encode::ten_seconds_fit_in_40_kb -- --exact`

**C4** - WAVs mono 48 kHz, estéreo 44,1 kHz e estéreo 48 kHz de 24 bits devolvem `UnsupportedWav` com o formato encontrado, e a pasta da sessão fica sem nenhum arquivo (AC 4)
Proof: `cargo test -p fala-retention --test retention encode::rejects_other_formats -- --exact`

### S2 - validar antes de apagar o WAV · ~3 files · ~20 KB · ~5k

**C5** - Para um WAV de 10 s mais 7 amostras (480 007 quadros), `RetainedAudio.samples` é 480 007 e `validate_opus` de cada arquivo com 480 007 passa (AC 5)
Proof: `cargo test -p fala-retention --test retention validate::sample_count_matches_wav -- --exact`

**C6** - Depois de `retain_wav` com sucesso, o WAV não existe e a pasta da sessão contém exatamente `{mic.opus, sys.opus}` (AC 6)
Proof: `cargo test -p fala-retention --test retention validate::wav_deleted_only_after_success -- --exact`

**C7** - Com a pasta da sessão apontando para dentro de um arquivo comum (impossível de criar), `retain_wav` devolve erro e o WAV continua com o mesmo tamanho em bytes (AC 7)
Proof: `cargo test -p fala-retention --test retention validate::failure_keeps_wav -- --exact`

**C8** - `validate_opus` devolve `ValidationFailed` para um `mic.opus` truncado na metade e para a contagem esperada + 1 sobre um arquivo íntegro (AC 8)
Proof: `cargo test -p fala-retention --test retention validate::truncated_or_wrong_count_fails -- --exact`

**C9** - Com `mic.opus.part` e `sys.opus.part` com lixo deixados por uma tentativa anterior, `retain_wav` termina com `{mic.opus, sys.opus}` válidos e sem `.part` (AC 9)
Proof: `cargo test -p fala-retention --test retention validate::rerun_overwrites_leftover_parts -- --exact`

### S3 - política de retenção · ~2 files · ~10 KB · ~3k

**C10** - `RetentionPolicy::default()` é `Keep`, e `audio_due` com `Keep` devolve vazio para sessões encerradas há 0, 1 e 10 000 dias, com e sem transcrição (AC 10)
Proof: `cargo test -p fala-retention --lib policy::tests::keep_never_deletes -- --exact`

**C11** - Com `DeleteAfterDays(30)`, sessões encerradas há exatamente 30 dias e há 31 dias vencem; há 30 dias menos 1 min não vence (AC 11)
Proof: `cargo test -p fala-retention --lib policy::tests::delete_after_days_boundary -- --exact`

**C12** - Com `DeleteAfterTranscript`, vencem exatamente as sessões com `transcript_confirmed = true`, independente da idade (AC 12)
Proof: `cargo test -p fala-retention --lib policy::tests::delete_after_transcript -- --exact`

**C13** - Uma sessão com `ended_at = None` e transcrição confirmada nunca vence com nenhuma das 3 políticas, mesmo 10 000 dias depois (AC 13)
Proof: `cargo test -p fala-retention --lib policy::tests::unfinished_session_never_due -- --exact`

**C14** - `delete_after_days(0)` e `(3651)` devolvem `DaysOutOfRange`; `(1)` e `(3650)` são aceitos (AC 14)
Proof: `cargo test -p fala-retention --lib policy::tests::days_bounds -- --exact`

**C15** - A política serializa como `"keep"`, `{"delete_after_days":30}`, `"delete_after_transcript"` e volta igual (AC 15)
Proof: `cargo test -p fala-retention --lib policy::tests::serialized_forms -- --exact`

### S4 - apagar o áudio de uma sessão · ~2 files · ~5 KB · ~2k

**C16** - Com `root/<A>/mic.opus`, `root/<B>/mic.opus` e `root/solto.txt`, `delete_session_audio(root, A)` devolve `true`, remove `root/<A>/` e mantém `root/<B>/mic.opus` e `root/solto.txt` (AC 16)
Proof: `cargo test -p fala-retention --test retention delete::removes_only_that_session -- --exact`

**C17** - `delete_session_audio(root, C)` sem `root/<C>/` devolve `Ok(false)` (AC 17)
Proof: `cargo test -p fala-retention --test retention delete::missing_dir_is_false -- --exact`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| arquivos retidos (2) | `mic.opus` C1 · `sys.opus` C1 | - |
| canais do WAV (2) | L -> mic C2 · R -> sys C2 | - |
| formatos recusados (3) | mono 48 kHz C4 · estéreo 44,1 kHz C4 · estéreo 24 bits C4 | - |
| falhas da validação (2) | truncado C8 · contagem diferente C8 | - |
| `RetentionPolicy` (3) | `Keep` C10 · `DeleteAfterDays` C11 · `DeleteAfterTranscript` C12 | - |
| borda de dias (3) | 30 d − 1 min C11 · 30 d C11 · 31 d C11 | - |
| limites de dias (4 bordas) | 0 C14 · 1 C14 · 3650 C14 · 3651 C14 | - |
| saída de `delete_session_audio` (2) | `true` C16 · `false` C17 | - |
| Landing doors (5) | crate novo C1 · dependências (libopus compila e roda) C1 · formato e layout C1, C2, C3 · forma da política C15 · apagar o WAV só depois de validar C6, C7 | - |

- Nenhum check alega mais que os casos que a prova exercita

## Swept

- validation: C4, C14
- failure modes: C7, C8 - o WAV fica quando algo falha
- idempotency: C9 - rodar de novo sobre `.part` velhos termina igual
- authorization: n/a - biblioteca local sem chamador remoto; `delete_session_audio` só aceita `SessionId` (26 caracteres Crockford), que não expressa `..` nem separador de caminho
- concurrency: n/a - o pipeline converte uma sessão por vez e o job de retenção só apaga sessões encerradas (C13); duas conversões da mesma sessão ao mesmo tempo não acontecem no desenho da fase 2
- data lifecycle: C6, C10, C11, C12, C13, C16
- dependency failure: C7 - erro de I/O vira `RetentionError`, sem pânico; erro do libopus idem (mesmo caminho de `?`)
- state transitions: C6 - `.part` -> renomeado -> WAV apagado, nessa ordem
- observability: n/a - o crate não loga; os erros carregam o caminho e a causa, e quem chama loga (sem conteúdo de áudio)

## Handoff

- S1-S4 ≈ 22k tokens (crate novo de ~4 arquivos, ~500 linhas com testes; leitura do código do `opus`/`ogg` já feita), todos em `fala-retention`, sob o orçamento de 150k - one builder
- Mechanism: one builder (cabe no orçamento; sem pergunta)
