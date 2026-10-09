import { formatElapsed, meetingTone } from "./pillModel";

export interface MeetingPillProps {
  paused: boolean;
  /** Recorded time, without pauses (`MeetingStatus.recorded_ms`). */
  recordedMs: number;
  /** A channel has been silent for two minutes. */
  muted: boolean;
  /** Translated name for screen readers. */
  label: string;
}

/** The meeting indicator (ADR-0005): a dot and the recorded time, visible for the whole call. */
export function MeetingPill({
  paused,
  recordedMs,
  muted,
  label,
}: MeetingPillProps) {
  return (
    <div
      className={`fpill meeting ${meetingTone(paused, muted)}`}
      role="status"
      aria-label={label}
    >
      <span className="dot" />
      <span className="time">{formatElapsed(recordedMs)}</span>
    </div>
  );
}
