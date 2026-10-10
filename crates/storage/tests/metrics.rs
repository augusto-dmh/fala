//! `dictation_metrics`: a tabela aditiva, a linha sem texto e o resumo p50/p90.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use chrono::{DateTime, Duration, FixedOffset, Utc};
use common::{env, unedited};
use fala_core::Language;
use fala_storage::{DayCount, DictationMetrics, MetricsSummary, Percentiles};
use rusqlite::Connection;

fn ago(d: Duration) -> DateTime<FixedOffset> {
    (Utc::now() - d).fixed_offset()
}

fn metrics(created_at: DateTime<FixedOffset>, e2e_ms: u32) -> DictationMetrics {
    DictationMetrics {
        dictation_id: None,
        created_at,
        e2e_ms,
        asr_ms: e2e_ms / 2,
        llm_ms: None,
        paste_ms: 20,
        speech_ms: 1500,
        words: 5,
        lang: Language::PtBr,
        llm_used: false,
        fallback: None,
        model: "parakeet-tdt-0.6b-v3".to_string(),
        app: Some("notepad".to_string()),
    }
}

fn with_llm(mut m: DictationMetrics, llm_ms: u32, fallback: Option<&str>) -> DictationMetrics {
    m.llm_ms = Some(llm_ms);
    m.llm_used = fallback.is_none();
    m.fallback = fallback.map(str::to_string);
    m.words = 20;
    m
}

fn tables(db: &std::path::Path) -> Vec<String> {
    let conn = Connection::open(db).unwrap();
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap();
    let names = stmt.query_map([], |r| r.get(0)).unwrap();
    names.map(Result::unwrap).collect()
}

fn user_version(db: &std::path::Path) -> i64 {
    Connection::open(db)
        .unwrap()
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap()
}

#[test]
fn metrics_table_is_additive() {
    let env = env("metrics_table_is_additive");
    let id = env
        .open()
        .add(&unedited("antes"), common::at(9, 0, 0))
        .unwrap()
        .id;
    assert!(tables(&env.db).contains(&"dictation_metrics".to_string()));
    let version = user_version(&env.db);

    // Um banco de antes desta mudança: mesma versão, sem a tabela.
    Connection::open(&env.db)
        .unwrap()
        .execute_batch("DROP TABLE dictation_metrics;")
        .unwrap();
    let store = env.open();
    assert!(tables(&env.db).contains(&"dictation_metrics".to_string()));
    assert_eq!(user_version(&env.db), version);
    assert_eq!(store.get(&id).unwrap().dictation.final_text, "antes");
}

#[test]
fn metrics_have_no_text_and_outlive_the_item() {
    let env = env("metrics_have_no_text_and_outlive_the_item");
    let store = env.open();
    let id = store
        .add(
            &unedited("um texto que não pode ir para as métricas"),
            common::at(9, 0, 0),
        )
        .unwrap()
        .id;
    let mut m = metrics(ago(Duration::minutes(1)), 400);
    m.dictation_id = Some(id.clone());
    store.add_metrics(&m).unwrap();
    store.delete(&id).unwrap();

    let conn = Connection::open(&env.db).unwrap();
    let stmt = conn.prepare("SELECT * FROM dictation_metrics").unwrap();
    let columns = "rowid dictation_id created_at created_ms e2e_ms asr_ms llm_ms paste_ms \
                   speech_ms words lang llm_used fallback model app";
    assert_eq!(stmt.column_names().join(" "), columns);
    let (linked, lang): (String, String) = conn
        .query_row(
            "SELECT dictation_id, lang FROM dictation_metrics",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!((linked, lang.as_str()), (id, "pt-BR"));
    assert_eq!(store.metrics_summary(1).unwrap().dictations, 1);
}

#[test]
fn summary_percentiles_and_days() {
    let env = env("summary_percentiles_and_days");
    let store = env.open();
    let today = ago(Duration::minutes(1));
    let earlier = ago(Duration::hours(30));
    for e2e in (100..=1000).step_by(100) {
        store.add_metrics(&metrics(today, e2e)).unwrap();
    }
    for (e2e, fallback) in [
        (1100, None),
        (1300, None),
        (2100, None),
        (2400, Some("timeout")),
    ] {
        let m = with_llm(metrics(earlier, e2e), e2e - 400, fallback);
        store.add_metrics(&m).unwrap();
    }
    store
        .add_metrics(&metrics(ago(Duration::days(10)), 9000))
        .unwrap();

    let s = store.metrics_summary(7).unwrap();
    assert_eq!((s.days, s.dictations, s.words), (7, 14, 130));
    assert_eq!((s.llm_attempts, s.fallbacks), (4, 1));
    let day = |at: DateTime<FixedOffset>| at.format("%Y-%m-%d").to_string();
    assert_eq!(
        s.per_day,
        [
            DayCount {
                day: day(earlier),
                dictations: 4,
                words: 80
            },
            DayCount {
                day: day(today),
                dictations: 10,
                words: 50
            }
        ]
    );
    let p = |p50, p90| Percentiles {
        p50: Some(p50),
        p90: Some(p90),
    };
    assert_eq!(s.e2e, p(500, 900));
    assert_eq!(s.e2e_llm, p(1300, 2400));
    assert_eq!(s.llm, p(900, 2000));
    assert_eq!(s.paste, p(20, 20));
    assert_eq!(s.speech, p(1500, 1500));
    // asr = e2e / 2 sobre os 14: 50..500 e 550, 650, 1050, 1200.
    assert_eq!(s.asr, p(350, 1050));
}

#[test]
fn empty_summary() {
    let env = env("empty_summary");
    assert_eq!(
        env.open().metrics_summary(7).unwrap(),
        MetricsSummary {
            days: 7,
            ..MetricsSummary::default()
        }
    );
}
