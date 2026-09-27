# Roadmap do Fala

Fases decididas em 2026-09-26 (`fala-research/research/06-grilling-decisoes.md`). Cada fase tem apetite em semanas de horas vagas, critério de "fase fechada" e a bug bar que vale durante ela. Regra Shape Up: fase que estoura o apetite corta escopo, não estende. O pitch de cada fase vive em `fala-research/pitches/`.

| Fase | Apetite | Fechada quando | Pitch |
|---|---|---|---|
| **0 — Spikes e benchmark** | 2 semanas | 4 relatórios em `docs/spikes/`, tabela WER/RTF em `fala-research/benchmarks/`, ADR-0003 confirmada ou substituída | `fase-0-spikes.md` |
| **1 — Ditado no Windows** | 6 semanas | 2 semanas usando o Fala em vez do Wispr Flow, sem voltar; latência dentro do orçamento do `ARCHITECTURE.md`; um colega instalou sozinho; `docs/RELEASE.md` escrito e primeiro `.exe` distribuído | `fase-1-ditado-windows.md` |
| **2 — Reunião e vídeos no Windows** | 8 semanas | Granola desinstalado; 5 reuniões reais transcritas com "eu/eles" e notas enriquecidas; um vídeo importado por URL; áudio retido e recuperável | a escrever ao fechar a fase 1 |
| **3 — Linux GNOME Wayland** | 4 semanas | Ditado e reunião funcionando no notebook do trabalho via portais e PipeWire; `.deb` instalado sem passo manual além do grupo `input` se necessário | a escrever ao fechar a fase 2 |
| **4 — Polimento** | sem apetite fixo; itens entram um a um | cada item com pitch próprio: Nemotron streaming com parciais, context awareness com toggle e transparência, estilo por app, snippets, speaker tags no Meet, briefs, sync por pasta | por item |

## Bug bar por fase

O que precisa ser verdade para um bug ser corrigido *agora*; o resto vai para o backlog com data.

- **Fase 0:** só o que impede o spike de medir.
- **Até o vertical slice (fase 1, semanas 1-2):** só o que quebra o caminho principal: não grava, não transcreve, não insere, trava.
- **Fase 1, uso próprio:** anterior + qualquer perda de texto ditado + inserção no app errado + latência acima do máximo da tabela + qualquer envio de áudio para fora da máquina.
- **Antes de dar o instalador a um colega:** anterior + qualquer coisa que um tissue tester tenha travado + instalação e desinstalação limpas + nenhuma chave em log + "desfazer edição da IA" funcionando. Bug reportado por colega entra na barra automaticamente.
- **Fase 2:** anterior + gravação nunca começa sem clique + indicador visível durante toda a gravação + retenção configurável funcionando + áudio nunca perdido em crash.
- **Fase 3:** anterior + nada do Windows regride (CI roda nos dois).
- Início de cada fase: 2-3 dias de "milestone de qualidade" pagando o que ficou abaixo da barra na anterior.

## Marcos de qualidade (vocabulário emprestado dos estúdios)

- **Vertical slice:** um caminho completo em qualidade final (fase 1, semana 2).
- **Alpha:** todas as features da fase implementadas, polish incompleto.
- **Beta:** só correção de bugs e tuning; tissue test com 2-3 pessoas.
- **Release da fase:** bug bar zerada, pitch com retro preenchida, `CHANGELOG.md` atualizado.
