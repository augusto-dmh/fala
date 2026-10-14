import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../hooks/useSettings";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { KeyField } from "../meeting/KeyField";
import { AppStyles } from "./AppStyles";
import { CleanupLevelCards } from "./CleanupLevelCards";
import { DEFAULT_CLEANUP_LEVEL } from "./cleanupLevelModel";

const GEMINI_PROVIDER_ID = "gemini";

/** The automatic dictation LLM: on/off, how much it rewrites, the Gemini key, the apps where it
 *  stays off and the writing style of each app. */
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
  const cleanupLevel = getSetting("cleanup_level") ?? DEFAULT_CLEANUP_LEVEL;
  const disabledApps = getSetting("llm_disabled_apps") ?? [];
  const apiKey = settings?.post_process_api_keys?.[GEMINI_PROVIDER_ID] ?? "";
  const appsUpdating = isUpdating("llm_disabled_apps");
  // On without a key, the AI waits: the row says so instead of looking active (the Privacy
  // section says the same). The switch stays the person's choice, so it can be turned off
  // before a key is saved.
  const hasKey = apiKey.trim() !== "";
  const formatting = enabled && hasKey;

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
        description={
          enabled && !hasKey
            ? t("settings.llm.enabled.waitingKey")
            : t("settings.llm.enabled.description")
        }
        descriptionMode="tooltip"
        grouped={true}
      />
      <CleanupLevelCards
        value={cleanupLevel}
        onChange={(level) => updateSetting("cleanup_level", level)}
        dimmed={!formatting}
      />
      <SettingContainer
        title={t("settings.llm.apiKey.title")}
        description={t("settings.llm.apiKey.description")}
        descriptionMode="tooltip"
        layout="stacked"
        grouped={true}
      >
        <KeyField
          label={t("settings.llm.apiKey.title")}
          saved={hasKey}
          onSave={(key) =>
            void updatePostProcessApiKey(GEMINI_PROVIDER_ID, key)
          }
          onRemove={() => void updatePostProcessApiKey(GEMINI_PROVIDER_ID, "")}
        />
      </SettingContainer>
      <SettingContainer
        title={t("settings.llm.disabledApps.title")}
        description={t("settings.llm.disabledApps.description")}
        descriptionMode="tooltip"
        layout="stacked"
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
        {/* The apps sit in the same card as the field that adds them. */}
        {disabledApps.length > 0 && (
          <div className="mt-2 flex flex-wrap gap-1">
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
      </SettingContainer>
      <AppStyles />
    </>
  );
});

LlmSettings.displayName = "LlmSettings";
