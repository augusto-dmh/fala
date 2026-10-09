import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../hooks/useSettings";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { ApiKeyField } from "./PostProcessingSettingsApi/ApiKeyField";

const GEMINI_PROVIDER_ID = "gemini";

/** The automatic dictation LLM: on/off, the Gemini key and the apps where it stays off. */
export const LlmSettings: React.FC = React.memo(() => {
  const { t } = useTranslation();
  const {
    settings,
    getSetting,
    updateSetting,
    isUpdating,
    updatePostProcessApiKey,
  } = useSettings();
  const [newApp, setNewApp] = useState("");
  const enabled = getSetting("llm_enabled") ?? true;
  const disabledApps = getSetting("llm_disabled_apps") ?? [];
  const apiKey = settings?.post_process_api_keys?.[GEMINI_PROVIDER_ID] ?? "";
  const appsUpdating = isUpdating("llm_disabled_apps");

  const handleAddApp = () => {
    const app = newApp.trim();
    if (!app) return;
    // The backend reduces each entry to the app name and drops repeats.
    updateSetting("llm_disabled_apps", [...disabledApps, app]);
    setNewApp("");
  };

  return (
    <>
      <ToggleSwitch
        checked={enabled}
        onChange={(value) => updateSetting("llm_enabled", value)}
        isUpdating={isUpdating("llm_enabled")}
        label={t("settings.llm.enabled.label")}
        description={t("settings.llm.enabled.description")}
        descriptionMode="tooltip"
        grouped={true}
      />
      <SettingContainer
        title={t("settings.llm.apiKey.title")}
        description={t("settings.llm.apiKey.description")}
        descriptionMode="tooltip"
        layout="horizontal"
        grouped={true}
      >
        <ApiKeyField
          value={apiKey}
          onBlur={(value) => {
            const trimmed = value.trim();
            if (trimmed !== apiKey) {
              void updatePostProcessApiKey(GEMINI_PROVIDER_ID, trimmed);
            }
          }}
          placeholder={t("settings.llm.apiKey.placeholder")}
          disabled={!enabled}
        />
      </SettingContainer>
      <SettingContainer
        title={t("settings.llm.disabledApps.title")}
        description={t("settings.llm.disabledApps.description")}
        descriptionMode="tooltip"
        grouped={true}
      >
        <div className="flex items-center gap-2">
          <Input
            type="text"
            className="max-w-40"
            value={newApp}
            onChange={(e) => setNewApp(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                handleAddApp();
              }
            }}
            placeholder={t("settings.llm.disabledApps.placeholder")}
            variant="compact"
            disabled={appsUpdating}
          />
          <Button
            onClick={handleAddApp}
            disabled={!newApp.trim() || appsUpdating}
            variant="primary"
            size="md"
          >
            {t("settings.llm.disabledApps.add")}
          </Button>
        </div>
      </SettingContainer>
      {disabledApps.length > 0 && (
        <div className="px-4 p-2 flex flex-wrap gap-1">
          {disabledApps.map((app) => (
            <Button
              key={app}
              onClick={() =>
                updateSetting(
                  "llm_disabled_apps",
                  disabledApps.filter((a) => a !== app),
                )
              }
              disabled={appsUpdating}
              variant="secondary"
              size="sm"
              className="inline-flex items-center gap-1 cursor-pointer"
              aria-label={t("settings.llm.disabledApps.remove", { app })}
            >
              <span>{app}</span>
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

LlmSettings.displayName = "LlmSettings";
