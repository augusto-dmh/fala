# Spike 01 — ASR em pt-BR: WER e RTF

**Fase:** 0 · **Status:** CLI pronta; rodadas com os cortes reais pendentes · **Decisão que alimenta:** ADR-0003

## Objetivo

Medir, na voz do Augusto e nas duas máquinas, o WER e o RTF dos modelos de ASR candidatos ao
ditado, para confirmar ou substituir a ADR-0003 (Parakeet-TDT-0.6B-v3 int8 local). A ADR se
apoia em números de leaderboard (WER pt ~6 %, treino majoritariamente pt-PT; 10-20x tempo real
em CPU) que nunca foram medidos aqui.

Modelos: Parakeet v3 int8, Nemotron 3.5 ASR Streaming 0.6B, whisper large-v3-turbo (CPU e
Vulkan) e faster-whisper turbo int8 no Linux; whisper turbo em CUDA, Voxtral e Parakeet no
Windows.

O Parakeet é medido pelo runtime que o produto embarca, `transcribe-rs` sobre ONNX Runtime (o
mesmo do `apps/desktop`), e não pelo sherpa-onnx que a ADR-0003 cita no título: o número que
importa é o do caminho que vai para o produto.

Este relatório mede; não recomenda. A leitura dos números contra a ADR-0003 é do Augusto.

## Como reproduzir

Cada rodada usa o commit do `fala` em `main` que contém `fala-cli bench`
(`git rev-parse --short HEAD`), anotado na tabela da rodada em
`fala-research/benchmarks/README.md` § Rodadas. Toda
medição usa build de release (a build `dev` compila o ggml em Debug e infla o RTF do gguf).

### Cortes e referências

Trechos das quatro entrevistas de `fala-research/benchmarks/audio/` (~2 de reunião com 10 min;
~3 de ditado, só a voz do Augusto, 1-3 min), convertidos para o único formato que a CLI aceita
(16 kHz, mono, PCM de 16 bits):

```sh
cd ~/projects/fala-research/benchmarks
ffmpeg -ss <início> -t <duração> -i audio/<arquivo>.m4a -ar 16000 -ac 1 -sample_fmt s16 audio/cuts/<stem>.wav
```

Referência: ElevenLabs Scribe v2 uma vez por corte, feita à mão pelo Augusto fora da CLI e
corrigida à mão, em `reference/<stem>.txt`. O áudio não sai da máquina pela CLI.

### Modelos

| Modelo | Origem | Conferência |
| --- | --- | --- |
| Parakeet-TDT-0.6B-v3 int8 | pasta `~/.local/share/com.pais.handy/models/parakeet-tdt-0.6b-v3-int8/` (baixada pelo app; `encoder-model.int8.onnx`, `decoder_joint-model.int8.onnx`, `nemo128.onnx`, `vocab.txt`) | - |
| whisper large-v3-turbo (ggml) | `curl -fL -o ggml-large-v3-turbo.bin https://blob.handy.computer/ggml-large-v3-turbo.bin` (URL já catalogada em `apps/desktop/src/managers/model.rs`) | sha256 `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69` |
| Nemotron 3.5 ASR Streaming 0.6B Q8_0 (GGUF) | `https://huggingface.co/handy-computer/nemotron-3.5-asr-streaming-0.6b-gguf/resolve/6d44e540bc31b0de1dbe174a3cea87f53a7f22fb/nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf` (entrada do catálogo do desktop) | sha256 `b94545b313b3223fda7b2857a52681da813935c2127643d1e9ff0c23d988089c` |
| faster-whisper large-v3-turbo int8 | baixado do Hugging Face pelo próprio `faster-whisper` (alias `large-v3-turbo`) | - |

Os arquivos de modelo ficam fora do repositório (aqui, `~/.cache/fala-bench/models/`).

### Invocações

```sh
cargo build --release -p fala-cli                    # CPU
cargo build --release -p fala-cli --features vulkan  # + Vulkan
B=~/projects/fala-research/benchmarks M=~/.cache/fala-bench/models
FALA=target/release/fala-cli

# Parakeet v3 int8 (ONNX Runtime; threads padrão, sem opção)
$FALA bench --cuts $B/audio/cuts --refs $B/reference --engine parakeet-onnx \
  --model ~/.local/share/com.pais.handy/models/parakeet-tdt-0.6b-v3-int8 --chunk-s 60 \
  --out $B/hyp/parakeet-v3-int8 --tag linux

# whisper large-v3-turbo, CPU com varredura de threads, e iGPU por Vulkan
for t in 4 8 12; do
  $FALA bench --cuts $B/audio/cuts --refs $B/reference --engine gguf \
    --model $M/ggml-large-v3-turbo.bin --threads $t --tag linux-cpu-t$t
done
target/release/fala-cli bench --cuts $B/audio/cuts --refs $B/reference --engine gguf \
  --model $M/ggml-large-v3-turbo.bin --device gpu --tag linux-vulkan   # build --features vulkan

# Nemotron 3.5 (GGUF pelo transcribe.cpp): ver rabbit hole abaixo
$FALA bench --cuts $B/audio/cuts --refs $B/reference --engine gguf \
  --model $M/nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf --tag linux

# faster-whisper turbo int8: hipóteses por script, WER pelo fala-cli
uv run $B/scripts/bench_faster_whisper.py --cuts $B/audio/cuts --out $B/hyp/faster-whisper-turbo-int8
$FALA bench --cuts $B/audio/cuts --refs $B/reference --hyp $B/hyp/faster-whisper-turbo-int8 --tag linux
```

