# granola-csv-import — o histórico do Granola pelo "Historical Data Export"

Profile: light · fase 2 ("Granola desinstalado") · ADR-0006 (tabela aditiva), ADR-0008 (arquivo
local, sem conta), ADR-0016 (resumo do Granola marcado como IA). Nenhuma contrariada.

## Flow

1. Reuniões → Importar → "Histórico do Granola (CSV)": diálogo de arquivo (`.csv` ou `.zip`).
2. `preview_granola_import(path)`: lê e valida, não grava; "N reuniões, de A a B; M ignoradas".
3. `run_granola_import(path)`: uma reunião por vez, progresso "Importando N de M".
4. Cada linha vira sessão `import` terminada no instante de `document_created`, sem áudio:
   `notes` → anotações, `summary` → `notes_md` com o marcador de IA por linha e rodapé,
   `transcript` → trechos sem tempo (`Me:`/`You:` = mic; outro rótulo = "Pessoa N" com nome).
5. `meeting_imports(source, external_id, meeting_id, imported_at)`: importar de novo reescreve a
   mesma sessão. O `.md` sai pelo espelho de sempre.

## Slices (PRs empilhados, gate de 1000 linhas)

- S1 `feat/granola-csv-import`: parser (`crates/storage/src/granola.rs`, `csv`, `zip` só deflate).
- S2 `feat/granola-csv-import-2`: `meeting_imports` e `Store::import_granola_meeting`.
- S3 `feat/granola-csv-import-3`: comandos do desktop, diálogo, selo "Granola", locales.
