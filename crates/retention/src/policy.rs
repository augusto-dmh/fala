use fala_meeting::{SessionId, UnixMillis};
use serde::{Deserialize, Serialize};

use crate::RetentionError;

const DAY_MS: u64 = 86_400_000;

/// Dias de retenção, de 1 a 3650. Validado também ao desserializar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct RetentionDays(u16);

impl RetentionDays {
    pub const MIN: u16 = 1;
    pub const MAX: u16 = 3650;

    pub fn new(days: u16) -> Result<Self, RetentionError> {
        if (Self::MIN..=Self::MAX).contains(&days) {
            Ok(Self(days))
        } else {
            Err(RetentionError::DaysOutOfRange(days))
        }
    }

    pub fn get(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for RetentionDays {
    type Error = RetentionError;

    fn try_from(days: u16) -> Result<Self, Self::Error> {
        Self::new(days)
    }
}

impl From<RetentionDays> for u16 {
    fn from(days: RetentionDays) -> Self {
        days.0
    }
}

/// Quando o áudio retido de uma sessão encerrada pode ser apagado (design doc §3.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionPolicy {
    /// Nunca apaga sozinho (padrão).
    #[default]
    Keep,
    /// Apaga N dias depois do fim da sessão.
    DeleteAfterDays(RetentionDays),
    /// Apaga quando a transcrição da sessão foi confirmada.
    DeleteAfterTranscript,
}

impl RetentionPolicy {
    pub fn delete_after_days(days: u16) -> Result<Self, RetentionError> {
        Ok(Self::DeleteAfterDays(RetentionDays::new(days)?))
    }
}

/// O que a política precisa saber de uma sessão.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RetainedSession {
    pub id: SessionId,
    /// `None` enquanto a sessão grava ou se foi interrompida sem fim.
    pub ended_at: Option<UnixMillis>,
    pub transcript_confirmed: bool,
}

/// As sessões cujo áudio a política manda apagar no instante `now`.
pub fn audio_due(
    policy: RetentionPolicy,
    sessions: &[RetainedSession],
    now: UnixMillis,
) -> Vec<SessionId> {
    sessions
        .iter()
        .filter(|s| {
            let Some(ended_at) = s.ended_at else {
                return false;
            };
            match policy {
                RetentionPolicy::Keep => false,
                RetentionPolicy::DeleteAfterDays(days) => {
                    now.0.saturating_sub(ended_at.0) >= u64::from(days.get()) * DAY_MS
                }
                RetentionPolicy::DeleteAfterTranscript => s.transcript_confirmed,
            }
        })
        .map(|s| s.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: UnixMillis = UnixMillis(1_800_000_000_000);
    const MINUTE_MS: u64 = 60_000;

    fn ended(n: u128, ago_ms: u64, transcript_confirmed: bool) -> RetainedSession {
        RetainedSession {
            id: SessionId::from_parts(1_700_000_000_000, n),
            ended_at: Some(UnixMillis(NOW.0 - ago_ms)),
            transcript_confirmed,
        }
    }

    #[test]
    fn keep_never_deletes() {
        assert_eq!(RetentionPolicy::default(), RetentionPolicy::Keep);
        let sessions = [
            ended(1, 0, false),
            ended(2, DAY_MS, true),
            ended(3, 10_000 * DAY_MS, false),
            ended(4, 10_000 * DAY_MS, true),
        ];
        assert!(audio_due(RetentionPolicy::Keep, &sessions, NOW).is_empty());
    }

    #[test]
    fn delete_after_days_boundary() {
        let policy = RetentionPolicy::delete_after_days(30).unwrap();
        let almost = ended(1, 30 * DAY_MS - MINUTE_MS, false);
        let exactly = ended(2, 30 * DAY_MS, false);
        let older = ended(3, 31 * DAY_MS, false);
        assert_eq!(
            audio_due(policy, &[almost, exactly, older], NOW),
            vec![exactly.id, older.id]
        );
    }

    #[test]
    fn delete_after_transcript() {
        let confirmed_new = ended(1, 0, true);
        let pending_old = ended(2, 400 * DAY_MS, false);
        let confirmed_old = ended(3, 400 * DAY_MS, true);
        assert_eq!(
            audio_due(
                RetentionPolicy::DeleteAfterTranscript,
                &[confirmed_new, pending_old, confirmed_old],
                NOW
            ),
            vec![confirmed_new.id, confirmed_old.id]
        );
    }

    #[test]
    fn unfinished_session_never_due() {
        let unfinished = RetainedSession {
            id: SessionId::from_parts(1_700_000_000_000, 9),
            ended_at: None,
            transcript_confirmed: true,
        };
        let later = UnixMillis(NOW.0 + 10_000 * DAY_MS);
        for policy in [
            RetentionPolicy::Keep,
            RetentionPolicy::delete_after_days(1).unwrap(),
            RetentionPolicy::DeleteAfterTranscript,
        ] {
            assert!(
                audio_due(policy, &[unfinished], later).is_empty(),
                "{policy:?}"
            );
        }
    }

    #[test]
    fn days_bounds() {
        for days in [0, 3651] {
            assert!(matches!(
                RetentionPolicy::delete_after_days(days),
                Err(RetentionError::DaysOutOfRange(d)) if d == days
            ));
        }
        for days in [1, 3650] {
            assert_eq!(
                RetentionPolicy::delete_after_days(days).unwrap(),
                RetentionPolicy::DeleteAfterDays(RetentionDays::new(days).unwrap())
            );
        }
    }

    #[test]
    fn serialized_forms() {
        for (policy, json) in [
            (RetentionPolicy::Keep, r#""keep""#),
            (
                RetentionPolicy::delete_after_days(30).unwrap(),
                r#"{"delete_after_days":30}"#,
            ),
            (
                RetentionPolicy::DeleteAfterTranscript,
                r#""delete_after_transcript""#,
            ),
        ] {
            assert_eq!(serde_json::to_string(&policy).unwrap(), json);
            assert_eq!(
                serde_json::from_str::<RetentionPolicy>(json).unwrap(),
                policy
            );
        }
        assert!(serde_json::from_str::<RetentionPolicy>(r#"{"delete_after_days":0}"#).is_err());
    }
}
