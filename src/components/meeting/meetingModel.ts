import type {
  MeetingDetail,
  MeetingError,
  MeetingKeys,
  MeetingLine,
  MeetingSummary,
  ProgressStage,
  TranscriptWait,
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

/** How full a level meter is, 0 to 100: speech fills most of it. */
export function levelPercent(rms: number): number {
  return Math.round(Math.min(1, Math.sqrt(rms) * 2) * 100);
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
    case "unreachable":
    case "cancelled":
    case "busy":
    case "unknown_template":
    case "notes":
    case "notes_edited":
    case "tool_missing":
    case "invalid_url":
    case "download":
    case "import":
    case "import_no_audio":
    case "import_too_long":
    case "erase_too_long":
      return `meeting.errors.${kind}`;
    default:
      return "meeting.errors.generic";
  }
}

/** What a session has to write notes from, and whether it has audio to transcribe. */
export interface SessionContent {
  transcript: boolean;
  /** Notes typed in "My notes", saved or not. */
  typed: boolean;
  /** The audio to transcribe: kept on disk, never there (a Granola import) or deleted (by the
   * retention, or restored from the trash without it). Without it no key would help. */
  audio: "kept" | "imported" | "deleted";
}

/** Why "Transcrever" and "Gerar notas" are off, when the person can fix it: a missing key, or
 * notes with nothing to write from (no transcript and nothing typed). `null` when available.
 * Without audio "Transcrever" is off anyway; it says why only while there is no transcript yet,
 * so a transcribed meeting whose audio expired stays quiet and the notes' reason shows. */
export function sessionHints(
  keys: MeetingKeys | null,
  content: SessionContent,
): { transcribe: string | null; generate: string | null } {
  return {
    transcribe:
      content.audio !== "kept"
        ? content.transcript
          ? null
          : content.audio === "imported"
            ? "meeting.granola.noAudio"
            : "meeting.source.noAudio"
        : null,
    generate:
      keys && !keys.anthropic
        ? "meeting.keys.notesNeedKey"
        : !content.transcript && !content.typed
          ? "meeting.keys.notesNeedContent"
          : null,
  };
}

/** What the bar says about an automatic transcript that has not arrived (it waits for a retry or
 * the local model, or failed for good), or about one the local model wrote, and the label
 * "Transcrever" takes: "Transcrever na nuvem" once the ElevenLabs key is there. Keys of i18n. */
export function transcriptWait(
  wait: TranscriptWait | null | undefined,
  transcribing: boolean,
  local: boolean | null = false,
  cloudKey = false,
): { status: string | null; action: string } {
  if (transcribing)
    return { status: null, action: "meeting.view.transcribing" };
  if (wait?.state === "waiting")
    return {
      status: `meeting.view.wait.${wait.reason}`,
      action: "meeting.view.retryNow",
    };
  if (wait?.state === "failed")
    return {
      status:
        wait.error.kind === "invalid_key"
          ? "meeting.view.wait.keyRefused"
          : "meeting.view.wait.failed",
      action: "meeting.view.retry",
    };
  if (local)
    return {
      status: "meeting.view.localTranscript",
      action: cloudKey
        ? "meeting.view.transcribeCloud"
        : "meeting.view.transcribe",
    };
  return { status: null, action: "meeting.view.transcribe" };
}

/** The step whose command failed: it says which key a `missing_key` is about. */
export type KeyStep = "transcribe" | "notes";

/** A missing-key error as the key the step needs (ElevenLabs to transcribe, Anthropic for the
 * notes); `null` for any other error. */
export function missingKeyMessage(
  error: MeetingError | string | null,
  step: KeyStep | null,
): string | null {
  if (typeof error !== "object" || error?.kind !== "missing_key") return null;
  if (step === "transcribe") return "meeting.keys.missingElevenlabs";
  if (step === "notes") return "meeting.keys.missingAnthropic";
  return null;
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

/**
 * Generating again with the template of the notes the person edited replaces the edits: ask
 * first. Another template is a new version they asked for (Granola asks the same way).
 */
export function needsConfirmation(
  detail: Pick<MeetingDetail, "notes_edited" | "notes_template">,
  templateId: string,
): boolean {
  return detail.notes_edited && detail.notes_template === templateId;
}

/**
 * Set by the app when a click on the meeting pill asked for the session being recorded, so the
 * page opens it even if it mounts after the event.
 */
let openRequested: string | null = null;

export function requestOpenMeeting(id: string): void {
  openRequested = id;
}

export function takeOpenMeetingRequest(): string | null {
  const id = openRequested;
  openRequested = null;
  return id;
}
