import type { MeetingDetail } from "@/bindings";
import { Button } from "../ui/Button";
import type { Translate } from "./meetingModel";

/** Moving to the trash asks only when there is audio, the one thing that does not come back. */
export const trashNeedsConfirm = (detail: MeetingDetail): boolean =>
  detail.audio_available || detail.summary.audio_retained;

export interface DeleteConfirmProps {
  onConfirm: () => void;
  onCancel: () => void;
  /** `trash`: moving a session with retained audio, which goes now (ADR-0005). */
  kind?: "delete" | "trash";
  t: Translate;
}

/** The literal confirmation: what goes, and what does not come back (ADR-0005). */
export function DeleteConfirm({
  onConfirm,
  onCancel,
  kind = "delete",
  t,
}: DeleteConfirmProps) {
  const keys = kind === "trash" ? "meeting.trash.confirm" : "meeting.delete";
  return (
    <div
      role="alertdialog"
      aria-label={t(`${keys}.title`)}
      className="space-y-2 rounded-lg border border-danger/40 p-3"
    >
      <p className="text-sm font-semibold">{t(`${keys}.title`)}</p>
      <p className="text-sm">{t(`${keys}.body`)}</p>
      <div className="flex gap-2">
        <Button variant="danger" onClick={onConfirm}>
          {t(`${keys}.confirm`)}
        </Button>
        <Button variant="secondary" onClick={onCancel}>
          {t("meeting.delete.cancel")}
        </Button>
      </div>
    </div>
  );
}
