import { ChevronDown } from "lucide-react";
import { useEffect, useRef, useState, type ReactNode } from "react";
import type { MeetingStatus } from "@/bindings";
import { Button } from "../ui/Button";
import { clock, type Translate } from "./meetingModel";

export interface NewMeetingButtonProps {
  disabled: boolean;
  onStart: () => void;
  /** The recorder's choices (mode, sound, title, draft, notice), behind the arrow. */
  options: ReactNode;
  t: Translate;
}

/** "● Nova reunião", and the arrow that opens the recording options. */
export function NewMeetingButton(props: NewMeetingButtonProps) {
  const { t } = props;
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement>(null);
  // A recording that starts (here or from the tray) closes the options.
  useEffect(() => {
    if (props.disabled) setOpen(false);
  }, [props.disabled]);
  useEffect(() => {
    if (!open) return;
    const outside = (event: MouseEvent) => {
      if (!box.current?.contains(event.target as Node)) setOpen(false);
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", outside);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("mousedown", outside);
      document.removeEventListener("keydown", escape);
    };
  }, [open]);

  return (
    <div ref={box} className="relative flex">
      <Button
        className="flex items-center gap-2 rounded-e-none"
        disabled={props.disabled}
        onClick={props.onStart}
      >
        <span className="size-2 rounded-full bg-rec" aria-hidden="true" />
        {t("meeting.home.new")}
      </Button>
      <Button
        className="rounded-s-none border-s-on-accent/30 px-2"
        aria-label={t("meeting.home.options")}
        aria-expanded={open}
        disabled={props.disabled}
        onClick={() => setOpen(!open)}
      >
        <ChevronDown size={16} strokeWidth={1.5} aria-hidden="true" />
      </Button>
      {open && (
        <div className="absolute end-0 top-full z-20 mt-1 w-[min(36rem,calc(100vw-6rem))] rounded-xl border border-border bg-surface-1 shadow-lg">
          {props.options}
        </div>
      )}
    </div>
  );
}

export interface RecordingStripProps {
  status: MeetingStatus;
  title: string;
  onOpen: () => void;
  t: Translate;
}

/** The meeting recording now, on top of the list: the time, a warning if any, and "Abrir". */
export function RecordingStrip({
  status,
  title,
  onOpen,
  t,
}: RecordingStripProps) {
  if (status.state !== "recording" && status.state !== "paused") return null;
  const attention =
    status.muted_mic ||
    status.muted_system ||
    status.call_output !== null ||
    status.cap_remaining_ms !== null ||
    status.low_disk_bytes !== null;
  return (
    <div className="sticky top-0 z-10 flex items-center gap-3 rounded-lg border border-border bg-surface-1 px-4 py-2">
      <span
        className="size-2 shrink-0 rounded-full bg-rec"
        aria-hidden="true"
      />
      {/* Only the state is a live region: the clock changes four times a second. */}
      <span role="status" className="min-w-0 flex-1 truncate text-body">
        {status.state === "paused"
          ? t("meeting.home.paused", { title })
          : t("meeting.home.recording", { title })}
      </span>
      {attention && (
        <span className="text-caption text-warn">
          {t("meeting.home.attention")}
        </span>
      )}
      <span className="text-body tabular-nums">
        {clock(status.recorded_ms)}
      </span>
      <Button size="sm" variant="secondary" onClick={onOpen}>
        {t("meeting.home.open")}
      </Button>
    </div>
  );
}
