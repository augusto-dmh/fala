import type { MeetingLine } from "@/bindings";
import { clock, speakerOf, type Translate } from "./meetingModel";
import type { NoteSource } from "./notesModel";
import { PlayButton } from "./TranscriptView";

export interface SourcePopoverProps {
  sources: NoteSource[];
  segments: Map<number, MeetingLine>;
  annotations: Map<number, string>;
  audioAvailable: boolean;
  playing: number | null;
  onPlay: (line: MeetingLine) => void;
  /** Scrolls the transcript to the segment and highlights it. */
  onReveal: (seq: number) => void;
  onClose: () => void;
  t: Translate;
}

/**
 * Where a generated line came from, read from the pointers stored with the notes: no LLM call
 * and no network (ADR-0016).
 */
export function SourcePopover(props: SourcePopoverProps) {
  const { t } = props;
  return (
    <div
      role="dialog"
      aria-label={t("meeting.source.title")}
      className="absolute top-full right-0 z-10 mt-1 max-h-[28rem] w-80 space-y-3 overflow-y-auto rounded-lg border border-border bg-surface-2 p-3 shadow-lg"
      onKeyDown={(event) => {
        if (event.key === "Escape") props.onClose();
      }}
    >
      <p className="text-caption font-semibold tracking-wide text-text-2 uppercase">
        {t("meeting.source.title")}
      </p>
      {props.sources.length === 0 ? (
        <p className="text-sm text-text-2">{t("meeting.source.none")}</p>
      ) : (
        <ul className="space-y-3">
          {props.sources.map((source) => {
            if (source.kind === "annotation") {
              return (
                <li key={`a${source.n}`} className="space-y-0.5">
                  <span className="text-caption text-text-2">
                    {t("meeting.source.typed")}
                  </span>
                  <p className="text-sm text-text">
                    {props.annotations.get(source.n)}
                  </p>
                </li>
              );
            }
            const line = props.segments.get(source.seq);
            if (!line) return null;
            return (
              <li key={`s${source.seq}`} className="space-y-0.5">
                <div className="flex items-center gap-2 text-caption text-text-2">
                  <span>{speakerOf(line, t)}</span>
                  <button
                    type="button"
                    className="cursor-pointer underline hover:text-text focus-visible:focus-ring"
                    aria-label={t("meeting.source.goTo", {
                      time: clock(line.t0_ms),
                    })}
                    onClick={() => props.onReveal(line.seq)}
                  >
                    {clock(line.t0_ms)}
                  </button>
                  <PlayButton
                    playing={props.playing === line.seq}
                    audioAvailable={props.audioAvailable}
                    onClick={() => props.onPlay(line)}
                    t={t}
                  />
                </div>
                <p className="text-sm text-text">{line.text}</p>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
