# Spike 04 — Captura dupla (mic + sistema) e drift

**Fase:** 0 · **Status:** Linux e Windows medidos · **Risco:** design doc §7

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

Três rodadas de 60 min em 2026-09-29 no Alienware 16 (Windows 11 Pro 25H2, build
10.0.26200.9457, Core 7 240H), build de release do `fala-cli` em `da03312`
(`cargo build --release -p fala-cli`). Como reproduzir, com os nomes exatos desta máquina:

```powershell
target\release\fala-cli record --system "Voicemeeter Input (VB-Audio Voicemeeter VAIO)" `
  --mic "Microfone (Realtek(R) Audio)" --duration 60m --out C:\fala-captura\r1-realtek-60min.wav
target\release\fala-cli record --system "Voicemeeter Input (VB-Audio Voicemeeter VAIO)" `
  --mic "Voicemeeter Out B2" --duration 60m --out C:\fala-captura\r2-voicemeeter-60min.wav
target\release\fala-cli record --analyze C:\fala-captura\<arquivo>.wav
```

### Roteamento desta máquina

O áudio passa todo pelo Voicemeeter Banana, e isso decide o que dá para gravar:

| Faixa do Voicemeeter | Dispositivo | Envia para |
| --- | --- | --- |
| Hardware Input 1 | `Microfone (Realtek(R) Audio)` (o mic físico; gate em 1.4) | B2 |
| Virtual "Desktop" | `Voicemeeter Input` (a saída padrão do Windows) | A1, B1 |
| Virtual "Communications" | `Voicemeeter AUX Input` | A1, B1 |
| Bus A1 | `Fones de ouvido (JBL Tour Pro 3)` (Bluetooth) | - |

A entrada padrão do Windows é `Voicemeeter Out B2`, que carrega só o mic físico. Lido pela API
remota do Voicemeeter (`VoicemeeterRemote64.dll`) em 2026-09-29.

O que não deu para gravar, e por quê:

| Tentativa | Resultado |
| --- | --- |
| `--system "Fones de ouvido (JBL Tour Pro 3)"` | exit 1, `não consegui abrir o stream system: ... 0x8889000A` (`AUDCLNT_E_DEVICE_IN_USE`): o Voicemeeter segura o JBL como A1 |
| `--mic "Headset (JBL Tour Pro 3)"` | exit 2, `o dispositivo não oferece 48000 Hz f32; oferece: 16000-16000 Hz ... 1 ch`: o mic do fone Bluetooth só abre a 16 kHz mono, e o `record` exige 48 kHz |
| `--mic "Voicemeeter Out B1"` | o B1 recebe a faixa Desktop: 86 % das amostras de L e R iguais, R atrasado 960 frames; não é um mic |
| `--system "Altofalantes (Realtek(R) Audio)"` com `--no-click` e nada tocando | exit 1 em 5 s, `o stream system não entrega frames há 5 s`: o loopback de um dispositivo sem stream ativo não entrega pacotes. O `Voicemeeter Input` entrega frames em silêncio (o Voicemeeter mantém o stream aberto) |
| `--mic "Voicemeeter Out B2"` com o sistema em silêncio | exit 1 em 2 s, `a captura do sistema caiu no microfone: os primeiros 96000 frames de L e R são idênticos`: o gate zera o mic e os dois canais são zero digital. Por isso a rodada 2 tem uma entrevista tocando desde o início |

Com o JBL no ouvido, o clique não chega ao mic físico. Para as rodadas 1 e 3 o Realtek é aberto
direto (o Voicemeeter também o usa como Hardware Input 1, em paralelo).

### Condições

