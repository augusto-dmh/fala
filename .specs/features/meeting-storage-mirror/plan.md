# meeting-storage-mirror — `Reuniões/*.md`, `reindex` de reuniões e busca nas transcrições

Profile: light · fase 2 (F5) · ADR-0006 (cumpre) · ADR-0011 (proposta; vale no frontmatter)

## Problem

As reuniões ficam só no `fala.sqlite`: perder o banco perde transcrições e notas. A ADR-0006 pede
espelho Markdown escrito depois do SQLite e `reindex`. A página Reuniões não busca.

## Slices (PRs empilhados, pelo gate de 1000 linhas)

- **S1** `feat/meeting-storage-mirror`: `Reuniões/AAAA-MM-DD Título.md` escrito pelo `Store` depois
  de cada mudança da sessão; apagar a sessão apaga o `.md`; a abertura do desktop escreve os que
  faltam ou divergem.
- **S2** `feat/meeting-storage-mirror-2`: o `.md` lido de volta; `Store::reindex_meetings` e
  `fala-cli reindex` reconstroem as reuniões (round-trip da ADR-0006).
- **S3** `feat/meeting-storage-mirror-3`: `meetings_fts` (`unicode61 remove_diacritics 2`) sobre
  título, anotações, notas e trechos; busca na página Reuniões com trecho e link.

## O arquivo (porta de mão única)

```
---
id: "01K7AB3C4D5E6F7G8H9J0KMNPQ"
title: "Planejamento"
date: "2026-10-12"
mode: "meeting"
local_only: false
created_at: "2026-10-12T14:00:00-03:00"
started_at: "2026-10-12T14:02:00-03:00"
ended_at: "2026-10-12T14:40:00-03:00"
recorded_ms: 2100000
stop_reason: "user"
audio_retained: true
transcribed_at: "2026-10-12T14:41:00-03:00"
template: "geral"
notes: true
notes_edited_at: null
participants: ["Ana","Bruno"]
speakers: {"1":"Ana"}
annotations: "decidir data\nAna: contrato"
---
# Planejamento

2026-10-12 14:02

## Anotações

decidir data ^a1

Ana: contrato ^a2

## Notas · Reunião geral

### Resumo

- Lançamento em 15/11. [[#^s2|01:05]] <!-- fala:ia -->

## Transcrição

- **[00:00] Eu:** Vamos falar da data. ^s1
- **[01:05] Ana:** Proponho 15 de novembro. ^s2

<!-- fala:segments
s1 mic me 0 4200
s2 system 1 65000 70000
-->
```

- Frontmatter como o dos ditados: `chave: <JSON>` por linha. `date` é derivado e o `reindex` o
  ignora. Nunca caminho de áudio nem de pasta (ADR-0011).
- O corpo é o mesmo documento do "Copiar Markdown" (`fala_notes::note_document`, rótulos pt-BR):
  título, data e hora, as notas (ou, sem elas, o bloco de anotações), a transcrição com âncoras
  `^sN`. O bloco `<!-- fala:segments -->` no fim (escondido no Obsidian) guarda canal, falante e
  tempos em ms de cada trecho, para o `reindex` voltar sem perda.

## Decisions

- O formato do `.md` mora em `fala-storage` (render + parse), sem depender de `fala-notes` (que
  traz o cliente HTTP); um teste com `fala-notes` como dev-dependency prova que o corpo é igual ao
  `note_document`.
- Rótulos e títulos de seção do espelho em pt-BR, como as pastas `Ditados/` e `Reuniões/`; as
  notas geradas entram como estão, no idioma em que foram geradas.
- Nome do arquivo: data local do início (ou da criação, no rascunho) + título sem os caracteres
  proibidos no Windows e nos links do Obsidian; sem título, "Reunião"; colisão vira `(2)`, `(3)`.
  O nome em uso fica em `meeting_files` (tabela aditiva), para renomear quando o título muda.
- Falha ao escrever o espelho não falha a mudança (o banco já tem a verdade): vai para o log e a
  próxima abertura do desktop reescreve. Apagar remove o `.md` dentro da transação, antes do
  commit: se o arquivo não sai, a sessão fica e volta erro, porque um `.md` órfão ressuscitaria a
  sessão no `reindex`.
- O `reindex` de reuniões só acrescenta as sessões que faltam no banco: nunca apaga nem reescreve
  uma sessão que o banco tem (o banco é a fonte de verdade e guarda o que o `.md` não guarda,
  como sugestões de nome pendentes). Com id repetido, vale o arquivo cujo nome bate com o título.
- Pausas: o banco guarda só a duração gravada (`recorded_ms`); os intervalos pausados não existem
  no SQLite, então não entram.
