import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { MeetingSummary } from "@/bindings";
import {
  dayLabel,
  groupMeetingsByDay,
  rowState,
  startOf,
  startTime,
  type LiveSessions,
  type RowState,
} from "./meetingDays";
import { clock, titleOf } from "./meetingModel";
import { snippetParts } from "./meetingSearch";

export interface MeetingListProps {
  /** As `list_meetings` returns them; the search results in their own order. */
  sessions: MeetingSummary[];
  onOpen: (id: string) => void;
  /** While searching: the excerpt that matched, by session. */
  snippets?: Map<string, string>;
  /** What runs now, for each row's dot; none when absent. */
  live?: LiveSessions;
  /** "Today" for the day headings; the tests pin it. */
  now?: Date;
}

const NOTHING_LIVE: LiveSessions = {
  recording: null,
  working: new Set(),
  failed: new Set(),
  notesFailed: new Set(),
};

const DOT: Record<RowState, string> = {
  recording: "bg-rec",
  working: "bg-text-2",
  ready: "bg-ok",
  failed: "bg-danger",
  idle: "bg-border",
};

/** The Meetings home: sessions by day, or the search results with the excerpt that matched. */
export function MeetingList({
  sessions,
  onOpen,
  snippets,
  live = NOTHING_LIVE,
  now = new Date(),
}: MeetingListProps) {
  const { t, i18n } = useTranslation();
  const locale = i18n.language;
  // The page renders four times a second while it records; the days change with the list.
  const days = useMemo(() => groupMeetingsByDay(sessions), [sessions]);

  if (sessions.length === 0) {
    return snippets ? (
      <p className="py-4 text-body text-text-2">
        {t("meeting.list.noResults")}
      </p>
    ) : (
      <div className="rounded-lg border border-dashed border-border px-6 py-10 text-center">
        <p className="text-body font-semibold">{t("meeting.list.empty")}</p>
        <p className="mt-1 text-caption text-text-2">
          {t("meeting.list.emptyHint")}
        </p>
      </div>
    );
  }

  // `stamp`: the time under a day heading, the day itself in the search results.
  // A failed row says what is missing; notes that didn't finish (often a missing key) are a
  // warning, a meeting without a transcript is an error.
  const row = (session: MeetingSummary, stamp: string, wide = false) => {
    const state = rowState(session, live);
    const snippet = snippets?.get(session.id);
    return (
      <li key={session.id}>
        <button
          type="button"
          className="flex w-full items-start gap-3 px-4 py-2.5 text-start transition-colors hover:bg-surface-2 focus-visible:focus-ring"
          onClick={() => onOpen(session.id)}
        >
          <span
            className={`mt-2 size-2 shrink-0 rounded-full ${
              state === "failed" && live.notesFailed?.has(session.id)
                ? "bg-warn"
                : DOT[state]
            }`}
            title={t(`meeting.home.state.${state}`)}
            aria-hidden="true"
          />
          <span
            className={`${wide ? "w-28" : "w-16"} shrink-0 whitespace-nowrap pt-px text-caption text-text-2 tabular-nums`}
          >
            {stamp}
          </span>
          <span className="min-w-0 flex-1">
            <span className="block truncate text-body">
              {titleOf(session, t)}
            </span>
            {state === "failed" ? (
              <span className="block text-caption text-text-2">
                {t(
                  live.notesFailed?.has(session.id)
                    ? "meeting.home.failedWhy.notes"
                    : "meeting.home.failedWhy.transcript",
                )}
              </span>
            ) : (
              state !== "idle" && (
                <span className="sr-only">
                  {t(`meeting.home.state.${state}`)}
                </span>
              )
            )}
            {snippet !== undefined && (
              <span className="block text-caption text-text-2">
                {snippetParts(snippet).map((part, i) =>
                  part.match ? (
                    <mark key={i}>{part.text}</mark>
                  ) : (
                    <span key={i}>{part.text}</span>
                  ),
                )}
              </span>
            )}
          </span>
          <span className="shrink-0 pt-px text-caption text-text-2 tabular-nums">
            {session.imported_from === "granola"
              ? t("meeting.list.granola")
              : session.started_at === null
                ? t("meeting.list.draft")
                : clock(session.recorded_ms)}
          </span>
        </button>
      </li>
    );
  };
  const card =
    "divide-y divide-border rounded-lg border border-border bg-surface-1";

  if (snippets) {
    // The search's own order; each row says its day instead of a heading per row.
    return (
      <ul className={card}>
        {sessions.map((session) =>
          row(session, dayLabel(startOf(session), now, locale, t), true),
        )}
      </ul>
    );
  }
  return (
    <div className="space-y-5">
      {days.map((day) => (
        <section key={day.key} className="space-y-2">
          <h2 className="text-caption font-semibold text-text-2">
            {dayLabel(day.date, now, locale, t)}
          </h2>
          <ul className={card}>
            {day.sessions.map((session) =>
              row(session, startTime(session, locale)),
            )}
          </ul>
        </section>
      ))}
    </div>
  );
}
