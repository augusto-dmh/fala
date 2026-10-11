import { useTranslation } from "react-i18next";
import type { MeetingSummary } from "@/bindings";
import { clock, titleOf, when } from "./meetingModel";

export interface MeetingListProps {
  /** Newest first, as `list_meetings` returns them. */
  sessions: MeetingSummary[];
  selectedId: string | null;
  onOpen: (id: string) => void;
}

export function MeetingList({
  sessions,
  selectedId,
  onOpen,
}: MeetingListProps) {
  const { t } = useTranslation();
  if (sessions.length === 0) {
    return <p className="p-4 text-sm">{t("meeting.list.empty")}</p>;
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
          </button>
        </li>
      ))}
    </ul>
  );
}