| Rodada | Horário (UTC) | `--mic` | Clique até o mic | O que tocava | Atividade no notebook |
| --- | --- | --- | --- | --- | --- |
| 1 | 03:06:50 → 04:06:50 | `Microfone (Realtek(R) Audio)` | pelo ar: `Altofalantes (Realtek(R) Audio)` como A2 do Voicemeeter e Desktop→A2 ligado; `Voicemeeter Input` e alto-falantes com volume 1.00 e mic Realtek em 1.00 | só os cliques | o app Fala rodou os spikes 02 e 03 (03:13-03:24, 03:46-03:57, 03:57-04:07): ~1200 gravações curtas no `Voicemeeter Out B2` |
| 2 | 04:09:35 → 05:09:35 | `Voicemeeter Out B2` | não chega (A2 desligado; JBL) | `audio-aula-2025-05-13` a partir de 600 s, no JBL | nenhuma |
| 3 | 05:09:56 → 06:09:56 | `Microfone (Realtek(R) Audio)` | não chega (A2 desligado; JBL) | nada | nenhuma |

Nas rodadas 2 e 3, o roteamento e os volumes estavam restaurados ao original: A2 sem
dispositivo, `Voicemeeter Input` em 0.10, alto-falantes em 0.75, mic Realtek em 0.76 (o canal do
sistema é gravado antes do volume: o clique aparece a 0.500 nas três rodadas).

Duas partidas da rodada 2, às 04:08:18Z e 04:08:42Z, saíram com exit 1 em 2 s pelo guard acima,
com `audio-aula-2025-05-02` tocando desde o começo: o arquivo está em silêncio digital a partir
de ~5 s (-91 dB em 5 s e em 30 s, `ffmpeg -af volumedetect`), e as duas partidas caíram nesse
trecho (mapa dos silêncios em `fala-research/benchmarks/README.md`). A rodada que valeu usa o
`05-13`.

### Resumo das gravações

Rodada 1:

| rate | wall_s | mic_frames | sys_frames | mic_ppm | sys_ppm | rel_drift_ms | dropped_mic | dropped_sys | stream_errors | click_1_s | click_2_s | mic_peak | mic_rms_dbfs | sys_peak | sys_rms_dbfs |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 48000 | 3600.003 | 172799040 | 172784064 | -6.4 | -93.1 | 312.0 | 0 | 0 | 0 | 2.010 | 3598.090 | 0.163 | -55.3 | 0.500 | -54.2 |

Rodada 2:

| rate | wall_s | mic_frames | sys_frames | mic_ppm | sys_ppm | rel_drift_ms | dropped_mic | dropped_sys | stream_errors | click_1_s | click_2_s | mic_peak | mic_rms_dbfs | sys_peak | sys_rms_dbfs |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 48000 | 3600.054 | 172802400 | 172803360 | -1.1 | 4.5 | -20.0 | 0 | 0 | 0 | 2.015 | 3598.043 | 0.050 | -78.5 | 0.987 | -23.5 |

Rodada 3:

| rate | wall_s | mic_frames | sys_frames | mic_ppm | sys_ppm | rel_drift_ms | dropped_mic | dropped_sys | stream_errors | click_1_s | click_2_s | mic_peak | mic_rms_dbfs | sys_peak | sys_rms_dbfs |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 48000 | 3600.099 | 172804320 | 172804320 | -2.4 | -2.4 | 0.0 | 0 | 0 | 0 | 2.012 | 3598.086 | 0.030 | -69.3 | 0.500 | -57.7 |

### Análise dos cliques

Nenhuma rodada deu análise completa:

| Rodada | Saída do `--analyze` | Por quê |
| --- | --- | --- |
| 1 | exit 1 com 0.1, 0.05 e 0.03: `mic/start: nenhum onset ≥ … na janela; pico 0.017`; com 0.012 e 0.01 o onset do mic cai em 5.319 s e 1.407 s, antes do clique (ruído) | o clique inicial chegou ao mic a 0.017, perto do ruído; num teste de 20 s às 03:04 ele chegou a 0.056 e deu drift 1.0 ms com 0.05 |
| 2 | exit 1 com 0.1 e 0.03: `mic/start: … pico 0.007` | o clique não chega ao mic por este caminho |
| 3 | exit 1 com 0.1 e 0.03: `mic/start: … pico 0.001` | o clique não chega ao mic por este caminho |

