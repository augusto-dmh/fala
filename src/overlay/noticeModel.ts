/** Code of a notice in the overlay (`NoticeKind` in `overlay.rs`). */
export type NoticeKind =
  | "mic_denied"
  | "no_mic"
  | "mic_failed"
  | "model_missing";

/** Payload of `overlay-notice`. */
export interface OverlayNotice {
  kind: NoticeKind;
  /** The microphone's name, when the notice names it. */
  device: string | null;
  /** Replaces the pill; otherwise the notice stacks over the recording pill. */
  alone: boolean;
  duration_ms: number;
}

export const NOTICE_KINDS: readonly NoticeKind[] = [
  "mic_denied",
  "no_mic",
  "mic_failed",
  "model_missing",
];

/** The `overlay.notice.*` key of a notice's text. */
export function noticeKey(kind: NoticeKind): string {
  switch (kind) {
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

/** The dot before the text: red for a gesture that failed. */
export function noticeTone(kind: NoticeKind): "warn" | "error" | null {
  switch (kind) {
    case "mic_denied":
    case "no_mic":
    case "mic_failed":
    case "model_missing":
      return "error";
  }
}
