# Spike 04 — Captura dupla (mic + sistema) e drift

**Fase:** 0 · **Status:** Linux medido; Windows pendente · **Risco:** design doc §7

## Objetivo

Medir quanto o microfone ("eu") e o áudio do sistema ("eles") se desalinham quando gravados
juntos por uma reunião inteira. O design doc §7 lista o risco: "Drift entre mic e loopback →
'eu' e 'eles' desalinhados → spike 4; correção por timestamp a cada N s". O pitch pede mic +
loopback por 60 min, WAV estéreo L/R, e o drift medido por um clique sincronizado no início e no
fim; o spike mede, não corrige.

Esta é a metade Linux: PipeWire, alto-falante interno e microfone interno. A metade Windows
(WASAPI loopback, fone comum, Voicemeeter VAIO) fica na seção `## Windows`.

Este relatório mede; não recomenda.

## Como reproduzir

`fala-cli record` grava L = mic e R = sistema num WAV de 48 kHz, estéreo, i16, pareando os frames
por ordem de chegada, sem reamostrar nem ressincronizar, e reescreve o cabeçalho a cada 10 s.
Toca um clique (1 kHz, 20 ms, amplitude 0.5) no alto-falante 2 s depois do início e 2 s antes do
fim. No Linux, o canal do sistema é o monitor do sink, aberto pelo plugin ALSA do PipeWire
(`PIPEWIRE_NODE=<node.name do sink>` e `PIPEWIRE_ALSA='{ stream.capture.sink = true }'` só
enquanto o stream do sistema abre).

```sh
cargo build --release -p fala-cli
SINK=alsa_output.pci-0000_00_1f.3-platform-skl_hda_dsp_generic.HiFi__Speaker__sink

# Rodada 1: fala tocando no alto-falante durante toda a gravação (volume do sink em 0.68)
ffplay -nodisp -autoexit fala-research/benchmarks/audio/audio-aula-2025-05-02_14-56-18.m4a &
target/release/fala-cli record --system $SINK --duration 60m --out captura-60min.wav
target/release/fala-cli record --analyze captura-60min.wav

# Rodadas 2 e 3: só os cliques, nada mais tocando
target/release/fala-cli record --system $SINK --duration 60m --out captura-cliques-60min.wav
target/release/fala-cli record --analyze captura-cliques-60min.wav
```

Condições: alto-falante interno, microfone digital interno com ganho 0.35, ninguém usando o
áudio. Confira o volume do sink e o roteamento antes, durante e depois (`wpctl get-volume
@DEFAULT_AUDIO_SINK@`; um fone plugado troca o sink e o mic): o canal do sistema é gravado antes
do volume e não mostra essa mudança. O microfone interno só ouve o clique com o volume alto
(0.68 aqui; a 0.06 não ouve).

O WAV (~690 MB) fica fora do repositório.

## Evidência medida

Três rodadas de 60 min em 2026-09-27 no notebook Linux (Ubuntu 25.04, i7-1355U), sink
`alsa_output.pci-0000_00_1f.3-platform-skl_hda_dsp_generic.HiFi__Speaker__sink`, mic = entrada
padrão do ALSA (`default`, que no PipeWire é
`alsa_input.pci-0000_00_1f.3-platform-skl_hda_dsp_generic.HiFi__Mic1__source`, o microfone
digital interno), grafo do PipeWire a 48 kHz, build de release do commit `8b04aca` da branch
`feat/cli-record-capture` (o código de captura é o mesmo que entra em `main` no squash).

| Rodada | Horário (UTC) | Volume do sink | O que tocava |
| --- | --- | --- | --- |
| 1 | 19:03:24 → 20:03:30 | 0.68 | uma entrevista de `fala-research/benchmarks/audio/`, do começo ao fim, e os cliques |
| 2 | 20:14:08 → 21:14:09 | 0.06 (baixado entre as rodadas; notado só depois) | só os cliques |
| 3 | 22:50:31 → 23:50:31 | 0.68 nas 57 leituras por minuto, alto-falante e mic interno o tempo todo | só os cliques |

