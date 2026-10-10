//! Métricas por ditado: uma linha sem texto por ditado do desktop e o resumo p50/p90.
//!
//! A tabela é aditiva e fica fora do `user_version`: `open` a cria com `IF NOT EXISTS`. Ela não
//! tem FK para `dictations`; a linha sobrevive ao item (retenção, apagar) porque não tem texto.

use chrono::{DateTime, FixedOffset};
use fala_core::Language;
use rusqlite::{params, Connection};
use serde::Serialize;

use crate::StorageError;

pub(crate) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS dictation_metrics (
    rowid INTEGER PRIMARY KEY,
    dictation_id TEXT,
    created_at TEXT NOT NULL,
    created_ms INTEGER NOT NULL,
    e2e_ms INTEGER NOT NULL,
    asr_ms INTEGER NOT NULL,
    llm_ms INTEGER,
    paste_ms INTEGER NOT NULL,
    speech_ms INTEGER NOT NULL,
    words INTEGER NOT NULL,
    lang TEXT NOT NULL,
    llm_used INTEGER NOT NULL CHECK (llm_used IN (0, 1)),
    fallback TEXT,
    model TEXT NOT NULL,
    app TEXT
);
CREATE INDEX IF NOT EXISTS dictation_metrics_by_time ON dictation_metrics (created_ms);
";

/// Os números de um ditado. Nunca o texto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictationMetrics {
    /// O item em `dictations`, quando ele foi gravado.
    pub dictation_id: Option<String>,
    pub created_at: DateTime<FixedOffset>,
    /// Soltar a tecla → fim da colagem.
    pub e2e_ms: u32,
    pub asr_ms: u32,
    /// Só quando o LLM foi tentado.
    pub llm_ms: Option<u32>,
    pub paste_ms: u32,
    /// O áudio entregue ao ASR (depois do VAD).
    pub speech_ms: u32,
    pub words: u32,
    pub lang: Language,
    /// O LLM produziu o texto colado.
    pub llm_used: bool,
    /// Por que o LLM foi tentado e não ficou (`timeout`, `http`, `network`, `invalid`).
    pub fallback: Option<String>,
    pub model: String,
    pub app: Option<String>,
}

/// Percentis por posto mais próximo; `None` sem amostras.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Percentiles {
    pub p50: Option<u32>,
    pub p90: Option<u32>,
}

/// Ditados e palavras de um dia, na data local de quem ditou (`AAAA-MM-DD`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DayCount {
    pub day: String,
    pub dictations: u32,
    pub words: u32,
}

/// O resumo dos últimos `days` dias.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct MetricsSummary {
    pub days: u32,
    pub dictations: u32,
    pub words: u32,
    /// Ditados em que o LLM foi tentado (usado ou fallback).
    pub llm_attempts: u32,
    pub fallbacks: u32,
    /// Do mais antigo ao mais recente.
    pub per_day: Vec<DayCount>,
    /// Soltar → texto dos ditados sem LLM tentado.
    pub e2e: Percentiles,
    /// Soltar → texto dos ditados com LLM tentado.
    pub e2e_llm: Percentiles,
    pub asr: Percentiles,
    pub llm: Percentiles,
    pub paste: Percentiles,
    pub speech: Percentiles,
}

pub(crate) fn insert(conn: &Connection, m: &DictationMetrics) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO dictation_metrics (dictation_id, created_at, created_ms, e2e_ms, asr_ms,
             llm_ms, paste_ms, speech_ms, words, lang, llm_used, fallback, model, app)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            m.dictation_id,
            m.created_at
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, false),
            m.created_at.timestamp_millis(),
            m.e2e_ms,
            m.asr_ms,
            m.llm_ms,
            m.paste_ms,
            m.speech_ms,
            m.words,
            m.lang.tag(),
            m.llm_used,
            m.fallback,
            m.model,
            m.app,
        ],
    )?;
    Ok(())
}

pub(crate) fn summary(
    conn: &Connection,
    days: u32,
    now_ms: i64,
) -> Result<MetricsSummary, StorageError> {
    let since = now_ms - i64::from(days) * 86_400_000;
    let mut stmt = conn.prepare(
        "SELECT substr(created_at, 1, 10), e2e_ms, asr_ms, llm_ms, paste_ms, speech_ms, words,
             llm_used, fallback IS NOT NULL
         FROM dictation_metrics WHERE created_ms >= ?1 ORDER BY created_ms",
    )?;
    let mut out = MetricsSummary {
        days,
        ..MetricsSummary::default()
    };
    let (mut e2e, mut e2e_llm, mut asr, mut llm, mut paste, mut speech) =
        (vec![], vec![], vec![], vec![], vec![], vec![]);
    let mut rows = stmt.query([since])?;
    while let Some(row) = rows.next()? {
        let day: String = row.get(0)?;
        let words: u32 = row.get(6)?;
        let (llm_used, fell_back): (bool, bool) = (row.get(7)?, row.get(8)?);
        out.dictations += 1;
        out.words += words;
        out.fallbacks += u32::from(fell_back);
        match out.per_day.last_mut() {
            Some(last) if last.day == day => {
                last.dictations += 1;
                last.words += words;
            }
            _ => out.per_day.push(DayCount {
                day,
                dictations: 1,
                words,
            }),
        }
        if llm_used || fell_back {
            out.llm_attempts += 1;
            e2e_llm.push(row.get(1)?);
        } else {
            e2e.push(row.get(1)?);
        }
        asr.push(row.get(2)?);
        if let Some(ms) = row.get::<_, Option<u32>>(3)? {
            llm.push(ms);
        }
        paste.push(row.get(4)?);
        speech.push(row.get(5)?);
    }
    out.e2e = percentiles(e2e);
    out.e2e_llm = percentiles(e2e_llm);
    out.asr = percentiles(asr);
    out.llm = percentiles(llm);
    out.paste = percentiles(paste);
    out.speech = percentiles(speech);
    Ok(out)
}

fn percentiles(mut values: Vec<u32>) -> Percentiles {
    values.sort_unstable();
    // O posto `ceil(q·n)` em inteiros: q = 50/100 e 90/100.
    let at = |pct: usize| {
        let n = values.len();
        let rank = (pct * n).div_ceil(100).max(1);
        values.get(rank - 1).copied()
    };
    Percentiles {
        p50: at(50),
        p90: at(90),
    }
}
