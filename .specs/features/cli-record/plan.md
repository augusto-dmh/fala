# cli-record — captura dupla (mic + sistema) e medição de drift

## Problem

A fase 2 depende de gravar o microfone ("eu") e o áudio do sistema ("eles") em dois canais por
uma reunião inteira e de os dois continuarem alinhados até o fim. O design doc §7 lista o risco
literalmente: "Drift entre mic e loopback → 'eu' e 'eles' desalinhados → spike 4; correção por
timestamp a cada N s". O pitch da fase 0 pede: "mic + loopback por 60 min, WAV estéreo L/R, medir
drift no fim com um clique sincronizado no início e no fim" e avisa que "drift entre clocks de
dois dispositivos é conhecido; o spike só precisa medir quanto, não corrigir". Hoje `fala-cli
record` é um stub e ninguém mediu nada em nenhuma das duas máquinas.

Esta máquina é a metade Linux do spike: PipeWire expõe o monitor do sink e o `cpal` já está na
árvore. A metade Windows (WASAPI loopback, fone comum, Voicemeeter VAIO) só roda lá e fica
`TODO(windows)` com os comandos prontos. Aviso que muda a leitura do número: no PipeWire as duas
capturas são reamostradas para o clock do grafo, então um drift ~0 no Linux não prevê o Windows;
o que o Linux prova é o mecanismo (60 min sem falha, arquivo válido) e o método de medição.

Quando isto fechar, `fala-cli record --system <sink> --duration 60m --out reuniao.wav` grava um
WAV estéreo (L = mic, R = sistema) com um clique no início e um no fim, imprime quantos frames
cada stream entregou contra o relógio de parede, e `fala-cli record --analyze reuniao.wav` mede o
deslocamento entre os cliques nos dois canais e o drift em ms e ppm.

## Flow

Reusa `cpal` (mic, monitor via plugin ALSA do PipeWire, loopback WASAPI no Windows por
`build_input_stream` num dispositivo de saída) e `hound`, ambos já no lockfile via
`apps/desktop`; nada de `pipewire-rs`, `pw-record` ou JACK.

1. `fala-cli record (--out <wav> --duration <Ns|Nm|Nh> --system <nome> [--mic <nome>] [--no-click] [--flush-s N] | --analyze <wav> [--click-threshold X])` -> `Cli` clap em `apps/cli/src/main.rs` (exists) - valida a combinação; inválida sai com 2 antes de tocar áudio
2. resolução de dispositivos (new, no door - placement per conventions) - mic: dispositivo de entrada do `cpal` cujo nome contém `--mic`, ou o padrão; sistema: no host ALSA, `PIPEWIRE_NODE` + `PIPEWIRE_ALSA` apontam o monitor do sink e o dispositivo aberto é `pipewire` (door 1); em outro host, o dispositivo de saída cujo nome contém `--system`, aberto como entrada (loopback do `cpal`, exists)
3. dois `cpal::Stream` de entrada (exists) - cada callback empurra f32 no seu ring buffer e conta frames; erros do callback de erro são contados
4. escritor (new, no door - placement) - pareia L=mic R=sistema por índice, sem ressincronizar, grava `hound::WavWriter` 48 kHz estéreo i16 e chama `flush()` a cada `--flush-s` (door 2); aos 2 s e a 2 s do fim toca um clique num `cpal::Stream` de saída (exists) e anota o instante
5. `--analyze` (new, no door - placement) - lê o WAV, acha o primeiro onset por canal nos 15 s iniciais e nos 15 s finais, calcula offset e drift (door 3)
6. out: tabela Markdown de resumo no stdout (gravação ou análise), progresso e erros no stderr; exit 0/1/2

Todo o código novo fica em `apps/cli`. Não há `#[cfg(target_os)]`: a diferença Linux/Windows é
decidida em tempo de execução por `cpal::Host::id()` (ADR-0007 reserva `cfg` aos crates de
plataforma e ao desktop). A migração para `crates/audio` é da fase 2/3.

## Impact

