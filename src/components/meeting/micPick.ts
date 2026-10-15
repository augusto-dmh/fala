/** What the pill's muted-mic card sends to the main window (`open_meeting_mic_picker`). */
export const PICK_MIC_EVENT = "meeting-pick-mic";

/**
 * The pill asked for the microphone picker. The main window opens on the session first, so the
 * picker may mount after the event: a request with nobody to open it waits for the next one.
 */
let pending = false;
const pickers = new Set<() => void>();

export function requestMicPick(): void {
  if (pickers.size === 0) {
    pending = true;
  } else {
    pickers.forEach((open) => open());
  }
}

/** A picker on screen: opens now if a request is waiting, and on every later one. */
export function onMicPick(open: () => void): () => void {
  pickers.add(open);
  if (pending) {
    pending = false;
    open();
  }
  return () => {
    pickers.delete(open);
  };
}
