import { MoreHorizontal } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { commands, type MeetingSummary } from "@/bindings";
import { useDismiss } from "../../hooks/useDismiss";
import { Button } from "../ui/Button";
import { dayLabel, startOf, startTime } from "./meetingDays";
import { clock, type Translate } from "./meetingModel";

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
  const item = (
    label: string,
    onClick: () => void,
    disabled = false,
    danger = false,
  ) => (
    <button
      type="button"
      role="menuitem"
      disabled={disabled}
      className={`block w-full rounded-md px-3 py-1.5 text-start text-body transition-colors hover:bg-surface-2 focus-visible:focus-ring disabled:cursor-not-allowed disabled:opacity-50 ${
        danger ? "text-danger" : ""
      }`}
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
      {/* Apart and in red, like "Delete entry" in the history's menu. */}
      <div role="separator" className="my-1 border-t border-border" />
      {item(
        t("meeting.trash.action"),
        props.onDelete,
        props.deleteDisabled,
        true,
      )}
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