| Front | What changes |
| --- | --- |
| domain | novo termo: `system channel` - o áudio que sai pelo sink (o que a outra pessoa fala numa reunião), capturado como monitor no PipeWire e como loopback no WASAPI; canal R do WAV |
| domain | novo termo: `drift` - diferença, em ms e ppm, entre o que os dois streams entregaram no mesmo intervalo de parede; medido por contagem de frames e por deslocamento dos cliques |
| build | `apps/cli` passa a linkar `cpal` (libasound no Linux; o CI já instala `libasound2-dev` para o desktop) e `rtrb`; nenhum crate novo no grafo do workspace, só arestas de `fala-cli` no `Cargo.lock` |
| stored data | nada a migrar: o WAV de 60 min (~690 MB a 48 kHz estéreo i16) fica fora do repo, em `~/projects/fala-research/benchmarks/` ou onde `--out` apontar |
| process | `unsafe { std::env::set_var }` (edition 2024) antes de qualquer thread existir, só no host ALSA; documentado no código como mecanismo de spike |

## Relations

`None - no stored-data shape change` (um arquivo WAV por gravação, lido só por `--analyze`).

## Surface

Só o subcomando que esta feature implementa.

| Route | In | Out | Status |
| --- | --- | --- | --- |
| `fala-cli record` | modo gravação: `--out <wav>`, `--duration <Ns/Nm/Nh>`, `--system <nome>` (Linux: `node.name` do sink no PipeWire; Windows: nome do dispositivo de saída), `--mic <nome>` (default: entrada padrão), `--no-click`, `--flush-s N` (default 10). Modo análise: `--analyze <wav>`, `--click-threshold X` (default 0.1) | stdout: tabela Markdown de resumo (gravação: `rate wall_s mic_frames sys_frames mic_ppm sys_ppm rel_drift_ms dropped_mic dropped_sys stream_errors click_1_s click_2_s mic_peak mic_rms_dbfs sys_peak sys_rms_dbfs`; análise: `onset_mic_start_s onset_sys_start_s offset_start_ms onset_mic_end_s onset_sys_end_s offset_end_ms drift_ms drift_ppm`); stderr: progresso a cada 60 s e erros; disco: o WAV | exit `0` sucesso · `1` falha de stream, canal caído, fallback detectado ou onset não encontrado (arquivo mantido) · `2` argumentos, dispositivo não encontrado, saída indisponível para o clique, WAV fora do formato; sem status HTTP (comando local; 200-599 n/a) |

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. captura do monitor do sink no Linux pelo plugin ALSA do PipeWire, via `cpal` | antes de abrir o stream do sistema: `PIPEWIRE_NODE=<node.name do sink>` e `PIPEWIRE_ALSA='{ stream.capture.sink = true }'`, dispositivo `cpal` de nome `pipewire`; as duas variáveis são removidas antes de abrir o mic. Medido nesta máquina em 2026-09-27: com as duas variáveis, 0 amostras não nulas em 2 s sem reprodução e 47 503 com; só `PIPEWIRE_NODE` cai no microfone (1 574 de pico em silêncio). Deps em `apps/cli/Cargo.toml`: `cpal = "0.16.0"`, `rtrb = "0.4.0"`, `hound = "3.5.1"` | crate `pipewire`: libpipewire-dev + bindgen/libclang no CI para um spike, e é o caminho da fase 3, não deste. `pw-record` como subprocesso: dois arquivos e um caminho de código que o Windows não compartilha. Host JACK do `cpal`: libjack e `pw-link` à mão em cada rodada |
| 2. formato do WAV de captura dupla | 48 kHz, 2 canais, i16, L = mic, R = sistema, frames pareados por índice de chegada sem ressincronização, cabeçalho reescrito por `WavWriter::flush()` a cada `--flush-s`; um processo morto deixa o arquivo legível até o último flush | 16 kHz mono por canal com resample: esconde o drift que o spike existe para medir. Dois arquivos mono: o alinhamento vira problema do leitor e o pitch pede L/R |
| 3. definição das métricas de drift | `X_ppm = (X_frames / (rate × wall_s) − 1) × 1e6`; `rel_drift_ms = (mic_frames − sys_frames) / rate × 1000`; `offset_ms = (onset_sys − onset_mic) × 1000` por janela; `drift_ms = offset_end − offset_start`; `drift_ppm = drift_ms / ((onset_mic_end − onset_mic_start) × 1000) × 1e6`; onset = primeiro frame da janela com `abs(x) ≥ click-threshold` (escala cheia) | correlação cruzada: precisa de FFT e mais código para um sinal que é um clique; timestamps de callback do `cpal`: não são expostos de forma portátil |

