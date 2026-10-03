# Spike 01 — ASR em pt-BR: WER e RTF

**Fase:** 0 · **Status:** rodadas Linux (2026-09-29) e Windows (2026-09-30) feitas · **Decisão que alimenta:** ADR-0003

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

Cinco trechos das quatro entrevistas de `fala-research/benchmarks/audio/`: três de "ditado"
(só a voz do Augusto, 2:20 a 3:00) e dois de reunião (10 min, o Augusto e dois ou três
entrevistadores), convertidos para o único formato que a CLI aceita (16 kHz, mono, PCM de 16
bits). O `benchmarks/README.md` § Cortes tem o comando exato de cada um e o sha256 de cada WAV,
além de como foram escolhidos (impressão de voz e transcrição locais):

```sh
cd ~/projects/fala-research/benchmarks
ffmpeg -ss <início> -t <duração> -i audio/<arquivo>.m4a -ar 16000 -ac 1 -sample_fmt s16 audio/cuts/<stem>.wav
```

Referência em `reference/<stem>.txt` (`benchmarks/README.md` § Referências): ElevenLabs Scribe
v2, uma vez por corte, num comando rodado pelo Augusto; depois revisada de ouvido pelo Augusto
nos 94 trechos em que nenhuma engine local confirmava o Scribe (19 linhas mudaram). Convenção:
verbatim, sem a hesitação `hã`/"ahn" e sem fragmentos de palavra. O áudio não sai da máquina
pela CLI.

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
    --model $M/ggml-large-v3-turbo.bin --threads $t --out $B/hyp/whisper-turbo-cpu-t$t --tag linux-cpu-t$t
done
target/release/fala-cli bench --cuts $B/audio/cuts --refs $B/reference --engine gguf \
  --model $M/ggml-large-v3-turbo.bin --device gpu \
  --out $B/hyp/whisper-turbo-vulkan --tag linux-vulkan   # build --features vulkan

# Nemotron 3.5 (GGUF pelo transcribe.cpp); o idioma vem da lista do modelo (ver abaixo)
$FALA bench --cuts $B/audio/cuts --refs $B/reference --engine gguf \
  --model $M/nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf --chunk-s 60 \
  --out $B/hyp/nemotron-3.5-q8 --tag linux

# faster-whisper turbo int8: hipóteses por script, WER pelo fala-cli
uv run $B/scripts/bench_faster_whisper.py --cuts $B/audio/cuts --out $B/hyp/faster-whisper-turbo-int8
$FALA bench --cuts $B/audio/cuts --refs $B/reference --hyp $B/hyp/faster-whisper-turbo-int8 --tag linux

