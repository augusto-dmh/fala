import { ChevronLeft, RotateCcw, Trash2 } from "lucide-react";
import { useState } from "react";
import type { TrashedMeetingSummary } from "@/bindings";
import { Button } from "../ui/Button";
import { DeleteConfirm } from "./DeleteMeeting";
import { dayLabel, startOf } from "./meetingDays";
import { titleOf, type Translate } from "./meetingModel";

/** Calendar days since the session went to the trash, like the dates beside it; never negative. */
export function daysInTrash(trashedAt: string, now: Date): number {
  const day = (date: Date) =>
    Date.UTC(date.getFullYear(), date.getMonth(), date.getDate());
  const days = (day(now) - day(new Date(trashedAt))) / 86_400_000;
  return Math.max(0, Math.round(days));
}

/** "apagada hoje", "apagada há 1 dia", "apagada há N dias". */
export const sinceLabel = (days: number, t: Translate): string =>
  days === 0
    ? t("meeting.trash.today")
    : t("meeting.trash.since", { count: days });

export interface TrashViewProps {
  trashed: TrashedMeetingSummary[];
  onBack: () => void;
  onRestore: (id: string) => void;
  /** After the confirmation: transcript, annotations and notes, for good. */
  onDelete: (id: string) => void;
  locale: string;
  t: Translate;
  /** "Today" for the dates; the tests pin it. */
  now?: Date;
}

/** "Lixeira": each session with Restore and Delete forever, and the 30-day rule below. */
export function TrashView({
  trashed,
  onBack,
  onRestore,
  onDelete,
  locale,
  t,
  now = new Date(),
}: TrashViewProps) {
  const [confirming, setConfirming] = useState<string | null>(null);
  return (
    <div className="space-y-4">
      <button
        type="button"
        onClick={onBack}
        className="flex items-center gap-1 rounded-lg text-body text-text-2 transition-colors hover:text-text focus-visible:focus-ring"
      >
        <ChevronLeft
          size={16}
          strokeWidth={1.5}
          className="rtl:rotate-180"
          aria-hidden="true"
        />
        {t("meeting.list.title")}
      </button>
      <h1 className="font-display text-title font-semibold">
        {t("meeting.trash.title")}
      </h1>
      {trashed.length === 0 ? (
        <p className="py-4 text-body text-text-2">{t("meeting.trash.empty")}</p>
      ) : (
        <ul className="divide-y divide-border rounded-lg border border-border bg-surface-1">
          {trashed.map(({ summary, trashed_at }) => (
            <li key={summary.id} className="space-y-2 px-4 py-3">
              <div className="flex items-center gap-3">
                <div className="min-w-0 flex-1">
                  <p className="truncate text-body font-medium">
                    {titleOf(summary, t)}
                  </p>
                  <p className="text-caption text-text-2">
                    {dayLabel(startOf(summary), now, locale, t)}
                    {" · "}
                    {sinceLabel(daysInTrash(trashed_at, now), t)}
                  </p>
                </div>
                <Button
                  size="sm"
                  variant="secondary"
                  className="flex items-center gap-1"
                  onClick={() => onRestore(summary.id)}
                >
                  <RotateCcw size={14} strokeWidth={1.5} aria-hidden="true" />
                  {t("meeting.trash.restore")}
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  className="flex items-center gap-1"
                  onClick={() => setConfirming(summary.id)}
                >
                  <Trash2 size={14} strokeWidth={1.5} aria-hidden="true" />
                  {t("meeting.trash.deleteForever")}
                </Button>
              </div>
              {confirming === summary.id && (
                <DeleteConfirm
                  onConfirm={() => {
                    setConfirming(null);
                    onDelete(summary.id);
                  }}
                  onCancel={() => setConfirming(null)}
                  t={t}
                />
              )}
            </li>
          ))}
        </ul>
      )}
      <p className="text-caption text-text-2">{t("meeting.trash.footer")}</p>
    </div>
  );
}
