import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";
import { useOsType } from "../../hooks/useOsType";

interface ShowIdlePillProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

/** Keeps a quiet pill on screen between dictations, with the shortcut on hover. Windows only, and
 *  only while a pill style is on. */
export const ShowIdlePill: React.FC<ShowIdlePillProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const osType = useOsType();

    if (osType !== "windows" || getSetting("overlay_style") === "none") {
      return null;
    }

    return (
      <ToggleSwitch
        checked={getSetting("pill_idle_visible") ?? false}
        onChange={(enabled) => updateSetting("pill_idle_visible", enabled)}
        isUpdating={isUpdating("pill_idle_visible")}
        label={t("settings.advanced.idlePill.label")}
        description={t("settings.advanced.idlePill.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
        tooltipPosition="bottom"
      />
    );
  },
);
