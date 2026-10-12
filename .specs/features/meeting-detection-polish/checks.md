# meeting-detection-polish checks

Profile: light
Plan: `.specs/features/meeting-detection-polish/plan.md`

8 checks in 1 slice · 0 one-way doors · 1 open (manual Windows with real calls)

Windows 11 (Alienware), `CARGO_TARGET_DIR=C:\f\meeting-detection-polish`.

## Checks

**C1** - Títulos → reunião: Meet por prefixo e "Google Meet", Teams, Zoom, Jitsi, huddle do Slack; claude.ai, Notion, "meet" minúsculo, teste de mic → nada; palavra precisa de fronteira (AC 1)
Proof: `cargo test -p fala-meeting --test detect browser_titles_name_the_web_call -- --exact`
Proof: `cargo test -p fala-meeting --lib detect::tests`

**C2** - Navegador com o mic sem reunião não avisa; a reunião aparece e avisa na hora, com o nome; Meet direto espera 2 s (AC 2)
Proof: `cargo test -p fala-meeting --test detect a_browser_offers_only_once_a_window_shows_a_meeting -- --exact`

**C3** - Gravando, trocar de aba não é sair do Meet; navegador sem reunião não pergunta nem segura a pergunta de outro app (AC 3)
Proof: `cargo test -p fala-meeting --test detect switching_tabs_during_a_recorded_meet_is_not_leaving_it -- --exact`
Proof: `cargo test -p fala-meeting --test detect asks_only_when_every_call_app_left -- --exact`

**C4** - "Parar?" ≤ 2 por gravação, 3 min entre eles, conta zera numa gravação nova (relógio falso) (AC 4)
Proof: `cargo test -p fala-meeting --test detect ask_stop_waits_three_minutes_and_stops_after_two -- --exact`

**C5** - Os checks de `meeting-detection` seguem verdes (apps, 2 s, 2 min, gravação, ADR-0005)
Proof: `cargo test -p fala-meeting --test detect`

**C6** - O payload leva `web_name`; [Não avisar] silencia o navegador (AC 5)
Proof: `cargo test -p fala --lib call_prompt`

**C7** - Título "Parece uma chamada do Google Meet no Edge"; chaves `titleWeb` em pt e en com `{{app}}` e `{{web}}` (AC 5)
Proof: `bun src/callprompt/callprompt.test.tsx`

**C8** - manual Windows, calls reais: modo de voz do claude.ai no Edge não avisa; Meet no Edge avisa "Google Meet no Edge"; gravando, sair e voltar da call três vezes dá no máximo duas perguntas (AC 2, 3, 4)
Proof: manual (Augusto)
