import { MoreHorizontal } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { commands, type MeetingSeries, type MeetingSummary } from "@/bindings";
import { useDismiss } from "../../hooks/useDismiss";
import { Button } from "../ui/Button";
import { dayLabel, startOf, startTime } from "./meetingDays";
import { clock, titleOf, type Translate } from "./meetingModel";

const MODE_KEYS: Record<string, string> = {
  meeting: "meeting.recorder.online",
  in_person: "meeting.recorder.inPerson",
  system_only: "meeting.recorder.systemOnly",
  import: "meeting.screen.imported",
};

/** "Hoje · 14:02 · 37:30 · Online": when, how long and how the session was recorded. */
export function metaLine(
  summary: MeetingSummary,
  now: Date,
  locale: string,
  t: Translate,
  recording = false,
): string {
  const parts = [
    dayLabel(startOf(summary), now, locale, t),
    startTime(summary, locale),
    // While it records the stored duration is stale; the live time is in the recorder.
    recording
      ? t("meeting.home.state.recording")
      : summary.started_at === null
        ? t("meeting.list.draft")
        : clock(summary.recorded_ms),
  ];
  const mode = MODE_KEYS[summary.mode];
  if (mode) parts.push(t(mode));
  return parts.join(" · ");
}

export interface SeriesLineProps {
  series?: MeetingSeries | null;
  onOpen: (id: string) => void;
  /** "ver todas (N)": the list filtered by the series. */
  onAll: (series: MeetingSeries) => void;
  locale: string;
  t: Translate;
  /** "Today" for the dates; the tests pin it. */
  now?: Date;
}

/** "Mesma série:" and the meetings before this one, by day; "ver todas (N)" when there are more. */
export function SeriesLine({
  series,
  onOpen,
  onAll,
  locale,
  t,
  now = new Date(),
}: SeriesLineProps) {
  if (!series) return null;
  const more = series.total > series.earlier.length + 1;
  const days = series.earlier.map((s) => dayLabel(startOf(s), now, locale, t));
  // Two on the same day are told apart by the time.
  const label = (session: MeetingSummary, i: number) =>
    days.indexOf(days[i]) === days.lastIndexOf(days[i])
      ? days[i]
      : `${days[i]} ${startTime(session, locale)}`;
  const link =
    "rounded underline decoration-dotted underline-offset-2 hover:text-text focus-visible:focus-ring";
  return (
    <p className="flex flex-wrap items-center gap-x-2 text-caption text-text-2">
      <span>{t("meeting.series.same")}</span>
      {series.earlier.map((session, i) => (
        <button
          key={session.id}
          type="button"
          className={link}
          title={titleOf(session, t)}
          onClick={() => onOpen(session.id)}
        >
          {label(session, i)}
        </button>
      ))}
      {more && (
        <button type="button" className={link} onClick={() => onAll(series)}>
          {t("meeting.series.all", { n: series.total })}
        </button>
      )}
    </p>
  );
}

export interface TitleFieldProps {
  id: string;
  title: string;
  /** "Reunião de …", shown while the title is empty. */
  fallback: string;
  onSaved: () => void;
  t: Translate;
}

/** The session's title, edited in place: saved on Enter or when the field loses focus. */
export function TitleField({
  id,
  title,
  fallback,
  onSaved,
  t,
}: TitleFieldProps) {
  const [text, setText] = useState(title);
  const [failed, setFailed] = useState(false);
  useEffect(() => setText(title), [title]);
  const save = async () => {
    if (text.trim() === title.trim()) return;
    const result = await commands.setMeetingTitle(id, text.trim());
    setFailed(result.status === "error");
    if (result.status === "ok") onSaved();
    else setText(title);
  };
  return (
    <h1 className="min-w-0 flex-1">
      <input
        aria-label={t("meeting.screen.title")}
        className="w-full rounded-md bg-transparent font-display text-title font-semibold text-text placeholder:text-text-3 focus:outline-none focus-visible:focus-ring"
        value={text}
        placeholder={fallback}
        onChange={(event) => setText(event.target.value)}
        onBlur={() => void save()}
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
          if (event.key === "Escape") setText(title);
        }}
      />
      {failed && (
        <span role="alert" className="block text-caption text-danger">
          {t("meeting.screen.titleFailed")}
        </span>
      )}
    </h1>
  );
}

export interface SessionMenuProps {
  /** The session has a transcript to copy. */
  transcript: boolean;
  /** Recording, transcribing or writing notes: nothing to delete yet. */
  deleteDisabled: boolean;
  onCopy: () => void;
  onCopyTranscript: () => void;
  onPeople: () => void;
  onDelete: () => void;
  t: Translate;
}

/** What "⋯" offers: copy the Markdown or the transcript, the participants, the trash. */
export function SessionMenuItems(props: SessionMenuProps) {
  const { t } = props;
  const item = (label: string, onClick: () => void, disabled = false) => (
    <button
      type="button"
      role="menuitem"
      disabled={disabled}
      className="block w-full rounded-md px-3 py-1.5 text-start text-body transition-colors hover:bg-surface-2 focus-visible:focus-ring disabled:cursor-not-allowed disabled:opacity-50"
      onClick={onClick}
    >
      {label}
    </button>
  );
  return (
    <div role="menu" className="p-1">
      {item(t("meeting.view.copy"), props.onCopy)}
      {props.transcript &&
        item(t("meeting.view.copyTranscript"), props.onCopyTranscript)}
      {item(t("meeting.people.participants"), props.onPeople)}
      {item(t("meeting.trash.action"), props.onDelete, props.deleteDisabled)}
    </div>
  );
}

/** The "⋯" button and its menu; picking an item closes it. */
export function SessionMenu(props: SessionMenuProps) {
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement>(null);
  const close = useCallback(() => setOpen(false), []);
  useDismiss(open, close, box);
  const then = (action: () => void) => () => {
    close();
    action();
  };
  return (
    <div ref={box} className="relative shrink-0">
      <Button
        variant="ghost"
        className="px-2"
        aria-label={props.t("meeting.screen.more")}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        <MoreHorizontal size={20} strokeWidth={1.5} aria-hidden="true" />
      </Button>
      {open && (
        <div className="absolute end-0 top-full z-20 mt-1 w-56 rounded-xl border border-border bg-surface-1 shadow-lg">
          <SessionMenuItems
            {...props}
            onCopy={then(props.onCopy)}
            onCopyTranscript={then(props.onCopyTranscript)}
            onPeople={then(props.onPeople)}
            onDelete={then(props.onDelete)}
          />
        </div>
      )}
    </div>
  );
}
