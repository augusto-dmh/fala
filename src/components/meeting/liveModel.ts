import type { MeetingPreview, PreviewLine } from "@/bindings";

/** The preview as the page keeps it: lines in time order, and what each channel is hearing. */
export interface LiveState {
  lines: PreviewLine[];
  mic: boolean;
  system: boolean;
}

export const emptyLive: LiveState = { lines: [], mic: false, system: false };

const key = (line: PreviewLine) => `${line.channel}:${line.t0_ms}`;

/**
 * Adds an event (or the snapshot) to the state: lines already there are not repeated, and the
 * two channels end up interleaved by start time. A preview the transcript of record replaced
 * empties it.
 */
export function mergeLive(
  state: LiveState,
  preview: MeetingPreview,
): LiveState {
  if (preview.replaced) return emptyLive;
  const seen = new Set(state.lines.map(key));
  const added = preview.lines.filter((line) => !seen.has(key(line)));
  const lines =
    added.length === 0
      ? state.lines
      : [...state.lines, ...added].sort((a, b) => a.t0_ms - b.t0_ms);
  return {
    lines,
    mic: preview.mic_listening,
    system: preview.system_listening,
  };
}

/** Within this many pixels of the end, the list follows new lines. */
export const STICK_PX = 32;

/** Whether the list is at its end, so new lines scroll into view; a person who scrolled up to
 *  read is left where they are. */
export function atEnd(
  scrollTop: number,
  clientHeight: number,
  scrollHeight: number,
) {
  return scrollHeight - (scrollTop + clientHeight) <= STICK_PX;
}
