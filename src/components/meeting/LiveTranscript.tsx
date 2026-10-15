import {
  type RefObject,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import {
  commands,
  events,
  type PreviewChannel,
  type PreviewLine,
} from "@/bindings";
import { Button } from "../ui/Button";
import { atEnd, emptyLive, type LiveState, mergeLive } from "./liveModel";
import { clock, type Translate } from "./meetingModel";

/**
 * The live preview of `id` (ADR-0019): what the app has so far, then each line as the model
 * gives it, until the app says the transcript of record replaced it.
 */
export function useLivePreview(id: string): LiveState {
  const [live, setLive] = useState<LiveState>(emptyLive);
  const shown = useRef(id);
  useEffect(() => {
    let alive = true;
    // Another meeting: nothing of the last one stays while its preview loads.
    if (shown.current !== id) setLive(emptyLive);
    shown.current = id;
    const unlisten = events.meetingPreview.listen((event) => {
      if (alive && event.payload.id === id)
        setLive((s) => mergeLive(s, event.payload));
    });
    // Listening first, then the snapshot merged in: no line falls between the two.
    void unlisten
      .then(() => commands.meetingPreview(id))
      .then((r) => {
        if (alive && r.status === "ok") setLive((s) => mergeLive(s, r.data));
      });
    return () => {
      alive = false;
      void unlisten.then((stop) => stop());
    };
  }, [id]);
  return live;
}

/** "You" (in person, "Room") for the mic, "Them" for the system. */
export function channelLabel(
  channel: PreviewChannel,
  mode: string,
  t: Translate,
) {
  if (channel === "system") return t("meeting.live.them");
  return t(mode === "in_person" ? "meeting.live.room" : "meeting.live.you");
}

/** One line for the bar while recording: the last thing said, or that someone is speaking. */
export function liveTicker(live: LiveState, mode: string, t: Translate) {
  const last = live.lines[live.lines.length - 1] as PreviewLine | undefined;
  if (last) return `${channelLabel(last.channel, mode, t)}: ${last.text}`;
  return live.mic || live.system ? t("meeting.live.listening") : null;
}

export interface LiveLinesProps {
  live: LiveState;
  mode: string;
  recording: boolean;
  t: Translate;
}

/**
 * The preview as a conversation, like the transcript (you on the right, them on the left), in
 * the muted colour of a draft, with "listening…" where speech has no text yet.
 */
export function LiveLines({ live, mode, recording, t }: LiveLinesProps) {
  const listening = (["mic", "system"] as const).filter((c) =>
    c === "mic" ? live.mic : live.system,
  );
  if (!recording && live.lines.length === 0) return null;
  return (
    <section aria-label={t("meeting.live.title")} className="space-y-2">
      <p className="text-caption text-text-2">
        {t(recording ? "meeting.live.note" : "meeting.live.stopped")}
      </p>
      {live.lines.length === 0 && listening.length === 0 && (
        <p className="text-sm text-text-2">{t("meeting.live.waiting")}</p>
      )}
      <ol aria-live="polite" className="space-y-1">
        {live.lines.map((line, index) => {
          const me = line.channel === "mic";
          const turn =
            index === 0 || live.lines[index - 1].channel !== line.channel;
          return (
            <li
              key={`${line.channel}:${line.t0_ms}`}
              data-channel={line.channel}
              className={`flex flex-col ${me ? "items-end" : "items-start"} ${turn && index > 0 ? "pt-2" : ""}`}
            >
              {turn && (
                <span className="px-1 text-caption text-text-2">
                  {channelLabel(line.channel, mode, t)} · {clock(line.t0_ms)}
                </span>
              )}
              <div
                className={`max-w-[85%] rounded-lg px-2 py-1 text-sm text-text-2 ${me ? "bg-me/10" : "bg-them/10"}`}
              >
                {line.text}
              </div>
            </li>
          );
        })}
        {listening.map((channel) => (
          <li
            key={`listening-${channel}`}
            data-channel={channel}
            className={`flex pt-2 ${channel === "mic" ? "justify-end" : "justify-start"} px-1 text-caption text-text-2 italic`}
          >
            {channelLabel(channel, mode, t)} · {t("meeting.live.listening")}
          </li>
        ))}
      </ol>
    </section>
  );
}

export interface LiveTranscriptProps extends LiveLinesProps {
  /** The element that scrolls (the transcript drawer). */
  scroller: RefObject<HTMLElement | null>;
}

/**
 * [`LiveLines`] that follow the newest line while the person is at the end of the list, and
 * stay put, with a button to jump back, once they scroll up to read.
 */
export function LiveTranscript(props: LiveTranscriptProps) {
  const { scroller, live, t } = props;
  const following = useRef(true);
  const [behind, setBehind] = useState(false);
  useEffect(() => {
    const element = scroller.current;
    if (!element) return;
    const onScroll = () => {
      following.current = atEnd(
        element.scrollTop,
        element.clientHeight,
        element.scrollHeight,
      );
      if (following.current) setBehind(false);
    };
    element.addEventListener("scroll", onScroll);
    return () => element.removeEventListener("scroll", onScroll);
  }, [scroller]);
  // A line that takes the place of its "listening…" changes this even when the count does not.
  const shape = `${live.lines.length}:${live.mic}:${live.system}`;
  useLayoutEffect(() => {
    const element = scroller.current;
    if (!element || shape === "0:false:false") return;
    if (following.current) element.scrollTop = element.scrollHeight;
    else setBehind(true);
  }, [shape, scroller]);
  return (
    <>
      <LiveLines {...props} />
      {behind && (
        <div className="sticky bottom-0 flex justify-center pt-2">
          <Button
            variant="secondary"
            size="sm"
            onClick={() => {
              const element = scroller.current;
              if (element) element.scrollTop = element.scrollHeight;
              following.current = true;
              setBehind(false);
            }}
          >
            {t("meeting.live.latest")}
          </Button>
        </div>
      )}
    </>
  );
}
