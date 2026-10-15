# live-transcript checks

Profile: light · Cargo com `CARGO_TARGET_DIR=C:\f\live-transcript`.

**C1** - O tap recebe exatamente os frames do WAV, seguidos, com o mic zerado no ditado — PR 1
Proof: `cargo test -p fala-audio meeting::recorder::tests::the_tap`

**C2** - `LiveSegmenter`: tempos certos, corte no teto, silêncio digital sem VAD, `speaking` — PR 1
Proof: `cargo test -p fala-audio meeting::live`

**C3** - `Preview`: linhas por canal com o offset da tomada; ditado segura a fila e ela segue na
ordem; modelo ocupado guarda, erro descarta; silêncio não dá linha; teto da fila — PR 2
Proof: `cargo test -p fala --lib meeting::preview`

**C4** - Painel: linhas na ordem do tempo sem repetir, vazio quando a transcrição de registro
substitui; segue o fim só com a pessoa nele; Você à direita, Eles à esquerda, "ouvindo…"; estados
gravando, parado e rotulado; Sala no presencial; a última fala na barra — PR 3
Proof: `bun src/components/meeting/live.test.tsx`

**C5** - Portões: fmt, clippy do workspace, testes dos crates tocados, lint, format, tsc,
traduções, `scripts/check-brand.sh` — todo PR

**Manual (Windows).** M1 debug portátil, reunião "só sistema" tocando fala pelo alto-falante:
linhas "Eles" durante a gravação · M2 ditar durante a reunião: o ditado sai no tempo de sempre e
o texto ditado não aparece na prévia · M3 a prévia some quando a transcrição de registro chega.
