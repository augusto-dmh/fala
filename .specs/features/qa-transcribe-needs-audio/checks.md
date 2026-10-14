# qa-transcribe-needs-audio — checks

R5 e o resto do Q15 da segunda passada de QA (`research/27-reqa-fala.md`): uma reunião sem áudio
(restaurada da lixeira, ou com o áudio apagado pela retenção) oferecia "Transcrever" com a dica da
chave; o cabeçalho da importação do Granola mostrava "00:00".

| ID | Check | Proof |
| --- | --- | --- |
| A1 | sem áudio no disco (nem WAV à espera da conversão), "Transcrever" desliga; sem transcrição, diz por quê (os textos de "sem áudio" que já existem), com ou sem a chave; com transcrição, fica quieto e a razão das notas aparece; com áudio, só a chave conta | `bun src/components/meeting/transcribeAudio.test.tsx` A1, `meetingKeys.test.tsx` K3 |
| A2 | o cabeçalho de uma importação do Granola não mostra "00:00" | `transcribeAudio.test.tsx` A2 |
| A3 | portões | `bun run lint`, `bunx prettier --check .`, `bunx tsc --noEmit`, `bun run check:translations`, os testes de Reuniões |
