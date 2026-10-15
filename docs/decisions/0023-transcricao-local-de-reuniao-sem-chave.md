---
status: proposed
date: 2026-10-15
---
# Reunião sem chave da ElevenLabs é transcrita localmente pelo Parakeet

## Contexto e problema
A ADR-0005 manda a reunião para a Scribe v2 e põe o Parakeet local como fallback "quando offline ou quando o usuário marcar a sessão como só local". Ela não fala de quem nunca guardou a chave da ElevenLabs. Hoje essa pessoa grava, para, e a reunião fica sem transcrição e sem notas: o botão "Transcrever" fica desligado com "falta a chave" (QA Q28, `research/26`). O Parakeet já está carregado para o ditado, e a crate `asr` já tem `LocalMeeting` e `transcribe_with_fallback`.

O pedido do usuário é que a reunião funcione como no Granola, que transcreve sempre. Sem chave, nada sai da máquina, então o caminho local só melhora a privacidade em relação à nuvem.

## Opções consideradas
* Transcrever localmente quando não há chave, marcar a transcrição como local e oferecer "Transcrever na nuvem" quando a chave existir.
* Manter como está: sem chave, a transcrição fica para um clique depois de guardar a chave.
* Pedir a chave no fim da reunião, num diálogo, antes de qualquer transcrição.

## Decisão
Sem a chave da ElevenLabs no keyring, a transcrição automática da reunião roda pelo Parakeet local, sobre o áudio retido. Fica marcada como local, e a reunião oferece "Transcrever na nuvem" quando a chave existir. Motivo: a reunião nunca fica sem transcrição, e nenhum dado sai da máquina.

Esta ADR complementa a 0005 sem substituí-la: a nuvem continua sendo o caminho padrão quando há chave, e o "só local" continua valendo.

### Consequências
* Bom: toda reunião tem transcrição, e as notas podem seguir dela. Sem chave, nem o áudio nem o texto saem da máquina.
* Ruim: a qualidade local é menor (Parakeet: 6,2 % WER em pt, contra 2,8 % da Scribe) e não há diarização: o canal do mic é "eu" e o do sistema é uma pessoa só. O CPU fica ocupado por alguns minutos depois de uma reunião longa.
* Obrigatório: a transcrição local é marcada como local no registro e na UI. "Transcrever na nuvem" só aparece com a chave e só roda por clique, porque substitui a transcrição local e renumera os falantes.

## Confirmação
Teste: sem chave no keyring, a transcrição automática passa pelo caminho local e não abre conexão (`transcribe_with_fallback` com `MeetingRoute::LocalOnly` contra um servidor falso sem conexões). A transcrição gravada tem a origem `local`, e a UI mostra o rótulo e a oferta de nuvem só com a chave.