O clique final da rodada 1 aparece nos dois canais (limiar 0.012): mic em 3598.432 s, sistema em
3597.912 s, offset de -519 ms (o mic chega depois).

### Diferença `sys_frames − mic_frames` por minuto

Rodada 1, os 59 valores (instante em s: frames):

```
60:960 120:960 180:480 240:960 300:960 360:480 420:960 480:1440 540:1440 600:960
660:960 720:1440 780:960 840:1056 900:1056 960:576 1020:1056 1080:-768 1140:-768 1200:-288
1260:-768 1320:-288 1380:-768 1440:-288 1500:-768 1560:-768 1620:-768 1681:-768 1741:-768 1801:-288
1861:-768 1921:-768 1981:-768 2041:-288 2101:-288 2161:-768 2221:-288 2281:-768 2341:-768 2401:-768
2461:-2112 2521:-2112 2581:-2112 2641:-2112 2701:-2112 2761:-4416 2821:-8448 2881:-8448 2941:-8448 3001:-8448
3061:-8832 3121:-8832 3181:-8832 3241:-9312 3301:-11136 3361:-13248 3422:-14592 3482:-14592 3542:-14976
```

| Rodada | Valores (frames) | Em ms | Tendência |
| --- | --- | --- | --- |
| 1 | de +1440 a -14976 | +30 a -312 | estável por 40 min (-768 a +1440); depois cai em degraus: 1020→1080 s (-1824), 2401→2461 s (-1344), 2701→2821 s (-6336), 3241→3422 s (-5280) |
| 2 | 480 em 57 minutos, 960 em 2 | 10 ou 20 | nenhuma |
| 3 | 0 em 38 minutos, 480 em 18, -480 em 3 | -10 a +10 | nenhuma |

Os degraus da rodada 1 caem nas janelas em que o app Fala abria e fechava gravações no
`Voicemeeter Out B2`: fim da primeira rodada do spike 02 (03:24:20Z = 1050 s), repetição do spike
02 (03:46:21Z-03:56:58Z = 2371-3008 s) e rodadas de pill do spike 03 (04:01:02Z-04:07:00Z =
3252-3610 s). A rodada 1 não separa essa atividade do resto; a rodada 3 repete a rodada 1 sem
ela.

### Leitura dos números

- Realtek aberto direto e sistema no `Voicemeeter Input`: a rodada 1 deu `mic_ppm` -6.4 e
  `sys_ppm` -93.1; a rodada 3, os mesmos dois dispositivos sem o app rodando, deu -2.4 e -2.4 ppm,
  o mesmo número de frames ao fim e diferença por minuto entre -480 e +480 frames (±10 ms) sem
  tendência. Os -93 ppm e os degraus da rodada 1 não se repetiram com o notebook quieto.
- Mic pelo Voicemeeter (`Out B2`, o uso real ao ditar): -1.1 e 4.5 ppm, e a diferença por minuto
  fica em 480 frames (10 ms) em 57 dos 59 minutos.
- Nenhuma das três rodadas descartou frame nem teve erro de stream.
- Nesta máquina, mic e sistema passam pelo Voicemeeter ou por um dispositivo que ele também abre
  (o Realtek); um Windows sem Voicemeeter não foi medido.

Ao lado da linha do design doc §7, "Drift entre mic e loopback → correção por timestamp a cada N
s": nas rodadas 2 e 3 a diferença entre os canais ficou em até 20 ms na hora, abaixo de 50 ms; na
rodada 1 passou de 50 ms (-312 ms ao fim, em degraus nas janelas de atividade do app). Não houve
`drift_ms` pelos cliques em nenhuma rodada.
