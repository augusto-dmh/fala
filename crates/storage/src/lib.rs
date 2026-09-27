//! Persistência local: SQLite com FTS5 e espelho Markdown.
//!
//! SQLite (`rusqlite` bundled, WAL, FTS5 `unicode61 remove_diacritics 2`) é a fonte de verdade (ADR-0006).
//! Os `.md` com frontmatter são o espelho para leitura e sync por pasta; `reindex` reconstrói o banco.
//! Escrita no SQLite primeiro, espelho depois.
//! Nasce vazio no dia 1; o histórico do desktop migra na fase 1.
