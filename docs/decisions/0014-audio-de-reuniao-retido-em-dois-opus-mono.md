---
status: accepted
date: 2026-10-02
---
# Áudio de reunião retido em dois arquivos Ogg Opus mono de 24 kbps, com libopus compilado do fonte

## Contexto e problema
A ADR-0005 manda gravar mic e sistema como dois canais e reter o áudio ("WAV durante, Opus depois", ~13 MB/h por canal). O design doc §3.5 congelado diz `audio/<sessão>.opus` e o §3.2 diz "gravador WAV→Opus estéreo L=mic R=sistema". Abaixo de ~64 kbps o encoder Opus estéreo pode usar intensity stereo e misturar L e R, o que destrói o "eu vs eles" retido; e cada canal vai sozinho ao ASR (diarização só no sistema). O WAV de trabalho tem ~690 MB/h e não pode ser o formato retido. Pela regra do design doc, a divergência vira ADR.

Encoder: o pitch da fase 2 (D2) compara libopus por bindings compilado do fonte, `ffmpeg` em subprocesso e encoder em Rust puro. O `ffmpeg` faria a retenção, que é obrigatória, depender de um binário que só a importação, que é opcional, precisa; o Rust puro ainda é imaturo.

## Opções consideradas
* (a) Dois arquivos Ogg Opus mono por sessão, `audio/<id>/mic.opus` e `audio/<id>/sys.opus`, 48 kHz, 24 kbps cada.
* (b) Um Ogg Opus multistream com os dois canais desacoplados, `audio/<id>.opus`.
* (c) Um Opus estéreo comum a 48 kbps, `audio/<id>.opus`.

## Decisão
(a), codificado pelo libopus (crates `opus` 0.3 e `audiopus_sys` 0.2 com a feature `static`, que compila o libopus do fonte com CMake quando o sistema não o tem), em pacotes de 20 ms, aplicação VoIP, VBR com alvo de 24 kbps, num crate próprio (`fala-retention`) e não em `crates/audio`. `<id>` é o ULID da sessão (`fala-meeting::SessionId`). É o mais simples de codificar, validar e enviar, e preserva a separação dos canais.

### Consequências
* Bom: "eu vs eles" preservado por construção; cada arquivo vai direto ao ASR; ~11 MB/h por canal; o Ogg Opus toca em navegadores e players comuns.
* Ruim: dois arquivos por sessão em vez de um; uma dependência C (libopus) no build, com CMake no Windows (MSVC) e no CI.
* Ruim, conhecido em 2026-10-04: o `audiopus_sys` 0.2 (binding do libopus usado pelo crate `opus`) está sem manutenção (RUSTSEC-2026-0150, aceito no `deny.toml` com justificativa) e não compila com CMake 4 sem ajuda: o CI Windows reprovou com "Compatibility with CMake < 3.5 has been removed". O `.cargo/config.toml` define `CMAKE_POLICY_VERSION_MINIMUM=3.5`, que o CMake 4 lê do ambiente. Se isso deixar de funcionar, vale a saída por FLAC desta ADR.
* Obrigatório: o WAV de trabalho só é apagado depois de os dois Opus serem decodificados de volta e a contagem de amostras conferir com a do WAV; os arquivos são escritos como `.part` e renomeados só depois da validação. Se o libopus não compilar no CI Windows (trilha G) em 2 dias, a saída é FLAC mono por canal (Rust puro, ~50 % do WAV) até o milestone de qualidade da fase 3, por ADR nova.

## Confirmação
Teste do crate `fala-retention`: um WAV estéreo com tom no canal esquerdo e silêncio no direito vira `mic.opus` com o tom e `sys.opus` em silêncio, ambos mono a 48 kHz, e a decodificação devolve exatamente o número de amostras do WAV; o WAV continua no disco quando a validação falha.