- Nothing else in this change is hard to reverse: nomes de flag, tamanho do ring buffer, forma do clique e layout do módulo mudam num commit e nada fora do repo os consome.

## Criteria

### S1: gravação dupla por 60 min (P1)

Um comando grava mic e sistema num WAV estéreo válido, mesmo se morrer no meio, e diz quantos frames cada lado entregou.

**Acceptance Criteria**

1. WHEN `--out <wav> --duration <D> --system <nome>` é dado THEN o sistema SHALL abrir o stream do mic e o do sistema, gravar por D e escrever `<wav>` com 48 000 Hz, 2 canais, i16, L = mic e R = sistema, saindo com 0
2. The system SHALL chamar `WavWriter::flush()` a cada `--flush-s` segundos (default 10), de modo que um processo morto com SIGKILL deixe `<wav>` legível pelo `hound` com todos os frames gravados até o último flush
3. WHILE grava, o sistema SHALL escrever no stderr, no máximo uma vez a cada 60 s, uma linha com os segundos decorridos, `mic_frames` e `sys_frames`
4. IF os primeiros 96 000 frames de L e R são idênticos amostra a amostra THEN o sistema SHALL sair com 1 dizendo que a captura do sistema caiu no microfone, mantendo o arquivo
5. IF um stream não entrega nenhum frame por 5 s THEN o sistema SHALL sair com 1 nomeando o stream (`mic` ou `system`), mantendo o arquivo
6. WHEN a gravação termina THEN o sistema SHALL imprimir no stdout uma tabela Markdown com `rate`, `wall_s`, `mic_frames`, `sys_frames`, `mic_ppm`, `sys_ppm`, `rel_drift_ms`, `dropped_mic`, `dropped_sys`, `stream_errors`, `click_1_s`, `click_2_s`, `mic_peak`, `mic_rms_dbfs`, `sys_peak`, `sys_rms_dbfs`, calculados como a door 3
7. IF `--mic` é dado e nenhum dispositivo de entrada tem nome que o contenha THEN o sistema SHALL sair com 2 listando os nomes de entrada disponíveis
8. IF o host não é ALSA e nenhum dispositivo de saída tem nome que contenha `--system` THEN o sistema SHALL sair com 2 listando os nomes de saída disponíveis
9. WHEN o ring buffer de um stream enche THEN o sistema SHALL descartar os frames que não couberam e contá-los em `dropped_mic`/`dropped_sys`, nunca bloquear o callback

**Independent test:** `fala-cli record --system alsa_output.pci-0000_00_1f.3-platform-skl_hda_dsp_generic.HiFi__Speaker__sink --duration 20s --out t.wav` com um vídeo tocando; `ffprobe t.wav` mostra 2 canais 48 kHz; `kill -9` aos 15 s deixa ≥ 10 s legíveis.

### S2: cliques sincronizados (P1)

Um clique no início e um no fim, tocados pelo próprio comando, marcam os dois canais.

**Acceptance Criteria**

10. WHERE `--no-click` não é dado THEN o sistema SHALL tocar no dispositivo de saída padrão um burst senoidal de 1 000 Hz com 20 ms e amplitude 0.5, uma vez 2 s após os dois streams começarem e outra 2 s antes do fim, e SHALL registrar os dois instantes (segundos desde o início da gravação) em `click_1_s` e `click_2_s`
11. IF `--no-click` não é dado e o dispositivo de saída padrão não abre THEN o sistema SHALL sair com 2 antes de começar a gravar, dizendo que `--no-click` pula o clique
12. WHERE `--no-click` é dado THEN `click_1_s` e `click_2_s` SHALL ser `-`

**Independent test:** gravação de 10 s com os alto-falantes ligados; `--analyze` acha onsets em L e R perto de 2 s e de 8 s.

### S3: análise de drift (P1)

O WAV gravado devolve offset no início, offset no fim e o drift.

**Acceptance Criteria**

