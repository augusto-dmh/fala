---
status: proposed
date: 2026-10-14
---
# Reunião: prévia local ao vivo durante a gravação; a transcrição de registro continua em batch

## Contexto e problema
A ADR-0005 decidiu gravar mic e sistema e transcrever em batch pela Scribe v2 ao parar, com o Parakeet local como fallback quando offline ou em sessão "só local"; uma das consequências registradas foi "transcrição só ao fim (sem ao vivo na v1)". Na prática, a tela da reunião fica vazia durante toda a call, e ao parar o toast diz "Nada foi transcrito ainda". O Granola e o Wispr mostram o texto enquanto se fala; o Wispr ainda faz um segundo passe melhor no servidor depois da reunião (`fala-research/research/24` § 5, "refined transcript"). O Augusto resume a lacuna como "não funciona como o Granola".

O ditado já carrega um modelo local rápido: o Parakeet pelo transcribe-cpp com Vulkan transcreve a ~24× o tempo real no Alienware. O Silero v4 (ADR-0009) já segmenta o ditado por voz. Uma prévia local, portanto, não manda nada novo para fora da máquina e não carrega um modelo a mais. A decisão é necessária agora porque o texto ao vivo muda o que o usuário vê durante e logo depois da reunião, e porque divide o único modelo com o ditado (ADR-0015).

Evidência: `fala-research/plans/roadmap-proposta-2026-10-02.md` § 3 (candidata 0019), `fala-research/pitches/fase-4-parciais-ao-vivo.md`, `fala-research/research/24` § 5, `fala-research/plans/status/brief-noite-2026-10-14.md` (as reuniões que terminaram com "Nada foi transcrito ainda"). A pasta `fala-research` não é versionada neste repo.

## Opções consideradas
* (a) Nada ao vivo, como hoje.
* (b) Prévia local: o modelo do ditado transcreve, durante a gravação, os trechos de fala que o VAD fecha em cada canal; a Scribe continua sendo a transcrição de registro e substitui a prévia quando chega.
* (c) Ao vivo pela nuvem (Scribe Realtime ou outro provedor de streaming), o que substituiria a ADR-0005 e mandaria áudio para fora durante a call.
* (d) Um motor de streaming local próprio (Nemotron) carregado só para a reunião.

## Decisão
(b), complementando a ADR-0005 sem substituí-la: é a única opção que dá texto durante a call sem mandar nada novo para fora e sem um segundo modelo na memória. (a) é a queixa; (c) troca a decisão de privacidade e custo da 0005; (d) dobra a memória e o trabalho, sem medição de que a janela por VAD não basta.

As regras são estas:
* **Fonte:** a prévia lê o áudio no mesmo ponto em que ele vai ao WAV de trabalho, já alinhado e com o canal do mic zerado nos intervalos de ditado (ADR-0015). A voz de um ditado nunca aparece na prévia.
* **Segmentação:** o Silero v4 fecha trechos por canal, com as mesmas regras de início e de silêncio do ditado e um teto menor, para o texto não atrasar numa fala longa. Mic é "Você" (no modo presencial, a sala); sistema é "Eles". Não há diarização ao vivo.
* **Modelo:** o modelo carregado para o ditado, sempre o mesmo, sem segunda instância. Nenhum caminho da prévia abre conexão.
* **Prioridade do ditado:** enquanto um ditado está em curso (da tecla até o fim da transcrição), a prévia não usa o modelo e os trechos esperam na fila. Um ditado nunca falha por causa da prévia; no pior caso espera um trecho curto que já estava sendo transcrito.
* **Registro:** a prévia nunca vira transcrição de registro. A de registro continua a da 0005: em batch ao parar, pela Scribe ou pelo fallback local sobre o áudio retido inteiro. Quando ela chega, substitui a prévia. Se ela falhar de vez, a prévia continua visível, rotulada como prévia, até o app fechar.
* **Persistência:** a prévia fica só em memória, durante a gravação e até a transcrição de registro chegar. O áudio retido continua sendo a fonte de verdade: se o app cair, a recuperação transcreve o áudio como hoje.
* **Sessão "só local":** a prévia funciona igual, porque não sai da máquina.

### Consequências
* Bom: texto durante a call; ao parar, a reunião já tem algo para ler enquanto a transcrição de registro trabalha; nada novo sai da máquina.
* Ruim: GPU ou CPU ocupada durante a call (no Alienware, ~4 % do tempo real por canal com fala; na CPU do notebook Linux, ~10 %); a qualidade da prévia é a do Parakeet por trechos curtos (WER pt ~6 % contra ~3 % da Scribe); a prévia some com o app, porque não é gravada; o modelo do ditado fica carregado enquanto a reunião grava, mesmo com "descarregar imediatamente".
* Obrigatório: a prévia nunca lê o áudio antes de o canal do mic ser zerado; o ditado nunca recebe erro por o modelo estar emprestado à prévia; a UI marca a prévia como prévia até a Scribe substituí-la.
* Quando esta ADR for aceita, a consequência "transcrição só ao fim (sem ao vivo na v1)" da 0005 passa a ler-se como "transcrição de registro só ao fim; prévia local durante (ADR-0019)". A 0005 não é editada.

## Confirmação
* Teste no `fala-audio`: a prévia recebe exatamente os frames que o WAV recebe, com zeros no intervalo de um ditado.
* Teste do segmentador: trechos com início e fim certos em ms, corte no teto, nada em silêncio digital.
* Teste no desktop: a política de prioridade não deixa a prévia usar o modelo com o ditado ocupado; um trecho adiado é transcrito depois, na ordem.
* Teste: a prévia nunca é escrita no `fala.sqlite` nem no espelho `.md`; quando a transcrição de registro da sessão é gravada, a prévia dela é descartada.
