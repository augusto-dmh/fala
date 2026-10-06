---
status: proposed
date: 2026-10-02
---
# Destinos BYOK para notas de reunião, com transcript desligado por padrão

## Contexto e problema
As notas da fase 2 ficam no Fala e na pasta do usuário. Levar um resumo ao Slack do time ou a um database do Notion hoje é copiar e colar. O Granola resolve com apps OAuth e servidor próprio, e quase todas as integrações dele mandam só o resumo: entre integrações e exportações, o transcript sai apenas pelo Zapier, pela API, pelo MCP pago e pelo CSV. No Fala não há backend (ADR-0008), então cada destino tem de usar uma credencial do próprio usuário, e todo transcript carrega fala de terceiros (ADR-0005).

Um webhook genérico de saída cobre Zapier, Make, n8n e o Incoming Webhook do Slack de uma vez; o Notion aceita token de integração interna; o Slack aceita Incoming Webhook ou o token de um bot que o próprio usuário criou. Um app OAuth distribuído (app do Slack, app do Zapier) exigiria `client_secret` no binário ou um backend. Cada destino é uma saída de dados nova, fora das "três chamadas de rede opcionais" do `ARCHITECTURE.md`.

Evidência: `fala-research/research/14-granola-mcp-e-integracoes.md` §3.3, §3.4, §6.5 e §6.7; `fala-research/research/15-granola-funcionalidades-2026.md` §5.1; `fala-research/pitches/fase-4-destinos-byok.md`; `fala-research/plans/roadmap-proposta-2026-10-02.md` §1 (itens 53 e 59-69) e §3 (candidata 0011). A pasta `fala-research` não é versionada neste repo.

## Opções consideradas
* Nenhum destino: só "Copiar Markdown", um futuro export local e o espelho numa pasta sincronizada.
* Módulo de destinos com credencial do usuário (webhook de saída, Notion por token interno, Slack por Incoming Webhook ou bot próprio), transcript desligado por padrão em cada destino, disparo manual ou por regra, prévia no primeiro envio e log local de envios.
* Apps OAuth distribuídos (app do Slack, app do Zapier) ou webhook de entrada.

## Decisão
Módulo de destinos com credencial do usuário e transcript desligado por padrão. Entrega o essencial das integrações do Granola sem backend e sem segredo no binário; o app OAuth distribuído fere a ADR-0008, e "nenhum destino" deixa o usuário copiando e colando.

As regras do envio são estas:
* **Credencial:** token, URL de webhook (a URL é segredo: quem a tem consegue postar) e qualquer segredo do destino ficam no keyring; `Settings` guarda só a referência (ADR-0008).
* **Payload enumerado:** título, início, duração, nomes dos participantes, título e horário do evento do calendário, anotações do usuário, notas geradas, action items e, só com o interruptor do destino ligado, o transcript com timestamps. Cada destino formata essa lista no próprio formato (JSON do webhook, blocos do Notion, mrkdwn do Slack), sem acrescentar campo. Nunca áudio, nunca e-mail de participante, nunca caminho de disco: diferente do pitch e do relatório 14, o payload não leva `md_path`, porque o caminho revela o usuário e a estrutura de pastas e não serve a um destino remoto.
* **Transcript desligado por padrão** em cada destino; ligá-lo é um ajuste por destino, e o primeiro envio com transcript avisa que ele contém dados pessoais de terceiros.
* **Disparo:** manual, por nota, ou por uma regra de tag que o usuário configurou. No primeiro envio a cada destino, a UI mostra a prévia exata do payload antes de enviar.
* **Sessão "só local" nunca sai por destino**, nem manual nem por regra; para enviar, o usuário tira a marca da sessão antes. O pitch proibia só o envio automático; a extensão ao manual segue a ADR-0016 (proposta), que dá a "só local" o sentido de "nada desta sessão sai".
* **Log local de envios:** cada envio vira uma linha com o nome que o usuário deu ao destino (nunca a URL nem o token), o instante, os campos enviados e um id do envio, nunca o conteúdo. É o que responde "o que saiu da máquina e para onde".
* **Hook local:** um comando configurado pelo usuário pode rodar depois de "nota gerada", recebendo o caminho do `.md`. Ele não roda em sessão "só local", entra no mesmo log, e a UI diz que o que o comando faz com o arquivo (que contém o transcript) está fora do controle do Fala.
* Um destino novo (Linear, GitHub) cabe nesta ADR se seguir as mesmas regras e usar só credencial do usuário; OAuth com `client_secret` distribuído exige ADR nova.

### Consequências
* Bom: Slack, Notion e qualquer automação (Zapier, Make, n8n) a partir de um módulo só; o destino é escolhido pelo usuário, não por um database fixo; o log dá ao usuário a auditoria que o Granola só oferece no Enterprise.
* Ruim: cada destino tem limites e formato próprios a manter; envio offline pede fila local que não perca nem duplique; configurar um bot próprio do Slack dá mais trabalho que um clique de OAuth.
* Obrigatório: o payload de destino é um tipo próprio, montado só com campos de texto, que não aceita nenhum tipo de áudio (de ditado ou de reunião); nenhum app OAuth com `client_secret` no repo ou no binário; nenhum webhook de entrada; destinos só para notas de reunião (ditados não saem por destino sem ADR nova); o cliente HTTP dos destinos não entra na árvore de dependências de `fala-storage` nem do servidor MCP da ADR-0010 (proposta).
* Quando esta ADR for aceita, o `ARCHITECTURE.md` troca "As chamadas de rede opcionais são três" por uma lista de saídas de dados opcionais (se a ADR-0010 ainda não o tiver feito) e inclui "envio de notas de reunião a destino configurado pelo usuário, sem áudio e com transcript desligado por padrão"; os invariantes ganham "nenhum áudio em payload de destino". Se o módulo virar crate próprio, o Code Map muda no commit que o criar.

## Confirmação
* Teste de integração com servidor falso, por destino: os campos do corpo recebido são um subconjunto da lista enumerada; um destino recém-criado não envia transcript; com o interruptor ligado, o transcript chega; nenhum valor contém bytes de áudio, o caminho da pasta de dados ou um e-mail dos participantes da fixture.
* Teste de compilação: o tipo do payload de destino não aceita os tipos de áudio.
* Teste: o envio de uma sessão "só local" falha antes da rede, por disparo manual e por regra de tag, e o servidor falso não recebe conexão; o hook não roda para essa sessão.
* Teste: cada envio grava uma linha no log; um texto sentinela da nota, a URL e o token sentinela do destino não aparecem no log.
* O scanner de segredos do CI (ADR-0008) cobre o repo; `Settings` não tem campo de token nem de URL de webhook, só referência ao keyring. A prévia do primeiro envio e o aviso de terceiros são conferidos na revisão da fase.
