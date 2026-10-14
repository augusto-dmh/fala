import { ChevronLeft } from "lucide-react";
import type { MeetingPerson, MeetingSummary } from "@/bindings";
import type { LiveSessions } from "./meetingDays";
import { MeetingList } from "./MeetingList";
import type { Translate } from "./meetingModel";

export interface PersonViewProps {
  person: MeetingPerson;
  /** As `person_meetings` returns them; `null` while they load. */
  sessions: MeetingSummary[] | null;
  onBack: () => void;
  onOpen: (id: string) => void;
  live?: LiveSessions;
  t: Translate;
  /** "Today" for the day headings; the tests pin it. */
  now?: Date;
}

/** One person: "← Pessoas", the name, and the meetings with them by day, like the home. */
export function PersonView({
  person,
  sessions,
  onBack,
  onOpen,
  live,
  t,
  now,
}: PersonViewProps) {
  return (
    <div className="space-y-4">
      <button
        type="button"
        onClick={onBack}
        className="flex items-center gap-1 rounded-lg text-body text-text-2 transition-colors hover:text-text focus-visible:focus-ring"
      >
        <ChevronLeft
          size={16}
          strokeWidth={1.5}
          className="rtl:rotate-180"
          aria-hidden="true"
        />
        {t("meeting.tabs.people")}
      </button>
      <div>
        <h1 className="font-display text-title font-semibold">{person.name}</h1>
        <p className="text-caption text-text-2">
          {t("meeting.directory.meetings", {
            count: sessions?.length ?? person.meetings,
          })}
        </p>
      </div>
      {sessions?.length === 0 && (
        <p className="py-4 text-body text-text-2">
          {t("meeting.directory.none")}
        </p>
      )}
      {sessions && sessions.length > 0 && (
        <MeetingList
          sessions={sessions}
          onOpen={onOpen}
          live={live}
          now={now}
        />
      )}
    </div>
  );
}
