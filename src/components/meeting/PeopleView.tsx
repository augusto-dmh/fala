import type { MeetingPerson } from "@/bindings";
import { matchesQuery } from "@/lib/utils/search";
import { dayLabel } from "./meetingDays";
import type { Translate } from "./meetingModel";

export type MeetingsTab = "meetings" | "people";

/** The people whose name has the search box's text, without case or accents. */
export function filterPeople(
  people: MeetingPerson[],
  query: string,
): MeetingPerson[] {
  return people.filter((p) => matchesQuery(p.name, query));
}

export interface TabsProps {
  tab: MeetingsTab;
  onTab: (tab: MeetingsTab) => void;
  t: Translate;
}

/** "Reuniões | Pessoas", beside the search box. */
export function MeetingsTabs({ tab, onTab, t }: TabsProps) {
  return (
    <div
      role="group"
      aria-label={t("meeting.tabs.label")}
      className="inline-flex rounded-lg border border-border bg-surface-1 p-0.5"
    >
      {(["meetings", "people"] as const).map((option) => (
        <button
          key={option}
          type="button"
          aria-pressed={option === tab}
          onClick={() => onTab(option)}
          className={`min-h-[28px] cursor-pointer rounded-md px-3 text-body transition-colors focus-visible:focus-ring ${
            option === tab
              ? "bg-text/10 font-semibold text-text"
              : "text-text-2 hover:bg-text/5"
          }`}
        >
          {t(`meeting.tabs.${option}`)}
        </button>
      ))}
    </div>
  );
}

export interface PeopleViewProps {
  /** As `meeting_directory` returns them: latest meeting first; `null` while they load. */
  people: MeetingPerson[] | null;
  /** The page's search box. */
  query: string;
  onOpen: (person: MeetingPerson) => void;
  locale: string;
  t: Translate;
  /** "Today" for the dates; the tests pin it. */
  now?: Date;
}

/** "Pessoas": Person · Last meeting · Meetings, filtered by the search box. */
export function PeopleView({
  people,
  query,
  onOpen,
  locale,
  t,
  now = new Date(),
}: PeopleViewProps) {
  if (people === null) return null;
  if (people.length === 0) {
    return (
      <div className="rounded-lg border border-dashed border-border px-6 py-10 text-center">
        <p className="text-body text-text-2">{t("meeting.directory.empty")}</p>
      </div>
    );
  }
  const shown = filterPeople(people, query);
  if (shown.length === 0) {
    return (
      <p className="py-4 text-body text-text-2">
        {t("meeting.directory.noResults")}
      </p>
    );
  }
  const head = "px-4 py-2 text-start text-caption font-semibold text-text-2";
  return (
    <table className="w-full overflow-hidden rounded-lg border border-border bg-surface-1">
      <thead className="border-b border-border">
        <tr>
          <th scope="col" className={head}>
            {t("meeting.directory.name")}
          </th>
          <th scope="col" className={head}>
            {t("meeting.directory.last")}
          </th>
          <th scope="col" className={`${head} text-end`}>
            {t("meeting.directory.count")}
          </th>
        </tr>
      </thead>
      <tbody className="divide-y divide-border">
        {shown.map((person) => (
          <tr key={person.key} className="hover:bg-surface-2">
            <td className="max-w-0 w-1/2">
              <button
                type="button"
                onClick={() => onOpen(person)}
                className="block w-full truncate px-4 py-2.5 text-start text-body focus-visible:focus-ring"
              >
                {person.name}
              </button>
            </td>
            <td className="whitespace-nowrap px-4 text-caption text-text-2">
              {dayLabel(new Date(person.last_meeting_at), now, locale, t)}
            </td>
            <td className="px-4 text-end text-caption text-text-2 tabular-nums">
              {person.meetings}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
