# UI do Fala: tokens e layout

Os valores da direção visual "Tinta". A regra que eles seguem está na ADR-0017; este arquivo
muda sem ADR nova, no mesmo commit que muda `src/styles/theme.css`. O teste
`bun src/styles/tokens.test.ts` compara a tabela de cor abaixo com o `theme.css` e falha quando
os dois divergem.

Fonte: relatório de UI do `fala-research` (Parte 3, §3.4 e §3.7).

## Cor

Neutros puros; o acento é a própria tinta. As únicas cores são semânticas. Os tokens vivem em
`src/styles/theme.css` como `--light-color-<token>` e `--dark-color-<token>`, e as telas usam
`--color-<token>` (no Tailwind, `bg-surface-1`, `text-text-2` etc.).

| Token | Claro | Escuro | Uso |
| --- | --- | --- | --- |
| `surface-0` | `#f3f3f3` | `#202020` | fundo da janela (base do Mica no Windows 11) |
| `surface-1` | `#ffffff` | `#2b2b2b` | cartões, lista, notepad |
| `surface-2` | `#f7f7f7` | `#323232` | hover, item ativo, campo |
| `border` | `#e5e5e5` | `#3a3a3a` | divisores e contornos de cartão |
| `text` | `#1a1a1a` | `#f5f5f5` | texto principal, texto meu nas notas |
| `text-2` | `#5c5c5c` | `#c4c4c4` | descrições, metadados, texto da IA |
| `text-3` | `#8a8a8a` | `#8f8f8f` | placeholder, desabilitado (só texto não essencial) |
| `accent` | `#1a1a1a` | `#f5f5f5` | botão primário, toggle ligado, barra do item ativo (tinta) |
| `on-accent` | `#ffffff` | `#1a1a1a` | texto sobre `accent` |
| `warn` | `#b45309` | `#f5a524` | limite de 19 min, canal mudo, avisos |
| `ok` | `#1a7f4b` | `#4cc38a` | sucesso, canal "eu" |
| `them` | `#8a8a8a` | `#8f8f8f` | canal "eles" |
| `danger` | `#c42b1c` | `#ff99a4` | apagar, erro |
| `rec` | `#e5322d` | `#e5322d` | gravando (o vermelho da pill), ponto do tray |
| `me` | = `ok` | = `ok` | canal "eu" |

Foco: anel duplo do Fluent, 2 px em `text` por fora e 1 px em `surface-1` por dentro. Campos de
texto marcam o foco com o sublinhado de tinta.

Contraste (WCAG): `text` e `text-2` ficam acima de 4,5:1 sobre as três superfícies nos dois temas
(o teste de tokens verifica); `text-3` sobre branco dá 3,5:1 e por isso nunca carrega informação
essencial. Os nomes herdados do Handy (`logo-primary`, `background-ui`, `mid-gray`,
`background`, `warning`, `error`) são aliases dos tokens acima e não entram em código novo.

## Tipografia

- Família: `"Segoe UI Variable Text", "Segoe UI Variable", "Segoe UI", system-ui, sans-serif`;
  títulos em `"Segoe UI Variable Display"`. No Linux cai na sans do sistema. Nenhuma fonte
  embutida.
- Rampa do Windows 11, raiz em 14 px:

| Degrau | Tamanho/linha | Peso | Classe |
| --- | --- | --- | --- |
| legenda | 12/16 | 400 | `text-caption` (`text-xs`) |
| corpo | 14/20 | 400 | `text-body` (`text-sm`, `text-base`) |
| corpo forte | 14/20 | 600 | `text-body font-semibold` |
| corpo grande | 18/24 | 400 | `text-body-lg` (`text-lg`) |
| subtítulo | 20/28 | 600 | `text-subtitle` (`text-xl`) |
| título | 28/36 | 600 | `text-title` (`text-2xl`) |

- Notas e transcrição: 15/24 para leitura longa.
- Números: `tabular-nums` em horas, timers, contagens e porcentagens.
- Sentence case em tudo.

## Forma, espaço e movimento

- Raio: 4 px em controles, cartões e itens de lista (`rounded-sm`, `rounded-md`, `rounded-lg`);
  8 px em diálogos, menus, toasts e popovers (`rounded-xl`); pill totalmente arredondada.
- Espaçamento em base 4: 4, 8, 12, 16, 24, 32.
- Linha de configuração: 56 px de altura mínima sem descrição, 68 px com descrição. Linha de
  ditado: 44 px recolhida.
- Sombras só em overlays (menus, toasts, diálogos). Cartões sem sombra, separados por 4 px.
- Movimento: 80 a 120 ms `ease-out` para hover e troca de estado (padrão 100 ms); nenhuma
  animação de entrada em listas; `prefers-reduced-motion` desliga transições.
- Ícones `lucide-react` a 20 px com traço 1,5.

## Janela

- 960×640 por padrão, mínimo 720×520. Barra de título nativa.
- Rail à esquerda: 220 px, recolhido em 48 px de ícones abaixo de 840 px de largura. Wordmark
  "Fala" em tinta, 20 px semibold, com a pill em miniatura. Item ativo: barra vertical de 3 px
  em `accent` e fundo sutil, nunca preenchimento colorido. Configurações fica no rodapé do rail,
  com o estado do modelo.
- Conteúdo: cabeçalho com título 28/36 semibold e ações à direita; coluna de leitura de no
  máximo 720 px nas listas e 680 px nas notas; margens de 24 px (16 px na largura mínima).
- Superfícies: janela em `surface-0`, cartões em `surface-1`, sem bordas pesadas.
