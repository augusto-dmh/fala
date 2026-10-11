# loopback-device-select verification

Profile: light · Windows 11 (Alienware 16), 2026-10-11 · `CARGO_TARGET_DIR=C:\f\loopback-device-select`

| Check | Resultado | Evidência |
|---|---|---|
| C1 | ok | `cargo test -p fala-meeting --test route silent_channel_and_sound_elsewhere_for_10_s_warns`: 1 passed |
| C2 | ok | `... --test route both_silent_leaves_it_to_the_mute_watch`: 1 passed |
| C3 | ok | `cargo test -p fala-meeting --test route`: 10 passed; o crate inteiro: 10 + 25 + 1 + 2 doc |
| C4 | ok | `... --test route -- digital_zero ordinary_silence the_mic_only`: 3 passed |
| C5 | ok | `cargo test -p fala-audio --lib -- outputs system::tests::exact_output_name`: 3 passed |
| C6 | ok | `cargo test -p fala-audio --lib switching_the_system_rate`: 1 passed; `--lib` inteiro: 31 passed |
| C7 | ok | `cargo run -p fala-audio --example outputs -- 3`: 10 saídas listadas com a padrão marcada (`Voicemeeter Input`), pico 0.0000 em todas (nada tocando) |
| C8 | ok | `cargo test -p fala --lib meeting::output`: 2 passed; `--lib` inteiro: 428 passed, 1 ignored |
| C9 | ok | `bun src/components/meeting/output.test.tsx`: OUT1, OUT2 ok |
| C10 | ok | OUT3, OUT4 ok; `bun src/overlay/pill.test.tsx` e `bun src/components/meeting/meeting.test.tsx`: all assertions passed |
| C11 | ok | clippy `-D warnings` em `fala-audio`, `fala-meeting` e `fala` limpo; `cargo fmt --all --check`; `tsc`, `eslint`, `prettier --check`, `check:translations`, `check-no-tauri-in-crates.sh`, `check-brand.sh` ok |
| M1 | não feito | precisa de um fone USB e de tocar áudio; roteiro no PR |

## Sem prova

- O aviso e a troca a quente com um fone de verdade (M1), inclusive se o `sys.opus` tem o som
  depois da troca.
- Um pico de verdade no `OutputMeter`: nesta rodada nada tocava, então só os zeros foram lidos.
- Se um app de call em pausa de fala entrega zero digital (acenderia o ponto âmbar a cada 3 s de
  silêncio) ou ruído de conforto (não acende).
- O cartão na pill desenhado de verdade (tamanho da janela, posição em cima e embaixo).
- Linux: o CI compila; nada roda lá (lista só com a padrão, sem medidor).
