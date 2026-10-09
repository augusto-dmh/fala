---
status: accepted
date: 2026-10-02
---
# ASR do ditado roda localmente: Silero VAD v4 + Parakeet-TDT-0.6B-v3 int8 via transcribe-rs

Substitui a ADR-0003 por inteiro: reafirma o que dela se manteve e troca o que os spikes e o
código desmentiram. Ao aceitar esta ADR, a única edição na 0003 é `status: superseded by ADR-0009`.

## Contexto e problema
O ditado captura tudo o que a pessoa fala em qualquer app, inclusive conteúdo sensível, e pt-BR é
critério eliminatório. A ADR-0003 escolheu "Parakeet v3 via sherpa-onnx, utterance-level, com
Silero VAD v6", com números de leaderboard. A fase 0 mediu o Parakeet nas duas máquinas, sempre
pelo `transcribe-rs` (ONNX Runtime direto), que é o runtime que o `apps/desktop` herdado já
embarca: rodada Linux em 2026-09-29 (`docs/spikes/01-asr-pt-br.md`, PR #19) e rodada Windows em
2026-09-30 (mesmo relatório, PR #20), com os dados em `fala-research/benchmarks/README.md`
§ Rodadas (commits `3041c60` e `10f11b1` do fala-research). A fase 1 começa a mover o ditado para
`crates/audio` e `crates/asr` e precisa fixar o runtime, que vira porta de uma via quando o
`crates/asr` nascer em cima dele.

O que a 0003 afirmava, contra o medido (RTF sem a carga do modelo; janela de 60 s):

| A ADR-0003 afirma | Linux, i7-1355U | Windows, Core 7 240H | Leitura |
| --- | --- | --- | --- |
| WER pt de ~6 % | ditado 8,94 %, reunião 17,79 %, total 15,03 % | ditado 8,88 %, reunião 17,56 %, total 14,86 % | não se confirmou nestes cortes (fala conversacional, não ditado de verdade) |
| 10-20x tempo real em CPU | RTF 0,097 (10,3x); 0,097-0,114 entre sessões | RTF 0,062-0,063 (15,9x) | confirmado; o Linux fica no limite de baixo |
| ~1 ponto pior que o Whisper large-v3 | +1,06 ponto no ditado contra o whisper turbo | +1,00 ponto | bate no ditado contra o turbo; na reunião, +3,6 |
| ~2-3 pontos pior que a nuvem | não medido | não medido | sem evidência |
| 0,5-1 s depois de soltar a tecla | não medido | não medido | a trilha A da fase 1 mede de ponta a ponta |
| via sherpa-onnx | medido via `transcribe-rs` | idem | divergência de runtime |
| com Silero VAD v6 | o VAD não entra no `bench` | idem | nenhum spike mediu VAD |

A carga do modelo leva ~2,3 s no Linux e ~3,5 s no Windows, acima da meta de 3 s para o início a
frio (design doc §5): o primeiro ditado espera o modelo se a hotkey funcionar antes.

## Opções consideradas
* Parakeet-TDT-0.6B-v3 int8 via `transcribe-rs`, modo utterance, Silero VAD v4 via `vad-rs` (o que
  o desktop e o benchmark usam).
* Parakeet v3 via sherpa-onnx (`sherpa-rs`) com Silero v6, como a 0003 escreve.
* Parakeet via `transcribe-rs` com Silero v6 por inferência própria sobre `ort`.
* Nemotron 3.5 streaming, Whisper turbo local ou ASR em nuvem (as alternativas da 0003).

## Decisão
O ditado usa Parakeet-TDT-0.6B-v3 int8, local, em modo utterance (o VAD fecha a frase e ela é
transcrita inteira, enquanto a pessoa segue falando), pelo `transcribe-rs`, com Silero VAD v4 pelo
`vad-rs`. O áudio de ditado nunca sai da máquina. Nemotron fica como evolução para parciais na tela.

O runtime é o medido: o número do benchmark vale para o produto, já está no lockfile e roda nas
duas máquinas pelo mesmo caminho do desktop; sherpa-onnx exige build CMake fora da árvore e
mediria um runtime que ninguém embarca. No ditado, nas duas máquinas, o Parakeet é o mais rápido
em CPU, fica ~1 ponto atrás do whisper turbo e ~2,3-2,7 pontos atrás do faster-whisper turbo, e à
frente do Nemotron em WER e velocidade.

O Silero v4 **não** vem de spike: vem do código. O `vad-rs` só lê o v4 (tensores `h`/`c`; o v5 e o
v6 usam `state`), e a única evidência é o teste do `crates/audio` (silêncio não abre utterance,
fala real abre). O v6 fica para quando houver motivo medido, por exemplo falso negativo no começo
de frase, com um VAD próprio sobre `ort`.

### Consequências
* Bom: offline; sem custo; pontuação e capitalização nativas; um só runtime ONNX no binário.
* Ruim: WER de ditado ~8,9 % e ~15 % no total nos cortes da fase 0, acima do que a 0003 contava;
  depende do LLM (ADR-0004) e do dicionário para nomes próprios. O Silero v4 é mais antigo que o v6
  e vem de um fork git (`cjpais/vad-rs`, liberado em `deny.toml`). A carga de ~3,5 s no Windows
  passa da meta de início a frio.
* Obrigatório: o trait `Transcriber` de `crates/asr` isola o runtime, para a troca continuar
  barata; o trace do ditado (`FALA_TRACE`) separa a carga do modelo da transcrição.

## Confirmação
`crates/asr` e `crates/audio` não dependem de `sherpa-onnx`/`sherpa-rs` (testes de manifesto);
`fala-cli bench` mede o Parakeet pelo `transcribe-rs`; `fala-cli dictate` com `FALA_TRACE=1`
registra a carga e o "soltar → texto" de cada ditado, a medida que a 0003 nunca teve. O tipo de
áudio de ditado não implementa serialização para clientes HTTP (ADR-0003, mantido).