Uma tentativa às 22:48:28Z, antes da rodada 3, foi interrompida em ~23 s e descartada: o volume
estava em 0.98 e o `wpctl status` lido durante ela mostrava os streams ligados a `Headphones` e
`Headset Mono Microphone` (um fone plugado); o arquivo gravado não registra essa troca.

Resumo das gravações, como o `fala-cli record` imprime:

Rodada 1:

| rate | wall_s | mic_frames | sys_frames | mic_ppm | sys_ppm | rel_drift_ms | dropped_mic | dropped_sys | stream_errors | click_1_s | click_2_s | mic_peak | mic_rms_dbfs | sys_peak | sys_rms_dbfs |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 48000 | 3600.029 | 172797952 | 172801536 | -19.9 | 0.9 | -74.7 | 0 | 0 | 0 | 2.013 | 3598.016 | 0.398 | -53.3 | 1.000 | -23.6 |

Rodada 2:

| rate | wall_s | mic_frames | sys_frames | mic_ppm | sys_ppm | rel_drift_ms | dropped_mic | dropped_sys | stream_errors | click_1_s | click_2_s | mic_peak | mic_rms_dbfs | sys_peak | sys_rms_dbfs |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 48000 | 3600.060 | 172800000 | 172806144 | -16.8 | 18.8 | -128.0 | 0 | 0 | 0 | 2.010 | 3598.051 | 0.398 | -51.9 | 0.500 | -58.6 |

Rodada 3:

| rate | wall_s | mic_frames | sys_frames | mic_ppm | sys_ppm | rel_drift_ms | dropped_mic | dropped_sys | stream_errors | click_1_s | click_2_s | mic_peak | mic_rms_dbfs | sys_peak | sys_rms_dbfs |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 48000 | 3600.048 | 172800000 | 172808192 | -13.4 | 34.0 | -170.7 | 0 | 0 | 0 | 2.013 | 3598.039 | 0.398 | -47.9 | 0.500 | -56.8 |

Análise dos cliques da rodada 3 (`--analyze`, exit 0; `--click-threshold 0.05` dá 2.132 / 101.8 /
101.8 / 0.0, o mesmo drift):

| onset_mic_start_s | onset_sys_start_s | offset_start_ms | onset_mic_end_s | onset_sys_end_s | offset_end_ms | drift_ms | drift_ppm |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 2.133 | 2.234 | 100.8 | 3598.144 | 3598.245 | 100.8 | -0.0 | -0.0 |

(`-0.0` é o zero negativo do ponto flutuante na formatação: o drift é zero na resolução de 1
frame.)

As rodadas 1 e 2 não deram análise:

| Rodada | Saída do `--analyze` | Por quê |
| --- | --- | --- |
| 1 | exit 1, `mic/end: nenhum onset ≥ 0.1 na janela; pico 0.092` | nos 15 s finais, a fala chega a 0.97 no canal do sistema e cobre o clique; no mic, o maior pico é fala |
| 2 | exit 1, `mic/start: nenhum onset ≥ 0.1 na janela; pico 0.029` | com o sink a 0.06, o microfone não ouve o clique; em testes de 10 s depois da rodada, com o volume lido por `wpctl` no mesmo minuto, a 0.18 o mic fica em 0.002 nos instantes dos cliques e a 0.68 os cliques chegam a 0.095-0.136 contra ruído mediano de 0.0017-0.0019 (WAVs e `peaks.txt` em `~/.cache/fala-bench/recordings/diag/`, fora do repo) |

Diferença `sys_frames − mic_frames` nas linhas de progresso (uma por minuto, 59 por rodada):

