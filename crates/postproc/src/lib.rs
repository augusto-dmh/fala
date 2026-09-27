//! Pós-processamento do texto ditado.
//!
//! Trait `Formatter`: regras determinísticas pt-BR (fillers, dicionário) e backend LLM opcional.
//! O LLM recebe só texto, o nome do app e o dicionário; nunca áudio, tela ou campo ativo (ADR-0004).
//! Acima de 2 s de resposta, o texto bruto é inserido; "desfazer edição da IA" por item.
//! Nasce vazio no dia 1; implementado na fase 1, semanas 3-4.
