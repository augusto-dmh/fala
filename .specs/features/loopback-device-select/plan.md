# loopback-device-select plan

Profile: light · Fase 2 (F0/F2 do pitch: seletor do loopback)

## Problema

O gravador de reunião abre a saída padrão. Quando a call toca num fone USB que não é o padrão, o
canal "eles" grava zeros, e o `MuteWatch` só avisa depois de 2 min, sem dizer por quê.

## Entrega

1. `crates/audio`: `SystemAudio::outputs()` lista as saídas com a padrão marcada (nomes do `cpal`,
   os mesmos que `SystemAudio::open` aceita, agora com o nome exato antes do "contém");
   `OutputMeter` lê o pico de cada endpoint de render ativo (`IAudioMeterInformation`, só
   Windows, numa thread própria a cada 50 ms, lista relida a cada 3 s);
   `MeetingRecorder::set_system_rate` troca a taxa do canal do sistema sem parar o relógio.
   Exemplo `outputs` lista as saídas e o pico por N segundos.
2. `crates/meeting`: `OutputWatch` (canal do sistema em zero digital + outra saída com pico por
   10 s → nome dessa saída) e `ZeroWatch` (zero digital num canal por 3 s). Lógica pura, sobre a
   duração gravada.
3. `apps/desktop`: `meeting_output_device` no `Settings`; `start` resolve a saída lembrada (ou a
   padrão); o gravador manda os picos a cada relatório e aceita `SwitchSystem` (abre a nova antes
   de soltar a antiga; o WAV fica com zeros no intervalo); `MeetingStatus` ganha `system_output`,
   `call_output`, `zero_mic`, `zero_system`; comandos `meeting_outputs`, `set_meeting_output`,
   `switch_meeting_output`.
4. UI: seletor "Som da reunião" no `RecorderPanel` parado; ao vivo, "Som de <saída>" e o aviso com
   [Gravar deste dispositivo]; na pill, o mesmo aviso num cartão sobre o indicador (a janela toma o
   tamanho de um aviso enquanto ele aparece) e o ponto âmbar com zero digital.

## Doors (decididas pela recomendação da definição)

1. A troca pelo botão vale só para a sessão; o seletor do painel é o que fica lembrado.
2. Saída lembrada ausente no início (fone desplugado): grava a padrão e loga, sem erro.
3. Limiares: zero digital = pico ≤ 1e-6 (o do Granola); saída "tocando" = pico > 1e-6; uma pausa
   de fala não zera a contagem da outra saída, 10 s de silêncio nela zeram.
4. No Linux a lista traz só a padrão e não há medidor (ADR-0013: troca a quente no PipeWire pede
   ADR nova); o aviso nunca acende lá.

## Fora

Loopback por processo, seguir o Teams sozinho, todas as saídas ao mesmo tempo, AEC, Linux, o aviso
de mic diferente do Wispr e o tom de 20 Hz do Granola.

## ADRs

ADR-0007 (enumeração e pico em `crates/audio`, `cfg(windows)` só ali), ADR-0005 (o indicador
continua embaixo do cartão), ADR-0013 (Linux fora). Nenhuma contrariada.
