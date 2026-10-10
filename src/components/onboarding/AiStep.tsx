import React from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";
import { ApiKeyField } from "../settings/PostProcessingSettingsApi/ApiKeyField";
import { useSettings } from "../../hooks/useSettings";

const GEMINI_PROVIDER_ID = "gemini";

interface AiStepProps {
  onFinish: () => void;
}

/** Step 3 (optional): what leaves the machine, and the Gemini key or skip. */
export const AiStep: React.FC<AiStepProps> = ({ onFinish }) => {
  const { t } = useTranslation();
  const { settings, updatePostProcessApiKey } = useSettings();
  const apiKey = settings?.post_process_api_keys?.[GEMINI_PROVIDER_ID] ?? "";

  return (
    <div className="flex flex-col gap-4">
      <p className="text-body text-text">{t("onboarding.ai.privacy")}</p>
      <SettingContainer
        title={t("onboarding.ai.keyLabel")}
        description={t("onboarding.ai.keyDescription")}
        layout="stacked"
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
          disabled={false}
        />
      </SettingContainer>
      <div className="flex justify-end gap-2">
        <Button variant="secondary" onClick={onFinish}>
          {t("onboarding.ai.skip")}
        </Button>
        <Button variant="primary" onClick={onFinish}>
          {t("onboarding.ai.finish")}
        </Button>
      </div>
    </div>
  );
};
