import { formatElapsed, meetingTone } from "./pillModel";

export interface MeetingPillProps {
  paused: boolean;
  /** Recorded time, without pauses (`MeetingStatus.recorded_ms`). */
  recordedMs: number;
  /** A channel has been silent for two minutes, or in digital zero for ~3 s. */
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

export interface CallOutputCardProps {
  /** The translated "the call's sound is playing on <device>". */
  text: string;
  switchLabel: string;
  onSwitch: () => void;
}

/** Over the meeting pill while the call plays on an output that is not being recorded. */
export function CallOutputCard({
  text,
  switchLabel,
  onSwitch,
}: CallOutputCardProps) {
  return (
    <div className="fnotice call_output" role="alert">
      <div className="fnotice-line">
        <span className="fnotice-dot warn" />
        <span className="fnotice-text">{text}</span>
      </div>
      <div className="fnotice-actions">
        <button type="button" onClick={onSwitch}>
          {switchLabel}
        </button>
      </div>
    </div>
  );
}

/** Over the meeting pill in the first seconds of each recording: ask the others first. */
export function ConsentCard({ text }: { text: string }) {
  return (
    <div className="fnotice consent" role="status">
      <div className="fnotice-line">
        <span className="fnotice-dot" />
        <span className="fnotice-text">{text}</span>
      </div>
    </div>
  );
}
