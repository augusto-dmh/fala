import { Play, Square } from "lucide-react";
import { useRef, useState } from "react";
import type { MeetingLine } from "@/bindings";
import { Input } from "../ui/Input";
import { clock, speakerOf, type Translate } from "./meetingModel";

export interface PlayButtonProps {
  playing: boolean;
  audioAvailable: boolean;
  onClick: () => void;
  t: Translate;
}

/** ▶ of a segment; ■ while it plays; disabled when the audio is gone. */
export function PlayButton(props: PlayButtonProps) {
  const { t } = props;
  return (
    <button
      type="button"
      className="shrink-0 cursor-pointer rounded p-1 text-text-2 hover:text-text focus-visible:focus-ring disabled:cursor-not-allowed disabled:opacity-40"
      aria-label={
        props.playing ? t("meeting.source.stop") : t("meeting.source.play")
      }
      title={props.audioAvailable ? undefined : t("meeting.source.noAudio")}
      disabled={!props.audioAvailable}
      onClick={props.onClick}
    >
      {props.playing ? <Square size={12} /> : <Play size={12} />}
    </button>
  );
}

export interface SpeakerNameProps {
  line: MeetingLine;
  /** Absent: the label is plain text. */
  onRename?: (person: number, name: string) => void;
  t: Translate;
}

/**
 * The speaker of a turn. For "Person N", a click opens a field: Enter or leaving it applies the
 * name to the whole session, Escape keeps it, an empty field gives "Person N" back.
 */
export function SpeakerName({ line, onRename, t }: SpeakerNameProps) {
  const [typed, setTyped] = useState<string | null>(null);
  // Enter closes the field, and a blur may follow as it unmounts: one rename per edit.
  const open = useRef(false);
  const person = line.person;
  if (person === null || !onRename) return <>{speakerOf(line, t)}</>;
  const current = line.name ?? "";
  const label = t("meeting.speaker.person", { n: person });
  if (typed === null) {
    return (
      <button
        type="button"
        className="cursor-pointer rounded underline decoration-dotted underline-offset-2 hover:text-text focus-visible:focus-ring"
        title={t("meeting.rename.hint", { person: label })}
        onClick={() => {
          open.current = true;
          setTyped(current);
        }}
      >
        {speakerOf(line, t)}
      </button>
    );
  }
  const close = (save: boolean) => {
    if (!open.current) return;
    open.current = false;
    setTyped(null);
    // The app collapses spaces the same way: an edit that changes nothing is not a rename.
    if (save && typed.trim().replace(/\s+/g, " ") !== current) {
      onRename(person, typed);
    }
  };
  return (
    <Input
      variant="compact"
      autoFocus
      aria-label={t("meeting.rename.label", { person: label })}
      placeholder={label}
      value={typed}
      onChange={(event) => setTyped(event.target.value)}
      onBlur={() => close(true)}
      onKeyDown={(event) => {
        // Enter that confirms an IME composition is not the end of the name.
        if (event.key === "Enter" && !event.nativeEvent.isComposing)
          close(true);
        if (event.key === "Escape") close(false);
      }}
    />
  );
}

/** The DOM id of a segment bubble, the target of a source's `mm:ss`. */
export const segmentAnchor = (seq: number) => `segment-${seq}`;

export interface TranscriptViewProps {
  lines: MeetingLine[];
  audioAvailable: boolean;
  /** `seq` of the segment playing, if any. */
  playing: number | null;
  /** `seq` the note sources last pointed to. */
  highlighted: number | null;
  onPlay: (line: MeetingLine) => void;
  /** A name typed for "Person N"; absent, the labels are plain text. */
  onRename?: (person: number, name: string) => void;
  t: Translate;
}

/**
 * The transcript as a conversation: "me" on the right, the others on the left, the speaker and
 * time only when the speaker changes, a ▶ on each bubble.
 */
export function TranscriptView(props: TranscriptViewProps) {
  const { lines, t } = props;
  return (
    <ol className="space-y-1">
      {lines.map((line, index) => {
        const me = line.person === null;
        const turn = index === 0 || lines[index - 1].person !== line.person;
        const highlighted = props.highlighted === line.seq;
        return (
          <li
            key={line.seq}
            id={segmentAnchor(line.seq)}
            data-speaker={me ? "me" : "them"}
            className={`flex flex-col ${me ? "items-end" : "items-start"} ${turn && index > 0 ? "pt-2" : ""}`}
          >
            {turn && (
              <span className="px-1 text-caption text-text-2">
                <SpeakerName line={line} onRename={props.onRename} t={t} /> ·{" "}
                {clock(line.t0_ms)}
              </span>
            )}
            <div
              className={`flex max-w-[85%] items-start gap-1 rounded-lg px-2 py-1 text-sm text-text ${me ? "flex-row-reverse bg-me/15" : "bg-them/15"} ${highlighted ? "ring-2 ring-accent" : ""}`}
            >
              <PlayButton
                playing={props.playing === line.seq}
                audioAvailable={props.audioAvailable}
                onClick={() => props.onPlay(line)}
                t={t}
              />
              <span>{line.text}</span>
            </div>
          </li>
        );
      })}
    </ol>
  );
}
