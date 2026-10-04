import {
  PILL_BARS,
  PROCESSING_BARS,
  pillBars,
  type PillMode,
} from "./pillModel";

export interface PillProps {
  mode: PillMode;
  /** Push-to-talk style activation: the capsule turns red while recording. */
  holdToTalk: boolean;
  /** Microphone samples are flowing (`recording-ready`). */
  ready: boolean;
  /** Smoothed `mic-level` buckets. */
  levels: readonly number[];
  /** Translated name for screen readers; nothing is drawn as text. */
  label: string;
}

const RESTING = Array<number>(PILL_BARS).fill(3);

/** The phase 1 pill: a black capsule with ten mirrored bars, no icon, no text. */
export function Pill({ mode, holdToTalk, ready, levels, label }: PillProps) {
  const recording = mode === "recording";
  const heights = !recording
    ? PROCESSING_BARS
    : ready
      ? pillBars(levels)
      : RESTING;
  const className = [
    "fpill",
    mode,
    recording && holdToTalk ? "hold" : "",
    recording && !ready ? "arming" : "",
  ]
    .filter(Boolean)
    .join(" ");
  return (
    <div className={className} role="status" aria-label={label}>
      {heights.map((h, i) => (
        <i key={i} style={{ height: `${h}px` }} />
      ))}
    </div>
  );
}
