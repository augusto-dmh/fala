import type { MeetingOrphan } from "@/bindings";
import { Button } from "../ui/Button";
import type { Translate } from "./meetingModel";

/** `HH:MM` where the recording started, in the offset it was recorded in. */
export const orphanTime = (orphan: MeetingOrphan): string =>
  orphan.started_at.slice(11, 16);

/** Whole minutes, at least 1: a few seconds still read as a recording. */
export const orphanMinutes = (orphan: MeetingOrphan): number =>
  Math.max(1, Math.round(orphan.recorded_ms / 60_000));

export interface RecoverBannerProps {
  orphans: MeetingOrphan[];
  /** A recovery is running: one click at a time. */
  busy?: boolean;
  onRecover: (id: string) => void;
  t: Translate;
}

/** "Recover the recording of 14:02 (12 min)?" for each recording the app closed on. */
export function RecoverBanner({
  orphans,
  busy = false,
  onRecover,
  t,
}: RecoverBannerProps) {
  if (orphans.length === 0) return null;
  return (
    <div className="space-y-2 p-4">
      {orphans.map((orphan) => (
        <div
          key={orphan.id}
          role="alert"
          className="flex flex-wrap items-center gap-3 rounded-lg border border-border p-3"
        >
          <div className="min-w-0 flex-1">
            <p className="text-sm font-semibold">
              {t("meeting.recover.title", {
                time: orphanTime(orphan),
                minutes: orphanMinutes(orphan),
              })}
            </p>
            <p className="text-caption text-text-2">
              {t("meeting.recover.body")}
            </p>
          </div>
          <Button disabled={busy} onClick={() => onRecover(orphan.id)}>
            {t("meeting.recover.action")}
          </Button>
        </div>
      ))}
    </div>
  );
}
