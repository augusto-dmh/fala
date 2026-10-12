import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { useSettings } from "../../hooks/useSettings";
import { Button } from "../ui/Button";
import { Dropdown, type DropdownOption } from "../ui/Dropdown";
import { SettingContainer } from "../ui/SettingContainer";

type Translate = (key: string, options?: Record<string, unknown>) => string;

const KEEP = "keep";
/** The days offered; a stored number outside the list is shown too. */
export const RETENTION_DAYS = [7, 30, 90, 180, 365];
export const CAP_HOURS = [1, 2, 3, 4, 5, 6, 7, 8];

export function retentionOptions(
  stored: number | null,
  t: Translate,
): DropdownOption[] {
  const days =
    stored === null || RETENTION_DAYS.includes(stored)
      ? RETENTION_DAYS
      : [...RETENTION_DAYS, stored].sort((a, b) => a - b);
  return [
    { value: KEEP, label: t("settings.meetings.retention.keep") },
    ...days.map((n) => ({
      value: String(n),
      label: t("settings.meetings.retention.days", { days: n }),
    })),
  ];
}

export const retentionValue = (stored: number | null): string =>
  stored === null ? KEEP : String(stored);

export const retentionDays = (value: string): number | null =>
  value === KEEP ? null : Number(value);

/** Choosing "delete after N days" deletes the audio already past N days at once: ask first. */
export const retentionNeedsConfirm = (
  current: number | null,
  next: number | null,
): boolean => next !== null && (current === null || next < current);

export interface RetentionConfirmProps {
  days: number;
  onConfirm: () => void;
  onCancel: () => void;
  t: Translate;
}

export function RetentionConfirm({
  days,
  onConfirm,
  onCancel,
  t,
}: RetentionConfirmProps) {
  return (
    <div role="alertdialog" className="space-y-2 px-4 pb-3">
      <p className="text-sm">
        {t("settings.meetings.retention.confirm", { days })}
      </p>
      <div className="flex gap-2">
        <Button variant="danger" size="sm" onClick={onConfirm}>
          {t("settings.meetings.retention.confirmAction")}
        </Button>
        <Button variant="secondary" size="sm" onClick={onCancel}>
          {t("settings.meetings.retention.cancel")}
        </Button>
      </div>
    </div>
  );
}

/** Settings > General > Meetings: how long the audio of a meeting stays, and the cap. */
export const MeetingAudioSettings: React.FC = React.memo(() => {
  const { t } = useTranslation();
  const { getSetting, refreshSettings } = useSettings();
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const [pending, setPending] = useState<number | null>(null);
  const days = getSetting("meeting_audio_retention_days") ?? null;
  const hours = getSetting("meeting_cap_hours") ?? 3;

  const run = async (
    change: () => Promise<{ status: "ok" | "error" }>,
  ): Promise<void> => {
    setBusy(true);
    setFailed(false);
    try {
      const result = await change();
      setFailed(result.status === "error");
      await refreshSettings();
    } finally {
      setBusy(false);
    }
  };

  const setRetention = (next: number | null) =>
    void run(() => commands.changeMeetingRetentionSetting(next));

  return (
    <>
      <SettingContainer
        title={t("settings.meetings.retention.title")}
        description={t("settings.meetings.retention.description")}
        descriptionMode="tooltip"
        grouped={true}
      >
        <Dropdown
          className="min-w-40"
          options={retentionOptions(days, t)}
          selectedValue={retentionValue(days)}
          onSelect={(value) => {
            const next = retentionDays(value);
            if (next === days) return;
            if (next !== null && retentionNeedsConfirm(days, next)) {
              setPending(next);
            } else setRetention(next);
          }}
          disabled={busy || pending !== null}
        />
      </SettingContainer>
      {pending !== null && (
        <RetentionConfirm
          days={pending}
          onConfirm={() => {
            setPending(null);
            setRetention(pending);
          }}
          onCancel={() => setPending(null)}
          t={t}
        />
      )}
      <SettingContainer
        title={t("settings.meetings.cap.title")}
        description={t("settings.meetings.cap.description")}
        descriptionMode="tooltip"
        grouped={true}
      >
        <Dropdown
          className="min-w-24"
          options={CAP_HOURS.map((n) => ({
            value: String(n),
            label: t("settings.meetings.cap.hours", { hours: n }),
          }))}
          selectedValue={String(hours)}
          onSelect={(value) =>
            void run(() => commands.changeMeetingCapSetting(Number(value)))
          }
          disabled={busy}
        />
      </SettingContainer>
      {failed && (
        <p role="alert" className="px-4 pb-3 text-sm text-danger">
          {t("settings.meetings.saveFailed")}
        </p>
      )}
    </>
  );
});

MeetingAudioSettings.displayName = "MeetingAudioSettings";
