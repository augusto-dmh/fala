---
status: proposed
date: 2026-10-02
---
# Captura do sistema no Linux pelo plugin ALSA do PipeWire, não por pipewire-rs

Diverge do design doc §3.2, que diz "Linux via `pipewire-rs` no monitor do sink".

## Contexto e problema
A gravação de reunião precisa do áudio do sistema como um segundo canal. No Linux (GNOME, PipeWire)
o design doc congelado previa o crate `pipewire-rs` lendo o monitor do sink. O spike 04
(`docs/spikes/04-captura-dupla.md`) provou outro caminho: o `cpal` abre o dispositivo ALSA
`pipewire` com `PIPEWIRE_NODE=<node.name do sink>` e `PIPEWIRE_ALSA='{ stream.capture.sink = true }'`
definidas só durante a abertura, e gravou três rodadas de 60 min sem frame descartado nem erro de
stream. A trilha F leva esse caminho para `crates/audio` (`SystemAudio`) na rodada de 2026-10-02,
e o mesmo código abre o loopback do WASAPI no Windows, separado pelo nome do host do `cpal` em
tempo de execução, sem `cfg(target_os)` (ADR-0007).

O `pipewire-rs` exige `libpipewire-0.3-dev`, `libspa-0.2-dev` e bindgen/libclang no CI e mais
memória de build, numa máquina de desenvolvimento que já reiniciou por falta de RAM.

## Opções consideradas
* `cpal` + plugin ALSA do PipeWire, com as variáveis de ambiente definidas só durante a abertura.
* `pipewire-rs` nativo.
* O caminho ALSA para capturar, mais `pw-dump` em subprocesso para enumerar dispositivos e
  detectar reunião.

## Decisão
O caminho ALSA para capturar, com `pw-dump` em subprocesso para enumerar e detectar quando isso
for planejado (o seletor de dispositivos e a detecção de reunião não estão nesta rodada). É o
mecanismo medido, compartilha o código com o Windows e não traz dependência de sistema nova.

### Consequências
* Bom: um só caminho de captura (`cpal`) nos dois sistemas; nenhum pacote `-dev` nem bindgen novo
  no CI.
* Ruim: `PIPEWIRE_NODE` e `PIPEWIRE_ALSA` valem para o processo inteiro, então abrir dois streams
  com alvos diferentes ao mesmo tempo é uma corrida; o `cpal` enumera só `default`, `pipewire` e
  os nós ALSA crus, sem os nomes do PipeWire; a troca de sink a quente (fone plugado no meio da
  reunião) não segue o novo padrão de forma previsível.
* Obrigatório: a abertura de streams que mexe nessas variáveis é serializada por um `Mutex` dentro
  de `crates/audio`. O `pipewire-rs` só volta por ADR nova, se a troca de dispositivo a quente
  virar requisito.

## Confirmação
`crates/audio` não depende de `pipewire`/`libspa`; `SystemAudio::open` serializa a abertura; a
gravação de reunião da trilha F roda no PipeWire desta máquina pelo teste `#[ignore]` com
`FALA_TEST_SINK`.