# tabela da rodada: cada hyp/<config> pontuado por `bench --hyp` contra reference/
uv run $B/scripts/score_rounds.py --fala $FALA --date <data> --machine "<máquina>" --commit <commit>
```

`--chunk-s 60` no Parakeet e no Nemotron evita o pico de RAM de um corte de 10 min inteiro nos
14 GB desta máquina (o Nemotron sem janela chegou a 7,3 GB e foi morto por falta de memória com o
desktop aberto); a rodada registra o valor usado. As hipóteses e os `wall_s` ficam em `hyp/`;
o WER e o RTF da tabela saem do `bench --hyp`, que dá o mesmo resultado da engine (check C19).

Idioma na engine gguf: o `bench` pede `pt-BR` (ou o valor de `--language`) e resolve o código
contra a lista que o modelo anuncia: o código exato quando existe, senão o prefixo sem região
(`pt`, o que o whisper lista); um modelo que não anuncia nenhum dos dois sai com 2 listando o
que suporta. O código resolvido sai no stderr como `idioma: <código>` (o Nemotron registra
`idioma: pt-BR`, o whisper `idioma: pt`, então as rodadas do whisper seguem comparáveis). O WER normaliza os dois lados igual (NFC, minúsculas,
tudo o que não é letra ou dígito vira espaço) e o agregado é Σ(S+D+I)/ΣN, nunca média de WERs.

## Evidência medida

### Rodada Linux, 2026-09-29

`fala` `da03312`, i7-1355U (12 threads, 14 GB), na tomada, perfil `balanced`, build de release.
Cinco cortes (1.682 s: três de ditado, 482 s; duas reuniões, 1.200 s) contra as referências
revisadas. A tabela por corte está em `fala-research/benchmarks/README.md` § Rodadas; aqui, o
agregado Σ(S+D+I)/ΣN por tipo de corte:

| Engine | Threads · device | WER ditado | WER reunião | WER total | RTF | × tempo real |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Parakeet v3 int8 (`parakeet-onnx`, janela 60 s) | - · cpu | 8.94 % | 17.79 % | 15.03 % | 0.097 | 10.3× |
| whisper large-v3-turbo (`gguf`) | 4 · cpu | 8.32 % | 14.23 % | 12.39 % | 0.744 | 1.3× |
| whisper large-v3-turbo (`gguf`) | 8 · cpu | 7.88 % | 14.23 % | 12.25 % | 0.666 | 1.5× |
| whisper large-v3-turbo (`gguf`) | 12 · cpu | 7.88 % | 14.23 % | 12.25 % | 0.635 | 1.6× |
| whisper large-v3-turbo (`gguf`) | 12 · Vulkan0 (iGPU) | 7.94 % | 14.28 % | 12.31 % | 0.344 | 2.9× |
| Nemotron 3.5 Q8_0 (`gguf`, janela 60 s) | 12 · cpu | 10.63 % | 12.75 % | 12.09 % | 0.273 | 3.7× |
| faster-whisper turbo int8 (script) | padrão · cpu | 6.19 % | 10.58 % | 9.21 % | 0.662 | 1.5× |

Contra a ADR-0003, que cita para o Parakeet-TDT-0.6B-v3 WER pt ~6 % e 10-20x tempo real em CPU:

- **WER** (ADR-0003: ~6 %): o Parakeet v3 int8 mediu 8.94 % nos cortes de ditado, 17.79 % nas
  reuniões e 15.03 % no total.
- **Velocidade** (ADR-0003: 10-20x): RTF 0.097, 10.3× tempo real, no limite de baixo da faixa,
  sem contar a carga do modelo (~2,3 s) e com janela de 60 s.
- **Ordem** entre as engines: no ditado, faster-whisper < whisper turbo < Parakeet < Nemotron;
  nas reuniões, faster-whisper < Nemotron < whisper turbo < Parakeet.

Quanto esses números podem mexer:

- **Referências.** Antes da revisão de ouvido, o WER do Parakeet no ditado ia de 9.47 % (rascunho
  do Scribe como veio) a 7.87 % (cada trecho contestado resolvido a favor da maioria das
  engines); revisado, deu 8.94 %. A ordem das engines por tipo de corte foi a mesma nos três
  casos.
- **Grafia coloquial.** A referência é verbatim, e Parakeet e whisper "limpam" a fala (`pra` →
  `para`, `tá` → `está`, `tava` → `estava`). Contando essas grafias como a mesma palavra, o
  Parakeet vai a 7.99 % no ditado e 13.56 % no total; whisper turbo a 6.55 % e 10.77 %;
  faster-whisper a 5.56 % e 8.49 %; Nemotron quase não muda (10.49 % e 12.01 %). A ordem se
  mantém. `tu` → `você` não entra nessa conta: é outra palavra.
- **Janela.** O Nemotron sem janela (execução só de texto) deu 11.24 % no total, 0,85 ponto
  melhor; a janela veio da falta de memória. O Parakeet não foi medido sem janela.
- **Condições.** O mesmo Parakeet repetido deu RTF 0.097, 0.102 e 0.114 em três sessões (texto
  idêntico): cerca de 15 % de variação entre sessões. Nemotron e faster-whisper rodaram numa
  sessão diferente de Parakeet e whisper.
- **Amostra.** 482 s de ditado (1.599 palavras) e 1.200 s de reunião; fala conversacional de
  entrevista, não ditado de verdade (ver as limitações abaixo).

### Rabbit holes (limite de 1 hora cada)

Medidos em 2026-09-27 nesta máquina (i7-1355U, 12 threads, Intel Graphics RPL-U), com o
código da branch `feat/cli-bench-engines` antes do squash em `main`, com um corte
de fumaça de 15 s de uma das entrevistas e referência provisória: só prova que o caminho roda; o
WER desses números não significa nada. Build `dev` (ggml em Debug), então o RTF também não é o
da rodada.

| Rabbit hole | Coube na hora? | O que se viu |
| --- | --- | --- |
| Vulkan na iGPU | sim, ~5 min | `device=Vulkan0`. Build com `--features vulkan` compilou; `vulkaninfo` lista `Intel(R) Graphics (RPL-U)` (integrated) e `llvmpipe` (CPU); `--device gpu` escolheu `device=Vulkan0` = `Intel(R) Graphics (RPL-U)`, não o `llvmpipe`; whisper turbo transcreveu o corte |
| Nemotron 3.5 | sim, ~10 min | `device=cpu`. O GGUF do catálogo carrega no `transcribe.cpp` (arquitetura parakeet). Na primeira tentativa a engine recusou `language = "pt"` (`unsupported language`, status 10): o modelo anuncia locais BCP-47 (`pt-BR`, `pt-PT`) e o `bench` passava `pt` fixo. Desde então o `bench` resolve o idioma contra a lista do modelo e registra `idioma: pt-BR` no stderr; com isso o Nemotron transcreve o corte de fumaça sem flag. NeMo-Speech.cpp não foi tentado: o GGUF já cobre o modelo. Em 2026-09-28, build de release no `fala` `da03312`, mesmo corte de fumaça, sem `--language`, 3 execuções: stderr `idioma: pt-BR`, legenda `threads=12 device=cpu`, `load_s` 1,22-2,65, RTF 0,385-0,415 (`wall_s` 5,77-6,22 em 15 s de áudio). O WER (766,67 %) não significa nada: a referência é provisória e não transcreve o corte |

Limitações que valem para toda rodada:

- Os cortes de "ditado" são fala conversacional das entrevistas, não ditado; o registro de ditado
  de verdade fica para a fase 1, com o histórico do app.
- O Parakeet ONNX não expõe número de threads (`ParakeetModel::load` não tem `intra_threads`):
  a varredura 4/8/12 vale só para a engine gguf.
- O RTF exclui a carga do modelo (`load_s` vai na legenda) e não mede latência ponta a ponta de
  ditado (VAD fecha → texto no campo), que é da fase 1.

## Windows

### Rodada Windows, 2026-09-30

`fala` `881e77c`, Alienware 16 (Core 7 240H, 16 threads, 31,7 GB; RTX 5050 Laptop 8 GB, driver
616.56), Windows 11 Pro 10.0.26200, na tomada, plano `Balanced`, build de release. Os mesmos cinco
cortes e as mesmas referências revisadas da rodada Linux. A tabela por corte e o estado da máquina
estão em `fala-research/benchmarks/README.md` § Rodadas; aqui, o agregado Σ(S+D+I)/ΣN por tipo de
corte:

| Engine | Threads · device | WER ditado | WER reunião | WER total | RTF | × tempo real |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Parakeet v3 int8 (`parakeet-onnx`, janela 60 s) | - · cpu | 8.88 % | 17.56 % | 14.86 % | 0.063 | 15.9× |
| whisper large-v3-turbo (`gguf`) | 4 · cpu | 7.88 % | 13.89 % | 12.02 % | 0.404 | 2.5× |
| whisper large-v3-turbo (`gguf`) | 8 · cpu | 7.88 % | 13.89 % | 12.02 % | 0.305 | 3.3× |
| whisper large-v3-turbo (`gguf`) | 16 · cpu | 7.88 % | 13.89 % | 12.02 % | 0.285 | 3.5× |
| whisper large-v3-turbo (`gguf`) | 16 · Vulkan0 (RTX 5050) | 7.69 % | 14.23 % | 12.19 % | 0.016 | 62.5× |
| Nemotron 3.5 Q8_0 (`gguf`, janela 60 s) | 16 · cpu | 10.51 % | 12.61 % | 11.96 % | 0.084 | 11.9× |
| Nemotron 3.5 Q8_0 (`gguf`, janela 60 s) | 16 · Vulkan0 (RTX 5050) | 10.44 % | 12.58 % | 11.92 % | 0.030 | 33.3× |
| Voxtral Mini 4B Realtime Q4_K_M (`gguf`, janela 60 s) | 16 · Vulkan0 (RTX 5050) | 7.13 % | 13.40 % | 11.45 % | 0.126 | 7.9× |
| faster-whisper turbo int8 (script) | padrão · cpu | 6.57 % | 9.79 % | 8.78 % | 0.309 | 3.2× |

Contra a ADR-0003, que cita para o Parakeet-TDT-0.6B-v3 WER pt ~6 % e 10-20x tempo real em CPU:

- **WER** (ADR-0003: ~6 %): o Parakeet v3 int8 mediu 8.88 % nos cortes de ditado, 17.56 % nas
  reuniões e 14.86 % no total.
- **Velocidade** (ADR-0003: 10-20x): RTF 0.063, 15.9× tempo real no Core 7, sem contar a carga
  do modelo (~3,5 s) e com janela de 60 s.
- **Ordem** entre as engines: no ditado, faster-whisper < Voxtral < whisper turbo < Parakeet <
  Nemotron; nas reuniões, faster-whisper < Nemotron < Voxtral < whisper turbo < Parakeet.
- **Mesma engine nas duas máquinas:** o WER total ficou a até 0,43 ponto do Linux (Parakeet
  15.03 → 14.86 %, whisper turbo em CPU 12.25 → 12.02 %, Nemotron 12.09 → 11.96 %,
  faster-whisper 9.21 → 8.78 %). Os WAVs dos cortes não são os mesmos bytes (ffmpeg 9.0.2 aqui,
  7.1.1 no Linux), com o mesmo número de amostras. O RTF em CPU caiu de 0.097 para 0.063
  (Parakeet), de 0.635 para 0.285 (whisper, todas as threads) e de 0.273 para 0.084 (Nemotron).
- **Condições.** O Parakeet repetido no começo e no fim da rodada deu RTF 0.062 e 0.063, com o
  mesmo texto.

### Invocações

Os mesmos comandos de § Invocações, com estes caminhos e diferenças:

```sh
# CARGO_TARGET_DIR curto (docs/dev/build-windows.md); o link do Vulkan precisa de %VULKAN_SDK%\Lib no LIB
CARGO_TARGET_DIR='C:\f\cpu' cargo build --release -p fala-cli
LIB="$VULKAN_SDK\\Lib;$LIB" CARGO_TARGET_DIR='C:\f\vk' cargo build --release -p fala-cli --features vulkan
B=/c/dev/fala-research/benchmarks M=/c/fala-models   # modelos baixados do catálogo, sha256 conferido

