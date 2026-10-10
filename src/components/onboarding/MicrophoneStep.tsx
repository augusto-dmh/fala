import React from "react";
import { useTranslation } from "react-i18next";
import { ShieldAlert } from "lucide-react";
import { Button } from "../ui/Button";
import { MicrophoneSelector } from "../settings/MicrophoneSelector";
import { useSettings } from "../../hooks/useSettings";
import { MicLevelMeter } from "./MicLevelMeter";
import type { MicAccess } from "./onboardingModel";

interface MicrophoneStepProps {
  access: MicAccess;
  /** The person already opened the privacy settings; access is being polled. */
  waiting: boolean;
  onOpenPrivacySettings: () => void;
}

/** Step 1: pick the microphone and watch the meter react; on Windows, how to
 *  lift a denied microphone access, in the same place. */
export const MicrophoneStep: React.FC<MicrophoneStepProps> = ({
  access,
  waiting,
  onOpenPrivacySettings,
}) => {
  const { t } = useTranslation();
  const { getSetting } = useSettings();

  if (access === "denied") {
    return (
      <div className="flex gap-3 rounded-lg border border-warn/40 bg-warn/10 p-4">
        <ShieldAlert
          className="mt-0.5 h-5 w-5 shrink-0 text-warn"
          strokeWidth={1.5}
        />
        <div className="flex flex-col gap-3">
          <h3 className="text-body font-semibold text-text">
            {t("onboarding.microphone.denied.title")}
          </h3>
          <ol className="list-decimal space-y-1 ps-5 text-body text-text">
            <li>{t("onboarding.microphone.denied.step1")}</li>
            <li>{t("onboarding.microphone.denied.step2")}</li>
            <li>{t("onboarding.microphone.denied.step3")}</li>
          </ol>
          <div className="flex items-center gap-3">
            <Button variant="primary" onClick={onOpenPrivacySettings}>
              {t("onboarding.microphone.denied.openSettings")}
            </Button>
            {waiting && (
              <span className="text-caption text-text-2">
                {t("onboarding.microphone.denied.waiting")}
              </span>
            )}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <MicrophoneSelector descriptionMode="inline" />
      <MicLevelMeter microphone={getSetting("selected_microphone")} />
    </div>
  );
};
