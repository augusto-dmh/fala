import type { ShortcutActivation } from "@/bindings";

/** What the pill draws: the mic is listening, or the dictation is being turned into text. */
export type PillMode = "recording" | "processing";

/** Payload of `show-overlay`. */
export type OverlayEvent =
  | "recording"
  | "streaming"
  | "transcribing"
  | "processing";

export const PILL_BARS = 10;

const MIN_BAR_PX = 3;
const MAX_BAR_PX = 18;

/** Fixed, mirrored shape while processing: the bars stop and the capsule pulses. */
export const PROCESSING_BARS = [5, 7, 9, 11, 13, 13, 11, 9, 7, 5];

/** The minimal overlay's mode for an event; `null` for the Live overlay, which is not the pill. */
export function toPillMode(event: OverlayEvent): PillMode | null {
  switch (event) {
    case "recording":
      return "recording";
    case "transcribing":
    case "processing":
      return "processing";
    case "streaming":
      return null;
  }
}

/**
 * Red while the key is held: push-to-talk, and the modes that start as a hold
 * (hold-or-toggle and the default hold-or-double-tap).
 */
export function isHoldToTalk(
  activation: ShortcutActivation | undefined,
): boolean {
  return (
    activation === "push_to_talk" ||
    activation === "hold_or_toggle" ||
    activation === "push_to_talk_double_tap"
  );
}

/**
 * The capsule's colour while it draws: amber close to the session limit (`recording-limit-warning`,
 * over the red of a held key), red while the key is held, black otherwise (`null`). Processing is
 * always black: the recording is over.
 */
export function pillTone(
  mode: PillMode,
  holdToTalk: boolean,
  limit: boolean,
): "limit" | "hold" | null {
  if (mode !== "recording") return null;
  if (limit) return "limit";
  return holdToTalk ? "hold" : null;
}

function barHeight(level: number | undefined): number {
  const v = Math.max(0, Math.min(1, level ?? 0));
  return MIN_BAR_PX + Math.pow(v, 0.7) * (MAX_BAR_PX - MIN_BAR_PX);
}

/**
 * Ten mirrored bar heights (px) from the smoothed `mic-level` buckets: `levels[0]` (the low
 * frequencies, where the voice is) on the two centre bars, `levels[4]` on the two ends.
 */
export function pillBars(levels: readonly number[]): number[] {
  return Array.from({ length: PILL_BARS }, (_, i) =>
    barHeight(
      levels[i < PILL_BARS / 2 ? PILL_BARS / 2 - 1 - i : i - PILL_BARS / 2],
    ),
  );
}

/** How long a load runs during the recording before the pill shows it; warm loads never flash. */
export const SLOW_LOAD_MS = 2000;

/**
 * When the in-flight model load started (`Date.now()` ms), after one `model-state-changed`
 * event; `null` when none is in flight. Only a completed or failed load ends it: the idle
 * watcher can send `unloaded` in the middle of a load.
 */
export function nextModelLoadStart(
  current: number | null,
  eventType: string,
  now: number,
): number | null {
  switch (eventType) {
    case "loading_started":
      return current ?? now;
    case "loading_completed":
    case "loading_failed":
      return null;
    default:
      return current;
  }
}

/**
 * Whether the pill shows the model load. After the key is released the wait is the load, so
 * at once; while recording the mic is already capturing, so only once the load is slow.
 */
export function showsModelLoading(
  mode: PillMode,
  loadStart: number | null,
  now: number,
): boolean {
  if (loadStart === null) return false;
  return mode === "processing" || now - loadStart >= SLOW_LOAD_MS;
}