13. WHEN `--analyze <wav>` é dado THEN o sistema SHALL achar, por canal, o primeiro frame com `abs(x) ≥ --click-threshold` nos 15 s iniciais e o primeiro nos 15 s finais, e imprimir no stdout `onset_mic_start_s`, `onset_sys_start_s`, `offset_start_ms`, `onset_mic_end_s`, `onset_sys_end_s`, `offset_end_ms`, `drift_ms` e `drift_ppm` como a door 3, com 1 casa em ms e 3 em s
14. WHEN um WAV sintético de 48 kHz tem um clique em L a 2.000 s e em R a 2.010 s, e outro em L a 3 598.000 s e em R a 3 598.030 s THEN o sistema SHALL imprimir `offset_start_ms` = `10.0`, `offset_end_ms` = `30.0` e `drift_ms` = `20.0`
15. IF um canal não tem onset numa das janelas THEN o sistema SHALL sair com 1 nomeando canal e janela (`mic/start`, `sys/end`…) e imprimindo o pico daquele canal na janela
16. IF `<wav>` não tem 2 canais i16 THEN o sistema SHALL sair com 2 nomeando a especificação encontrada

**Independent test:** `cargo test -p fala-cli` com o WAV sintético do AC 14 gerado no teste.

### S4: interface (P1)

Combinação errada morre cedo; o stdout é só a tabela.

**Acceptance Criteria**

17. IF `--analyze` é combinado com `--out`, `--duration`, `--mic`, `--system`, `--no-click` ou `--flush-s` THEN o sistema SHALL sair com 2
18. The system SHALL aceitar `--duration` nas formas `<n>s`, `<n>m` e `<n>h` e SHALL sair com 2 para qualquer outra
19. The system SHALL escrever no stdout apenas a tabela de resumo; progresso, avisos e erros vão para o stderr

**Independent test:** `fala-cli record --analyze x.wav --duration 5s` sai com 2; `--duration 5min` sai com 2.

### S5: relatório (P2)

A evidência da metade Linux e os comandos da metade Windows.

**Acceptance Criteria**

20. WHEN o spike fecha THEN `docs/spikes/04-captura-dupla.md` SHALL existir com `## Objetivo`, `## Como reproduzir` e `## Evidência medida`, e SHALL NOT conter título com "Recomenda"
21. The section `## Evidência medida` SHALL trazer a tabela de resumo e a tabela de análise de uma gravação de 60 min nesta máquina, com data, `node.name` do sink e do mic, e o commit do `fala` usado
22. The section `## Evidência medida` SHALL registrar que no PipeWire as duas capturas são reamostradas para o clock do grafo e que o número do Linux não prevê o Windows
23. The report SHALL ter uma seção `## Windows` marcada `TODO(windows)` com as invocações prontas (`--system "<nome do dispositivo de saída>"` com fone comum; e com Voicemeeter VAIO como `--mic`) e o que registrar
24. WHERE `drift_ms` ou `rel_drift_ms` de 60 min ficar acima de 50 ms THEN o relatório SHALL dizer isso ao lado da linha "correção por timestamp a cada N s" do design doc §7, sem recomendar

**Independent test:** `grep -c '^## ' docs/spikes/04-captura-dupla.md` ≥ 4, `grep -ci recomenda` = 0, `grep -c 'TODO(windows)'` ≥ 1.

## Out of scope

Product capabilities only. Process and harness rules live in AGENTS.md or as Observable `n/a`.

| Excluded | Why |
| --- | --- |
| Corrigir o drift (resample adaptativo, timestamps a cada N s) | o pitch: "o spike só precisa medir quanto, não corrigir"; correção é fase 2 |
| Resample para 16 kHz, Opus, VAD, indicador de gravação | fase 2 (`crates/audio`); aqui o arquivo é cru para o número ser cru |
| Captura nativa por `pipewire-rs` ou portal | fase 3 (Linux GNOME Wayland); o spike usa o plugin ALSA que já existe |
| Medição no Windows (WASAPI loopback, Voicemeeter VAIO, fone) | só roda lá; comandos e seção `TODO(windows)` ficam prontos |
| Ctrl-C limpo | `--duration` é obrigatório e o `flush()` periódico já deixa o arquivo válido; um handler de sinal traria dependência nova |
| Trade study cpal loopback vs crate `wasapi` | documento próprio, só depois da medição no Windows (tarefa 4 do handoff) |
| Múltiplos sinks ou mics, mixagem, Bluetooth HFP | um par de dispositivos basta para medir; o headset é pergunta do pitch já respondida (fone comum) |

## Assumptions

