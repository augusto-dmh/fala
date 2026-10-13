//! Where the PC slept during a recording: `audio/<id>/sleeps.json`, next to the audio, read by
//! the transcript to show "O computador dormiu por N min" at that point.

use std::path::Path;

use fala_meeting::{Gap, GapKind};
use serde::{Deserialize, Serialize};
use specta::Type;

const FILE: &str = "sleeps.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct MeetingSleep {
    /// Position in the recorded audio (without pauses), comparable to a line's `t0_ms`.
    pub at_ms: u64,
    pub slept_ms: u64,
}

/// A closed suspend gap of the session as a marker.
pub(crate) fn from_gap(gap: &Gap) -> Option<MeetingSleep> {
    if gap.kind != GapKind::Suspend {
        return None;
    }
    let to = gap.to?;
    Some(MeetingSleep {
        at_ms: u64::try_from(gap.audio_offset.as_millis()).unwrap_or(u64::MAX),
        slept_ms: to.0.saturating_sub(gap.from.0),
    })
}

pub(crate) fn append(dir: &Path, sleep: MeetingSleep) -> std::io::Result<()> {
    let mut all = read(dir);
    all.push(sleep);
    let json = serde_json::to_vec(&all).map_err(std::io::Error::other)?;
    std::fs::write(dir.join(FILE), json)
}

/// The markers of a session, in order; none when the PC never slept (or the audio is gone).
pub(crate) fn read(dir: &Path) -> Vec<MeetingSleep> {
    let Ok(bytes) = std::fs::read(dir.join(FILE)) else {
        return Vec::new();
    };
    serde_json::from_slice(&bytes).unwrap_or_else(|e| {
        log::warn!("{}: {e}", dir.join(FILE).display());
        Vec::new()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fala_meeting::UnixMillis;
    use std::time::Duration;

    #[test]
    fn a_closed_suspend_gap_is_a_marker() {
        let gap = Gap {
            kind: GapKind::Suspend,
            audio_offset: Duration::from_secs(240),
            from: UnixMillis(1_000_000),
            to: Some(UnixMillis(1_000_000 + 12 * 60_000)),
        };
        assert_eq!(
            from_gap(&gap),
            Some(MeetingSleep {
                at_ms: 240_000,
                slept_ms: 720_000
            })
        );
        assert_eq!(from_gap(&Gap { to: None, ..gap }), None);
        let pause = Gap {
            kind: GapKind::Pause,
            ..gap
        };
        assert_eq!(from_gap(&pause), None);
    }

    #[test]
    fn markers_add_up_next_to_the_audio() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read(dir.path()).is_empty());
        let first = MeetingSleep {
            at_ms: 60_000,
            slept_ms: 300_000,
        };
        let second = MeetingSleep {
            at_ms: 90_000,
            slept_ms: 5_000,
        };
        append(dir.path(), first).unwrap();
        append(dir.path(), second).unwrap();
        assert_eq!(read(dir.path()), vec![first, second]);
        std::fs::write(dir.path().join(FILE), b"not json").unwrap();
        assert!(read(dir.path()).is_empty());
    }
}
