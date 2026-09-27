---
status: accepted
date: 2026-09-26
---
# ASR do ditado roda localmente: Silero VAD + Parakeet-TDT-0.6B-v3 via sherpa-onnx

## Contexto e problema
O ditado captura tudo o que a pessoa fala em qualquer app, inclusive conteúdo sensível. O hardware é CPU-first (Intel iGPU no Linux; Windows a confirmar). pt-BR é critério eliminatório. Em CPU, Whisper large-v3-turbo fica em 1,5-3x tempo real (2-5 s por chunk); Parakeet v3 int8 roda 10-20x tempo real com WER pt de ~6 % (Open ASR Leaderboard); Nemotron 3.5 streaming tem pt-BR "transcription-ready" mas runtime CPU menos maduro.

## Opções consideradas
* Parakeet-TDT-0.6B-v3 int8 via sherpa-onnx, modo utterance (VAD fecha a frase, transcreve inteira).
* Nemotron 3.5 ASR Streaming 0.6B via NeMo-Speech.cpp, parciais ao vivo.
* Whisper turbo local (CPU ou Vulkan na iGPU).
* ASR em nuvem (Soniox, AssemblyAI, Scribe v2 Realtime).

## Decisão
Parakeet v3 via sherpa-onnx, utterance-level, com Silero VAD v6. É o caminho mais portátil (Windows e Linux, ONNX int8), já integrado no Handy via `transcribe-rs`, e entrega 0,5-1 s após soltar a tecla. Nemotron fica como evolução quando quisermos parciais na tela. O áudio de ditado nunca sai da máquina.

### Consequências
* Bom: offline; latência local; sem custo; pontuação e capitalização nativas.
* Ruim: WER pt ~1 ponto pior que Whisper large-v3 e ~2-3 pontos pior que a nuvem; depende do LLM (ADR-0004) e do dicionário para nomes próprios.
* Obrigatório: benchmark da fase 0 com a voz do usuário confirma ou substitui esta ADR; trait `Transcriber` mantém a troca de backend barata.

## Confirmação
`fala-cli bench` reporta WER e RTF sobre o corpus de referência em `fala-research/benchmarks/`; o tipo de áudio de ditado não implementa serialização para clientes HTTP (teste de compilação).
