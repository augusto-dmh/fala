import type { MeetingSummary } from "@/bindings";
import { formatClock } from "@/utils/dateFormat";
import type { Translate } from "./meetingModel";

/** A day of the Meetings home: sessions that started that local day, latest first. */
export interface MeetingDay {
  key: string;
  /** When the day's latest session started. */
  date: Date;
  sessions: MeetingSummary[];
}

/** When the session started, or when the draft was created. */
export const startOf = (session: MeetingSummary): Date =>
  new Date(session.started_at ?? session.created_at);

const dayKey = (date: Date) =>
  `${date.getFullYear()}-${date.getMonth() + 1}-${date.getDate()}`;

/**
 * Sessions by local calendar day, latest start first. The list comes ordered by creation, and a
 * draft recorded days later (or an import of an old file) would otherwise put a day out of place.
 */
export function groupMeetingsByDay(sessions: MeetingSummary[]): MeetingDay[] {
  const days = new Map<string, MeetingDay>();
  const latestFirst = sessions
    .map((session) => ({ session, date: startOf(session) }))
    .sort((a, b) => b.date.getTime() - a.date.getTime());
  for (const { session, date } of latestFirst) {
    const key = dayKey(date);
    const day = days.get(key);
    if (day) day.sessions.push(session);
    else days.set(key, { key, date, sessions: [session] });
  }
  return [...days.values()];
}

// One formatter per locale and shape: the page renders four times a second while it records.
const formatters = new Map<string, Intl.DateTimeFormat>();
function format(
  locale: string,
  options: Intl.DateTimeFormatOptions,
  date: Date,
): string {
  const id = `${locale}|${JSON.stringify(options)}`;
  let formatter = formatters.get(id);
  if (!formatter) {
    formatter = new Intl.DateTimeFormat(locale, options);
    formatters.set(id, formatter);
  }
  return formatter.format(date);
}

/** "Hoje", "Ontem", or "seg., 6 de out." (with the year outside the current one). */
export function dayLabel(
  date: Date,
  now: Date,
  locale: string,
  t: Translate,
): string {
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (dayKey(date) === dayKey(now)) return t("meeting.home.today");
  if (dayKey(date) === dayKey(yesterday)) return t("meeting.home.yesterday");
  return format(
    locale,
    {
      weekday: "short",
      day: "numeric",
      month: "short",
      year: date.getFullYear() === now.getFullYear() ? undefined : "numeric",
    },
    date,
  );
}

/** The local hour and minute the session started ("3:50 PM", "15:50"). */
export const startTime = (session: MeetingSummary, locale: string): string =>
  formatClock(startOf(session), locale);

/** The dot of a row: recording, transcribing or writing notes, notes ready, or failed. */
export type RowState = "recording" | "working" | "ready" | "failed" | "idle";

export interface LiveSessions {
  /** The session recording (or paused) now. */
  recording: string | null;
  /** Being transcribed or getting its notes. */
  working: ReadonlySet<string>;
  /** Its last transcription or notes run failed while the page was open. */
  failed: ReadonlySet<string>;
  /** Of those, the ones where the notes step is what failed. */
  notesFailed?: ReadonlySet<string>;
}

export function rowState(
  session: MeetingSummary,
  live: LiveSessions,
): RowState {
  if (session.id === live.recording) return "recording";
  if (live.working.has(session.id)) return "working";
  if (live.failed.has(session.id)) return "failed";
  return session.has_notes ? "ready" : "idle";
}
