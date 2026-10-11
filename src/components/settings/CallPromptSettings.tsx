import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type CallAppInfo } from "@/bindings";
import { useSettings } from "../../hooks/useSettings";
import { Button } from "../ui/Button";
import { Dropdown } from "../ui/Dropdown";
import { SettingContainer } from "../ui/SettingContainer";
import { ToggleSwitch } from "../ui/ToggleSwitch";

/** Settings > General > Meetings: offer to record when a call starts, and the apps it never
 *  asks about. Writes through its own commands, then reloads the settings store. */
export const CallPromptSettings: React.FC = React.memo(() => {
  const { t } = useTranslation();
  const { getSetting, refreshSettings } = useSettings();
  const [apps, setApps] = useState<CallAppInfo[]>([]);
  const [busy, setBusy] = useState(false);
  const enabled = getSetting("call_prompt_enabled") ?? true;
  const muted = getSetting("call_prompt_muted_apps") ?? [];

  useEffect(() => {
    void commands.callPromptApps().then(setApps);
  }, []);

  const name = (id: string) => apps.find((app) => app.id === id)?.name ?? id;

  const run = async (change: () => Promise<unknown>) => {
    setBusy(true);
    try {
      await change();
      await refreshSettings();
    } finally {
      setBusy(false);
    }
  };

  const setMuted = (ids: string[]) =>
    run(() => commands.changeCallPromptMutedAppsSetting(ids));

  return (
    <>
      <ToggleSwitch
        checked={enabled}
        onChange={(value) =>
          run(() => commands.changeCallPromptEnabledSetting(value))
        }
        isUpdating={busy}
        label={t("settings.meetings.callPrompt.label")}
        description={t("settings.meetings.callPrompt.description")}
        descriptionMode="tooltip"
        grouped={true}
      />
      <SettingContainer
        title={t("settings.meetings.mutedApps.title")}
        description={t("settings.meetings.mutedApps.description")}
        descriptionMode="tooltip"
        grouped={true}
      >
        <Dropdown
          className="min-w-40"
          options={apps
            .filter((app) => !muted.includes(app.id))
            .map((app) => ({ value: app.id, label: app.name }))}
          selectedValue={null}
          onSelect={(id) => void setMuted([...muted, id])}
          placeholder={t("settings.meetings.mutedApps.add")}
          disabled={busy || !enabled}
        />
      </SettingContainer>
      {muted.length > 0 && (
        <div className="px-4 p-2 flex flex-wrap gap-1">
          {muted.map((id) => (
            <Button
              key={id}
              onClick={() => void setMuted(muted.filter((m) => m !== id))}
              disabled={busy}
              variant="secondary"
              size="sm"
              className="inline-flex items-center gap-1 cursor-pointer"
              aria-label={t("settings.meetings.mutedApps.remove", {
                app: name(id),
              })}
            >
              <span>{name(id)}</span>
              <svg
                className="w-3 h-3"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </Button>
          ))}
        </div>
      )}
    </>
  );
});

CallPromptSettings.displayName = "CallPromptSettings";
