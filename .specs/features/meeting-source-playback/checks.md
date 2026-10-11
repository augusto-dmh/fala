# meeting-source-playback checks

Profile: light
Plan: `.specs/features/meeting-source-playback/plan.md`

16 checks in 3 slices · 5 doors · 0 open

Todas as provas Rust rodam com `CARGO_TARGET_DIR=C:\f\meeting-source-playback`. Os Opus de teste
são gerados pelos próprios testes; nada de microfone, rede ou chave.

## Checks

### S1 - `decode_range` em `fala-retention` (PR A)

**C1** - De um WAV de 10 s com bipes de 1 kHz no mic em 2,0 s e 6,5 s e no sistema em 4,0 s,
`decode_range` de 1,5–3,0 s e de 6,0–7,0 s do `mic.opus` e de 3,5–4,5 s do `sys.opus` devolve a
duração pedida ± 960 amostras e o bipe a 0,5 s do início ± 960 amostras (20 ms) (AC 1)
Proof: `cargo test -p fala-retention --test retention range::beep_lands_where_it_was -- --exact`

**C2** - 3,5–5,0 s do `mic.opus` (entre os bipes) volta com pico ≤ 2 LSB (−84 dBFS) (AC 1)
Proof: `cargo test -p fala-retention --test retention range::silence_comes_back_zero -- --exact`

**C3** - 9,5–20 s de um arquivo de 10 s devolve exatamente 0,5 s; 11–12 s devolve vazio; um
arquivo que não é Ogg devolve erro (AC 1)
Proof: `cargo test -p fala-retention --test retention range::range_is_clipped_to_the_file -- --exact`

**C4** - Num Ogg Opus de 2 h (pages de 50 pacotes), 1 s a partir de 1:59:59 devolve 48 000
amostras decodificando entre 50 e 150 pacotes (de 360 050); o fim é recortado no granule final e
um início depois do fim devolve vazio (AC 2)
Proof: `cargo test -p fala-retention --lib decode::tests::two_hours_in_decodes_only_a_few_seconds -- --exact`

**C5** - Intervalo vazio devolve vazio sem abrir o arquivo; arquivo inexistente devolve `Io` (AC 1)
Proof: `cargo test -p fala-retention --lib decode::tests::empty_range_reads_nothing -- --exact`

### S2 - `meeting_clip` e `audio_available` no desktop (PR A)

**C6** - `window` dá 0,5 s de margem de cada lado, não passa de 0 no início, trata `t1 < t0` e
corta em 60 s (AC 3)
Proof: `cargo test -p fala --lib meeting::clip::tests::window_adds_margins_and_caps_at_a_minute -- --exact`

**C7** - O clipe de 1,0–2,0 s de uma sessão retida de 4 s é um WAV `RIFF`/`WAVEfmt ` mono,
48 kHz, 16 bits, de 2,0 s, com pico acima do de cada canal sozinho (mic + sys somados); 3–30 s
devolve 1,5 s (AC 3)
Proof: `cargo test -p fala --lib meeting::clip::tests::clip_is_a_mono_48k_wav_of_the_window -- --exact`

**C8** - Sem os dois `.opus` (nenhum, ou só o `mic.opus`), `audio_available` é `false` e o
clipe devolve `MeetingError::NoAudio` (AC 3, AC 4)
Proof: `cargo test -p fala --lib meeting::clip::tests::session_without_audio_is_no_audio -- --exact`

**C9** - A soma satura em `i16::MAX`/`i16::MIN` e completa o canal mais curto com zero (AC 3)
Proof: `cargo test -p fala --lib meeting::clip::tests::mix_saturates -- --exact`

**C10** - `MeetingDetail` em `src/bindings.ts` tem `audio_available: boolean`, regenerado pelo
binário de debug; `meeting_clip` é despachado pelo nome antes do handler do tauri-specta (AC 4,
door 1)
Proof: `grep -n "audio_available: boolean" src/bindings.ts`
Proof: `bun src/components/meeting/meeting.test.tsx` (SP-player: `lib.rs` despacha `meeting_clip`)

### S3 - A UI (PR B)

**C11** - `parseNotes` do Markdown no formato do `render.rs` separa 2 linhas humanas (âncora
`^aN` fora, texto guardado por N) de 3 geradas, extrai `s1`, `s2`, `a2`, descarta `s999` e
devolve os títulos com o nível (AC 5)
Proof: `bun src/components/meeting/meeting.test.tsx` (SP-parse)

**C12** - A nota renderizada não contém `[[#^`, `fala:ia`, `^a1` nem `^a2`; cada linha gerada
tem `text-text-2` e cada humana `text-text`; há uma lupa por linha gerada (3), visível por
`group-hover` e `focus-visible`; o `MeetingView` não usa mais `<pre>` (AC 6)
Proof: `bun src/components/meeting/meeting.test.tsx` (SP-render, SP-transcript)

**C13** - A lupa de uma linha com `s2` e `a2` mostra "Eu", `01:10` como botão "Ir para 01:10 na
transcrição", o texto do segmento, ▶ e o texto digitado de `a2`; a linha sem fonte mostra
"Esta linha não tem trecho de origem."; sem áudio o ▶ fica `disabled` (AC 7)
Proof: `bun src/components/meeting/meeting.test.tsx` (SP-popover)

**C14** - A transcrição tem "Pessoa 1" à esquerda (`items-start`, `bg-them/15`) e "Eu" à direita
(`items-end`, `bg-me/15`); dois segmentos seguidos do mesmo falante mostram o rótulo uma vez;
cada balão tem âncora `segment-<seq>` e ▶; o destacado tem `ring-accent`; sem áudio os 3 ▶ ficam
`disabled` com o título "O áudio desta reunião não está mais no disco." e o aviso aparece acima da
transcrição (AC 7, AC 8)
Proof: `bun src/components/meeting/meeting.test.tsx` (SP-transcript)

**C15** - O `ClipPlayer` pede `meeting_clip` com `{ id, t0Ms, t1Ms }`, cria um `Blob`
`audio/wav`, toca só o pedido mais novo (o mais antigo que chega depois é descartado), para e
revoga a URL anterior antes do próximo, e uma falha devolve o erro e volta a "parado"; o
`assetProtocol` continua só com `$APPDATA/recordings/**` (AC 9)
Proof: `bun src/components/meeting/meeting.test.tsx` (SP-player)

**C16** - Gates: `bun run lint`, `bun run format:check`, `bun run check:translations`,
`bun run build`, `cargo clippy --workspace --all-targets -- -D warnings`
Proof: os comandos, com saída limpa

## Manual (fora da tabela; não muda o veredito)

Windows, com áudio ao vivo e as chaves ElevenLabs + Anthropic (o Augusto): gravar 2–3 min falando
números em instantes anotados, transcrever, gerar notas, clicar ▶ em 10 segmentos e conferir o
trecho certo ± 1 s em 10 de 10; confirmar que o WebView2 toca o WAV por `Blob` URL.
