# meeting-detection checks

Profile: light
Plan: `.specs/features/meeting-detection/plan.md`

16 checks in 2 slices · 0 one-way doors · 1 open (manual Windows with real calls)

Windows 11 (Alienware), `CARGO_TARGET_DIR=C:\f\meeting-detection`. "manual" marca o que só se vê
com o app aberto e calls reais.

## Checks

### S1 - quem captura o mic e o detector

**C1** - `ancestors` segue os pais até um pid que saiu, um ciclo ou 4 níveis; `exe_name` tira o caminho e a caixa (AC 1)
Proof: `cargo test -p fala-audio --lib activity`

**C2** - `mic_holders` lista quem captura o mic nesta máquina, sem abrir stream (AC 1)
Proof: `cargo run -p fala-audio --example mic_holders` (manual: a saída lista o processo)

**C3** - Teams pelo exe e pelo WebView2 filho, WhatsApp pelo WebView2, Edge, Zoom; app qualquer e filho de app de call não são call (AC 2)
Proof: `cargo test -p fala-meeting --test detect identifies_apps_by_exe_and_by_webview_parent -- --exact`

**C4** - O Fala nunca é call, nem como pai de WebView2, nem com outro nome de exe (AC 2)
Proof: `cargo test -p fala-meeting --test detect fala_itself_is_never_a_call -- --exact`

**C5** - Uma call de 10 min dá um aviso, em t + 2 s; ids e exes da tabela são únicos e minúsculos (AC 3)
Proof: `cargo test -p fala-meeting --test detect one_offer_per_call_after_two_seconds -- --exact`
Proof: `cargo test -p fala-meeting --lib detect::tests`

**C6** - Teste de mic de 1 s não avisa; sumir 1 s não é outra call (AC 3)
Proof: `cargo test -p fala-meeting --test detect a_short_probe_or_a_flicker_is_not_a_new_call -- --exact`

**C7** - Menos de 2 min depois de um aviso, nada; outra call depois disso avisa (AC 4)
Proof: `cargo test -p fala-meeting --test detect a_new_call_after_the_gap_gets_a_new_offer -- --exact`

**C8** - App silenciado e chave desligada não avisam (AC 3)
Proof: `cargo test -p fala-meeting --test detect muted_apps_and_the_switch_stay_silent -- --exact`

**C9** - Gravando, nada; parar a gravação com a call no ar não avisa (AC 4)
Proof: `cargo test -p fala-meeting --test detect no_offer_while_recording_nor_after_it_for_the_same_call -- --exact`

**C10** - "Parar?" só com ≥ 3 min gravados, uma vez, e só quando todos os apps de call largaram (AC 5)
Proof: `cargo test -p fala-meeting --test detect the_app_leaving_the_mic_only_asks_after_three_minutes -- --exact`
Proof: `cargo test -p fala-meeting --test detect asks_only_when_every_call_app_left -- --exact`

**C11** - ADR-0005: uma call inteira sem clique deixa a sessão em `Idle`; o "parar?" não para; só o clique grava (AC 6)
Proof: `cargo test -p fala-meeting --test detect never_starts_recording_without_a_click -- --exact`

### S2 - o aviso no desktop

**C12** - Posição e tamanho da janela pela área de trabalho e pela escala (AC 7)
Proof: `cargo test -p fala --lib call_prompt::tests::placement_top_right -- --exact`

**C13** - Cada resposta leva à ação certa e uma resposta que não casa com o aviso não faz nada; [Não avisar] não duplica o id (AC 8, AC 9)
Proof: `cargo test -p fala --lib call_prompt::tests -- --exact` (os testes `answer_*`)

**C14** - `UserAction::StartRecording` continua só em `MeetingManager::start` (ADR-0005)
Proof: `grep -rn "UserAction::StartRecording" apps/desktop/src` só em `meeting/mod.rs`

**C15** - Contagem de 20 s, pausa no hover, fecha sem gravar; textos pelas chaves (AC 10, AC 11)
Proof: `bun src/callprompt/callprompt.test.tsx`; `bun run check:translations`; `bun run lint`

**C16** - manual Windows, PC livre, calls reais: Teams desktop, Meet no Edge e WhatsApp avisam em ≤ 3 s; WhatsApp excluído fica quieto; desligar a call depois de 3 min mostra "Parar a gravação?"
Proof: manual (open: precisa de calls reais com o PC livre)
