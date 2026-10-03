---
status: proposed
date: 2026-10-02
---
# Pós-processamento por LLM, só texto; a seleção sai só por ação explícita, com transparência por item

Substitui a ADR-0004 por inteiro: reafirma a decisão dela sem mudança e acrescenta uma exceção. Ao aceitar esta ADR, a única edição na 0004 é `status: superseded by ADR-0012`.

## Contexto e problema
O que faz o Wispr Flow entregar "texto pronto" é um LLM que remove fillers, entende "na verdade…", pontua, faz listas e aplica o dicionário. Em CPU de notebook, qualquer LLM local útil leva 10-15 s por frase. Na nuvem, Gemini 2.5 Flash-Lite responde em ~0,5 s e Claude Haiku 4.5 em ~1,2 s. O usuário aceita que texto (não áudio) saia da máquina. A ADR-0004 decidiu isso e proibiu enviar o conteúdo do campo ativo ou da tela.

Os três recursos de IA mais visíveis do Wispr batem nessa proibição: Transforms (prompts nomeados presos a atalhos sobre o texto selecionado), Command Mode (uma instrução falada sobre a seleção) e context awareness (ler campo, conversa e tela para acertar nomes e tom). A brief proíbe context awareness "que leia a tela ou o campo ativo sem toggle explícito e sem mostrar o que foi lido": o no-go é sobre ler, não só sobre enviar. O Wispr lê por padrão e não mostra o que leu, e a privacidade é o tema nº 1 das críticas a ele. Em Transforms e Command Mode, selecionar e apertar o atalho já é o gesto de enviar; no context awareness não há gesto, e a leitura acontece em todo ditado.

Evidência: `fala-research/research/03-modelos-asr-e-llm.md` §6 (latências, já citado pela ADR-0004); `fala-research/research/16-wispr-flow-notas-e-lacunas.md` §2.1, §2.2, §2.5, §2.6, §3.2, §4.1 e §4.3; `fala-research/research/01-referencia-wispr-flow.md` §2.3 e §2.4; `fala-research/pitches/00-brief.md` (no-gos); `fala-research/pitches/fase-4-contexto-e-comandos.md`; `fala-research/plans/roadmap-proposta-2026-10-02.md` §1 (itens 84-87) e §3 (candidata 0012). A pasta `fala-research` não é versionada neste repo.

## Opções consideradas
Para o ditado, as opções da ADR-0004, que continuam valendo:
* Automático com Gemini 2.5 Flash-Lite, lista de apps onde fica desligado, "desfazer edição da IA" por item.
* Automático com Claude Haiku 4.5.
* Só sob hotkey "formatar".
* Nunca; só regras locais.

Para o conteúdo além do texto ditado:
* Manter a proibição como está e não fazer Transforms, Command Mode nem context awareness.
* Só a variante local do contexto: o que for lido do campo vira dica para a correção local e nunca vai ao LLM.
* Exceção por ação explícita: selecionar e apertar o atalho envia a seleção (Transforms, Command Mode), com o que saiu guardado e visível no item; o contexto lido sem gesto continua só local e atrás de toggle.
* O mesmo, mais context awareness enviado ao LLM atrás de um toggle desligado por padrão, com recorte mínimo e painel "o que foi lido".
* Context awareness automático e ligado por padrão, como no Wispr.

## Decisão
Para o ditado, a ADR-0004 sem mudança: Gemini Flash-Lite automático para textos acima de ~15 palavras, com o dicionário pessoal e o nome do app no prompt (cacheado), desligado em apps sensíveis, e desfazer por item no histórico. Custo estimado abaixo de US$ 1/mês. Haiku 4.5 perde por latência (1,2 s); "só sob hotkey" perde porque o texto pronto tem de ser automático; "nunca" perde "na verdade…", listas e tom. O Transform abaixo é adicional, não substitui o automático.

Para o conteúdo além do texto ditado, a exceção por ação explícita: **só a seleção vai ao LLM, e só quando o usuário seleciona um texto e aperta o atalho de Transform ou de Command Mode; o resto do campo e a tela nunca vão.** Cada item assim guarda e mostra exatamente o que saiu. O gesto é o consentimento, por item, e o usuário vê o que enviou. Manter a proibição inteira deixa de fora os recursos que a brief admite com transparência; a variante só local não cobre Transforms nem Command Mode; o automático ligado por padrão contraria a brief.

