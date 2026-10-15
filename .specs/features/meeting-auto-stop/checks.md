# meeting-auto-stop checks

Profile: light
Plan: `.specs/features/meeting-auto-stop/plan.md`

7 checks in 1 slice · 0 one-way doors · 1 open (manual Windows with "Meet now")

Windows 11 (Alienware), `CARGO_TARGET_DIR=C:\f\meeting-auto-stop`.

## Checks

**C1** - `CallQuiet`: sem card não para; armado e no piso do Opus calado (−100 dBFS) por 20 s para; janelas reais de uma call baixa (com 3,5 s calados) desarmam uma vez; −65 dBFS conta como som; pausa desarma (AC 1, 2)
Proof: `cargo test -p fala-meeting --test detect an_expired_ask_stops_only_if_the_system_stayed_quiet -- --exact`

**C2** - `Input::CallEnded` para como `Silence` em `Recording`; pausada ou sem gravação, nada (AC 2, 3)
Proof: `cargo test -p fala-meeting --test detect a_call_that_ended_stops_as_silence_and_only_while_recording -- --exact`

**C3** - O limite do "parar?" segue (2 por gravação, 3 min entre eles) e o detector só pergunta (AC 5, 6)
Proof: `cargo test -p fala-meeting --test detect`

**C4** - `answer_action(AskStop, Expired)`: com `auto_stop` e sessão → `StopCallEnded(id)`; sem `auto_stop` ou sem sessão → `Close`; os botões seguem (AC 1)
Proof: `cargo test -p fala --lib call_prompt::tests`

**C5** - O card `CallEnded` tem [Retomar] para a sessão certa e fecha ao expirar; a transcrição só espera a oferta da própria sessão (AC 4)
Proof: `cargo test -p fala --lib call_prompt::tests::answer_resumes_the_session_the_call_stopped -- --exact`
Proof: `cargo test -p fala --lib meeting::call_end`

**C6** - O card: "Paro a gravação em {{seconds}} s" com a contagem, [Continuar gravando] primeiro, o card "Parei a gravação" com [Retomar], textos pt/en (AC 1, 4)
Proof: `bun src/callprompt/callprompt.test.tsx`
Proof: `bun run check:translations`

**C7** - Manual, Windows: "Meet now" no Teams sem ninguém, 3 min com vídeo, fechar o vídeo e sair: o card conta e para em 20 s; [Retomar] grava na mesma sessão. De novo com o vídeo tocando depois de sair: o card só fecha
Proof: manual (Augusto), roteiro no PR
