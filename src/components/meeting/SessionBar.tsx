import { AudioLines, ChevronDown } from "lucide-react";
import type { ReactNode } from "react";
import type { MeetingStatus } from "@/bindings";
import { Button } from "../ui/Button";
import { clock, type Translate } from "./meetingModel";
import { useMenuPopover } from "./TemplateSelect";

/** The recording of this session, while it records, stops or is processed. */
export interface LiveControls {
  status: MeetingStatus;
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
}

export interface SessionBarProps {
  draft: boolean;
  live: LiveControls | null;
  transcriptOpen: boolean;
  onToggleTranscript: () => void;
  onRecordDraft: () => void;
  transcribing: boolean;
  transcribeDisabled: boolean;
  /** Why "Transcrever" is off, when it is for lack of something the person can fix. */
  transcribeHint?: string;
  onTranscribe: () => void;
  onCancelTranscription: () => void;
  /** "Gerar notas", or what the notes run is doing. */
  generateLabel: string;
  generateDisabled: boolean;
  /** Why "Gerar notas" is off, when it is for lack of something the person can fix. */
  generateHint?: string;
  onGenerate: () => void;
  /** The notes template picker next to "Gravar" in a draft. */
  templateSelect: ReactNode;
  /** The templates as menu items behind the arrow of "Gerar notas"; `close` shuts the menu
   *  after a pick. */
  templateMenu: (close: () => void) => ReactNode;
  t: Translate;
}

/** "Gerar notas" with the templates as the items of its arrow's menu. */
function GenerateButton(props: SessionBarProps) {
  const { open, setOpen, close, box } = useMenuPopover();
  return (
    <div ref={box} className="relative flex">
      <Button
        className="rounded-e-none"
        disabled={props.generateDisabled}
        title={props.generateHint}
        onClick={props.onGenerate}
      >
        {props.generateLabel}
      </Button>
      <Button
        className="rounded-s-none border-s-on-accent/30 px-2"
        aria-label={props.t("meeting.view.template")}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        <ChevronDown size={16} strokeWidth={1.5} aria-hidden="true" />
      </Button>
      {open && (
        <div className="absolute end-0 bottom-full z-20 mb-2 rounded-xl border border-border bg-surface-1 p-1 shadow-lg">
          {props.templateMenu(close)}
        </div>
      )}
    </div>
  );
}

/**
 * The bar at the bottom of a meeting (Granola's pill): the transcript on the left, the room for
 * "Ask about this meeting" in the middle, and on the right what the session can do now.
 */
export function SessionBar(props: SessionBarProps) {
  const { t, live } = props;
  const recording =
    live?.status.state === "recording" || live?.status.state === "paused";

  let actions: ReactNode;
  if (live && recording) {
    const paused = live.status.state === "paused";
    actions = (
      <>
        <span className="size-2 rounded-full bg-rec" aria-hidden="true" />
        <span className="text-body tabular-nums">
          {clock(live.status.recorded_ms)}
        </span>
        <Button
          variant="secondary"
          onClick={paused ? live.onResume : live.onPause}
        >
          {paused ? t("meeting.recorder.resume") : t("meeting.recorder.pause")}
        </Button>
        <Button variant="danger" onClick={live.onStop}>
          {t("meeting.recorder.stop")}
        </Button>
      </>
    );
  } else if (live) {
    actions = (
      <span role="status" className="text-caption text-text-2">
        {t("meeting.recorder.processing")}
      </span>
    );
  } else if (props.draft) {
    actions = (
      <>
        {props.templateSelect}
        <Button
          className="flex items-center gap-2"
          onClick={props.onRecordDraft}
        >
          <span className="size-2 rounded-full bg-rec" aria-hidden="true" />
          {t("meeting.view.recordDraft")}
        </Button>
      </>
    );
  } else {
    actions = (
      <>
        {props.transcribing && (
          <Button variant="ghost" onClick={props.onCancelTranscription}>
            {t("meeting.view.cancel")}
          </Button>
        )}
        <Button
          variant="secondary"
          disabled={props.transcribeDisabled}
          title={props.transcribeHint}
          onClick={props.onTranscribe}
        >
          {props.transcribing
            ? t("meeting.view.transcribing")
            : t("meeting.view.transcribe")}
        </Button>
        <GenerateButton {...props} />
      </>
    );
  }

  return (
    <div
      role="toolbar"
      aria-label={t("meeting.bar.label")}
      className="sticky bottom-4 z-40 flex min-h-14 items-center gap-2 rounded-full border border-border bg-surface-1 px-3 py-2 shadow-lg"
    >
      {!props.draft && (
        <Button
          variant="ghost"
          className="rounded-full px-2"
          aria-label={t("meeting.view.transcript")}
          title={t("meeting.view.transcript")}
          aria-pressed={props.transcriptOpen}
          onClick={props.onToggleTranscript}
        >
          <AudioLines size={20} strokeWidth={1.5} aria-hidden="true" />
        </Button>
      )}
      {/* Kept for "Ask about this meeting" (meeting-chat), which takes this space later. Until
          then it says why a button is off: a disabled button takes no focus, so a tooltip
          alone would not reach the keyboard or a screen reader. */}
      <div className="min-w-0 flex-1 text-caption text-text-2">
        {!live && !props.draft && (props.transcribeHint ?? props.generateHint)}
      </div>
      {actions}
    </div>
  );
}