Defaults that are not already a numbered criterion. Drop a row once it is.

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Taxa e formato nativos | pede 48 kHz f32 aos dois streams e converte para i16 na escrita; se um dispositivo não oferecer 48 kHz, sai com 2 nomeando as taxas suportadas | o mic e o sink desta máquina são 48 kHz; resample esconderia o drift | n |
| Windows: `--system` é o nome de um dispositivo de saída aberto com `build_input_stream` | mesmo binário, sem `cfg`; `cpal` 0.16 liga `AUDCLNT_STREAMFLAGS_LOOPBACK` nesse caso | é o caminho que a fase 2 vai usar; fica `TODO(windows)` até rodar lá | n |
| Clique pelo alto-falante interno, mic interno aberto | o atraso acústico (~1 ms por 34 cm) é constante e some em `drift_ms = offset_end − offset_start` | é o que o pitch descreve; um cabo de loopback físico não existe aqui | n |
| Capacidade do ring buffer | 2 s por stream (96 000 frames); estouro conta em `dropped_*` | o escritor acorda a cada 100 ms; 2 s cobre um GC de disco | n |
| `unsafe { env::set_var }` no host ALSA | feito antes de qualquer thread, com as duas variáveis removidas depois de abrir o stream do sistema | é código de spike em `apps/cli`; a fase 3 troca por `pipewire-rs` | n |
| Detecção de onset | limiar absoluto `0.1` de escala cheia, sem filtro; a pessoa ajusta com `--click-threshold` olhando o pico impresso pelo AC 15 | um clique de amplitude 0.5 no monitor e o mic a meio metro passam de 0.1; ruído de sala não | n |
| A rodada de 60 min | feita pela sessão de build nesta máquina, com volume ≥ 50 %, um vídeo com fala tocando e ninguém usando o áudio | é a evidência que o AC 21 pede; não precisa de pessoa falando | n |
| Contagem de xruns | só o que o callback de erro do `cpal` reporta; overruns recuperados internamente pelo ALSA não aparecem; registrado como limitação no relatório | `cpal` não expõe contador de xrun portátil | n |

**Open questions:** none - all resolved or logged above.

## Observable

Worksheet, not the review. `n/a` needs its reason. One row may group the same decision
across several routes.

| Surface | Decision | Landing |
| --- | --- | --- |
| command `fala-cli record` | output format (gravação) | AC 6 |
| command `fala-cli record` | output format (análise) | AC 13 |
| command `fala-cli record` | verbosity and progress | AC 3, AC 19 |
| command `fala-cli record` | every flag and its default | AC 1, 2, 10, 12, 13, 18; `--mic` default entrada padrão (AC 7) |
| command `fala-cli record` | exit codes | AC 7, 8, 11, 16, 17, 18 (2); AC 4, 5, 15 (1); AC 1 (0) |
| command `fala-cli record` | what it prints when it fails halfway | AC 4, AC 5 - sai com 1 nomeando a causa e mantém o arquivo válido até o último flush (AC 2) |
| command `fala-cli record` | wrong device silently recording the wrong thing | AC 4 |
| document `docs/spikes/04-captura-dupla.md` | structure | AC 20 |
| document `docs/spikes/04-captura-dupla.md` | depth | AC 21, AC 23 |
| document `docs/spikes/04-captura-dupla.md` | what the reader does next | AC 24 - compara com a linha do design doc §7 sem recomendar |
| document `docs/spikes/04-captura-dupla.md` | tone | existing - formato do relatório 11 §3b: objetivo, como reproduzir, evidência medida, sem recomendação |
| collection | n/a - nenhuma coleção nova; o WAV é um arquivo por gravação fora do repo |
| screen | n/a - a fase 0 não tem UI; o indicador de gravação da ADR-0005 é da fase 2 |
| API or webhook | n/a - nada sai pela rede; o áudio fica no disco local |

## Sources

- `~/projects/fala-research/pitches/fase-0-spikes.md` - o spike 4 e o rabbit hole "medir, não corrigir"
- `docs/design/2026-10-fala-v1.md` §7 - a linha "Drift entre mic e loopback → correção por timestamp a cada N s"
- `~/projects/fala-research/benchmarks/README.md` - setup de áudio do Windows (fone comum, Voicemeeter VAIO) que a seção `TODO(windows)` cita
- `ARCHITECTURE.md` - `crates/audio` como destino futuro (cpal, loopback WASAPI, PipeWire) e ADR-0007 sobre `cfg`
- medição de 2026-09-27 nesta máquina (`pw-record` e `arecord -D pipewire` com `PIPEWIRE_NODE` + `PIPEWIRE_ALSA`) - o mecanismo da door 1
