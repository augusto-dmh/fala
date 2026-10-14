import type { MeetingLine } from "@/bindings";
import { Button } from "../ui/Button";
import { clock, type Translate } from "./meetingModel";

/** The most lines one stretch takes (Granola's limit; the app refuses more too). */
export const ERASE_MAX_LINES = 500;

/** From the start of the first picked line to the end of the last, widened to the lines it
 *  takes, and those lines. */
export interface Stretch {
  t0_ms: number;
  t1_ms: number;
  /** Lines that leave, in both channels; the markers do not count. */
  lines: number;
  /** Their seqs, to mark them while picking. */
  taken: Set<number>;
}

const marker = (line: MeetingLine) =>
  line.omitted || line.erased || line.resumed;

/** Whether erasing `[t0, t1]` takes the line: it crosses it (touching an edge is not enough) or
 *  fits in it. The same rule as the app's `MeetingSegment::crosses`. */
export const crosses = (line: MeetingLine, t0: number, t1: number) =>
  (line.t0_ms < t1 && line.t1_ms > t0) ||
  (line.t0_ms >= t0 && line.t1_ms <= t1);

/** The stretch between two picked lines, in either order; `null` until both are picked. A line
 *  that crosses an edge leaves whole, so the stretch grows to cover it, as the app does. */
export function stretchOf(
  lines: MeetingLine[],
  picked: number[],
): Stretch | null {
  const chosen = lines.filter((line) => picked.includes(line.seq));
  if (picked.length < 2 || chosen.length < 2) return null;
  let t0_ms = Math.min(...chosen.map((line) => line.t0_ms));
  let t1_ms = Math.max(...chosen.map((line) => line.t1_ms));
  const spoken = lines.filter((line) => !marker(line));
  for (;;) {
    const taken = spoken.filter((line) => crosses(line, t0_ms, t1_ms));
    const from = Math.min(t0_ms, ...taken.map((line) => line.t0_ms));
    const to = Math.max(t1_ms, ...taken.map((line) => line.t1_ms));
    if (from === t0_ms && to === t1_ms) {
      return {
        t0_ms,
        t1_ms,
        lines: taken.length,
        taken: new Set(taken.map((line) => line.seq)),
      };
    }
    [t0_ms, t1_ms] = [from, to];
  }
}

export interface EraseBarProps {
  picked: number;
  stretch: Stretch | null;
  onErase: () => void;
  onCancel: () => void;
  t: Translate;
}

/** While picking: what to click, how many lines go, [Erase stretch] and [Cancel]. */
export function EraseBar({
  picked,
  stretch,
  onErase,
  onCancel,
  t,
}: EraseBarProps) {
  const tooMany = stretch !== null && stretch.lines > ERASE_MAX_LINES;
  return (
    <div
      role="status"
      className="space-y-2 rounded-lg border border-danger/40 p-3 text-sm"
    >
      <p>
        {stretch === null
          ? t(
              picked === 0
                ? "meeting.erase.pickFirst"
                : "meeting.erase.pickLast",
            )
          : tooMany
            ? t("meeting.erase.tooMany", { max: ERASE_MAX_LINES })
            : t("meeting.erase.picked", {
                count: stretch.lines,
                from: clock(stretch.t0_ms),
                to: clock(stretch.t1_ms),
              })}
      </p>
      <div className="flex gap-2">
        <Button
          variant="danger"
          size="sm"
          disabled={stretch === null || stretch.lines === 0 || tooMany}
          onClick={onErase}
        >
          {t("meeting.erase.action")}
        </Button>
        <Button variant="secondary" size="sm" onClick={onCancel}>
          {t("meeting.view.cancel")}
        </Button>
      </div>
    </div>
  );
}

export interface EraseConfirmProps {
  stretch: Stretch;
  /** The session still has audio, which goes too. */
  audio: boolean;
  onConfirm: () => void;
  onCancel: () => void;
  t: Translate;
}

/** "Apagar este trecho?": the times, both channels, how many lines, and that it is for good. */
export function EraseConfirm({
  stretch,
  audio,
  onConfirm,
  onCancel,
  t,
}: EraseConfirmProps) {
  const values = {
    count: stretch.lines,
    from: clock(stretch.t0_ms),
    to: clock(stretch.t1_ms),
  };
  return (
    <div
      role="alertdialog"
      aria-label={t("meeting.erase.title")}
      className="space-y-2 rounded-lg border border-danger/40 p-3"
    >
      <p className="text-sm font-semibold">{t("meeting.erase.title")}</p>
      <p className="text-sm">
        {t(audio ? "meeting.erase.body" : "meeting.erase.bodyNoAudio", values)}
      </p>
      <div className="flex gap-2">
        <Button variant="danger" onClick={onConfirm}>
          {t("meeting.erase.action")}
        </Button>
        <Button variant="secondary" onClick={onCancel}>
          {t("meeting.view.cancel")}
        </Button>
      </div>
    </div>
  );
}
