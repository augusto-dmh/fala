---
status: accepted
date: 2026-09-26
---
# Storage local: SQLite (rusqlite + FTS5) como fonte de verdade, com espelho Markdown no disco

## Contexto e problema
Ditados, transcrições, notas e metadados precisam de busca por texto com acentos ignorados, funcionar sem servidor, sobreviver a crash e ser legíveis fora do app (Obsidian, sync por pasta entre o Windows pessoal e o Linux do trabalho).

## Opções consideradas
* SQLite (`rusqlite` bundled + FTS5 `unicode61 remove_diacritics 2`, WAL) + espelho Markdown com frontmatter.
* Só arquivos Markdown.
* Só SQLite.
* libsql/Turso com replicas (caminho do anarlog).

## Decisão
SQLite como fonte de verdade da busca e do estado; Markdown como espelho gerado para leitura e sync. Índice reconstruível a partir dos `.md` se o banco for perdido.

### Consequências
* Bom: busca boa e barata; interoperabilidade; sem servidor.
* Ruim: dois lugares para manter consistentes; áudio não entra no espelho nem no sync.
* Obrigatório: escrita no SQLite primeiro, espelho depois; comando `fala-cli reindex` regenera o banco a partir dos `.md`.

## Confirmação
Teste round-trip: exportar sessão → apagar banco → `reindex` → busca retorna o mesmo resultado.
