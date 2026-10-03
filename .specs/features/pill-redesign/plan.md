# pill-redesign — a pill da fase 1: cápsula preta, 10 barras, sem texto

## Problem

O overlay mínimo herdado (`src/overlay/RecordingOverlay.tsx`) é um cartão de 172-216 px com um
ponto, 9 barras, um botão X, um spinner e os rótulos "Transcrevendo..."/"Processando...". O pitch
da fase 1 fixa outra coisa: "cápsula preta ~84×30 px, 10 barras simétricas, vermelha em
push-to-talk, 'processando' com barras paradas e pulso, sem ícone, sem texto". A pill é o único
sinal visível do ditado (o app fica escondido), então a diferença não é cosmética: o cartão largo
com texto e botão disputa atenção com o campo onde o texto vai cair, e o pitch quer um sinal
que se lê com o canto do olho. O delta da fase 1 (`plans/fase-1-delta-e-semanas-3-4.md`, linha S4)
lista o redesenho como pendente. Não há número de incidente: o custo é de produto.

Quando isto fechar, os estados gravando, processando e oculto do overlay mínimo são a cápsula do
pitch, desenhada dentro da janela transparente que já existe, sem nenhuma linha de Rust.

## Flow

Usa os eventos que o desktop já emite (`show-overlay`, `hide-overlay`, `recording-ready`,
`mic-level`) e `commands.getAppSettings()`, que o overlay já chama. Nenhum evento novo.

1. `show-overlay` com `recording` | `transcribing` | `processing` -> listener em `src/overlay/RecordingOverlay.tsx` (exists) - além do que já faz, lê `shortcut_activation` das settings
2. `RecordingOverlay` (exists) -> `Pill` em `src/overlay/Pill.tsx` (new, no door - placement per conventions) - recebe `mode` (`recording` ou `processing`), `holdToTalk`, `ready`, os 16 níveis do `mic-level` e o rótulo acessível já traduzido
3. `pillBars(levels)` em `src/overlay/pillModel.ts` (new, no door - placement per conventions) - transforma os 16 níveis suavizados em 10 alturas espelhadas
4. `hide-overlay` -> `RecordingOverlay` devolve `null` (exists, sem mudança)
5. out: a cápsula renderizada dentro da janela `recording_overlay` de 256×50 que `apps/desktop/src/overlay.rs` cria (exists, intocada)

## Impact

| Front | What changes |
| --- | --- |
| domain | novo termo: `holdToTalk` - a pill é vermelha quando a ativação do atalho é `push_to_talk` ou `hold_or_toggle`; vive em `src/overlay/Pill.tsx` |
| domain | termo existente: os estados `transcribing` e `processing` do overlay mínimo, hoje dois rótulos, passam a ser um só visual "processando" |
| stored data | nothing to migrate - nenhuma setting nova, nenhum dado persistido |
| ui | o overlay "Ao vivo" (`state === "streaming"`) não muda; só o ramo mínimo troca de desenho |
| i18n | chave nova `overlay.recording` em `pt` e `en`, usada só como rótulo acessível (`aria-label`), nunca visível |

## Relations

None - no stored-data shape change

## Surface

None - nothing consumed outside: o overlay só escuta eventos que já existem; nenhum comando, evento ou binding novo.

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| None | - | - |

- Nothing else in this change is hard to reverse: é CSS e um componente React, sem dado, sem contrato
  e sem dependência nova; a janela nativa continua 256×50.

## Criteria

### S1: gravando (P1)

Enquanto o microfone grava, a pill é uma cápsula com 10 barras que reagem ao áudio.

**Acceptance Criteria**

1. WHEN o overlay recebe `show-overlay` com `recording` THEN the system SHALL desenhar uma cápsula de 84×30 px com `border-radius` de 15 px, sem texto visível, sem ícone e sem botão
2. WHILE grava, the system SHALL desenhar exatamente 10 barras numa única linha centralizada na cápsula
3. The system SHALL calcular as 10 alturas espelhadas (barra `i` igual à barra `9 - i`) a partir dos níveis 0-4 do `mic-level`, com o nível 0 nas duas barras centrais
4. The system SHALL limitar cada altura de barra entre 3 px (silêncio) e 18 px (nível 1,0), crescendo com o nível
5. WHILE grava com `shortcut_activation` `push_to_talk` ou `hold_or_toggle` the system SHALL pintar a cápsula de vermelho (`#e5322d`); com `toggle`, de preto (`#000`)
6. WHILE o `recording-ready` não chegou, the system SHALL desenhar as 10 barras na altura mínima com opacidade 0,45, sem reagir ao nível

**Independent test:** `bun src/overlay/Pill.test.tsx` renderiza a pill gravando com níveis conhecidos e confere barras, alturas, cor e ausência de texto.

### S2: processando (P1)

Depois de soltar a tecla, a pill fica parada e pulsa até o texto cair.

**Acceptance Criteria**