| Rodada | Valores (frames) | Em ms | Tendência |
| --- | --- | --- | --- |
| 1 | de 1024 a 7168; ~6144 no minuto 1, ~2048 no minuto 59 | 21 a 149 | cai ~4000 frames (~85 ms) na hora |
| 2 | 4096 em 39 minutos, 6144 em 20 | 85 ou 128 | nenhuma |
| 3 | 6144 em 55 minutos, 8192 em 4 | 128 ou 171 | nenhuma |

Leitura dos números:

- No PipeWire, as duas capturas são reamostradas para o clock do grafo (o driver é o sink
  interno), então mic e monitor andam no mesmo relógio. Por isso o número do Linux não prevê o Windows:
  lá o WASAPI entrega o mic e o loopback em clocks de dispositivos diferentes. O que o Linux
  prova é o mecanismo (três gravações de 60 min sem frame descartado nem erro de stream, arquivo
  válido) e o método: na rodada 3 os cliques aparecem nos dois canais e o offset é o mesmo no
  início e no fim.
- O offset de ~101 ms é constante: o mic chega ao WAV antes do monitor (latências diferentes de
  cada stream na partida), não um drift.
- O `rel_drift_ms` (diferença de frames entregues ao fim) é deslocamento de partida mais a
  granularidade dos blocos de entrega (2048 frames = 42.7 ms), não drift de clock: nas rodadas 2
  e 3 a diferença por minuto não cresce. Na rodada 1 ela caiu ~85 ms ao longo da hora; a
  diferença de condição foi a fala tocando (outro cliente no grafo); esta rodada não separa as
  duas coisas.
- `mic_ppm` e `sys_ppm` comparam frames entregues com o relógio de parede e incluem o atraso de
  partida de cada stream.
- Os instantes `click_1_s` e `click_2_s` são quando o comando pediu o clique; no canal do
  sistema ele aparece ~0.2 s depois (buffer de saída e a linha do tempo do WAV começando no
  primeiro frame capturado). O drift usa só os onsets dentro do WAV.
- Contagem de xruns: só o que o callback de erro do `cpal` reporta (`stream_errors`); overruns
  recuperados dentro do ALSA não aparecem.

Ao lado da linha do design doc §7, "Drift entre mic e loopback → correção por timestamp a cada N s":
o `drift_ms` pelos cliques ficou em 0.0 na rodada 3, abaixo de 50 ms; o `rel_drift_ms` passou de
50 ms nas três rodadas (-74.7, -128.0, -170.7), pelos motivos acima.

## Windows

`TODO(windows)`: rodar no Alienware (Windows 11) depois do build por `docs/dev/build-windows.md`
(`cargo build --release -p fala-cli`). No Windows, `--system` é parte do nome de um dispositivo
de saída, aberto como loopback do WASAPI; `--mic` é parte do nome de uma entrada. Um nome que
não bate sai com 2 e lista os disponíveis.

```powershell
# 1. Fone comum (o uso real nas reuniões): sistema = saída do fone, mic = entrada padrão
target\release\fala-cli record --system "<nome da saída do fone>" --duration 60m --out C:\fala-captura\fone-60min.wav
target\release\fala-cli record --analyze C:\fala-captura\fone-60min.wav

# 2. Voicemeeter VAIO como mic (o uso real ao ditar)
target\release\fala-cli record --system "<nome da saída do fone>" --mic "Voicemeeter" --duration 60m --out C:\fala-captura\voicemeeter-60min.wav
target\release\fala-cli record --analyze C:\fala-captura\voicemeeter-60min.wav
```

Com fone, o clique não chega ao microfone pelo ar: segure o fone perto do microfone nos 15 s
iniciais e finais, ou ajuste `--click-threshold` olhando o pico que o `--analyze` imprime.

O que registrar em cada rodada: data, nomes exatos dos dispositivos (`--system`, `--mic`), a
tabela de resumo e a de análise, e se o WASAPI entregou o mic e o loopback na mesma taxa
(`mic_ppm` e `sys_ppm` longe um do outro indicam clocks diferentes, que é o caso que o design
doc §7 prevê).
