import { Play, Square } from "lucide-react";
import type { MeetingLine } from "@/bindings";
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
                {speakerOf(line, t)} · {clock(line.t0_ms)}
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
