# loopback-device-select checks

Profile: light
Plan: `.specs/features/loopback-device-select/plan.md`

11 checks in 3 slices · 4 doors · 0 open

Prefixo de todo `cargo` abaixo (PowerShell, Windows):
`$env:CARGO_TARGET_DIR='C:\f\loopback-device-select'`.

### S1 - a regra, neutra (`crates/meeting/tests/route.rs`)

**C1** - canal do sistema em zero e pico noutra saída por 10 s → aviso com o nome dela; 9,75 s não
Proof: `cargo test -p fala-meeting --test route silent_channel_and_sound_elsewhere_for_10_s_warns`

**C2** - os dois mudos → nenhum aviso de saída, e o `MuteWatch` avisa aos 2 min como antes
Proof: `cargo test -p fala-meeting --test route both_silent_leaves_it_to_the_mute_watch`

**C3** - sinal no canal gravado zera; a saída gravada nunca conta; pausa de fala não zera, 10 s
quietos ou a saída sumida zeram; a troca recomeça; o tempo pausado não conta (door 3)
Proof: `cargo test -p fala-meeting --test route`

**C4** - zero digital num canal por 3 s acende; silêncio comum de mic não; o mic só conta nos modos
com mic
Proof: `cargo test -p fala-meeting --test route -- digital_zero ordinary_silence the_mic_only`

### S2 - saídas, pico e troca de taxa (`crates/audio`)

**C5** - a lista marca a padrão e não repete nomes; o nome exato ganha do "contém"; picos guardam o
máximo por nome até a leitura
Proof: `cargo test -p fala-audio --lib -- outputs system::tests::exact_output_name`

**C6** - trocar a taxa do sistema no meio mantém o relógio, escreve zeros no intervalo e a fonte
nova reamostrada depois
Proof: `cargo test -p fala-audio --lib switching_the_system_rate`

**C7** - o exemplo lista as saídas e o pico de cada uma (Windows, só leitura)
Proof: `cargo run -p fala-audio --example outputs -- 3`

### S3 - desktop e UI

**C8** - a saída lembrada vale enquanto existe; ausente → padrão (door 2); o status segue a rota
e a troca apaga o aviso
Proof: `cargo test -p fala --lib meeting::output`

**C9** - painel: seletor "Som da reunião" com a padrão nomeada, a escolha lembrada (mesmo ausente);
ao vivo, "Som de" e o aviso com o botão; a página grava a escolha e chama a troca (door 1)
Proof: `bun src/components/meeting/output.test.tsx` (OUT1, OUT2)

**C10** - pill: o cartão com o botão fica sobre o indicador; zero digital deixa o ponto âmbar; o
aviso de 2 min continua
Proof: `bun src/components/meeting/output.test.tsx` (OUT3, OUT4) e `bun src/overlay/pill.test.tsx`

**C11** - clippy, fmt, testes do frontend, tsc, lint, traduções, scripts do CI
Proof: `cargo clippy -p fala-audio -p fala-meeting -p fala --all-targets -- -D warnings`,
`cargo fmt --all --check`, `bunx tsc --noEmit`, `bun run lint`, `bunx prettier --check .`,
`bun run check:translations`, `scripts/check-no-tauri-in-crates.sh`, `scripts/check-brand.sh`

### Manual (Windows, com fone USB fora do padrão)

**M1** - tocar um vídeo no fone USB com a saída padrão nos alto-falantes e gravar: aviso em ~10 s
no painel e na pill; [Gravar deste dispositivo] troca; o `sys.opus` tem o som depois da troca.
