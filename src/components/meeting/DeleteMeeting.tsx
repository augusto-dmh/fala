import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../ui/Button";
import type { Translate } from "./meetingModel";

export interface DeleteConfirmProps {
  onConfirm: () => void;
  onCancel: () => void;
  t: Translate;
}

/** The literal confirmation: what goes, and that it does not come back (ADR-0005). */
export function DeleteConfirm({ onConfirm, onCancel, t }: DeleteConfirmProps) {
  return (
    <div
      role="alertdialog"
      aria-label={t("meeting.delete.title")}
      className="space-y-2 rounded-lg border border-danger/40 p-3"
    >
      <p className="text-sm font-semibold">{t("meeting.delete.title")}</p>
      <p className="text-sm">{t("meeting.delete.body")}</p>
      <div className="flex gap-2">
        <Button variant="danger" onClick={onConfirm}>
          {t("meeting.delete.confirm")}
        </Button>
        <Button variant="secondary" onClick={onCancel}>
          {t("meeting.delete.cancel")}
        </Button>
      </div>
    </div>
  );
}

export interface DeleteMeetingProps {
  /** Recording, transcribing or generating notes: nothing to delete yet. */
  disabled: boolean;
  onDelete: () => void;
}

/** "Delete meeting", which asks first. */
export function DeleteMeeting({ disabled, onDelete }: DeleteMeetingProps) {
  const { t } = useTranslation();
  const [confirming, setConfirming] = useState(false);
  // A job that starts meanwhile closes the question; it never comes back by itself.
  useEffect(() => {
    if (disabled) setConfirming(false);
  }, [disabled]);
  if (confirming && !disabled) {
    return (
      <div className="px-4 pb-4">
        <DeleteConfirm
          onConfirm={() => {
            setConfirming(false);
            onDelete();
          }}
          onCancel={() => setConfirming(false)}
          t={t}
        />
      </div>
    );
  }
  return (
    <div className="px-4 pb-4">
      <Button
        variant="danger-ghost"
        disabled={disabled}
        onClick={() => setConfirming(true)}
      >
        {t("meeting.delete.action")}
      </Button>
    </div>
  );
}