7. WHEN o overlay recebe `show-overlay` com `transcribing` ou `processing` THEN the system SHALL desenhar a mesma cápsula preta de 84×30 px com as 10 barras paradas numa forma fixa e espelhada, sem texto visível e sem spinner
8. WHILE processa, the system SHALL animar a cápsula com um pulso de opacidade (entre 1 e 0,55) num ciclo de 1,2 s, e as barras SHALL ignorar o `mic-level`
9. WHERE o sistema pede `prefers-reduced-motion: reduce` the system SHALL desligar o pulso e manter a cápsula opaca

**Independent test:** renderizar a pill processando com níveis aleatórios duas vezes dá o mesmo HTML.

### S3: oculto e acessível (P1)

A pill some ao fim e diz o que é a quem usa leitor de tela.

**Acceptance Criteria**

10. WHEN o overlay recebe `hide-overlay` THEN the system SHALL não renderizar a pill
11. The system SHALL expor a pill com `role="status"` e um `aria-label` traduzido: `overlay.recording` gravando, `overlay.processing` processando, em pt-BR e en
12. The system SHALL manter a pill preta e as barras brancas nos temas claro e escuro

**Independent test:** `bun run lint`, `bun run check:translations` e `bun run build` passam; o teste confere o `role` e o `aria-label`.

## Out of scope

| Excluded | Why |
| --- | --- |
| Estado "Colar" quando a inserção falha | o app não detecta colagem que falhou (o log diz "Text pasted successfully" mesmo quando o texto não entrou), então o estado não tem gatilho; é pergunta da trilha D/inject (auditoria da fase 0, linha E) |
| Mudar a janela nativa (tamanho 256×50, topmost, sem foco) | `apps/desktop/src/overlay.rs` fica intocado: a medição do spike 03 (200/200, p50 0 ms) vale para a janela atual |
| Overlay "Ao vivo" (streaming com texto) | é um estilo opcional para modelos com streaming; o pitch fala da pill do ditado comum. Fica como está |
| Botão de cancelar na pill | o pitch diz "sem ícone"; cancelar continua pelo Esc durante a gravação (W2 do delta) |
| Distinguir hold de tap no `hold_or_toggle` | o overlay não recebe evento quando um toque trava a gravação; precisaria de Rust. A pill fica vermelha desde o início nesse modo |
| Estado de aviso aos 19 min e erro visível na pill | dependem de eventos que ainda não existem (W4, S10 do delta) |
| Pill arrastável com posição memorizada | semanas 5-6 do pitch |

## Assumptions

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| Perfil | `ui`, com o esboço do pitch como fonte binding | recomendado pelo orquestrador; a pill é uma tela | y - delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| "vermelha em push-to-talk" | a cápsula inteira fica vermelha (o adjetivo concorda com "cápsula"); barras brancas | leitura literal do esboço | y - delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| Processando | sempre preta, mesmo em push-to-talk | a tecla já foi solta; o vermelho quer dizer "o mic está ouvindo" | y - delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| Fonte dos níveis | os 5 primeiros buckets dos 16 do `mic-level`, com a suavização exponencial que já existe | as frequências baixas têm a energia da voz; o espelho dá a simetria do esboço | y - delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| Como provar sem `tauri dev` | um script `node:assert` rodado por `bun`, como `clipboard.test.ts`, renderizando com `react-dom/server`; mais `bun run build` e lint | sem dependência nova (vitest e testing-library ficariam no `package.json`, fora da fronteira); `tauri dev` derruba a RAM | y - delegado pelo Augusto em 2026-10-02, decidido pelo painel |

**Open questions:** none - all resolved or logged above.

## Observable

| Surface | Decision | Landing |
| --- | --- | --- |
| screen `pill` gravando | arrangement (cápsula, uma linha de 10 barras, nada mais) | AC 1, AC 2 |
| screen `pill` gravando | copy | AC 1 (nenhum texto visível), AC 11 (só `aria-label`) |
| screen `pill` gravando | loading state | AC 6 (antes do `recording-ready`) |
| screen `pill` processando | arrangement e copy | AC 7 |
| screen `pill` processando | loading state | AC 8 - o próprio estado é a espera |
| screen `pill` | empty state | n/a - a pill só existe enquanto há ditado; fora dele está oculta (AC 10) |
| screen `pill` | error state | n/a - não há evento de erro para o overlay nesta rodada (Out of scope, S10 do delta) |
| screen `pill` | destructive action confirms | n/a - a pill não tem ação |
| screen `pill` | theme | AC 12 |
| screen `pill` | motion | AC 8, AC 9 |

## Sources

- `~/projects/fala-research/pitches/fase-1-ditado-windows.md` linha 19 - esboço da pill (binding): cápsula preta ~84×30, 10 barras simétricas, vermelha em push-to-talk, processando com barras paradas e pulso, sem ícone, sem texto
- `~/projects/fala-research/plans/fase-0-fechamento-auditoria.md` linha E - não tocar em `overlay.rs`; sem estado "Colar"
- `~/projects/fala-research/plans/fase-1-delta-e-semanas-3-4.md` S4 - o que existe hoje e o que muda
