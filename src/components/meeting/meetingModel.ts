import type { MeetingError, MeetingLine, MeetingSummary } from "@/bindings";

/** `mm:ss`, or `h:mm:ss` from one hour on. */
export function clock(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const two = (n: number) => String(n).padStart(2, "0");
  return h > 0 ? `${h}:${two(m)}:${two(s)}` : `${two(m)}:${two(s)}`;
}

/** The i18n key of a command error: one per kind, the generic one otherwise. */
export function errorKey(error: MeetingError | string | undefined): string {
  const kind = typeof error === "object" && error ? error.kind : undefined;
  switch (kind) {
    case "consent_required":
    case "already_active":
    case "already_started":
    case "dictation_active":
    case "insufficient_disk":
    case "audio_device":
    case "not_found":
    case "still_recording":
    case "no_audio":
    case "missing_key":
    case "invalid_key":
    case "keyring":
    case "transcription":
    case "cancelled":
    case "busy":
    case "unknown_template":
    case "notes":
      return `meeting.errors.${kind}`;
    default:
      return "meeting.errors.generic";
  }
}

/** `YYYY-MM-DD HH:MM` in the offset the session was recorded in. */
export function when(rfc3339: string): string {
  return rfc3339.slice(0, 16).replace("T", " ");
}

export type Translate = (
  key: string,
  options?: Record<string, unknown>,
) => string;

/** The title, or "Meeting" and the date when it has none. */
export function titleOf(summary: MeetingSummary, t: Translate): string {
  const title = summary.title.trim();
  return title || t("meeting.untitled", { date: when(summary.created_at) });
}

/** `[mm:ss] Eu: texto` / `[mm:ss] Pessoa 2: texto`. */
export function lineText(line: MeetingLine, t: Translate): string {
  const who =
    line.person === null
      ? t("meeting.speaker.me")
      : t("meeting.speaker.person", { n: line.person });
  return `[${clock(line.t0_ms)}] ${who}: ${line.text}`;
}

/**
 * Set by the app when the tray asked to record before the notice was accepted, so the page
 * opens with the notice even if it mounts after the event.
 */
let consentRequested = false;

export function requestConsent(): void {
  consentRequested = true;
}

export function takeConsentRequest(): boolean {
  const requested = consentRequested;
  consentRequested = false;
  return requested;
}
