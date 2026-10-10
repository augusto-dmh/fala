# upstream-catchup PR 4 (captura do mic no formato nativo) checks

Profile: light
Plan: none - o diff cabe numa frase (AGENTS.md, "Fluxo por feature")

## Intent

A issue do upstream #2141 mostra um mic Realtek no Windows que entrega silêncio digital quando o
cliente WASAPI pede f32: com os efeitos do driver ligados ("Voice clarity"), um cliente em f32
recebe só zeros sem erro, e um cliente no mix format recebe o sinal. O Fala força f32 nos dois
caminhos de captura: `AudioRecorder::get_preferred_config` (desktop) põe F32 acima do formato do
dispositivo, e `fala_audio::Mic::open` recusa um dispositivo sem f32. Com este PR, os dois abrem o
formato padrão (nativo) do dispositivo e convertem para f32 no callback.

Decisões (Confirmed? y — delegado):

- No desktop, o formato nativo vale só no Windows (`#[cfg(windows)]`, permitido em `apps/desktop`
  pela ADR-0007): lá o padrão é o mix format do WASAPI. No Linux o padrão é a escolha do próprio
  `cpal` (estéreo primeiro no ALSA), e usá-lo trocaria a captura mono de hoje por estéreo (achado
  da rodada 1 do Verifier), então o Linux continua na busca pontuada. O caminho de conversão já
  existia (`build_stream::<T>` para u8, i8, i16, i32 e f32); se o formato padrão não é um desses,
  o Windows também cai na busca.
- O upstream tentou a mesma troca no desktop (cjpais/Handy#2144, "just use default input config"),
  o reporter disse que não resolveu e o PR foi fechado sem merge; com os efeitos ligados, nem o
  gravador do Windows funcionava para ele. Este PR não promete resolver o Realtek: tira o f32
  forçado e deixa o resultado para C6.
- `Mic` converte qualquer formato inteiro ou float que o `cpal` entrega (i8 a i64, u8 a u64, f32,
  f64) e recusa com `UnsupportedConfig` o resto.
- A captura do áudio do sistema (`meeting/system.rs`, loopback de reunião) continua exigindo f32:
  é outro caminho, sem relato de bug, e fica para quando a fase 2 ligar a reunião no desktop.
- Detectar buffer todo zero e avisar (#1899) não entra: é feature à parte.

6 checks in 1 slice · 0 one-way doors · 0 open, of which 0 block

Comandos de cargo com `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`, na
raiz do worktree.

## Checks

### S1 - formato nativo · 3 files · 70 KB · ~18k

**C1** - `downmix` converte para f32 cada formato nativo: f32 `[0.5, -0.25]` → 0.125; i16
`[MAX, MAX]` → 1.0, `[MIN]` → -1.0, `[16384, 0]` → 0.25; i32 `[MIN, MIN]` → -1.0, `[2^30]` → 0.5;
u8 `[128, 128]` → 0.0, `[255]` → 127/128; f64 `[0.5, 0.5]` → 0.5 (tolerância 1e-3)
Proof: `cargo test -p fala-audio mic::tests::downmix_converts_every_native_format_to_f32`

**C2** - Um sinal inteiro não vira silêncio: i16 `[8192, 8192]` dá magnitude > 0.2
Proof: `cargo test -p fala-audio mic::tests::downmix_keeps_a_signal_that_is_not_silence`

**C3** - `Mic::open` não procura mais f32: usa `default_input_config()` e escolhe `build::<T>`
pelo `sample_format()` padrão, sem `supported_input_configs` e sem a mensagem "não oferece f32"
Proof: `! grep -q 'supported_input_configs\|não oferece f32' crates/audio/src/mic.rs && grep -q 'SampleFormat::I16 => build::<i16>' crates/audio/src/mic.rs && grep -q 'SampleFormat::F32 => build::<f32>' crates/audio/src/mic.rs`

**C4** - No desktop, `builds_sample_format` é `true` para U8, I8, I16, I32 e F32 e `false` para
U16, U32, I64 e F64; e, só com `#[cfg(windows)]`, `get_preferred_config` devolve `default_config`
quando `builds_sample_format(default_config.sample_format())`, antes da busca pontuada
Proof: `cargo test -p fala --lib -- audio_toolkit::audio::recorder::tests::device_default_format_is_kept_when_the_builder_handles_it`
Proof: `awk '/fn get_preferred_config/{f=1} f&&/builds_sample_format\(default_config.sample_format\(\)\)/{a=NR} f&&/for config_range in supported_configs/{b=NR; exit} END{exit !(a && a<b)}' apps/desktop/src/audio_toolkit/audio/recorder.rs && grep -B1 'if builds_sample_format(default_config.sample_format())' apps/desktop/src/audio_toolkit/audio/recorder.rs | grep -q '#\[cfg(windows)\]'`

**C5** - Os testes que já existiam no `fala-audio` e do gravador do desktop continuam verdes sem
asserção editada
Proof: `cargo test -p fala-audio`
Proof: `cargo test -p fala --lib -- audio_toolkit::audio::recorder::tests`

**C6** - TODO(windows): num notebook com mic Realtek e "Voice clarity" ligado, o log mostra
`Format:` igual ao mix format do dispositivo (não `F32` forçado) e um ditado de 5 s sai com texto;
no `fala-cli dictate` o log `microfone:` mostra o mesmo formato. Não dá para reproduzir no Linux
Proof: `TODO(windows)` manual - 5 ditados com Voice clarity ligado e 5 desligado; esperado 10/10 com texto

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| plataformas do gravador do desktop (2) | Windows usa o padrão C4 · Linux mantém a busca C4 | - |
| caminhos de captura do mic (2) | `crates/audio/src/mic.rs` C1, C2, C3 · `audio_toolkit/audio/recorder.rs` C4 | - |
| formatos que o desktop abre como vêm (5) | U8 C4 · I8 C4 · I16 C4 · I32 C4 · F32 C4, table-driven | - |
| formatos que caem na busca pontuada (4) | U16 C4 · U32 C4 · I64 C4 · F64 C4 | - |
| famílias de amostra convertidas no `Mic` (4) | f32 C1 · inteiro com sinal C1 · inteiro sem sinal C1 · f64 C1 | - |

- C3 e a segunda prova de C4 são estruturais (abrir um dispositivo real não roda no CI); C6 é a
  prova do sintoma e fica para o Windows.

## Swept

- validation: C3 (formato fora da lista vira `UnsupportedConfig`)
- failure modes: C2 (silêncio digital)
- idempotency: n/a - nenhuma operação repetível nova
- authorization: n/a - nada de acesso novo
- concurrency: n/a - o callback continua empurrando no mesmo ring lock-free
- data lifecycle: n/a - nada persistido
- dependency failure: C6 (driver com efeitos)
- state transitions: n/a - sem estado novo
- observability: C3 (o log `microfone:` passa a dizer o formato)

## Handoff

- S1 = ~18k (3 arquivos, 70 KB / 4), sob o orçamento de 150k - one builder
