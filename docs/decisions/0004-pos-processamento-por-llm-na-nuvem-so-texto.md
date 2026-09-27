---
status: accepted
date: 2026-09-26
---
# Pós-processamento do ditado por LLM na nuvem (Gemini 2.5 Flash-Lite), recebendo só texto, automático e desligável por app

## Contexto e problema
O que faz o Wispr Flow entregar "texto pronto" é um LLM que remove fillers, entende "na verdade…", pontua, faz listas e aplica o dicionário. Em CPU de notebook, qualquer LLM local útil leva 10-15 s por frase. Na nuvem, Gemini 2.5 Flash-Lite responde em ~0,5 s e Claude Haiku 4.5 em ~1,2 s. O usuário aceita que texto (não áudio) saia da máquina.

## Opções consideradas
* Automático com Gemini 2.5 Flash-Lite, lista de apps onde fica desligado, "desfazer edição da IA" por item.
* Automático com Claude Haiku 4.5.
* Só sob hotkey "formatar".
* Nunca; só regras locais.

## Decisão
Gemini Flash-Lite automático para textos acima de ~15 palavras, com o dicionário pessoal e o nome do app no prompt (cacheado), desligado em apps sensíveis, e desfazer por item no histórico. Custo estimado abaixo de US$ 1/mês.

### Consequências
* Bom: resultado comparável ao Wispr; latência total dentro do orçamento (≤ 1,2 s).
* Ruim: texto ditado sai da máquina quando ligado; depende de rede (fallback: regras locais); um prompt em pt-BR a manter.
* Obrigatório: nunca enviar o conteúdo do campo ativo ou tela, só o texto ditado, o nome do app e o dicionário; se a resposta passar de 2 s, inserir o texto bruto e oferecer "aplicar edição".

## Confirmação
Teste de integração com servidor falso verifica o payload enviado (texto, app, dicionário; nada mais) e o comportamento de timeout.