# --device gpu só no build Vulkan; a legenda tem de dizer Vulkan0 = NVIDIA GeForce RTX 5050 Laptop GPU
/c/f/vk/release/fala-cli bench --cuts $B/audio/cuts --refs $B/reference --engine gguf \
  --model $M/Voxtral-Mini-4B-Realtime-2602-Q4_K_M.gguf --chunk-s 60 --device gpu \
  --out $B/hyp/voxtral-4b-q4-vulkan-win --tag win-vulkan

# faster-whisper: o PyAV 19 quebra o faster-whisper 1.2.1 (metadata_errors)
uv run --with 'av<19' $B/scripts/bench_faster_whisper.py --cuts $B/audio/cuts \
  --out $B/hyp/faster-whisper-turbo-int8-win
```

Cada configuração grava em `hyp/<config>-win/`; a tabela sai de `score_rounds.py` com a lista das
pastas `-win`.

### Rabbit holes (limite de 1 hora cada)

| Rabbit hole | Coube na hora? | O que se viu |
| --- | --- | --- |
| whisper em CUDA na RTX 5050 | não tentado | O CUDA Toolkit não está instalado nesta máquina (sem `nvcc` nem `CUDA_PATH`). A GPU foi medida por Vulkan (`--features vulkan`), como a iGPU no Linux |
| Vulkan na RTX 5050 | sim | O primeiro link falhou com `LNK1181: cannot open input file 'vulkan-1.lib'`; com `%VULKAN_SDK%\Lib` no `LIB`, compilou. Entre a iGPU Intel e a NVIDIA, `--device gpu` escolheu `Vulkan0 (NVIDIA GeForce RTX 5050 Laptop GPU)` em todas as execuções |
| Voxtral Mini 4B Realtime | sim | Carrega no `transcribe.cpp` (`voxtral_realtime`), `idioma: pt`. Sem janela, caiu no primeiro corte de reunião com `vk::Device::allocateMemory: ErrorOutOfDeviceMemory`, e no `ditado-2025-05-02` saiu com 23.93 % de WER (106 deleções); com `--chunk-s 60` rodou os cinco cortes (6.34 % nesse corte) |
