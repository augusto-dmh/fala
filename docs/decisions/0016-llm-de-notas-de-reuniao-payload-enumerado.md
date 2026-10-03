---
status: proposed
date: 2026-10-02
---
# LLM de notas de reunião: só texto, payload enumerado, nada em sessão "só local"

## Contexto e problema
O `ARCHITECTURE.md` lista "transcrição para o LLM de notas" entre as chamadas de rede opcionais, e o design doc §3.4 manda transcrição, anotações e evento do calendário ao Claude, mas nenhuma ADR rege esse canal: a ADR-0004 trata só do ditado, e a ADR-0005 trata só do áudio que vai ao ASR. A revisão do pitch da fase 2 aumentou o payload (perfil do usuário, idioma das notas, dicionário pessoal e nomes dos participantes para sugerir quem é "Pessoa N"). O payload é difícil de reverter: o que saiu para um provedor não volta.

A ADR-0005 define a sessão "só local" apenas para o ASR (Parakeet no lugar da Scribe) e não fala do LLM. Quem marca uma sessão como "só local" lê "nada desta sessão sai da máquina", e um LLM de notas que recebesse o transcript dessa sessão contrariaria essa leitura. Não há medição de LLM local gerando notas de uma hora de transcrição; a ADR-0004 registra 10-15 s por frase curta em CPU, o que indica minutos por reunião e qualidade em pt-BR não verificada.

Esta ADR complementa a ADR-0005 sem substituí-la: o fallback do ASR continua como está, e o que se acrescenta é que a sessão "só local" também não chama o LLM de notas.

Evidência: design doc §3.4 e §6; `fala-research/pitches/fase-2-reuniao-videos.md` (F6 e sua revisão, D10, D13, pergunta sobre "só local" e notas por LLM, custo no critério de fechamento); `fala-research/research/15-granola-funcionalidades-2026.md` §1.2, §1.3, §5.2 e §6.1; `fala-research/research/16-wispr-flow-notas-e-lacunas.md` §1.2, §1.5 e §4.2; `fala-research/plans/roadmap-proposta-2026-10-02.md` §3 (candidata 0016) e decisão 7. A pasta `fala-research` não é versionada neste repo.

## Opções consideradas
* Payload livre, definido só no plano da feature de notas.
* Payload enumerado e conferido por servidor falso, nunca áudio nem caminho; sessão "só local" não chama o LLM de notas.
* Payload enumerado, com a sessão "só local" ainda chamando o LLM de notas (a ADR-0005 só fixa o ASR e deixa o LLM em aberto).
* LLM local para notas.

## Decisão
Payload enumerado, só texto, pela chave do usuário (ADR-0008), e nenhuma chamada em sessão "só local". Num repo público, quem instala confere pela ADR e pelo teste o que sai sobre terceiros; o payload livre não dá essa garantia; deixar a sessão "só local" chamar o LLM contradiz o que o usuário entende ao marcá-la; e o LLM local fica fora até haver medição. O provedor segue o design doc (Claude) e é configuração, como o modelo: trocar de provedor não exige ADR nova, desde que o payload continue o desta lista.

O payload de uma geração de notas contém só:
* a transcrição da sessão com timestamps, id por segmento e rótulos de falante ("Eu", "Pessoa N" ou o nome que o usuário aplicou), incluindo os marcadores "trecho de ditado omitido" (ADR-0015, proposta);
* as anotações que o usuário digitou durante a sessão, com id por linha (para os ponteiros da nota gerada);
* o título, a data e a hora de início da sessão e, se houver, título e horário do evento do calendário;
* o template escolhido (instruções e seções);
* o perfil do usuário (campo de texto em `Settings`), o idioma das notas e o dicionário pessoal;
* os nomes dos participantes, para sugerir nomes de falantes; participante sem nome não entra, e nunca se deriva nome da parte local de um e-mail.

Nunca: áudio, caminho de disco, e-mail de participante (reforço desta ADR sobre o pitch, que pede só os nomes), descrição ou pauta do convite, conteúdo de outra sessão, a nota gerada anterior ao regenerar, chaves ou outros campos de `Settings`, e tools de servidor ou busca na web na requisição.

### Consequências
* Bom: notas em pt-BR mesmo em call mista, com o perfil e o dicionário do usuário; o usuário sabe exatamente o que sai; "só local" passa a ter um sentido só. Com as ADRs 0010 e 0011 (propostas), a sessão "só local" também não sai por MCP nem por destino.
* Ruim: transcrição com fala de terceiros vai ao provedor quando a sessão não é "só local"; sessão "só local" fica sem notas geradas (só transcrição e anotações); offline, as notas esperam a rede; ~US$ 1-3/mês estimado para ~22 h/mês; regenerar descarta a nota anterior, por isso o pitch pede confirmação antes de regenerar uma nota editada.
* Obrigatório: um único construtor monta o payload, num tipo cujos campos são os da lista; o aviso de dados de terceiros da ADR-0005 passa a nomear também o provedor de notas e aparece no primeiro envio de notas; marcar ou desmarcar "só local" depois da gravação vale para as gerações seguintes, e a UI diz que marcar depois de um envio não desfaz o que já saiu; os nomes sugeridos nunca são aplicados sem clique.
* O chat sobre reuniões (fase 4) não usa este payload: terá o seu, por ADR nova ou por uma ADR que substitua esta. Até lá, este canal serve só à geração de notas de uma sessão.
* Quando esta ADR for aceita, na lista de chamadas ou saídas opcionais do `ARCHITECTURE.md`, "transcrição para o LLM de notas" vira "transcrição, anotações e os campos enumerados na ADR-0016 para o LLM de notas, nunca em sessão só local". Se a 0010 e a 0011 também forem aceitas, vale um invariante único: "sessão 'só local' não sai da máquina por nenhum canal".

## Confirmação
* Teste de integração com servidor falso: as chaves do JSON enviado para uma sessão de fixture são iguais a uma lista fixa; o corpo não tem `tools`; nenhum valor contém o caminho da pasta de dados, bytes de áudio, um e-mail dos participantes da fixture, um sentinela posto num campo de `Settings` fora da lista ou um sentinela de outra sessão.
* Teste: gerar notas de uma sessão "só local" não abre conexão com o servidor falso e devolve só transcrição e anotações; depois de desmarcar "só local", a geração seguinte chega ao servidor.
* Teste: uma sugestão de nome devolvida pelo servidor falso não altera os rótulos da sessão até a ação de aplicar.
