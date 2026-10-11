import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface PauseMediaProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const PauseMediaWhileRecording: React.FC<PauseMediaProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    return (
      <ToggleSwitch
        checked={getSetting("pause_media_while_recording") ?? false}
        onChange={(enabled) =>
          updateSetting("pause_media_while_recording", enabled)
        }
        isUpdating={isUpdating("pause_media_while_recording")}
        label={t("settings.sound.pauseMediaWhileRecording.label")}
        description={t("settings.sound.pauseMediaWhileRecording.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  },
);

export const PauseMediaOnMeeting: React.FC<PauseMediaProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    return (
      <ToggleSwitch
        checked={getSetting("pause_media_on_meeting") ?? true}
        onChange={(enabled) => updateSetting("pause_media_on_meeting", enabled)}
        isUpdating={isUpdating("pause_media_on_meeting")}
        label={t("settings.sound.pauseMediaOnMeeting.label")}
        description={t("settings.sound.pauseMediaOnMeeting.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  },
);
