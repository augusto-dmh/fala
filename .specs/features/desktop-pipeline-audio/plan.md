# desktop-pipeline-audio (F9a: captura e VAD do desktop pelo `fala-audio`)

## Problem

A captura do microfone e o VAD existem duas vezes. O desktop grava pelo `AudioRecorder` herdado do Handy (`audio_toolkit/audio/recorder.rs`, `FrameResampler`, `SmoothedVad` sobre Silero ou Earshot), enquanto `crates/audio` (`Mic` + `DictationCapture` + `SileroVad`, #47) já faz o mesmo caminho com o pré-buffer de 300 ms e foi medido headless no Windows (`fala-cli dictate`, 212-523 ms de soltar até texto). O desktop não tem pré-buffer: o que entra antes do `Cmd::Start` é descartado (`ChunkDisposition::Discard`), e a primeira sílaba depende só da velocidade de abrir o mic. Cada correção no segmentador precisa ser feita duas vezes, e o VAD do app pode ser o Earshot, que a ADR-0009 não prevê.

Quando isto entra, o `AudioRecordingManager` grava pelo `fala-audio`: `Mic` abre a entrada escolhida nos settings, `DictationCapture` reamostra para 16 kHz, guarda os últimos 300 ms enquanto o stream está aberto e segmenta com o Silero v4 de `resources/models/silero_vad_v4.onnx`. O que o manager devolve no `stop_recording` continua sendo um `Vec<f32>` a 16 kHz mono, agora a concatenação das utterances da sessão.

## Flow

Reusa `Mic`, `DictationCapture`, `SileroVad`, `VoiceDetector` (`fala-audio`, exists), `AudioVisualiser` (desktop, exists), a resolução de dispositivo do manager (cache, clamshell, fallback para o padrão; exists) e o `StreamRouter` (exists).

1. a pessoa aperta o atalho: `TranscribeAction::start` chama `try_start_recording` como hoje; sob demanda, o manager resolve o `cpal::Device` dos settings e abre o `DictationRecorder` novo (`apps/desktop/src/dictation_capture.rs`)
2. o worker do recorder abre `fala_audio::Mic::open_device(device, channel)` (door 4) e monta um `DictationCapture` na taxa do mic com o Silero carregado uma vez por recorder (não por abertura)
3. a cada 10 ms o worker drena o mic para o `DictationCapture`: fora de uma sessão só alimenta o pré-buffer de 300 ms; numa sessão alimenta as barras da pill (`AudioVisualiser`, como hoje) e o VAD
4. `Start(policy)`: o que o mic já entregou entra antes; `DictationCapture::start` abre a sessão com o pré-buffer; o primeiro bloco depois dele resolve o `RecordingReadiness` (a pill e o som de início não mudam)
5. cada utterance que o VAD fecha vai para o `StreamRouter` (modelos com streaming) e para o buffer da sessão
6. `Stop`: drena o mic, `DictationCapture::stop` entrega a utterance aberta; o recorder devolve a concatenação. Daqui para frente nada muda (padding de áudio curto, ASR, LLM, colagem)
7. `VadPolicy::Disabled` (`vad_enabled = false`) faz o detector responder "voz" em todo quadro: a sessão inteira, com o pré-buffer, vira uma utterance

## Impact

| Front | What changes |
| --- | --- |
| domain | o áudio de um ditado ganha até 300 ms antes do `Start` quando o stream já estava aberto (always-on ou janela de `lazy_stream_close`); sob demanda, o pré-buffer tem o que chegou entre abrir o mic e o `Start` |
| domain | segmentação: onset 60 ms, pre-roll 450 ms, hangover 450 ms, corte sem perda aos 15 s (`DictationCapture`, o mesmo desenho do `SmoothedVad` offline). O perfil streaming (hangover 1650 ms) sai: modelos com streaming recebem utterances fechadas em vez de quadros de 30 ms |
| domain | o VAD do ditado é sempre o Silero v4 (ADR-0009); `vad_backend = "earshot"` num store antigo é lido e ignorado |
| stored data | nenhuma mudança de schema; `vad_backend` continua no store por compatibilidade |
| platform | entradas sem configuração f32 na taxa padrão falham ao abrir (`UnsupportedConfig`) em vez de serem convertidas; WASAPI compartilhado e o plugin ALSA do PipeWire (ADR-0013) entregam f32 |

## Surface

| Surface | Change |
| --- | --- |
| `fala_audio::Mic::open_device(cpal::Device, Option<usize>)` | novo: abre um dispositivo já resolvido, opcionalmente um canal só |
| `fala_audio::Mic::failed()` | novo: o callback de erro do `cpal` marcou o stream como perdido (dispositivo removido) |
| comando Tauri `change_vad_backend_setting` | removido, com o seletor "Backend de detecção de voz" da aba Avançado |
| `get_microphone_channels`, `set_selected_channel`, seletor de dispositivo | sem mudança de contrato |

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. mic sempre aberto × abrir ao apertar | o default continua **sob demanda** (`always_on_microphone = false`, já no store). O pré-buffer de 300 ms vale sempre que o stream está aberto antes do `Start`: com `always_on_microphone = true`, na janela de `lazy_stream_close` e entre abrir e o `Start`. Medido no Windows (log do app instalado, 2026-10-09, 10 ditados): abrir 7,4-15,9 ms + primeiras amostras 20,5-21,9 ms depois, ~28-38 ms da tecla à captura, dentro dos 50 ms | mic sempre aberto por padrão: o indicador "mic em uso" do Windows fica aceso o dia todo e o ganho é ~35 ms que já cabem na meta; pré-armar o mic no primeiro modificador do atalho: mexe no hotkey, fora do escopo da F9 |
| 2. o VAD do ditado | só o Silero v4 do `fala-audio`; o seletor Earshot e o comando `change_vad_backend_setting` saem; o enum `VadBackend` e o campo ficam para o store antigo carregar | manter o Earshot com um adaptador de quadros de 30 ms: a ADR-0009 fixa o Silero v4 e o Earshot nunca foi medido |
| 3. o que o recorder devolve | a concatenação das utterances da sessão (pré-buffer + pre-roll + fala + hangover), 16 kHz mono | devolver as utterances separadas: muda o contrato do `TranscriptionManager`, que é da F9b |
| 4. API de abertura do `Mic` | `Mic::open_device(device: cpal::Device, channel: Option<usize>)`; `channel` fora do alcance vira a média dos canais, como no herdado; `Mic::open(needle)` passa a delegar para ele | o desktop passar o nome para o `Mic::open(needle)`: o casamento por substring pode abrir "Microfone 2" quando a pessoa escolheu "Microfone"; o resolver herdado (cache, clamshell, fallback) já devolve o `cpal::Device` |

- O código herdado que sai do caminho (`recorder.rs` e os testes dele, `FrameResampler`, `vad/`, o `earshot`, o `bin/cli.rs` órfão) é apagado num PR empilhado só de remoção, com os `allow` de lint de `apps/desktop/Cargo.toml` que só ele usava. Separado porque a remoção passa de 2 mil linhas e o portão de tamanho do CI é 1000.

## Criteria

### S1: o desktop grava pelo `fala-audio` (P1)

1. WHEN o stream está aberto há mais de 300 ms antes do `Start` THEN o `DictationRecorder` SHALL entregar no `stop` os últimos 300 ms anteriores ao `Start` à frente da fala (VAD que sempre diz voz)
2. WHEN a política é `Disabled` THEN o recorder SHALL entregar todas as amostras do pré-buffer e da sessão, sem passar pelo detector
3. WHEN o VAD fecha utterances durante a sessão THEN o recorder SHALL entregar cada uma ao callback de áudio na ordem e devolver no `stop` a concatenação delas
4. WHEN o detector falha num quadro THEN o recorder SHALL tratar o quadro como voz (como o herdado) e registrar um `warn` uma vez por sessão
5. The `RecordingReadiness` SHALL resolver no primeiro bloco do mic depois do `Start`, e o `stop` antes disso SHALL descartá-lo
6. The barras da pill SHALL vir só de amostras de dentro da sessão
7. The Silero SHALL ser carregado uma vez por recorder; reabrir o stream não recarrega o modelo
8. WHEN o `cpal` reporta erro no stream THEN `needs_reopen()` SHALL valer `true` e o próximo `try_start_recording` reabre o stream (o caminho herdado de reconexão)

### S2: abertura do `Mic` (P1)

9. WHEN `channel` está dentro do número de canais THEN o `Mic` SHALL entregar só esse canal; fora do alcance ou `None`, a média
10. The `Mic::open(needle)` SHALL continuar abrindo a primeira entrada cujo nome contém `needle` (o `fala-cli` não muda)

### S3: app e documentos (P1)

11. The seletor de dispositivo de entrada SHALL continuar trocando o mic (o manager reabre o stream com o dispositivo novo)
12. WHEN o app roda no Windows (`bun run tauri dev`, store portátil) THEN `ctrl+shift+space` SHALL ditar no Bloco de Notas, e o log SHALL mostrar a abertura pelo `fala-audio` e o tempo tecla → primeiras amostras
13. The `ARCHITECTURE.md` SHALL dizer que a captura e o VAD do desktop são do `fala-audio`, e a linha do pré-buffer SHALL refletir a door 1

## Out of scope

| Excluded | Why |
| --- | --- |
| ASR do crate, catálogo de modelos (D8) | F9b |
| `FALA_TRACE` no desktop | F9c; aqui só o log `debug` de tempos que já existe |
| hotkey, colagem, pill | contratos sem mudança; semanas 5-6 |
| `set_mute`/`get_mute` (mudo durante a gravação) | continuam no `managers/audio.rs`, não são captura |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| door 1 | mic sob demanda por padrão, pré-buffer quando o stream já está aberto | atende ≤ 50 ms medido e não acende o indicador do SO | y — delegado (2026-10-09), decidido pelo executor |
| door 2 | Earshot sai | ADR-0009 | y — delegado (2026-10-09), decidido pelo executor |
| perfil streaming | hangover único de 450 ms | o modelo instalado no Windows tem `supports_streaming=false` (log de 2026-10-09); a F9b corta o catálogo | y — delegado (2026-10-09), decidido pelo executor |
| formatos não f32 | falham ao abrir | o `Mic` do crate só abre f32; WASAPI compartilhado é f32 | y — delegado (2026-10-09), decidido pelo executor |

**Open questions:** none - all resolved or logged above.

## Observable

- `info`: `microfone: <nome>, <taxa> Hz, <n> canal(is)` (já existe no crate) e `Microphone stream initialized in <t>` (herdado).
- `debug`: `dictation capture: first samples <t> after Start`, no lugar do log herdado `first captured samples`. Nenhum conteúdo de áudio nem texto.

## Sources

- `fala-research/plans/fase-1-delta-e-semanas-3-4.md` § F9 e § 3 D8
- `fala-research/plans/status/inventario-2026-10-09.md` § 1 e § 4 (itens 2, 4, 5)
- `docs/decisions/0009-*`, `0002-*`, `0007-*`; `ARCHITECTURE.md` (orçamento de latência)
- `%LOCALAPPDATA%\br.com.augusto.fala\logs\fala.log`, 2026-10-09 04:12-04:13 (tempos de abertura do mic)
