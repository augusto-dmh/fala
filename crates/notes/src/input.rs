use std::collections::HashSet;

use fala_core::{Dictionary, Language};
use serde::{Deserialize, Serialize};

use crate::{NotesError, Template};

/// Canal de origem de um segmento. Não sai no payload (ADR-0016 não o lista).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Mic,
    System,
}

/// Quem fala num segmento: `"me"`, `{"person":N}` (forma da 2.F4) ou `{"name":"Ana"}`, o nome
/// que o usuário aplicou a um rótulo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Speaker {
    Me,
    Person(u32),
    #[serde(rename = "name")]
    Named(String),
}

/// Um trecho transcrito com id: o `Segment` da 2.F4 (`channel`, `speaker`, `t0_ms`, `t1_ms`,
/// `text`) mais o `id` que o chamador atribui e que os ponteiros das notas citam.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    pub id: u32,
    pub channel: Channel,
    pub speaker: Speaker,
    pub t0_ms: u64,
    pub t1_ms: u64,
    pub text: String,
}

/// Tudo o que a geração de notas de uma sessão recebe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotesInput {
    pub title: String,
    /// `AAAA-MM-DD`.
    pub date: String,
    /// `HH:MM`, hora local de início.
    pub start_time: String,
    /// Idioma das notas.
    pub language: Language,
    pub dictionary: Dictionary,
    pub template: Template,
    /// O texto que o usuário digitou durante a sessão; cada linha não vazia vira uma anotação.
    pub annotations: String,
    pub transcript: Vec<Segment>,
    /// Sessão marcada "só local": nada sai da máquina.
    pub local_only: bool,
}

impl NotesInput {
    pub(crate) fn validate(&self) -> Result<(), NotesError> {
        let mut ids = HashSet::new();
        for segment in &self.transcript {
            if !ids.insert(segment.id) {
                return Err(NotesError::InvalidInput(format!(
                    "id de segmento repetido: {}",
                    segment.id
                )));
            }
            if segment.t1_ms < segment.t0_ms {
                return Err(NotesError::InvalidInput(format!(
                    "segmento {} termina antes de começar",
                    segment.id
                )));
            }
        }
        if !is_date(&self.date) {
            return Err(NotesError::InvalidInput(
                "data fora do formato AAAA-MM-DD".to_string(),
            ));
        }
        if !is_time(&self.start_time) {
            return Err(NotesError::InvalidInput(
                "hora fora do formato HH:MM".to_string(),
            ));
        }
        if self.transcript.is_empty() && self.annotation_lines().is_empty() {
            return Err(NotesError::EmptySession);
        }
        Ok(())
    }

    /// As linhas não vazias das anotações, aparadas, na ordem; a posição + 1 é o id (`a1`, ...).
    pub(crate) fn annotation_lines(&self) -> Vec<&str> {
        self.annotations
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect()
    }
}

fn digits(s: &str) -> Option<u32> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

fn is_date(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    let [year, month, day] = parts.as_slice() else {
        return false;
    };
    if year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return false;
    }
    let (Some(year), Some(month), Some(day)) = (digits(year), digits(month), digits(day)) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day)
}

fn is_time(s: &str) -> bool {
    let Some((hour, minute)) = s.split_once(':') else {
        return false;
    };
    if hour.len() != 2 || minute.len() != 2 {
        return false;
    }
    matches!((digits(hour), digits(minute)), (Some(h), Some(m)) if h < 24 && m < 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::input;

    #[test]
    fn segment_deserializes_from_meeting_asr_shape() {
        let system: Segment = serde_json::from_str(
            r#"{"id":3,"channel":"system","speaker":{"person":1},"t0_ms":65000,"t1_ms":70000,"text":"oi"}"#,
        )
        .unwrap();
        assert_eq!(
            system,
            Segment {
                id: 3,
                channel: Channel::System,
                speaker: Speaker::Person(1),
                t0_ms: 65_000,
                t1_ms: 70_000,
                text: "oi".to_string(),
            }
        );
        let mic: Segment = serde_json::from_str(
            r#"{"id":4,"channel":"mic","speaker":"me","t0_ms":0,"t1_ms":1500,"text":"tudo bem"}"#,
        )
        .unwrap();
        assert_eq!(mic.channel, Channel::Mic);
        assert_eq!(mic.speaker, Speaker::Me);
        assert_eq!((mic.id, mic.t0_ms, mic.t1_ms), (4, 0, 1_500));
        assert_eq!(mic.text, "tudo bem");
        let named: Segment = serde_json::from_str(
            r#"{"id":5,"channel":"system","speaker":{"name":"Ana"},"t0_ms":0,"t1_ms":1,"text":"x"}"#,
        )
        .unwrap();
        assert_eq!(named.speaker, Speaker::Named("Ana".to_string()));
    }

    #[test]
    fn invalid_times_and_dates_are_rejected() {
        assert_eq!(input().validate(), Ok(()));

        let mut backwards = input();
        backwards.transcript[0].t1_ms = 0;
        backwards.transcript[0].t0_ms = 10;
        assert!(matches!(
            backwards.validate(),
            Err(NotesError::InvalidInput(_))
        ));

        for date in ["2026-13-02", "02/10/2026", "2026-02-30", "2026-10-2", ""] {
            let mut bad = input();
            bad.date = date.to_string();
            assert!(
                matches!(bad.validate(), Err(NotesError::InvalidInput(_))),
                "{date:?}"
            );
        }
        for time in ["25:00", "9h", "14:60", "1:02", ""] {
            let mut bad = input();
            bad.start_time = time.to_string();
            assert!(
                matches!(bad.validate(), Err(NotesError::InvalidInput(_))),
                "{time:?}"
            );
        }
        let mut leap = input();
        leap.date = "2028-02-29".to_string();
        leap.start_time = "00:00".to_string();
        assert_eq!(leap.validate(), Ok(()));
    }
}
