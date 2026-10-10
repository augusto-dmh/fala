import React from "react";
import { SettingContainer } from "./SettingContainer";

interface ToggleSwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  isUpdating?: boolean;
  label: string;
  description: string;
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
  tooltipPosition?: "top" | "bottom";
}

export const ToggleSwitch: React.FC<ToggleSwitchProps> = ({
  checked,
  onChange,
  disabled = false,
  isUpdating = false,
  label,
  description,
  descriptionMode = "tooltip",
  grouped = false,
  tooltipPosition = "top",
}) => {
  return (
    <SettingContainer
      title={label}
      description={description}
      descriptionMode={descriptionMode}
      grouped={grouped}
      disabled={disabled}
      tooltipPosition={tooltipPosition}
    >
      <label
        className={`flex items-center ${disabled || isUpdating ? "cursor-not-allowed" : "cursor-pointer"}`}
      >
        <input
          type="checkbox"
          value=""
          className="sr-only peer"
          checked={checked}
          disabled={disabled || isUpdating}
          onChange={(e) => onChange(e.target.checked)}
        />
        {/* Fluent toggle: 40x20 outlined track with a 12 px knob when off,
            ink track with an on-accent knob when on. */}
        <div className="relative w-[40px] h-[20px] rounded-full border border-text-2 bg-transparent transition-colors peer-hover:bg-text/5 peer-checked:bg-accent peer-checked:border-accent peer-focus-visible:focus-ring peer-disabled:opacity-50 after:content-[''] after:absolute after:top-[3px] after:start-[3px] after:h-[12px] after:w-[12px] after:rounded-full after:bg-text-2 after:transition-transform peer-checked:after:bg-on-accent peer-checked:after:translate-x-[20px] rtl:peer-checked:after:-translate-x-[20px]"></div>
      </label>
      {isUpdating && (
        <div className="absolute inset-0 flex items-center justify-center">
          <div className="w-4 h-4 border-2 border-text-2 border-t-transparent rounded-full animate-spin"></div>
        </div>
      )}
    </SettingContainer>
  );
};