O context awareness sem gesto fica **só local**: atrás de um toggle desligado por padrão, o que for lido do campo vira dica para a correção local, nada sai, e o item mostra o que foi lido. Enviá-lo ao LLM, mesmo atrás de toggle, fica para uma ADR nova com a medição da variante local: é uma categoria de dado nova saindo a cada ditado, e esta ADR não a pré-autoriza sem número.

### Consequências
* Bom: no ditado, resultado comparável ao Wispr, com latência total dentro do orçamento (≤ 1,2 s); Transforms e Command Mode em pt-BR desde o início; o painel "o que foi enviado" é um diferencial que o Wispr não tem.
* Ruim: texto ditado sai da máquina quando o LLM está ligado; a seleção sai quando o usuário a envia; depende de rede (fallback: regras locais); um prompt em pt-BR a manter; ler a seleção depende de UI Automation no Windows, que falha em parte dos apps Electron/Chromium, e no GNOME não há caminho sem extensão; se a variante local não bastar, o context awareness enviado pede mais uma substituição desta ADR.
* Obrigatório, herdado da ADR-0004: no ditado, nunca enviar o conteúdo do campo ativo ou da tela, só o texto ditado, o nome do app e o dicionário; se a resposta passar de 2 s, inserir o texto bruto e oferecer "aplicar edição".
* Obrigatório, da exceção:
  * mesmo provedor e mesma chave do ditado (ADR-0008); o modelo é configuração;
  * o payload de um Transform é a seleção, o prompt nomeado, o nome do app e o dicionário; o de um Command Mode é a seleção, o texto da instrução falada, o nome do app e o dicionário; nada além disso; o áudio da instrução é áudio de ditado e nunca sai (ADR-0003);
  * o valor que carrega a seleção só nasce de um `UserAction` de Transform ou de Command Mode; sem seleção, nada é enviado; nunca de campo de senha, nem de app da lista "LLM desligado";
  * o texto enviado fica guardado só no próprio item do histórico, para o painel "o que foi enviado": não entra no espelho `.md`, não é indexado pela busca e não sai pelo MCP; apagar o item o apaga;
  * o resultado só substitui a seleção depois de conferir que a janela-alvo é a mesma, com desfazer no histórico; se a resposta não vier em 10 s ou o usuário cancelar, a seleção fica como estava;
  * um termo que o usuário aceitou explicitamente para o dicionário passa a ser dicionário, não conteúdo do campo;
  * um Transform aplicado automaticamente depois do ditado é o caminho do ditado e só carrega texto ditado.
* Fora desta ADR: screenshots, dicionário alimentado por texto da tela sem aceite por termo e busca na web por voz.
* Quando esta ADR for aceita:
  * o status da ADR-0004 passa a `superseded by ADR-0012`;
  * na lista de chamadas ou saídas opcionais do `ARCHITECTURE.md`, "texto do ditado para o LLM" vira "texto do ditado e, por ação explícita, a seleção, para o LLM";
  * o `AGENTS.md` passa a citar a ADR-0012 no lugar da 0004 no invariante "só texto vai ao LLM" e a dizer "nunca edite uma ADR aceita, exceto o `status` quando outra a substitui".

## Confirmação
* Teste de integração com servidor falso verifica o payload do ditado (texto, app, dicionário; nada mais) e o comportamento de timeout, como na ADR-0004; com o context awareness local ligado e um texto sentinela no campo, o sentinela não chega ao servidor.
* O mesmo servidor falso verifica o payload de Transform e de Command Mode (só os campos listados) e que nada chega sem seleção, de campo de senha ou de app da lista "LLM desligado"; sem resposta em 10 s, a seleção fica intacta.
* Teste de compilação (`compile_fail`): o tipo da seleção não se constrói sem o `UserAction` correspondente, e o cliente do LLM do ditado não o aceita.
* Teste: o texto do painel "o que foi enviado" é igual, byte a byte, ao que o servidor falso recebeu, não aparece no `.md` nem na busca, e some quando o item é apagado.
* Teste: se a janela-alvo mudou antes da resposta, a seleção não é substituída.
