---
status: accepted
date: 2026-09-26
---
# Reunião: dois canais gravados e retidos localmente; transcrição em batch pela ElevenLabs Scribe v2, com fallback local

## Contexto e problema
Em reuniões, nomes e números importam e a nuvem erra cerca de metade dos modelos locais rodáveis neste hardware (Scribe v2: 2,8 % WER em pt; Parakeet: 6,2 %). Diarização local em CPU é lenta (pyannote: 30-60 min por hora). O Granola não guarda áudio, e a reclamação nº 1 dos usuários é captura perdida sem como recuperar. O usuário decidiu tratar o consentimento como responsabilidade pessoal (grilling Q7), mantendo os controles no app.

## Opções consideradas
* ElevenLabs Scribe v2 batch (diarização até 32 falantes inclusa, US$ 0,22/h).
* Soniox (US$ 0,10/h, diarização em tempo real, sem WER pt público).
* Groq whisper-large-v3 + Sortformer local.
* Só local (Parakeet) até liberação formal.

## Decisão
Gravar mic e loopback como dois canais sincronizados (WAV durante, Opus depois), reter o áudio por padrão com retenção configurável, e transcrever em batch ao fim pela Scribe v2: canal do mic é "eu"; diarização do provedor só no canal do sistema. Parakeet local é o fallback quando offline ou quando o usuário marcar a sessão como "só local".

### Consequências
* Bom: melhor WER pt disponível; "eu vs eles" grátis e exato; nada se perde se a transcrição falhar; custo ~US$ 5/mês.
* Ruim: áudio de terceiros sai da máquina por escolha do usuário; transcrição só ao fim (sem ao vivo na v1); ~13 MB/h por canal em disco.
* Obrigatório: gravação só por clique explícito, indicador visível durante, aviso na UI de que transcrições contêm dados pessoais de terceiros. Uso no trabalho segue a política interna de classificação da informação e a LGPD; alinhar com o DPO antes de usar com clientes é a recomendação registrada, não adotada.

## Confirmação
Teste: nenhuma sessão de gravação inicia sem `UserAction::StartRecording`; o indicador de gravação é um estado obrigatório da máquina de estados da sessão.
