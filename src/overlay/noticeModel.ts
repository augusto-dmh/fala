/** Code of a notice in the overlay (`NoticeKind` in `overlay.rs`). */
export type NoticeKind =
  | "mic_in_use"
  | "mic_muted"
  | "mic_denied"
  | "no_mic"
  | "mic_failed"
  | "model_missing";

/** Payload of `overlay-notice`. */
export interface OverlayNotice {
  kind: NoticeKind;
  /** The microphone's name, for `mic_in_use`. */
  device: string | null;
  /** Replaces the pill; otherwise the notice stacks over the recording pill. */
  alone: boolean;
  duration_ms: number;
}

export const NOTICE_KINDS: readonly NoticeKind[] = [
  "mic_in_use",
  "mic_muted",
  "mic_denied",
  "no_mic",
  "mic_failed",
  "model_missing",
];

/** The `overlay.notice.*` key of a notice's text. */
export function noticeKey(kind: NoticeKind): string {
  switch (kind) {
    case "mic_in_use":
      return "overlay.notice.micInUse";
    case "mic_muted":
      return "overlay.notice.micMuted";
    case "mic_denied":
      return "overlay.notice.micDenied";
    case "no_mic":
      return "overlay.notice.noMic";
    case "mic_failed":
      return "overlay.notice.micFailed";
    case "model_missing":
      return "overlay.notice.modelMissing";
  }
}

/** The dot before the text: amber for a muted microphone, red for a gesture that failed. */
export function noticeTone(kind: NoticeKind): "warn" | "error" | null {
  switch (kind) {
    case "mic_in_use":
      return null;
    case "mic_muted":
      return "warn";
    default:
      return "error";
  }
}

/** The microphone notices offer [Choose microphone] and [Fix it]. */
export function noticeHasActions(kind: NoticeKind): boolean {
  return (
    kind === "mic_muted" ||
    kind === "mic_denied" ||
    kind === "no_mic" ||
    kind === "mic_failed"
  );
}
