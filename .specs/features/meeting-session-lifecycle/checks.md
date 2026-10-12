# meeting-session-lifecycle checks

Profile: light
Plan: `.specs/features/meeting-session-lifecycle/plan.md`

Windows 11 (Alienware), `CARGO_TARGET_DIR=C:\f\meeting-session-lifecycle`. "manual" marca o que
só se vê com o app aberto.

## S1 — apagar, retenção, teto (PR 1)

**C1** - Apagar uma sessão remove a linha e, em cascata, trechos, participantes e falantes; as de
outra sessão ficam; apagar de novo é `NotFound`
Proof: `cargo test -p fala-storage --test meetings delete_meeting_cascades_and_is_final`

**C2** - Limpar o áudio retido volta `audio_retained` a falso; id desconhecido é `NotFound`
Proof: `cargo test -p fala-storage --test meetings retained_audio_can_be_cleared`

**C3** - Retenção: manter nunca apaga; "N dias" apaga só áudio já retido, vencido e fora da
transcrição em curso; a linha fica; exatamente N dias vence, um minuto antes não; valor fora de
1-3650 mantém
Proof: `cargo test -p fala --lib meeting::lifecycle::tests::retention_deletes_only_retained_audio_past_its_days`

**C4** - Apagar remove `audio/<id>/` e a linha, deixa as outras pastas, apaga rascunho sem pasta
Proof: `cargo test -p fala --lib meeting::lifecycle::tests::delete_removes_audio_and_row`

**C5** - A confirmação de apagar diz o que sai e que não volta e é o único caminho da página para
`deleteMeeting`; retenção padrão "Manter" com presets; encurtar pede confirmação
Proof: `bun src/components/meeting/lifecycle.test.tsx`

**C6** - O teto vem de `meeting_cap_hours` (1-8, padrão 3); fora disso, o comando recusa e o
gravador usa 3 h
Proof: `cargo test -p fala --lib settings` e leitura de `MeetingManager::start`

**C7** - manual: no app de debug portátil, apagar uma sessão e conferir que `audio/<id>/` sumiu;
Configurações → Geral → Reuniões mostra "Áudio das reuniões" e "Limite de gravação"
Proof: manual (Windows, app de debug portátil)

## S2 — recuperar a órfã, sair gravando (PR 2)

**C8** - Recuperar fecha só sessão começada e sem fim, com `stop_reason` nulo
Proof: `cargo test -p fala-storage --test meetings recover_meeting_ends_only_an_unfinished_session`

**C9** - Órfã = começada, sem fim e com `recording.wav`; um WAV morto (cabeçalho do último
flush, meio segundo depois dele) recupera o que o cabeçalho anuncia e vira Opus com essas
amostras: perde ≤ 1 s; recuperar duas vezes recusa
Proof: `cargo test -p fala --lib meeting::recovery`

**C10** - Sair gravando ou pausado pergunta; [Parar e sair] para e sai; [Cancelar] e o fim da
contagem não saem; o banner diz "Recuperar gravação de 14:02 (12 min)?"
Proof: `cargo test -p fala --lib quit` (`quitting_while_recording_asks`,
`answer_maps_each_quit_button`);
`bun src/components/meeting/lifecycle.test.tsx`; `bun src/callprompt/callprompt.test.tsx`

**C11** - manual: gravar ~1 min, `taskkill /F /IM fala.exe`, reabrir, Recuperar; a duração perde
≤ ~1 s e `mic.opus`/`sys.opus` aparecem; sair pela bandeja gravando mostra o cartão
Proof: manual (Windows, app de debug portátil)
