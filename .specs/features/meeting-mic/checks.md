# meeting-mic checks

Profile: light
Plan: `.specs/features/meeting-mic/plan.md`

Prefixo de todo `cargo`: `CARGO_TARGET_DIR='C:\f\meeting-mic'`.

**C1** - a reunião pede o mic à mesma resolução do ditado (seleção, clamshell, canal), na
abertura, na reabertura e depois do sono; sem `default_input_device` no gravador
Proof: `grep -n default_input_device apps/desktop/src/meeting/recorder.rs` vazio; leitura do diff

**C2** - trocar o mic durante a gravação manda `SwitchMic` ao gravador; a troca de taxa mantém o
relógio (o `set_mic_rate` do `fala-audio`)
Proof: `cargo test -p fala-audio --lib set_mic_rate` (existente) e leitura do diff

**C3** - a call medida (sistema entre −69 e −44 dBFS, mic em zero) por 100 s não é engano; só o
som de início (1 s alto) com os dois canais mudos é; um mic que funciona numa sala quieta (−55
dBFS) é transcrito; o card que expira vira Manter
Proof: `cargo test -p fala --lib meeting::lifecycle`

**C4** - o painel mostra "Microfone: <nome>" vindo do status
Proof: `bun src/components/meeting/meeting.test.tsx`

**C5** - app de debug portátil grava 10 s de reunião e o log diz `mic <o escolhido>`
Proof: log do app portátil
