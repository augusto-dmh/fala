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
