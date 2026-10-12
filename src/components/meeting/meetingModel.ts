import type {
  MeetingError,
  MeetingLine,
  MeetingSummary,
  ProgressStage,
} from "@/bindings";

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
    case "tool_missing":
    case "invalid_url":
    case "download":
    case "import":
    case "import_no_audio":
    case "import_too_long":
      return `meeting.errors.${kind}`;
    default:
      return "meeting.errors.generic";
  }
}

/** The `{{detail}}` an error key may show: the tool name, or the tail of the tool's message. */
export function errorDetail(error: MeetingError | string | null): string {
  return typeof error === "object" && error && "detail" in error
    ? String(error.detail)
    : "";
}

/** What "Import" accepts as a URL: `http://` or `https://` with a host, nothing else. */
export function isHttpUrl(text: string): boolean {
  return /^https?:\/\/[^\s/?#]+\S*$/i.test(text.trim());
}

/** The line shown while an import runs, from its last progress event. */
export function importLine(stage: ProgressStage | null, t: Translate): string {
  if (stage?.stage === "retaining") return t("meeting.import.saving");
  if (stage?.stage !== "importing") return t("meeting.import.starting");
  const percent =
    stage.total > 0
      ? Math.min(100, Math.floor((stage.done * 100) / stage.total))
      : null;
  if (stage.phase === "convert")
    return t("meeting.import.converting", { percent: percent ?? 0 });
  return percent === null
    ? t("meeting.import.downloadingBytes", {
        mb: (stage.done / (1024 * 1024)).toFixed(1),
      })
    : t("meeting.import.downloading", { percent });
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

/** "Eu" for the mic outside in-person mode, "Pessoa N" otherwise, or the name applied to it. */
export function speakerOf(line: MeetingLine, t: Translate): string {
  return line.person === null
    ? t("meeting.speaker.me")
    : (line.name ?? t("meeting.speaker.person", { n: line.person }));
}

/** `[mm:ss] Eu: texto` / `[mm:ss] Pessoa 2: texto`, or the name applied to Pessoa 2. */
export function lineText(line: MeetingLine, t: Translate): string {
  return `[${clock(line.t0_ms)}] ${speakerOf(line, t)}: ${line.text}`;
}

/** "Who was there" as typed: names split by comma, semicolon or line break, without blanks. */
export function parseParticipants(typed: string): string[] {
  return typed
    .split(/[,;\n]/)
    .map((name) => name.trim().replace(/\s+/g, " "))
    .filter((name) => name !== "");
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
