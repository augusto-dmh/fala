# meeting-session-lifecycle — apagar, retenção, teto, recuperar a órfã, avisos, "só local"

Profile: light · fase 2 (F3 e o resto do F1) · ADR-0005, 0014, 0016 · nenhuma contrariada

## Problem

Uma reunião não se apaga; um `taskkill` no meio da gravação deixa o `recording.wav` sem que o app
o ofereça; a retenção (`fala-retention::audio_due`) e o teto (`RecordingCap`, 1-8 h) existem nos
crates mas não no app. Bug bar da fase 2: "retenção configurável + áudio nunca perdido em crash".

## Slices (PRs empilhados `feat/meeting-session-lifecycle[-N]`, pelo gate de 1000 linhas)

- **S1**: apagar reunião (confirmação literal, irreversível, cascata); retenção (manter | apagar
  o áudio após N dias; na abertura, a cada dia e ao mudar, só áudio já retido); teto 1-8 h.
- **S2**: "Recuperar gravação de HH:MM (N min)?" para sessão sem fim com `recording.wav`; WAV de
  sessão encerrada retido na abertura; sair pela bandeja gravando pergunta [Parar e sair].
- **S3**: "Ainda está gravando?" antes do fim por silêncio, "Parou por silêncio" e "Gravou por
  engano?", na janela do aviso de call.
- **S4**: "só local" por sessão (Parakeet já carregado, notas sem LLM) e fallback local.

## Decisions

- Retenção só apaga áudio com `audio_retained`: um WAV que nunca virou Opus não some (ADR-0014).
- Retenção em presets (manter, 7, 30, 90, 180, 365); encurtar pede confirmação, porque apaga já.
- Suspensão do SO fica para `meeting-power-guard`.
