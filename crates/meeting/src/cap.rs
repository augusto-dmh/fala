use std::time::Duration;

use crate::SessionError;

/// Teto da duração gravada de uma sessão (decisão 6 do roadmap): 1 a 8 h, padrão 3 h.
///
/// Ao bater o teto a sessão para e manda processar; nada é descartado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RecordingCap {
    hours: u8,
}

impl RecordingCap {
    pub const MIN_HOURS: u8 = 1;
    pub const MAX_HOURS: u8 = 8;
    pub const DEFAULT_HOURS: u8 = 3;

    pub fn from_hours(hours: u8) -> Result<Self, SessionError> {
        if (Self::MIN_HOURS..=Self::MAX_HOURS).contains(&hours) {
            Ok(Self { hours })
        } else {
            Err(SessionError::CapOutOfRange(hours))
        }
    }

    pub fn hours(self) -> u8 {
        self.hours
    }

    pub fn duration(self) -> Duration {
        Duration::from_secs(u64::from(self.hours) * 3600)
    }

    /// O teto com mais 1 h ("mais 1 h" do aviso), recusado a partir de 8 h.
    pub fn extended(self) -> Result<Self, SessionError> {
        if self.hours >= Self::MAX_HOURS {
            return Err(SessionError::CapAtMaximum);
        }
        Ok(Self {
            hours: self.hours + 1,
        })
    }
}

impl Default for RecordingCap {
    fn default() -> Self {
        Self {
            hours: Self::DEFAULT_HOURS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_bounds() {
        assert_eq!(RecordingCap::from_hours(1).unwrap().hours(), 1);
        assert_eq!(RecordingCap::from_hours(8).unwrap().hours(), 8);
        assert_eq!(
            RecordingCap::from_hours(0),
            Err(SessionError::CapOutOfRange(0))
        );
        assert_eq!(
            RecordingCap::from_hours(9),
            Err(SessionError::CapOutOfRange(9))
        );
        assert_eq!(
            RecordingCap::default().duration(),
            Duration::from_secs(3 * 3600)
        );
    }
}
