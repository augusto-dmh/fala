import { useTranslation } from "react-i18next";
import type { MeetingSummary } from "@/bindings";
import { clock, titleOf, when } from "./meetingModel";
import { snippetParts } from "./meetingSearch";

export interface MeetingListProps {
  /** Newest first, as `list_meetings` returns them. */
  sessions: MeetingSummary[];
  selectedId: string | null;
  onOpen: (id: string) => void;
  /** While searching: the excerpt that matched, by session. */
  snippets?: Map<string, string>;
}

export function MeetingList({
  sessions,
  selectedId,
  onOpen,
  snippets,
}: MeetingListProps) {
  const { t } = useTranslation();
  if (sessions.length === 0) {
    return (
      <p className="p-4 text-sm">
        {t(snippets ? "meeting.list.noResults" : "meeting.list.empty")}
      </p>
    );
  }
  return (
    <ul className="divide-y divide-mid-gray/20">
      {sessions.map((session) => (
        <li key={session.id}>
          <button
            type="button"
            className={`w-full px-4 py-2 text-start text-sm ${
              session.id === selectedId ? "bg-mid-gray/10" : ""
            }`}
            onClick={() => onOpen(session.id)}
          >
            <span className="font-semibold">{titleOf(session, t)}</span>
            <span className="ms-2 text-xs">
              {when(session.started_at ?? session.created_at)}
            </span>
            <span className="ms-2 text-xs">
              {session.started_at === null
                ? t("meeting.list.draft")
                : clock(session.recorded_ms)}
            </span>
            {session.transcribed && (
              <span className="ms-2 text-xs">
                {t("meeting.list.transcribed")}
              </span>
            )}
            {session.has_notes && (
              <span className="ms-2 text-xs">{t("meeting.list.notes")}</span>
            )}
            {snippets?.has(session.id) && (
              <span className="block text-xs text-mid-gray">
                {snippetParts(snippets.get(session.id) ?? "").map((part, i) =>
                  part.match ? (
                    <mark key={i}>{part.text}</mark>
                  ) : (
                    <span key={i}>{part.text}</span>
                  ),
                )}
              </span>
            )}
          </button>
        </li>
      ))}
    </ul>
  );
}
