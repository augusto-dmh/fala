---
status: proposed
date: 2026-10-10
---
# A UI abre no conteúdo, usa cor só semântica e tira os tokens de uma fonte única

## Contexto e problema
O Fala herdou do Handy uma janela que abre nas configurações, uma paleta rosa usada como "cor da
marca" e cores espalhadas em literais pelos componentes. A pilha de UI da fase 1 trocou isso: os
tokens "Tinta" em `src/styles/theme.css` (neutros puros, acento = tinta, vermelho só para
gravação), as linhas de configuração no padrão do Windows 11, um rail que abre em Início (os
ditados) e o onboarding em três passos.

Essa direção afeta toda tela futura, inclusive as de reunião da fase 2, e é fácil de desfazer
sem querer: um agente que volta a usar `logo-primary` como "a cor da marca", escreve um hex num
componente ou põe Configurações de volta como a tela inicial não quebra nenhum teste de
comportamento. Os valores exatos (hex, rampa, raios, larguras) vão mudar com o uso; as regras
que os organizam não deveriam.

## Opções consideradas
* Registrar só as regras numa ADR e os valores num documento de design que muda sem ADR nova.
* Registrar regras e valores na mesma ADR.
* Não registrar; confiar na revisão do diff.

## Decisão
Uma ADR com três regras, e os valores em `docs/design/ui.md`, porque as regras são o que um
agente desfaz sem perceber e os valores são o que precisa mudar sem cerimônia.

1. **O app abre no conteúdo, não em configurações.** A janela principal abre em Início (os
   ditados; na fase 2, também as reuniões). Configurações é um destino do rail, nunca a tela
   inicial, e o primeiro uso termina em Início.
2. **Cor só semântica, sem cor de marca.** Vermelho = gravando (`rec`), âmbar = atenção (`warn`),
   verde = sucesso e canal "eu" (`ok`/`me`), cinza = canal "eles" e texto da IA (`them`,
   `text-2`), vermelho de erro = apagar e falha (`danger`). O acento é a tinta (`accent`). Nenhum
   token decorativo; a identidade vem da pill e da tinta.
3. **Os tokens vêm de `src/styles/theme.css`, fonte única.** Componentes usam `--color-<token>`
   (ou a classe Tailwind correspondente), nunca hex nem as cores nomeadas do Tailwind; os nomes
   herdados (`logo-primary`, `background-ui`, `mid-gray` etc.) são aliases em extinção e não
   entram em código novo. Os valores de cada token, a rampa tipográfica, os raios e o layout da
   janela ficam em `docs/design/ui.md`, que muda no mesmo commit que o `theme.css`.

### Consequências
* Bom: uma tela nova herda o visual sem decisões de cor; trocar um valor é editar o `theme.css`
  e o `ui.md`, sem ADR; o vermelho de gravação nunca disputa com um acento.
* Ruim: sem cor para ajudar, a hierarquia depende de tipografia e espaço, e o risco é o "cinza
  sobre cinza"; se o dono quiser uma cor de marca (por exemplo, só no wordmark), precisa de uma
  ADR nova que substitua esta.
* Obrigatório: componente novo não cita hex nem os aliases herdados; o `ui.md` acompanha o
  `theme.css`.

## Confirmação
`bun src/styles/tokens.test.ts` falha quando a tabela de cor do `docs/design/ui.md` diverge do
`theme.css`, quando um hex rosa herdado reaparece em `src/` e quando o contraste de `text` ou
`text-2` cai abaixo de 4,5:1. `bun src/components/shell.test.tsx` falha se o app deixar de abrir
em Início. Na revisão por fase, um diff que cite `logo-primary` ou um hex em componente novo é
devolvido.