`--chunk-s 60` no Parakeet evita o pico de RAM de um corte de 10 min inteiro nos 14 GB desta
máquina; a rodada registra o valor usado. O WER normaliza os dois lados igual (NFC, minúsculas,
tudo o que não é letra ou dígito vira espaço) e o agregado é Σ(S+D+I)/ΣN, nunca média de WERs.

## Evidência medida

Rodadas sobre os cortes reais: **pendentes** até existirem `audio/cuts/` e `reference/` (a
pergunta aberta 1 do plano: só o Augusto escolhe os trechos e roda o Scribe). Quando existirem,
cada rodada vira uma tabela em `fala-research/benchmarks/README.md` § Rodadas e esta seção
compara o WER e o RTF do Parakeet v3 int8 com os valores da ADR-0003 (WER pt ~6 %; 10-20x tempo
real).

### Rabbit holes (limite de 1 hora cada)

Medidos em 2026-09-27 nesta máquina (i7-1355U, 12 threads, Intel Graphics RPL-U), com o
código da branch `feat/cli-bench-engines` antes do squash em `main`, com um corte
de fumaça de 15 s de uma das entrevistas e referência provisória: só prova que o caminho roda; o
WER desses números não significa nada. Build `dev` (ggml em Debug), então o RTF também não é o
da rodada.

| Rabbit hole | Coube na hora? | O que se viu |
| --- | --- | --- |
| Vulkan na iGPU | sim, ~5 min | `device=Vulkan0`. Build com `--features vulkan` compilou; `vulkaninfo` lista `Intel(R) Graphics (RPL-U)` (integrated) e `llvmpipe` (CPU); `--device gpu` escolheu `device=Vulkan0` = `Intel(R) Graphics (RPL-U)`, não o `llvmpipe`; whisper turbo transcreveu o corte |
| Nemotron 3.5 | sim, ~10 min (medição bloqueada pelo idioma, não pelo tempo) | `device=cpu`. O GGUF do catálogo carrega no `transcribe.cpp` (arquitetura parakeet), mas a engine recusa `language = "pt"` (`unsupported language`, status 10): o modelo anuncia locais BCP-47 (`pt-BR`, `pt-PT`), e o `bench` passa `pt` fixo por critério do plano. Com `pt-BR` num teste local descartado, transcreveu o corte. Medir o Nemotron exige mudar o plano do `bench` (idioma por modelo); devolvido ao planejamento. NeMo-Speech.cpp não foi tentado: o GGUF já cobre o modelo |

Limitações que valem para toda rodada:

- Os cortes de "ditado" são fala conversacional das entrevistas, não ditado; o registro de ditado
  de verdade fica para a fase 1, com o histórico do app.
- O Parakeet ONNX não expõe número de threads (`ParakeetModel::load` não tem `intra_threads`):
  a varredura 4/8/12 vale só para a engine gguf.
- O RTF exclui a carga do modelo (`load_s` vai na legenda) e não mede latência ponta a ponta de
  ditado (VAD fecha → texto no campo), que é da fase 1.

## Windows

`TODO(windows)`: rodar no Alienware (Core 7 240H, 16 threads, RTX 5050 8 GB) depois do build por
`docs/dev/build-windows.md`. A RTX 5050 é Blackwell (sm_120) e exige CUDA 12.8+; se o build com
`--features cuda` não trouxer kernels para ela em 1 hora, registrar e seguir com Vulkan.

```powershell
cargo build --release -p fala-cli --features cuda     # whisper turbo em CUDA
cargo build --release -p fala-cli --features vulkan   # alternativa se o CUDA falhar
$B = "$HOME\projects\fala-research\benchmarks"; $M = "C:\fala-models"

# whisper large-v3-turbo na RTX 5050 (confira device= na legenda: tem de ser a NVIDIA)
target\release\fala-cli bench --cuts $B\audio\cuts --refs $B\reference --engine gguf `
  --model $M\ggml-large-v3-turbo.bin --device gpu --tag win-cuda

# Voxtral Mini 4B Realtime (GGUF do catálogo: handy-computer/Voxtral-Mini-4B-Realtime-2602-gguf, Q4_K_M,
# sha256 39dc1f65539373a406edea7490505822d77c12edff521744678717eef4da4723); se recusar "pt" como o Nemotron, registrar
target\release\fala-cli bench --cuts $B\audio\cuts --refs $B\reference --engine gguf `
  --model $M\Voxtral-Mini-4B-Realtime-2602-Q4_K_M.gguf --device gpu --tag win-cuda

# Parakeet v3 int8 no Core 7 (CPU)
target\release\fala-cli bench --cuts $B\audio\cuts --refs $B\reference --engine parakeet-onnx `
  --model <pasta parakeet-tdt-0.6b-v3-int8> --chunk-s 60 --tag win

# whisper turbo em CPU, varredura de threads
foreach ($t in 4, 8, 16) {
  target\release\fala-cli bench --cuts $B\audio\cuts --refs $B\reference --engine gguf `
    --model $M\ggml-large-v3-turbo.bin --threads $t --tag win-cpu-t$t
}
```

Registrar em § Rodadas, com a mesma forma das rodadas do Linux: data, máquina, engine, modelo,
versão, threads, device (o nome da legenda), corte, WER e RTF.
