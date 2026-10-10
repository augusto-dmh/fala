import React from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../ui/Button";
import { SettingContainer } from "../../ui/SettingContainer";
import {
  ONBOARDING_STEPS,
  type OnboardingStepId,
} from "../../onboarding/onboardingModel";

export type OnboardingPreviewStep = OnboardingStepId;

interface OnboardingPreviewProps {
  onPreview: (step: OnboardingPreviewStep) => void;
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const OnboardingPreview: React.FC<OnboardingPreviewProps> = ({
  onPreview,
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();

  return (
    <SettingContainer
      title={t("settings.debug.onboardingPreview.title")}
      description={t("settings.debug.onboardingPreview.description")}
      descriptionMode={descriptionMode}
      grouped={grouped}
    >
      <div className="flex gap-2">
        {ONBOARDING_STEPS.map((step) => (
          <Button
            key={step}
            variant="secondary"
            size="md"
            onClick={() => onPreview(step)}
          >
            {t(`settings.debug.onboardingPreview.${step}Button`)}
          </Button>
        ))}
      </div>
    </SettingContainer>
  );
};
